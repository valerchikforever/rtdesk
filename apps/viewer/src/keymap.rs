//! Physical key → PC/AT set-1 scancode, identical on every viewer OS.
//!
//! The host injects set-1 scancodes. Windows reports them natively, but macOS and Linux use other
//! key code spaces, so we translate from winit's layout-independent [`KeyCode`] instead.
//! On macOS, Command maps to the Windows key and Option to Alt.

use winit::keyboard::KeyCode;

pub fn scancode(code: KeyCode) -> Option<u32> {
    use KeyCode::*;
    let sc = match code {
        Escape => 0x01,
        Digit1 => 0x02,
        Digit2 => 0x03,
        Digit3 => 0x04,
        Digit4 => 0x05,
        Digit5 => 0x06,
        Digit6 => 0x07,
        Digit7 => 0x08,
        Digit8 => 0x09,
        Digit9 => 0x0A,
        Digit0 => 0x0B,
        Minus => 0x0C,
        Equal => 0x0D,
        Backspace => 0x0E,
        Tab => 0x0F,
        KeyQ => 0x10,
        KeyW => 0x11,
        KeyE => 0x12,
        KeyR => 0x13,
        KeyT => 0x14,
        KeyY => 0x15,
        KeyU => 0x16,
        KeyI => 0x17,
        KeyO => 0x18,
        KeyP => 0x19,
        BracketLeft => 0x1A,
        BracketRight => 0x1B,
        Enter => 0x1C,
        ControlLeft => 0x1D,
        KeyA => 0x1E,
        KeyS => 0x1F,
        KeyD => 0x20,
        KeyF => 0x21,
        KeyG => 0x22,
        KeyH => 0x23,
        KeyJ => 0x24,
        KeyK => 0x25,
        KeyL => 0x26,
        Semicolon => 0x27,
        Quote => 0x28,
        Backquote => 0x29,
        ShiftLeft => 0x2A,
        Backslash => 0x2B,
        KeyZ => 0x2C,
        KeyX => 0x2D,
        KeyC => 0x2E,
        KeyV => 0x2F,
        KeyB => 0x30,
        KeyN => 0x31,
        KeyM => 0x32,
        Comma => 0x33,
        Period => 0x34,
        Slash => 0x35,
        ShiftRight => 0x36,
        NumpadMultiply => 0x37,
        AltLeft => 0x38,
        Space => 0x39,
        CapsLock => 0x3A,
        F1 => 0x3B,
        F2 => 0x3C,
        F3 => 0x3D,
        F4 => 0x3E,
        F5 => 0x3F,
        F6 => 0x40,
        F7 => 0x41,
        F8 => 0x42,
        F9 => 0x43,
        F10 => 0x44,
        NumLock => 0x45,
        ScrollLock => 0x46,
        Numpad7 => 0x47,
        Numpad8 => 0x48,
        Numpad9 => 0x49,
        NumpadSubtract => 0x4A,
        Numpad4 => 0x4B,
        Numpad5 => 0x4C,
        Numpad6 => 0x4D,
        NumpadAdd => 0x4E,
        Numpad1 => 0x4F,
        Numpad2 => 0x50,
        Numpad3 => 0x51,
        Numpad0 => 0x52,
        NumpadDecimal => 0x53,
        IntlBackslash => 0x56,
        F11 => 0x57,
        F12 => 0x58,
        NumpadEqual => 0x59,
        F13 => 0x64,
        F14 => 0x65,
        F15 => 0x66,
        IntlRo => 0x73,
        IntlYen => 0x7D,
        // Extended keys (0xE0 prefix)
        NumpadEnter => 0xE01C,
        ControlRight => 0xE01D,
        NumpadDivide => 0xE035,
        PrintScreen => 0xE037,
        AltRight => 0xE038,
        Home => 0xE047,
        ArrowUp => 0xE048,
        PageUp => 0xE049,
        ArrowLeft => 0xE04B,
        ArrowRight => 0xE04D,
        End => 0xE04F,
        ArrowDown => 0xE050,
        PageDown => 0xE051,
        Insert => 0xE052,
        Delete => 0xE053,
        SuperLeft => 0xE05B,
        SuperRight => 0xE05C,
        ContextMenu => 0xE05D,
        AudioVolumeMute => 0xE020,
        AudioVolumeDown => 0xE02E,
        AudioVolumeUp => 0xE030,
        MediaPlayPause => 0xE022,
        MediaTrackNext => 0xE019,
        MediaTrackPrevious => 0xE010,
        _ => return None,
    };
    Some(sc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const ALL: &[KeyCode] = &[
        KeyCode::Escape,
        KeyCode::Digit1,
        KeyCode::Digit0,
        KeyCode::KeyQ,
        KeyCode::KeyA,
        KeyCode::KeyZ,
        KeyCode::KeyM,
        KeyCode::Enter,
        KeyCode::Space,
        KeyCode::ShiftLeft,
        KeyCode::ShiftRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::AltLeft,
        KeyCode::AltRight,
        KeyCode::SuperLeft,
        KeyCode::F1,
        KeyCode::F12,
        KeyCode::ArrowUp,
        KeyCode::ArrowDown,
        KeyCode::ArrowLeft,
        KeyCode::ArrowRight,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::Insert,
        KeyCode::Delete,
        KeyCode::Numpad0,
        KeyCode::Numpad9,
        KeyCode::NumpadEnter,
        KeyCode::NumpadDivide,
        KeyCode::PrintScreen,
        KeyCode::IntlBackslash,
        KeyCode::Backquote,
        KeyCode::CapsLock,
        KeyCode::Tab,
        KeyCode::Backspace,
    ];

    #[test]
    fn known_values() {
        assert_eq!(scancode(KeyCode::KeyA), Some(0x1E));
        assert_eq!(scancode(KeyCode::ArrowLeft), Some(0xE04B));
        assert_eq!(scancode(KeyCode::Fn), None);
    }

    #[test]
    fn unique_and_accepted_by_host() {
        let mut seen = HashSet::new();
        for &k in ALL {
            let sc = scancode(k).unwrap_or_else(|| panic!("{k:?} unmapped"));
            assert!(seen.insert(sc), "duplicate scancode for {k:?}");
            // Same rule the host state machine enforces.
            let prefix = sc >> 8;
            assert!(sc & 0xFF != 0 && (prefix == 0 || prefix == 0xE0), "{k:?} → {sc:#x}");
        }
    }
}
