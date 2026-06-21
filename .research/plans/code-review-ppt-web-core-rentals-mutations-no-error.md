# code-review-ppt-web-core-rentals-mutations-no-error

**Vector:** bug
**Score:** 3
**Source:** Phase 1.5 review (ppt-web-core, 2026-06-21)
**Confidence:** high

## Hypothesis
The `createConnection` and `syncPlatforms` `useMutation` calls in `frontend/apps/ppt-web/src/routes/groups/rentals.tsx` only define `onSuccess` — there is no `onError` and the caller does not surface `isError`. When the Airbnb / Booking connect or sync API returns a 4xx/5xx, the UI silently swallows the failure: no toast, no UI feedback, no log. Users believe the click did nothing or that the action succeeded. The fix is to mirror the existing error-handling pattern in `financial.tsx:156-162` (translated `showToast` on error).

## Evidence
- `frontend/apps/ppt-web/src/routes/groups/rentals.tsx:258` — `createConnection = useMutation({ mutationFn: ..., onSuccess: invalidate })` with no `onError`.
- `frontend/apps/ppt-web/src/routes/groups/rentals.tsx:272` — `syncPlatforms = useMutation({ ..., onSuccess: invalidate })` with no `onError`.
- `frontend/apps/ppt-web/src/routes/groups/financial.tsx:156-162` — `sendInvoiceMutation` shows the working pattern: `onError: (err) => showToast(...)` with a `useTranslation` call site.
- The downstream `PlatformConnectionsPage` component only consumes the mutation's `mutate` / `isPending` props; it never reads `isError`, so the surrounding route is the only layer where the error can surface.

## Files
- `frontend/apps/ppt-web/src/routes/groups/rentals.tsx:258`
- `frontend/apps/ppt-web/src/routes/groups/rentals.tsx:272`
- `frontend/apps/ppt-web/src/routes/groups/financial.tsx:156`

## Dependencies
<!-- no upstream blockers -->

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
1. Open the Rentals page in `ppt-web` while signed in as a manager and pick a unit with an inactive Booking.com / Airbnb connection.
2. With the network panel open, force the next POST `/api/v1/rentals/connections` (or `/sync-platforms`) to fail (e.g. throttle to 500 via DevTools or temporarily point `VITE_API_BASE_URL` at a stub that returns `500 { message: "boom" }`).
3. Click **Connect** (or **Sync**).
4. **Expected:** a toast / inline error explains the failure and the row stays unchanged.
   **Actual:** the dialog dismisses, no toast is shown, no console error, the list is invalidated and re-fetches — the user assumes the action worked.

## Suggested approach
1. Add `onError` handlers to both mutations in `routes/groups/rentals.tsx`:
   - Import `useTranslation` from `react-i18next` (already used by `financial.tsx`) and call `showToast({ severity: 'error', message: t('rentals.errors.connectFailed') })` / `t('rentals.errors.syncFailed')`.
   - Wire the toast component (`useToast` or whatever `financial.tsx` consumes today — confirm at line 156 before duplicating).
2. Add the matching string keys under `frontend/apps/ppt-web/messages/en.json` (`rentals.errors.connectFailed`, `rentals.errors.syncFailed`) and the Slovak counterpart (`messages/sk.json` if present).
3. Optionally surface `createConnection.isError` on the `PlatformConnectionsPage` button so the button can show a retry affordance — confirm the prop already exists, otherwise skip (out of scope).
4. Add a Vitest test next to `routes/groups/rentals.tsx`: render the route, make the mutation reject, assert the toast fires once.

## Alternatives considered
- **Global `QueryClient` `defaultOptions.mutations.onError`** — rejected because it would swallow all unhandled errors uniformly with a generic message, defeating the more specific `rentals.errors.connectFailed` translation and surprising every other page that already handles its own errors.
- **Add an ErrorBoundary around the route** — rejected because mutation failures are not thrown synchronously (`useMutation` swallows them into `isError`), so an ErrorBoundary cannot intercept them. ErrorBoundary catches render-time throws, not promise rejections.

## Root-cause trace
1. Symptom: connect/sync failure silently dismisses the dialog and re-invalidates the cache.
2. ← `createConnection` / `syncPlatforms` at `rentals.tsx:258,272` define only `onSuccess`, leaving the mutation in `error` state with no UI consumer.
3. ← The original implementation copied the `onSuccess` half from a working pattern (the project standardised on `onError` later via `financial.tsx`, see PR #1610 / #1616 era) but never came back to add `onError` to `rentals.tsx`.
4. Origin: introduced when `rentals.tsx` was first wired to the generated `@ppt/api-client` (commit predates Phase 1.5 review of 2026-06-21; the route file does not appear in the post-2026-06-16 churn window for this issue).

## Test plan
- [ ] New test at `frontend/apps/ppt-web/src/routes/groups/rentals.mutations-error.test.tsx`: mock `rentalsApiCreateConnection` to reject, render the route, assert `showToast` (or the project's toast surface) was called with the expected i18n key.
- [ ] Same for `syncPlatforms`.
- [ ] Snapshot-free: assert the toast fires exactly once per error (not duplicated by the invalidation path).
- [ ] Command: `cd frontend && pnpm -F @ppt/ppt-web test -- rentals.mutations-error` (or `pnpm test` at the root).

## Out of scope
- Refactoring the inline `queryKey: ['rentals', 'connections']` to use the `lib/queryKeys.ts` factory — tracked separately as `code-review-ppt-web-core-rentals-financial-querykeys-bypass`.
- Adding `onError` to unrelated mutations elsewhere in the file (`checkIn` at :373, `checkOut` at :385) — these belong on a separate sweep unless they show the same pattern (verify but do not silently expand scope).
- Backend-side error message localisation — the route should rely on a generic translated string, not surface the server `message` verbatim.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-rentals-mutations-no-error.md`
- Mark the matching `backlog.json` row as `status: "done"`
