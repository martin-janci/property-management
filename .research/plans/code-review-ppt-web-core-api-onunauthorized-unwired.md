# code-review-ppt-web-core-api-onunauthorized-unwired

**Vector:** bug
**Score:** 4
**Source:** signal:code-review-ppt-web-core-api-onunauthorized-unwired (2026-08-16, high) + signal:code-review-ppt-web-core-api-client-no-onunauthorized (2026-08-15, medium)
**Confidence:** high

## Hypothesis

The hand-rolled axios client at `frontend/apps/ppt-web/src/lib/api.ts` has a 401-handler branch (api.ts:219-221) that only fires when a module-level `onUnauthorizedCallback` is set, but the sole call site — `AuthContext.tsx:337` — invokes `configureApiClient({ getToken: getAccessToken })` with NO `onUnauthorized`. The callback stays permanently undefined at runtime, the 401 branch is dead, and the feature hooks that consume `getApiClient()` (sentiment, predictive-maintenance) silently fail on session expiry with no silent refresh, no logout, and no redirect-to-login — a returning user with an expired access token gets stuck failing requests until they manually reload. Two independent expert reviews (one HIGH-conf, one medium-conf) traced the same defect end-to-end on the same file:line pair. Fix: pass an `onUnauthorized` handler at `AuthContext.tsx:337` that triggers the existing `refreshToken()` with single-flight semantics and falls back to `logout()` on failure — matching the documented contract already asserted by the consumer hooks.

## Evidence

- `frontend/apps/ppt-web/src/lib/api.ts:180,195-196,219-221` — `onUnauthorizedCallback` is a module-level `let` populated only inside `createAxiosInstance` from `config.onUnauthorized`; the response interceptor's 401 handling is guarded by `if (error.response?.status === 401 && onUnauthorizedCallback)`, so an unset callback makes the entire 401 recovery branch dead code.
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:337` — the ONLY production call site of `configureApiClient` passes `{ getToken: getAccessToken }` and omits `onUnauthorized`. Confirmed by `grep -rn 'configureApiClient' frontend/apps/ppt-web/src` — no other non-test caller exists.
- `frontend/apps/ppt-web/src/features/sentiment/hooks/useSentiment.ts:19` and `frontend/apps/ppt-web/src/features/predictive-maintenance/hooks/usePredictiveMaintenance.ts:10` — both consumer hooks' own comments document the shared client as doing `401 → onUnauthorized`. That contract is not wired; on session expiry these feature endpoints silently reject with a transformed `ApiError` and the user sees a stuck screen.
- `AuthContext.tsx` already owns `refreshToken()` and `logout()`, so the callback the wiring should install is available in the same closure as the `configureApiClient(...)` call — one-line contract repair, no cross-file plumbing.
- The generated `@ppt/api-client` (`packages/api-client/src/auth/interceptors.ts:36-51`, wired at `apps/ppt-web/src/main.tsx:26`) registers only a REQUEST interceptor (`registerAuthInterceptors`) that injects `Authorization` + `X-Tenant-ID`; it does not perform a silent refresh-on-401 either. This plan scopes to the hand-rolled axios client only; the generated client is out of scope.

## Files

- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:337`
- `frontend/apps/ppt-web/src/lib/api.ts:180`
- `frontend/apps/ppt-web/src/lib/api.ts:219`
- `frontend/apps/ppt-web/src/features/sentiment/hooks/useSentiment.ts:19`
- `frontend/apps/ppt-web/src/features/predictive-maintenance/hooks/usePredictiveMaintenance.ts:10`

## Dependencies



## Required capabilities

- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):** C4/C5 unticked → runnable in cloud.

Mode: cloud-ok

## Repro steps

