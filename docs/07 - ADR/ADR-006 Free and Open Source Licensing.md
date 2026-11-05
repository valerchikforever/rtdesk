---
tags: [adr, licensing, product]
status: accepted
updated: 2026-11-05
---

# ADR-006 Free and Open Source Licensing

- **Status:** accepted
- **Date:** 2026-11-05

## Context
Requirement from the product owner: the application must be **absolutely free**. A free product must still be sustainable, trustworthy (auditable) and protected from being turned into a closed paid fork that drops our security guarantees.

## Options considered
1. **Freeware, closed source** — free of charge but not auditable; contradicts [[Security Principles]] ("zero trust in infrastructure" must be verifiable).
2. **MIT / Apache-2.0** — maximum freedom; anyone can ship a closed, paid, possibly backdoored fork under a similar name.
3. **AGPL-3.0-or-later** — free and open; modified versions offered over a network (e.g. modified rendezvous/relay/console) must publish their source. Used by RustDesk for the same reasons.

## Decision
- All applications and servers: **AGPL-3.0-or-later**.
- Protocol specification (`proto/` + docs): **CC-BY-4.0**, so independent compatible implementations are possible.
- No CLA that would allow relicensing to a proprietary edition; contributions under DCO (`Signed-off-by`).
- No paid editions, ever; trademark policy only prevents confusing forks from using the name/logo.

## Consequences
- + Users get every feature for free; code is auditable.
- + Network-service forks must stay open.
- − Some companies avoid AGPL; acceptable because the product itself is free to use (AGPL obligations only apply when distributing/modifying).
- − Dependencies must be AGPL-compatible → `cargo-deny` license allowlist (MIT, Apache-2.0, BSD-2/3, ISC, Zlib, MPL-2.0, Unicode, CC0, GPL-3.0/AGPL-3.0).
- Funding via donations and grants — see [[Risk Register]] R-12.
