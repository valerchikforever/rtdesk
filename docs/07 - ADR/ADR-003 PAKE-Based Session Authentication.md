---
tags: [adr, security, crypto]
status: proposed
updated: 2026-11-05
---

# ADR-003 PAKE-Based Session Authentication

- **Status:** proposed
- **Date:** 2026-11-05

## Context
Users authenticate with short codes and memorable passwords. Common designs send a (salted) hash or challenge-response over the encrypted channel; if the channel is MITM'd by a server or a transcript is captured, the secret is exposed to **offline** brute force. The rendezvous server also mediates first contact and could substitute keys.

## Options considered
1. Hash/challenge-response inside TLS — simple; vulnerable to offline guessing if TLS endpoint isn't truly the host (server MITM on first contact).
2. **Balanced PAKE (CPace/SPAKE2)** for one-time codes — mutual auth from low-entropy secret; only online guessing.
3. **Augmented PAKE (OPAQUE)** for stored passwords — host stores only a record; no password-equivalent at rest.
4. Certificates only (no passwords) — great for org mode, unusable for ad-hoc support.

## Decision
- Attended: CPace (fallback SPAKE2 if library maturity requires) with OTC.
- Unattended: OPAQUE with Argon2id KSF.
- Org mode: signed grants (certificates) — no passwords.
- All bound to the QUIC/TLS session via exporter channel binding.

## Implementation note (2026-11-05)
MVP uses **SPAKE2 (symmetric, `spake2` crate)** for attended sessions. Identity input = `"bridge-pake-v1" ‖ TLS exporter ‖ host key ‖ viewer key`; key confirmation with role-labelled HMAC-SHA256. Status for the attended part: **accepted**. OPAQUE follows with unattended access (Beta).

## Consequences
- + Server/relay MITM impossible without knowing the secret; offline guessing eliminated.
- + Strong story for audits/marketing ("we never see your password").
- − Library maturity must be evaluated (`opaque-ke` audited; CPace crates to be reviewed) — Phase 0 spike.
- − More complex than hash-based auth → external crypto review mandatory.
