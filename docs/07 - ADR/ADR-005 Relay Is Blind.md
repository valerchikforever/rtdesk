---
tags: [adr, security, networking]
status: proposed
updated: 2026-11-05
---

# ADR-005 Relay Is Blind

- **Status:** proposed
- **Date:** 2026-11-05

## Context
Relays carry a significant share of sessions. If a relay terminates encryption (hop-by-hop), a compromised or malicious operator — including a self-hosting admin or our own cloud after a breach — could read screens and inject input.

## Decision
Relays never terminate the session's cryptography. They pair two allocations by a signed token and forward bytes. All session confidentiality/integrity/authentication is end-to-end ([[Cryptography Design]]).

## Consequences
- + Relay compromise ⇒ at most DoS and traffic-metadata exposure (timing, volume).
- + Relays can be run by third parties/customers without trust.
- − No server-side content features (server-side recording, transcoding, content inspection). Recording, if needed, is done by an endpoint under org policy.
- − Traffic analysis (e.g. inferring typing from packet timing) remains possible → consider optional padding/constant-rate input mode for high-security deployments (backlog).
