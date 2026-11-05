# Security Policy

Bridge is remote-access software: a vulnerability can give an attacker control of someone's computer.
We take reports seriously and fix them first.

## Reporting a vulnerability
**Do not open a public issue.** Use GitHub's private reporting:
<https://github.com/valerchikforever/rtdesk/security/advisories/new>

Please include affected version/commit, a description, reproduction steps or a proof of concept, and the impact you see.

## What to expect
| Step | Target |
|---|---|
| Acknowledgement | 3 days |
| Initial assessment | 7 days |
| Fix for Critical | 72 hours after confirmation |
| Fix for High | 14 days |
| Fix for Medium | 60 days |

We publish a GitHub Security Advisory (with CVE) once a fix is released and credit reporters who want it.
Bridge is free and has no budget for bounties yet; we maintain a hall of fame in the advisories.

## Scope
In scope: everything in this repository — host, viewer, rendezvous, relay, protocol, cryptography, build and release pipeline.

Especially interesting:
- Authentication bypass, session takeover, accepting input before consent or beyond granted permissions
- Breaking end-to-end encryption, MITM by a malicious rendezvous/relay server
- Memory-safety or DoS bugs reachable from the network
- Local privilege escalation via Bridge components
- Ways to make a session invisible to the local user

Out of scope: social engineering of users, attacks requiring an already compromised OS account,
volumetric DDoS against a specific deployment.

## Supported versions
Only the latest release receives security fixes while the project is pre-1.0.

## Design documentation
The threat model and cryptography design are public: see `docs/03 - Security/`.
