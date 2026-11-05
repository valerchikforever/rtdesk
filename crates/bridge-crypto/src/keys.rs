//! Ed25519 device identity keys.

use base64::Engine;
use ring::signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::{CryptoError, Result};

pub type PublicKey = [u8; 32];

/// Long-term device identity. The private key never leaves this struct except
/// as PKCS#8 for protected storage / TLS configuration.
pub struct DeviceKey {
    pkcs8: Zeroizing<Vec<u8>>,
    pair: Ed25519KeyPair,
}

impl std::fmt::Debug for DeviceKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DeviceKey({})", fingerprint(&self.public()))
    }
}

impl DeviceKey {
    pub fn generate() -> Result<Self> {
        let rng = ring::rand::SystemRandom::new();
        let doc = Ed25519KeyPair::generate_pkcs8(&rng).map_err(|_| CryptoError::Rng)?;
        Self::from_pkcs8(doc.as_ref())
    }

    pub fn from_pkcs8(der: &[u8]) -> Result<Self> {
        let pair = Ed25519KeyPair::from_pkcs8(der).map_err(|_| CryptoError::BadKey)?;
        Ok(Self { pkcs8: Zeroizing::new(der.to_vec()), pair })
    }

    pub fn pkcs8(&self) -> &[u8] {
        &self.pkcs8
    }

    pub fn public(&self) -> PublicKey {
        let mut out = [0u8; 32];
        out.copy_from_slice(self.pair.public_key().as_ref());
        out
    }

    pub fn sign(&self, msg: &[u8]) -> [u8; 64] {
        let mut out = [0u8; 64];
        out.copy_from_slice(self.pair.sign(msg).as_ref());
        out
    }
}

pub fn verify(public: &[u8], msg: &[u8], sig: &[u8]) -> bool {
    public.len() == 32 && UnparsedPublicKey::new(&ED25519, public).verify(msg, sig).is_ok()
}

pub fn public_from_slice(bytes: &[u8]) -> Option<PublicKey> {
    bytes.try_into().ok()
}

/// Encoding used in config files and CLI flags.
pub fn encode_public(key: &PublicKey) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(key)
}

pub fn decode_public(s: &str) -> Option<PublicKey> {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(s.trim()).ok()?;
    public_from_slice(&bytes)
}

/// Human-comparable fingerprint: SHA-256 → first 80 bits → Crockford base32, `XXXX-XXXX-XXXX-XXXX`.
pub fn fingerprint(key: &PublicKey) -> String {
    let digest = Sha256::digest(key);
    let b32 = crockford_base32(&digest[..10]);
    b32.as_bytes().chunks(4).map(|c| std::str::from_utf8(c).unwrap_or("????")).collect::<Vec<_>>().join("-")
}

fn crockford_base32(data: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let mut out = String::new();
    let mut buffer: u32 = 0;
    let mut bits = 0;
    for &b in data {
        buffer = (buffer << 8) | u32::from(b);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(ALPHABET[((buffer >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(ALPHABET[((buffer << (5 - bits)) & 31) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_verify_and_reload() {
        let k = DeviceKey::generate().unwrap();
        let sig = k.sign(b"hello");
        assert!(verify(&k.public(), b"hello", &sig));
        assert!(!verify(&k.public(), b"hellO", &sig));
        let k2 = DeviceKey::from_pkcs8(k.pkcs8()).unwrap();
        assert_eq!(k.public(), k2.public());
    }

    #[test]
    fn encodings() {
        let k = DeviceKey::generate().unwrap();
        let p = k.public();
        assert_eq!(decode_public(&encode_public(&p)), Some(p));
        let fp = fingerprint(&p);
        assert_eq!(fp.len(), 19);
        assert_eq!(fp.matches('-').count(), 3);
    }
}
