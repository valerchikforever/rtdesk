//! Rendezvous lookup, path selection (direct → relay), PAKE authentication, host key pinning.

use std::io::Write;
use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use bridge_crypto::keys::{self, DeviceKey};
use bridge_crypto::pake::{self, PakeContext, Role};
use bridge_crypto::{keystore, otc};
use bridge_net::config::{self, ClientConfig, KnownKeys};
use bridge_net::endpoint::{channel_binding, session_client_config};
use bridge_net::relay_proxy::RelayProxy;
use bridge_net::rendezvous;
use bridge_proto::framing::{read_msg, write_msg, MAX_CONTROL};
use bridge_proto::msg::control::Kind;
use bridge_proto::msg::{Control, Hello, PakeConfirm, PakeMsg, SessionGranted};
use bridge_proto::PROTOCOL_VERSION;
use quinn::{Connection, Endpoint, RecvStream, SendStream};
use tokio::task::JoinSet;
use tracing::info;

use crate::Args;

pub struct Session {
    pub conn: Connection,
    pub send: SendStream,
    pub recv: RecvStream,
    pub granted: SessionGranted,
    /// Kept alive for the duration of a relayed session.
    pub _relay: Option<RelayProxy>,
    pub _endpoint: Endpoint,
    pub path: &'static str,
}

async fn send_ctrl(send: &mut SendStream, kind: Kind) -> Result<()> {
    write_msg(send, &Control::new(kind)).await?;
    Ok(())
}

async fn recv_ctrl(recv: &mut RecvStream) -> Result<Kind> {
    let msg: Control = read_msg(recv, MAX_CONTROL).await?;
    msg.kind.context("empty message from host")
}

pub async fn connect(args: &Args, device_id: &str) -> Result<Session> {
    if device_id.len() != 9 {
        bail!("a device ID has 9 digits");
    }
    let dir = args.config_dir.clone().unwrap_or_else(config::default_dir);
    std::fs::create_dir_all(&dir)?;
    let mut cfg = ClientConfig::load(&dir)?;
    if let Some(s) = &args.server {
        cfg.server = Some(s.clone());
    }
    if let Some(k) = &args.server_key {
        keys::decode_public(k).context("--server-key is not a valid key")?;
        cfg.server_key = Some(k.clone());
    }
    cfg.save(&dir)?;

    let key = keystore::load_or_create_device_key(&dir.join("device.key"))?;
    let endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    let rv = rendezvous::connect(&endpoint, &dir, &cfg).await?;

    // Ask for the code before the lookup: relay tokens are short-lived.
    let code = tokio::task::spawn_blocking(|| -> std::io::Result<String> {
        print!("Enter the one-time code shown on the remote computer: ");
        std::io::stdout().flush()?;
        let mut s = String::new();
        std::io::stdin().read_line(&mut s)?;
        Ok(otc::normalize(&s))
    })
    .await??;
    if code.len() != otc::LEN {
        bail!("the code has {} characters", otc::LEN);
    }

    let lookup = rendezvous::lookup(&rv, device_id).await?;
    if !lookup.found {
        bail!("device {device_id} is offline or unknown");
    }
    let host_key = keys::public_from_slice(&lookup.host_pubkey).context("invalid host key from server")?;

    // TOFU check *before* connecting; the pin is stored only after successful authentication.
    let mut known = KnownKeys::load(dir.join("known_hosts.json"))?;
    let pinned = known.get(device_id);
    match pinned {
        Some(k) if k == host_key => {}
        Some(k) if !args.accept_new_host_key => bail!(
            "\n!!! THE KEY OF DEVICE {device_id} HAS CHANGED !!!\n\
             pinned: {}\n\
             now:    {}\n\
             This happens after a reinstall — or during an attack. Ask the remote user to read the\n\
             key fingerprint from their screen; if it matches the new one, re-run with --accept-new-host-key.",
            keys::fingerprint(&k),
            keys::fingerprint(&host_key)
        ),
        _ => println!(
            "Host key fingerprint: {}\n(First connection or accepted change — compare it with the fingerprint shown on the remote screen.)",
            keys::fingerprint(&host_key)
        ),
    }

    let qcfg = session_client_config(&key, host_key)?;
    let mut candidates: Vec<SocketAddr> = lookup.candidates.iter().filter_map(|a| a.parse().ok()).take(4).collect();
    // Testing aid: skip the direct path. Only affects path choice, never authentication.
    if std::env::var_os("BRIDGE_FORCE_RELAY").is_some() {
        candidates.clear();
    }
    let (conn, relay, path) = match try_direct(&endpoint, &qcfg, &candidates).await {
        Some(c) => (c, None, "direct (peer-to-peer)"),
        None => {
            let info = lookup.relay.as_ref().context("direct connection failed and the server offers no relay")?;
            info!("direct connection failed, using relay {}", info.addr);
            let proxy = RelayProxy::start(info, None).await?;
            let connecting = endpoint.connect_with(qcfg, proxy.local_addr(), "bridge-host")?;
            let conn = tokio::time::timeout(Duration::from_secs(10), connecting)
                .await
                .context("relay connection timed out")?
                .context("relay connection failed")?;
            (conn, Some(proxy), "relayed (end-to-end encrypted)")
        }
    };

    let (mut send, mut recv) = conn.open_bi().await?;
    let granted = authenticate(&conn, &mut send, &mut recv, &key, host_key, &code).await?;
    if pinned != Some(host_key) {
        known.insert(device_id, &host_key)?;
    }
    Ok(Session { conn, send, recv, granted, _relay: relay, _endpoint: endpoint, path })
}

