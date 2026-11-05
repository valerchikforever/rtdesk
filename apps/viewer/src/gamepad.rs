//! Local gamepads → remote virtual pads (XInput layout).
//!
//! Input is sent only while the viewer window has focus; on focus loss every pad gets a neutral
//! state so nothing stays pressed on the host (T-57). Pads are polled at ~250 Hz and only changes
//! are sent.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use bridge_proto::gamepad as gp;
use bridge_proto::msg::control::Kind;
use bridge_proto::msg::GamepadState;
use gilrs::{Axis, Button, EventType, GamepadId, Gilrs};
use tokio::sync::mpsc;
use tracing::{info, warn};

const POLL: Duration = Duration::from_millis(4);

const BUTTONS: [(Button, u32); 13] = [
    (Button::South, gp::A),
    (Button::East, gp::B),
    (Button::West, gp::X),
    (Button::North, gp::Y),
    (Button::LeftTrigger, gp::LEFT_SHOULDER),
    (Button::RightTrigger, gp::RIGHT_SHOULDER),
    (Button::Select, gp::BACK),
    (Button::Start, gp::START),
    (Button::Mode, gp::GUIDE),
    (Button::LeftThumb, gp::LEFT_THUMB),
    (Button::RightThumb, gp::RIGHT_THUMB),
    (Button::DPadUp, gp::DPAD_UP),
    (Button::DPadDown, gp::DPAD_DOWN),
];

fn stick(v: f32) -> i32 {
    (v.clamp(-1.0, 1.0) * 32_767.0).round() as i32
}

fn trigger(v: f32) -> u32 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u32
}

fn read_pad(index: u32, pad: &gilrs::Gamepad<'_>) -> GamepadState {
    let mut buttons = BUTTONS.iter().filter(|(b, _)| pad.is_pressed(*b)).fold(0, |acc, (_, bit)| acc | bit);
    // D-pad left/right as buttons, or as a hat axis on some backends.
    let dpad_x = pad.value(Axis::DPadX);
    let dpad_y = pad.value(Axis::DPadY);
    if pad.is_pressed(Button::DPadLeft) || dpad_x < -0.5 {
        buttons |= gp::DPAD_LEFT;
    }
    if pad.is_pressed(Button::DPadRight) || dpad_x > 0.5 {
        buttons |= gp::DPAD_RIGHT;
    }
    if dpad_y > 0.5 {
        buttons |= gp::DPAD_UP;
    }
    if dpad_y < -0.5 {
        buttons |= gp::DPAD_DOWN;
    }
    let analog = |b: Button| pad.button_data(b).map(|d| trigger(d.value())).unwrap_or(0);
    GamepadState {
        index,
        connected: true,
        buttons,
        left_trigger: analog(Button::LeftTrigger2),
        right_trigger: analog(Button::RightTrigger2),
        left_x: stick(pad.value(Axis::LeftStickX)),
        left_y: stick(pad.value(Axis::LeftStickY)),
        right_x: stick(pad.value(Axis::RightStickX)),
        right_y: stick(pad.value(Axis::RightStickY)),
    }
}

/// Runs on its own thread until `stop` is set or the channel closes.
pub fn run(tx: mpsc::Sender<Kind>, focused: Arc<AtomicBool>, stop: Arc<AtomicBool>) {
    let mut gilrs = match Gilrs::new() {
        Ok(g) => g,
        Err(e) => {
            warn!("gamepad input unavailable: {e}");
            return;
        }
    };
    let mut slots: HashMap<GamepadId, u32> = HashMap::new();
    let mut last: HashMap<u32, GamepadState> = HashMap::new();

    while !stop.load(Ordering::Relaxed) {
        while let Some(ev) = gilrs.next_event() {
            if ev.event == EventType::Disconnected {
                if let Some(slot) = slots.remove(&ev.id) {
                    info!(slot, "gamepad disconnected");
                    last.remove(&slot);
                    if tx.blocking_send(Kind::Gamepad(GamepadState { index: slot, ..Default::default() })).is_err() {
                        return;
                    }
                }
            }
        }
        let is_focused = focused.load(Ordering::Relaxed);
        for (id, pad) in gilrs.gamepads() {
            if !pad.is_connected() {
                continue;
            }
            let slot = match slots.get(&id) {
                Some(&s) => s,
                None => {
                    let Some(free) = (0..gp::MAX_PADS).find(|s| !slots.values().any(|v| v == s)) else { continue };
                    info!(slot = free, name = pad.name(), "gamepad connected");
                    slots.insert(id, free);
                    free
                }
            };
            let state = if is_focused { read_pad(slot, &pad) } else { GamepadState::neutral(slot) };
            if last.get(&slot) != Some(&state) {
                match tx.try_send(Kind::Gamepad(state.clone())) {
                    Ok(()) => {
                        last.insert(slot, state);
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => return,
                    Err(mpsc::error::TrySendError::Full(_)) => {} // retried next poll
                }
            }
        }
        std::thread::sleep(POLL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_scaling_stays_in_range() {
        assert_eq!(stick(1.0), 32_767);
        assert_eq!(stick(-1.0), -32_767);
        assert_eq!(stick(5.0), 32_767);
        assert_eq!(stick(0.0), 0);
        assert_eq!(trigger(1.0), 255);
        assert_eq!(trigger(-0.3), 0);
        assert_eq!(trigger(0.5), 128);
    }

    #[test]
    fn mapped_buttons_are_valid_xinput_bits() {
        let all = BUTTONS.iter().fold(0, |a, (_, b)| a | b) | gp::DPAD_LEFT | gp::DPAD_RIGHT;
        assert_eq!(all & !gp::ALL_BUTTONS, 0);
    }
}
