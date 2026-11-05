---
tags: [security, incidents, research]
status: living
updated: 2026-11-05
---

# Lessons from Prior Incidents

Publicly reported incidents and vulnerabilities in remote-access products, and what we do differently. 

> [!warning] Verify before citing externally
> Summaries are from public reporting as recalled by the team. Before quoting in external material, verify details (affected versions, CVSS, dates) against NVD / vendor advisories and add the source link to each entry.

| # | Incident | What happened (summary) | Our lesson / control |
|---|---|---|---|
| 1 | **RDP BlueKeep** — CVE-2019-0708 | Pre-authentication RCE in Microsoft RDP, wormable | Minimal **pre-auth attack surface**; memory-safe parsing; no listening ports on host by default ([[Threat Model]] T-50) |
| 2 | **AnyDesk** — CVE-2020-13160 | Format-string bug in LAN discovery (UDP) leading to code execution | LAN discovery **off by default**; fuzz every network parser; Rust eliminates format-string class |
| 3 | **TeamViewer** — CVE-2020-13699 | URI handler parameters not properly quoted; a malicious web page could make the client connect to an attacker SMB share and leak NTLM hashes | URI handler only pre-fills ID; reject UNC/remote paths everywhere ([[Viewer Client]], T-05, T-35) |
| 4 | **TeamViewer account takeovers (2016)** | Wave of accounts hijacked, attributed by vendor largely to password reuse; attackers used unattended access to victims' PCs | Mandatory 2FA for unattended rights, new-device approval, breached-password checks ([[Authentication and Authorization]]) |
| 5 | **ConnectWise ScreenConnect** — CVE-2024-1709 / CVE-2024-1708 | Authentication bypass via setup wizard reachable after install, combined with path traversal; mass exploitation | Setup/initialization endpoints **permanently disabled** after init; strict path canonicalization; emergency patch process ([[Management Console]]) |
| 6 | **AnyDesk vendor breach (early 2024)** | Production systems compromised; code-signing certificate revoked and replaced, portal passwords reset | Signing keys in **HSM**, production segmented, signing-key rotation runbook, update metadata via TUF so a single key compromise isn't fatal ([[CI-CD and Release]], [[Incident Response]]) |
| 7 | **TeamViewer corporate breach (mid-2024)** | Corporate IT environment compromised (attributed to APT29) via an employee account; vendor stated product environment was segregated | Strict separation of corporate IT and product/build environments; phishing-resistant MFA for staff |
| 8 | **RustDesk** — CVE-2024-25140 | Windows installer (1.2.3) installed a self-signed "WDKTestCert" into the trusted root store to load its virtual display driver | **Never** install root certs or test-signing; drivers only via Microsoft attestation signing (SR-09) |
| 9 | **Signed-installer config abuse** (reported for several RMM tools) | Attackers append/modify configuration in unauthenticated regions of a signed installer, creating "legitimately signed" malware that connects to attacker servers | No config in unsigned PE areas; custom server settings must be signed by org key or confirmed locally (T-13) |
| 10 | **Tech-support scams** (industry-wide, ongoing) | Legit tools used to defraud victims; tools get blocked by banks/AV | Built-in anti-scam UX & abuse ops ([[Abuse and Scam Prevention]]) |
| 11 | **Weak local credential storage** (various tools) | Saved/permanent passwords recoverable from config files with reversible obfuscation | Store only OPAQUE records / OS-keystore-protected secrets (SR-10) |

> [!todo]
> - Add source links for each row.
> - Review RustDesk's GitHub security advisories and issue tracker for additional design-level lessons (e.g. public-server metadata exposure, permanent password handling).
> - Track new CVEs in remote-access products quarterly (owner: Security Lead).

## Related
[[Threat Model]] · [[Security Principles]]
