//! bridge-host — the machine being accessed.
//!
//! MVP = "portable, attended" mode (docs: Host Agent, Platform Hardening):
//! runs as the logged-in user, every session requires the one-time code AND a click on Accept,
//! a session indicator is always visible and lets the local user disconnect at any time.

mod session;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use bridge_crypto::keys::{self, DeviceKey};
use bridge_crypto::{keystore, otc};
use bridge_net::config::{self, ClientConfig};
use bridge_net::endpoint::{local_ip_toward, punch, session_server_config};
use bridge_net::relay_proxy::RelayProxy;
use bridge_net::rendezvous;
use bridge_proto::msg::Incoming;
use bridge_session::AuthLimiter;
use clap::Parser;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};

#[derive(Parser, Debug)]
#[command(name = "bridge-host", version, about = "Bridge host — allow a trusted person to access this computer")]
struct Args {
    /// Rendezvous server `host[:port]` (remembered for next time).
    #[arg(long)]
    server: Option<String>,
    /// Rendezvous server key printed by bridge-rendezvous (remembered). Without it the key is pinned on first use.
    #[arg(long)]
    server_key: Option<String>,
    /// Configuration directory (default: %APPDATA%\Bridge).
    #[arg(long)]
    config_dir: Option<PathBuf>,
}

/// Shared host state.
pub struct HostCtx {
    pub key: Arc<DeviceKey>,
    code: Mutex<String>,
    pub limiter: Mutex<AuthLimiter>,
    pub busy: AtomicBool,
    device_id: Mutex<Option<String>>,
    proxies: Mutex<Vec<RelayProxy>>,
}

impl HostCtx {
    fn new(key: Arc<DeviceKey>) -> Self {
        Self {
            key,
            code: Mutex::new(otc::generate()),
            limiter: Mutex::new(AuthLimiter::new()),
            busy: AtomicBool::new(false),
            device_id: Mutex::new(None),
            proxies: Mutex::new(Vec::new()),
        }
    }

    pub fn code(&self) -> String {
        self.code.lock().map(|c| c.clone()).unwrap_or_default()
    }

    /// Codes are single-use and also rotate after repeated failures.
    pub fn rotate_code(&self) {
        if let Ok(mut c) = self.code.lock() {
            *c = otc::generate();
        }
        self.print_banner();
    }

    fn set_device_id(&self, id: String) {
        if let Ok(mut d) = self.device_id.lock() {
            *d = Some(id);
        }
    }

    fn keep_proxy(&self, p: RelayProxy) {
        if let Ok(mut v) = self.proxies.lock() {
            v.retain(|p| !p.is_finished());
            v.push(p);
        }
    }

    pub fn print_banner(&self) {
        let id = self.device_id.lock().ok().and_then(|d| d.clone());
        let id = id.map(|i| format_id(&i)).unwrap_or_else(|| "(connecting to server…)".into());
        println!();
        println!("  ┌──────────────────────────────────────────────────────────┐");
        println!("  │  Bridge — this computer can be accessed remotely         │");
        println!("  │                                                          │");
        println!("  │     Your ID:    {id:<41}│");
        println!("  │     Code:       {:<41}│", otc::display(&self.code()));
        println!("  │                                                          │");
        println!("  │  Key fingerprint: {:<39}│", keys::fingerprint(&self.key.public()));
        println!("  │                                                          │");
        println!("  │  ! Only share the ID and code with someone you know and  │");
        println!("  │  ! trust. Banks, Microsoft or the police never ask you   │");
        println!("  │  ! to install remote access software.                    │");
        println!("  └──────────────────────────────────────────────────────────┘");
        println!("  Close this window to stop accepting connections.");
    }
}

