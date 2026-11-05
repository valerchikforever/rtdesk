---
tags: [engineering, repo]
status: draft
updated: 2026-11-05
---

# Repository Layout

Monorepo, Cargo workspace.

```
bridge/
├─ Cargo.toml                  # workspace
├─ rust-toolchain.toml         # pinned toolchain
├─ deny.toml                   # cargo-deny policy
├─ supply-chain/               # cargo-vet audits
├─ proto/                      # .proto definitions (single source of truth)
├─ crates/
│  ├─ bridge-proto/            # generated types + strict decoding, size limits
│  ├─ bridge-crypto/           # PAKE, key mgmt, channel binding (small, heavily reviewed)
│  ├─ bridge-transport/        # quinn setup, NAT traversal, relay client
│  ├─ bridge-session/          # state machine, permission dispatcher
│  ├─ bridge-media/            # codec abstraction, packetizer, FEC, congestion control
│  ├─ bridge-capture/          # platform capture backends (cfg per OS)
│  ├─ bridge-input/            # platform input injection
│  ├─ bridge-ipc/              # service<->agent<->ui IPC with peer verification
│  ├─ bridge-platform-win/     # Windows-only FFI (unsafe concentrated here)
│  ├─ bridge-platform-mac/
│  ├─ bridge-platform-linux/
│  └─ bridge-update/           # TUF client, staged install
├─ apps/
│  ├─ host-service/
│  ├─ host-agent/
│  ├─ viewer/                  # core + UI shell
│  ├─ ui/                      # Flutter or Tauri frontend (see ADR-004)
│  ├─ rendezvous/
│  ├─ relay/
│  └─ console/                 # axum API + web/ (React)
├─ fuzz/                       # cargo-fuzz targets
├─ tests/                      # integration & e2e (netns NAT simulation)
├─ packaging/                  # WiX, pkg, deb/rpm, flatpak, docker
├─ xtask/                      # build orchestration
└─ docs/                       # this Obsidian vault
```

## Rules
- `bridge-crypto` and `bridge-session` have **CODEOWNERS = Security Lead + Tech Lead**.
- `unsafe` is `#![forbid(unsafe_code)]` in all crates except `bridge-platform-*`, `bridge-capture`, `bridge-input`.
- No crate depends "upward" (apps → crates only).

## Related
[[Coding Standards]] · [[CI-CD and Release]]
