---
tags: [planning, risk]
status: living
updated: 2026-11-05
---

# Risk Register

Project/delivery risks (security threats live in [[Threat Model]]).

| ID | Risk | L | I | Mitigation | Owner |
|---|---|---|---|---|---|
| R-01 | Low-latency video pipeline harder than estimated (driver quirks, HW encoder differences) | 4 | 4 | Phase 0 spike, early hardware lab (NVIDIA/AMD/Intel/Apple), software fallback | Media lead |
| R-02 | Windows secure desktop / UAC handling unstable | 3 | 4 | Dedicated spike, extensive manual test matrix, crash telemetry | Host lead |
| R-03 | Wayland restrictions block unattended Linux use | 4 | 2 | Document limitation; support X11; track portal/libei progress | Linux owner |
| R-04 | Code signing / driver attestation delays | 3 | 3 | Start EV cert + Microsoft Partner Center onboarding in Phase 0 | Release eng |
| R-05 | Product blocklisted by AV/banks due to scam abuse | 3 | 5 | [[Abuse and Scam Prevention]], vendor relationships, reputation building | PM + Security |
| R-06 | Crypto design flaw found late | 2 | 5 | External review in Phase 2 before public beta; standard protocols only | Security lead |
| R-07 | Community relay bandwidth costs exceed donations | 3 | 3 | Self-hosting as default, maximize P2P, fair-use limits, community-run relays, sponsorships | Infra lead |
| R-13 | ViGEmBus (virtual gamepad driver) is end-of-life; a Windows update could break it | 3 | 2 | Gamepad support degrades gracefully (feature flag); evaluate successor / own signed driver before v1.0 ([[ADR-008 Gamepad and Relative Mouse]]) | Host lead |
| R-12 | Unavoidable fixed costs for a free project (Apple Developer Program, pentests, servers) | 3 | 3 | GitHub Sponsors / Open Collective, SignPath Foundation (free OSS code signing), OSS-Fuzz, security audit grants (e.g. OSTIF, NLnet) | PM |
| R-08 | Key person dependency (crypto/Windows internals) | 3 | 4 | Pairing, documentation in this vault, ADRs | EM |
| R-09 | App store policy rejection (iOS/Android, macOS) | 2 | 3 | Follow guidelines early, viewer-only on iOS | Mobile lead |
| R-10 | Legal/regulatory: export controls (crypto), GDPR for metadata | 2 | 4 | Legal review in Phase 1, data minimization, DPA templates | PM |
| R-11 | License contamination (GPL dependency in proprietary build, or vice versa) | 2 | 3 | `cargo-deny` license policy; decide project license early (ADR) | EM |
