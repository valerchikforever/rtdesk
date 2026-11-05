---
tags: [architecture, component, server]
status: draft
updated: 2026-11-05
---

# Rendezvous Server

Signaling/presence service, comparable in role to RustDesk's `hbbs` or an ICE signaling server.

## Responsibilities
1. **Registration:** host connects outbound (QUIC or TLS over TCP/443), proves possession of its device key (signs a server nonce), receives/keeps its Device ID.
2. **Presence:** keepalive every 25–30 s (keeps NAT mappings alive); presence stored in Redis with TTL.
3. **Lookup:** viewer asks for Device ID → server returns host public key + candidate endpoints. The host **signs** its candidate set + ephemeral info so a server can't silently substitute a key without breaking the signature for already-pinned viewers.
4. **Hole-punch coordination:** exchanges reflexive addresses, triggers simultaneous open. See [[NAT Traversal]].
5. **Relay allocation:** issues short-lived, HMAC-signed relay tokens (relay ID, session ID, expiry, bandwidth class) for [[Relay Server]].
6. **Rate limiting / abuse:** per-IP and per-account limits on lookups (prevents ID enumeration), connection-attempt counters per target device.

## Device ID allocation
- 9–10 digit numeric ID for usability, **assigned by server and bound to the device public key** on first registration. Re-registration of an existing ID requires a signature by the bound key.
- IDs are not secrets. Security never depends on ID secrecy.
- ID squatting/recycling: IDs inactive > 12 months may be recycled only after a quarantine; viewers with pinned keys see a key-change warning.

## What the server knows (metadata)
Device ID, public key, public IP, online status, who connected to whom and when. This is **sensitive metadata** → minimal retention (default 30 days for connection metadata, configurable), documented in privacy policy, logs pseudonymized.

## Hardening
- Stateless frontends; state in PostgreSQL + Redis.
- Protobuf messages with hard max sizes; per-connection memory caps; slow-loris timeouts.
- Separate admin API port bound to internal network only.
- Key pair for signing relay tokens and org operations stored in HSM/KMS on the community server; self-hosters may use a file key with `0600` permissions or a TPM/YubiHSM.

## Related
[[Relay Server]] · [[NAT Traversal]] · [[Threat Model]]
