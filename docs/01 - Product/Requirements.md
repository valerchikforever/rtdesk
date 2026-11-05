---
tags: [product, requirements]
status: draft
updated: 2026-11-05
---

# Requirements

IDs: `FR-*` functional, `NFR-*` non-functional, `SR-*` security (security requirements are traced in [[Threat Model]]).

## Functional requirements
| ID | Requirement | Phase |
|---|---|---|
| FR-01 | Host displays a **Device ID** and a rotating **one-time code** (OTC) | MVP |
| FR-02 | Viewer connects using Device ID + OTC; host user must **accept** the session (attended mode) | MVP |
| FR-03 | Screen streaming with adaptive bitrate/framerate, multi-monitor switching | MVP |
| FR-04 | Mouse, keyboard (incl. special keys, layouts, Ctrl+Alt+Del on Windows via service) | MVP |
| FR-05 | Text clipboard sync (both directions, toggleable per session) | MVP |
| FR-06 | Unattended access using a device password (aPAKE) — opt-in, disabled by default | Beta |
| FR-07 | File transfer (drag & drop, resumable, integrity-checked) | Beta |
| FR-08 | In-session chat | Beta |
| FR-09 | Permission profiles per session: view-only, control, clipboard, files, terminal, restart | Beta |
| FR-10 | Windows: control of UAC prompts and login screen (service-based helper) | Beta |
| FR-11 | Reconnect after reboot (unattended only) | Beta |
| FR-12 | Address book, device groups, organization accounts | v1.0 |
| FR-13 | SSO (OIDC), 2FA (TOTP, WebAuthn) for viewer accounts | v1.0 |
| FR-14 | Central policy: who may access which devices, with which permissions, at which times | v1.0 |
| FR-15 | Audit log (who, when, from where, what permissions, duration, file transfers), SIEM export | v1.0 |
| FR-16 | Session recording (host-side policy; recording indicator always visible) | v1.0 |
| FR-17 | Self-hosted servers with zero dependency on our cloud | MVP |
| FR-18 | Web viewer (WebTransport/WebCodecs) | v1.x |
| FR-19 | Audio, remote terminal, Android host | v1.x |

## Non-functional requirements
| ID | Requirement |
|---|---|
| NFR-01 | Latency: see targets in [[Vision and Scope#Success metrics]] |
| NFR-02 | 1080p60 at ≤ 4 Mbit/s with hardware encoder; graceful degradation to 300 kbit/s |
| NFR-03 | Host idle CPU < 1 %, RAM < 60 MB |
| NFR-04 | Relay: ≥ 2 000 concurrent relayed sessions per 4 vCPU node; horizontally scalable |
| NFR-05 | Rendezvous: ≥ 100 000 online devices per node (keepalive load) |
| NFR-06 | Community servers: best effort; self-hosted servers must run on a $5/month VPS or a home Raspberry Pi 4 for ≤ 100 devices |
| NFR-11 | **Free of charge:** no feature may depend on payment, license key or account tier ([[ADR-006 Free and Open Source Licensing]]) |
| NFR-07 | Installer < 30 MB; portable (no-install) mode for attended sessions |
| NFR-08 | Accessibility: screen-reader friendly viewer UI; keyboard-only operation |
| NFR-09 | i18n from day one (Fluent / ICU message format) |
| NFR-10 | Reproducible builds for all release artifacts |

## Security requirements (summary)
Full rationale in [[Threat Model]], [[Cryptography Design]], [[Authentication and Authorization]].

| ID | Requirement |
|---|---|
| SR-01 | All session traffic is **end-to-end encrypted** between viewer and host; servers cannot decrypt |
| SR-02 | Mutual authentication; host identity pinned (TOFU + fingerprint, or org-CA) |
| SR-03 | Passwords/codes are never transmitted or stored in a form allowing offline brute-force by a server (PAKE / aPAKE) |
| SR-04 | Online brute-force limited: per-device exponential backoff + lockout + alert |
| SR-05 | Explicit, visible consent for attended sessions; persistent on-screen indicator during any session |
| SR-06 | Least privilege: privileged service does minimal work; UI and parsers run unprivileged |
| SR-07 | All parsers of network input are memory-safe and fuzzed |
| SR-08 | Signed binaries and **signed updates** with key rotation and rollback protection |
| SR-09 | No installation of root certificates, no test-signed drivers, no disabling of OS security features |
| SR-10 | Secrets at rest in OS keystore (DPAPI/Keychain/Secret Service/TPM where available) |
| SR-11 | Audit events are tamper-evident (hash-chained) and exportable |
| SR-12 | Server-side: rate limiting, abuse detection, no plaintext secrets in logs |
| SR-13 | Anti-scam safeguards in the attended flow (see [[Abuse and Scam Prevention]]) |
| SR-14 | Supply chain: SBOM, dependency audit, pinned toolchains, protected signing keys (HSM) |
