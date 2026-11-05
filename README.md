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
- Windows only for host and viewer; macOS/Linux/mobile come in the Beta phase.
- Portable/attended mode: no unattended access, no control of UAC prompts or elevated windows yet.
- Codec: JPEG dirty-tiles (patent-free, CPU); hardware H.264/HEVC/AV1 is planned.
- Console UI; the graphical UI (Flutter, ADR-004) is planned.
- IPv4 only.

## Downloads
Pre-built binaries are attached to [GitHub Releases](https://github.com/valerchikforever/rtdesk/releases).
Verify them before use:
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
