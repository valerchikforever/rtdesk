---
tags: [security, hardening, windows, macos, linux]
status: draft
updated: 2026-11-05
---

# Platform Hardening

## Privilege separation (all platforms)
- `bridge-service` (privileged) exposes a **minimal IPC API**: `SpawnAgent(session_id, desktop)`, `InjectSAS()`, `ApplyVerifiedUpdate(path)`, `GetSecuritySettings()`, `SetSecuritySetting(…)` (requires elevation token proof), `SignWithDeviceKey(challenge)`.
- **No** generic "execute command", "write file", "read file" verbs in the service.
- Every IPC request: authenticate caller (PID → image path → signature check → expected SID), validate size and schema, log.

## Windows
| Control | Detail |
|---|---|
| Install location | `%ProgramFiles%\Bridge` (admin-only write). Never run binaries from user-writable paths |
| DLL loading | `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32 \| LOAD_LIBRARY_SEARCH_APPLICATION_DIR)`, delay-load hardening, no `LoadLibrary` with relative names |
| Process mitigations | CFG, CET shadow stacks, ASLR/high-entropy, DEP, `PROCESS_MITIGATION_BINARY_SIGNATURE_POLICY` (Microsoft-signed + own-signed only) for service, `ProhibitDynamicCode` where possible, child process policy for service |
| Named pipes | Explicit DACL, `PIPE_REJECT_REMOTE_CLIENTS`, `FILE_FLAG_FIRST_PIPE_INSTANCE`, impersonation level limited |
| Service | Runs as LocalSystem only where necessary for Winlogon; consider splitting: updater as separate service; service SID restricted |
| Drivers | Only if needed (IDD virtual display). Must be attestation/WHQL-signed. **Never** install test certs or enable testsigning |
| Unquoted service paths | Always quoted ImagePath |
| Updater | Downloads into SYSTEM-only dir, verifies TUF metadata + Authenticode, then executes; no `%TEMP%` |
| Portable mode | Runs with user privileges only; cannot access secure desktop; clearly labeled |
| Defender/SmartScreen | EV/OV code signing with stable publisher for reputation |

## macOS
- Hardened Runtime, notarization, no `com.apple.security.cs.disable-library-validation`.
- Privileged helper via `SMAppService`; XPC with audit-token-based caller verification (not PID — PID reuse races).
- TCC permissions requested with clear explanation; MDM PPPC profile documented for fleets.

## Linux
- Service under systemd with `ProtectSystem=strict`, `ProtectHome=read-only` (agent needs home for file transfer — agent runs as user, service doesn't), `NoNewPrivileges=yes`, `CapabilityBoundingSet=` minimal, `SystemCallFilter=@system-service`, `PrivateTmp=yes`.
- Agent: seccomp filter for decoder/parser subprocesses.
- Packages signed (deb/rpm repo keys), Flatpak sandbox for viewer.

## Servers
- Containers: distroless, non-root UID, read-only root fs, drop all capabilities, seccomp default profile.
- TLS: TLS 1.3 only (1.2 only for legacy console browsers, with strong suites), HSTS, OCSP stapling.
- Secrets from KMS/Vault, not env files in repos.
- Admin endpoints on separate internal listener with mTLS.
- Network policies: relay can't talk to DB; console signing service isolated.

## Viewer sandboxing
- Decoder + clipboard/rich-content parsers in a child process with restricted token/AppContainer (Win), App Sandbox (mac), seccomp+namespaces (Linux).

## Related
[[Host Agent]] · [[Threat Model]] · [[Secure SDLC]]
