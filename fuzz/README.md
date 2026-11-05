# Fuzzing

Targets cover every parser that handles untrusted input:

| Target | Attacker | Code |
|---|---|---|
| `relay_parse` | anyone on the internet (UDP to relay) | `bridge-proto::relay` |
| `session_admit` | authenticated or unauthenticated viewer | `bridge-session` state machine |
| `frame_decode` | malicious host (attacking the viewer) | `bridge-codec::FrameDecoder` |
| `rendezvous_decode` | anyone / malicious server | rendezvous protobuf messages |

Run (Linux or macOS, nightly toolchain):
```bash
cargo install cargo-fuzz
```
```bash
cargo +nightly fuzz run relay_parse -- -max_total_time=300
```
CI runs each target for 60 s on every push (`.github/workflows/ci.yml`).
Crashes found go to `fuzz/artifacts/`; add the reproducer as a regression unit test before fixing.
