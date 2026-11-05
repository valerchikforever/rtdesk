---
tags: [architecture, stack]
status: draft
updated: 2026-11-05
---

# Tech Stack

> [!decision] Summary
> Rust everywhere for core, servers and protocol ([[ADR-001 Rust for Core and Servers]]). QUIC end-to-end ([[ADR-002 QUIC End-to-End Transport]]). UI via Flutter or Tauri — see [[ADR-004 UI Framework]].

> [!info] MVP reality
> See [[Implementation Status#Deviations from the plan]]: `ring` instead of `aws-lc-rs`, hand-written prost derives, SPAKE2, JPEG tiles ([[ADR-007 MVP Screen Codec]]), winit + softbuffer viewer window.

## Core (shared library `bridge-core`)
| Concern | Choice | Notes |
|---|---|---|
| Language | Rust (stable, pinned via `rust-toolchain.toml`) | Memory safety for all network-facing parsing |
| Async runtime | `tokio` | |
| Transport | `quinn` (QUIC) + `rustls` (TLS 1.3) | Datagrams extension (RFC 9221) for video/input |
| Crypto primitives | `rustls` w/ `aws-lc-rs` provider, `ed25519-dalek`, `x25519-dalek`, `chacha20poly1305` | No home-grown crypto |
| PAKE | CPace / SPAKE2 (attended), OPAQUE (`opaque-ke`) (unattended) | [[ADR-003 PAKE-Based Session Authentication]] |
| Serialization | Protocol Buffers (`prost`) with strict size limits | Schema evolution, fuzzable |
| Video codecs | H.264 / HEVC / AV1 via hardware (NVENC, QSV, AMF, VideoToolbox, VA-API, MediaCodec); software fallback `openh264` / `libvpx` (VP9) / `dav1d`+`rav1e`/`SVT-AV1` | [[Media Pipeline]] |
| Logging | `tracing` with redaction layer | Never log secrets/OTCs |

## Platform layers
| Platform | Capture | Input | Service |
|---|---|---|---|
| Windows | Desktop Duplication API (DXGI) primary; Windows.Graphics.Capture fallback; GDI for secure desktop fallback | `SendInput`; secure desktop via helper in Winlogon session spawned by service | Windows Service (SYSTEM) |
| macOS | ScreenCaptureKit | `CGEvent` (Accessibility permission) | LaunchDaemon + LaunchAgent |
| Linux X11 | XShm / XDamage | XTest | systemd unit |
| Linux Wayland | PipeWire via `xdg-desktop-portal` ScreenCast | RemoteDesktop portal / libei | systemd user unit |
| Android (host, v1.x) | MediaProjection | AccessibilityService | Foreground service |

## Servers
- `bridge-rendezvous`, `bridge-relay`: Rust, tokio, quinn; single static binary; config via TOML + env.
- `bridge-console`: Rust (`axum`) API + TypeScript/React SPA; PostgreSQL via `sqlx`; OIDC via `openidconnect` crate.
- Containers: distroless images, non-root, read-only rootfs.

## Tooling
- Build: Cargo workspace, `cargo-xtask` for orchestration, `cross` / native runners for cross-compilation.
- Quality: `clippy` (pedantic subset), `rustfmt`, `cargo-deny`, `cargo-audit`, `cargo-vet`, `cargo-fuzz`, `miri` for unsafe modules.
- Packaging: MSIX/MSI (WiX), notarized `.pkg`/`.dmg`, `.deb`/`.rpm`/Flatpak, APK/AAB.
