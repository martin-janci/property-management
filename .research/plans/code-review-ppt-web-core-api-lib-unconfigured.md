# code-review-ppt-web-core-api-lib-unconfigured

**Vector:** bug
**Score:** 3
**Source:** code-review-ppt-web-core-2026-07-02 (Phase 1.5 rotating expert review)
**Confidence:** high

## Hypothesis
The ppt-web axios client is instantiated at module load with no auth wiring — `configureApiClient()` sets the `tokenGetter`/`onUnauthorized` interceptors but is only ever called from `api.test.tsx`, never from production bootstrap. Any feature slice that imports `getApiClient()` directly (predictive-maintenance + sentiment AI hooks today) fires unauthenticated requests to `/api/v1/ai/*`, so those endpoints return 401 with no refresh fallback and the feature is effectively broken for signed-in users. The smallest change is to call `configureApiClient()` from `AuthProvider` (or `main.tsx`) with the same `getToken`/`onUnauthorized` closures the app already uses for the query-client-side flow.

## Evidence
- `frontend/apps/ppt-web/src/lib/api.ts:281` — module-level `const apiClient = getApiClient()` runs before any `configureApiClient(...)` call is possible.
- `frontend/apps/ppt-web/src/lib/api.ts:258` — `configureApiClient()` is exported but `grep -rn "configureApiClient(" frontend/apps/ppt-web/src` returns only `api.ts` (definition) and `api.test.tsx` (test setup); no production caller.
- `frontend/apps/ppt-web/src/features/predictive-maintenance/hooks/usePredictiveMaintenance.ts:5,33,50,63,76` — five hooks call `getApiClient()`; the `// Auth interceptors handle token attachment` comment at line 8-11 is inaccurate.
- `frontend/apps/ppt-web/src/features/sentiment/hooks/useSentiment.ts:7` — same broken client for AI sentiment endpoints.
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx` — already owns the refresh-token + logout flow that the interceptor needs; wiring point is the `AuthProvider` mount effect.

## Files
- `frontend/apps/ppt-web/src/lib/api.ts:258`
- `frontend/apps/ppt-web/src/lib/api.ts:281`
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx`
- `frontend/apps/ppt-web/src/features/predictive-maintenance/hooks/usePredictiveMaintenance.ts`
- `frontend/apps/ppt-web/src/features/sentiment/hooks/useSentiment.ts`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
- If C4 or C5 is ticked → `local` (implementer must run on the user's Mac)
- Otherwise → `cloud-ok` (can run as a claude.ai routine via the `ppt-bridge` MCP endpoint)

Mode: cloud-ok

## Repro steps
1. Sign in to ppt-web as any manager whose org has AI features enabled.
2. Open a building detail page and mount the predictive-maintenance widget (or navigate to the sentiment analytics tab).
3. Expected: hook calls `GET /api/v1/ai/predictive-maintenance/…` with the Bearer-token auth header attached and the widget renders data. Actual: request goes out with no auth header, backend returns 401, hook surfaces error state; if the app had a working interceptor, the token would be attached and the 401 would trigger a silent refresh.

## Suggested approach
1. In `frontend/apps/ppt-web/src/contexts/AuthContext.tsx`, right after the initial mount effect wires up storage/state, call `configureApiClient({ getToken: () => tokenRef.current, onUnauthorized: handleUnauthorized })` before rendering children. Use a ref so the closure sees the latest access token without needing to reconfigure per render.
2. Add a `useEffect` in `AuthProvider` that re-installs interceptors when the storage-token-getter identity changes (defensive; expected to run once).
3. In `frontend/apps/ppt-web/src/lib/api.ts`, change the module-level `apiClient` to a lazy getter that throws (or logs a dev-mode warning) when accessed before `configureApiClient()` has run — this converts the current silent failure into a loud one during any future regression.
4. Add `frontend/apps/ppt-web/src/lib/api.test.tsx` case: assert that after `configureApiClient({...})`, a `apiClient.get('/x')` call includes an `Authorization` header sourced from `getToken`.
5. Add a hook-level test at `frontend/apps/ppt-web/src/features/predictive-maintenance/hooks/usePredictiveMaintenance.test.tsx` that mocks `axios` and asserts the outgoing request carries the Bearer token.
6. Run `pnpm -F @ppt/web test` and `pnpm -F @ppt/web typecheck`; sanity-check the sentiment hook doesn't need a matching test if the API-client-level test covers it.

## Alternatives considered
- **Inline `Authorization` header inside each hook** — rejected because the interceptor pattern already exists to centralize token refresh + logout on 401; sprinkling headers across N hooks recreates the same class of bug when a new feature is added.
- **Remove the interceptor scaffolding and switch to fetch + query-client middleware** — rejected as out-of-scope: the codebase has a working query-client refresh flow elsewhere; the minimal fix is to wire the existing axios interceptors, not rewrite the transport layer.

## Root-cause trace
1. Symptom: predictive-maintenance and sentiment AI widgets return 401 with no refresh attempt.
2. ← `getApiClient()` at `frontend/apps/ppt-web/src/features/predictive-maintenance/hooks/usePredictiveMaintenance.ts:5` returns the module-level `apiClient`.
3. ← That `apiClient` was created at `frontend/apps/ppt-web/src/lib/api.ts:281` with default (undefined) `tokenGetter` because `configureApiClient()` at `frontend/apps/ppt-web/src/lib/api.ts:258` is never called outside `api.test.tsx`.
4. Origin: introduced when the axios-based `getApiClient()` was extracted for testability but the production bootstrap wire-up was left as a TODO; specific commit not identified — surfaces first when any consumer calls `getApiClient()` from a signed-in flow.

## Test plan
- [ ] `frontend/apps/ppt-web/src/lib/api.test.tsx` — new case: `configureApiClient({ getToken: () => 'ABC' })` then `apiClient.get('/x')` (mocked adapter) → assert the outgoing request has the Bearer-token auth header set to `ABC`.
- [ ] `frontend/apps/ppt-web/src/features/predictive-maintenance/hooks/usePredictiveMaintenance.test.tsx` — new file: mount hook via `renderHook`, mock axios, assert every emitted request carries the Bearer header sourced from the auth context.
- [ ] Regression scenario: verify the 401 path still triggers `onUnauthorized`/refresh from the AuthContext (existing behavior); add a happy-path test that a 401 followed by a successful refresh replays the original request.
- [ ] Run: `pnpm -F @ppt/web test` (Vitest) and `pnpm -F @ppt/web typecheck`.

## Out of scope
- Rewriting the axios layer to fetch/query middleware.
- Fixing the `usePerformanceMetrics.ts` visibilitychange leak (separate backlog row `code-review-ppt-web-core-perf-listener-leak`).
- Fixing the AuthContext expiry-check gap (separate backlog row `code-review-ppt-web-core-init-no-expiry-check`).

## After-merge
- Move this file to `plans/_archive/<slug>.md`
- Mark the matching `backlog.json` row as `status: "done"`
