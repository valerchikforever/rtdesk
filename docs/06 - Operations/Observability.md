---
tags: [operations, observability]
status: draft
updated: 2026-11-05
---

# Observability

## Server metrics (Prometheus / OpenTelemetry)
- Rendezvous: online devices, registrations/s, lookups/s, hole-punch success rate, relay allocation rate, auth failures by reason.
- Relay: active sessions, bytes/s, per-region utilization, dropped packets, token rejections.
- Console: login success/failure, 2FA failures, policy changes, grant issuance rate.

## Security monitoring / detections
| Signal | Possible meaning |
|---|---|
| Spike in failed PAKE attempts reported by hosts for one viewer key | Brute force / scanning |
| Viewer account connecting to many new hosts in short time | Scam operation (see [[Abuse and Scam Prevention]]) |
| Grant issuance for many devices by one admin outside working hours | Compromised admin / insider |
| Host key change for a long-lived Device ID | Reinstall — or hijack attempt |
| Login from new country + immediate unattended connect | Account takeover |

## Client telemetry
Opt-in only, anonymous: crash reports (scrubbed), connection path stats, codec/hardware capability, latency histograms. No screen content, no file names, no keystrokes — ever.

## Logging rules
- Structured logs (`tracing` JSON); redaction of secrets enforced by types and a log-scrubbing test.
- IPs pseudonymized after 7 days.

## Related
[[Incident Response]] · [[Deployment]]