1. Sign in to ppt-web so an access token is stored, then let the access-token TTL elapse (or evict the token from the browser's stored session mid-conversation).
2. Navigate to a feature that calls `getApiClient()` — e.g. `/sentiment` (uses `useSentiment.ts`) or `/predictive-maintenance` (uses `usePredictiveMaintenance.ts`).
3. Expected: the shared client hits a 401, `onUnauthorized` fires, `AuthContext.refreshToken()` runs, the retried request succeeds, the UI populates.
4. Actual: the interceptor at `api.ts:219` short-circuits (`onUnauthorizedCallback` is `undefined`), the promise rejects with an `ApiError('Unauthorized', 401)`, the hook's `useQuery` surfaces an error and the page stays empty until the user manually reloads and re-logs-in.

## Suggested approach

1. In `AuthContext.tsx` (near the existing `refreshToken` and `logout` closures), add a local `handleUnauthorized` callback: it calls `refreshToken()` under a single-flight guard (reuse the existing `refreshInProgressRef`-style pattern if one exists; otherwise a local `Promise<void> | null` guard is sufficient) and, on any thrown/rejected refresh, calls `logout()` and routes to `/login` (or lets the existing `logout()` side-effect handle the redirect).
2. Update the call at `AuthContext.tsx:337` from `configureApiClient({ getToken: getAccessToken })` to `configureApiClient({ getToken: getAccessToken, onUnauthorized: handleUnauthorized })`. Keep the existing cleanup in the `useEffect` return (`resetApiClient()`).
3. Confirm `api.ts:180,195-196,219-221` needs no changes — the interceptor already delegates to the callback; only the wiring is missing.
4. Add an integration test at `frontend/apps/ppt-web/src/lib/api.401-recovery.test.ts` (co-located with `api.ts`): mock a 401 response from a `getApiClient().get(...)` call, assert `onUnauthorized` is invoked exactly once even for concurrent in-flight requests (single-flight), assert a subsequent success response resolves the original promise chain, and assert a failing refresh results in `logout()` being called.
5. Update the JSDoc comments at `useSentiment.ts:9-19` and `usePredictiveMaintenance.ts:10` if any inline note misrepresents the (now-actually-live) behavior — otherwise leave untouched.
6. Manual smoke: run `pnpm --filter @ppt/web dev`, invalidate the token client-side (e.g. `localStorage.setItem('ppt.access_token', '<expired-jwt>')` if that's the storage key), reload, and verify a `/sentiment` fetch triggers a refresh then succeeds.
7. Do NOT touch the generated `@ppt/api-client` — that client's silent-refresh gap is a separate finding (`code-review-ppt-web-core-api-client-no-onunauthorized` cited it explicitly as related-but-distinct); adding response-side refresh to it is a broader change with its own test surface and belongs to a future plan.

## Alternatives considered

- **Add the silent-refresh interceptor to the generated `@ppt/api-client` instead** — rejected because the generated client is regenerated from OpenAPI (`@hey-api/openapi-ts`) and lives in `packages/api-client/`; wiring behavior there would either be lost on regeneration or require plumbing through the generator config. The hand-rolled axios instance is the one whose contract is already documented as "401 → onUnauthorized" by consumers, and the closure that owns `refreshToken`/`logout` is the natural home for the callback. Fix the wiring where the bug is.
- **Delete the dead 401 branch from `api.ts:219-221` and mark the consumer hooks as "no silent refresh"** — rejected because that regresses a documented contract (comments in `useSentiment.ts:19` and `usePredictiveMaintenance.ts:10` explicitly state `401 → onUnauthorized`), pushes the recovery burden onto every consumer, and makes the sentiment / predictive-maintenance surfaces even less resilient to token expiry than they are today.

## Root-cause trace

1. Symptom: an authenticated user hits a `getApiClient()`-consuming route after their access token expires; the request rejects with a 401 and the UI stays broken until manual reload.
2. ← `frontend/apps/ppt-web/src/lib/api.ts:219-221` — the response interceptor's 401 branch is gated on `onUnauthorizedCallback && …`. `onUnauthorizedCallback` is a module-level `let` (api.ts:180) whose only writer is `createAxiosInstance` at api.ts:195-196, populated from `config.onUnauthorized`.
3. ← `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:337` — the sole production caller of `configureApiClient` passes only `{ getToken: getAccessToken }`; the `onUnauthorized` slot is left `undefined`, so the module-level callback stays `undefined` for the lifetime of the app.
4. Origin: the `configureApiClient({ getToken: getAccessToken })` wiring was introduced when the hand-rolled axios client's token-getter contract was added (context: comment at `AuthContext.tsx:329-336` describes the token-getter wiring rationale but doesn't mention the 401 callback). The `onUnauthorized` field was added to the client's `ApiClientConfig` (api.ts:180,195-196,219-221) but never plumbed at the call site — a latent-since-introduction wiring omission, not a regression.

## Test plan

- [ ] New: `frontend/apps/ppt-web/src/lib/api.401-recovery.test.ts` — mock 401 → expect `onUnauthorized` invoked; concurrent in-flight requests → expect exactly one refresh invocation (single-flight); successful refresh → original request retries and resolves; failed refresh → `logout()` called and promise rejects with the transformed `ApiError`.
- [ ] New: `frontend/apps/ppt-web/src/contexts/AuthContext.401-wiring.test.tsx` — mount `<AuthProvider>` under a test harness, assert that `configureApiClient` is called with BOTH `getToken` and `onUnauthorized` (spy on `configureApiClient`); assert that invoking the captured `onUnauthorized` triggers the provider's `refreshToken` code path.
- [ ] Command: `pnpm --filter @ppt/web test -- --run api.401-recovery AuthContext.401-wiring` (must fail on `main` today because the callback is never passed; must pass after the fix).

## Out of scope

- Adding response-side refresh to the generated `@ppt/api-client` (separate plan; the JSDoc-fix in this plan is bounded to the hand-rolled client's consumer hooks only).
- Any redesign of the token-refresh single-flight primitive — reuse whatever pattern `AuthContext.refreshToken` already exposes; do not introduce a new abstraction.
- Broader session-recovery UX (toasts, "session expired" modals) — the plan restores the documented contract; UX polish is a follow-up.
- The related `code-review-ppt-web-core-authctx-init-no-exp-check` cold-boot init gap (separate finding) — that touches `initializeAuth` at `AuthContext.tsx:412-415`, a different code path; leave it in backlog for its own score accumulation.

## After-merge

- Move this file to `plans/_archive/code-review-ppt-web-core-api-onunauthorized-unwired.md`
- Mark the matching `backlog.json` row (`code-review-ppt-web-core-api-onunauthorized-unwired`) as `status: "done"`
