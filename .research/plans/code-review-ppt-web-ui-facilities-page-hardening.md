# code-review-ppt-web-ui-facilities-page-hardening

**Vector:** bug
**Score:** 4
**Source:** dispatcher Tier-1d rotating-expert-review 2026-09-17 (ppt-web-ui features/facilities)
**Confidence:** medium

## Hypothesis
`FacilitiesPage.tsx` — the primary list entry point of the ppt-web Facilities feature (Epic 56) — has two independent completeness defects on the same file. First, its module-local `useIsManager()` hook is a hardcoded `return true;` placeholder from before `AuthContext` was implemented, so residents see the manager-only Create/Edit affordances rendered by `FacilityList` / `FacilityCard`. Second, its `listFacilities` fetch has no `error` state — the catch block only `console.error`s, so a failed load renders as the normal empty-list UI, hiding real failures. Both are small, single-file fixes: replace the placeholder with `useAuth() + isManagerRole()` (the canonical pattern already used in `App.tsx`), and add an `error` state with an error/retry block (the canonical pattern already used in sibling `EditFacilityPage.tsx` / `BookFacilityPage.tsx`).

## Evidence
- `frontend/apps/ppt-web/src/features/facilities/pages/FacilitiesPage.tsx:35-41` — module-local `useIsManager()` hook `return true;` unconditionally; its own doc-comment admits it is a placeholder pending AuthContext. `FacilitiesPage.tsx:46` consumes it and passes to `<FacilityList isManager={isManager} />` at `:116`.
- Downstream effect: `FacilityList.tsx:82` and `:141` render the `Create facility` button `{isManager && onCreate && ...}`; `FacilityCard.tsx:151` renders per-facility `Edit` gated on `isManager`. Every user (residents included) sees these controls, though backend RLS still rejects the underlying mutations.
- Canonical role helper already exists: `frontend/apps/ppt-web/src/routes/shared.tsx` exports `isManagerRole(role)` backed by `MANAGER_ROLES`, and `AuthContext.tsx:742` exports `useAuth()`. Sibling `MyBookingsPage.tsx:106` sets `isManager={false}` by route intent, and `PendingBookingsPage.tsx:117` sets `isManager={true}` — but `FacilitiesPage` serves all roles and must derive from the real user.
- `FacilitiesPage.tsx:63-74` — `listFacilities` useEffect catch only calls `console.error('Failed to fetch facilities:', error)` and `finally` sets `isLoading=false`. No `error` state variable (`:47-51` declares `facilities/total/page/isLoading/filters` only), so `facilities` stays `[]` and the page renders normal empty UI on failure.
- Canonical error-state pattern in the same folder: `EditFacilityPage.tsx:30,91` sets `setError('Failed to load facility')` then renders `{error && ...}`; `BookFacilityPage.tsx:35,131` does the same.
- Cluster: consolidates signal ids `code-review-ppt-web-ui-facilities-ismanager-hardcoded-true` (score 2) and `code-review-ppt-web-ui-facilities-page-swallows-load-error` (score 2); both dispatcher Tier-1d 2026-09-17.

