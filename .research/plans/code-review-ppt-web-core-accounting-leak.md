# code-review-ppt-web-core-accounting-leak

**Vector:** security
**Score:** 2
**Source:** Phase 1.5 code-review (ppt-web-core, 2026-06-29)
**Confidence:** high

## Hypothesis

`AUTHED_QUERY_KEY_ROOTS` in `frontend/apps/ppt-web/src/lib/queryKeys.ts:347–368`
enumerates every root key that `AuthContext.logout()` purges from the TanStack
Query cache, but it omits `'accounting'`. `queryKeys.accounting.all` is rooted
at `'accounting'` (line 142–151), so when a user logs out, their cached
invoices, contacts, statements, statement lines, and line-match queries are
**not** evicted. The next user to log in on the same browser tab loads their
own session but TanStack returns the prior user's cached financial data
until the next refetch. This is a regression of the same scoping bug that
issue #712 closed for other roots. Fix: add `'accounting'` to
`AUTHED_QUERY_KEY_ROOTS` (one line) and add a unit test that purges every
known root on logout.

## Evidence

- `frontend/apps/ppt-web/src/lib/queryKeys.ts:142–151` — `queryKeys.accounting.all = ['accounting'] as const`; downstream keys (`invoices`, `contacts`, `statements`, `statementLines`, `lineMatches`) all hang off this root.
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:347–368` — `AUTHED_QUERY_KEY_ROOTS` lists every root that should be purged on logout. `'accounting'` is absent.
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:542–544` — `logout()` calls `queryClient.removeQueries({ queryKey: [root] })` for each entry in `AUTHED_QUERY_KEY_ROOTS`; accounting keys are skipped.
- Issue #712 — original logout-cache-leak fix for other roots; this is the same class of bug regressing for accounting.

## Files

- `frontend/apps/ppt-web/src/lib/queryKeys.ts`
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx`

## Dependencies

<!-- No upstream dependencies — fix is self-contained inside ppt-web. -->

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

1. Build ppt-web (`pnpm -F @ppt/ppt-web build`) or run `pnpm dev:ppt`.
2. Log in as user A, navigate to any accounting page (invoices / contacts / statements). Network tab shows `/api/v1/accounting/...` calls; results are cached.
3. Log out via the user menu.
4. Log in as user B (different organization) in the same tab.
5. Open browser DevTools → React Query devtools (or run `queryClient.getQueryCache().findAll(['accounting'])`).
6. **Expected:** every cached query under `['accounting', ...]` is removed at step 3 and only user B's data is fetched at step 4.
7. **Actual:** user A's invoice / contact / statement entries still exist in the cache; React components reading those keys initially render user A's financial data until React Query refetches.

## Suggested approach

1. Append `'accounting'` to the `AUTHED_QUERY_KEY_ROOTS` array in `frontend/apps/ppt-web/src/lib/queryKeys.ts:347–368`. Maintain alphabetical order if the existing array is sorted; otherwise group with related domain roots.
2. Add a unit test at `frontend/apps/ppt-web/src/lib/queryKeys.test.ts` (create if absent) asserting that **every** root used by `queryKeys.*.all` appears in `AUTHED_QUERY_KEY_ROOTS`. Implementation: enumerate `Object.keys(queryKeys)` and assert each top-level entry's `.all[0]` is contained. This is the regression guard against this class of bug.
3. Add an `AuthContext` integration test (Vitest + React Testing Library) that mounts an `AuthProvider`, primes the cache with an accounting query, calls `logout()`, and asserts `queryClient.getQueryCache().findAll(['accounting'])` is empty.
4. Run `pnpm -F @ppt/ppt-web check` (Biome + typecheck) and `pnpm -F @ppt/ppt-web test` (Vitest); both green.
5. Open PR titled `fix(ppt-web): purge accounting query cache on logout (security-cache-leak)` referencing this plan slug and issue #712 lineage.

## Alternatives considered

- **Switch `AuthContext.logout()` to `queryClient.clear()` (purge everything)** — rejected because some keys are intentionally session-independent (locale, feature flags, build info) and clearing them causes a visual flicker + extra fetches for the next user. The allow-list approach in the existing code is correct; the bug is that the list is incomplete.
- **Introduce a top-level `'authed'` namespace and re-root every key** — rejected because it's a much larger refactor that touches every accounting / non-accounting page; out of scope for a one-line scoping fix. Open as a separate refactor vector if desired.

## Root-cause trace

1. Symptom: After logout, switching user reveals previous user's accounting data until next refetch (data leak across sessions).
2. ← `AuthContext.tsx:542–544` `logout()` purges only the roots in `AUTHED_QUERY_KEY_ROOTS`.
3. ← `queryKeys.ts:347–368` `AUTHED_QUERY_KEY_ROOTS` was extended in #712 but `'accounting'` (added later by the accounting MVP — PR #1808 / #1821) was never appended.
4. Origin: accounting query-key infrastructure landed in `queryKeys.ts` without a corresponding `AUTHED_QUERY_KEY_ROOTS` entry. Likely PR #1808 (`feat(accounting-web): public marketing/landing + signup`) or #1821 (`Epic ACC: Invoicing & Accounting MVP`).

## Test plan

- [ ] `frontend/apps/ppt-web/src/lib/queryKeys.test.ts` — regression guard: every `queryKeys.<root>.all` value must appear in `AUTHED_QUERY_KEY_ROOTS`. Fails today (before the one-line fix), passes after.
- [ ] `frontend/apps/ppt-web/src/contexts/AuthContext.test.tsx` (or extend existing) — primes the React Query cache with an accounting key, calls `logout()`, asserts cache is empty under `['accounting']`. Fails today, passes after.
- [ ] `pnpm -F @ppt/ppt-web test` — full Vitest suite green.
- [ ] `pnpm -F @ppt/ppt-web check && pnpm -F @ppt/ppt-web typecheck` — lint + type green.

## Out of scope

- Refactoring `queryKeys.ts` into a single `'authed'` namespace (separate refactor vector).
- Adding e2e Playwright coverage for the cross-user cache-leak path (integration tests in step 2/3 are sufficient evidence).
- Touching `admin-web` or `reality-web` query-key infrastructure — this plan is scoped to `ppt-web`.

## After-merge

- Move this file to `plans/_archive/code-review-ppt-web-core-accounting-leak.md`.
- Mark the matching `backlog.json` row (`id: code-review-ppt-web-core-accounting-leak`) as `status: "done"` with the resolving PR in `sources`.
- If the regression-guard unit test catches a similar omission for any other root in a future code review, link this plan as prior art.
