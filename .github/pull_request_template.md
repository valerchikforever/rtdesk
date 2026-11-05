## What and why

<!-- Short description; link the issue. -->

## Security impact
Does this change touch any of the following? (If yes, a security-focused review is required.)
- [ ] Authentication, PAKE, key handling or pinning
- [ ] Parsing of network input (protocol, video, clipboard, relay, rendezvous)
- [ ] Permissions / session state machine
- [ ] `unsafe` code or FFI
- [ ] Build, release, update or signing pipeline
- [ ] None of the above

## Checklist
- [ ] Tests added/updated (including negative tests for untrusted input)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all` pass
- [ ] Docs in `docs/` updated (ADR if the decision is hard to reverse)
- [ ] Commits are signed off (`git commit -s`)
