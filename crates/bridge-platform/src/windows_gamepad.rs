//! Virtual Xbox 360 gamepads via the ViGEmBus driver (ADR-008).
//!
//! SEC: we never install drivers ourselves (SR-09). If ViGEmBus is not installed by the user,
//! gamepad support is reported as unavailable. Virtual pads exist only while a session is active:
//! dropping [`VirtualGamepads`] unplugs all of them (T-56).

use std::rc::Rc;

use vigem_client::{Client, TargetId, XButtons, XGamepad, Xbox360Wired};

use crate::{GamepadInput, PlatformError, Result};

const MAX_PADS: usize = 4;

/// Is the ViGEmBus driver installed and reachable?
pub fn gamepad_available() -> bool {
    Client::connect().is_ok()
}

/// Up to four virtual pads. Not `Send`: create it on the thread that uses it.
pub struct VirtualGamepads {
    client: Rc<Client>,
    pads: [Option<Xbox360Wired<Rc<Client>>>; MAX_PADS],
}

impl VirtualGamepads {
    pub fn new() -> Result<Self> {
        let client =
            Client::connect().map_err(|e| PlatformError::Os(format!("ViGEmBus driver not available: {e:?}")))?;
        Ok(Self { client: Rc::new(client), pads: Default::default() })
    }

    pub fn update(&mut self, input: &GamepadInput) -> Result<()> {
        let slot = self.pads.get_mut(input.index).ok_or(PlatformError::InvalidInput)?;
        if !input.connected {
            if let Some(mut pad) = slot.take() {
                let _ = pad.unplug();
            }
            return Ok(());
        }
        if slot.is_none() {
            let mut pad = Xbox360Wired::new(self.client.clone(), TargetId::XBOX360_WIRED);
            pad.plugin().map_err(|e| PlatformError::Os(format!("plugging virtual gamepad: {e:?}")))?;
            pad.wait_ready().map_err(|e| PlatformError::Os(format!("virtual gamepad not ready: {e:?}")))?;
            *slot = Some(pad);
        }
        let report = XGamepad {
            buttons: XButtons { raw: input.buttons },
            left_trigger: input.left_trigger,
            right_trigger: input.right_trigger,
            thumb_lx: input.left_x,
            thumb_ly: input.left_y,
            thumb_rx: input.right_x,
            thumb_ry: input.right_y,
        };
        match slot.as_mut() {
            Some(pad) => pad.update(&report).map_err(|e| PlatformError::Os(format!("gamepad update: {e:?}"))),
            None => Ok(()),
        }
    }
}

impl Drop for VirtualGamepads {
    fn drop(&mut self) {
        for pad in self.pads.iter_mut().filter_map(Option::take) {
            let mut pad = pad;
            let _ = pad.unplug();
        }
    }
}
