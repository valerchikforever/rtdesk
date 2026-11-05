---
tags: [security, authn, authz]
status: draft
updated: 2026-11-05
---

# Authentication and Authorization

## Access modes
| Mode | Who decides | Factors | Default |
|---|---|---|---|
| **Attended** | Local user clicks Accept and chooses permissions | OTC (PAKE) + human consent | ON |
| **Unattended – personal** | Host policy | Device password (OPAQUE) **+** optional trusted-viewer-key requirement **+** optional TOTP on host | OFF |
| **Unattended – org** | Org policy + host local policy | Viewer account (SSO/2FA) → signed grant; host verifies offline | OFF until enrollment |

## Viewer accounts (console)
- Login: OIDC SSO or email+password (Argon2id, breached-password check) **+ mandatory 2FA** for anyone with unattended access rights. WebAuthn preferred; TOTP allowed; SMS not supported.
- New viewer device must be approved (email link / existing device / admin) — mitigates credential stuffing as seen in TeamViewer 2016 ([[Lessons from Prior Incidents]]).
- Tokens: short-lived access token (≤ 10 min), refresh token bound to the viewer's device key (proof-of-possession), revocable.
- Step-up authentication (fresh WebAuthn) for: unattended connect to "sensitive" groups, policy changes, adding admins, exporting audit.

## Permission model
Permission set per session: `view`, `control`, `clipboard_text`, `clipboard_files`, `file_transfer`, `terminal`, `restart`, `privacy_screen`, `record`.

Effective permissions = **intersection** of:
1. What the viewer requested,
2. What the org policy grants (org mode),
3. What the host local policy allows (machine-wide, admin-set),
4. What the local user chose in the consent dialog (attended).

The host user can **downgrade or terminate** at any time from the session indicator.

## Policy examples
```yaml
# host local policy (machine-wide, admin-only)
unattended_access: false
require_consent: always          # always | when_user_present | never
allowed_permissions: [view, control, clipboard_text, file_transfer]
max_session_minutes: 240
allowed_org_keys: ["ed25519:4f1c…"]
block_during_hours: null
```
```yaml
# org policy (console)
- name: Helpdesk L1
  subjects: [group:helpdesk-l1]
  devices: [group:office-workstations]
  permissions: [view, control, clipboard_text]
  require_host_consent: true
  require_mfa_age_minutes: 60
- name: Server admins
  subjects: [group:infra-admins]
  devices: [group:servers]
  permissions: [view, control, file_transfer, terminal, restart]
  require_host_consent: false
  two_person_approval: true
```

## Session-time controls
- Idle timeout (default 30 min), max duration, lock host screen on disconnect (option, default ON for unattended), block local input (only with explicit permission; local user can always regain control via Ctrl+Alt+Del on Windows / defined hotkey).

## Related
[[Cryptography Design]] · [[Management Console]] · [[Abuse and Scam Prevention]]
