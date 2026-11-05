//! Protocol messages. Hand-written `prost` derives (no `protoc` needed at build time).
//! Tags are part of the wire format: never renumber, only add.

use prost::Message;

// ───────────────────────────── Session: viewer <-> host ─────────────────────────────

#[derive(Clone, PartialEq, Message)]
pub struct Hello {
    #[prost(uint32, tag = "1")]
    pub version: u32,
    #[prost(string, tag = "2")]
    pub agent: String,
}

/// SPAKE2 message (one per side).
#[derive(Clone, PartialEq, Message)]
pub struct PakeMsg {
    #[prost(bytes = "vec", tag = "1")]
    pub msg: Vec<u8>,
}

/// Key-confirmation MAC proving both sides derived the same PAKE key.
#[derive(Clone, PartialEq, Message)]
pub struct PakeConfirm {
    #[prost(bytes = "vec", tag = "1")]
    pub mac: Vec<u8>,
}

#[derive(Clone, PartialEq, Message)]
pub struct SessionGranted {
    #[prost(uint32, tag = "1")]
    pub permissions: u32,
    #[prost(uint32, tag = "2")]
    pub width: u32,
    #[prost(uint32, tag = "3")]
    pub height: u32,
    /// Optional host capabilities, see [`crate::features`].
    #[prost(uint32, tag = "4")]
    pub features: u32,
}

#[derive(Clone, PartialEq, Message)]
pub struct SessionDenied {
    #[prost(string, tag = "1")]
    pub reason: String,
}

/// Absolute pointer position in remote framebuffer pixels.
#[derive(Clone, PartialEq, Message)]
pub struct MouseMove {
    #[prost(int32, tag = "1")]
    pub x: i32,
    #[prost(int32, tag = "2")]
    pub y: i32,
}

#[derive(Clone, PartialEq, Message)]
pub struct MouseButton {
    #[prost(uint32, tag = "1")]
    pub button: u32,
    #[prost(bool, tag = "2")]
    pub down: bool,
}

/// Wheel delta in Windows WHEEL_DELTA units (120 = one notch).
#[derive(Clone, PartialEq, Message)]
pub struct MouseWheel {
    #[prost(int32, tag = "1")]
    pub dx: i32,
    #[prost(int32, tag = "2")]
    pub dy: i32,
}

/// PC/AT set-1 scancode; extended keys carry the 0xE0 prefix in bits 8..16 (e.g. 0xE04B).
#[derive(Clone, PartialEq, Message)]
pub struct KeyEvent {
    #[prost(uint32, tag = "1")]
    pub scancode: u32,
    #[prost(bool, tag = "2")]
    pub down: bool,
}

