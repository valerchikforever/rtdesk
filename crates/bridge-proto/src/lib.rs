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

/// Host capability bits in [`msg::SessionGranted::features`].
pub mod features {
    /// Host can inject relative mouse motion.
    pub const RELATIVE_MOUSE: u32 = 1;
    /// Host can create virtual gamepads (ViGEmBus installed on Windows).
    pub const GAMEPAD: u32 = 1 << 1;
}

/// Gamepad layout shared by viewer and host (identical to XInput `XINPUT_GAMEPAD_*`).
pub mod gamepad {
    pub const MAX_PADS: u32 = 4;
    pub const DPAD_UP: u32 = 0x0001;
    pub const DPAD_DOWN: u32 = 0x0002;
    pub const DPAD_LEFT: u32 = 0x0004;
    pub const DPAD_RIGHT: u32 = 0x0008;
    pub const START: u32 = 0x0010;
    pub const BACK: u32 = 0x0020;
    pub const LEFT_THUMB: u32 = 0x0040;
    pub const RIGHT_THUMB: u32 = 0x0080;
    pub const LEFT_SHOULDER: u32 = 0x0100;
    pub const RIGHT_SHOULDER: u32 = 0x0200;
    pub const GUIDE: u32 = 0x0400;
    pub const A: u32 = 0x1000;
    pub const B: u32 = 0x2000;
    pub const X: u32 = 0x4000;
    pub const Y: u32 = 0x8000;
    /// All defined bits; anything else is rejected by the host.
    pub const ALL_BUTTONS: u32 = 0xF7FF;
}

/// Mouse buttons in [`msg::MouseButton::button`].
pub mod button {
    pub const LEFT: u32 = 1;
    pub const RIGHT: u32 = 2;
    pub const MIDDLE: u32 = 3;
}
