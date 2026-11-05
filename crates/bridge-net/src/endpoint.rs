//! QUIC endpoint and TLS configuration helpers.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use bridge_crypto::keys::{DeviceKey, PublicKey};
use bridge_crypto::tls::{self, ServerKeyPolicy};
use bridge_proto::{ALPN_RENDEZVOUS, ALPN_SESSION};
use quinn::crypto::rustls::{QuicClientConfig, QuicServerConfig};
use quinn::{IdleTimeout, MtuDiscoveryConfig, TransportConfig, VarInt};
use rustls::pki_types::CertificateDer;

pub fn transport_config() -> Arc<TransportConfig> {
    let mut t = TransportConfig::default();
    t.keep_alive_interval(Some(Duration::from_secs(10)));
    t.max_idle_timeout(Some(IdleTimeout::from(VarInt::from_u32(30_000))));
    t.max_concurrent_bidi_streams(VarInt::from_u32(8));
    t.max_concurrent_uni_streams(VarInt::from_u32(8));
    // Leave headroom for the 22-byte relay header within a 1500-byte MTU.
    let mut mtu = MtuDiscoveryConfig::default();
    mtu.upper_bound(1400);
    t.mtu_discovery_config(Some(mtu));
    Arc::new(t)
}

fn server(tls: rustls::ServerConfig) -> Result<quinn::ServerConfig> {
    let crypto = QuicServerConfig::try_from(tls).context("QUIC server crypto")?;
    let mut cfg = quinn::ServerConfig::with_crypto(Arc::new(crypto));
    cfg.transport_config(transport_config());
    cfg.migration(true);
    Ok(cfg)
}

fn client(tls: rustls::ClientConfig) -> Result<quinn::ClientConfig> {
    let crypto = QuicClientConfig::try_from(tls).context("QUIC client crypto")?;
    let mut cfg = quinn::ClientConfig::new(Arc::new(crypto));
    cfg.transport_config(transport_config());
    Ok(cfg)
}

/// Host side: accepts sessions; requires the viewer to present an Ed25519 client key.
pub fn session_server_config(key: &DeviceKey) -> Result<quinn::ServerConfig> {
    server(tls::server_config(key, ALPN_SESSION, true)?)
}

/// Viewer side: authenticates with its device key and accepts only the pinned host key.
pub fn session_client_config(key: &DeviceKey, host: PublicKey) -> Result<quinn::ClientConfig> {
    client(tls::client_config(Some(key), ServerKeyPolicy::Pinned(host), ALPN_SESSION)?)
}

pub fn rendezvous_server_config(key: &DeviceKey) -> Result<quinn::ServerConfig> {
    server(tls::server_config(key, ALPN_RENDEZVOUS, false)?)
}

pub fn rendezvous_client_config(policy: ServerKeyPolicy) -> Result<quinn::ClientConfig> {
    client(tls::client_config(None, policy, ALPN_RENDEZVOUS)?)
}

/// Public key the peer proved possession of in the TLS handshake.
pub fn peer_key(conn: &quinn::Connection) -> Option<PublicKey> {
    let any = conn.peer_identity()?;
    let certs = any.downcast::<Vec<CertificateDer<'static>>>().ok()?;
    tls::ed25519_key_from_cert(certs.first()?)
}

/// 32-byte TLS exporter value for PAKE channel binding.
pub fn channel_binding(conn: &quinn::Connection) -> Result<[u8; 32]> {
    let mut out = [0u8; 32];
    conn.export_keying_material(&mut out, bridge_proto::CHANNEL_BINDING_LABEL, b"")
        .map_err(|_| anyhow::anyhow!("TLS exporter unavailable"))?;
    Ok(out)
}

pub async fn resolve(addr: &str) -> Result<SocketAddr> {
    tokio::net::lookup_host(addr)
        .await
        .with_context(|| format!("resolving {addr}"))?
        .find(SocketAddr::is_ipv4)
        .with_context(|| format!("no IPv4 address for {addr}"))
}

/// Local interface address used to reach `toward` (no packets are sent).
pub fn local_ip_toward(toward: SocketAddr) -> Option<std::net::IpAddr> {
    let sock = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect(toward).ok()?;
    let ip = sock.local_addr().ok()?.ip();
    (!ip.is_unspecified() && !ip.is_loopback()).then_some(ip)
}

/// Send QUIC Initial packets towards `targets` to open NAT mappings (hole punching).
/// The handshakes are expected to fail; only the outbound packets matter.
pub fn punch(endpoint: &quinn::Endpoint, targets: &[SocketAddr]) {
    let Ok(cfg) = rendezvous_client_config(ServerKeyPolicy::Pinned([0; 32])) else { return };
    for &addr in targets {
        if let Ok(connecting) = endpoint.connect_with(cfg.clone(), addr, "punch") {
            tokio::spawn(async move {
                let _ = tokio::time::timeout(Duration::from_secs(3), connecting).await;
            });
        }
    }
}
