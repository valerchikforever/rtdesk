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

## Next steps (Phase 1 completion → Beta)
1. Fuzz targets (`cargo-fuzz`) for framing, relay parser, codec decoder, session admit.
2. DXGI Desktop Duplication capture; monitor selection; adaptive quality.
3. Windows service + Winlogon agent (UAC, lock screen), privilege-separated IPC.
4. Installer (MSI) with code signing via SignPath Foundation (free for OSS).
5. GUI per ADR-004; unattended access with OPAQUE.
6. CI pipeline per [[CI-CD and Release]].
