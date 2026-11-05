---
tags: [planning, team]
status: draft
updated: 2026-11-05
---

# Team and Roles

| Role | Count | Focus |
|---|---|---|
| Tech Lead / Architect | 1 | Architecture, protocol, ADRs |
| Security Lead | 1 | Threat model, crypto, SDLC, pentest coordination, abuse ops |
| Host/Platform engineers | 2–3 | Windows internals (service, Winlogon, IDD), macOS, Linux |
| Media engineer | 1–2 | Capture, encode/decode, congestion control |
| Networking / backend engineer | 1–2 | QUIC, NAT traversal, rendezvous, relay, console API |
| UI engineer | 1–2 | Viewer/host UI, mobile, console frontend |
| QA / Test automation | 1 | Device lab, E2E, performance regression |
| DevOps / Release | 1 (shared) | CI, signing, update infra, server ops |
| Product manager | 1 | Scope, users, legal coordination |

## Responsibilities (RACI excerpt)
| Activity | Tech Lead | Security Lead | Eng | Release |
|---|---|---|---|---|
| ADRs | A | C | R | I |
| Threat model | C | A/R | C | I |
| Crypto changes | C | A | R | I |
| Release signing | I | C | I | A/R |
| Incident response | R | A | R | R |

## Related
[[Roadmap]] · [[Secure SDLC]]
