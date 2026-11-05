//! Windows implementation.
//!
//! Known MVP limitations (documented in the vault):
//! - Runs as the interactive user ("portable mode"): cannot see/control UAC prompts or elevated
//!   windows (UIPI). The SYSTEM service with Winlogon helper comes in Beta.
//! - GDI BitBlt capture (~10–25 ms at 1080p). DXGI Desktop Duplication comes next.

use std::ffi::c_void;

use windows::core::HSTRING;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GdiFlush, GetDC, ReleaseDC, SelectObject,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ, ROP_CODE, SRCCOPY,
};
use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN,
    MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_VIRTUALDESK, MOUSEEVENTF_WHEEL, MOUSEINPUT, MOUSE_EVENT_FLAGS, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DrawIconEx, FindWindowW, GetCursorInfo, GetIconInfo, GetSystemMetrics, MessageBoxW, PostMessageW, CURSORINFO,
    CURSOR_SHOWING, DI_NORMAL, HICON, ICONINFO, IDNO, IDYES, MB_DEFBUTTON3, MB_ICONINFORMATION, MB_ICONWARNING, MB_OK,
    MB_SETFOREGROUND, MB_TOPMOST, MB_YESNOCANCEL, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
    SM_YVIRTUALSCREEN, WM_CLOSE,
};

use crate::{Consent, InputEvent, PlatformError, Result};

