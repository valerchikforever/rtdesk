---
tags: [architecture, component, server, console]
status: draft
updated: 2026-11-05
---

# Management Console

Web application + API for organizations. **Optional** — the product works peer-to-peer without it (attended sessions, personal unattended devices).

## Features
- Organizations, users, roles (Owner, Admin, Technician, Auditor, ReadOnly).
- Device enrollment (enrollment token → device key signed by org CA), groups, tags.
- Access policies: `who` (user/group) × `what` (device group) × `permissions` (view/control/files/terminal/restart) × `when` (schedule) × `conditions` (require host consent, require 2FA freshness, IP ranges).
- Address book synced to viewers.
- Audit log (from server events + device-uploaded signed audit records), SIEM export (syslog/CEF, webhook, S3).
- SSO: OIDC, SAML (later); SCIM provisioning (later).
- 2FA enforcement: WebAuthn (preferred), TOTP.

## Authorization model in org mode
The console issues a **short-lived signed access grant** (Ed25519, ≤ 5 min validity) naming viewer key, target device, permissions and session nonce. The host verifies it **offline** with the org public key pinned at enrollment, then still runs its local policy (e.g. "always require consent"). See [[Authentication and Authorization]].

> [!warning] The console becomes a high-value target
> Whoever controls the org signing key can access every enrolled device. Mitigations:
> - Org signing key in HSM/KMS; signing service separated from web tier.
> - Grants are logged on the device *before* the session starts.
> - Optional "two-person rule" for unattended access to sensitive device groups.
> - Optional policy "host consent always required" that the console cannot override (enforced on device).
> - Console admin actions require WebAuthn re-auth.

## Web security baseline
OWASP ASVS L2 target: CSP with nonces, `SameSite=Strict` session cookies, CSRF tokens, strict CORS, input validation, parameterized queries (`sqlx` compile-time checks), rate limits, account lockout with progressive delays, breached password check (k-anonymity), security headers, no setup wizard reachable after initialization (see ScreenConnect CVE-2024-1709 in [[Lessons from Prior Incidents]]).

## Related
[[Authentication and Authorization]] · [[Deployment]] · [[Observability]]
