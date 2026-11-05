---
tags: [glossary]
status: living
updated: 2026-11-05
---

# Glossary

| Term | Meaning |
|---|---|
| **Host** | The machine being controlled; runs the [[Host Agent]] |
| **Viewer** | The machine/app controlling the host; runs the [[Viewer Client]] |
| **Device ID** | Stable, human-friendly identifier of a host, bound to its public key by the [[Rendezvous Server]] |
| **Device key** | Long-term Ed25519 identity key pair generated on the device |
| **OTC** | One-time code shown on the host for attended sessions; input to PAKE |
| **Attended session** | Requires a human on the host to accept |
| **Unattended access** | Session without local acceptance, authorized by password or org policy |
| **Rendezvous** | Signaling server: presence, ID lookup, NAT traversal coordination. Never sees session content |
| **Relay** | Packet forwarder used when P2P fails. Sees only ciphertext ([[ADR-005 Relay Is Blind]]) |
| **PAKE** | Password-Authenticated Key Exchange (e.g. CPace, SPAKE2) |
| **aPAKE** | Augmented PAKE (e.g. OPAQUE): server side stores only a verifier |
| **TOFU** | Trust On First Use — pin the peer key on first successful authenticated connect |
| **Channel binding** | Tying an authentication step to a specific TLS session via exporter secret, preventing relay/MITM |
| **Secure desktop** | Windows desktop for UAC prompts / login screen (Winlogon) |
| **IDD** | Windows Indirect Display Driver — virtual monitor for headless hosts |
| **MOC** | Map of Content (Obsidian index note) |
| **ADR** | Architecture Decision Record |
