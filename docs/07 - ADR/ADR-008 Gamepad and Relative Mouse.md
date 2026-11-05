---
tags: [adr, input, gaming]
status: accepted
updated: 2026-11-05
---

# ADR-008 Gamepad and Relative Mouse

- **Status:** accepted
- **Date:** 2026-11-05

## Context
Games and 3D software (Blender, CAD, game engines) need two things a plain remote desktop does not give:
1. **Relative mouse.** They hide the cursor and read Raw Input deltas (mouse-look, orbit). Absolute positioning makes the camera jump or stick at the screen edge, and the viewer's local cursor leaves the window.
2. **Gamepads.** The host must see a real controller device; key mapping is not enough.

## Decision

### Relative mouse (pointer lock)
- New message `MouseMoveRelative{dx,dy}` (raw counts, |d| ≤ 10 000). The host injects it with `SendInput` **without** `MOUSEEVENTF_ABSOLUTE`, so Raw Input readers get exactly these deltas.
- The viewer reads raw deltas (`DeviceEvent::MouseMotion`), hides its cursor and locks it to the window (`Locked`, falling back to `Confined` on Windows).
- **Auto mode:** the host reports `CursorState{visible}` (polled at 10 Hz). While the remote cursor is hidden, the viewer switches to relative mode; when it reappears, back to absolute.
- **Ctrl+Alt+M** (not forwarded) toggles manually. A single press always releases the mouse; if auto mode was what locked it, auto is disabled for the session. Focus loss always releases. `--no-auto-relative` disables auto mode.

### Gamepads
- New message `GamepadState` = full XInput state of one pad (index 0–3, buttons, triggers 0–255, sticks ±32767), sent on change only.
- Viewer: `gilrs` (Windows.Gaming.Input/XInput), polled at ~250 Hz; sends only while its window has focus, neutral state on focus loss.
- Host: virtual **Xbox 360 controller via ViGEmBus** (`vigem-client` crate). The host advertises `features::GAMEPAD` only if the driver is reachable and the session has CONTROL permission.

### Options considered for the host gamepad
| Option | Verdict |
|---|---|
| ViGEmBus (user installs signed driver) | **Chosen.** Widely used (e.g. by Sunshine); games see a real XInput pad |
| `Windows.UI.Input.Preview.Injection` gamepad injection | Only visible to Windows.Gaming.Input apps; most games use XInput/DirectInput |
| Own virtual HID/USB driver | Correct long term, but needs kernel development and WHQL/attestation signing |
| Map pad to keyboard/mouse | Wrong for games with analog input |

## Security consequences
- No driver is installed by Bridge (SR-09): the user installs ViGEmBus from its official signed release, otherwise gamepads are reported as unavailable.
- Gamepad and relative mouse messages require the `CONTROL` permission and the Active phase; values are range-checked (T-52). Covered by unit tests and the `session_admit` fuzz target.
- Virtual pads exist only during a session and are unplugged when it ends (T-56).
- Neutral pad state on focus loss / disconnect prevents stuck inputs (T-57).
- The consent dialog now says "mouse, keyboard, gamepad, clipboard".

## Risks
- **ViGEmBus is archived by its author (end-of-life).** It still works, but no fixes will come. Track alternatives; plan an own signed driver or a successor project before v1.0 ([[Risk Register]] R-13).
- Testing relative mode needs two machines: on one PC the injected motion is seen again by the viewer's raw input (feedback loop).