#[derive(Clone, PartialEq, Message)]
pub struct ClipboardText {
    #[prost(string, tag = "1")]
    pub text: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct Bye {
    #[prost(string, tag = "1")]
    pub reason: String,
}

/// Relative pointer motion in raw mouse counts (pointer-lock mode for games / 3D apps).
#[derive(Clone, PartialEq, Message)]
pub struct MouseMoveRelative {
    #[prost(int32, tag = "1")]
    pub dx: i32,
    #[prost(int32, tag = "2")]
    pub dy: i32,
}

/// Host → viewer: whether the remote system cursor is visible. Games and 3D viewports hide it
/// while they capture the mouse; the viewer then switches to relative mode automatically.
#[derive(Clone, PartialEq, Message)]
pub struct CursorState {
    #[prost(bool, tag = "1")]
    pub visible: bool,
}

/// Full state of one gamepad in XInput layout (sent on every change).
#[derive(Clone, PartialEq, Eq, Message)]
pub struct GamepadState {
    /// Pad slot 0..=3.
    #[prost(uint32, tag = "1")]
    pub index: u32,
    /// `false` unplugs the virtual pad on the host.
    #[prost(bool, tag = "2")]
    pub connected: bool,
    /// XInput button bits, see [`crate::gamepad`].
    #[prost(uint32, tag = "3")]
    pub buttons: u32,
    /// 0..=255
    #[prost(uint32, tag = "4")]
    pub left_trigger: u32,
    /// 0..=255
    #[prost(uint32, tag = "5")]
    pub right_trigger: u32,
    /// -32768..=32767, +Y is up.
    #[prost(sint32, tag = "6")]
    pub left_x: i32,
    #[prost(sint32, tag = "7")]
    pub left_y: i32,
    #[prost(sint32, tag = "8")]
    pub right_x: i32,
    #[prost(sint32, tag = "9")]
    pub right_y: i32,
}

impl GamepadState {
    /// All buttons released, sticks centred — sent on focus loss so nothing stays pressed.
    pub fn neutral(index: u32) -> Self {
        Self { index, connected: true, ..Default::default() }
    }
}

/// Envelope for everything on the reliable control stream.
#[derive(Clone, PartialEq, Message)]
pub struct Control {
    #[prost(oneof = "control::Kind", tags = "1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14")]
    pub kind: Option<control::Kind>,
}

pub mod control {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Kind {
        #[prost(message, tag = "1")]
        Hello(super::Hello),
        #[prost(message, tag = "2")]
        PakeMsg(super::PakeMsg),
        #[prost(message, tag = "3")]
        PakeConfirm(super::PakeConfirm),
        #[prost(message, tag = "4")]
        Granted(super::SessionGranted),
        #[prost(message, tag = "5")]
        Denied(super::SessionDenied),
        #[prost(message, tag = "6")]
        MouseMove(super::MouseMove),
        #[prost(message, tag = "7")]
        MouseButton(super::MouseButton),
        #[prost(message, tag = "8")]
        MouseWheel(super::MouseWheel),
        #[prost(message, tag = "9")]
        Key(super::KeyEvent),
        #[prost(message, tag = "10")]
        Clipboard(super::ClipboardText),
        #[prost(message, tag = "11")]
        Bye(super::Bye),
        #[prost(message, tag = "12")]
        MouseMoveRelative(super::MouseMoveRelative),
        #[prost(message, tag = "13")]
        CursorState(super::CursorState),
        #[prost(message, tag = "14")]
        Gamepad(super::GamepadState),
    }

    impl Kind {
        /// Stable name for logs / violations (never includes payload data).
        pub fn name(&self) -> &'static str {
            match self {
                Kind::Hello(_) => "Hello",
                Kind::PakeMsg(_) => "PakeMsg",
                Kind::PakeConfirm(_) => "PakeConfirm",
                Kind::Granted(_) => "SessionGranted",
                Kind::Denied(_) => "SessionDenied",
                Kind::MouseMove(_) => "MouseMove",
                Kind::MouseButton(_) => "MouseButton",
                Kind::MouseWheel(_) => "MouseWheel",
                Kind::Key(_) => "Key",
                Kind::Clipboard(_) => "Clipboard",
                Kind::Bye(_) => "Bye",
                Kind::MouseMoveRelative(_) => "MouseMoveRelative",
                Kind::CursorState(_) => "CursorState",
                Kind::Gamepad(_) => "Gamepad",
            }
        }
    }
}

impl Control {
    pub fn new(kind: control::Kind) -> Self {
        Self { kind: Some(kind) }
    }
}

// ───────────────────────────── Video stream: host -> viewer ─────────────────────────────

/// A JPEG-compressed rectangle of the remote framebuffer.
#[derive(Clone, PartialEq, Message)]
pub struct Tile {
    #[prost(uint32, tag = "1")]
    pub x: u32,
    #[prost(uint32, tag = "2")]
    pub y: u32,
    #[prost(uint32, tag = "3")]
    pub width: u32,
    #[prost(uint32, tag = "4")]
    pub height: u32,
    #[prost(bytes = "vec", tag = "5")]
    pub jpeg: Vec<u8>,
}

#[derive(Clone, PartialEq, Message)]
pub struct FrameUpdate {
    #[prost(uint32, tag = "1")]
    pub width: u32,
    #[prost(uint32, tag = "2")]
    pub height: u32,
    #[prost(message, repeated, tag = "3")]
    pub tiles: Vec<Tile>,
}

// ───────────────────────────── Rendezvous ─────────────────────────────

#[derive(Clone, PartialEq, Message)]
pub struct RegisterStart {}

