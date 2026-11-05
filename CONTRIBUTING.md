# Contributing to Bridge

Thanks for helping! Bridge is free software (AGPL-3.0-or-later) and will stay free — there are no paid editions.

## Before you start
- Read `docs/00 - Home.md` (open `docs/` as an Obsidian vault, or read the Markdown directly), especially
  the [Security Principles](docs/03%20-%20Security/Security%20Principles.md) and [Threat Model](docs/03%20-%20Security/Threat%20Model.md).
- For larger changes open an issue first. Irreversible design decisions need an ADR in `docs/07 - ADR/`.
- Security issues: see [SECURITY.md](SECURITY.md), never public issues.

## Development
```bash
cargo build
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
cargo deny check
```
Fuzzing (Linux/macOS, nightly): see [`fuzz/README.md`](fuzz/README.md).

## Rules
- `unsafe` only in `crates/bridge-platform` and `crates/bridge-crypto/src/keystore.rs`, each block with a `// SAFETY:` comment.
- Network input is untrusted: bound every size, never panic on it, add a negative test and a fuzz corpus entry.
- Mark code that implements a mitigation from the threat model with `// SEC: M-xx`.
- Never log secrets (codes, keys, clipboard content, file names).
- New dependencies must be compatible with AGPL-3.0 (`deny.toml`) and justified in the PR.

## Developer Certificate of Origin
We use the [DCO](https://developercertificate.org/) instead of a CLA, so the project can never be relicensed
to a proprietary edition. Sign off every commit:
```bash
git commit -s
```
