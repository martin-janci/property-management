# Test-Hardening Batch: thb-2026-05-25

**Slotted:** 2026-05-25  
**Priority:** medium  
**Owner role:** pm-frontend  
**Batch status:** in-progress

## Summary

Post-merge review of PRs merged 2026-05-24/25 surfaced 8 follow-up issues (#480–#487) covering test gaps and minor security/UX regressions across four areas:

| Area | Issues | Specialist |
|------|--------|------------|
| Messaging / WebSocket realtime | #480, #484, #486 | rust-backend, react-web |
| Document share-flow | #485 | react-web |
| OAuth UI & security | #481, #482 | rust-backend, react-web |
| MFA test coverage | #487 | rust-backend |
| Security IDOR fix | #483 | rust-backend |

## Gate rule

**Do not promote a story to `done` while any of its gated issues are still open.**  
Close or explicitly defer each issue first, then update `sprint-status.yaml`.

## Issue details

### #480 — WebSocket JWT in access logs + no session re-validation (severity: high)
- **Source PR:** #472 (Epic 8A Story 3 — WebSocket realtime sync)
- **Gated stories:** `8a-3-notification-preference-sync`
- **Findings:**
  - JWT access token passed as `?token=` query param → written into access/proxy logs
  - WebSocket session kept alive for up to 4 h after JWT expiry (no re-validation)
  - Idle-ping loop accumulates unanswered pings; never forcibly closes connection
- **Proposed fix:** ws-ticket endpoint (`POST /api/v1/auth/ws-ticket`) returning a short-lived opaque token; extract `exp` from JWT and break session loop on expiry; add `pings_unanswered` counter

### #481 — OAuth revoked_at IS NULL removed from refresh-token lookup (severity: high)
- **Source PR:** #470 (OAuth integration tests)
- **Gated stories:** `10a-1-oauth-authorization-server`, `10a-3-oauth-token-management`
- **Findings:**
  - `AND revoked_at IS NULL` stripped from production refresh-token query to make a test pass
  - Revoked tokens can now obtain new access tokens — breaks RFC 9700
  - No negative test for revoked token rejection
- **Proposed fix:** Separate `find_refresh_token_including_revoked()` method for family-reuse detection; restore `AND revoked_at IS NULL` on the production path; add `test_revoked_refresh_token_cannot_be_reused`

### #482 — ProtectedRoute role fallback uses tenants[0]; no unit tests (severity: medium)
- **Source PR:** #459 (ProtectedRoute role-gate hardening)
- **Gated stories:** `10a-2-oauth-client-registration`
- **Findings:**
  - `AuthContext.tsx` falls back to `tenants[0].role` — wrong for multi-tenant users
  - No Vitest unit tests for deny-on-missing-role logic
  - Stale role cached in localStorage until next login (not refreshed on token rotation)
- **Proposed fix:** Resolve role from JWT `tenant_id` claim or sort tenants by privilege level; add `ProtectedRoute.test.tsx` with 3 test cases

### #483 — Voice device IDOR fix: no tests; list-commands leaks existence (severity: medium)
- **Source PR:** #461 (IDOR fix on voice device endpoints)
- **Gated stories:** _(none in current sprint)_
- **Findings:**
  - IDOR fix (ownership check via `user_id`) has no integration tests
  - `list_voice_commands` returns HTTP 200 + empty list (existence oracle) vs. `deactivate_voice_device` returning 404
- **Proposed fix:** Add `test_voice_device_delete_idor_prevented` + `test_voice_command_list_idor_prevented`; unify disclosure posture to 404

### #484 — Notification pipeline serial dispatch + FCM stub swallows failures (severity: medium)
- **Source PR:** #463 (Epic 2B notification delivery pipeline)
- **Gated stories:** `8a-2-critical-notification-override`, `8a-3-notification-preference-sync`
- **Findings:**
  - `dispatch_to_users` runs 200 sequential DB round-trips for building-wide announcements
  - `FcmPushAdapter::send` returns `Ok(())` when FCM is not configured → false `sent` count
- **Proposed fix:** `buffer_unordered(20)` or `JoinSet` for concurrent dispatch; `NotificationError::PushNotConfigured` variant to distinguish skipped vs. sent

### #485 — Document share panel: window.confirm + no UUID validation (severity: medium)
- **Source PR:** #467 (Web document sharing UI panel)
- **Gated stories:** `7a-5-document-sharing`
- **Findings:**
  - `window.confirm` for revoke dialog (blocks JS loop, can't be styled, suppressed in iframes)
  - No client-side UUID validation on User ID field — generic 422 error on bad input
- **Proposed fix:** Replace with `DestructiveConfirmDialog` component; add UUID regex validation on `userId` field before enabling submit

### #486 — Announcements wiring bypasses axios interceptor via direct getToken() (severity: medium)
- **Source PR:** #466 (Wire AnnouncementsPage + FaultsPage to API hooks)
- **Gated stories:** `6-2-announcement-viewing-acknowledgment`, `6-5-direct-messaging`
- **Findings:**
  - Custom `fetchJson` helper calls `getToken()` directly, bypassing shared axios interceptors
  - Silent token refresh on 401 won't work for announcement hooks
  - Inline IIFE in `onFilterChange` creates new references on every render
- **Proposed fix:** Replace custom `fetchJson` with shared `apiClient` axios instance; wrap `onFilterChange` in `useCallback`

### #487 — MFA tests: missing rate-limit coverage; mod common double-declaration (severity: medium)
- **Source PR:** #473 (TOTP MFA e2e integration tests)
- **Gated stories:** `10a-1-oauth-authorization-server`
- **Findings:**
  - No brute-force/rate-limit test for MFA verify/login endpoints (10^6 TOTP space)
  - `mod common;` declared inside `mfa_e2e_tests.rs` may create duplicate module instances
- **Proposed fix:** Add `test_mfa_verify_rate_limited` (or `#[ignore]` stub if rate limiting not yet implemented); change to `use super::common::*;`

## Completion checklist

- [ ] #480 closed or deferred → unblock `8a-3-notification-preference-sync`
- [ ] #481 closed or deferred → unblock `10a-1-oauth-authorization-server`, `10a-3-oauth-token-management`
- [ ] #482 closed or deferred → unblock `10a-2-oauth-client-registration`
- [ ] #483 closed or deferred → _(no sprint story gate)_
- [ ] #484 closed or deferred → unblock `8a-2-critical-notification-override`, `8a-3-notification-preference-sync`
- [ ] #485 closed or deferred → unblock `7a-5-document-sharing`
- [ ] #486 closed or deferred → unblock `6-2-announcement-viewing-acknowledgment`, `6-5-direct-messaging`
- [ ] #487 closed or deferred → unblock `10a-1-oauth-authorization-server`
- [ ] Update `sprint-status.yaml` `test_hardening_batch.status` → `done` once all items resolved
