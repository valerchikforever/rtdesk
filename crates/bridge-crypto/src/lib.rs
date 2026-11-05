//! Cryptography for Bridge. Standard primitives only (see `docs/03 - Security/Cryptography Design.md`):
//! - Ed25519 device identity keys (ring), stored with OS protection (DPAPI on Windows)
//! - TLS 1.3 with self-signed certs carrying the device key, verified by **pinning**
//! - SPAKE2 over the one-time code, bound to the TLS session (channel binding)
//! - HMAC-SHA256 relay tokens

pub mod keys;
pub mod keystore;
pub mod otc;
pub mod pake;
pub mod relay_token;
pub mod tls;

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("random number generator failure")]
    Rng,
    #[error("invalid key material")]
    BadKey,
    #[error("PAKE failed")]
    Pake,
    #[error("certificate generation failed: {0}")]
    Cert(String),
    #[error("TLS configuration failed: {0}")]
    Tls(#[from] rustls::Error),
    #[error("key storage: {0}")]
    Storage(String),
    #[error("i/o: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, CryptoError>;