/// Call once at startup: makes capture and input use physical pixels on scaled displays.
pub fn init_process() {
    // SAFETY: plain Win32 call without pointers. Fails harmlessly if already set (e.g. by manifest).
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct VirtualScreen {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

fn virtual_screen() -> VirtualScreen {
    // SAFETY: GetSystemMetrics has no preconditions.
    unsafe {
        VirtualScreen {
            x: GetSystemMetrics(SM_XVIRTUALSCREEN),
            y: GetSystemMetrics(SM_YVIRTUALSCREEN),
            w: GetSystemMetrics(SM_CXVIRTUALSCREEN).max(1),
            h: GetSystemMetrics(SM_CYVIRTUALSCREEN).max(1),
        }
    }
}

/// Size of the area that is captured (all monitors).
pub fn screen_size() -> (u32, u32) {
    let v = virtual_screen();
    (v.w as u32, v.h as u32)
}

/// GDI screen capturer for the whole virtual desktop. Not `Send`: create it on the capture thread.
pub struct ScreenCapturer {
    screen: VirtualScreen,
    screen_dc: HDC,
    mem_dc: HDC,
    bitmap: HBITMAP,
    old: HGDIOBJ,
    bits: *mut u8,
}

impl ScreenCapturer {
    pub fn new() -> Result<Self> {
        let screen = virtual_screen();
        // SAFETY: standard GDI DIB-section setup; every handle is checked and released in Drop.
        unsafe {
            let screen_dc = GetDC(HWND::default());
            if screen_dc.is_invalid() {
                return Err(PlatformError::Os("GetDC failed".into()));
            }
            let mem_dc = CreateCompatibleDC(screen_dc);
            if mem_dc.is_invalid() {
                ReleaseDC(HWND::default(), screen_dc);
                return Err(PlatformError::Os("CreateCompatibleDC failed".into()));
            }
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: screen.w,
                    biHeight: -screen.h, // top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits: *mut c_void = std::ptr::null_mut();
            let bitmap = match CreateDIBSection(screen_dc, &bmi, DIB_RGB_COLORS, &mut bits, None, 0) {
                Ok(b) if !bits.is_null() => b,
                _ => {
                    let _ = DeleteDC(mem_dc);
                    ReleaseDC(HWND::default(), screen_dc);
                    return Err(PlatformError::Os("CreateDIBSection failed".into()));
                }
            };
            let old = SelectObject(mem_dc, bitmap);
            Ok(Self { screen, screen_dc, mem_dc, bitmap, old, bits: bits as *mut u8 })
        }
    }

    pub fn size(&self) -> (u32, u32) {
        (self.screen.w as u32, self.screen.h as u32)
    }

    /// Capture into `out` as top-down BGRA. Re-initializes if the monitor layout changed.
    pub fn capture(&mut self, out: &mut Vec<u8>) -> Result<()> {
        if virtual_screen() != self.screen {
            *self = Self::new()?;
        }
        let VirtualScreen { x, y, w, h } = self.screen;
        // SAFETY: handles are valid for the lifetime of self; `bits` points to a w*h*4 DIB owned
        // by `bitmap`, read only after GdiFlush.
        unsafe {
            BitBlt(self.mem_dc, 0, 0, w, h, self.screen_dc, x, y, ROP_CODE(SRCCOPY.0 | CAPTUREBLT.0))
                .map_err(|e| PlatformError::Os(format!("BitBlt: {e}")))?;
            draw_cursor(self.mem_dc, x, y);
            let _ = GdiFlush();
            let len = w as usize * h as usize * 4;
            out.clear();
            out.extend_from_slice(std::slice::from_raw_parts(self.bits, len));
        }
        Ok(())
    }
}

impl Drop for ScreenCapturer {
    fn drop(&mut self) {
        // SAFETY: releasing exactly the handles acquired in `new`.
        unsafe {
            SelectObject(self.mem_dc, self.old);
            let _ = DeleteObject(self.bitmap);
            let _ = DeleteDC(self.mem_dc);
            ReleaseDC(HWND::default(), self.screen_dc);
        }
    }
}

/// GDI capture does not include the cursor; draw it so the viewer sees it.
unsafe fn draw_cursor(dc: HDC, origin_x: i32, origin_y: i32) {
    let mut ci = CURSORINFO { cbSize: std::mem::size_of::<CURSORINFO>() as u32, ..Default::default() };
    if GetCursorInfo(&mut ci).is_err() || ci.flags != CURSOR_SHOWING {
        return;
    }
    let icon = HICON(ci.hCursor.0);
    let mut info = ICONINFO::default();
    let (mut hx, mut hy) = (0, 0);
    if GetIconInfo(icon, &mut info).is_ok() {
        hx = info.xHotspot as i32;
        hy = info.yHotspot as i32;
        if !info.hbmMask.is_invalid() {
            let _ = DeleteObject(info.hbmMask);
        }
        if !info.hbmColor.is_invalid() {
            let _ = DeleteObject(info.hbmColor);
        }
    }
    let _ = DrawIconEx(
        dc,
        ci.ptScreenPos.x - origin_x - hx,
        ci.ptScreenPos.y - origin_y - hy,
        icon,
        0,
        0,
        0,
        None,
        DI_NORMAL,
    );
}

/// Inject one input event. Coordinates are clamped to the virtual screen.
pub fn inject(ev: InputEvent) -> Result<()> {
    let input = match ev {
        InputEvent::MouseMove { x, y } => {
            let v = virtual_screen();
            let x = i64::from(x.clamp(0, v.w - 1));
            let y = i64::from(y.clamp(0, v.h - 1));
            let nx = (x * 65_535 / i64::from((v.w - 1).max(1))) as i32;
            let ny = (y * 65_535 / i64::from((v.h - 1).max(1))) as i32;
            mouse(nx, ny, 0, MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK)
        }
        InputEvent::MouseButton { button, down } => {
            let flags = match (button, down) {
                (1, true) => MOUSEEVENTF_LEFTDOWN,
                (1, false) => MOUSEEVENTF_LEFTUP,
                (2, true) => MOUSEEVENTF_RIGHTDOWN,
                (2, false) => MOUSEEVENTF_RIGHTUP,
                (3, true) => MOUSEEVENTF_MIDDLEDOWN,
                (3, false) => MOUSEEVENTF_MIDDLEUP,
                _ => return Err(PlatformError::InvalidInput),
            };
            mouse(0, 0, 0, flags)
        }
        InputEvent::Wheel { dx, dy } => {
            if dy != 0 {
                send(&[mouse(0, 0, dy.clamp(-1200, 1200), MOUSEEVENTF_WHEEL)])?;
            }
            if dx == 0 {
                return Ok(());
            }
            mouse(0, 0, dx.clamp(-1200, 1200), MOUSEEVENTF_HWHEEL)
        }
        InputEvent::Key { scancode, down } => {
            let prefix = scancode >> 8;
            if scancode & 0xFF == 0 || !(prefix == 0 || prefix == 0xE0 || prefix == 0xE1) {
                return Err(PlatformError::InvalidInput);
            }
            let mut flags: KEYBD_EVENT_FLAGS = KEYEVENTF_SCANCODE;
            if prefix == 0xE0 {
                flags |= KEYEVENTF_EXTENDEDKEY;
            }
            if !down {
                flags |= KEYEVENTF_KEYUP;
            }
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0),
                        wScan: (scancode & 0xFF) as u16,
                        dwFlags: flags,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            }
        }
    };
    send(&[input])
}

