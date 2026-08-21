# code-review-ppt-web-core-no-401-refresh-wiring

**Vector:** bug
**Score:** 3
**Source:** hotspot in frontend/apps/ppt-web/src/contexts/AuthContext.tsx
**Confidence:** medium

## Hypothesis
`ppt-web` implements a token-refresh function but never calls it. `configureApiClient` is invoked with only `getToken`, so the axios interceptor's 401 branch is dead code, and `initializeAuth` restores a stored session without checking the access token's `exp`. The result is that once the access token expires, every subsequent request 401s with no recovery: the UI still looks authenticated while all data fetches fail, until the user manually logs out and back in. The smallest change is to pass `onUnauthorized` into `configureApiClient` and to check `exp` during init so an already-expired token falls through to the existing refresh branch.

## Evidence
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:337` — `configureApiClient({ getToken: getAccessToken });` supplies only `getToken`; `onUnauthorized` is never passed.
- `frontend/apps/ppt-web/src/lib/api.ts:219` — `if (error.response?.status === 401 && onUnauthorizedCallback) {` is unreachable, because `onUnauthorizedCallback` is only ever assigned from `config.onUnauthorized` at `frontend/apps/ppt-web/src/lib/api.ts:196`, which no caller sets.
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:412` — the restore branch comments that it validates the token is not expired, but performs no such check and calls `setUser(storedUser)` unconditionally, so the `refreshTokenInternal()` branch at `:426` is never reached on reload.
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:82` — `refreshToken` is exposed on `AuthContextValue`, and a repo-wide grep for `refreshToken` outside tests and `AuthContext.tsx` itself returns zero call sites: no silent-refresh path exists, reactive or proactive.
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:449` — a de-duplicated `refreshToken()` implementation already exists and is ready to be wired.

## Files
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx`
- `frontend/apps/ppt-web/src/lib/api.ts`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. In a vitest environment, seed `localStorage` with `ppt_user`, a `ppt_access_token` whose JWT `exp` claim is in the past, and a valid `ppt_refresh_token`.
2. Mount `<AuthProvider>` and let `initializeAuth` run to completion.
3. Expected: the provider detects the expired access token and calls the auth API's `refreshToken`. Actual: `setUser(storedUser)` runs at `AuthContext.tsx:415`, `refreshToken` is never called, and the app renders as authenticated with a token every request will reject.

## Suggested approach
1. In the effect at `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:325-344`, extend the `configureApiClient` call to `configureApiClient({ getToken: getAccessToken, onUnauthorized: … })`, where the callback invokes the existing de-duplicated `refreshToken()` at `:449`.
2. In that callback, on a rejected refresh, clear the stored tokens and user and de-authenticate — the same teardown the existing logout path performs.
3. In `initializeAuth` at `:405-443`, use the existing `decodeJwtPayload` helper to read `exp` from the stored access token; when it is absent or already past, skip `setUser(storedUser)` and fall through to the `refreshTokenInternal()` branch at `:426`.
4. In `frontend/apps/ppt-web/src/lib/api.ts:219`, after a successful refresh, retry the original request once with the new token instead of only notifying, so the in-flight call the user triggered succeeds rather than surfacing an error.
5. Guard the retry with a per-request flag so a 401 on the retried request cannot loop.
6. Add both tests from *Test plan* and confirm each fails against the current code first.

## Alternatives considered
- **Proactive refresh on a timer keyed to `exp`** — rejected as the primary fix because it still leaves the reactive 401 path dead; a timer drifts when the tab is backgrounded or the device sleeps, so the interceptor hook is the load-bearing half and must be wired regardless.
- **Have `getAccessToken` refresh inline whenever the token is near expiry** — rejected because `getToken` is called synchronously from the axios request interceptor for every request; making it async and refresh-capable would serialise all requests behind a shared refresh and duplicates the de-duplication logic that `refreshToken()` at `:449` already implements.

## Root-cause trace
1. Symptom: after the access token expires, every API call 401s and the UI stays in a broken authenticated state until a manual re-login.
2. ← Immediate cause at `frontend/apps/ppt-web/src/lib/api.ts:219` — the interceptor's 401 branch is guarded on `onUnauthorizedCallback`, which is null.
3. ← Upstream cause at `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:337` — `configureApiClient` is called without `onUnauthorized`, so `frontend/apps/ppt-web/src/lib/api.ts:196` never assigns the callback.
4. ← Compounding cause at `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:412` — the restore path skips the expiry check its own comment describes, so a page reload cannot recover either.
5. Origin: the `refreshToken` member added to `AuthContextValue` at `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:82` was never given a call site — the capability was built and left unconnected.

## Test plan
- [ ] New `frontend/apps/ppt-web/src/contexts/AuthContext.unauthorized-refresh.test.tsx`, case A: seed `localStorage` with an expired `ppt_access_token` plus a valid refresh token, mount `<AuthProvider>`, assert the auth API's `refreshToken` was called (fails today — init short-circuits at `AuthContext.tsx:412`).
- [ ] Same file, case B: mount `<AuthProvider>`, have `getApiClient().get('/x')` respond `401` via a mocked adapter, assert the refresh endpoint was hit and the original request was retried once.
- [ ] `cd frontend && pnpm --filter @ppt/web test src/contexts/AuthContext.unauthorized-refresh.test.tsx`

## Out of scope
- Changing the token TTLs or any backend refresh-endpoint behaviour.
- Adding a proactive refresh timer — a possible follow-up once the reactive path works.
- The unrelated `/dashboard/manager` route-guard gap, tracked separately as `code-review-ppt-web-core-dashboard-route-unguarded`.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-no-401-refresh-wiring.md`
- Mark the matching `backlog.json` row as `status: "done"`
