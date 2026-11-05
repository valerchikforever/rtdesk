---
tags: [adr]
status: proposed
updated: 2026-11-05
---

# ADR-001 Rust for Core and Servers

- **Status:** proposed
- **Date:** 2026-11-05

## Context
The host agent parses untrusted network input while having control over the machine; memory-corruption bugs in remote-access software have historically led to RCE (see [[Lessons from Prior Incidents]]). We also need native performance for capture/encode and cross-platform support.

## Options considered
1. **C++** — mature media/OS ecosystem; memory-unsafe, highest risk for parsers.
2. **Go** — memory-safe, great for servers; GC pauses and FFI (cgo) overhead problematic for media path and OS integration.
3. **Rust** — memory-safe without GC, strong async networking (tokio, quinn, rustls), good FFI; proven in this domain (RustDesk).

## Decision
Rust for core library, host agent, viewer core, rendezvous, relay and console API. `unsafe` restricted to platform/FFI crates.

## Consequences
- + Eliminates most memory-safety bug classes in network code.
- + One language for shared protocol code client & server.
- − Hiring/learning curve; some platform APIs require careful FFI (`windows-rs`, `objc2`).
- − Codec libraries remain C/C++ (FFI) → sandbox decoding ([[Media Pipeline]]).
