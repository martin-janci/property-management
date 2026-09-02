## code-review-mobile-rn-refresh-purges-offline-queue

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review (Phase 1.5 mobile-rn 2026-09-02)
**Confidence:** high

## Hypothesis
`AuthContext.refreshToken()` catches every error from the token-refresh fetch and unconditionally calls `logout()` — including transient network failures (offline, DNS, TLS). `logout()` then routes through `resetLocalData()`, which purges the AsyncStorage `ppt_offline_queue`. Combined with `initialize()` calling `refreshToken()` on cold start whenever the stored access token has expired, opening the app offline after the JWT TTL wipes every offline-queued fault / meter / vote silently. Distinguish transient network failures from real auth-server rejections (401/403) and only invoke `logout()` on the latter; leave the offline queue intact on network errors.

## Evidence
- `frontend/apps/mobile/src/contexts/AuthContext.tsx:206` — the `catch (error)` block in `refreshToken()` calls `await logout()` on any throw. The fetch throws `TypeError` for a network-unreachable failure — indistinguishable from a real 401 here.
- `frontend/apps/mobile/src/contexts/AuthContext.tsx:326` — cold-start `initialize()` calls `refreshToken()` whenever `isJwtExpired(accessToken, TOKEN_REFRESH_SKEW_SECONDS)` is true. Opening the app offline past TTL triggers this path.
- `frontend/apps/mobile/src/contexts/AuthContext.tsx:159` — `logout()` awaits `resetLocalData()` which purges `QUEUE_KEY` (`ppt_offline_queue`) alongside the auth caches (see the block comment above line 159 — the wipe is intentional for org-switch hygiene, but it fires here from a network-error path too).
- `frontend/apps/mobile/src/test/integration/auth.integration.test.tsx:206-224` — the existing test only covers a server-side 401 refresh rejection. No test covers the fetch-rejects (network-unreachable) branch, so the data-loss regression is uncaptured.

## Files
- `frontend/apps/mobile/src/contexts/AuthContext.tsx`
- `frontend/apps/mobile/src/hooks/useOfflineSupport.ts`
- `frontend/apps/mobile/src/test/integration/auth.integration.test.tsx`

## Dependencies
_None._

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok (React Native + jest-expo tests build in cloud CI; no ADB/simulator needed for the regression proof — the failing test is a jest-expo integration test against the mocked fetch)

## Repro steps
1. Log in on a device / simulator; while online, create a fault report that goes offline (or explicitly enqueue via the offline-support hook). Confirm `ppt_offline_queue` in AsyncStorage contains the queued action.
2. Wait past `TOKEN_REFRESH_SKEW_SECONDS` so the access token would be considered expired, then take the device offline (airplane mode / disable network).
3. Cold-start the app (kill + relaunch).
4. **Expected:** offline queue survives; app either recovers a valid session or drops to login while retaining `ppt_offline_queue` so the user's queued content re-plays once online.
5. **Actual:** `initialize()` → `refreshToken()` → fetch rejects (network unreachable) → `catch` → `logout()` → `resetLocalData()` → `ppt_offline_queue` cleared. Queued content is silently lost; user has no indication anything was discarded.

## Suggested approach
1. In `refreshToken()` (`frontend/apps/mobile/src/contexts/AuthContext.tsx` around line 206), classify the caught error:
   - **Network error** (fetch rejection: `TypeError`, or explicit `error.name === 'AbortError'`) → do **not** call `logout()`; re-throw so the caller can decide (initialize surfaces "offline session" state; the offline queue survives).
   - **HTTP 401/403 from the refresh endpoint** (fetch resolved but `response.ok` is false) → keep the current behaviour: `await logout(); throw`. This is a real rejection.
   - **Non-2xx non-auth (5xx, 429, network transient reachable-but-server-error)** → skip `logout()`, treat as transient; surface as a recoverable state.