fn mouse(dx: i32, dy: i32, data: i32, flags: MOUSE_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 { mi: MOUSEINPUT { dx, dy, mouseData: data as _, dwFlags: flags, time: 0, dwExtraInfo: 0 } },
    }
}

fn send(inputs: &[INPUT]) -> Result<()> {
    // SAFETY: `inputs` is a valid slice of fully initialized INPUT structs.
    let sent = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize == inputs.len() {
        Ok(())
    } else {
        // Typically UIPI: an elevated window has focus.
        Err(PlatformError::Os("SendInput blocked (elevated window focused?)".into()))
    }
}

/// Title of the consent dialog (used to close it on timeout / viewer departure).
pub const CONSENT_TITLE: &str = "Bridge — incoming remote access request";

/// Consent dialog (SEC: SR-05, AS-01). Default button is "Cancel" = reject, to resist consent fatigue.
pub fn ask_consent(viewer_fingerprint: &str, remote_addr: &str) -> Consent {
    let text = format!(
        "Someone wants to connect to this computer.\n\n\
         Viewer key fingerprint:  {viewer_fingerprint}\n\
         Network address:  {remote_addr}\n\n\
         WARNING: never allow access to someone who contacted you unexpectedly \
         (phone call, pop-up, e-mail, social media). Banks, Microsoft, police and \
         \"technical support\" never ask you for this.\n\n\
         Yes  —  allow FULL CONTROL (mouse, keyboard, clipboard)\n\
         No  —  allow VIEW ONLY\n\
         Cancel  —  reject"
    );
    // SAFETY: HSTRINGs live across the call.
    let r = unsafe {
        MessageBoxW(
            HWND::default(),
            &HSTRING::from(text),
            &HSTRING::from(CONSENT_TITLE),
            MB_YESNOCANCEL | MB_ICONWARNING | MB_DEFBUTTON3 | MB_TOPMOST | MB_SETFOREGROUND,
        )
    };
    match r {
        IDYES => Consent::FullControl,
        IDNO => Consent::ViewOnly,
        _ => Consent::Reject,
    }
}

/// Blocking "session in progress" indicator; returns when the local user clicks Disconnect (OK)
/// or the window is closed via [`close_dialog`].
pub fn session_indicator(title: &str, text: &str) {
    // SAFETY: HSTRINGs live across the call.
    unsafe {
        MessageBoxW(
            HWND::default(),
            &HSTRING::from(text),
            &HSTRING::from(title),
            MB_OK | MB_ICONINFORMATION | MB_TOPMOST,
        );
    }
}

/// Close a message box created by this process by its (unique) title.
/// WM_CLOSE resolves to Cancel (consent → reject) or OK (indicator) — never to an accept.
pub fn close_dialog(title: &str) {
    // SAFETY: FindWindowW/PostMessageW with a valid HSTRING; posting to a missing window is harmless.
    unsafe {
        if let Ok(hwnd) = FindWindowW(None, &HSTRING::from(title)) {
            if !hwnd.is_invalid() {
                let _ = PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
            }
        }
    }
}
