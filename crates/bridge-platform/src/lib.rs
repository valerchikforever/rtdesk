//! Platform layer. `unsafe` FFI is concentrated here (see `docs/05 - Engineering/Coding Standards.md`).
//! MVP: Windows only (GDI capture, SendInput, MessageBox UI). DXGI/WGC capture, macOS and Linux follow
//! in Beta per the roadmap.

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("not supported on this platform in the MVP")]
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
    /// 1 = left, 2 = right, 3 = middle.
    MouseButton { button: u32, down: bool },
    /// WHEEL_DELTA units.
    Wheel { dx: i32, dy: i32 },
    /// Set-1 scancode, 0xE0xx for extended keys.
    Key { scancode: u32, down: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Consent {
    FullControl,
    ViewOnly,
    Reject,
}

#[cfg(windows)]
mod windows_impl;
#[cfg(windows)]
pub use windows_impl::*;

#[cfg(not(windows))]
mod unsupported;
#[cfg(not(windows))]
pub use unsupported::*;
