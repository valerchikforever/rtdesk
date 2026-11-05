//! bridge-rendezvous — see `docs/02 - Architecture/Components/Rendezvous Server.md`.
//!
//! The server is untrusted for confidentiality: it never sees session keys or codes.
//! It can deny service, but cannot grant access (hosts pin keys and run a channel-bound PAKE).

mod limiter;
mod registry;

use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use bridge_crypto::keys::{self, PublicKey};
use bridge_crypto::{keystore, relay_token};
use bridge_net::endpoint::rendezvous_server_config;
use bridge_net::relay_proxy::unix_now;
use bridge_proto::framing::{read_msg, write_msg, MAX_RENDEZVOUS};
use bridge_proto::msg::{
    register_transcript, rv_request, rv_response, Challenge, Incoming, LookupResult, Register, Registered, RelayInfo,
    RvError, RvRequest, RvResponse,
};
use bridge_proto::relay::{ROLE_HOST, ROLE_VIEWER};
use clap::Parser;
use quinn::{Connection, RecvStream, SendStream};
use rand::RngCore;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use limiter::{Kind, RateLimiter};
use registry::Registry;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_LOCAL_ADDRS: usize = 8;

#[derive(Parser, Debug)]
#[command(name = "bridge-rendezvous", version, about = "Bridge rendezvous server (free & open source, AGPL-3.0)")]
struct Args {
    /// UDP address to listen on.
    #[arg(long, default_value = "0.0.0.0:21116")]
    listen: SocketAddr,
    /// Directory for the server key, relay secret and device registry.
    #[arg(long, default_value = "data")]
    data_dir: PathBuf,
    /// Public `host:port` of the relay handed to clients. Omit to disable relay fallback.
    #[arg(long)]
    relay: Option<String>,
    /// Validity of relay tokens in seconds.
    #[arg(long, default_value_t = 60)]
    relay_token_ttl: u64,
}

struct OnlineHost {
    conn_id: usize,
    tx: mpsc::Sender<Incoming>,
    observed: SocketAddr,
    local_addrs: Vec<SocketAddr>,
}

struct State {
    registry: Mutex<Registry>,
    online: Mutex<std::collections::HashMap<String, OnlineHost>>,
    limiter: Mutex<RateLimiter>,
    relay_secret: [u8; 32],
    relay: Option<String>,
    relay_ttl: u64,
}

impl State {
    fn allow(&self, ip: IpAddr, kind: Kind) -> bool {
        self.limiter.lock().map(|mut l| l.allow(ip, kind)).unwrap_or(false)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let args = Args::parse();
    std::fs::create_dir_all(&args.data_dir).with_context(|| format!("creating {}", args.data_dir.display()))?;

    let key = keystore::load_or_create_device_key(&args.data_dir.join("server.key"))?;
    let relay_secret = load_or_create_secret(&args.data_dir.join("relay.secret"))?;
    let registry = Registry::load(args.data_dir.join("devices.json"))?;

    let pubkey = key.public();
    println!("──────────────────────────────────────────────────────────────");
    println!(" Bridge rendezvous server listening on udp/{}", args.listen);
    println!(" Server key (give to clients as --server-key):");
    println!("   {}", keys::encode_public(&pubkey));
    println!(" Fingerprint: {}", keys::fingerprint(&pubkey));
    match &args.relay {
        Some(r) => println!(" Relay: {r} (shares {}/relay.secret)", args.data_dir.display()),
        None => println!(" Relay: disabled (pass --relay host:port to enable fallback)"),
    }
    println!(" Registered devices: {}", registry.len());
    println!("──────────────────────────────────────────────────────────────");

    let state = Arc::new(State {
        registry: Mutex::new(registry),
        online: Mutex::new(Default::default()),
        limiter: Mutex::new(RateLimiter::new()),
        relay_secret,
        relay: args.relay,
        relay_ttl: args.relay_token_ttl,
    });

    let endpoint = quinn::Endpoint::server(rendezvous_server_config(&key)?, args.listen)?;
    while let Some(incoming) = endpoint.accept().await {
        let ip = incoming.remote_address().ip();
        // SEC: T-40 — validate source addresses (stateless retry) before doing any work,
        // so spoofed packets cannot use us for amplification.
        if !incoming.remote_address_validated() {
            let _ = incoming.retry();
            continue;
        }
        if !state.allow(ip, Kind::Connect) {
            incoming.refuse();
            continue;
        }
        let st = state.clone();
        tokio::spawn(async move {
            match incoming.await {
                Ok(conn) => handle_conn(st, conn).await,
                Err(e) => debug!(%ip, "handshake failed: {e}"),
            }
        });
    }
    Ok(())
}

fn load_or_create_secret(path: &std::path::Path) -> Result<[u8; 32]> {
    if path.exists() {
        let bytes = std::fs::read(path)?;
        return bytes.as_slice().try_into().context("relay.secret must be exactly 32 bytes");
    }
    let mut s = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut s);
    keystore::write_private(path, &s)?;
    Ok(s)
}

