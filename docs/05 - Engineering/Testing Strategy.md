---
tags: [engineering, testing]
status: draft
updated: 2026-11-05
---

# Testing Strategy

| Level | Scope | Tooling |
|---|---|---|
| Unit | Pure logic: state machine, permission intersection, sanitizers, FEC | `cargo test`, `proptest` |
| Property-based | Protocol encode/decode roundtrip, sanitizer invariants (no output path escapes target dir) | `proptest` |
| Fuzzing | All network parsers, IPC messages, relay framing, file names, clipboard formats | `cargo-fuzz` (libFuzzer), ClusterFuzzLite in CI, OSS-Fuzz later |
| Integration | Viewer ↔ host ↔ rendezvous ↔ relay in-process | `tokio` test harness |
| Network simulation | NAT types, loss, jitter, bandwidth caps | Linux netns + `tc netem`, nftables NAT configs |
| E2E | Real installs on VMs: Windows 10/11, macOS, Ubuntu/Fedora (X11, Wayland) | Device lab, scripted with input/screen assertions |
| Security tests | Auth bypass attempts: messages out of state, permission escalation, replayed grants, wrong channel binding | Dedicated negative-test suite (must stay green) |
| Upgrade tests | Update from N-1, N-2; rollback; interrupted update | VM snapshots |

## Performance testing
- Automated latency measurement: host renders a timestamp/QR pattern, viewer captures and decodes → glass-to-glass latency per commit on reference hardware.
- Bandwidth/quality regression: VMAF/PSNR on standard screen-content clips at fixed bitrates.
- Server load tests: `k6`/custom Rust load generator for 100k simulated registrations, relay throughput.

## Mandatory security regression tests (examples)
- `InputEvent` before `SessionGranted` → connection closed, nothing injected.
- File chunk on session without `file_transfer` → closed.
- PAKE with mismatched channel binding → fails.
- 6th wrong OTC → OTC rotated, backoff active.
- Org grant with expired timestamp / wrong device / reused nonce → rejected.
- Received file name `..\..\Windows\System32\x.dll`, `CON`, `a:stream`, `\\server\share` → sanitized or rejected.

## Related
[[Secure SDLC]] · [[Media Pipeline]]
