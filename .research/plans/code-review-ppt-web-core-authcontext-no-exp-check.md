# code-review-ppt-web-core-authcontext-no-exp-check

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review (Phase 1.5, 2026-06-24)
**Confidence:** medium

## Hypothesis

`AuthContext.tsx` `initializeAuth()` reads any stored access token from `localStorage` and treats it as authenticated state without verifying the JWT `exp` claim. When a user returns after a stale session, the expired token is loaded as "logged in", the first API call 401s, the global axios interceptor silently triggers logout, and the user sees a confusing "appears logged in → bounced to login" transition. The fix is a one-liner: in the refresh-on-init branch, take the refresh path when the access token is missing **OR** when `decodeJwtPayload(accessToken).exp` is past now. `decodeJwtPayload()` already exists at L219 of the same file.

## Evidence

- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:396-414` — `initializeAuth()` only checks `accessToken && refreshToken` truthy; never decodes `exp`. Comment at L397-398 acknowledges "we might verify" but no check is performed.
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:219` — `decodeJwtPayload(token)` already implemented; returns `{exp: number, ...} | null`.
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:244` — `decodeJwtPayload(accessToken)` already used in `setUserFromTokens()` for claim reads; the helper is proven safe.
- Phase 1.5 frontend expert finding, 2026-06-24 ppt-web-core slice (segment last reviewed 2026-06-05; 19d stale).
- The refresh-on-init branch at L400 should be taken whenever `exp <= Date.now()/1000`, not only when `accessToken` is absent.

## Files

- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:396`
- `frontend/apps/ppt-web/src/contexts/AuthContext.test.tsx`

## Dependencies

(none)

## Required capabilities

- [x] C1 — Systematic debugging (bug class)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode:** the change is a JS branching condition with a Vitest regression test; no browser or device needed.

Mode: cloud-ok

## Repro steps

1. In a working ppt-web session, capture the current `access_token` value from `localStorage`. Sign out (clears tokens); paste the captured access token back into `localStorage["access_token"]` along with a still-valid `refresh_token`.
2. Wait until the `exp` of the access token has passed (or decode it locally: `exp` is in `decodeJwtPayload(token).exp`, treat as seconds since epoch). Reload the app.
3. Expected after fix: the auth-init flow calls `/api/v1/auth/refresh` (because the access token is expired) and lands on the dashboard with a fresh access token.
4. Actual today: app boots into the authenticated UI, the first protected query (e.g. `GET /api/v1/me`) returns 401, the axios `401` interceptor calls `logout()`, and the user is bounced to the login screen with no explanation.

## Suggested approach

1. In `frontend/apps/ppt-web/src/contexts/AuthContext.tsx`, define a small helper near `decodeJwtPayload` (around L230):
   ```ts
   function isAccessTokenExpired(token: string | null | undefined): boolean {
     if (!token) return true;
     const claims = decodeJwtPayload(token);
     const exp = claims?.exp;
     if (typeof exp !== "number") return true;
     // 30 s skew to avoid borderline-just-expired tokens slipping through
     return exp * 1000 <= Date.now() + 30_000;
   }
   ```
2. In `initializeAuth()` at L396-414, change the refresh-on-init condition. Today it's roughly `if (!accessToken && refreshToken)`; replace with `if (refreshToken && isAccessTokenExpired(accessToken))`.
3. Keep the existing branch where neither token is present → unauthenticated landing.
4. Author `AuthContext.test.tsx` that:
   - mocks `localStorage` with `{access_token: <token with exp in the past>, refresh_token: "valid-refresh"}`
   - mocks the refresh endpoint to return a fresh access token
   - asserts the refresh endpoint was called exactly once during `<AuthProvider>` mount
   - asserts the user lands in the authenticated state (not bounced to login)
5. Add a second test for the truly-missing-access case: `{access_token: null, refresh_token: "valid-refresh"}` still hits refresh (preserve existing behavior).
6. Run `pnpm -F @ppt/ppt-web test AuthContext`; expect both tests green.

## Alternatives considered

- **Catch the 401 in the axios interceptor and silently call refresh, retry the original request.** — rejected because it doesn't address the root cause (the "logged-in flash → bounce" UX still flashes the authenticated dashboard for ~200ms), it adds retry complexity at the interceptor layer, and it competes with the existing token-refresh path already in `setupAxiosTokenInterceptor`. Fix the assumption at the source instead.
- **Drop the access token from localStorage entirely; always refresh on init.** — rejected because the access token's expected lifetime is long enough that a cold start within its TTL is the dominant happy path; forcing a refresh round-trip on every session resume regresses startup latency for the vastly more common case.

## Root-cause trace

1. Symptom: User signs in, comes back hours later, sees the dashboard render briefly, then gets bounced to the login screen with no error toast.
2. ← `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:396-414` — `initializeAuth()` accepts a stored access token unconditionally and emits `setUserFromTokens(...)` → app renders authenticated UI.
3. ← `frontend/apps/ppt-web/src/api/axios.ts` (interceptor) — first protected query 401s; the 401 handler invokes `authContext.logout()` and redirects.
4. Origin: PR #1638 (refactor(api-core): dedup principal extractor JWT path) is unrelated; the AuthContext init path predates this work. The "we might verify" comment at L397-398 indicates the original author left the exp check as a TODO that never landed.

## Test plan

- [x] `frontend/apps/ppt-web/src/contexts/AuthContext.test.tsx` — new Vitest spec: `initializeAuth refreshes when stored access token is expired`. Asserts that with `{access_token: expired, refresh_token: valid}` in localStorage, `<AuthProvider>` calls the refresh endpoint and the user lands authenticated. Today this test fails (no refresh call); after the fix it passes (IG3 satisfied).
- [x] Regression: `initializeAuth still refreshes when only refresh_token is present` (preserves existing behavior).
- [x] Exact command: `pnpm -F @ppt/ppt-web test AuthContext`

## Out of scope

- Replacing the localStorage-backed token store with httpOnly cookies (separate epic).
- Rewriting the axios 401 interceptor (`api/axios.ts`) — leave the silent-logout fallback intact for genuinely-expired-refresh-too cases.
- Changing the refresh token's exp policy or sliding window (backend concern).
- Other AuthContext correctness issues (e.g. tenant switching) — narrow scope to the JWT-exp gap.

## After-merge

- Move this file to `plans/_archive/code-review-ppt-web-core-authcontext-no-exp-check.md`
- Mark the matching `backlog.json` row as `status: "done"`
