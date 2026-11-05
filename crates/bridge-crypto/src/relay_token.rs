//! Relay allocation tokens: HMAC-SHA256(secret, "bridge-relay-v1" | session_id | role | expiry).
//! Issued by the rendezvous server, verified by the relay (shared secret).

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

fn mac(secret: &[u8], session_id: &[u8; 16], role: u8, expiry: u64) -> HmacSha256 {
    let mut m = HmacSha256::new_from_slice(secret).expect("HMAC accepts any key length");
    m.update(b"bridge-relay-v1");
    m.update(session_id);
    m.update(&[role]);
    m.update(&expiry.to_be_bytes());
    m
}

pub fn issue(secret: &[u8], session_id: &[u8; 16], role: u8, expiry: u64) -> [u8; 32] {
    mac(secret, session_id, role, expiry).finalize().into_bytes().into()
}

pub fn verify(secret: &[u8], session_id: &[u8; 16], role: u8, expiry: u64, token: &[u8], now: u64) -> bool {
    now <= expiry && mac(secret, session_id, role, expiry).verify_slice(token).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_checks() {
        let s = [3u8; 32];
        let sid = [4u8; 16];
        let t = issue(&s, &sid, 1, 100);
        assert!(verify(&s, &sid, 1, 100, &t, 50));
        assert!(!verify(&s, &sid, 1, 100, &t, 101), "expired");
        assert!(!verify(&s, &sid, 2, 100, &t, 50), "role swap");
        assert!(!verify(&s, &sid, 1, 200, &t, 50), "expiry extension");
        assert!(!verify(&[5u8; 32], &sid, 1, 100, &t, 50), "wrong secret");
    }
}