async fn try_direct(endpoint: &Endpoint, cfg: &quinn::ClientConfig, candidates: &[SocketAddr]) -> Option<Connection> {
    let mut set = JoinSet::new();
    for &addr in candidates {
        if let Ok(connecting) = endpoint.connect_with(cfg.clone(), addr, "bridge-host") {
            set.spawn(async move { tokio::time::timeout(Duration::from_secs(3), connecting).await });
        }
    }
    while let Some(r) = set.join_next().await {
        if let Ok(Ok(Ok(conn))) = r {
            set.abort_all();
            return Some(conn);
        }
    }
    None
}

async fn authenticate(
    conn: &Connection,
    send: &mut SendStream,
    recv: &mut RecvStream,
    key: &DeviceKey,
    host_key: keys::PublicKey,
    code: &str,
) -> Result<SessionGranted> {
    send_ctrl(
        send,
        Kind::Hello(Hello {
            version: PROTOCOL_VERSION,
            agent: concat!("bridge-viewer/", env!("CARGO_PKG_VERSION")).into(),
        }),
    )
    .await?;
    let pctx = PakeContext { channel_binding: channel_binding(conn)?, host_pub: host_key, viewer_pub: key.public() };
    let (state, out) = pake::start(code, &pctx);
    send_ctrl(send, Kind::PakeMsg(PakeMsg { msg: out })).await?;

    let host_msg = match recv_ctrl(recv).await? {
        Kind::PakeMsg(m) => m,
        Kind::Denied(d) => bail!("access denied: {}", d.reason),
        other => bail!("unexpected {} from host", other.name()),
    };
    let secret = state.finish(&host_msg.msg).map_err(|_| anyhow::anyhow!("authentication failed"))?;
    send_ctrl(send, Kind::PakeConfirm(PakeConfirm { mac: secret.confirmation(Role::Viewer) })).await?;

    match recv_ctrl(recv).await? {
        Kind::PakeConfirm(c) if secret.verify(Role::Host, &c.mac) => {}
        Kind::PakeConfirm(_) => bail!("the host could not prove knowledge of the code — possible man-in-the-middle"),
        Kind::Denied(d) => bail!("access denied: {}", d.reason),
        other => bail!("unexpected {} from host", other.name()),
    }
    println!("Code accepted. Waiting for the remote user to allow the connection…");

    match tokio::time::timeout(Duration::from_secs(120), recv_ctrl(recv))
        .await
        .context("no answer from the remote user")??
    {
        Kind::Granted(g) => Ok(g),
        Kind::Denied(d) => bail!("access denied: {}", d.reason),
        other => bail!("unexpected {} from host", other.name()),
    }
}