async fn handle_conn(state: Arc<State>, conn: Connection) {
    while let Ok((send, recv)) = conn.accept_bi().await {
        let st = state.clone();
        let c = conn.clone();
        tokio::spawn(async move {
            let peer = c.remote_address();
            if let Err(e) = handle_stream(st, c, send, recv).await {
                debug!(%peer, "request failed: {e:#}");
            }
        });
    }
}

async fn respond(send: &mut SendStream, kind: rv_response::Kind) -> Result<()> {
    write_msg(send, &RvResponse { kind: Some(kind) }).await?;
    Ok(())
}

async fn fail(send: &mut SendStream, message: &str) -> Result<()> {
    respond(send, rv_response::Kind::Error(RvError { message: message.into() })).await?;
    let _ = send.finish();
    bail!("{message}")
}

async fn read_request(recv: &mut RecvStream) -> Result<RvRequest> {
    Ok(tokio::time::timeout(REQUEST_TIMEOUT, read_msg(recv, MAX_RENDEZVOUS)).await??)
}

async fn handle_stream(state: Arc<State>, conn: Connection, mut send: SendStream, mut recv: RecvStream) -> Result<()> {
    match read_request(&mut recv).await?.kind {
        Some(rv_request::Kind::RegisterStart(_)) => handle_register(state, conn, send, recv).await,
        Some(rv_request::Kind::Lookup(l)) => handle_lookup(state, conn, send, &l.device_id).await,
        _ => fail(&mut send, "unexpected request").await,
    }
}

fn valid_device_id(id: &str) -> bool {
    id.len() == 9 && id.bytes().all(|b| b.is_ascii_digit()) && !id.starts_with('0')
}

async fn handle_register(
    state: Arc<State>,
    conn: Connection,
    mut send: SendStream,
    mut recv: RecvStream,
) -> Result<()> {
    let observed = conn.remote_address();
    if !state.allow(observed.ip(), Kind::Register) {
        return fail(&mut send, "rate limited").await;
    }
    let mut nonce = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    respond(&mut send, rv_response::Kind::Challenge(Challenge { nonce: nonce.to_vec() })).await?;

    let Some(rv_request::Kind::Register(reg)) = read_request(&mut recv).await?.kind else {
        return fail(&mut send, "expected Register").await;
    };
    let Register { pubkey, device_id: wanted, signature, local_addrs } = reg;
    let Some(pubkey) = keys::public_from_slice(&pubkey) else {
        return fail(&mut send, "invalid public key").await;
    };
    // SEC: M-04 — proof of possession of the key, bound to our fresh nonce and the claimed ID.
    if !keys::verify(&pubkey, &register_transcript(&nonce, &pubkey, &wanted), &signature) {
        return fail(&mut send, "invalid signature").await;
    }
    if !wanted.is_empty() && !valid_device_id(&wanted) {
        return fail(&mut send, "invalid device id").await;
    }

    let bound = match state.registry.lock() {
        Ok(mut reg) => reg.bind(&pubkey, (!wanted.is_empty()).then_some(wanted.as_str())),
        Err(_) => Err("registry unavailable"),
    };
    let device_id = match bound {
        Ok(id) => id,
        Err(msg) => return fail(&mut send, msg).await,
    };

    let local_addrs = sanitize_local_addrs(&local_addrs);
    let (tx, mut rx) = mpsc::channel::<Incoming>(16);
    let conn_id = conn.stable_id();
    if let Ok(mut online) = state.online.lock() {
        online.insert(device_id.clone(), OnlineHost { conn_id, tx, observed, local_addrs });
    }
    respond(
        &mut send,
        rv_response::Kind::Registered(Registered { device_id: device_id.clone(), observed_addr: observed.to_string() }),
    )
    .await?;
    info!(%device_id, %observed, fingerprint = %keys::fingerprint(&pubkey), "host online");

    loop {
        tokio::select! {
            msg = rx.recv() => {
                let Some(incoming) = msg else { break };
                if respond(&mut send, rv_response::Kind::Incoming(incoming)).await.is_err() {
                    break;
                }
            }
            _ = conn.closed() => break,
        }
    }

    if let Ok(mut online) = state.online.lock() {
        if online.get(&device_id).is_some_and(|h| h.conn_id == conn_id) {
            online.remove(&device_id);
        }
    }
    info!(%device_id, "host offline");
    Ok(())
}

