//! macOS host backend.
//!
//! - Capture: CoreGraphics display image of the main display (Screen Recording permission required).
//! - Input: CGEvent posted at the HID level (Accessibility permission required).
//! - Dialogs: osascript.
//!
//! Known limitations (documented): main display only; the cursor is not drawn into the image
//! (the viewer shows its own); `CGDisplayCreateImage` is deprecated — ScreenCaptureKit is the
//! follow-up for better performance and multi-display support.

use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use core_graphics::display::CGDisplay;
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTapLocation, CGEventType, CGMouseButton, EventField, ScrollEventUnit,
};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;

use crate::unix_dialog;
use crate::{consent_text, keymaps, wheel_notches, Consent, InputEvent, PlatformError, Result, CONSENT_TITLE};

pub use crate::unix_dialog::close_dialog;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
    fn CGCursorIsVisible() -> i32;
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> u8;
}

/// Check (and request) the two privacy permissions a host needs.
pub fn init_process() {
    // SAFETY: argument-less CoreGraphics / Accessibility queries.
    unsafe {
        if !CGPreflightScreenCaptureAccess() {
            eprintln!(
                "Screen Recording permission is missing. macOS will ask now; afterwards enable it in\n\
                 System Settings → Privacy & Security → Screen Recording (for the Terminal app you started Bridge from)\n\
                 and restart bridge-host."
            );
            CGRequestScreenCaptureAccess();
        }
        if AXIsProcessTrusted() == 0 {
            eprintln!(
                "Accessibility permission is missing: the viewer will see the screen but cannot control it.\n\
                 Enable it in System Settings → Privacy & Security → Accessibility and restart bridge-host."
            );
        }
    }
}

/// Main display size in physical pixels (= captured image size).
pub fn screen_size() -> (u32, u32) {
    let d = CGDisplay::main();
    match d.display_mode() {
        Some(m) => (m.pixel_width() as u32, m.pixel_height() as u32),
        None => (d.pixels_wide() as u32, d.pixels_high() as u32),
    }
}

pub struct ScreenCapturer {
    display: CGDisplay,
    size: (u32, u32),
}

impl ScreenCapturer {
    pub fn new() -> Result<Self> {
        let size = screen_size();
        if size.0 == 0 || size.1 == 0 {
            return Err(PlatformError::Os("no display found".into()));
        }
        Ok(Self { display: CGDisplay::main(), size })
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    /// Capture into `out` as top-down BGRA.
    pub fn capture(&mut self, out: &mut Vec<u8>) -> Result<()> {
        let image = self.display.image().ok_or_else(|| {
            PlatformError::Os(
                "screen capture failed — allow Screen Recording in System Settings → Privacy & Security".into(),
            )
        })?;
        let (w, h, stride) = (image.width(), image.height(), image.bytes_per_row());
        if image.bits_per_pixel() != 32 || stride < w * 4 {
            return Err(PlatformError::Os("unexpected screen pixel format".into()));
        }
        let data = image.data();
        let bytes = data.bytes();
        if bytes.len() < stride * h {
            return Err(PlatformError::Os("short screen image".into()));
        }
        out.clear();
        out.reserve(w * h * 4);
        for row in 0..h {
            let start = row * stride;
            out.extend_from_slice(&bytes[start..start + w * 4]); // BGRA (32-bit little-endian, alpha first)
        }
        self.size = (w as u32, h as u32);
        Ok(())
    }
}

pub fn cursor_visible() -> bool {
    // SAFETY: argument-less CoreGraphics query.
    unsafe { CGCursorIsVisible() != 0 }
}

// ───────────────────────────── Input ─────────────────────────────

struct InputState {
    flags: CGEventFlags,
    /// bit 0 left, bit 1 right, bit 2 middle
    buttons: u8,
    /// (button, time, click count) of the last press, for double-click detection
    last_click: Option<(u32, Instant, i64)>,
}

fn state() -> &'static Mutex<InputState> {
    static S: OnceLock<Mutex<InputState>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(InputState { flags: CGEventFlags::empty(), buttons: 0, last_click: None }))
}

