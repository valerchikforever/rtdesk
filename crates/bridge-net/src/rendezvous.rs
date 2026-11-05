//! Rendezvous client: host registration and viewer lookup.

use std::path::Path;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use bridge_crypto::keys::{self, DeviceKey};
use bridge_crypto::tls::ServerKeyPolicy;
use bridge_proto::framing::{read_msg, write_msg, MAX_RENDEZVOUS};
use bridge_proto::msg::{
    register_transcript, rv_request, rv_response, Incoming, Lookup, LookupResult, Register, RegisterStart, Registered,
    RvRequest, RvResponse,
};
use quinn::{Connection, Endpoint, RecvStream, SendStream};
use tracing::warn;

use crate::config::{ClientConfig, KnownKeys, DEFAULT_PORT_RENDEZVOUS};
use crate::endpoint::{peer_key, rendezvous_client_config, resolve};

const TIMEOUT: Duration = Duration::from_secs(10);

/// Connect to the configured rendezvous server, enforcing its pinned key
/// (explicit `server_key`, else known_servers.json, else trust-on-first-use + pin).
pub async fn connect(endpoint: &Endpoint, dir: &Path, cfg: &ClientConfig) -> Result<Connection> {
    let server = cfg.server.as_deref().context("no rendezvous server configured — pass --server <host[:port]>")?;
    let server = crate::config::with_default_port(server, DEFAULT_PORT_RENDEZVOUS);
    let addr = resolve(&server).await?;
    let mut known = KnownKeys::load(dir.join("known_servers.json"))?;

    let policy = match (&cfg.server_key, known.get(&server)) {
        (Some(k), _) => ServerKeyPolicy::Pinned(keys::decode_public(k).context("invalid --server-key")?),
        (None, Some(k)) => ServerKeyPolicy::Pinned(k),
        (None, None) => ServerKeyPolicy::TrustOnFirstUse,
    };
    let connecting = endpoint
        .connect_with(rendezvous_client_config(policy)?, addr, "bridge-rendezvous")
        .context("starting rendezvous connection")?;
    let conn = tokio::time::timeout(TIMEOUT, connecting)
        .await
        .map_err(|_| anyhow!("rendezvous server {server} did not answer"))?
        .map_err(|e| match policy {
            ServerKeyPolicy::Pinned(_) => anyhow!(
                "rendezvous handshake failed: {e}. If the server key changed legitimately, remove it from known_servers.json"
            ),
            ServerKeyPolicy::TrustOnFirstUse => anyhow!("rendezvous handshake failed: {e}"),
        })?;

    if matches!(policy, ServerKeyPolicy::TrustOnFirstUse) {
        let key = peer_key(&conn).context("server presented no Ed25519 key")?;
        known.insert(&server, &key)?;
        warn!(
            fingerprint = %keys::fingerprint(&key),
            "pinned rendezvous server key on first use — compare the fingerprint with the server's console output"
        );
    }
    Ok(conn)
}

async fn request(send: &mut SendStream, req: rv_request::Kind) -> Result<()> {
    write_msg(send, &RvRequest { kind: Some(req) }).await?;
    Ok(())
}

async fn response(recv: &mut RecvStream) -> Result<rv_response::Kind> {
    let resp: RvResponse = tokio::time::timeout(TIMEOUT, read_msg(recv, MAX_RENDEZVOUS))
        .await
        .map_err(|_| anyhow!("rendezvous timeout"))??;
    match resp.kind {
        Some(rv_response::Kind::Error(e)) => bail!("rendezvous: {}", e.message),
        Some(k) => Ok(k),
        None => bail!("empty rendezvous response"),
    }
}

/// Long-lived host registration. Keep `send` alive; read [`next_incoming`] from `recv`.
pub struct HostRegistration {
    pub registered: Registered,
    pub send: SendStream,
    pub recv: RecvStream,
}

pub async fn register_host(
    conn: &Connection,
    key: &DeviceKey,
    wanted_id: Option<&str>,
    local_addrs: Vec<String>,
) -> Result<HostRegistration> {
    let (mut send, mut recv) = conn.open_bi().await?;
    request(&mut send, rv_request::Kind::RegisterStart(RegisterStart {})).await?;
    let nonce = match response(&mut recv).await? {
        rv_response::Kind::Challenge(c) if c.nonce.len() == 32 => c.nonce,
        _ => bail!("unexpected rendezvous response (expected challenge)"),
    };
    let pubkey = key.public();
    let device_id = wanted_id.unwrap_or_default().to_string();
    let signature = key.sign(&register_transcript(&nonce, &pubkey, &device_id)).to_vec();
    request(
        &mut send,
        rv_request::Kind::Register(Register { pubkey: pubkey.to_vec(), device_id, signature, local_addrs }),
    )
    .await?;
    let registered = match response(&mut recv).await? {
        rv_response::Kind::Registered(r) => r,
        _ => bail!("unexpected rendezvous response (expected registered)"),
    };
    Ok(HostRegistration { registered, send, recv })
}

/// Wait for the next incoming-connection notice (no timeout: the stream is long-lived).
pub async fn next_incoming(recv: &mut RecvStream) -> Result<Incoming> {
    let resp: RvResponse = read_msg(recv, MAX_RENDEZVOUS).await?;
    match resp.kind {
        Some(rv_response::Kind::Incoming(i)) => Ok(i),
        Some(rv_response::Kind::Error(e)) => bail!("rendezvous: {}", e.message),
        _ => bail!("unexpected message on registration stream"),
    }
}

pub async fn lookup(conn: &Connection, device_id: &str) -> Result<LookupResult> {
    let (mut send, mut recv) = conn.open_bi().await?;
    request(&mut send, rv_request::Kind::Lookup(Lookup { device_id: device_id.to_string() })).await?;
    let _ = send.finish();
    match response(&mut recv).await? {
        rv_response::Kind::LookupResult(r) => Ok(r),
        _ => bail!("unexpected rendezvous response (expected lookup result)"),
    }
}
