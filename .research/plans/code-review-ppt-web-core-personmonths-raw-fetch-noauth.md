# code-review-ppt-web-core-personmonths-raw-fetch-noauth

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review (ppt-web-core segment, 2026-08-27)
**Confidence:** medium

## Hypothesis
`useBuildingUnits()` in `frontend/apps/ppt-web/src/routes/groups/person-months.tsx:75-91` is the only raw `fetch()` in the entire ppt-web-core scope. Ppt-web authenticates with a localStorage bearer token via `@ppt/api-client`, and this hand-rolled call carries neither the bearer-token Authorization header nor `X-Tenant-ID`, so the /api/v1/units request 401s in production. The failure is swallowed to an empty array, silently breaking every person-months page (building summary, bulk grid, unit designations) even though the pages render without a visible error.

## Evidence
- `frontend/apps/ppt-web/src/routes/groups/person-months.tsx:75-91` — `useBuildingUnits()` uses `fetch(\`/api/v1/units?buildingId=${buildingId}&limit=500\`, { headers: { 'Content-Type': 'application/json' } })` with no Authorization header.
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:325-344` — wires `setTokenProvider` + `configureApiClient` so the generated api-client injects the bearer token; nothing similar runs for a bare `fetch()`.
- `frontend/apps/ppt-web/src/features/**` repo grep confirms every other data path goes through the generated `@ppt/api-client` hooks or `lib/api.ts` axios instance.
- Person-months surfaces (`PersonMonthsPageRoute`, `BulkEntryPageRoute`, `UnitPersonMonthsPageRoute`) all depend on `useBuildingUnits`; with `units: []` the building summary shows zero units and the bulk grid renders empty.
- Response is hand-typed inline as `{ data: Array<{ id: string; unitNumber: string }> }`, bypassing the generated `Unit` type — silent break if the envelope shape shifts.

## Files
- `frontend/apps/ppt-web/src/routes/groups/person-months.tsx`

## Dependencies
<!-- No dependencies -->

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. Sign in to ppt-web as a manager with a building that has ≥1 unit; navigate to a person-months surface for that building (e.g. `/groups/{id}/person-months`).
2. Observe the building summary lists **zero units** even though the same building's units are visible via the buildings screen. The Network tab shows GET `/api/v1/units?buildingId=...` returning 401 (no Authorization header sent).
3. Expected: units render with their designations; bulk-entry grid populates.

## Suggested approach
1. Delete the local `BuildingUnit` type and hand-rolled `fetch()` in `useBuildingUnits()`.
2. Import the generated units list hook (e.g. `useListUnits({ buildingId, limit: 500 })`) from `@ppt/api-client` — the same module `person-months.tsx` already imports `useBuilding` / `useBuildingPersonMonths` from.
3. Map the generated `Unit` shape to `{ id, designation: unit.unitNumber }` at the hook boundary so downstream components are unchanged.
4. Move the query key into the units resource namespace (e.g. `['units','by-building', buildingId]`) so a units-scoped cache invalidation reaches it.
5. If the generated client doesn't expose a units-list-by-building hook yet, use `lib/api.ts`'s axios instance (bearer + X-Tenant-ID already installed) as a fallback — never a bare `fetch()`.

## Alternatives considered
- **Patch the raw fetch to inject the bearer manually** — rejected because it duplicates `AuthContext`'s wiring and skips the X-Tenant-ID interceptor, breaking the moment a second header requirement lands (this is precisely the drift the api-client abstraction prevents).
- **Wrap all raw fetches in a linter rule instead** — rejected because there's only one raw fetch left in ppt-web-core; fixing it and adding a no-restricted-syntax rule for `fetch(` is stricter and one-shot.

## Root-cause trace
1. Symptom: person-months surfaces show zero units for buildings that clearly have units.
2. ← `useBuildingUnits()` returns `{ units: [] }` at `person-months.tsx:91` because `data` is undefined.
3. ← TanStack Query catches the thrown `Error("HTTP 401")` at `person-months.tsx:83-84` and sets `data` to undefined.
4. ← `fetch(...)` at `person-months.tsx:80` sends the request with no `Authorization` header, so the api-server rejects with 401 (ppt-web is a bearer-token, not cookie, client).
5. Origin: the file was added as a hand-rolled fetch before the code-owner conventions settled around `@ppt/api-client`; no PR-time review caught that the module bypasses the auth interceptor.

## Test plan
- [ ] `frontend/apps/ppt-web/src/routes/groups/__tests__/person-months.test.tsx` — new test: render `PersonMonthsPageRoute` with a mocked buildings/units api-client; assert the units list renders and no bare `fetch(` was called (spy on `global.fetch`).
- [ ] Regression: repo-wide `pnpm --filter @ppt/ppt-web lint` guarded by a `no-restricted-syntax` rule against `fetch(` in the `routes/` directory (allowlist only `lib/api.ts` if needed).
- [ ] Run locally: `cd frontend && pnpm --filter @ppt/ppt-web test person-months` and `pnpm --filter @ppt/ppt-web typecheck`.

## Out of scope
- Rewriting the person-months bulk-entry data flow beyond swapping the units hook.
- Auditing other apps (reality-web, mobile) for raw `fetch()` — separate scope.
- Adding a UI-visible error toast when the units list fails — that's a separate error-state pass across all person-months surfaces.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-personmonths-raw-fetch-noauth.md`
- Mark the matching `backlog.json` row as `status: "done"`
