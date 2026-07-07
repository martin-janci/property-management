# code-review-ppt-web-core-authed-query-roots-drift

**Vector:** security
**Score:** 3
**Source:** Phase 1.5 review of ppt-web-core segment on 2026-07-07 | frontend/apps/ppt-web/src/lib/queryKeys.ts:347
**Confidence:** high

## Hypothesis
`AUTHED_QUERY_KEY_ROOTS` in `frontend/apps/ppt-web/src/lib/queryKeys.ts:347-368` has drifted from the auth-scoped query-key roots actually defined in the same file plus the three feature-local root literals in `features/{sentiment,notification-analytics,predictive-maintenance}`. `AuthContext.logout()` at `contexts/AuthContext.tsx:542-544` iterates only `AUTHED_QUERY_KEY_ROOTS` when purging TanStack Query on session end, so on a same-tab user-A → user-B session swap, four auth-scoped cache subtrees — `accounting` (invoices/contacts/statements/statement lines — user-scoped financial PII), `sentiment`, `notification-analytics`, and `predictive-maintenance` — survive the logout and are readable to user B via `queryClient.getQueryData(['accounting', …])` or any component mounted with the corresponding hooks. The smallest fix is to add the four missing string roots to `AUTHED_QUERY_KEY_ROOTS`, replace the three feature-local root literals with references to the centralized keyer, and lock the invariant with a `vitest` unit test that fails if any `authedQueryKeys.*` root is absent from the purge list (regression witness).

## Evidence
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:347` — `export const AUTHED_QUERY_KEY_ROOTS = [...]` — array of literal strings iterated on logout
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:142` — `accounting` is the top-level root for `authedQueryKeys.accounting.*` (invoices, contacts, statements, statementLines, lineMatches), authored user-scoped, NOT listed in `AUTHED_QUERY_KEY_ROOTS`
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:342-345` — leading comment: "When you add a new auth-scoped query root, add it here too" — the design already anticipated this drift
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:542-544` — `logout()` calls `AUTHED_QUERY_KEY_ROOTS.forEach(root => queryClient.removeQueries({ queryKey: [root] }))`
- `frontend/apps/ppt-web/src/features/sentiment/hooks/useSentiment.ts:24`, `.../notification-analytics/hooks/useNotificationAnalytics.ts:54`, `.../predictive-maintenance/hooks/usePredictiveMaintenance.ts:15` — three feature-local roots (`'sentiment'`, `'notification-analytics'`, `'predictive-maintenance'`) bypass the central keyer entirely

## Files
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:347`
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:542`
- `frontend/apps/ppt-web/src/features/sentiment/hooks/useSentiment.ts:24`
- `frontend/apps/ppt-web/src/features/notification-analytics/hooks/useNotificationAnalytics.ts:54`
- `frontend/apps/ppt-web/src/features/predictive-maintenance/hooks/usePredictiveMaintenance.ts:15`

## Dependencies

## Required capabilities
- [ ] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. `cd frontend && pnpm dev:ppt` and log in as user A; navigate to `/accounting/invoices` (or any accounting/sentiment/notification-analytics/predictive-maintenance page) so a `useQuery(['accounting', 'invoices', …])` populates the client cache.
2. In DevTools console: `window.__TSQ_CLIENT__?.getQueryData(['accounting','invoices','list',{}])` — confirm user A's invoice list is cached (mirror the actual queryKey the hook uses).
3. Click "Log out"; then in the same tab log in as user B (different tenant / different user_id).
4. After login as B completes, run the same `getQueryData(['accounting', …])` call. **Expected:** `undefined` (cache purged). **Actual on today's `dev`:** user A's invoice list is still returned — cross-account read.

