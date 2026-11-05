---
tags: [security, abuse, ux]
status: draft
updated: 2026-11-05
---

# Abuse and Scam Prevention

Remote desktop tools are the primary instrument of **tech-support and refund scams** ("your bank account is compromised, install X so I can help"). Popular free tools get blocklisted by banks, AV vendors and enterprises because of it. Anti-abuse is therefore a product feature *and* a business survival requirement.

## Threat scenarios
1. Cold call / pop-up "Microsoft support" → victim installs the client and reads out the OTC.
2. Fake bank "security department" → asks victim to log into online banking while connected.
3. Scammer enables unattended access to return later.
4. Scammer uses "privacy screen"/black screen to hide actions (e.g. money transfer).
5. Stalkerware: partner silently installs unattended access.

## Product safeguards
| ID | Safeguard |
|---|---|
| AS-01 | **First-session warning** on host: full-screen, plain language: *"Never give access to someone who contacted you unexpectedly. Banks, Microsoft, police never ask for this."* Requires deliberate acknowledgement (not a single default button) |
| AS-02 | Consent dialog shows **who** connects: verified organization name (org mode, with verification badge) vs. "Unverified individual", viewer country (from IP), account age |
| AS-03 | Privacy screen / blank screen **only** for org-enrolled devices with policy — never in attended consumer sessions |
| AS-04 | Enabling unattended access requires admin elevation + 24 h reminder notification + visible tray state "Unattended access enabled" |
| AS-05 | **Sensitive-app detection** (opt-in, local only): if a banking site/app is in foreground during an unverified attended session, show a warning overlay to the local user and pause input from the remote side |
| AS-06 | One-click **"Report & disconnect"** button in the session indicator; reports viewer account/key/IP to abuse desk |
| AS-07 | Server-side: anomaly detection on viewer accounts (many distinct new hosts per day, high-risk geographies vs. host locale, reported accounts) → throttling, mandatory verification, bans |
| AS-08 | Fair-use rate limits on the **community** server only (e.g. sessions per day to *new* devices per viewer key) — purely anti-abuse, never a paywall; self-hosted servers configure their own limits |
| AS-09 | Community rendezvous refuses known-abusive viewer keys network-wide; self-hosters can subscribe to the published blocklist |
| AS-10 | No hidden mode, no "stealth install" option, installer always shows UI; silent install only via MSI with org enrollment token |
| AS-11 | Clear uninstall path; uninstall removes unattended access immediately |

## Process safeguards
- Abuse desk mailbox + web form, SLA 24 h for reports.
- Cooperation channel with banks/AV vendors to reduce blanket blocking (publish IoCs of abusive accounts, not of the product).
- Transparency report (community server): abuse actions taken, law enforcement requests.

## Related
[[Threat Model#Human factors / abuse (beyond STRIDE)]] · [[Authentication and Authorization]]
