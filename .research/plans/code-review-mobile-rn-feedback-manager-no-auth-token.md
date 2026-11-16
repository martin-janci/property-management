# code-review-mobile-rn-feedback-manager-no-auth-token

**Vector:** security
**Score:** 3
**Source:** Phase 1.5 rotating expert review 2026-11-15 mobile-rn
**Confidence:** high

## Hypothesis
The mobile-rn FeedbackManager singleton (`frontend/apps/mobile/src/onboarding/FeedbackManager.ts:290`) is exported pre-constructed but `setAuthToken` is never called from the app — every `POST /api/v1/feedback` leaves the device without a bearer-token authorization header. If the api-server enforces auth on `/feedback`, every submission is dropped (401) and quietly queued for infinite retry; if the endpoint accepts anonymous POSTs, every submission is persisted without user attribution. The smallest change wires `AuthContext` token updates into `feedbackManager.setAuthToken(token)` the same way `WidgetBridge` and `NFCAccessController` already do.

## Evidence
- `frontend/apps/mobile/src/onboarding/FeedbackManager.ts:290` — `export const feedbackManager = new FeedbackManager(API_BASE_URL, APP_VERSION, BUILD_NUMBER)` is the sole instantiation.
- `frontend/apps/mobile/src/onboarding/FeedbackManager.ts:259-272` — `apiRequest` only sets the bearer header `headers.Authorization = 'Bearer ${this.authToken}'` when `this.authToken` is truthy; otherwise the header is omitted.
- `grep -rn "feedbackManager.setAuthToken" frontend/apps/mobile/src` returns zero hits (confirmed 2026-11-15).
- `frontend/apps/mobile/src/screens/help/FeedbackScreen.tsx:20` imports `feedbackManager` and calls `submitFeedback` directly — no token plumbing.
- Same bug class as the just-landed PR #3537 (`code-review-mobile-rn-widget-bridge-auth-token-never-set`) — identical shape, different singleton.

## Files
- `frontend/apps/mobile/src/onboarding/FeedbackManager.ts:290`
- `frontend/apps/mobile/src/screens/help/FeedbackScreen.tsx:20`
- `frontend/apps/mobile/src/contexts/AuthContext.tsx`

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
1. Check out `dev`; `cd frontend/apps/mobile`; sign in to any test tenant on the Expo build.
2. Open the Help → Feedback screen, enter any text, tap Send.
3. On the api-server side, observe the `/api/v1/feedback` request headers — Authorization header is absent.
4. Expected: a bearer-token authorization header carrying the signed-in user's access token is present; feedback is attributed to the signed-in user. Actual: no auth header; endpoint either returns 401 (and the body goes to the PENDING retry queue forever) or persists an anonymous row.

## Suggested approach
1. Mirror the wiring PR #3537 used for `WidgetBridge`: inside `frontend/apps/mobile/src/contexts/AuthContext.tsx` (or whichever effect propagates the access token), call `feedbackManager.setAuthToken(tokens?.accessToken ?? null)` on every token change and on sign-out.
2. In `FeedbackManager.ts`, make `setAuthToken` clear `pendingFeedback` entries that were created pre-auth so the retry loop stops thrashing on the stored anonymous bodies.
3. Add `frontend/apps/mobile/src/onboarding/FeedbackManager.test.ts` — Jest + fetch-mock: (a) `submitFeedback` without `setAuthToken` → request has no Authorization; (b) after `setAuthToken('xyz')` → request authorization header matches `Bearer xyz`; (c) AuthContext hook test that logout clears the token on the singleton.
4. Sanity-check the matching endpoint in `backend/servers/reality-server/src/routes/feedback.rs` (or api-server, whichever mounts `/api/v1/feedback`) — confirm it currently requires auth; if not, file a follow-up.
5. `pnpm -F mobile test` to lock behaviour.

## Alternatives considered
- **Make the FeedbackManager fetch the token lazily from SecureStore inside `apiRequest`** — rejected because async token reads inside a retry loop complicate error handling and duplicate `AuthContext`'s refresh logic; AuthContext already knows when the token rotates.
- **Guard `submitFeedback` to no-op when `authToken` is null and surface "please sign in"** — rejected because an unauthenticated kiosk shouldn't reach the Feedback screen at all; if it does, the fix is to wire auth state, not to paper over the gap.

## Root-cause trace
1. Symptom: `/api/v1/feedback` POST ships without a bearer-token authorization header — api-server either returns 401 and the submission is queued forever, or logs an unattributed row.
2. ← Immediate cause at `FeedbackManager.ts:268-272` — Authorization header only set when `this.authToken` is truthy, and the token is null by default.
3. ← Upstream cause at `FeedbackManager.ts:57-59` — `setAuthToken` exists but no call site wires AuthContext token changes into the singleton (vs `WidgetBridge`/`NFCAccessController` which do).
4. Origin: the FeedbackManager module was added in the onboarding epic without an AuthContext consumer; subsequent singletons added the wiring, this one was missed.

## Test plan
- [ ] `frontend/apps/mobile/src/onboarding/FeedbackManager.test.ts` — asserts Authorization header is set after `setAuthToken` and absent before.
- [ ] `frontend/apps/mobile/src/contexts/AuthContext.test.tsx` — asserts logout propagates `feedbackManager.setAuthToken(null)` (regression for shared-device logout).
- [ ] Local command: `pnpm -F mobile test -- --testPathPattern=FeedbackManager`.

## Out of scope
- Rewriting the retry queue to drop pre-auth PENDING entries beyond what the fix touches.
- Backend changes to the `/feedback` endpoint auth policy — surface a follow-up if the endpoint currently accepts anonymous POSTs.
- Other mobile singletons missing auth wiring — this plan only covers FeedbackManager.

## After-merge
- Move this file to `plans/_archive/code-review-mobile-rn-feedback-manager-no-auth-token.md`
- Mark the matching `backlog.json` row as `status: "done"`
