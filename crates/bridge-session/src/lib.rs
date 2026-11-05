//! Host-side session logic, kept free of I/O so it can be exhaustively tested.
//!
//! SEC: M-52 — every inbound control message passes [`HostSession::admit`]; anything not valid
//! in the current phase or not covered by granted permissions terminates the session.
#![forbid(unsafe_code)]

mod limiter;

pub use limiter::{AuthLimiter, FailureOutcome};

use bridge_proto::msg::control::Kind;
use bridge_proto::PROTOCOL_VERSION;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Permissions(pub u32);

impl Permissions {
    pub const NONE: Self = Self(0);
    pub const VIEW: Self = Self(1);
    pub const CONTROL: Self = Self(1 << 1);
    pub const CLIPBOARD: Self = Self(1 << 2);
    pub const ALL: Self = Self(Self::VIEW.0 | Self::CONTROL.0 | Self::CLIPBOARD.0);

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Drop unknown bits (forward compatibility must never widen access).
    pub const fn sanitize(bits: u32) -> Self {
        Self(bits & Self::ALL.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    AwaitHello,
    AwaitPake,
    AwaitConfirm,
    AwaitConsent,
    Active,
    Closed,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Violation {
    #[error("unexpected {msg} in phase {phase:?}")]
    Unexpected { msg: &'static str, phase: Phase },
    #[error("{0} not permitted in this session")]
    NotPermitted(&'static str),
    #[error("unsupported protocol version {0}")]
    Version(u32),
    #[error("message too large: {0}")]
    TooLarge(&'static str),
    #[error("invalid value in {0}")]
    InvalidValue(&'static str),
}

/// Largest accepted relative mouse step (raw counts per message).
pub const MAX_RELATIVE_DELTA: u32 = 10_000;

fn validate_gamepad(g: &bridge_proto::msg::GamepadState) -> Result<(), Violation> {
    use bridge_proto::gamepad::{ALL_BUTTONS, MAX_PADS};
    let stick = |v: i32| (-32_768..=32_767).contains(&v);
    let valid = g.index < MAX_PADS
        && g.buttons & !ALL_BUTTONS == 0
        && g.left_trigger <= 255
        && g.right_trigger <= 255
        && stick(g.left_x)
        && stick(g.left_y)
        && stick(g.right_x)
        && stick(g.right_y);
    if valid {
        Ok(())
    } else {
        Err(Violation::InvalidValue("Gamepad"))
    }
}

#[derive(Debug)]
pub struct HostSession {
    phase: Phase,
    perms: Permissions,
}

impl Default for HostSession {
    fn default() -> Self {
        Self::new()
    }
}

impl HostSession {
    pub fn new() -> Self {
        Self { phase: Phase::AwaitHello, perms: Permissions::NONE }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn permissions(&self) -> Permissions {
        self.perms
    }

    /// Validate an inbound message against the state machine and permissions.
    /// On error the session is closed and the caller must drop the connection.
    pub fn admit(&mut self, kind: &Kind) -> Result<(), Violation> {
        let result = self.check(kind);
        if result.is_err() {
            self.phase = Phase::Closed;
        }
        result
    }

    fn check(&mut self, kind: &Kind) -> Result<(), Violation> {
        use Phase::*;
        match (self.phase, kind) {
            (Closed, k) => Err(Violation::Unexpected { msg: k.name(), phase: Closed }),
            (_, Kind::Bye(_)) => {
                self.phase = Closed;
                Ok(())
            }
            (AwaitHello, Kind::Hello(h)) => {
                if h.version != PROTOCOL_VERSION {
                    return Err(Violation::Version(h.version));
                }
                self.phase = AwaitPake;
                Ok(())
            }
            (AwaitPake, Kind::PakeMsg(m)) => {
                if m.msg.len() > 64 {
                    return Err(Violation::TooLarge("PakeMsg"));
                }
                self.phase = AwaitConfirm;
                Ok(())
            }
            (AwaitConfirm, Kind::PakeConfirm(c)) => {
                if c.mac.len() != 32 {
                    return Err(Violation::InvalidValue("PakeConfirm"));
                }
                self.phase = AwaitConsent;
                Ok(())
            }
            (Active, Kind::MouseMove(_) | Kind::MouseWheel(_)) => self.require(Permissions::CONTROL, kind),
            (Active, Kind::MouseButton(b)) => {
                self.require(Permissions::CONTROL, kind)?;
                if !(1..=3).contains(&b.button) {
                    return Err(Violation::InvalidValue("MouseButton"));
                }
                Ok(())
            }
            (Active, Kind::Key(k)) => {
                self.require(Permissions::CONTROL, kind)?;
                // set-1 scancode, optionally with 0xE0 / 0xE1 prefix
                let prefix = k.scancode >> 8;
                if k.scancode & 0xFF == 0 || !(prefix == 0 || prefix == 0xE0 || prefix == 0xE1) {
                    return Err(Violation::InvalidValue("Key"));
                }
                Ok(())
            }
            (Active, Kind::Clipboard(c)) => {
                self.require(Permissions::CLIPBOARD, kind)?;
                if c.text.len() > bridge_proto::framing::MAX_CLIPBOARD_TEXT {
                    return Err(Violation::TooLarge("Clipboard"));
                }
                Ok(())
            }
            (Active, Kind::MouseMoveRelative(m)) => {
                self.require(Permissions::CONTROL, kind)?;
                if m.dx.unsigned_abs() > MAX_RELATIVE_DELTA || m.dy.unsigned_abs() > MAX_RELATIVE_DELTA {
                    return Err(Violation::InvalidValue("MouseMoveRelative"));
                }
                Ok(())
            }
            (Active, Kind::Gamepad(g)) => {
                self.require(Permissions::CONTROL, kind)?;
                validate_gamepad(g)
            }
            (phase, k) => Err(Violation::Unexpected { msg: k.name(), phase }),
        }
    }

    fn require(&self, p: Permissions, kind: &Kind) -> Result<(), Violation> {
        if self.perms.contains(p) {
            Ok(())
        } else {
            Err(Violation::NotPermitted(kind.name()))
        }
    }

    /// Local user (or policy) granted access. Only valid right after successful key confirmation.
    pub fn grant(&mut self, perms: Permissions) -> Result<(), Violation> {
        if self.phase != Phase::AwaitConsent {
            let phase = self.phase;
            self.phase = Phase::Closed;
            return Err(Violation::Unexpected { msg: "grant", phase });
        }
        self.perms = Permissions::sanitize(perms.0).union(Permissions::VIEW);
        self.phase = Phase::Active;
        Ok(())
    }

    /// Host user downgrades permissions during a session (never upgrades without new consent).
    pub fn restrict(&mut self, keep: Permissions) {
        self.perms = Permissions(self.perms.0 & keep.0).union(Permissions::VIEW);
    }

    pub fn close(&mut self) {
        self.phase = Phase::Closed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bridge_proto::msg::*;

    fn hello() -> Kind {
        Kind::Hello(Hello { version: PROTOCOL_VERSION, agent: "t".into() })
    }
    fn pake() -> Kind {
        Kind::PakeMsg(PakeMsg { msg: vec![0; 33] })
    }
    fn confirm() -> Kind {
        Kind::PakeConfirm(PakeConfirm { mac: vec![0; 32] })
    }
    fn mouse() -> Kind {
        Kind::MouseMove(MouseMove { x: 1, y: 1 })
    }
    fn clip() -> Kind {
        Kind::Clipboard(ClipboardText { text: "x".into() })
    }

    fn active(perms: Permissions) -> HostSession {
        let mut s = HostSession::new();
        s.admit(&hello()).unwrap();
        s.admit(&pake()).unwrap();
        s.admit(&confirm()).unwrap();
        s.grant(perms).unwrap();
        s
    }

    #[test]
    fn input_before_auth_is_rejected() {
        for early in [mouse(), clip(), Kind::Key(KeyEvent { scancode: 0x1E, down: true })] {
            let mut s = HostSession::new();
            assert!(s.admit(&early).is_err());
            assert_eq!(s.phase(), Phase::Closed);
        }
        let mut s = HostSession::new();
        s.admit(&hello()).unwrap();
        s.admit(&pake()).unwrap();
        s.admit(&confirm()).unwrap();
        // Authenticated but not yet consented.
        assert!(s.admit(&mouse()).is_err());
    }

    #[test]
    fn grant_only_after_confirmation() {
        let mut s = HostSession::new();
        assert!(s.grant(Permissions::ALL).is_err());
        let mut s = HostSession::new();
        s.admit(&hello()).unwrap();
        s.admit(&pake()).unwrap();
        assert!(s.grant(Permissions::ALL).is_err());
    }

    #[test]
    fn view_only_blocks_input_and_clipboard() {
        let mut s = active(Permissions::VIEW);
        assert_eq!(s.admit(&mouse()), Err(Violation::NotPermitted("MouseMove")));
        let mut s = active(Permissions::VIEW);
        assert!(s.admit(&clip()).is_err());
    }

    #[test]
    fn full_control_allows_input() {
        let mut s = active(Permissions::ALL);
        s.admit(&mouse()).unwrap();
        s.admit(&clip()).unwrap();
        s.admit(&Kind::Key(KeyEvent { scancode: 0xE04B, down: true })).unwrap();
        s.admit(&Kind::MouseButton(MouseButton { button: 1, down: true })).unwrap();
    }

    #[test]
    fn invalid_values_rejected() {
        let mut s = active(Permissions::ALL);
        assert!(s.admit(&Kind::MouseButton(MouseButton { button: 9, down: true })).is_err());
        let mut s = active(Permissions::ALL);
        assert!(s.admit(&Kind::Key(KeyEvent { scancode: 0x1234_5678, down: true })).is_err());
    }

    #[test]
    fn host_side_messages_from_viewer_are_violations() {
        let mut s = active(Permissions::ALL);
        assert!(s.admit(&Kind::Granted(SessionGranted { permissions: 7, width: 1, height: 1, features: 0 })).is_err());
    }

    #[test]
    fn wrong_version_and_replayed_hello() {
        let mut s = HostSession::new();
        assert_eq!(s.admit(&Kind::Hello(Hello { version: 99, agent: String::new() })), Err(Violation::Version(99)));
        let mut s = active(Permissions::ALL);
        assert!(s.admit(&hello()).is_err());
    }

    #[test]
    fn unknown_permission_bits_dropped_and_restrict_never_widens() {
        let mut s = active(Permissions(0xFFFF_FFFF));
        assert_eq!(s.permissions(), Permissions::ALL);
        s.restrict(Permissions::VIEW);
        assert_eq!(s.permissions(), Permissions::VIEW);
        s.restrict(Permissions::ALL);
        assert_eq!(s.permissions(), Permissions::VIEW);
    }

    #[test]
    fn relative_mouse_and_gamepad_need_control() {
        let rel = Kind::MouseMoveRelative(MouseMoveRelative { dx: 5, dy: -3 });
        let pad = Kind::Gamepad(GamepadState {
            index: 1,
            connected: true,
            buttons: 0x1000,
            left_x: -32768,
            ..Default::default()
        });
        let mut s = active(Permissions::ALL);
        s.admit(&rel).unwrap();
        s.admit(&pad).unwrap();
        for k in [rel.clone(), pad.clone()] {
            let mut s = active(Permissions::VIEW);
            assert!(matches!(s.admit(&k), Err(Violation::NotPermitted(_))));
        }
        // Not before consent either.
        let mut s = HostSession::new();
        assert!(s.admit(&pad).is_err());
    }

    #[test]
    fn invalid_relative_and_gamepad_values() {
        let bad = [
            Kind::MouseMoveRelative(MouseMoveRelative { dx: 1_000_000, dy: 0 }),
            Kind::MouseMoveRelative(MouseMoveRelative { dx: 0, dy: i32::MIN }),
            Kind::Gamepad(GamepadState { index: 4, connected: true, ..Default::default() }),
            Kind::Gamepad(GamepadState { index: 0, buttons: 0x0800, ..Default::default() }),
            Kind::Gamepad(GamepadState { index: 0, left_trigger: 256, ..Default::default() }),
            Kind::Gamepad(GamepadState { index: 0, right_y: 40_000, ..Default::default() }),
        ];
        for k in bad {
            let mut s = active(Permissions::ALL);
            assert!(s.admit(&k).is_err(), "{k:?}");
        }
    }

    #[test]
    fn cursor_state_from_viewer_is_a_violation() {
        let mut s = active(Permissions::ALL);
        assert!(s.admit(&Kind::CursorState(CursorState { visible: false })).is_err());
    }

    #[test]
    fn closed_session_rejects_everything() {
        let mut s = active(Permissions::ALL);
        s.admit(&Kind::Bye(Bye { reason: String::new() })).unwrap();
        assert!(s.admit(&mouse()).is_err());
    }
}
