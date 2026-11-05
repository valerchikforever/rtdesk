---
tags: [security, sdlc, process]
status: draft
updated: 2026-11-05
---

# Secure SDLC

## Per change (every PR)
- [ ] PR template includes **security impact** checkbox: touches auth / crypto / parsers / IPC / installer / updater? → mandatory review by Security Champion.
- [ ] `cargo clippy -D warnings`, `cargo deny check` (licenses, advisories, bans, sources), `cargo audit`.
- [ ] New `unsafe` block → `// SAFETY:` comment + second reviewer + Miri test if possible.
- [ ] New network message type → added to fuzz corpus + state-machine test.
- [ ] Secret scanning (gitleaks) pre-commit and in CI.
- [ ] SAST: CodeQL (Rust, TS), Semgrep rules for project-specific patterns (e.g. logging of `Secret<T>`).

## Per milestone
- Threat model review ([[Threat Model]]) — update risks, add new threats.
- Dependency review via `cargo-vet` (trusted audits imported from Mozilla/Google/Bytecode Alliance).
- Continuous fuzzing on OSS-Fuzz / ClusterFuzzLite: protocol decoders, PAKE message parsing, file-name sanitization, clipboard parsing, relay framing.
- DAST against console (OWASP ZAP baseline in CI; Burp manual before release).

## Pre-release (Beta, 1.0, every major)
- External **penetration test** (host agent, protocol, servers, console).
- External **cryptographic review** of [[Cryptography Design]] and implementation.
- Optional: formal model of handshake in Tamarin/ProVerif.
- Release blocking: 0 open Critical/High; Mediums need documented risk acceptance.

## Ongoing
- **Vulnerability disclosure policy** + `security.txt` + bug bounty (start private, go public after 1.0).
- CVE numbering via GitHub Security Advisories (CNA) — publish advisories with fixed versions.
- SLA for fixes: Critical 72 h, High 14 days, Medium 60 days.
- Security champions in each team; annual secure-coding training (Rust-specific + Windows internals for host team).
- Staff security: phishing-resistant MFA (FIDO2) for all, separate admin accounts, production access via JIT/PAM, build environment isolated from corporate IT ([[Lessons from Prior Incidents]] #7).

## Related
[[Testing Strategy]] · [[CI-CD and Release]] · [[Incident Response]]
