//! SPAKE2 (symmetric) over the one-time code, bound to the TLS session.
//!
//! SEC: M-02 — the PAKE identity includes the TLS exporter value and both device keys.
//! A man-in-the-middle (e.g. a malicious rendezvous server) has two different TLS sessions,
//! hence different exporters, and cannot complete the PAKE even if it relays messages.
//! Captured transcripts allow no offline guessing of the code (SEC: M-31).

use hmac::{Hmac, Mac};
use sha2::Sha256;
use spake2::{Ed25519Group, Identity, Password, Spake2};
use zeroize::Zeroizing;

use crate::keys::PublicKey;
use crate::otc;
use crate::{CryptoError, Result};

type HmacSha256 = Hmac<Sha256>;

pub struct PakeContext {
    /// TLS exporter output (`CHANNEL_BINDING_LABEL`), identical on both ends of one TLS session.
    pub channel_binding: [u8; 32],
    pub host_pub: PublicKey,
    pub viewer_pub: PublicKey,
}

impl PakeContext {
    fn transcript(&self) -> Vec<u8> {
        let mut t = Vec::with_capacity(16 + 96);
        t.extend_from_slice(b"bridge-pake-v1\0");
        t.extend_from_slice(&self.channel_binding);
        t.extend_from_slice(&self.host_pub);
        t.extend_from_slice(&self.viewer_pub);
        t
    }
}

pub struct PakeInProgress {
    state: Spake2<Ed25519Group>,
    transcript: Vec<u8>,
}

/// Start the exchange. Returns the state and the message to send to the peer.
pub fn start(secret: &str, ctx: &PakeContext) -> (PakeInProgress, Vec<u8>) {
    let normalized = Zeroizing::new(otc::normalize(secret));
    let transcript = ctx.transcript();
    let (state, msg) =
        Spake2::<Ed25519Group>::start_symmetric(&Password::new(normalized.as_bytes()), &Identity::new(&transcript));
    (PakeInProgress { state, transcript }, msg)
}

impl PakeInProgress {
    pub fn finish(self, inbound: &[u8]) -> Result<SessionSecret> {
        let key = self.state.finish(inbound).map_err(|_| CryptoError::Pake)?;
        Ok(SessionSecret { key: Zeroizing::new(key), transcript: self.transcript })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Host,
    Viewer,
}

impl Role {
    fn label(self) -> &'static [u8] {
        match self {
            Role::Host => b"bridge-confirm-host",
            Role::Viewer => b"bridge-confirm-viewer",
        }
    }
}

/// Shared PAKE output. Only used for key confirmation; traffic keys come from TLS.
pub struct SessionSecret {
    key: Zeroizing<Vec<u8>>,
    transcript: Vec<u8>,
}

impl SessionSecret {
    fn mac(&self, role: Role) -> HmacSha256 {
        let mut mac = HmacSha256::new_from_slice(&self.key).expect("HMAC accepts any key length");
        mac.update(role.label());
        mac.update(&self.transcript);
        mac
    }

    /// MAC that `role` sends to prove knowledge of the key.
    pub fn confirmation(&self, role: Role) -> Vec<u8> {
        self.mac(role).finalize().into_bytes().to_vec()
    }

    /// Constant-time check of the peer's confirmation.
    pub fn verify(&self, role: Role, tag: &[u8]) -> bool {
        self.mac(role).verify_slice(tag).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(cb: u8) -> PakeContext {
        PakeContext { channel_binding: [cb; 32], host_pub: [1; 32], viewer_pub: [2; 32] }
    }

    fn run(code_h: &str, code_v: &str, cb_h: u8, cb_v: u8) -> bool {
        let (sh, mh) = start(code_h, &ctx(cb_h));
        let (sv, mv) = start(code_v, &ctx(cb_v));
        let (Ok(kh), Ok(kv)) = (sh.finish(&mv), sv.finish(&mh)) else { return false };
        kh.verify(Role::Viewer, &kv.confirmation(Role::Viewer)) && kv.verify(Role::Host, &kh.confirmation(Role::Host))
    }

    #[test]
    fn same_code_succeeds() {
        assert!(run("ABC-DEF-GHJ", "abcdefghj", 7, 7));
    }

    #[test]
    fn wrong_code_fails() {
        assert!(!run("ABCDEFGHJ", "ABCDEFGHK", 7, 7));
    }

    #[test]
    fn different_tls_session_fails() {
        // Simulates a MITM terminating TLS separately with each side.
        assert!(!run("ABCDEFGHJ", "ABCDEFGHJ", 7, 8));
    }

    #[test]
    fn reflected_confirmation_is_rejected() {
        let (sh, mh) = start("ABCDEFGHJ", &ctx(1));
        let (sv, mv) = start("ABCDEFGHJ", &ctx(1));
        let kh = sh.finish(&mv).unwrap();
        let _kv = sv.finish(&mh).unwrap();
        // The host's own confirmation must not be accepted as the viewer's.
        assert!(!kh.verify(Role::Viewer, &kh.confirmation(Role::Host)));
    }
}
