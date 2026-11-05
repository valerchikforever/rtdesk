//! Platform layer. `unsafe` FFI is concentrated here (see `docs/05 - Engineering/Coding Standards.md`).
//!
//! | OS | Capture | Input | Dialogs | Gamepads |
//! |---|---|---|---|---|
//! | Windows | GDI BitBlt | SendInput | MessageBox | ViGEmBus |
//! | macOS | CoreGraphics display image | CGEvent | osascript | — |
//! | Linux (X11) | X11 GetImage + XFixes cursor | XTest | zenity / kdialog | — |
//!
//! Every backend exports the same set of items: `init_process`, `ScreenCapturer`, `screen_size`,
//! `inject`, `cursor_visible`, `ask_consent`, `CONSENT_TITLE`, `session_indicator`, `close_dialog`,
//! `gamepad_available`, `VirtualGamepads`.

pub mod keymaps;

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("not supported on this platform yet")]
    Unsupported,
    #[error("{0}")]
    Os(String),
    #[error("invalid input event")]
    InvalidInput,
}

pub type Result<T> = std::result::Result<T, PlatformError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEvent {
    /// Absolute position in captured-frame pixels.
    MouseMove { x: i32, y: i32 },
    /// Relative motion in raw mouse counts (games, 3D viewports).
    MouseMoveRelative { dx: i32, dy: i32 },
    /// 1 = left, 2 = right, 3 = middle.
    MouseButton { button: u32, down: bool },
    /// WHEEL_DELTA units.
    Wheel { dx: i32, dy: i32 },
    /// Set-1 scancode, 0xE0xx for extended keys.
    Key { scancode: u32, down: bool },
}

/// One gamepad's state in XInput layout (validated by the session layer before it gets here).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GamepadInput {
    pub index: usize,
    pub connected: bool,
    pub buttons: u16,
    pub left_trigger: u8,
    pub right_trigger: u8,
    pub left_x: i16,
    pub left_y: i16,
    pub right_x: i16,
    pub right_y: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Consent {
    FullControl,
    ViewOnly,
    Reject,
}

/// Title of the consent dialog (used to close it on timeout / viewer departure).
pub const CONSENT_TITLE: &str = "Bridge — incoming remote access request";

/// Consent text shared by all backends (SEC: SR-05, AS-01).
pub fn consent_text(viewer_fingerprint: &str, remote_addr: &str) -> String {
    format!(
        "Someone wants to connect to this computer.\n\n\
         Viewer key fingerprint:  {viewer_fingerprint}\n\
         Network address:  {remote_addr}\n\n\
         WARNING: never allow access to someone who contacted you unexpectedly \
         (phone call, pop-up, e-mail, social media). Banks, Microsoft, Apple, police and \
         \"technical support\" never ask you for this.\n\n\
         Full control  —  mouse, keyboard, gamepad, clipboard\n\
         View only  —  the other side can only watch\n\
         Reject  —  do not connect"
    )
}

/// Clamp a WHEEL_DELTA value to whole notches, keeping at least one for any non-zero input.
pub fn wheel_notches(delta: i32) -> i32 {
    if delta == 0 {
        return 0;
    }
    let n = (delta.clamp(-1200, 1200) + delta.signum() * 60) / 120;
    if n == 0 {
        delta.signum()
    } else {
        n
    }
}

#[cfg(windows)]
mod windows_gamepad;
#[cfg(windows)]
mod windows_impl;
#[cfg(windows)]
pub use windows_gamepad::*;
#[cfg(windows)]
pub use windows_impl::*;

#[cfg(unix)]
mod unix_dialog;

#[cfg(target_os = "macos")]
mod macos_impl;
#[cfg(target_os = "macos")]
pub use macos_impl::*;

#[cfg(target_os = "linux")]
mod linux_impl;
#[cfg(target_os = "linux")]
pub use linux_impl::*;

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod no_gamepad;
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub use no_gamepad::*;

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
mod unsupported;
#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
pub use unsupported::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wheel() {
        assert_eq!(wheel_notches(0), 0);
        assert_eq!(wheel_notches(120), 1);
        assert_eq!(wheel_notches(-240), -2);
        assert_eq!(wheel_notches(10), 1, "small pixel deltas still scroll");
        assert_eq!(wheel_notches(-10), -1);
        assert_eq!(wheel_notches(1_000_000), 10, "clamped");
    }
}
