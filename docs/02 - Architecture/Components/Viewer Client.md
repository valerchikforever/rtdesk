---
tags: [architecture, component, viewer]
status: draft
updated: 2026-11-05
---

# Viewer Client

## Responsibilities
- Connect to a host (Device ID + OTC/password, or via address book in org mode).
- Decode and render video (hardware decode: D3D11VA/DXVA, VideoToolbox, VA-API, MediaCodec; WebCodecs in web viewer).
- Capture local input and forward; handle keyboard layout mapping (scancode-based by default, translated mode optional).
- Clipboard, file transfer UI, chat, session toolbar (quality, monitors, permissions requested, disconnect).
- Pin host keys; show fingerprint on first connect and **loud warning on key change**.

## Security notes
- The viewer is an attack target too: a **malicious host** can send crafted video, clipboard, file names, cursor shapes. All of these are untrusted input:
  - Video decoding happens in a **separate sandboxed process** where the OS allows it (Windows AppContainer/restricted token, macOS sandbox, Linux seccomp + namespaces).
  - File transfer: sanitize names (no `..`, absolute paths, ADS `:` on NTFS, reserved names like `CON`, trailing dots/spaces), never auto-open, mark downloaded files with Mark-of-the-Web / quarantine xattr.
  - Clipboard from host: size-limited; rich formats (HTML/RTF/images) off by default.
  - Cursor images: bounded dimensions.
- URI handler (`bridge://connect/<id>`) only pre-fills the ID; **never** accepts passwords, paths, server addresses or flags from a URI without explicit user confirmation (see TeamViewer CVE-2020-13699 in [[Lessons from Prior Incidents]]).
- Credentials (account tokens) stored in OS keystore; refresh tokens bound to device key (DPoP-style).

## Mobile
- Android/iOS viewer: touch-to-mouse modes (trackpad / direct), on-screen special keys. Store-compliant; no host on iOS.

## Related
[[Host Agent]] · [[Wire Protocol]] · [[Media Pipeline]]