## Suggested approach
1. Extend `AUTHED_QUERY_KEY_ROOTS` in `frontend/apps/ppt-web/src/lib/queryKeys.ts:347` to include `'accounting'`, `'sentiment'`, `'notification-analytics'`, and `'predictive-maintenance'`.
2. Move the three feature-local root literals into the central `authedQueryKeys` object (add `sentiment`, `notificationAnalytics`, `predictiveMaintenance` as top-level keyer branches following the existing shape) and update the three hook files to import from `queryKeys.ts` instead of using string literals.
3. Add a `vitest` unit test at `frontend/apps/ppt-web/src/lib/queryKeys.test.ts` that iterates every top-level key on `authedQueryKeys` and asserts each string is present in `AUTHED_QUERY_KEY_ROOTS` — locks the invariant so future drift trips CI.
4. Add a second `vitest` (component-level, `@testing-library/react` + a `QueryClientProvider`) that: seeds `['accounting', 'invoices']` with a marker payload, calls `AuthContext.logout()`, then asserts `queryClient.getQueryData(['accounting', 'invoices'])` returns `undefined`. Same for one of the three feature roots.
5. Run `pnpm -F @ppt/ppt-web test src/lib/queryKeys.test.ts` and the component test to confirm both fail on `dev` (regression witnesses) and pass with the fix.
6. `pnpm check && pnpm typecheck` clean.

## Alternatives considered
- **`queryClient.clear()` on logout** — rejected because it also wipes unauthenticated caches (public content, feature-flag manifests, i18n) that the app deliberately preserves across sessions; the current selective-purge design is correct, the *list* is what drifted.
- **Runtime `authedQueryKeys` reflection instead of a hand-maintained list** — rejected because tree-shaking and the `as const` types in `queryKeys.ts` mean `Object.keys(authedQueryKeys)` isn't stable across bundle configurations, and the test-based drift lock (step 3) gives the same guarantee with zero runtime cost.

## Root-cause trace
1. Symptom: user B, freshly logged in on the same tab, reads user A's cached `['accounting', 'invoices', …]` via TanStack Query.
2. ← `AuthContext.logout()` at `contexts/AuthContext.tsx:542-544` only iterates `AUTHED_QUERY_KEY_ROOTS`, not the full set of auth-scoped roots.
3. ← `AUTHED_QUERY_KEY_ROOTS` at `lib/queryKeys.ts:347-368` is a hand-maintained subset that was never updated when `accounting` (line 142) plus three feature-local roots (`sentiment`, `notification-analytics`, `predictive-maintenance`) were added.
4. Origin: latent since each of the four missing roots landed on `dev` without touching `queryKeys.ts:347` — the maintenance rule in the file's own leading comment (`342-345`) was not enforced by a CI signal.

## Test plan
- [ ] `frontend/apps/ppt-web/src/lib/queryKeys.test.ts` — new unit test: every top-level key on `authedQueryKeys` MUST be present in `AUTHED_QUERY_KEY_ROOTS` (fails today on `accounting`, would fail again the moment someone adds a new authed root without updating the list)
- [ ] `frontend/apps/ppt-web/src/contexts/AuthContext.logout.test.tsx` — new component test: seed `['accounting', 'invoices']` with a marker, call `logout()`, assert `queryClient.getQueryData(...)` is `undefined` (regression witness — passes today only for keys already in `AUTHED_QUERY_KEY_ROOTS`)
- [ ] Local run: `pnpm -F @ppt/ppt-web test -- src/lib/queryKeys.test.ts src/contexts/AuthContext.logout.test.tsx`

## Out of scope
- Refactoring `AUTHED_QUERY_KEY_ROOTS` into a fully reflected set from `authedQueryKeys` (see Alternatives).
- Migrating the three feature hooks off inline literals is only *required* insofar as their roots land in the central array — deeper refactor of the feature hooks' key shapes is a separate concern.
- Adjusting the SSR/hydration behavior of TanStack Query — not implicated in the same-tab session-swap path.
- The parallel server-side authorization gates on `/api/v1/accounting/*` — unchanged; this plan is exclusively about the client-side cache purge invariant.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-authed-query-roots-drift.md`
- Mark the matching `backlog.json` row (`code-review-ppt-web-core-authed-query-roots-drift`) as `status: "done"`
