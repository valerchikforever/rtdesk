---
tags: [product, scope]
status: draft
updated: 2026-11-05
---

# Vision and Scope

## Vision
A remote desktop tool that IT teams and individuals can **self-host** and **trust**: the operator of the servers (including us) can never see screen contents, keystrokes or files; every access is consented, authenticated, authorized and audited; and the product is hard to weaponize for tech-support scams.

## Free forever
> [!decision] The application is completely free
> No paid tiers, no license keys, no feature gating, no "commercial use" detection, no ads, no data monetization. Every feature — including organization management, SSO, audit and session recording — is available to everyone. Source code is open ([[ADR-006 Free and Open Source Licensing]]).

Consequences of this decision:
- **Self-hosting is the primary deployment model.** The project may run an optional community rendezvous/relay with fair-use limits that exist only for anti-abuse, never to upsell.
- Funding for unavoidable costs (Apple Developer Program, community servers, pentests) comes from donations/sponsorships (GitHub Sponsors, Open Collective) and free OSS programs (e.g. SignPath Foundation for Windows code signing, OSS-Fuzz for fuzzing).
- No telemetry required for anything; opt-in only.

## Problem statement
Commercial tools (TeamViewer, AnyDesk, ScreenConnect) are closed-source, have had serious vendor-side breaches (see [[Lessons from Prior Incidents]]) and are the #1 tool of tech-support scammers. Open-source alternatives exist (RustDesk), but we see room for a design that is **secure by default** rather than secure by configuration:
- PAKE-based authentication so passwords are never exposed to servers or offline brute force ([[ADR-003 PAKE-Based Session Authentication]]).
- A relay that is cryptographically blind ([[ADR-005 Relay Is Blind]]).
- Mandatory key pinning, signed updates, no root-certificate or unsigned-driver shortcuts.
- Built-in anti-scam UX ([[Abuse and Scam Prevention]]).

## Target users
| Persona | Need |
|---|---|
| **Helpdesk technician** (SMB / MSP) | Fast attended sessions, unattended access to managed fleet, audit trail |
| **Power user** | Reach own home PC from laptop/phone |
| **Enterprise IT / SecOps** | Self-hosted servers, SSO, policy, SIEM export, compliance |
| **End user being helped** | Simple "give a code" flow, clear understanding of who is connecting and what they can do |

## In scope (v1.0)
- Platforms: **Host** — Windows 10/11 (x64, arm64), macOS 13+, Linux (X11 + Wayland via portals). **Viewer** — same + Android, iOS (viewer only), Web viewer (later phase).
- Attended sessions via one-time code; unattended access via device password and/or organization policy.
- Screen view/control, multi-monitor, clipboard (text, then files), file transfer, chat, session recording (opt-in, host-side policy), reboot-and-reconnect, UAC/secure-desktop support on Windows.
- Self-hostable rendezvous + relay servers (single binary each, Docker images).
- Management console (address book, groups, policies, audit log) — phase 3.

## Out of scope (v1.0)
- Audio streaming (planned v1.1), printer redirection, USB redirection.
- Remote host on mobile (Android host is v1.x; iOS cannot technically be a host).
- Proprietary cloud-only features; everything must work self-hosted.
- Stealth/hidden operation of any kind — see [[Abuse and Scam Prevention]]. **We will never ship a mode that hides the session from the local user** except the explicit, policy-gated "privacy screen" for managed devices, which still shows an indicator.

## Success metrics
- Time-to-first-frame < 2 s on direct connection (p50), < 4 s via relay.
- Glass-to-glass latency < 80 ms on LAN, < 150 ms over relay at 50 ms RTT.
- 0 critical / high findings open at release from external pentest.
- ≥ 85 % of sessions direct P2P (not relayed) in field telemetry (opt-in).

## Related
[[Requirements]] · [[Roadmap]] · [[Threat Model]]
