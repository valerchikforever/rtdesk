---
tags: [operations, security, incident-response]
status: draft
updated: 2026-11-05
---

# Incident Response

## Severity levels
| Sev | Example | Response |
|---|---|---|
| SEV-1 | Signing key compromise, auth bypass exploited in the wild, mass account takeover | War room within 30 min, exec + legal, public communication |
| SEV-2 | Critical vuln reported (not exploited), server breach without key exposure | Fix within 72 h, advisory |
| SEV-3 | Abuse wave, partial outage | Ops on-call |

## Runbooks (to be written & rehearsed before GA)
- [ ] **Code-signing certificate compromise:** revoke, issue new cert, re-sign & redistribute, push TUF metadata excluding compromised targets, notify AV vendors. (Cf. AnyDesk 2024 in [[Lessons from Prior Incidents]].)
- [ ] **TUF key compromise:** rotate role keys via root (offline ceremony), threshold signing.
- [ ] **Rendezvous/relay breach:** rotate relay token keys and server identity keys; inform customers; confirm E2E guarantees (no content exposure by design) and assess metadata exposure.
- [ ] **Org CA compromise (console):** revoke org key on hosts via signed revocation list pushed through rendezvous + update channel; force re-enrollment.
- [ ] **Critical vuln in host agent:** emergency release, raise minimum protocol version, rendezvous refuses vulnerable clients (with clear UI message).
- [ ] **Mass credential stuffing:** force password reset, require 2FA, block offending IP ranges.
- [ ] **Scam campaign abusing product:** ban accounts/keys, publish IoCs, coordinate with banks/AV.

## Communication
- Status page, security advisories (GHSA + CVE), customer email list, `security.txt`.
- Post-incident review (blameless) within 10 days; actions tracked; this vault updated.

## Drills
Tabletop exercise each quarter; key-rotation drill twice a year.

## Related
[[Secure SDLC]] · [[CI-CD and Release]] · [[Observability]]
