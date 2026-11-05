---
tags: [engineering, standards]
status: draft
updated: 2026-11-05
---

# Coding Standards

## Rust
- Edition 2021 for now (MVP; migrate to 2024 later); `rustfmt` default; `clippy` with `-D warnings` and selected pedantic lints (`unwrap_used`, `expect_used` in non-test code, `indexing_slicing` in parsers, `arithmetic_side_effects` in parsers).
- Errors: `thiserror` in libraries, `anyhow` only in binaries; no panics on network input — a panic is a DoS bug.
- Secrets: wrap in `secrecy::SecretBox` / custom newtype with redacted `Debug`, `zeroize` on drop.
- Parsing: decode → validate → convert to domain types ("parse, don't validate"); never pass raw protobuf structs into business logic.
- Bounded everything: channels (`tokio::sync::mpsc` with capacity), buffers, retries, timeouts.
- `unsafe`: only in allowed crates, each block with `// SAFETY:` and a test.

## Comments with security tags
- `// SEC: M-xx` — implements mitigation from [[Threat Model]].
- `// SEC-TODO:` — blocks release; CI counts them.

## Commits & PRs
- Conventional commits; PR ≤ 400 lines preferred; security impact section in PR template ([[Secure SDLC]]).

## Documentation
- Public APIs documented; architecture-level changes → update vault note + ADR if irreversible.
