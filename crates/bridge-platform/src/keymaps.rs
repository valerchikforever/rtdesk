//! Set-1 scancode (wire format) → native key codes of macOS and Linux hosts.
//! Pure tables, compiled and tested on every OS.

/// Set-1 scancode → Linux evdev key code (`KEY_*`). X11 key code = evdev + 8.
pub fn set1_to_evdev(sc: u32) -> Option<u32> {
    let prefix = sc >> 8;
    let code = sc & 0xFF;
    match prefix {
        // For the classic block, evdev codes equal set-1 codes.
        0 if (0x01..=0x58).contains(&code) && code != 0x54 && code != 0x55 => Some(code),
        0 => match code {
            0x59 => Some(117), // KP_EQUAL
            0x64 => Some(183), // F13
            0x65 => Some(184), // F14
            0x66 => Some(185), // F15
            0x73 => Some(89),  // RO
            0x7D => Some(124), // YEN
            _ => None,
        },
        0xE0 => match code {
            0x1C => Some(96),  // KP_ENTER
            0x1D => Some(97),  // RIGHTCTRL
            0x35 => Some(98),  // KP_SLASH
            0x37 => Some(99),  // SYSRQ / PrintScreen
            0x38 => Some(100), // RIGHTALT
            0x47 => Some(102), // HOME
            0x48 => Some(103), // UP
            0x49 => Some(104), // PAGEUP
            0x4B => Some(105), // LEFT
            0x4D => Some(106), // RIGHT
            0x4F => Some(107), // END
            0x50 => Some(108), // DOWN
            0x51 => Some(109), // PAGEDOWN
            0x52 => Some(110), // INSERT
            0x53 => Some(111), // DELETE
            0x5B => Some(125), // LEFTMETA
            0x5C => Some(126), // RIGHTMETA
            0x5D => Some(127), // COMPOSE (menu)
            0x20 => Some(113), // MUTE
            0x2E => Some(114), // VOLUMEDOWN
            0x30 => Some(115), // VOLUMEUP
            0x22 => Some(164), // PLAYPAUSE
            0x19 => Some(163), // NEXTSONG
            0x10 => Some(165), // PREVIOUSSONG
            _ => None,
        },
        _ => None,
    }
}

