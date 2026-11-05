---
tags: [adr, ui]
status: proposed
updated: 2026-11-05
---

# ADR-004 UI Framework

- **Status:** proposed — decision at end of Phase 0 spike
- **Date:** 2026-11-05

## Context
Need desktop (Win/mac/Linux) + mobile (Android/iOS) viewer UIs, a small host tray/consent UI, and high-performance video rendering. The UI must not be on the privileged path.

## Options considered
| Option | Pros | Cons |
|---|---|---|
| **Flutter** + Rust core via FFI (`flutter_rust_bridge`) | One codebase for desktop + mobile; proven by RustDesk | Video texture integration per platform; larger binaries |
| **Tauri 2** (webview) + Rust | Small binaries; web skills; mobile support | Webview video rendering latency (needs native surface overlay); webview attack surface |
| **Native per platform** (WinUI/SwiftUI/GTK/Kotlin) | Best integration and performance | 4–5 codebases |
| **Slint / egui** (Rust-native) | Pure Rust, small | Less mature mobile/a11y |

## Preliminary recommendation
Flutter for viewer (desktop + mobile), with video rendered into a native GPU texture; host consent/indicator UI also Flutter (or minimal native) — final call after spike measuring latency, binary size and accessibility.

## Consequences
To be filled after spike.
