//! TLS 1.3 configuration with raw Ed25519 key pinning (no CAs).
//!
//! SEC: M-01 — the viewer accepts the host only if the certificate's key equals the pinned key.
//! The key is extracted with a real X.509 parser; the handshake signature is verified by
//! webpki against the *same* SubjectPublicKeyInfo, so a cert cannot claim one key and sign with another.

use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{verify_tls13_signature, CryptoProvider, WebPkiSupportedAlgorithms};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::{CertificateError, DigitallySignedStruct, DistinguishedName, SignatureScheme};
use subtle::ConstantTimeEq;

use crate::keys::{DeviceKey, PublicKey};
use crate::{CryptoError, Result};

const ED25519_OID: &str = "1.3.101.112";

pub fn provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// Extract the Ed25519 public key from a DER certificate. Rejects anything else.
pub fn ed25519_key_from_cert(der: &[u8]) -> Option<PublicKey> {
    let (rest, cert) = x509_parser::parse_x509_certificate(der).ok()?;
    if !rest.is_empty() {
        return None;
    }
    let spki = cert.public_key();
    if spki.algorithm.algorithm.to_id_string() != ED25519_OID {
        return None;
    }
    spki.subject_public_key.data.as_ref().try_into().ok()
}

/// Self-signed certificate wrapping the device key.
pub fn self_signed(key: &DeviceKey) -> Result<(CertificateDer<'static>, PrivateKeyDer<'static>)> {
    let kp = rcgen::KeyPair::try_from(key.pkcs8()).map_err(|e| CryptoError::Cert(e.to_string()))?;
    let params = rcgen::CertificateParams::new(vec!["bridge.peer".to_string()])
        .map_err(|e| CryptoError::Cert(e.to_string()))?;
    let cert = params.self_signed(&kp).map_err(|e| CryptoError::Cert(e.to_string()))?;
    let der = cert.der().clone();
    let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.pkcs8().to_vec()));
    Ok((der, key_der))
}

/// How a client authenticates the server's key.
#[derive(Clone, Copy, Debug)]
pub enum ServerKeyPolicy {
    /// Accept only this key.
    Pinned(PublicKey),
    /// Accept any Ed25519 key; the caller MUST pin it after the handshake (TOFU).
    TrustOnFirstUse,
}

#[derive(Debug)]
struct PinnedServerVerifier {
    policy: ServerKeyPolicy,
    algs: WebPkiSupportedAlgorithms,
}

impl ServerCertVerifier for PinnedServerVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        let key = ed25519_key_from_cert(end_entity)
            .ok_or(rustls::Error::InvalidCertificate(CertificateError::BadEncoding))?;
        match self.policy {
            ServerKeyPolicy::Pinned(expected) if bool::from(key.ct_eq(&expected)) => Ok(ServerCertVerified::assertion()),
            ServerKeyPolicy::Pinned(_) => Err(rustls::Error::InvalidCertificate(CertificateError::ApplicationVerificationFailure)),
            ServerKeyPolicy::TrustOnFirstUse => Ok(ServerCertVerified::assertion()),
        }
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Err(rustls::Error::General("TLS 1.2 is not supported".into()))
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, dss, &self.algs)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![SignatureScheme::ED25519]
    }
}

/// Requires a client certificate with an Ed25519 key; identity decisions happen in the app
/// (PAKE / pinning), this only guarantees proof of possession of the presented key.
#[derive(Debug)]
struct Ed25519ClientVerifier {
    algs: WebPkiSupportedAlgorithms,
}

impl ClientCertVerifier for Ed25519ClientVerifier {
    fn offer_client_auth(&self) -> bool {
        true
    }

    fn client_auth_mandatory(&self) -> bool {
        true
    }

    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _now: UnixTime,
    ) -> std::result::Result<ClientCertVerified, rustls::Error> {
        ed25519_key_from_cert(end_entity)
            .map(|_| ClientCertVerified::assertion())
            .ok_or(rustls::Error::InvalidCertificate(CertificateError::BadEncoding))
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Err(rustls::Error::General("TLS 1.2 is not supported".into()))
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, dss, &self.algs)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![SignatureScheme::ED25519]
    }
}

pub fn server_config(key: &DeviceKey, alpn: &[u8], require_client_key: bool) -> Result<rustls::ServerConfig> {
    let provider = provider();
    let algs = provider.signature_verification_algorithms;
    let (cert, key_der) = self_signed(key)?;
    let builder = rustls::ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])?;
    let mut cfg = if require_client_key {
        builder
            .with_client_cert_verifier(Arc::new(Ed25519ClientVerifier { algs }))
            .with_single_cert(vec![cert], key_der)?
    } else {
        builder.with_no_client_auth().with_single_cert(vec![cert], key_der)?
    };
    cfg.alpn_protocols = vec![alpn.to_vec()];
    cfg.max_early_data_size = 0; // no 0-RTT: replayable
    Ok(cfg)
}

pub fn client_config(client_key: Option<&DeviceKey>, policy: ServerKeyPolicy, alpn: &[u8]) -> Result<rustls::ClientConfig> {
    let provider = provider();
    let algs = provider.signature_verification_algorithms;
    let builder = rustls::ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(PinnedServerVerifier { policy, algs }));
    let mut cfg = match client_key {
        Some(k) => {
            let (cert, key_der) = self_signed(k)?;
            builder.with_client_auth_cert(vec![cert], key_der)?
        }
        None => builder.with_no_client_auth(),
    };
    cfg.alpn_protocols = vec![alpn.to_vec()];
    cfg.enable_early_data = false;
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cert_carries_device_key() {
        let k = DeviceKey::generate().unwrap();
        let (cert, _) = self_signed(&k).unwrap();
        assert_eq!(ed25519_key_from_cert(&cert), Some(k.public()));
        assert_eq!(ed25519_key_from_cert(b"garbage"), None);
    }

    #[test]
    fn configs_build() {
        let k = DeviceKey::generate().unwrap();
        server_config(&k, b"x", true).unwrap();
        client_config(Some(&k), ServerKeyPolicy::Pinned([0; 32]), b"x").unwrap();
    }
}
