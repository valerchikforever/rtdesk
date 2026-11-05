---
tags: [security, threat-model]
status: draft
updated: 2026-11-05
review_cadence: every milestone + on any protocol change
---

# Threat Model

Methodology: data-flow diagram + **STRIDE** per element, risk = likelihood × impact (1–5 each). Mitigation IDs `M-*` are referenced from code (`// SEC: M-07`) and tests.

## Assets
| ID | Asset | Why it matters |
|---|---|---|
| A1 | Control of host (input injection) | Full compromise of the user's machine |
| A2 | Screen contents, clipboard, files | Confidential data (passwords, banking, PII) |
| A3 | Device private keys, OPAQUE records, org signing key | Impersonation / mass access |
| A4 | User accounts, 2FA, sessions in console | Access to address books → fleet access |
| A5 | Update & code-signing keys | Supply-chain compromise of all users |
| A6 | Metadata (who connects to whom, IPs) | Privacy, targeting |
| A7 | Service availability | Helpdesk outage |

## Adversaries
| ID | Adversary | Capability |
|---|---|---|
| ADV1 | Remote internet attacker | Knows/guesses Device IDs, sends arbitrary packets |
| ADV2 | Network attacker (MITM on Wi-Fi/ISP) | Read/modify/drop traffic |
| ADV3 | Compromised/malicious rendezvous or relay operator | Full control of servers |
| ADV4 | Malicious peer (malicious host attacking viewer, or vice versa) | Valid session endpoint, crafted messages |
| ADV5 | Local unprivileged malware on host | Runs as user, wants SYSTEM or to enable unattended access |
| ADV6 | Social engineer / tech-support scammer | Convinces victim to install and grant access |
| ADV7 | Supply-chain attacker | Targets dependencies, CI, signing keys, update channel |
| ADV8 | Insider / compromised org admin | Abuses console to access devices |
| ADV9 | Credential stuffer | Reused passwords against viewer accounts |

## Trust boundaries
TB1 Internet ↔ host agent · TB2 host agent ↔ privileged service (IPC) · TB3 viewer ↔ host (peer) · TB4 clients ↔ servers · TB5 console web ↔ browser · TB6 CI ↔ release infra · TB7 user ↔ UI (human factors)

## STRIDE analysis

