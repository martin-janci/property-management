# code-review-ppt-web-core-person-months-fetch-missing-auth

**Vector:** bug
**Score:** 3
**Source:** Phase 1.5 rotating expert review (segment `ppt-web-core`, 2026-07-02)
**Confidence:** high

## Hypothesis
`person-months.tsx`'s `useBuildingUnits` hook makes a raw `fetch('/api/v1/units?buildingId=...')` with only a `Content-Type` header — no bearer-token authorization header and no `X-Tenant-ID`. Every sibling call in the same folder builds those headers via `useRentalsAuth()` (from `rentals.tsx`) or goes through `@ppt/api-client`, whose interceptor `setTokenProvider(getAccessToken)` wired in `AuthContext.tsx:325` attaches auth. A raw `window.fetch` bypasses that interceptor entirely, so the person-months screen either fails 401 in prod (silent — the hook only logs "HTTP 401") or, worse, hits the endpoint from an anonymous browser that still holds a session cookie. Replacing the raw fetch with the generated `@ppt/api-client` `listUnits(buildingId, {limit: 500})` call (or the `useListUnits` hook) closes the gap and matches the rest of the app.

## Evidence
- `frontend/apps/ppt-web/src/routes/groups/person-months.tsx:80` — `fetch('/api/v1/units?buildingId=…&limit=500', { headers: { 'Content-Type': 'application/json' } })` — no auth, no tenant.
- `frontend/apps/ppt-web/src/routes/groups/rentals.tsx:169-179` — canonical convention: `useRentalsAuth()` returns `{ authorization, xTenantId }` from `getToken()` + `useAuth().user.organizationId`.
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:325` — `setTokenProvider(getAccessToken)` hooks only `@ppt/api-client`, not `window.fetch`.
- `frontend/packages/api-client/src/buildings/api.ts:316` — `listUnits(buildingId, params)` exists and is auth-wired.
- `frontend/packages/api-client/src/buildings/hooks.ts:212-220` — `useListUnits(buildingId, params)` — the intended callsite.

## Files
- `frontend/apps/ppt-web/src/routes/groups/person-months.tsx`
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [x] C3 — Dev instance running (verify the fixed hook against the running backend)
- [ ] C4 — Browser  · local-only
- [ ] C5 — ADB device  · local-only
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Sign in to `ppt-web` locally (`pnpm dev:ppt`) as a manager whose org has ≥1 building with units.
2. Navigate to a person-months screen for that building (route group `groups/person-months`).
3. Open DevTools Network. Observe the `GET /api/v1/units?buildingId=<id>&limit=500` request headers.
4. Expected after fix: the bearer-token authorization header carrying `<jwt>` present, `X-Tenant-ID: <organizationId>` present. Actual today: only `Content-Type: application/json` — the request either 401s (backend rejects) or serves data cross-tenant if cookie-auth still resolves.

## Suggested approach
1. Import the generated client hook: `import { useListUnits } from '@ppt/api-client'` (or the api function `listUnits` if a raw fetch shape is preferred inside the existing `useQuery`).
2. Replace the `useBuildingUnits` implementation (`person-months.tsx:74-91`) with `useListUnits(buildingId ?? '', { limit: 500 })` (guarded by `enabled: !!buildingId`).
3. Map the response with the existing `.data ?? []` map to `{ id, designation: unitNumber }` — keep the return shape stable.
4. Delete the raw `fetch` and the local `res.ok` / `HTTP ${res.status}` error path (TanStack Query surfaces error state uniformly).
5. Run `pnpm --filter @ppt/web typecheck` — confirm no red.
6. Run `pnpm --filter @ppt/web test` — extend or add a test file (see *Test plan*).
7. Cross-check `AuthContext.tsx:325` — no change needed there, but confirm `setTokenProvider(getAccessToken)` still runs on mount so the api-client client is armed by the time `useListUnits` fires.

## Alternatives considered
- **Add `Authorization` and `X-Tenant-ID` to the raw `fetch` call by calling `getToken()` + `useAuth()` here directly** — rejected because it duplicates `useRentalsAuth`'s pattern in a third callsite and keeps the app's `window.fetch` bypass surface alive. The generated client is the single source of truth.
- **Wrap `window.fetch` globally with an auth interceptor** — rejected because it changes semantics for every third-party lib that shares the same `fetch` and hides where auth is enforced. The generated api-client already owns this.

## Root-cause trace
1. Symptom: person-months screen silently 401s (or returns cross-tenant data if a stale session cookie resolves), because `useBuildingUnits`'s raw fetch carries no `Authorization` / `X-Tenant-ID`.
2. ← `frontend/apps/ppt-web/src/routes/groups/person-months.tsx:80` — `fetch('/api/v1/units?…', { headers: { 'Content-Type': … } })` — bypasses `@ppt/api-client`.
3. ← `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:325` — `setTokenProvider(getAccessToken)` only wires the api-client instance, never `window.fetch`, so raw fetches inherit no auth.
4. Origin: introduced in the person-months implementation (see git-blame; likely landed alongside epic-3 / accounting features Q2 2026 without the reviewer catching the raw fetch amid the surrounding auth-wired sibling calls).

## Test plan
- [ ] Unit test in `frontend/apps/ppt-web/src/routes/groups/__tests__/person-months.test.tsx` (create if absent): mock `@ppt/api-client`'s `useListUnits` and assert `useBuildingUnits` invokes it exactly once per non-empty `buildingId`, `enabled` respected on empty id.
- [ ] Regression scenario: an integration test where the api-client is *not* armed (`setTokenProvider` skipped) surfaces the failure via `useListUnits`'s standard error branch — proving the query now goes through the interceptor instead of `window.fetch`.
- [ ] Exact command: `pnpm --filter @ppt/web test -- person-months` (Vitest) + `pnpm --filter @ppt/web typecheck`.

## Out of scope
- Migrating `useRentalsAuth` callsites in `rentals.tsx` to `@ppt/api-client` (separate refactor — 5+ callsites; do not couple this fix to it).
- Backend behavior on unauthenticated `/api/v1/units` — assumed already correct (401). No backend change.
- The `person-months` hardcoded-strings finding (`code-review-ppt-web-core-person-months-hardcoded-strings`, score 1) — separate row in `backlog.json`.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-person-months-fetch-missing-auth.md`
- Mark the matching `backlog.json` row as `status: "done"`
