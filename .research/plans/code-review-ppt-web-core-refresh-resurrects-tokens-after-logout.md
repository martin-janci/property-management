# code-review-ppt-web-core-refresh-resurrects-tokens-after-logout

**Vector:** security
**Score:** 2
**Source:** Phase 1.5 rotating expert review 2026-09-08 (ppt-web-core segment); related PR #2942 (single-flight refresh)
**Confidence:** high

## Hypothesis
The single-flight refresh path added in PR #2942 has a session-teardown race: when the user clicks Logout while a 401-triggered refresh is already in flight, `logout()` runs `tokenStorage.clear()` first, then the pending refresh completes and re-persists a fresh access + refresh pair to storage. React state stays logged out because `storedUser` is now null, but on the next page reload `initializeAuth` finds the resurrected tokens and silently re-authenticates the previous session. The 401 interceptor also replays the original request with the rotated bearer after logout has already run. The smallest fix is to bind refresh + replay to a per-session AbortController that `logout()` aborts, and to short-circuit the response write when the session has been torn down since the refresh started.

## Evidence
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:415-416` — `refreshTokenInternal` writes `tokenStorage.setAccessToken/setRefreshToken(response.*)` before checking `tokenStorage.getUser()`. The `if (storedUser)` guard only prevents state re-mount, not storage persistence.
- `frontend/apps/ppt-web/src/lib/api.ts` single-flight interceptor introduced by PR #2942 (`code-review-ppt-web-core-api-401-no-request-replay`) queues in-flight 401s and replays them all with the rotated bearer once refresh resolves — no cancellation on logout.
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:186-195` — `logout()` clears the 4 auth keys and query cache but has no handle on any in-flight refresh promise or replayed request.
- The three ppt-web-core hardening PRs (#2941 cold-boot JWT-exp, #2942 single-flight refresh, #2943 logout purge allowlist) each closed a related seam without composing against each other; this cross-composition gap was not covered by the individual test additions.

## Files
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx`
- `frontend/apps/ppt-web/src/lib/api.ts`
- `frontend/apps/ppt-web/src/contexts/AuthContext.api-client-unauthorized.test.tsx`

## Dependencies

## Required capabilities
- [ ] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):** cloud-ok — pure ppt-web unit tests (Vitest), no browser or device needed.

Mode: cloud-ok

## Repro steps
1. In a Vitest jsdom test, mount `AuthProvider`, seed `tokenStorage` with a valid user + access + refresh, and mock `POST /api/v1/auth/refresh` with a controllable deferred (returns `{ accessToken: "NEW", refreshToken: "NEW_R", user: … }`).
2. Fire an authenticated `GET /api/v1/…` request that resolves to `401`. The single-flight interceptor kicks off refresh (deferred still pending).
3. While refresh is still pending, call the exported `logout()` from `AuthContext`. This clears `tokenStorage` and query cache.
4. Resolve the deferred refresh with the mocked response.
5. Expected: `tokenStorage.getAccessToken()` and `tokenStorage.getRefreshToken()` return `null`; the replayed 401 request does NOT go out. Actual (today): storage holds `NEW`/`NEW_R`; the replayed request is sent with the rotated bearer.

## Suggested approach
1. Add a session-scoped `AbortController` field to `AuthContext` state (e.g. `sessionAbortRef`), created on login/init and replaced on logout.
2. In `logout()`, call `sessionAbortRef.current?.abort("logout")` BEFORE `tokenStorage.clear()` and query-cache purge.
3. In `refreshTokenInternal` (AuthContext.tsx:415-416), pass the session abort signal to the refresh axios call and, on resolution, re-check `tokenStorage.getUser()` (or a monotonic session-id counter captured at request start) before calling `setAccessToken/setRefreshToken`. If the session was torn down, discard the response.
4. In `frontend/apps/ppt-web/src/lib/api.ts`, the single-flight refresh promise must propagate abort: reject queued replays with the same abort reason so they never fire post-logout. Bind each queued request's replay to the same session id captured when it was enqueued.
5. Add `AuthContext.logout-refresh-race.test.tsx` covering the Repro steps sequence exactly (storage cleared, no replay bytes on the wire, `initializeAuth` on next mount finds no tokens).
6. Extend `AuthContext.api-client-unauthorized.test.tsx` with the "logout during refresh" case so the seam remains covered.
7. `pnpm -F @ppt/ppt-web test --run AuthContext` after the change; also run `pnpm -F @ppt/ppt-web check` for Biome.

## Alternatives considered
- **Persist a session-version number in storage and reject stale refresh responses on write** — rejected because it leaves a window where the network request has already gone out with a valid refresh token that a hostile page could observe if extension/network logging was in play; abort-on-logout stops the request itself, not just the write.
- **Track logout via a boolean flag read at write time** — rejected because concurrent logout + refresh races are exactly the case a boolean loses; a monotonic session id (or AbortController identity) is race-safe by construction.

## Root-cause trace
1. Symptom: after logout, next page reload silently re-authenticates the previous user.
2. ← `initializeAuth` at `AuthContext.tsx` reads a fresh access/refresh pair from `tokenStorage` that shouldn't be there.
3. ← `refreshTokenInternal` at `AuthContext.tsx:415-416` wrote `response.accessToken`/`response.refreshToken` to storage without checking whether the session was still alive.
4. ← Single-flight interceptor in `lib/api.ts` (added by PR #2942) held the refresh promise open across the `logout()` transition; PR #2942 removed the pre-existing per-request refresh guard that used to make this race negligibly narrow.
5. Origin: PR #2942 `code-review-ppt-web-core-api-401-no-request-replay: single-flight refresh + replay on 401` (merged 2026-09-06T10:34:59Z). The design change is correct on the 401 side; it just missed composing with `logout()`.

## Test plan
- [ ] `frontend/apps/ppt-web/src/contexts/AuthContext.logout-refresh-race.test.tsx` — new test asserting the Repro steps sequence: after logout mid-refresh, `tokenStorage.getAccessToken() === null` and no replayed request fires.
- [ ] Extend `frontend/apps/ppt-web/src/contexts/AuthContext.api-client-unauthorized.test.tsx` to cover the logout-during-refresh path so future single-flight refactors don't reintroduce the race.
- [ ] Regression: verify existing PR #2942 replay tests still pass — the abort is meant to affect ONLY the logout path.
- [ ] Command: `pnpm -F @ppt/ppt-web test --run AuthContext.logout-refresh-race` (test fails on current dev; passes after fix). Then `pnpm -F @ppt/ppt-web test --run AuthContext` and `pnpm -F @ppt/ppt-web check`.

## Out of scope
- Broader refactor of `AuthContext` state shape.
- Storage-key allowlist for logout purge (tracked separately as `code-review-ppt-web-core-logout-leaks-form-drafts-across-users`).
- Backend refresh-token rotation semantics (server side is already correct).

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-refresh-resurrects-tokens-after-logout.md`
- Mark the matching `backlog.json` row (`code-review-ppt-web-core-refresh-resurrects-tokens-after-logout`) as `status: "done"`