#[derive(Clone, PartialEq, Message)]
pub struct Register {
    #[prost(bytes = "vec", tag = "1")]
    pub pubkey: Vec<u8>,
    /// Previously assigned ID (empty on first registration).
    #[prost(string, tag = "2")]
    pub device_id: String,
    /// Ed25519 signature over `register_transcript(nonce, pubkey, device_id)`.
    #[prost(bytes = "vec", tag = "3")]
    pub signature: Vec<u8>,
    /// LAN candidates; only disclosed to viewers behind the same public IP.
    #[prost(string, repeated, tag = "4")]
    pub local_addrs: Vec<String>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Lookup {
    #[prost(string, tag = "1")]
    pub device_id: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct RvRequest {
    #[prost(oneof = "rv_request::Kind", tags = "1, 2, 3")]
    pub kind: Option<rv_request::Kind>,
}

pub mod rv_request {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Kind {
        #[prost(message, tag = "1")]
        RegisterStart(super::RegisterStart),
        #[prost(message, tag = "2")]
        Register(super::Register),
        #[prost(message, tag = "3")]
        Lookup(super::Lookup),
    }
}

#[derive(Clone, PartialEq, Message)]
pub struct Challenge {
    #[prost(bytes = "vec", tag = "1")]
    pub nonce: Vec<u8>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Registered {
    #[prost(string, tag = "1")]
    pub device_id: String,
    #[prost(string, tag = "2")]
    pub observed_addr: String,
}

/// Relay allocation handed to one peer.
#[derive(Clone, PartialEq, Message)]
pub struct RelayInfo {
    #[prost(string, tag = "1")]
    pub addr: String,
    #[prost(bytes = "vec", tag = "2")]
    pub session_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub token: Vec<u8>,
    #[prost(uint64, tag = "4")]
    pub expiry: u64,
    #[prost(uint32, tag = "5")]
    pub role: u32,
}

#[derive(Clone, PartialEq, Message)]
pub struct LookupResult {
    #[prost(bool, tag = "1")]
    pub found: bool,
    #[prost(bytes = "vec", tag = "2")]
    pub host_pubkey: Vec<u8>,
    #[prost(string, repeated, tag = "3")]
    pub candidates: Vec<String>,
    #[prost(message, optional, tag = "4")]
    pub relay: Option<RelayInfo>,
    #[prost(string, tag = "5")]
    pub observed_addr: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct Incoming {
    #[prost(string, repeated, tag = "1")]
    pub viewer_candidates: Vec<String>,
    #[prost(message, optional, tag = "2")]
    pub relay: Option<RelayInfo>,
}

#[derive(Clone, PartialEq, Message)]
pub struct RvError {
    #[prost(string, tag = "1")]
    pub message: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct RvResponse {
    #[prost(oneof = "rv_response::Kind", tags = "1, 2, 3, 4, 5")]
    pub kind: Option<rv_response::Kind>,
}

pub mod rv_response {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Kind {
        #[prost(message, tag = "1")]
        Challenge(super::Challenge),
        #[prost(message, tag = "2")]
        Registered(super::Registered),
        #[prost(message, tag = "3")]
        LookupResult(super::LookupResult),
        #[prost(message, tag = "4")]
        Incoming(super::Incoming),
        #[prost(message, tag = "5")]
        Error(super::RvError),
    }
}

/// Bytes signed by a host when registering: binds the server nonce, the key and the ID.
pub fn register_transcript(nonce: &[u8], pubkey: &[u8], device_id: &str) -> Vec<u8> {
    let mut t = Vec::with_capacity(32 + nonce.len() + pubkey.len() + device_id.len());
    t.extend_from_slice(b"bridge-rv-register-v1\0");
    t.extend_from_slice(&(nonce.len() as u32).to_be_bytes());
    t.extend_from_slice(nonce);
    t.extend_from_slice(&(pubkey.len() as u32).to_be_bytes());
    t.extend_from_slice(pubkey);
    t.extend_from_slice(device_id.as_bytes());
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_roundtrip() {
        let m = Control::new(control::Kind::Key(KeyEvent { scancode: 0xE04B, down: true }));
        let bytes = m.encode_to_vec();
        let back = Control::decode(bytes.as_slice()).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn transcript_is_length_prefixed() {
        // Different splits of the same bytes must not collide.
        let a = register_transcript(b"ab", b"c", "1");
        let b = register_transcript(b"a", b"bc", "1");
        assert_ne!(a, b);
    }
}
