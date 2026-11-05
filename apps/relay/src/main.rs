//! bridge-relay — see `docs/02 - Architecture/Components/Relay Server.md` and ADR-005.
//!
//! Security properties:
//! - Not an open proxy: a peer must present an HMAC token issued by the rendezvous server.
//! - Blind: payloads are end-to-end encrypted QUIC packets; the relay cannot read or forge them.
//! - No amplification: one packet in → at most one packet of the same size out; nothing is ever
//!   sent to an address that has not bound with a valid token.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use bridge_crypto::relay_token;
use bridge_proto::relay::{self, other_role, Packet, SessionId, MAX_PACKET, ROLE_HOST};
use clap::Parser;
use tokio::net::UdpSocket;
use tracing::{debug, info};

const SESSION_IDLE: Duration = Duration::from_secs(90);
const MAX_SESSIONS: usize = 20_000;

#[derive(Parser, Debug)]
#[command(name = "bridge-relay", version, about = "Bridge blind relay (free & open source, AGPL-3.0)")]
struct Args {
    #[arg(long, default_value = "0.0.0.0:21117")]
    listen: SocketAddr,
    /// Shared secret written by bridge-rendezvous (32 bytes).
    #[arg(long, default_value = "data/relay.secret")]
    secret_file: PathBuf,
}

#[derive(Default)]
struct Session {
    host: Option<SocketAddr>,
    viewer: Option<SocketAddr>,
    last: Option<Instant>,
    bytes: u64,
}

impl Session {
    fn slot(&mut self, role: u8) -> &mut Option<SocketAddr> {
        if role == ROLE_HOST {
            &mut self.host
        } else {
            &mut self.viewer
        }
    }
    fn addr(&self, role: u8) -> Option<SocketAddr> {
        if role == ROLE_HOST {
            self.host
        } else {
            self.viewer
        }
    }
}

struct Relay {
    secret: [u8; 32],
    sessions: HashMap<SessionId, Session>,
}

enum Action {
    None,
    Forward(SocketAddr),
}

impl Relay {
    fn handle(&mut self, pkt: Packet<'_>, from: SocketAddr, now: Instant, unix: u64) -> Action {
        match pkt {
            Packet::Bind { session_id, role, expiry, token } => {
                if !relay_token::verify(&self.secret, &session_id, role, expiry, &token, unix) {
                    debug!(%from, "invalid or expired relay token");
                    return Action::None;
                }
                if !self.sessions.contains_key(&session_id) && self.sessions.len() >= MAX_SESSIONS {
                    return Action::None;
                }
                let s = self.sessions.entry(session_id).or_default();
                if s.slot(role).replace(from) != Some(from) {
                    info!(%from, role, paired = s.host.is_some() && s.viewer.is_some(), "peer bound");
                }
                s.last = Some(now);
                Action::None
            }
            Packet::Data { session_id, role, payload } => {
                let Some(s) = self.sessions.get_mut(&session_id) else { return Action::None };
                // Source must be the address that bound this role with a valid token.
                if s.addr(role) != Some(from) {
                    return Action::None;
                }
                let Some(dst) = s.addr(other_role(role)) else { return Action::None };
                s.last = Some(now);
                s.bytes += payload.len() as u64;
                Action::Forward(dst)
            }
        }
    }

    fn cleanup(&mut self, now: Instant) {
        self.sessions.retain(|_, s| s.last.is_some_and(|t| now.duration_since(t) < SESSION_IDLE));
    }
}

fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let args = Args::parse();
    let secret: [u8; 32] = std::fs::read(&args.secret_file)
        .with_context(|| {
            format!(
                "reading {} — start bridge-rendezvous first (it creates the secret) or copy it here",
                args.secret_file.display()
            )
        })?
        .as_slice()
        .try_into()
        .context("relay secret must be 32 bytes")?;

    let sock = UdpSocket::bind(args.listen).await?;
    println!("Bridge relay listening on udp/{}", args.listen);
    let mut relay = Relay { secret, sessions: HashMap::new() };
    let mut buf = vec![0u8; MAX_PACKET + 1];
    let mut cleanup = tokio::time::interval(Duration::from_secs(10));

    loop {
        tokio::select! {
            r = sock.recv_from(&mut buf) => {
                let Ok((n, from)) = r else { continue };
                let Some(pkt) = relay::parse(&buf[..n]) else { continue };
                if let Action::Forward(dst) = relay.handle(pkt, from, Instant::now(), unix_now()) {
                    let _ = sock.send_to(&buf[..n], dst).await;
                }
            }
            _ = cleanup.tick() => {
                relay.cleanup(Instant::now());
                debug!(sessions = relay.sessions.len(), "cleanup");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bridge_proto::relay::ROLE_VIEWER;

    fn bind(r: &mut Relay, sid: SessionId, role: u8, from: SocketAddr, token: [u8; 32]) {
        let pkt = relay::encode_bind(&sid, role, 100, &token);
        r.handle(relay::parse(&pkt).unwrap(), from, Instant::now(), 50);
    }

    #[test]
    fn pairs_and_forwards_only_authorized() {
        let secret = [1u8; 32];
        let mut r = Relay { secret, sessions: HashMap::new() };
        let sid = [2u8; 16];
        let h: SocketAddr = "1.1.1.1:1".parse().unwrap();
        let v: SocketAddr = "2.2.2.2:2".parse().unwrap();
        let evil: SocketAddr = "6.6.6.6:6".parse().unwrap();

        // Forged token is ignored.
        bind(&mut r, sid, ROLE_HOST, evil, [0; 32]);
        assert!(r.sessions.is_empty());

        bind(&mut r, sid, ROLE_HOST, h, relay_token::issue(&secret, &sid, ROLE_HOST, 100));
        bind(&mut r, sid, ROLE_VIEWER, v, relay_token::issue(&secret, &sid, ROLE_VIEWER, 100));

        let mut out = Vec::new();
        relay::encode_data(&sid, ROLE_VIEWER, b"x", &mut out);
        assert!(matches!(r.handle(relay::parse(&out).unwrap(), v, Instant::now(), 50), Action::Forward(a) if a == h));
        // Spoofed sender for the viewer role is dropped.
        assert!(matches!(r.handle(relay::parse(&out).unwrap(), evil, Instant::now(), 50), Action::None));
        // Viewer token cannot be used for the host role.
        bind(&mut r, sid, ROLE_HOST, evil, relay_token::issue(&secret, &sid, ROLE_VIEWER, 100));
        assert_eq!(r.sessions[&sid].host, Some(h));
    }
}