### Spoofing
| ID | Threat | Risk | Mitigations |
|---|---|---|---|
| T-01 | Server substitutes host public key → MITM | 4×5 | M-01 key pinning (TOFU + fingerprint), M-02 PAKE channel-bound to TLS exporter (server can't complete without OTC/password), M-03 org-CA-signed device certs |
| T-02 | Attacker registers someone's Device ID | 3×4 | M-04 ID bound to key at registration; re-register requires signature |
| T-03 | Viewer account takeover via credential stuffing | 4×5 | M-05 mandatory 2FA for unattended/org access, WebAuthn, breached-password check, device approval for new viewer devices, rate limits |
| T-04 | Phishing UI / fake support site distributes trojanized client | 4×5 | M-06 code signing + reputation, M-07 in-app warnings, official-download-only guidance |
| T-05 | Spoofed URI handler invocation from web page | 3×4 | M-08 URI only pre-fills ID; never passes credentials/paths/flags |

### Tampering
| ID | Threat | Risk | Mitigations |
|---|---|---|---|
| T-10 | Modify traffic in transit | 2×5 | QUIC/TLS 1.3 AEAD; no plaintext mode exists |
| T-11 | Malicious update pushed via compromised update server | 3×5 | M-11 TUF-style signed metadata, threshold keys, rollback/freeze protection, key in HSM |
| T-12 | Local malware flips "unattended access on" or sets its own password | 4×5 | M-12 security settings only via service with admin elevation; notification to user + audit; settings HMAC'd |
| T-13 | Tampered installer config (rendezvous address, org key embedded in signed installer's unsigned area) | 3×5 | M-13 no config in unauthenticated PE regions; custom-server config must be signed or entered/confirmed locally |
| T-14 | DLL search-order hijacking of service / installer | 3×5 | M-14 `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32)`, absolute paths, install under `Program Files` ACLs, verify loaded module signatures |

### Repudiation
| ID | Threat | Mitigations |
|---|---|---|
| T-20 | Technician denies performing an action | M-20 hash-chained audit on host + server, viewer key signatures on session start, optional session recording |
| T-21 | Audit log deleted by local admin/malware | M-21 forward audit events to console in near-real-time; local chain detects truncation |

### Information disclosure
| ID | Threat | Risk | Mitigations |
|---|---|---|---|
| T-30 | Relay/rendezvous reads session content | 3×5 | E2E encryption; relay blind ([[ADR-005 Relay Is Blind]]) |
| T-31 | Offline brute-force of password from captured handshake or server DB | 3×5 | M-31 PAKE / OPAQUE: no offline-verifiable transcript; no password-equivalent stored server-side |
| T-32 | Device ID enumeration → who is online | 3×2 | M-32 lookup rate limiting, no presence disclosure to unauthenticated viewers beyond "can attempt" |
| T-33 | Secrets in logs / crash dumps | 3×4 | M-33 redaction layer, `zeroize`, crash dumps scrubbed, minidumps without heap by default |
| T-34 | Clipboard/screen leaks after session end | 2×4 | Clear transferred clipboard on disconnect (option), stop capture immediately |
| T-35 | NTLM hash leak via UNC path in URI/file path | 3×4 | Reject UNC paths in all external inputs |

### Denial of service
| ID | Threat | Mitigations |
|---|---|---|
| T-40 | Flood rendezvous/relay | Rate limits, QUIC retry tokens (address validation), anycast/CDN-shield for TCP/443, autoscaling |
| T-41 | Connection-request spam to a host (annoyance, consent-fatigue) | Per-target rate limit, block list, "do not disturb", consent prompt cooldown |
| T-42 | Resource exhaustion by crafted messages | Size limits, bounded queues, per-session memory caps |

### Elevation of privilege
| ID | Threat | Risk | Mitigations |
|---|---|---|---|
| T-50 | Memory corruption in protocol parser → RCE | 3×5 | Rust, fuzzing, no pre-auth complex parsing (pre-auth surface = QUIC + small auth messages) |
| T-51 | Local user → SYSTEM via service IPC | 3×5 | M-51 IPC ACLs, caller verification, minimal service API, no "run command as SYSTEM" verb |
| T-52 | Viewer escapes permissions (e.g. sends file messages when files disabled) | 3×4 | Host-side dispatcher enforcement + state machine (see [[Wire Protocol]]) |
| T-53 | Malicious host exploits viewer decoder | 3×5 | Sandboxed decoder process |
| T-54 | Installer/updater EoP (writable temp dirs, TOCTOU) | 3×5 | Updates staged in SYSTEM-only dir, verify-then-execute from protected path, no `%TEMP%` execution |
| T-55 | Session continues on secure desktop after viewer should lose access | 2×5 | Single session authority in service; terminating the session kills Winlogon agent too |
| T-56 | Virtual input devices (gamepads) outlive the session and keep being controllable | 2×4 | Pads owned by a session-scoped thread; dropped (unplugged) at session end; never created without CONTROL |
| T-57 | Stuck inputs (key, button, stick) after focus loss or disconnect | 3×3 | Viewer releases keys and sends neutral pad state on focus loss; host unplugs pads on disconnect |
| T-58 | Viewer locks the local mouse and the user cannot get out | 2×2 | Ctrl+Alt+M always releases; focus loss always releases; auto mode can be disabled |

### Human factors / abuse (beyond STRIDE)
| ID | Threat | Mitigations |
|---|---|---|
| T-60 | Tech-support scam: victim grants access | [[Abuse and Scam Prevention]] |
| T-61 | Stalkerware usage (hidden unattended access by partner) | Persistent indicator, periodic "unattended access is enabled" reminder, no hidden mode |
| T-62 | Consent fatigue — user clicks "accept" reflexively | Show viewer identity/verification level, permission choice in dialog, cooldown |

## Open questions
> [!todo]
> - Formal verification of auth state machine (e.g. Tamarin/ProVerif model of handshake + PAKE binding) — budget in Phase 2?
> - Policy for law-enforcement requests regarding metadata (community server).
> - Wayland unattended story and its security implications.

## Related
[[Cryptography Design]] · [[Authentication and Authorization]] · [[Platform Hardening]] · [[Risk Register]]
