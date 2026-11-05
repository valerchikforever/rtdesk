---
tags: [architecture, component, server]
status: draft
updated: 2026-11-05
---

# Relay Server

Forwards traffic between viewer and host when direct P2P fails (symmetric NATs, UDP-blocked corporate networks). Comparable to `hbbr` / TURN.

> [!decision] The relay is blind
> The relay forwards **opaque QUIC datagrams**; the end-to-end QUIC/TLS session runs *through* it. The relay never holds session keys. See [[ADR-005 Relay Is Blind]].

## Modes
| Mode | Transport client↔relay | Use |
|---|---|---|
| UDP relay | Raw UDP, framed with a 16-byte allocation header (TURN-ChannelData-like) | Default fallback |
| TCP/TLS-443 relay | TLS 1.3 over TCP:443, length-prefixed datagram frames (concept similar to MASQUE CONNECT-UDP, RFC 9298) | Restrictive firewalls / proxies |
| WebSocket/WebTransport | For web viewer and HTTP-proxy-only environments | Later phase |

## Allocation flow
1. Rendezvous issues `RelayToken = HMAC(k_relay, session_id ‖ relay_id ‖ role ‖ expiry ‖ bw_class)` to both peers.
2. Each peer connects to the relay and presents the token. The relay pairs the two halves by `session_id`; a token is single-use per role.
3. Unpaired allocations expire after 10 s; idle sessions after 60 s with no packets.

## Abuse & DoS controls
- Tokens required — the relay is not an open proxy.
- Per-session bandwidth cap (bw_class), per-account/per-IP concurrency limits, global fair-queuing.
- No amplification: relay never sends more bytes to a peer than it received from the counterpart.
- Accounting: bytes per session for capacity planning/abuse analytics (no billing — the product is free) — **no payload inspection** (it's encrypted anyway).

## Scaling
- Stateless except in-memory pairing table; region-aware selection by rendezvous (lowest RTT from both peers).
- Kernel bypass/`recvmmsg`/`sendmmsg` + `SO_REUSEPORT` for throughput; consider eBPF/XDP forwarding later.

## Related
[[Rendezvous Server]] · [[NAT Traversal]] · [[Cryptography Design]]
