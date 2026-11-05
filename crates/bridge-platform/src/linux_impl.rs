//! Linux host backend (X11 / Xorg sessions).
//!
//! - Capture: X11 `GetImage` of the root window (all monitors) + XFixes cursor image.
//! - Input: XTest fake input.
//! - Dialogs: zenity (GNOME and most desktops) or kdialog (KDE).
//!
//! Wayland sessions are refused explicitly: X11 calls there only see XWayland windows. A PipeWire /
//! xdg-desktop-portal backend is the follow-up (see Host Agent note).

use std::process::Command;
use std::sync::Mutex;

use x11rb::connection::Connection;
use x11rb::protocol::xfixes::ConnectionExt as _;
use x11rb::protocol::xproto::{
    ConnectionExt as _, ImageFormat, Window, BUTTON_PRESS_EVENT, BUTTON_RELEASE_EVENT, KEY_PRESS_EVENT,
    KEY_RELEASE_EVENT, MOTION_NOTIFY_EVENT,
};
use x11rb::protocol::xtest::ConnectionExt as _;
use x11rb::rust_connection::RustConnection;

use crate::unix_dialog::{self, have};
use crate::{consent_text, keymaps, wheel_notches, Consent, InputEvent, PlatformError, Result, CONSENT_TITLE};

pub use crate::unix_dialog::close_dialog;

fn err(e: impl std::fmt::Display) -> PlatformError {
    PlatformError::Os(format!("X11: {e}"))
}

fn check_session() -> Result<()> {
    let wayland = std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v.eq_ignore_ascii_case("wayland"));
    if wayland {
        return Err(PlatformError::Os(
            "Wayland sessions are not supported yet — log in with an \"Xorg\"/\"X11\" session \
             (gear icon on the login screen)"
                .into(),
        ));
    }
    if std::env::var_os("DISPLAY").is_none() {
        return Err(PlatformError::Os("no X11 display (DISPLAY is not set)".into()));
    }
    Ok(())
}

struct X {
    conn: RustConnection,
    root: Window,
    xfixes: bool,
}

fn open() -> Result<X> {
    check_session()?;
    let (conn, screen) = x11rb::connect(None).map_err(err)?;
    let root = conn.setup().roots.get(screen).ok_or_else(|| err("no screen"))?.root;
    let xfixes = conn.xfixes_query_version(5, 0).ok().and_then(|c| c.reply().ok()).is_some();
    Ok(X { conn, root, xfixes })
}

impl X {
    fn size(&self) -> Result<(u16, u16)> {
        let g = self.conn.get_geometry(self.root).map_err(err)?.reply().map_err(err)?;
        Ok((g.width, g.height))
    }

    fn fake(&self, kind: u8, detail: u8, x: i16, y: i16) -> Result<()> {
        self.conn.xtest_fake_input(kind, detail, 0, self.root, x, y, 0).map_err(err)?;
        Ok(())
    }
}

pub fn init_process() {
    if let Err(e) = check_session() {
        eprintln!("{e}");
    }
}

/// Shared connection for input injection and queries (reopened after errors).
static SHARED: Mutex<Option<X>> = Mutex::new(None);

fn with_x<T>(f: impl FnOnce(&X) -> Result<T>) -> Result<T> {
    let mut guard = SHARED.lock().map_err(|_| err("lock poisoned"))?;
    if guard.is_none() {
        *guard = Some(open()?);
    }
    let x = guard.as_ref().ok_or_else(|| err("not connected"))?;
    let result = f(x);
    if result.is_err() {
        *guard = None;
    }
    result
}

pub fn screen_size() -> (u32, u32) {
    with_x(|x| x.size()).map(|(w, h)| (u32::from(w), u32::from(h))).unwrap_or((0, 0))
}

pub struct ScreenCapturer {
    x: X,
    size: (u16, u16),
}

impl ScreenCapturer {
    pub fn new() -> Result<Self> {
        let x = open()?;
        let size = x.size()?;
        Ok(Self { x, size })
    }

    pub fn size(&self) -> (u32, u32) {
        (u32::from(self.size.0), u32::from(self.size.1))
    }

    /// Capture the root window into `out` as top-down BGRA.
    pub fn capture(&mut self, out: &mut Vec<u8>) -> Result<()> {
        self.size = self.x.size()?; // follows xrandr changes
        let (w, h) = self.size;
        let reply = self
            .x
            .conn
            .get_image(ImageFormat::Z_PIXMAP, self.x.root, 0, 0, w, h, !0)
            .map_err(err)?
            .reply()
            .map_err(err)?;
        let len = usize::from(w) * usize::from(h) * 4;
        if !(reply.depth == 24 || reply.depth == 32) || reply.data.len() < len {
            return Err(err(format!("unsupported visual (depth {})", reply.depth)));
        }
        out.clear();
        out.extend_from_slice(&reply.data[..len]); // BGRX on little-endian servers
        if self.x.xfixes {
            draw_cursor(&self.x, out, usize::from(w), usize::from(h));
        }
        Ok(())
    }
}