/// Set-1 scancode → macOS virtual key code (`kVK_*`). The Windows key maps to Command, Alt to Option.
pub fn set1_to_macos(sc: u32) -> Option<u16> {
    let vk = match sc {
        0x01 => 0x35,   // Escape
        0x02 => 0x12,   // 1
        0x03 => 0x13,   // 2
        0x04 => 0x14,   // 3
        0x05 => 0x15,   // 4
        0x06 => 0x17,   // 5
        0x07 => 0x16,   // 6
        0x08 => 0x1A,   // 7
        0x09 => 0x1C,   // 8
        0x0A => 0x19,   // 9
        0x0B => 0x1D,   // 0
        0x0C => 0x1B,   // Minus
        0x0D => 0x18,   // Equal
        0x0E => 0x33,   // Backspace (kVK_Delete)
        0x0F => 0x30,   // Tab
        0x10 => 0x0C,   // Q
        0x11 => 0x0D,   // W
        0x12 => 0x0E,   // E
        0x13 => 0x0F,   // R
        0x14 => 0x11,   // T
        0x15 => 0x10,   // Y
        0x16 => 0x20,   // U
        0x17 => 0x22,   // I
        0x18 => 0x1F,   // O
        0x19 => 0x23,   // P
        0x1A => 0x21,   // [
        0x1B => 0x1E,   // ]
        0x1C => 0x24,   // Return
        0x1D => 0x3B,   // Left Control
        0x1E => 0x00,   // A
        0x1F => 0x01,   // S
        0x20 => 0x02,   // D
        0x21 => 0x03,   // F
        0x22 => 0x05,   // G
        0x23 => 0x04,   // H
        0x24 => 0x26,   // J
        0x25 => 0x28,   // K
        0x26 => 0x25,   // L
        0x27 => 0x29,   // ;
        0x28 => 0x27,   // '
        0x29 => 0x32,   // `
        0x2A => 0x38,   // Left Shift
        0x2B => 0x2A,   // Backslash
        0x2C => 0x06,   // Z
        0x2D => 0x07,   // X
        0x2E => 0x08,   // C
        0x2F => 0x09,   // V
        0x30 => 0x0B,   // B
        0x31 => 0x2D,   // N
        0x32 => 0x2E,   // M
        0x33 => 0x2B,   // ,
        0x34 => 0x2F,   // .
        0x35 => 0x2C,   // /
        0x36 => 0x3C,   // Right Shift
        0x37 => 0x43,   // Keypad *
        0x38 => 0x3A,   // Left Option (Alt)
        0x39 => 0x31,   // Space
        0x3A => 0x39,   // Caps Lock
        0x3B => 0x7A,   // F1
        0x3C => 0x78,   // F2
        0x3D => 0x63,   // F3
        0x3E => 0x76,   // F4
        0x3F => 0x60,   // F5
        0x40 => 0x61,   // F6
        0x41 => 0x62,   // F7
        0x42 => 0x64,   // F8
        0x43 => 0x65,   // F9
        0x44 => 0x6D,   // F10
        0x45 => 0x47,   // Num Lock → Keypad Clear
        0x47 => 0x59,   // Keypad 7
        0x48 => 0x5B,   // Keypad 8
        0x49 => 0x5C,   // Keypad 9
        0x4A => 0x4E,   // Keypad -
        0x4B => 0x56,   // Keypad 4
        0x4C => 0x57,   // Keypad 5
        0x4D => 0x58,   // Keypad 6
        0x4E => 0x45,   // Keypad +
        0x4F => 0x53,   // Keypad 1
        0x50 => 0x54,   // Keypad 2
        0x51 => 0x55,   // Keypad 3
        0x52 => 0x52,   // Keypad 0
        0x53 => 0x41,   // Keypad .
        0x56 => 0x0A,   // ISO section (key left of Z)
        0x57 => 0x67,   // F11
        0x58 => 0x6F,   // F12
        0x59 => 0x51,   // Keypad =
        0x64 => 0x69,   // F13
        0x65 => 0x6B,   // F14
        0x66 => 0x71,   // F15
        0x73 => 0x5E,   // JIS underscore (Ro)
        0x7D => 0x5D,   // JIS yen
        0xE01C => 0x4C, // Keypad Enter
        0xE01D => 0x3E, // Right Control
        0xE035 => 0x4B, // Keypad /
        0xE038 => 0x3D, // Right Option
        0xE047 => 0x73, // Home
        0xE048 => 0x7E, // Up
        0xE049 => 0x74, // Page Up
        0xE04B => 0x7B, // Left
        0xE04D => 0x7C, // Right
        0xE04F => 0x77, // End
        0xE050 => 0x7D, // Down
        0xE051 => 0x79, // Page Down
        0xE052 => 0x72, // Insert → Help
        0xE053 => 0x75, // Forward Delete
        0xE05B => 0x37, // Left Windows → Command
        0xE05C => 0x36, // Right Windows → Right Command
        0xE05D => 0x6E, // Menu
        0xE020 => 0x4A, // Mute
        0xE02E => 0x49, // Volume Down
        0xE030 => 0x48, // Volume Up
        _ => return None,
    };
    Some(vk)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn all_scancodes() -> impl Iterator<Item = u32> {
        (0x01..=0x7Fu32).chain((0x01..=0x7Fu32).map(|c| 0xE000 | c))
    }

    #[test]
    fn evdev_known_and_unique() {
        assert_eq!(set1_to_evdev(0x1E), Some(30)); // A
        assert_eq!(set1_to_evdev(0x01), Some(1)); // Esc
        assert_eq!(set1_to_evdev(0xE04B), Some(105)); // Left
        assert_eq!(set1_to_evdev(0x54), None);
        let mut seen = HashSet::new();
        for sc in all_scancodes() {
            if let Some(code) = set1_to_evdev(sc) {
                assert!(seen.insert(code), "evdev {code} mapped twice (sc {sc:#x})");
                assert!(code + 8 <= 255, "X11 key codes are u8");
            }
        }
    }

    #[test]
    fn macos_known_and_unique() {
        assert_eq!(set1_to_macos(0x1E), Some(0x00)); // A
        assert_eq!(set1_to_macos(0xE05B), Some(0x37)); // Win → Command
        let mut seen = HashSet::new();
        for sc in all_scancodes() {
            if let Some(vk) = set1_to_macos(sc) {
                assert!(seen.insert(vk), "kVK {vk:#x} mapped twice (sc {sc:#x})");
                assert!(vk <= 0x7F);
            }
        }
    }
}
