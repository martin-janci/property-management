# code-review-ppt-web-core-person-months-units-fetch-unauthed

**Vector:** bug
**Score:** 3
**Source:** Phase 1.5 review of ppt-web-core segment (2026-07-05)
**Confidence:** high

## Hypothesis

`useBuildingUnits` in `routes/groups/person-months.tsx:80` calls `fetch('/api/v1/units?buildingId=…&limit=500', { headers: { 'Content-Type': 'application/json' } })` with no `Authorization` header and without using the generated `unitsApiList` in `@ppt/api-client`. The api-server units endpoint requires authentication, so the query always 401s for logged-in users; TanStack Query retries, the promise throws `HTTP 401`, and the building-unit dropdown on the person-months page silently stays empty. Additionally the code re-declares a wire shape (`{ data: [{ id, unitNumber }] }`) that already ships as `UnitsApiListResponses` in the generated client, drifting whenever the OpenAPI shape renames a field. The fix is to swap the raw `fetch` for `unitsApiList(...)` from `@ppt/api-client` (which reuses the shared client's interceptors, including auth) and delete the hand-typed row shape.

## Evidence

- `frontend/apps/ppt-web/src/routes/groups/person-months.tsx:75-93` — `useBuildingUnits` hand-authors the fetch, no `Authorization` header, and the queryFn maps `body.data` into `BuildingUnit[]`.
- `frontend/packages/api-client/src/generated/sdk.gen.ts:807` — `unitsApiList` already exists (`GET /api/v1/units`) and takes typed `UnitsApiListData` params. The same file re-exports the response types (`UnitsApiListResponses`) that the hook currently re-declares.
- Contrast pattern (works): sibling route hooks that call `@ppt/api-client` symbols pick up the shared auth interceptor set up in `main.tsx` / `authApiClient.ts`.

## Files

- `frontend/apps/ppt-web/src/routes/groups/person-months.tsx`
- `frontend/packages/api-client/src/generated/sdk.gen.ts`

## Dependencies

<!-- none -->

## Required capabilities

- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser  · **local-only**
- [ ] C5 — ADB device  · **local-only**
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps

1. Log into ppt-web as a manager with at least one building that has units.
2. Navigate to the person-months page (route: `/groups/person-months` — check `routes/groups/person-months.tsx`), pick a building in the header selector.
3. DevTools → Network: the `GET /api/v1/units?buildingId=…&limit=500` request goes out with no `Authorization` header; response is `401 Unauthorized`.
4. Observed: the "unit" dropdown stays empty, and the page renders no cards even when the user has units. Expected: the request includes the `Authorization` header carrying a bearer JWT, returns `200`, and the dropdown populates.

## Suggested approach

1. In `person-months.tsx`, remove the local `queryFn: async () => fetch(...)` block from `useBuildingUnits` (lines 79-89) and replace it with a call to `unitsApiList({ query: { buildingId, limit: 500 } })` (or the equivalent signature — verify against `sdk.gen.ts`).
2. Delete the private `body: { data: [{ id, unitNumber }] }` type declaration in the queryFn; use `UnitsApiListResponses` from `@ppt/api-client` (or destructure `data.data` and let inference carry the types).
3. Preserve the mapping to `BuildingUnit { id, designation }` — the UI shape depends on `designation` (the historical `unitNumber` field) so keep the projection. Confirm the generated type still exposes a compatible field; if the wire field was renamed, update the projection.
4. Ensure the `enabled: !!buildingId` guard is preserved so the query does not fire until a building is picked.
5. Add a Vitest that renders the hook via `renderHook` under a `QueryClientProvider`, mocks `unitsApiList`, and asserts the mapping into `BuildingUnit[]`.

## Alternatives considered

- **Attach an `Authorization` header to the raw `fetch`** — rejected because it fixes symptom (auth) but leaves the hand-typed row shape drifting from the generated client (the exact class the api-client generator exists to prevent). The generated-client swap fixes both defects.
- **Move the fetch into a feature hook (`features/person-months/api.ts`)** — rejected as scope creep for this bug fix. The route is small; keeping the query colocated is fine so long as it uses the shared client. Track the extraction separately if the routes/features boundary elsewhere warrants it.

## Root-cause trace

1. Symptom: the unit dropdown on the person-months page is empty for logged-in users with units; DevTools shows `401 Unauthorized` on `/api/v1/units?buildingId=…`.
2. ← Immediate cause: `useBuildingUnits` at `routes/groups/person-months.tsx:80` calls `fetch(...)` without `Authorization` header.
3. ← Upstream cause: the person-months route was authored without adopting `@ppt/api-client` (the generated symbols existed at the time — `unitsApiList` has been in `sdk.gen.ts` since Epic 5.x), and the hand-authored fetch was accepted at review.
4. Origin: git blame `routes/groups/person-months.tsx:80` — the commit that introduced the file.

## Test plan

- [ ] Add `frontend/apps/ppt-web/src/routes/groups/person-months.test.tsx` (or a `useBuildingUnits.test.ts` sibling) — mock `unitsApiList`, render the hook via `renderHook` + `QueryClientProvider`, and assert (a) the hook returns the mapped `{ id, designation }` shape and (b) `enabled: false` prevents any call while `buildingId` is undefined.
- [ ] Case: mocked client throws → hook's `isLoading` flips to false, `units` stays `[]`.
- [ ] Local command: `pnpm -F ppt-web test src/routes/groups`.

## Out of scope

- Consolidating other raw-`fetch` sites in `ppt-web` to `@ppt/api-client`. Track as a follow-up (there are at least a handful — the ai-chat feature is one, see the sibling plan).
- Any change to the api-server units route.

## After-merge

- Move this file to `plans/_archive/code-review-ppt-web-core-person-months-units-fetch-unauthed.md`
- Mark the matching `backlog.json` row as `status: "done"`
