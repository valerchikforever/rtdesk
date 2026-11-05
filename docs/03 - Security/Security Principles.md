---
tags: [security, principles]
status: accepted
updated: 2026-11-05
---

# Security Principles

A remote access tool is, by definition, a **legitimate backdoor**. Every design choice is judged against: *"Could this let someone other than the intended, consenting party control the machine?"*

1. **Zero trust in infrastructure.** Rendezvous, relay and our own cloud are assumed compromisable. Confidentiality and authentication are end-to-end. ([[ADR-005 Relay Is Blind]])
2. **Secure by default, not by configuration.** Unattended access off; consent on; LAN listener off; clipboard-files off; strict key pinning on.
3. **Host is sovereign.** The host makes the final access decision and enforces permissions. Viewers and servers only request.
4. **Visible always.** No invisible sessions. Indicator + log entry for every session.
5. **Least privilege & privilege separation.** Privileged code is tiny and never parses network input. ([[Platform Hardening]])
6. **Memory safety.** Rust for all network-facing code; `unsafe` confined to FFI modules with documented invariants and review. ([[ADR-001 Rust for Core and Servers]])
7. **Don't roll your own crypto.** Standard protocols (TLS 1.3, QUIC, OPAQUE/CPace), audited libraries. ([[Cryptography Design]])
8. **Defense in depth against people.** Design for scams, social engineering and credential reuse — not just protocol attackers. ([[Abuse and Scam Prevention]])
9. **Integrity of what we ship.** Signed, reproducible builds; signed updates; signing keys in HSM; vendor breach planned for. ([[CI-CD and Release]])
10. **Assume breach, keep evidence.** Tamper-evident audit logs, incident runbooks, key rotation drills. ([[Incident Response]])
11. **Learn from others' incidents.** ([[Lessons from Prior Incidents]])
