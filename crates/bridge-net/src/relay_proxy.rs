//! Local UDP proxy that tunnels QUIC packets through the relay.
//!
//! ```text
//! quinn endpoint ⇄ 127.0.0.1:proxy ⇄ [BRLY header + opaque QUIC packet] ⇄ relay ⇄ peer proxy ⇄ peer endpoint
//! ```
//! QUIC/TLS stays end-to-end; the relay never sees plaintext (ADR-005).

use std::net::SocketAddr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use bridge_proto::msg::RelayInfo;
use bridge_proto::relay::{self, Packet, SessionId, MAX_PACKET};
use tokio::net::UdpSocket;
use tokio::task::JoinHandle;
use tracing::debug;

use crate::endpoint::resolve;

const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
const REBIND_INTERVAL: Duration = Duration::from_secs(5);

pub struct RelayProxy {
    local_addr: SocketAddr,
    task: JoinHandle<()>,
}

impl RelayProxy {
    /// Address the local QUIC endpoint should talk to.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn is_finished(&self) -> bool {
        self.task.is_finished()
    }

    /// Start the proxy. `local_peer`: the local QUIC endpoint (host side, known in advance);
    /// `None` on the viewer side, where it is learned from the first local packet.
    pub async fn start(info: &RelayInfo, local_peer: Option<SocketAddr>) -> Result<Self> {
        let session_id: SessionId = info.session_id.as_slice().try_into().context("bad relay session id")?;
        let token: [u8; 32] = info.token.as_slice().try_into().context("bad relay token")?;
        let role = u8::try_from(info.role).context("bad relay role")?;
        if role != relay::ROLE_HOST && role != relay::ROLE_VIEWER {
            bail!("bad relay role");
        }
        let relay_addr = resolve(&info.addr).await?;

        let local = UdpSocket::bind("127.0.0.1:0").await?;
        let remote = UdpSocket::bind("0.0.0.0:0").await?;
        remote.connect(relay_addr).await?;
        let local_addr = local.local_addr()?;
        let bind = relay::encode_bind(&session_id, role, info.expiry, &token);
        remote.send(&bind).await?;
        let expiry = info.expiry;

        let task = tokio::spawn(async move {
            let mut peer = local_peer;
            let mut from_local = vec![0u8; MAX_PACKET];
            let mut from_relay = vec![0u8; MAX_PACKET];
            let mut out = Vec::with_capacity(MAX_PACKET);
            let mut rebind = tokio::time::interval(REBIND_INTERVAL);
            let idle = tokio::time::sleep(IDLE_TIMEOUT);
            tokio::pin!(idle);
            loop {
                tokio::select! {
                    r = local.recv_from(&mut from_local) => {
                        let Ok((n, from)) = r else { break };
                        if !from.ip().is_loopback() || n + relay::HEADER_LEN > MAX_PACKET {
                            continue;
                        }
                        match peer {
                            None => peer = Some(from),
                            Some(p) if p != from => continue,
                            _ => {}
                        }
                        relay::encode_data(&session_id, role, &from_local[..n], &mut out);
                        let _ = remote.send(&out).await;
                        idle.as_mut().reset(tokio::time::Instant::now() + IDLE_TIMEOUT);
                    }
                    r = remote.recv(&mut from_relay) => {
                        let Ok(n) = r else { break };
                        let Some(Packet::Data { session_id: sid, role: r, payload }) = relay::parse(&from_relay[..n]) else { continue };
                        if sid != session_id || r == role {
                            continue;
                        }
                        if let Some(p) = peer {
                            let _ = local.send_to(payload, p).await;
                        }
                        idle.as_mut().reset(tokio::time::Instant::now() + IDLE_TIMEOUT);
                    }
                    _ = rebind.tick() => {
                        if unix_now() < expiry {
                            let _ = remote.send(&bind).await;
                        }
                    }
                    _ = &mut idle => {
                        debug!("relay proxy idle, stopping");
                        break;
                    }
                }
            }
        });
        Ok(Self { local_addr, task })
    }
}

impl Drop for RelayProxy {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}
