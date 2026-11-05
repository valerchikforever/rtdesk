---
tags: [moc, home]
status: living
updated: 2026-11-05
---

# Remote Desktop Project — Home

> [!abstract] What this is
> Engineering knowledge base for **Project "Bridge"** (working name) — a self-hostable, end-to-end encrypted remote desktop application in the spirit of RustDesk, designed **security-first**. This vault is the single source of truth for scope, architecture, security, plans and decisions.

> [!success] Status (2026-11-05)
> MVP implemented and verified end to end on Windows — see [[Implementation Status]]. The product is free and open source ([[ADR-006 Free and Open Source Licensing]]).

## Start here
1. [[Vision and Scope]] — what we build and, equally important, what we don't
2. [[Requirements]] — functional and non-functional requirements
3. [[System Architecture]] — the big picture
4. [[Threat Model]] — what can go wrong and how we defend
5. [[Roadmap]] — phases, milestones, exit criteria

## Map of Content

### 01 · Product
- [[Vision and Scope]]
- [[Requirements]]
- [[Glossary]]

### 02 · Architecture
- [[System Architecture]]
- [[Tech Stack]]
- Components: [[Host Agent]] · [[Viewer Client]] · [[Rendezvous Server]] · [[Relay Server]] · [[Management Console]]
- [[Wire Protocol]]
- [[NAT Traversal]]
- [[Media Pipeline]]

### 03 · Security
- [[Security Principles]]
- [[Threat Model]]
- [[Cryptography Design]]
- [[Authentication and Authorization]]
- [[Platform Hardening]]
- [[Abuse and Scam Prevention]]
- [[Lessons from Prior Incidents]]
- [[Secure SDLC]]

### 04 · Planning
- [[Roadmap]]
- [[Risk Register]]
- [[Team and Roles]]

### 05 · Engineering
- [[Repository Layout]]
- [[Coding Standards]]
- [[Testing Strategy]]
- [[CI-CD and Release]]

### 06 · Operations
- [[Deployment]]
- [[Observability]]
- [[Incident Response]]

### 07 · Decisions (ADR)
- [[ADR-000 Template]]
- [[ADR-001 Rust for Core and Servers]]
- [[ADR-002 QUIC End-to-End Transport]]
- [[ADR-003 PAKE-Based Session Authentication]]
- [[ADR-004 UI Framework]]
- [[ADR-005 Relay Is Blind]]
- [[ADR-006 Free and Open Source Licensing]]
- [[ADR-007 MVP Screen Codec]]
- [[ADR-008 Gamepad and Relative Mouse]]

### 08 · Implementation
- [[Implementation Status]] — what is built, test evidence, deviations, next steps
- Build & run instructions: `README.md` in the repository root

## Conventions for this vault
- Language: **English**. Notes are short and linked, not long monolithic docs.
- Every note has frontmatter: `tags`, `status` (`draft` | `review` | `accepted` | `living` | `superseded`), `updated`.
- Decisions that are hard to reverse go into an **ADR** (`07 - ADR`). Notes link to the ADR rather than re-arguing it.
- Security-relevant statements carry an ID (e.g. `T-12` threat, `M-07` mitigation, `SR-04` security requirement) so they can be traced to code, tests and tickets.
- Callouts: `> [!warning]` for security pitfalls, `> [!todo]` for open questions, `> [!decision]` for summarized decisions.
