---
tags: [operations, deployment]
status: draft
updated: 2026-11-05
---

# Deployment

## Self-host: minimal (≤ 1 000 devices)
```yaml
# docker-compose.yml (sketch)
services:
  rendezvous:
    image: bridge/rendezvous:1.0.0@sha256:...
    ports: ["21115:21115/udp", "443:8443/tcp"]
    read_only: true
    user: "65532:65532"
    volumes: ["./data:/data"]        # SQLite + server keys
  relay:
    image: bridge/relay:1.0.0@sha256:...
    ports: ["21117:21117/udp", "8444:8444/tcp"]
    read_only: true
    user: "65532:65532"
```
- Server identity key printed on first start → admin distributes its fingerprint (or a signed config bundle) to clients.
- Clients configured via signed config / MSI properties / MDM profile — **not** via installer filename tricks or unsigned appended data.

## Production (community server / large self-host)
- Rendezvous: ≥ 3 nodes per region, L4 LB, Redis cluster for presence, PostgreSQL (HA, PITR backups, encrypted at rest).
- Relays: autoscaled pool per region, anycast or GeoDNS; health-based selection by rendezvous.
- Console: stateless API pods behind WAF/reverse proxy; **signing service** separate (HSM/KMS), reachable only from API over mTLS.
- Infrastructure as code (Terraform/OpenTofu + Helm), GitOps, no manual changes in prod.
- Environment separation: prod / staging / dev accounts; build & signing infra in its own isolated account.

## Data retention defaults
| Data | Retention |
|---|---|
| Connection metadata | 30 days (configurable) |
| Audit logs (org) | 1 year (configurable, org-owned) |
| Server logs | 14 days, pseudonymized IPs |
| Session recordings | Stored org-side; never on the community server |

## Related
[[System Architecture]] · [[Observability]]