fn source() -> Result<CGEventSource> {
    CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| PlatformError::Os("cannot create event source (Accessibility permission?)".into()))
}

fn os_err(what: &str) -> PlatformError {
    PlatformError::Os(format!("{what} failed (Accessibility permission?)"))
}

/// Main display bounds in points and the pixel → point scale (Retina = 0.5).
fn mapping() -> (core_graphics::geometry::CGRect, f64) {
    let bounds = CGDisplay::main().bounds();
    let (pw, _) = screen_size();
    let scale = if pw > 0 { bounds.size.width / f64::from(pw) } else { 1.0 };
    (bounds, scale)
}

fn clamp_to(bounds: &core_graphics::geometry::CGRect, p: CGPoint) -> CGPoint {
    CGPoint::new(
        p.x.clamp(bounds.origin.x, bounds.origin.x + bounds.size.width - 1.0),
        p.y.clamp(bounds.origin.y, bounds.origin.y + bounds.size.height - 1.0),
    )
}

fn current_position() -> Option<CGPoint> {
    CGEvent::new(source().ok()?).ok().map(|e| e.location())
}

fn move_to(st: &InputState, point: CGPoint, delta: Option<(i32, i32)>) -> Result<()> {
    // While a button is held macOS expects "dragged" events, otherwise drag & drop breaks.
    let (kind, button) = if st.buttons & 1 != 0 {
        (CGEventType::LeftMouseDragged, CGMouseButton::Left)
    } else if st.buttons & 2 != 0 {
        (CGEventType::RightMouseDragged, CGMouseButton::Right)
    } else if st.buttons & 4 != 0 {
        (CGEventType::OtherMouseDragged, CGMouseButton::Center)
    } else {
        (CGEventType::MouseMoved, CGMouseButton::Left)
    };
    let ev = CGEvent::new_mouse_event(source()?, kind, point, button).map_err(|_| os_err("mouse move"))?;
    if let Some((dx, dy)) = delta {
        // Games and 3D apps read these deltas rather than the absolute position.
        ev.set_integer_value_field(EventField::MOUSE_EVENT_DELTA_X, i64::from(dx));
        ev.set_integer_value_field(EventField::MOUSE_EVENT_DELTA_Y, i64::from(dy));
    }
    ev.set_flags(st.flags);
    ev.post(CGEventTapLocation::HID);
    Ok(())
}

fn modifier_flag(vk: u16) -> Option<CGEventFlags> {
    match vk {
        0x38 | 0x3C => Some(CGEventFlags::CGEventFlagShift),
        0x3B | 0x3E => Some(CGEventFlags::CGEventFlagControl),
        0x3A | 0x3D => Some(CGEventFlags::CGEventFlagAlternate),
        0x37 | 0x36 => Some(CGEventFlags::CGEventFlagCommand),
        _ => None,
    }
}

