---
tags: [implementation, status]
status: living
updated: 2026-11-05
---

# Implementation Status

> [!abstract] Current state — MVP (Phase 1 of [[Roadmap]])
> Working end to end on Windows: rendezvous + relay servers, host (portable, attended), viewer.
> Verified on 2026-11-05: registration, lookup, direct and **relayed** connection, SPAKE2 auth,
> wrong-code rejection, consent dialog, live screen streaming with full control, disconnect from host indicator.

## Code map
| Path | What | Plan reference |
|---|---|---|
| `crates/bridge-proto` | Messages (prost), length-prefixed framing with size limits, relay packet format | [[Wire Protocol]] |
| `crates/bridge-crypto` | Ed25519 keys, DPAPI key storage, TLS pinning verifiers, SPAKE2 + channel binding, OTC, relay tokens | [[Cryptography Design]] |
| `crates/bridge-session` | Host state machine + permission enforcement, auth rate limiter (pure, I/O-free) | [[Wire Protocol#Session state machine (host side)]] |
| `crates/bridge-codec` | Dirty-tile JPEG encoder / bounds-checked decoder | [[ADR-007 MVP Screen Codec]] |
| `crates/bridge-net` | QUIC configs, rendezvous client, hole punching, relay proxy, config + TOFU stores | [[NAT Traversal]] |
| `crates/bridge-platform` | Windows: GDI capture, SendInput, consent / indicator dialogs (all `unsafe` lives here) | [[Host Agent]] |
| `apps/rendezvous` | Registry, presence, lookup, relay token issuing, per-IP rate limits, QUIC retry | [[Rendezvous Server]] |
| `apps/relay` | Blind UDP relay with token-authorized pairing | [[Relay Server]] |
| `apps/host` | Host agent (portable attended mode) | [[Host Agent]] |
| `apps/viewer` | Viewer (winit + softbuffer) | [[Viewer Client]] |

## Platform matrix (v0.2.0, 2026-11-05)
| Platform | Host (being accessed) | Viewer | Servers | Built in CI | Tested live |
|---|---|---|---|---|---|
| Windows 10/11 x64 | ✅ | ✅ | ✅ | ✅ | ✅ (single PC) |
| macOS 11+ universal | ❌ (ScreenCaptureKit + CGEvent, Beta) | ✅ | ✅ | ✅ | ❌ |
| Linux x64 / ARM64 | ❌ (PipeWire portal / X11, Beta) | ✅ | ✅ | ✅ | ❌ |
| Android | ❌ (MediaProjection + Accessibility, v1.x) | ❌ needs touch UI | — | ❌ | ❌ |
| iOS | not possible (no screen-control APIs) | ❌ needs touch UI + Apple Developer account | — | ❌ | ❌ |

Viewer keyboard uses a portable `KeyCode → set-1 scancode` table (`apps/viewer/src/keymap.rs`); on macOS Command → Windows key, Option → Alt.

**Mobile viewer prerequisites:** a GUI for ID/code entry (no console on phones — [[ADR-004 UI Framework]]), touch-to-mouse modes and an on-screen keyboard, Android NDK build + signing key, and for iOS a Mac build host and a paid Apple Developer account (TestFlight/App Store). Binaries are unsigned: Windows SignPath, Apple notarization pending.

## Gamepads & relative mouse (2026-11-05)
Implemented per [[ADR-008 Gamepad and Relative Mouse]]: protocol messages, host-side validation (CONTROL only), relative `SendInput`, cursor-visibility reporting, ViGEmBus virtual Xbox 360 pads (session-scoped), viewer pointer lock with raw deltas, auto mode + Ctrl+Alt+M, `gilrs` gamepad polling with neutral state on focus loss.
- Verified: 48 unit tests (incl. permission/range checks, mode switching, delta accumulation), clippy, cargo-deny, fuzz targets updated.
- **Not yet verified live:** needs two PCs (single-PC relative mode loops back through raw input) plus a gamepad and ViGEmBus on the host.

## Requirements coverage
| ID | Status | Notes |
|---|---|---|
| FR-01 | ✅ | 9-digit ID bound to key; 9-char OTC, single-use |
| FR-02 | ✅ | Consent dialog: full control / view only / reject (default) |
| FR-03 | 🟡 | Streaming of the whole virtual desktop (all monitors); per-monitor switching TODO |
| FR-04 | 🟡 | Mouse + keyboard via scancodes; Ctrl+Alt+Del needs the service (Beta) |
| FR-05 | ✅ | Text clipboard both ways, only changes made during the session |
| FR-17 | ✅ | Self-hosted servers, no dependency on any cloud; Dockerfile |
| NFR-11 | ✅ | Free, AGPL-3.0, no feature gating |
| SR-01 | ✅ | QUIC/TLS 1.3 end-to-end; relay blind |
| SR-02 | ✅ | Host key pinned (TOFU + loud warning on change); server key pinned |
| SR-03 | ✅ | SPAKE2 channel-bound (balanced PAKE; OPAQUE comes with unattended access) |
| SR-04 | ✅ | Exponential backoff, code rotation every 5 failures |
| SR-05 | ✅ | Consent + always-visible indicator with disconnect |
| SR-06 | 🟡 | No privileged component yet (portable mode); service split in Beta |
| SR-07 | 🟡 | Rust parsers with size limits + negative tests; fuzz targets TODO |
| SR-10 | ✅ | DPAPI-protected device key |
| SR-12 | ✅ | Rate limits, address validation (QUIC retry), no secrets logged |

## Mitigations implemented (grep `SEC:` in code)
M-01 key pinning · M-02 channel-bound PAKE · M-04 key-bound IDs · M-31 no offline guessing · M-32 lookup rate limits · M-42 bounded framing · M-52 host-side state machine · M-53 viewer-side decoder validation · T-40 QUIC retry + per-IP limits · AS-01 warnings in banner and consent dialog.

## Test evidence
- 39 unit tests (`cargo test --workspace`), clippy clean.
- Security-relevant tests: wrong code / MITM (different channel binding) / reflected confirmation fail; input before consent, view-only input, unknown permission bits, replayed Hello rejected; forged / role-swapped / expired relay tokens rejected; spoofed relay senders dropped; ID hijack rejected; out-of-bounds or lying video tiles rejected; oversized frames rejected before allocation.
- Manual E2E on one PC (loopback): see summary above. Found and fixed: consent dialog stayed open after viewer left → now closes on viewer departure or after 60 s.

## Deviations from the plan
| Plan | MVP | Why |
|---|---|---|
| CPace | SPAKE2 (`spake2` crate) | Allowed fallback in [[ADR-003 PAKE-Based Session Authentication]]; mature RustCrypto crate |
| `aws-lc-rs` provider | `ring` provider | No CMake/NASM needed for builds |
| prost-build + `.proto` | Hand-written prost derives | No `protoc` in the toolchain; wire format identical |
| H.264/AV1 HW | JPEG dirty tiles | [[ADR-007 MVP Screen Codec]] |
| Flutter UI | Console + winit window | [[ADR-004 UI Framework]] still open |
| Rust edition 2024 | 2021 | Lower risk for FFI code; revisit later |

## Repository & pipeline (2026-11-05)
Public repository: <https://github.com/valerchikforever/rtdesk>

| Item | Where | Status |
|---|---|---|
| Licence (AGPL-3.0 full text) | `LICENSE` | ✅ |
| Vulnerability disclosure policy | `SECURITY.md` (GitHub private advisories, fix SLAs) | ✅ |
| Contribution rules, DCO sign-off | `CONTRIBUTING.md`, PR template with security checklist | ✅ |
| CI: fmt, clippy (`-D warnings`), tests, release build on Windows; libs + servers on Linux | `.github/workflows/ci.yml` | ✅ |
| Dependency policy: licences, advisories, sources | `deny.toml` + cargo-deny job | ✅ passes locally |
| Fuzzing: `relay_parse`, `session_admit`, `frame_decode`, `rendezvous_decode` (60 s each in CI) | `fuzz/` | ✅ |
| Release: tag `v*` → Windows + Linux binaries, SHA256SUMS, SLSA build provenance, **draft** release | `.github/workflows/release.yml` | ✅ |
| Dependency updates | `.github/dependabot.yml` (cargo + actions) | ✅ |
| Code signing | SignPath Foundation | ⏳ apply |

Dependency hygiene decision: winit is built without `wayland-csd-adwaita`, which pulled the unmaintained `ttf-parser` (RUSTSEC-2026-0192).

## Next steps (Phase 1 completion → Beta)
1. Repository settings (owner): enable private vulnerability reporting, branch protection on `main` (required CI checks, reviews), Dependabot alerts.
2. Pin GitHub Actions to commit SHAs (supply-chain hardening, SR-14).
3. DXGI Desktop Duplication capture; monitor selection; adaptive quality.
4. Windows service + Winlogon agent (UAC, lock screen), privilege-separated IPC.
5. Installer (MSI) with code signing via SignPath Foundation (free for OSS).
6. GUI per ADR-004; unattended access with OPAQUE.
