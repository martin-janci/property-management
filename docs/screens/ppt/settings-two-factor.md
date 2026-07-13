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
  - Epic-9
designSources: []
owner: pm-security
---

# Two-Factor Authentication

Wired to live backend MFA endpoints in Epic 9, Story 9.1. TwoFactorAuthPage now uses
useMfaStatus, useMfaSetup, useMfaVerify, useMfaDisable, and useMfaRegenerateBackupCodes
hooks from @ppt/api-client.

## Notes

### Specific (recent)
- 2026-06-03 — agent: recovery-codes UI (Story 9.2) — codes now shown once after enable (from the verify response, which carries `recoveryCodes`), reusable RecoveryCodesPanel with "Copy all"; exhausted (0) + low (≤3) warning banners; setup response no longer returns codes (api-client `MfaSetupResponse.backupCodes` dropped, `VerifyMfaResponse.recoveryCodes` added).
- 2026-05-24 — agent: wired all five MFA hooks; removed local-state-only scaffold; apiStatus promoted to complete.
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:377`.

## Agent Log
- 2026-07-13 — agent: gap-screens-normalize-frontmatter — normalized story-id-style epic ref(s) Epic-9-1,Epic-9-2 → Epic-9 (strip story suffix); /screens validate clean.
- 2026-06-03 — agent: gap-9-2-mfa-recovery-codes-ui — recovery-codes management UI (show 10 codes, copy-all, regenerate, exhausted/low warnings); realigned @ppt/api-client MFA types to the live backend contract (setup → {secret,qrUri}; verify → +recoveryCodes).
- 2026-05-24 — agent: gap-9-1-mfa-frontend-integration — wired TwoFactorAuthPage to /api/v1/auth/mfa/* via useMfa* hooks; added mfa module to @ppt/api-client; removed feature-not-exposed scaffold.
- 2026-05-18 — agent: created stub for unmapped route.