/// X11 `GetImage` excludes the cursor; blend the XFixes cursor image (premultiplied ARGB) in.
fn draw_cursor(x: &X, buf: &mut [u8], w: usize, h: usize) {
    let Ok(cookie) = x.conn.xfixes_get_cursor_image() else { return };
    let Ok(c) = cookie.reply() else { return };
    let (cw, ch) = (usize::from(c.width), usize::from(c.height));
    if c.cursor_image.len() < cw * ch {
        return;
    }
    let ox = i32::from(c.x) - i32::from(c.xhot);
    let oy = i32::from(c.y) - i32::from(c.yhot);
    for cy in 0..ch {
        for cx in 0..cw {
            let px = c.cursor_image[cy * cw + cx];
            let a = px >> 24;
            if a == 0 {
                continue;
            }
            let (tx, ty) = (ox + cx as i32, oy + cy as i32);
            if tx < 0 || ty < 0 || tx as usize >= w || ty as usize >= h {
                continue;
            }
            let i = (ty as usize * w + tx as usize) * 4;
            let inv = 255 - a;
            for (k, shift) in [(0, 0), (1, 8), (2, 16)] {
                let src = (px >> shift) & 0xFF;
                let dst = u32::from(buf[i + k]);
                buf[i + k] = (src + dst * inv / 255).min(255) as u8;
            }
        }
    }
}

pub fn cursor_visible() -> bool {
    // A hidden X11 cursor is a fully transparent cursor image.
    with_x(|x| {
        if !x.xfixes {
            return Ok(true);
        }
        let c = x.conn.xfixes_get_cursor_image().map_err(err)?.reply().map_err(err)?;
        Ok(c.cursor_image.iter().any(|p| p >> 24 != 0))
    })
    .unwrap_or(true)
}

fn clamp_i16(v: i32) -> i16 {
    v.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
}

pub fn inject(ev: InputEvent) -> Result<()> {
    with_x(|x| {
        match ev {
            InputEvent::MouseMove { x: px, y: py } => x.fake(MOTION_NOTIFY_EVENT, 0, clamp_i16(px), clamp_i16(py))?,
            // detail = 1: relative motion
            InputEvent::MouseMoveRelative { dx, dy } => x.fake(MOTION_NOTIFY_EVENT, 1, clamp_i16(dx), clamp_i16(dy))?,
            InputEvent::MouseButton { button, down } => {
                // wire: 1 left, 2 right, 3 middle — X11: 1 left, 2 middle, 3 right
                let detail = match button {
                    1 => 1,
                    2 => 3,
                    3 => 2,
                    _ => return Err(PlatformError::InvalidInput),
                };
                x.fake(if down { BUTTON_PRESS_EVENT } else { BUTTON_RELEASE_EVENT }, detail, 0, 0)?;
            }
            InputEvent::Wheel { dx, dy } => {
                // X11 scrolls with buttons: 4 up, 5 down, 6 left, 7 right
                for (notches, pos, neg) in [(wheel_notches(dy), 4u8, 5u8), (wheel_notches(dx), 7, 6)] {
                    let button = if notches > 0 { pos } else { neg };
                    for _ in 0..notches.unsigned_abs() {
                        x.fake(BUTTON_PRESS_EVENT, button, 0, 0)?;
                        x.fake(BUTTON_RELEASE_EVENT, button, 0, 0)?;
                    }
                }
            }
            InputEvent::Key { scancode, down } => {
                let evdev = keymaps::set1_to_evdev(scancode).ok_or(PlatformError::InvalidInput)?;
                let keycode = u8::try_from(evdev + 8).map_err(|_| PlatformError::InvalidInput)?;
                x.fake(if down { KEY_PRESS_EVENT } else { KEY_RELEASE_EVENT }, keycode, 0, 0)?;
            }
        }
        x.conn.flush().map_err(err)
    })
}

// ───────────────────────────── Dialogs ─────────────────────────────

/// Consent dialog. Without zenity/kdialog the request is rejected (fail safe).
pub fn ask_consent(viewer_fingerprint: &str, remote_addr: &str) -> Consent {
    let text = consent_text(viewer_fingerprint, remote_addr);
    if have("zenity") {
        let mut cmd = Command::new("zenity");
        cmd.args(["--question", "--no-markup", "--width", "520", "--default-cancel"])
            .args(["--ok-label", "Full control", "--cancel-label", "Reject", "--extra-button", "View only"])
            .arg("--title")
            .arg(CONSENT_TITLE)
            .arg("--text")
            .arg(&text);
        return match unix_dialog::run(CONSENT_TITLE, cmd) {
            Some((0, _)) => Consent::FullControl,
            Some((1, out)) if out == "View only" => Consent::ViewOnly,
            _ => Consent::Reject,
        };
    }
    if have("kdialog") {
        let mut cmd = Command::new("kdialog");
        cmd.arg("--title").arg(CONSENT_TITLE).arg("--yesnocancel").arg(&text).args([
            "--yes-label",
            "Full control",
            "--no-label",
            "View only",
            "--cancel-label",
            "Reject",
        ]);
        return match unix_dialog::run(CONSENT_TITLE, cmd) {
            Some((0, _)) => Consent::FullControl,
            Some((1, _)) => Consent::ViewOnly,
            _ => Consent::Reject,
        };
    }
    eprintln!("Incoming connection rejected: no dialog tool found. Install `zenity` (or `kdialog` on KDE).");
    Consent::Reject
}

/// Blocking "session in progress" dialog; returns when the user clicks Disconnect or it is closed.
/// Without a dialog tool it returns immediately, which ends the session (fail safe).
pub fn session_indicator(title: &str, text: &str) {
    let cmd = if have("zenity") {
        let mut c = Command::new("zenity");
        c.args(["--info", "--no-markup", "--width", "420", "--ok-label", "Disconnect"])
            .arg("--title")
            .arg(title)
            .arg("--text")
            .arg(text);
        c
    } else if have("kdialog") {
        let mut c = Command::new("kdialog");
        c.arg("--title").arg(title).arg("--msgbox").arg(text);
        c
    } else {
        return;
    };
    let _ = unix_dialog::run(title, cmd);
}
