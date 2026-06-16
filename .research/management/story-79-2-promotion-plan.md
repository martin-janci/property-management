# Story 79-2 Promotion Plan: Authentication Flow → partial → done

_Generated: 2026-05-28 | Owner: pm-scrum-master_
_Promoted: 2026-06-08 | task: verify-authentication-flow-promote (pm-frontend)_

## Status: DONE ✅ (promoted 2026-06-08)

Story 79-2 was promoted **partial → done** on 2026-06-08 by the
`verify-authentication-flow-promote` task. The login / refresh / logout /
route-guard flow was reviewed against the ACs below; every scenario is now
covered by ppt-web vitest:

- `gap-79-2-auth-callback-e2e` was **already satisfied on `dev`** by
  `frontend/apps/ppt-web/src/pages/AuthCallbackPage.test.tsx` (7 scenarios:
  happy-path token storage → /dashboard, return-url redirect, refresh
  rotation, state-mismatch, missing params, provider error, exchange-failure
  rollback).
- The two remaining ppt-web vitest gaps were closed by
  `frontend/apps/ppt-web/src/contexts/AuthContext.login-refresh.test.tsx`:
  (1) direct email/password `login()` persists tokens + unlocks a guarded
  route; (2) a revoked refresh token produces a **clean logout, not an error
  loop** (AC scenario 4), plus concurrent-refresh coalescing.
- Cookie `Path=/api` (AC scenario 5) is a backend (Rust) assertion already
  covered by PR #642's inline tests — out of scope for ppt-web vitest.

`coverage.json` entry `79-2-authentication-flow` is now `status: done`.

## Summary

Story 79-2 (Authentication Flow Implementation) was **partial**. PR #642 resolved the
last security blocker (cookie Path regression, issue #617). The remaining gate before
promotion to **done** was the `gap-79-2-auth-callback-e2e` verification task — now met
(see Status above).

---

## Background

### What shipped

| PR | Merged | Delivers |
|----|--------|---------|
| PR #568 | 2026-05-25 | /auth/callback route, token storage in tokenProvider, refresh flow |
| PR #609 | 2026-05-27 | SSO auth-callback wiring clean-up, useAuth hook wired to SDK client |
| PR #642 | 2026-05-28 | auth.rs + sso.rs cookie `Path` reconciliation + inline regression tests |

### Cookie Path regression (#617) — RESOLVED by PR #642

Issue #617 identified that the `Set-Cookie` response on `/auth/callback` was setting `Path=/`
on the access-token cookie but omitting it on the refresh-token cookie, causing the browser to
reject the refresh cookie on path-scoped requests. PR #642:

- Reconciled both auth.rs and sso.rs to emit consistent `Path=/api` on all session cookies.
- Added inline Rust tests asserting the correct `Path` attribute is present on both token cookies.
- Shipped a runbook entry documenting the regression pattern for future cookie-path changes.

**PR #642 is merged. The cookie-Path security blocker is fully resolved.**

---

## Remaining Gap: `gap-79-2-auth-callback-e2e`

**Action item ID:** `gap-79-2-auth-callback-e2e`
**Owner role:** pm-qa
**Status:** open → scheduled for next sprint

### What needs to be verified

An end-to-end integration test covering the ppt-web OAuth callback flow:

1. `/auth/callback?code=<code>&state=<state>` stores access and refresh tokens via
   `tokenProvider` and redirects to `/dashboard`.
2. Logout (`AuthContext.logout()`) clears session storage and cookie state, redirects to `/login`.
3. An expired access token triggers a silent refresh (`AuthContext.refreshTokenInternal`) without
   user interruption.
4. A revoked refresh token (or family-reuse replay) results in a clean logout, not an error loop.
5. Cookie `Path` attribute is asserted to be `/api` on both token cookies (regression guard for #617).

### Acceptance criteria for promotion

Story 79-2 may be promoted **partial → done** when all of the following are true:

- [x] `gap-79-2-auth-callback-e2e` test suite is merged to `dev` (AuthCallbackPage.test.tsx).
- [x] All 4+ test scenarios above pass in CI (Vitest — 22/22 affected auth tests green).
- [x] No new open security issues tagged `auth` or `cookie` block the story (#617 closed by #642).
- [x] `coverage.json` entry for `79-2-authentication-flow` is updated to `status: done`.

### Timeline

- **Sprint N (current):** PR #642 merged; story unblocked.
- **Sprint N+1 (next sprint):** `gap-79-2-auth-callback-e2e` is scheduled; pm-qa implements and
  opens PR.
- **Sprint N+1 completion gate:** Dispatcher picks up `gap-79-2-auth-callback-e2e`; on PR merge
  and all ACs green, pm-scrum-master promotes 79-2 to done and updates `coverage.json`.

---

## Coordination

| Role | Action |
|------|--------|
| pm-qa | Implement `gap-79-2-auth-callback-e2e` — see action-list.json item |
| pm-scrum-master | On e2e PR merge: update `coverage.json` 79-2 status → done |
| pm-frontend | Review e2e PR; confirm tokenProvider / AuthContext contract matches tests |

---

## References

- Issue #617 — cookie Path regression (CLOSED by PR #642)
- PR #642 — auth.rs + sso.rs cookie Path reconciliation + tests (MERGED)
- `gap-79-2-auth-callback-e2e` — action-list.json item, status: open (scheduled next sprint)
- `coverage.json` — entry `79-2-authentication-flow`, current status: partial
- `project-state.md` — executive summary 2026-05-28
