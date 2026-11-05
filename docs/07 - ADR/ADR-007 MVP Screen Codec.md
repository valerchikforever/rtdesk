---
tags: [adr, media]
status: accepted
updated: 2026-11-05
---

# ADR-007 MVP Screen Codec

- **Status:** accepted (for MVP; to be superseded by hardware codecs)
- **Date:** 2026-11-05

## Context
The MVP needs a screen codec that builds with no extra toolchains, is free of patent licensing questions (the product is free, [[ADR-006 Free and Open Source Licensing]]), and is safe to decode on the viewer.

## Options considered
1. **OpenH264 compiled from source** — good compression; patent coverage only applies to Cisco's binaries, not self-built ones.
2. **libvpx (VP8/VP9)** — royalty-free; painful to build on Windows (yasm/MSYS).
3. **Hardware H.264/HEVC/AV1** — best result; large per-vendor integration work (planned in [[Media Pipeline]]).
4. **Dirty-tile JPEG in pure Rust** (`jpeg-encoder`, `zune-jpeg`) — simple, patent-free, memory-safe decoder; higher bandwidth for video-like content.

## Decision
Option 4 for the MVP: 64×64 tile diff, one JPEG band per tile row containing changes, quality 65, ≤ 30 fps, natural backpressure through a reliable QUIC stream.

## Consequences
- + Zero cost when the screen is static; fine for office work and support.
- + Decoder is pure Rust with strict bounds checks.
- − Full-screen video/scrolling costs several Mbit/s; no temporal prediction.
- Follow-up: AV1 (royalty-free) and H.264 via OS hardware encoders, negotiated by capability.
