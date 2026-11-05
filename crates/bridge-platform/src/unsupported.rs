use crate::{Consent, InputEvent, PlatformError, Result};

pub fn init_process() {}

pub const CONSENT_TITLE: &str = "Bridge — incoming remote access request";

pub struct ScreenCapturer;

impl ScreenCapturer {
    pub fn new() -> Result<Self> {
        Err(PlatformError::Unsupported)
    }
    pub fn size(&self) -> (u32, u32) {
        (0, 0)
    }
    pub fn capture(&mut self, _out: &mut Vec<u8>) -> Result<()> {
        Err(PlatformError::Unsupported)
    }
}

pub fn screen_size() -> (u32, u32) {
    (0, 0)
}

pub fn inject(_ev: InputEvent) -> Result<()> {
    Err(PlatformError::Unsupported)
}

pub fn ask_consent(_viewer_fingerprint: &str, _remote_addr: &str) -> Consent {
    Consent::Reject
}

pub fn cursor_visible() -> bool {
    true
}

pub fn gamepad_available() -> bool {
    false
}

pub struct VirtualGamepads;

impl VirtualGamepads {
    pub fn new() -> Result<Self> {
        Err(PlatformError::Unsupported)
    }
    pub fn update(&mut self, _input: &crate::GamepadInput) -> Result<()> {
        Err(PlatformError::Unsupported)
    }
}

pub fn session_indicator(_title: &str, _text: &str) {}

pub fn close_dialog(_title: &str) {}
