//! Bridge wire protocol: message definitions (protobuf encoding via `prost`),
//! length-prefixed framing with hard size limits, and the relay packet format.
//!
//! See `docs/02 - Architecture/Wire Protocol.md`.
#![forbid(unsafe_code)]

pub mod framing;
pub mod msg;
pub mod relay;

/// Session protocol version. Peers with a different version are rejected
/// (no silent downgrade).
pub const PROTOCOL_VERSION: u32 = 1;

/// ALPN for viewer <-> host sessions.
pub const ALPN_SESSION: &[u8] = b"bridge/1";
/// ALPN for clients <-> rendezvous server.
pub const ALPN_RENDEZVOUS: &[u8] = b"bridge-rv/1";

/// TLS exporter label used for channel binding of the PAKE (RFC 5705 / 8446 §7.5).
pub const CHANNEL_BINDING_LABEL: &[u8] = b"EXPORTER-bridge-auth-v1";

/// Mouse buttons in [`msg::MouseButton::button`].
pub mod button {
    pub const LEFT: u32 = 1;
    pub const RIGHT: u32 = 2;
    pub const MIDDLE: u32 = 3;
}
