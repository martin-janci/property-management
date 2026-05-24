---
id: ppt/settings-two-factor
name: Two-Factor Authentication
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/settings/two-factor"
    component: TwoFactorAuthPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
endpoints:
  - auth_mfa_setup
  - auth_mfa_verify
  - auth_mfa_disable
  - auth_mfa_status
  - auth_mfa_backup_codes_regenerate
relatedScreens: []
sharedComponents: []
diagrams: []
useCases:
  - UC-14.10
epics:
  - Epic-9-1
designSources: []
owner: pm-security
---

# Two-Factor Authentication

Wired to live backend MFA endpoints in Epic 9, Story 9.1. TwoFactorAuthPage now uses
useMfaStatus, useMfaSetup, useMfaVerify, useMfaDisable, and useMfaRegenerateBackupCodes
hooks from @ppt/api-client.

## Notes

### Specific (recent)
- 2026-05-24 — agent: wired all five MFA hooks; removed local-state-only scaffold; apiStatus promoted to complete.
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:377`.

## Agent Log
- 2026-05-24 — agent: gap-9-1-mfa-frontend-integration — wired TwoFactorAuthPage to /api/v1/auth/mfa/* via useMfa* hooks; added mfa module to @ppt/api-client; removed feature-not-exposed scaffold.
- 2026-05-18 — agent: created stub for unmapped route.