2. In `initialize()` (`frontend/apps/mobile/src/contexts/AuthContext.tsx` around line 326), when `refreshToken()` throws from the network-error branch, set the state to `{ isLoading: false, isAuthenticated: true, accessToken: stored-expired-token, offlineSessionStale: true }` (or equivalent flag) — do NOT force login. The existing catch that drops to the login screen only fires when `logout()` also ran (real auth rejection).
3. Ensure `resetLocalData()` is only ever called from a real logout / org-switch code path, not from a network-error branch. If any callers currently rely on `logout()` firing on network error, gate them behind the classified branch.
4. Add a regression test in `frontend/apps/mobile/src/test/integration/auth.integration.test.tsx` alongside the existing 401 case (line 206): mock the refresh fetch to reject with `TypeError('Network request failed')`, seed `ppt_offline_queue` with a fixture action, cold-boot the AuthContext, then assert (a) `logout()` was NOT called, (b) `AsyncStorage.getItem('ppt_offline_queue')` still holds the fixture action.
5. Optional but recommended: expose an `offline_session_stale` flag in the AuthContext so the UI can render a subtle "reconnecting…" banner without forcing a re-login prompt.

## Alternatives considered
- **Split the AsyncStorage clear out of `logout()` and only run it on org-switch.** Rejected — the block comment at `AuthContext.tsx:150–160` explicitly documents that clearing `ppt_offline_queue` on real logout is a security posture (prior org's queued writes must not survive login as a different tenant). Keeping `resetLocalData()` inside `logout()` is intentional; the fix belongs one layer up (classify what counts as "logout").
- **Move the queue-persistence guarantee into `useOfflineSupport`** (a durable off-Storage backup that survives `resetLocalData`). Rejected — duplicates state, and the desired invariant ("keep the queue across transient network refresh failures") is a natural consequence of not routing through `logout()` in the first place. The minimal fix is the classification in `refreshToken()`.

## Root-cause trace
1. Symptom: user's offline-created faults / meter readings / votes silently disappear after opening the app in a place without network past JWT TTL.
2. ← `frontend/apps/mobile/src/hooks/useOfflineSupport.ts` reads `ppt_offline_queue` on next sync, finds it empty.
3. ← `frontend/apps/mobile/src/contexts/AuthContext.tsx:159` — `resetLocalData()` cleared `ppt_offline_queue` during `logout()`.
4. ← `frontend/apps/mobile/src/contexts/AuthContext.tsx:206` — `refreshToken()` caught the fetch-network-rejection and unconditionally invoked `logout()`.
5. ← `frontend/apps/mobile/src/contexts/AuthContext.tsx:326` — `initialize()` had called `refreshToken()` on cold start because the stored access token had passed `isJwtExpired(accessToken, TOKEN_REFRESH_SKEW_SECONDS)`.
6. Origin: the `catch(error) { await logout(); throw error; }` shape was landed together with the offline-queue purge in `resetLocalData()` at logout time (see the org-switch hygiene comment at lines 150–160). The two decisions are individually reasonable; the interaction is the bug.

## Test plan
- [ ] `frontend/apps/mobile/src/test/integration/auth.integration.test.tsx` — new case: mock refresh fetch to reject with `TypeError('Network request failed')`, seed `ppt_offline_queue`; assert queue survives + `logout()` not called + `AuthContext` did not drop `isAuthenticated`.
- [ ] Regression: existing 401 branch (lines 206–224) must still cause `logout()` + queue purge (no regression from the fix).
- [ ] Command: `pnpm -F @ppt/mobile test --testPathPattern=auth.integration`

## Out of scope
- `useOfflineSupport.ts:425` 4xx-silent-drop (tracked as separate backlog item `code-review-mobile-rn-4xx-silent-drop`, score 2 — needs its own UI-surface plan).
- Adding a persistent "failed actions" UI bucket (a broader UX change).
- Adjusting `TOKEN_REFRESH_SKEW_SECONDS` (orthogonal knob).

## After-merge
- Move this file to `plans/_archive/code-review-mobile-rn-refresh-purges-offline-queue.md`
- Mark the matching `backlog.json` row as `status: "done"`