## Files
- `frontend/apps/ppt-web/src/features/facilities/pages/FacilitiesPage.tsx`
- `frontend/apps/ppt-web/src/features/facilities/components/FacilityList.tsx`
- `frontend/apps/ppt-web/src/features/facilities/components/FacilityCard.tsx`
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx`
- `frontend/apps/ppt-web/src/routes/shared.tsx`

## Dependencies
<!-- No blocking dependencies — self-contained frontend change. -->

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode:** `Mode: cloud-ok`

## Repro steps
1. `pnpm --filter @ppt/ppt-web dev` and log in as a **resident** user (any non-manager role).
2. Navigate to `/buildings/<any-id>/facilities` — the page loads.
3. Observe the `Create facility` button in the toolbar and per-facility `Edit` buttons on each `FacilityCard`. **Expected:** neither should render for a resident. **Actual:** both render (backend rejects the actual mutation with 403, but the UI presents them as available).
4. Second symptom: force a network error on the `listFacilities` request (DevTools → Network → Block request). Reload the page. **Actual:** page silently shows the normal empty-list UI as if the building has no facilities. **Expected:** an error message and a retry affordance, matching `EditFacilityPage.tsx` behaviour.

## Suggested approach
1. Open `frontend/apps/ppt-web/src/features/facilities/pages/FacilitiesPage.tsx`. Delete the local `useIsManager()` hook (`:35-41`). Add:
   - `import { useAuth } from '../../../contexts'` (or the canonical `AuthContext` path — mirror how `OrganizationProvider.tsx:6` imports it).
   - `import { isManagerRole } from '../../../routes/shared'`.
2. Inside the component, replace `const isManager = useIsManager()` at `:46` with `const { user } = useAuth(); const isManager = isManagerRole(user?.role);`.
3. Add error state alongside the existing hooks at `:47-51`: `const [error, setError] = useState<string | null>(null);`.
4. Update the fetch effect at `:63-74`: on catch, `setError(String(error?.message ?? 'Failed to load facilities'))` — keep the `console.error` for developer trace. On the try's happy path, `setError(null)` before setting facilities.
5. Add an error block in the render before the empty-list branch: `{error ? <ErrorState message={error} onRetry={() => { setError(null); refetch(); }} /> : null}` (use whatever error/retry component the sibling pages import — mirror `EditFacilityPage.tsx:91` / `BookFacilityPage.tsx:131`).
6. Verify the page still compiles clean and the tests pass: `pnpm --filter @ppt/ppt-web typecheck && pnpm --filter @ppt/ppt-web test -- FacilitiesPage`.
7. Grep the ppt-web tree for other `useIsManager` placeholders (`grep -rn "useIsManager" frontend/apps/ppt-web/src`) and file follow-up signals (out of scope for this PR).

## Alternatives considered
- **Migrate the whole `useEffect + fetch + isLoading + error` block to `useQuery`** — rejected for this PR because it widens the diff into a lifecycle refactor (query keys, cache invalidation on the `?page=` param, retry semantics); do it as a separate refactor once the error/access defects are stopped from bleeding into residents' screens.
- **Hide the Create/Edit controls at the `FacilityList` / `FacilityCard` layer by ignoring the passed `isManager` and re-deriving inside them** — rejected because the two other consumers (`MyBookingsPage.tsx`, `PendingBookingsPage.tsx`) legitimately need to force the flag by route intent; the fix belongs at the page that misuses the placeholder, not at the shared components.

## Root-cause trace
1. Symptom: residents on `/buildings/:id/facilities` see manager-only Create/Edit affordances, and load failures are indistinguishable from empty buildings.
2. ← Immediate cause at `FacilitiesPage.tsx:35-41` (`useIsManager` hardcoded `return true`) and `:63-74` (catch only `console.error`s, no error state).
3. ← Upstream cause: the page was scaffolded before `AuthContext` shipped for ppt-web; the placeholder's own comment (`"when AuthContext is implemented"`) makes the intent explicit, and the fetch pattern predates the sibling pages' error-state convention.
4. Origin: initial `FacilitiesPage` scaffold (specific commit not required for fix — `git blame` would identify it).

## Test plan
- [ ] Add / extend `frontend/apps/ppt-web/src/features/facilities/pages/FacilitiesPage.test.tsx` (or create it) with two cases:
  - Renders with a **resident** `useAuth()` mock — assert the `Create facility` button is NOT in the DOM (`queryByRole('button', { name: /create facility/i })` returns `null`).
  - Renders with a **manager** `useAuth()` mock — assert the button IS present.
- [ ] Add an error-state test: mock `listFacilities` to reject, assert an error message renders (e.g. `getByText(/failed to load/i)`) and the empty-list copy is NOT shown.
- [ ] Regression: keep the existing "renders facility list" test green.
- [ ] Run: `pnpm --filter @ppt/ppt-web typecheck && pnpm --filter @ppt/ppt-web test -- FacilitiesPage`.

## Out of scope
- Any other `useIsManager` placeholder still surviving elsewhere in ppt-web (file a follow-up signal instead — this PR is about `FacilitiesPage` alone).
- Migrating the effect to `useQuery` (documented as an alternative above).
- Backend / RLS changes — the backend already rejects unauthorized mutations; this PR only fixes the UI presentation and error handling.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-ui-facilities-page-hardening.md`
- Mark the matching `backlog.json` row (id `code-review-ppt-web-ui-facilities-page-hardening`) as `status: "done"`
