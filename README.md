# Bridge — free, open-source, secure remote desktop

[![CI](https://github.com/valerchikforever/rtdesk/actions/workflows/ci.yml/badge.svg)](https://github.com/valerchikforever/rtdesk/actions/workflows/ci.yml)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)

Bridge is a **completely free** (AGPL-3.0), self-hostable remote desktop in the spirit of RustDesk,
designed security-first: the servers can never see your screen, keystrokes or codes.

> **Status: MVP (Phase 1).** Windows host + Windows viewer, attended sessions only.
> Design docs, threat model and roadmap live in the Obsidian vault in [`docs/`](docs/00%20-%20Home.md).

## Security at a glance
- **End-to-end encrypted** QUIC/TLS 1.3 between viewer and host; the relay forwards opaque packets only.
- **Pinned Ed25519 device keys** (no CAs); key changes are refused unless explicitly accepted.
- **SPAKE2 over the one-time code, bound to the TLS session** → a malicious server cannot MITM,
  and captured traffic gives no offline guessing of the code.
- One-time codes are **single-use**, failures are rate-limited with exponential backoff and force a new code.
- The host user must **click Accept** (default button = reject), chooses view-only or full control,
  and sees a **session indicator** with a disconnect button the whole time.
- Host-side **state machine** rejects any input that arrives before consent or exceeds granted permissions.
- Device keys at rest protected with **DPAPI**. Memory-safe Rust; `unsafe` only in the platform crate.

## Components
| Binary | Role |
|---|---|
| `bridge-rendezvous` | ID registry, presence, NAT hole-punch coordination, relay tokens (UDP 21116) |
| `bridge-relay` | Blind UDP relay used when a direct connection is impossible (UDP 21117) |
| `bridge-host` | Runs on the computer to be accessed — shows ID + one-time code |
| `bridge-viewer` | Connects to a host and shows/controls its screen |

## Build
Requirements: Rust (see `rust-toolchain.toml`), on Windows the MSVC C++ build tools.
```bash
cargo build --release
cargo test --workspace
```
Binaries end up in `target/release/`.

## Run
**1. Server** (any machine reachable by both sides — a $5 VPS or a Raspberry Pi is enough):
```bash
bridge-rendezvous --listen 0.0.0.0:21116 --data-dir data --relay YOUR_PUBLIC_IP:21117
```
```bash
bridge-relay --listen 0.0.0.0:21117 --secret-file data/relay.secret
```
The rendezvous prints its **server key**. Open UDP 21116 and 21117 in the firewall.

**2. Host** (the computer to be accessed):
```bash
bridge-host --server YOUR_SERVER --server-key SERVER_KEY
```
It shows **Your ID** and a **Code**. Settings are remembered in `%APPDATA%\Bridge`.

**3. Viewer**:
```bash
bridge-viewer 123456789 --server YOUR_SERVER --server-key SERVER_KEY
```
Enter the code, wait for the host user to accept. Close the window to disconnect.

### Host requirements per OS
| OS | Requirements | Not yet supported |
|---|---|---|
| Windows 10/11 | — (gamepads: ViGEmBus driver) | UAC prompts / elevated windows |
| macOS 11+ | Allow **Screen Recording** and **Accessibility** for the app you start `bridge-host` from (e.g. Terminal) in System Settings → Privacy & Security, then restart it | Secondary displays, gamepads, login window |
| Linux | An **Xorg/X11** session (choose it on the login screen) and `zenity` (or `kdialog` on KDE) for the consent dialog | Wayland sessions, gamepads |

On macOS, the Windows key from a viewer becomes Command and Alt becomes Option.

### Games and 3D applications
- **Relative mouse:** when a game or 3D viewport hides the cursor, the viewer locks your mouse and sends raw
  movement, so mouse-look and orbiting work. **Ctrl+Alt+M** toggles it manually and always releases the mouse;
  switching windows releases it too. Disable auto mode with `--no-auto-relative`.
- **Gamepads:** up to 4 controllers are forwarded while the viewer window has focus and appear on the remote
  computer as Xbox 360 controllers. The **remote computer needs the ViGEmBus driver**
  (<https://github.com/nefarius/ViGEmBus/releases>, signed; note the project is end-of-life).
  Bridge never installs drivers itself. Disable with `--no-gamepad`.

Without `--server-key` the server key is pinned on first use (a warning shows the fingerprint to compare).

### Docker (servers)
```bash
docker build -f packaging/docker/Dockerfile -t bridge-server .
```
```bash
docker run -d --name bridge-rv -p 21116:21116/udp -v bridge-data:/data bridge-server bridge-rendezvous --listen 0.0.0.0:21116 --data-dir /data --relay YOUR_PUBLIC_IP:21117
```
```bash
docker run -d --name bridge-relay -p 21117:21117/udp -v bridge-data:/data bridge-server bridge-relay --listen 0.0.0.0:21117 --secret-file /data/relay.secret
```

## Known MVP limitations
- Host: Windows (tested), macOS and Linux/X11 (experimental, built in CI but not yet tested on real machines).
  Android/iOS apps are planned (v1.x).
- Portable/attended mode: no unattended access, no control of UAC prompts or elevated windows yet.
  Games running as administrator or with anti-cheat may ignore injected input.
- Codec: JPEG dirty-tiles (patent-free, CPU); hardware H.264/HEVC/AV1 is planned.
- Console UI; the graphical UI (Flutter, ADR-004) is planned.
- IPv4 only.

## Downloads
Pre-built binaries are attached to [GitHub Releases](https://github.com/valerchikforever/rtdesk/releases):

| Platform | File | Contains |
|---|---|---|
| Windows 10/11 x64 | `bridge-<ver>-windows-x64.zip` | host, viewer, rendezvous, relay |
| macOS 11+ (Apple Silicon & Intel) | `bridge-<ver>-macos-universal.zip` | host*, viewer, rendezvous, relay |
| Linux x64 (glibc 2.35+) | `bridge-<ver>-linux-x64.tar.gz` | host*, viewer, rendezvous, relay |
| Linux ARM64 (e.g. Raspberry Pi) | `bridge-<ver>-linux-arm64.tar.gz` | host*, viewer, rendezvous, relay |

\* experimental, see [host requirements](#host-requirements-per-os).

Not yet code-signed: on Windows SmartScreen may warn; on macOS run once after unpacking
```bash
xattr -dr com.apple.quarantine bridge-*-macos-universal
```
On macOS the Command key is sent as the Windows key and Option as Alt.

Verify downloads before use:
```bash
sha256sum -c SHA256SUMS
```
```bash
gh attestation verify bridge-v0.1.0-x86_64-pc-windows-msvc.zip --repo valerchikforever/rtdesk
```
Maintainers: push a tag such as `v0.1.0` to build a draft release.

## Contributing & security
See [CONTRIBUTING.md](CONTRIBUTING.md). Report vulnerabilities privately — see [SECURITY.md](SECURITY.md).

## License
AGPL-3.0-or-later — free forever, no paid editions ([ADR-006](docs/07%20-%20ADR/ADR-006%20Free%20and%20Open%20Source%20Licensing.md)).
