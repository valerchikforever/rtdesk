---
tags: [security, crypto]
status: draft
updated: 2026-11-05
---

# Cryptography Design

> [!warning] Rule
> No custom primitives, no custom protocols where a standard exists. Any change to this note requires review by a cryptography-competent reviewer and an external audit before release.

## Keys
| Key | Type | Generated | Stored | Lifetime |
|---|---|---|---|---|
| Device identity key (host & viewer) | Ed25519 | On install | OS keystore / TPM-wrapped | Until reinstall/rotation |
| TLS certificate | Self-signed cert carrying device public key (or RFC 7250 raw public key) | Per start | Memory | Per process |
| QUIC session keys | TLS 1.3 key schedule (X25519 / X25519MLKEM768 hybrid when available) | Per connection | Memory, zeroized | Per connection |
| OTC | 9–10 chars, ~40+ bits entropy (base32 without ambiguous chars) | Host, rotates after each use or every N min | Memory | Single use |
| Unattended password record | OPAQUE registration record | When user sets password | Service ACL'd file | Until changed |
| Org CA key | Ed25519 | Org creation | HSM/KMS | Rotatable, with overlap |
| Relay token key | HMAC-SHA256 | Server | KMS | Rotated weekly |
| Update signing keys | Ed25519 (TUF roles: root, targets, snapshot, timestamp) | Offline ceremony | HSM + offline root | Root: years; timestamp: days |

## Handshake
1. **Transport:** QUIC with TLS 1.3, mutual authentication with device keys. Viewer verifies the host key against pin (TOFU) or org CA; host records viewer key.
   - Cipher suites: `TLS_AES_128_GCM_SHA256`, `TLS_CHACHA20_POLY1305_SHA256`, `TLS_AES_256_GCM_SHA384`. Key exchange: X25519; enable **hybrid post-quantum** (X25519MLKEM768) when supported by rustls on both ends — protects recorded sessions against future "harvest now, decrypt later".
2. **Channel binding:** both sides derive `cb = TLS-Exporter("EXPORTER-bridge-auth-v1", "", 32)` (RFC 5705/8446 exporter).
3. **Session authentication** (one of):
   - **Attended (OTC):** balanced PAKE **CPace** (or SPAKE2) with `password = OTC`, `context = cb ‖ host_pub ‖ viewer_pub`. Success yields key confirmation MACs exchanged on the control stream.
   - **Unattended password:** **OPAQUE** (aPAKE) — host = server role storing only the registration record; viewer = client. Associated data bound to `cb`.
   - **Org grant:** host verifies Ed25519 signature of org CA over `{viewer_pub, device_id, perms, nonce, cb_hash?, exp}`; nonce provided by host in the same session, preventing replay.
   - **Trusted device (remember me):** host has pinned the viewer key + per-pair secret from an earlier authenticated session; still requires host policy to allow.
4. Only after step 3 succeeds does the host transition to consent/policy ([[Wire Protocol#Session state machine (host side)]]).

### Why PAKE + channel binding?
- Rendezvous (or anyone) acting as MITM would have to terminate TLS separately with each side; the exporters differ, so the PAKE fails — and an **online** guess costs one attempt against a rate-limited host.
- Captured transcripts give no offline dictionary attack (unlike "send hash of password" schemes).
- A stolen host config (OPAQUE record) still requires an offline dictionary attack per password, not a direct login (use strong KSF: Argon2id in OPAQUE).

## Rate limiting of guesses (host-enforced)
- OTC: max 5 failed attempts → OTC rotates; global per-host: exponential backoff (1 s, 2 s, 4 s … cap 15 min).
- Password: 5 failures → 15 min lockout + notification on host + audit + optional email via console.
- With ~40 bits OTC entropy and these limits, online guessing success probability is negligible.

## Post-quantum stance
- Key exchange: hybrid ML-KEM when available (priority).
- Signatures: Ed25519 for now; monitor ML-DSA adoption for update signing (long-lived root keys) — plan root key migration path in TUF.

## Implementation rules
- Constant-time comparisons (`subtle`), `zeroize` for secrets, no secrets in `Debug` impls (use newtype wrappers).
- RNG: OS CSPRNG (`getrandom`) only.
- Crypto libraries pinned and covered by `cargo-vet` audits.

## Related
[[ADR-003 PAKE-Based Session Authentication]] · [[Authentication and Authorization]] · [[Threat Model]]
