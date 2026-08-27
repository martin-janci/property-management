# code-review-ppt-web-core-personmonths-raw-fetch-noauth

**Vector:** bug
**Score:** 3
**Source:** code-review-finding (ppt-web-core segment, 2026-08-27 tier1d review)
**Confidence:** medium

## Hypothesis
`useBuildingUnits()` in `frontend/apps/ppt-web/src/routes/groups/person-months.tsx` calls `/api/v1/units?buildingId=…&limit=500` via a hand-rolled `fetch()` that sets only `Content-Type` — it never attaches the `Authorization/Bearer <accessToken>` header the rest of the SPA uses (see `lib/api.ts:189-204`). In every environment where cookie-auth is not falling back (staging, cloud), the request returns 401 and the person-months page renders as "no units" instead of failing loudly. The fix is to route the call through the shared authed client (either `@ppt/api-client`'s generated units API or the `createAuthedFetch` used elsewhere) so the token is attached automatically.

## Evidence
- `frontend/apps/ppt-web/src/routes/groups/person-months.tsx:75-91` — `useBuildingUnits` — bare `fetch()` with headers only `Content-Type`.
- `frontend/apps/ppt-web/src/lib/api.ts:189-204` — the app-wide `client()` explicitly attaches `Authorization/Bearer ${token}` when a token is present.
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:84,120` — `getAccessToken(): string | null` is the canonical accessor other pages use.
- Rotating expert review 2026-08-27 (segment `ppt-web-core`): flagged as `code-review-finding`, score_delta=3, confidence=medium, vector=bug.
- Same pattern was already fixed elsewhere in this app (`useQuery` + `@ppt/api-client` calls); person-months is the outlier.

## Files
- `frontend/apps/ppt-web/src/routes/groups/person-months.tsx:75`
- `frontend/apps/ppt-web/src/lib/api.ts`
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Sign in to `ppt-web` as a manager whose org has ≥1 building with ≥1 unit and open the Person-months route.
2. In the browser network panel, filter to `/api/v1/units`; the request goes out with only `Content-Type` and no `Authorization` header.
3. Expected: request is authenticated and the building's units render in the select.
4. Actual: request returns `401` (or renders 0 units silently if the environment lets unauthed reads through), so the "unit" combobox shows an empty list and the person-months table cannot be populated.

## Suggested approach
1. Replace the raw `fetch()` in `useBuildingUnits()` with the generated `@ppt/api-client` units-list call (mirroring how other queries on the same page use the SDK), OR — if the SDK doesn't yet expose `listUnitsForBuilding`, wrap it in the shared `client()` helper so the auth header is attached automatically.
2. If the SDK path is chosen, delete `useBuildingUnits`'s inline typing (`Array<{id, unitNumber}>`) and rely on the generated response type.
3. Preserve the existing `queryKey: ['person-months', 'building-units', buildingId]` and `enabled: !!buildingId` semantics so TanStack Query caches behave identically.
4. Update the `.map()` back to `{ id, designation: u.unitNumber }` on the new return type.
5. Add a Vitest test in `frontend/apps/ppt-web/src/routes/groups/__tests__/person-months.spec.tsx` (or the existing sibling test file) mocking the SDK/authed client and asserting the request path + that the render shows units.
6. `pnpm -F @ppt/ppt-web typecheck && pnpm -F @ppt/ppt-web test -- person-months` locally.

## Alternatives considered
- **Attach Bearer manually inside the existing `fetch()`** — rejected because it re-introduces the drift the shared `client()` was created to avoid; the SDK path is what every other page uses.
- **Silently ignore** (call is cookie-authed elsewhere) — rejected because staging + cloud environments do not fall back on cookies for `/api/v1/*`, and the failing case is invisible in dev where cookies happen to work.

## Root-cause trace
1. Symptom: `/api/v1/units?buildingId=…` request from `person-months.tsx:75-91` returns 401 in non-cookie environments; unit combobox stays empty.
2. ← `useBuildingUnits` (`person-months.tsx:75-91`) uses a bare `fetch()` with only `Content-Type` and no `Authorization`.
3. ← The rest of the SPA routes queries through `lib/api.ts:189-204`'s `client()` which pulls `getAccessToken()` (`contexts/AuthContext.tsx:84,120`) into `Authorization/Bearer …`.
4. Origin: the person-months route was introduced as a one-off page that hand-rolled the units fetch instead of adding a units accessor to `@ppt/api-client` — the SDK gap was papered over rather than closed.

## Test plan
- [ ] `frontend/apps/ppt-web/src/routes/groups/__tests__/person-months.spec.tsx` — mock the units fetch/SDK call and assert (a) headers include `Authorization/Bearer` and (b) the mapped `BuildingUnit[]` renders in the select.
- [ ] Regression: assert that clearing the token causes the query to be disabled or to render an auth error rather than an empty list.
- [ ] `pnpm -F @ppt/ppt-web test -- person-months && pnpm -F @ppt/ppt-web typecheck`

## Out of scope
- The rest of the person-months page (mappers, form validation, submit handlers).
- Any refactor of the `/api/v1/units` server-side endpoint.
- Extending `@ppt/api-client` beyond what this call needs.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-personmonths-raw-fetch-noauth.md`
- Mark the matching `backlog.json` row as `status: "done"`
