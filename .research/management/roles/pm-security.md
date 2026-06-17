# pm-security — analysis

Generated: 2026-06-17T00:00:00Z

## Summary

Three pre-existing high-severity open issues (#481 revoked refresh tokens reusable, #480 JWT in WS query-param logs, #487 MFA rate-limit untested) carry over from 2026-05-27 with no evidence of closure; new #1538 CI gate gap means red backend test runs can merge to dev, undermining RLS and OAuth regression tests from BIT-74/76/78/85/98/110. Booking.com credential encryption at-rest confirmed shipped (BIT-98) and OAuth refresh-token revocation regression test exists, but neither is enforced by required CI without #1538 resolved.

## Next actions

- **[high]** Make backend `test` job a required status check on dev (issue #1538) — gates RLS/OAuth/credential-encryption regression tests — dep: `none`
- **[high]** Fix issue #481: restore revoked_at IS NULL filter in OAuth refresh-token production lookup query (RFC 9700) — dep: `pm-backend`
- **[high]** Fix issue #480: stop logging WS auth token from query param; move to header/cookie or redact at access log — dep: `pm-backend`
- **[high]** Resolve prior-run outstanding #614/#624 (cross-tenant schedule mutation + missing RBAC on update_schedule) and #617 (cookie Path regression from PR #565) — dep: `pm-backend`
- **[medium]** Add MFA rate-limit regression tests (issue #487) — gates story 10a-1-oauth-authorization-server — dep: `pm-backend`
- **[medium]** Audit announce/faults direct getToken() bypass (#486) — dual-path auth skips axios refresh interceptor — dep: `pm-frontend`

## Risks

- **[high/high]** #1538 CI gate gap: backend test not required on dev — RLS/authz regression harness (36+ tests) is advisory only
  - Mitigation: Add Backend / test as required check on dev branch protection
- **[high/high]** #481 OAuth refresh-token revocation bypass — revoked tokens exchangeable for fresh access tokens
  - Mitigation: Verify OAuthRepository::find_refresh_token_by_hash enforces revoked_at IS NULL; gate with regression test under required CI
- **[medium/high]** #480 JWT in WS access logs — credential exfiltration via SIEM/log aggregators
  - Mitigation: Move WS auth to header or short-lived ticket; redact query params at tracing layer
- **[medium/high]** Legacy Booking.com plaintext credential rows — decrypt_if_available passes plaintext through; no forced migration scheduled
  - Mitigation: One-time migration to re-encrypt rental_platform_connections rows lacking enc: prefix; periodic audit query
- **[medium/high]** #614/#624 cross-tenant schedule mutation — update_schedule handler lacks tenant/org scope + RBAC; unfixed 3 weeks
  - Mitigation: Thread tenant_id/org_id + RequireCapability into PUT /api/v1/reports/schedules/{id}; add cross-tenant regression test

## Decisions needed

- Classify #1538 as immediate release blocker; mandate fix before Epic 10A or Epic 7A stories promote to done — pm-lead
- Classify #481 + #480 as P0 pre-production blockers for OAuth provider (Epic 10A) — pm-security/pm-backend
- Decide whether legacy plaintext Booking.com creds require forced re-encryption migration or decrypt_if_available passthrough is acceptable — pm-backend/pm-lead

## Open questions

- Has #481 (revoked refresh token bypass) been fixed in find_refresh_token_by_hash since 2026-05-27?
- Were #614/#624 (cross-tenant schedule mutation, missing RBAC) and #617 closed in the 3-week gap?
- Does security-test-gate.yml block security-labelled PRs lacking a test file, or only advisory?
- Is a migration scheduled to force-encrypt legacy plaintext Booking.com credential rows?
- RLS enforcement status of BIT-76 (#1460) + BIT-78 (#1467) force-RLS landings — all handlers on RLS pool?
