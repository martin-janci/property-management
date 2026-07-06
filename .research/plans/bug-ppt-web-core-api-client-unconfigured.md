# bug-ppt-web-core-api-client-unconfigured

**Vector:** bug
**Score:** 3
**Source:** commit e6b5093 (origin/dev tip at Phase 1.5 review time, 2026-07-06); Phase 1.5 static review of ppt-web-core segment
**Confidence:** high

## Hypothesis
The parallel axios stack in `frontend/apps/ppt-web/src/lib/api.ts` is never wired to a JWT token getter at production bootstrap — `configureApiClient({ getToken, onUnauthorized })` is only called from `api.test.tsx`. Two production features consume that stack (`useSentiment`, `usePredictiveMaintenance`) via `getApiClient()`; because `tokenGetter` stays `undefined` there, every request from those hooks omits the `Authorization` header and api-server returns 401. `main.tsx` only wires the generated `@ppt/api-client` (via `registerAuthInterceptors`) — nothing wires the axios stack. Smallest change: call `configureApiClient` at the same bootstrap point that runs `registerAuthInterceptors`, threading the same token source (localStorage / AuthContext) and the same 401 onUnauthorized redirect.

## Evidence
- `frontend/apps/ppt-web/src/lib/api.ts:258` — `configureApiClient({ getToken, onUnauthorized })` is the only site that arms axios; a repo grep shows this fn is called only in `api.test.tsx`.
- `frontend/apps/ppt-web/src/features/sentiment/hooks/useSentiment.ts:35` — production hook calls `getApiClient()`; `tokenGetter` is `undefined` → no `Authorization` header → 401 on `/sentiment/{dashboard,trends,alerts,thresholds,acknowledge}`.
- `frontend/apps/ppt-web/src/features/predictive-maintenance/hooks/usePredictiveMaintenance.ts:33` — same pattern, breaks `equipment/predictions/needing-maintenance/acknowledge`.
- `frontend/apps/ppt-web/src/main.tsx` — bootstrap calls `registerAuthInterceptors(client)` on the generated api-client and never touches the axios stack.

## Files
- `frontend/apps/ppt-web/src/lib/api.ts`
- `frontend/apps/ppt-web/src/main.tsx`
- `frontend/apps/ppt-web/src/features/sentiment/hooks/useSentiment.ts`
- `frontend/apps/ppt-web/src/features/predictive-maintenance/hooks/usePredictiveMaintenance.ts`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. Start the api-server (or a mock that requires a Bearer token) and boot `ppt-web` logged in as any user; navigate to a page that mounts `useSentiment` (sentiment dashboard) or `usePredictiveMaintenance` (predictive maintenance widget).
2. Open DevTools → Network. Every request from those hooks — e.g. `GET /api/v1/sentiment/dashboard` — is sent without a Bearer-scheme `Authorization` header and comes back `401`. Expected: header present, request 200 like the calls made through the generated `@ppt/api-client`.

## Suggested approach
1. In `frontend/apps/ppt-web/src/main.tsx`, right after (or beside) the existing `registerAuthInterceptors(client)` call, invoke `configureApiClient({ getToken: () => localStorage.getItem("token"), onUnauthorized: () => { /* same redirect the generated client uses */ } })` so the axios stack uses the same token source.
2. Cross-check with `frontend/apps/ppt-web/src/contexts/AuthContext.tsx` — if the token source there is not `localStorage.getItem("token")` directly, thread the AuthContext accessor (or a stable module-level ref updated by AuthContext) into `configureApiClient` so the axios stack sees fresh tokens after refresh/rotation.
3. Verify no lingering circular imports between `lib/api.ts`, `contexts/AuthContext.tsx`, and `main.tsx` — if there are, use a lazy `getToken` closure that reads from a module-scoped `let` that AuthContext writes into on mount / token change.
4. Delete or update the two feature-file comments that claim the axios client "is authenticated the same way as the generated client" now that the claim is true (see the two hook files).
5. Add a Vitest that mounts `useSentiment` inside a `configureApiClient({ getToken: () => "test-token", onUnauthorized: vi.fn() })` bootstrap, mocks axios to capture the request headers, calls the hook, and asserts the outgoing `Authorization` header equals `Bearer test-token`.
6. Add a mirror test for `usePredictiveMaintenance` (same shape, different endpoint).
7. Run `pnpm -F @ppt/ppt-web test` locally to confirm both tests pass; verify the pre-fix tree (without the `configureApiClient` bootstrap call) fails both tests — that is IG3 evidence.

## Alternatives considered
- **Migrate `useSentiment` and `usePredictiveMaintenance` to the generated `@ppt/api-client`** — rejected because the sentiment/predictive-maintenance endpoints are not currently exposed via TypeSpec/OpenAPI, so migrating requires a spec change + client regen + generated-code shift; larger blast radius than a single bootstrap wire-up.
- **Delete the axios stack entirely and force all hooks through the generated client** — rejected because `lib/api.ts` still carries feature the generated client lacks (custom retry/backoff, structured 5xx logging, offline queue for POST retries); ripping it out is a separate refactor tracked in the roadmap, not a same-day fix for a 401 regression.

## Root-cause trace
1. Symptom: `useSentiment` / `usePredictiveMaintenance` calls return HTTP 401; api-server logs `missing_bearer_token`.
2. ← Immediate cause at `frontend/apps/ppt-web/src/lib/api.ts:258` — `tokenGetter` closure inside `getApiClient()` is `undefined` because `configureApiClient` was never invoked.
3. ← Upstream cause at `frontend/apps/ppt-web/src/main.tsx` — bootstrap wires the generated `@ppt/api-client` via `registerAuthInterceptors(client)` but has no matching call for the axios stack in `lib/api.ts`.
4. Origin: `lib/api.ts` `configureApiClient` was authored as an optional injection point; the two production hooks (`useSentiment.ts`, `usePredictiveMaintenance.ts`) started consuming `getApiClient()` under the assumption that the app had already wired the token — an assumption that was never true in production. No single introduction commit — the gap is architectural, not a regression.

## Test plan
- [ ] `frontend/apps/ppt-web/src/features/sentiment/hooks/useSentiment.test.ts` — mount hook with an axios mock; assert request carries a Bearer-scheme `Authorization` header with the injected token; fails on pre-fix code (no header), passes after wire-up.
- [ ] `frontend/apps/ppt-web/src/features/predictive-maintenance/hooks/usePredictiveMaintenance.test.ts` — mirror assertion for the predictive-maintenance endpoints.
- [ ] `pnpm -F @ppt/ppt-web test`

## Out of scope
- Migrating `useSentiment` / `usePredictiveMaintenance` to the generated `@ppt/api-client` (tracked separately).
- Retiring the axios stack in `lib/api.ts` (larger refactor; separate plan).
- Refactoring `AuthContext` token storage (out of scope; keep whatever `main.tsx` uses today).

## After-merge
- Move this file to `plans/_archive/bug-ppt-web-core-api-client-unconfigured.md`
- Mark the matching `backlog.json` row as `status: "done"`