/// Only private/link-local IPv4 candidates are accepted (LAN direct path).
fn sanitize_local_addrs(addrs: &[String]) -> Vec<SocketAddr> {
    addrs
        .iter()
        .take(MAX_LOCAL_ADDRS)
        .filter_map(|a| a.parse::<SocketAddr>().ok())
        .filter(|a| match a.ip() {
            IpAddr::V4(ip) => (ip.is_private() || ip.is_link_local()) && a.port() != 0,
            IpAddr::V6(_) => false,
        })
        .collect()
}

fn relay_info(state: &State, session_id: &[u8; 16], role: u8, expiry: u64) -> Option<RelayInfo> {
    let addr = state.relay.clone()?;
    Some(RelayInfo {
        addr,
        session_id: session_id.to_vec(),
        token: relay_token::issue(&state.relay_secret, session_id, role, expiry).to_vec(),
        expiry,
        role: u32::from(role),
    })
}

async fn handle_lookup(state: Arc<State>, conn: Connection, mut send: SendStream, device_id: &str) -> Result<()> {
    let viewer = conn.remote_address();
    // SEC: M-32 — limits ID enumeration / presence probing.
    if !state.allow(viewer.ip(), Kind::Lookup) {
        return fail(&mut send, "rate limited").await;
    }
    if !valid_device_id(device_id) {
        return fail(&mut send, "invalid device id").await;
    }
    let host_key: Option<PublicKey> = state.registry.lock().ok().and_then(|r| r.key_of(device_id));
    let online = state
        .online
        .lock()
        .ok()
        .and_then(|o| o.get(device_id).map(|h| (h.tx.clone(), h.observed, h.local_addrs.clone())));
    let (Some(host_key), Some((tx, host_observed, host_local))) = (host_key, online) else {
        respond(&mut send, rv_response::Kind::LookupResult(LookupResult { found: false, ..Default::default() }))
            .await?;
        let _ = send.finish();
        return Ok(());
    };

    let mut session_id = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut session_id);
    let expiry = unix_now() + state.relay_ttl;

    // LAN candidates only for viewers behind the same public IP (privacy, see NAT Traversal note).
    let mut candidates: Vec<String> = Vec::new();
    if host_observed.ip() == viewer.ip() {
        candidates.extend(host_local.iter().map(ToString::to_string));
    }
    candidates.push(host_observed.to_string());

    let notice = Incoming {
        viewer_candidates: vec![viewer.to_string()],
        relay: relay_info(&state, &session_id, ROLE_HOST, expiry),
    };
    if tx.try_send(notice).is_err() {
        return fail(&mut send, "host is busy, try again").await;
    }
    respond(
        &mut send,
        rv_response::Kind::LookupResult(LookupResult {
            found: true,
            host_pubkey: host_key.to_vec(),
            candidates,
            relay: relay_info(&state, &session_id, ROLE_VIEWER, expiry),
            observed_addr: viewer.to_string(),
        }),
    )
    .await?;
    let _ = send.finish();
    info!(%device_id, %viewer, "lookup");
    if state.relay.is_none() {
        warn!("relay disabled; connection works only if NAT traversal succeeds");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_and_addrs() {
        assert!(valid_device_id("123456789"));
        assert!(!valid_device_id("012345678"));
        assert!(!valid_device_id("12345678a"));
        let a = sanitize_local_addrs(&["192.168.1.5:4000".into(), "8.8.8.8:1".into(), "10.0.0.1:0".into(), "x".into()]);
        assert_eq!(a, vec!["192.168.1.5:4000".parse().unwrap()]);
    }
}