pub fn format_id(id: &str) -> String {
    id.as_bytes().chunks(3).map(|c| String::from_utf8_lossy(c).into_owned()).collect::<Vec<_>>().join(" ")
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,quinn=warn".into()),
        )
        .init();
    bridge_platform::init_process();
    let args = Args::parse();

    let dir = args.config_dir.unwrap_or_else(config::default_dir);
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let mut cfg = ClientConfig::load(&dir)?;
    if let Some(s) = args.server {
        cfg.server = Some(s);
    }
    if let Some(k) = args.server_key {
        keys::decode_public(&k).context("--server-key is not a valid key")?;
        cfg.server_key = Some(k);
    }
    cfg.save(&dir)?;

    let key = Arc::new(keystore::load_or_create_device_key(&dir.join("device.key"))?);
    let endpoint = quinn::Endpoint::server(session_server_config(&key)?, "0.0.0.0:0".parse()?)?;
    let ctx = Arc::new(HostCtx::new(key));
    ctx.print_banner();

    tokio::spawn(registration_loop(endpoint.clone(), dir, cfg, ctx.clone()));

    // SEC: T-42 — bound concurrent unauthenticated connections.
    let preauth = Arc::new(Semaphore::new(4));
    while let Some(incoming) = endpoint.accept().await {
        let Ok(permit) = preauth.clone().try_acquire_owned() else {
            incoming.refuse();
            continue;
        };
        let ctx = ctx.clone();
        tokio::spawn(async move {
            let remote = incoming.remote_address();
            match incoming.await {
                Ok(conn) => {
                    if let Err(e) = session::run(ctx, conn, permit).await {
                        warn!(%remote, "session ended with error: {e:#}");
                    }
                }
                Err(e) => debug!(%remote, "handshake failed: {e}"),
            }
        });
    }
    Ok(())
}

async fn registration_loop(endpoint: quinn::Endpoint, dir: PathBuf, mut cfg: ClientConfig, ctx: Arc<HostCtx>) {
    let mut backoff = Duration::from_secs(1);
    loop {
        match register_once(&endpoint, &dir, &mut cfg, &ctx).await {
            Ok(()) => backoff = Duration::from_secs(1),
            Err(e) => warn!("rendezvous: {e:#}"),
        }
        warn!("offline — reconnecting in {}s", backoff.as_secs());
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_secs(60));
    }
}

async fn register_once(
    endpoint: &quinn::Endpoint,
    dir: &std::path::Path,
    cfg: &mut ClientConfig,
    ctx: &Arc<HostCtx>,
) -> Result<()> {
    let conn = rendezvous::connect(endpoint, dir, cfg).await?;
    let port = endpoint.local_addr()?.port();
    let local_addrs = local_ip_toward(conn.remote_address())
        .map(|ip| vec![SocketAddr::new(ip, port).to_string()])
        .unwrap_or_default();
    let mut reg = rendezvous::register_host(&conn, &ctx.key, cfg.device_id.as_deref(), local_addrs).await?;
    let id = reg.registered.device_id.clone();
    if cfg.device_id.as_deref() != Some(id.as_str()) {
        cfg.device_id = Some(id.clone());
        cfg.save(dir)?;
    }
    info!(device_id = %id, observed = %reg.registered.observed_addr, "registered with rendezvous server");
    ctx.set_device_id(id);
    ctx.print_banner();

    loop {
        let incoming = rendezvous::next_incoming(&mut reg.recv).await?;
        on_incoming(endpoint, ctx, incoming).await;
    }
}

/// A viewer looked us up: open NAT mappings towards it and prepare the relay path.
async fn on_incoming(endpoint: &quinn::Endpoint, ctx: &Arc<HostCtx>, incoming: Incoming) {
    let targets: Vec<SocketAddr> = incoming.viewer_candidates.iter().filter_map(|a| a.parse().ok()).take(4).collect();
    debug!(?targets, "incoming connection notice");
    punch(endpoint, &targets);
    if let Some(info) = incoming.relay {
        let Ok(local) = endpoint.local_addr() else { return };
        let local = SocketAddr::new([127, 0, 0, 1].into(), local.port());
        match RelayProxy::start(&info, Some(local)).await {
            Ok(p) => ctx.keep_proxy(p),
            Err(e) => warn!("relay unavailable: {e:#}"),
        }
    }
}
