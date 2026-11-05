---
tags: [architecture, component, host]
status: draft
updated: 2026-11-05
---

# Host Agent

Runs on the controlled machine. Split into **three processes** for privilege separation (see [[Platform Hardening]]).

## Processes
| Process | Privilege | Responsibilities | Must NOT |
|---|---|---|---|
| `bridge-service` | SYSTEM / root | Start at boot, spawn `bridge-agent` into the active console session (and Winlogon desktop on Windows), apply updates after signature verification, hold device private key (or proxy to TPM) | Parse any network data, decode media, render UI |
| `bridge-agent` | Interactive user (or a dedicated low-priv user on Winlogon desktop) | Network/QUIC, PAKE, capture, encode, input injection, clipboard, file transfer | Run with admin token, write outside its sandbox dirs |
| `bridge-ui` | Interactive user | Tray icon, consent dialog, session indicator, settings | Talk to the network directly |

IPC: named pipes (Windows) / Unix domain sockets with **explicit ACLs** (only SYSTEM + the specific user SID), peer credential checks (`GetNamedPipeClientProcessId` + image signature check / `SO_PEERCRED`), length-prefixed protobuf messages with size limits.

## Windows specifics
- **Secure desktop (UAC, login screen):** the service spawns an agent instance on `winsta0\Winlogon` using `CreateProcessAsUser` with a token derived from the session — required to see/interact with UAC prompts. This is the riskiest code path; it gets dedicated review and fuzzing.
- **Ctrl+Alt+Del:** `SendSAS()` via service (requires policy `SoftwareSASGeneration`).
- **Headless hosts:** virtual display via IDD driver — must be **properly signed via Microsoft attestation/WHQL**. Never install test-signing certificates (see [[Lessons from Prior Incidents#RustDesk root certificate]]).
- **Privacy screen** (managed devices only): blank local monitor via Magnification API overlay or IDD; the local user still sees a "session in progress" notice.

## macOS specifics
- Requires user-granted TCC permissions: Screen Recording, Accessibility. Cannot be granted programmatically (except via MDM PPPC profile for managed fleets).
- Login window capture requires a LaunchDaemon helper; follow Apple's guidance, avoid private APIs.
- Hardened Runtime + notarization mandatory.

## Linux specifics
- Wayland: PipeWire via portal requires user consent dialog per session unless restore tokens are used; unattended Wayland is limited — document clearly.
- X11: works unattended; warn that X11 has no isolation between clients.

## State on disk
| Item | Location | Protection |
|---|---|---|
| Device private key | OS keystore / TPM-backed (Windows CNG + TPM, macOS Keychain/Secure Enclave for P-256 fallback, Linux Secret Service or file `0600` owned by root) | Never exported |
| OPAQUE registration record (unattended password) | Service-owned config | ACL SYSTEM only |
| Pinned viewer keys / trusted devices | Service-owned config | ACL SYSTEM only, HMAC'd |
| Audit log (local) | Service-owned, hash-chained | Append-only |
| Settings | Machine-wide (service) vs per-user (UI) | Security-relevant settings only changeable via service with admin auth |

> [!warning] Settings tampering
> Any setting that weakens security (enable unattended access, disable consent, change rendezvous server, trust an org key) must require **local admin elevation** and be logged. Malware or a scammer talking the user through the UI must not be able to flip them silently. See [[Abuse and Scam Prevention]].

## Related
[[System Architecture]] · [[Media Pipeline]] · [[Platform Hardening]]
