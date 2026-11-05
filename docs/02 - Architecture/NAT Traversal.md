---
tags: [architecture, networking]
status: draft
updated: 2026-11-05
---

# NAT Traversal

Goal: maximize direct P2P (lower latency, lower relay cost) while falling back fast and reliably.

## Candidate gathering (ICE-like, simplified)
| Candidate | Source |
|---|---|
| Host (LAN) | Local interface addresses — only shared if LAN discovery enabled; otherwise only used when both peers report same public IP |
| Server-reflexive | Observed public IP:port reported by rendezvous (STUN-like, RFC 8489 semantics) from 2 different server ports/IPs to classify NAT behavior |
| IPv6 global | Direct, often no NAT — try first |
| Relay | Allocated on demand from [[Relay Server]] |

## Strategy
1. Classify NAT (endpoint-independent vs. address/port-dependent mapping) during registration.
2. Rendezvous sends both peers each other's candidates and a synchronized "go" time.
3. Both sides send QUIC Initial packets to all candidate pairs concurrently (Happy-Eyeballs-style, staggered 50 ms).
4. For port-dependent NAT on one side: port prediction (try ±N ports) — limited attempts to avoid looking like a scan.
5. **Race the relay:** after 1.5 s without success, allocate relay in parallel; first working path wins. Path can be **upgraded** to direct later via QUIC connection migration.
6. TCP/443 relay fallback when UDP is blocked entirely (detected at registration).

## Security considerations
- Candidate lists are signed by the originating peer (with its device key) so the rendezvous can't redirect traffic to a third party without detection by a pinned peer. Even if redirected, TLS with pinned keys + PAKE prevents MITM — redirection degrades to DoS.
- Don't leak private LAN addresses to the rendezvous by default (privacy, internal network mapping).
- Limit hole-punch packet rates to avoid being an amplification/scanning tool.

## Metrics to collect (opt-in telemetry)
NAT type distribution, direct-vs-relay ratio, time-to-connect per path, hole-punch failure reasons.

## Related
[[Rendezvous Server]] · [[Relay Server]] · [[Wire Protocol]]