pub fn inject(ev: InputEvent) -> Result<()> {
    let mut st = state().lock().map_err(|_| PlatformError::Os("input state poisoned".into()))?;
    match ev {
        InputEvent::MouseMove { x, y } => {
            let (bounds, scale) = mapping();
            let p = CGPoint::new(bounds.origin.x + f64::from(x) * scale, bounds.origin.y + f64::from(y) * scale);
            move_to(&st, clamp_to(&bounds, p), None)
        }
        InputEvent::MouseMoveRelative { dx, dy } => {
            let (bounds, _) = mapping();
            let cur = current_position().unwrap_or(bounds.origin);
            let p = clamp_to(&bounds, CGPoint::new(cur.x + f64::from(dx), cur.y + f64::from(dy)));
            move_to(&st, p, Some((dx, dy)))
        }
        InputEvent::MouseButton { button, down } => {
            let (kind, cg_button, bit) = match (button, down) {
                (1, true) => (CGEventType::LeftMouseDown, CGMouseButton::Left, 1),
                (1, false) => (CGEventType::LeftMouseUp, CGMouseButton::Left, 1),
                (2, true) => (CGEventType::RightMouseDown, CGMouseButton::Right, 2),
                (2, false) => (CGEventType::RightMouseUp, CGMouseButton::Right, 2),
                (3, true) => (CGEventType::OtherMouseDown, CGMouseButton::Center, 4),
                (3, false) => (CGEventType::OtherMouseUp, CGMouseButton::Center, 4),
                _ => return Err(PlatformError::InvalidInput),
            };
            let now = Instant::now();
            let clicks = match st.last_click {
                Some((b, t, n)) if b == button && now.duration_since(t) < Duration::from_millis(400) => {
                    if down {
                        n + 1
                    } else {
                        n
                    }
                }
                _ => 1,
            };
            if down {
                st.last_click = Some((button, now, clicks));
                st.buttons |= bit;
            } else {
                st.buttons &= !bit;
            }
            let pos = current_position().unwrap_or(CGPoint::new(0.0, 0.0));
            let ev = CGEvent::new_mouse_event(source()?, kind, pos, cg_button).map_err(|_| os_err("mouse button"))?;
            ev.set_integer_value_field(EventField::MOUSE_EVENT_CLICK_STATE, clicks);
            ev.set_flags(st.flags);
            ev.post(CGEventTapLocation::HID);
            Ok(())
        }
        InputEvent::Wheel { dx, dy } => {
            let ev =
                CGEvent::new_scroll_event(source()?, ScrollEventUnit::LINE, 2, wheel_notches(dy), wheel_notches(dx), 0)
                    .map_err(|_| os_err("scroll"))?;
            ev.set_flags(st.flags);
            ev.post(CGEventTapLocation::HID);
            Ok(())
        }
        InputEvent::Key { scancode, down } => {
            let vk = keymaps::set1_to_macos(scancode).ok_or(PlatformError::InvalidInput)?;
            if let Some(flag) = modifier_flag(vk) {
                if down {
                    st.flags.insert(flag);
                } else {
                    st.flags.remove(flag);
                }
            }
            if vk == 0x39 && down {
                st.flags.toggle(CGEventFlags::CGEventFlagAlphaShift);
            }
            let ev = CGEvent::new_keyboard_event(source()?, vk, down).map_err(|_| os_err("key"))?;
            ev.set_flags(st.flags);
            ev.post(CGEventTapLocation::HID);
            Ok(())
        }
    }
}

// ───────────────────────────── Dialogs ─────────────────────────────

fn osascript(script_lines: &[&str], args: &[&str]) -> Command {
    let mut cmd = Command::new("osascript");
    for line in script_lines {
        cmd.arg("-e").arg(line);
    }
    // Text is passed as script arguments (argv), never interpolated into the script.
    cmd.args(args);
    cmd
}

pub fn ask_consent(viewer_fingerprint: &str, remote_addr: &str) -> Consent {
    let text = consent_text(viewer_fingerprint, remote_addr);
    let cmd = osascript(
        &[
            "on run argv",
            "activate",
            "set r to display dialog (item 1 of argv) with title (item 2 of argv) \
             buttons {\"Reject\", \"View only\", \"Full control\"} default button \"Reject\" \
             cancel button \"Reject\" with icon caution",
            "return button returned of r",
            "end run",
        ],
        &[&text, CONSENT_TITLE],
    );
    match unix_dialog::run(CONSENT_TITLE, cmd) {
        Some((0, out)) if out == "Full control" => Consent::FullControl,
        Some((0, out)) if out == "View only" => Consent::ViewOnly,
        _ => Consent::Reject,
    }
}

/// Blocking "session in progress" dialog; returns when the user clicks Disconnect or it is closed.
pub fn session_indicator(title: &str, text: &str) {
    let cmd = osascript(
        &[
            "on run argv",
            "activate",
            "display dialog (item 1 of argv) with title (item 2 of argv) buttons {\"Disconnect\"} \
             default button \"Disconnect\" with icon note",
            "end run",
        ],
        &[text, title],
    );
    let _ = unix_dialog::run(title, cmd);
}
