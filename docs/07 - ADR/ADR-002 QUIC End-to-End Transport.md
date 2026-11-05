---
tags: [adr, networking]
status: proposed
updated: 2026-11-05
---

# ADR-002 QUIC End-to-End Transport

- **Status:** proposed
- **Date:** 2026-11-05

## Context
We need encrypted, multiplexed, low-latency transport that works P2P through NATs, via a relay, and over TCP/443 in restrictive networks; video wants unreliable delivery, control wants reliability.

## Options considered
1. **TCP + TLS** (RustDesk-like custom framing + NaCl) — simple; head-of-line blocking hurts video; TCP hole punching unreliable.
2. **WebRTC** (DTLS-SRTP + SCTP) — standard, browser-compatible; large, complex stack, harder to control codecs/congestion, DTLS cert auth awkward for our key model.
3. **QUIC (quinn + rustls)** with datagrams — TLS 1.3 built in, streams + datagrams, connection migration (path upgrade relay→direct), UDP-friendly hole punching; browser path via WebTransport later.
4. **Custom UDP + Noise** — maximum control; we'd re-implement congestion control, loss recovery, etc.

## Decision
QUIC end-to-end between viewer and host. The relay forwards opaque UDP datagrams (or frames them inside TLS/TCP-443 tunnels) without terminating QUIC.

## Consequences
- + Single, standard security layer; relay blindness falls out naturally ([[ADR-005 Relay Is Blind]]).
- + Path migration lets us start on relay and upgrade to P2P.
- − QUIC-over-TCP tunnel adds overhead and double congestion control in fallback mode — acceptable for rare case.
- − Some enterprise firewalls block UDP/443 → TCP fallback mandatory.
