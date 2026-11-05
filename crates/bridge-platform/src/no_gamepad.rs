//! Virtual gamepads are not available on this host OS yet (Linux: uinput, macOS: needs a DriverKit
//! driver). The host advertises no GAMEPAD feature, so viewers never send pad input.

use crate::{GamepadInput, PlatformError, Result};

pub fn gamepad_available() -> bool {
    false
}

pub struct VirtualGamepads;

impl VirtualGamepads {
    pub fn new() -> Result<Self> {
        Err(PlatformError::Unsupported)
    }

    pub fn update(&mut self, _input: &GamepadInput) -> Result<()> {
        Err(PlatformError::Unsupported)
    }
}
