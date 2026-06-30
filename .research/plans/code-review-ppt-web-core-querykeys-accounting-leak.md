# code-review-ppt-web-core-querykeys-accounting-leak

**Vector:** security
**Score:** 3
**Source:** Phase 1.5 review of ppt-web-core segment (2026-06-30)
**Confidence:** high

## Hypothesis

The `AUTHED_QUERY_KEY_ROOTS` array in `frontend/apps/ppt-web/src/lib/queryKeys.ts` is missing the `'accounting'` root, even though `queryKeys.accounting` (invoices, contacts, statements, lines, matches) is auth-scoped and consumed by `PaymentMatchingPage.tsx` + `AccountingInvoiceManagementPage.tsx`. `AuthContext.logout` iterates this list and calls `queryClient.removeQueries({ queryKey: [root] })` for each. Because `accounting` is absent, the previous user's invoices/contacts/statements remain in the TanStack Query cache and become visible to whoever logs in next on the same device. The file's own doc-comment (lines 342–346) explicitly warns: *"When you add a new auth-scoped query root, add it here too — otherwise the cached data will leak into the next user's session."* The smallest change is a one-line array addition + a regression test asserting the cache is empty after logout.

## Evidence

- `frontend/apps/ppt-web/src/lib/queryKeys.ts:142-150` — `queryKeys.accounting` factory (`invoices`, `contacts`, `statements`, `statementLines`, `lineMatches`).
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:347-368` — `AUTHED_QUERY_KEY_ROOTS` array; lists 18 roots, no `accounting`.
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:342-346` — doc-comment warns about exactly this failure mode.
- Auth-scoped consumers: `PaymentMatchingPage.tsx`, `AccountingInvoiceManagementPage.tsx` (per Phase 1.5 review).
- Sibling pattern in the same file: every other feature-area root (`announcements`, `documents`, `votes`, …) is duplicated in both the factory and `AUTHED_QUERY_KEY_ROOTS`.

## Files

- `frontend/apps/ppt-web/src/lib/queryKeys.ts:347`
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx`

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

1. Log in to ppt-web as User A whose org has accounting data; navigate to Payment Matching or Accounting Invoice Management so that `queryKeys.accounting.invoices()` / `.contacts()` / `.statements()` populate the TanStack Query cache.
2. Without refreshing the tab, log out (calls `AuthContext.logout` → iterates `AUTHED_QUERY_KEY_ROOTS` to `removeQueries`).
3. Open DevTools and run `queryClient.getQueryCache().findAll({ queryKey: ['accounting'] })` (or inspect via React Query DevTools) — **expected** after logout: `[]`. **Actual:** the accounting cache entries from User A are still present.
4. Log in as User B in the same tab. Navigate to a route that reads from the accounting cache before the new fetch completes — User A's invoices/contacts briefly render to User B.

## Suggested approach

1. In `frontend/apps/ppt-web/src/lib/queryKeys.ts:347` add `'accounting',` to `AUTHED_QUERY_KEY_ROOTS` (alongside the other 18 roots). Place it next to the other feature-area roots so the list stays scannable.
2. Add a regression test at `frontend/apps/ppt-web/src/lib/queryKeys.test.ts` (or extend the existing one if present) asserting that every key returned by the `queryKeys` factory has its top-level root included in `AUTHED_QUERY_KEY_ROOTS`. This prevents the same leak from re-emerging when a future feature adds a new factory root.
3. Add a unit test for `AuthContext.logout` that seeds the cache with `['accounting', 'invoices']` and `['accounting', 'contacts']`, calls `logout()`, then asserts `queryClient.getQueryCache().findAll({ queryKey: ['accounting'] })` is empty.
4. Run `pnpm --filter @ppt/web typecheck && pnpm --filter @ppt/web test` to confirm green.

## Alternatives considered

- **Replace per-root `removeQueries` with `queryClient.clear()` in logout** — rejected because Issue #712 (referenced in the file's doc-comment) explicitly removed `queryClient.clear()` for being too aggressive (it nukes unauth-scoped caches like translations + static config). Per-root removal is the chosen contract.
- **Lint rule that forbids declaring a `queryKeys.<foo>` factory without adding `<foo>` to `AUTHED_QUERY_KEY_ROOTS`** — rejected as out of scope for this fix; the regression-test approach in step 2 catches the same class of bug with less tooling investment, and a future tightening can land as a separate plan.

## Root-cause trace

1. Symptom: User A's accounting data visible to User B after logout/login on same device.
2. ← `AuthContext.logout` iterates `AUTHED_QUERY_KEY_ROOTS` and calls `queryClient.removeQueries({ queryKey: [root] })` per root (frontend/apps/ppt-web/src/contexts/AuthContext.tsx).
3. ← `AUTHED_QUERY_KEY_ROOTS` at `frontend/apps/ppt-web/src/lib/queryKeys.ts:347` is missing `'accounting'` — so the loop never targets the accounting cache subtree.
4. Origin: the `queryKeys.accounting` factory (lines 142–150) was added by PAP-232 (native accounting MVP) without the matching entry in `AUTHED_QUERY_KEY_ROOTS`. The file's doc-comment was already in place but the contract was not enforced by a test.

## Test plan

- [ ] Unit test in `frontend/apps/ppt-web/src/lib/queryKeys.test.ts` asserting `AUTHED_QUERY_KEY_ROOTS` contains the root of every key returned by `queryKeys.<area>.<fn>()` (drives correctness for future factory additions).
- [ ] Unit/integration test of `AuthContext.logout`: seed `queryClient` with two `['accounting', …]` entries, call `logout()`, assert `queryClient.getQueryCache().findAll({ queryKey: ['accounting'] }).length === 0`.
- [ ] Command: `pnpm --filter @ppt/web test -- queryKeys` plus `pnpm --filter @ppt/web test -- AuthContext`.

## Out of scope

- Auditing other ppt-web feature areas for the same omission via a one-off scan — the new contract-test in step 2 covers all of them at once.
- Migrating to a tag-based cache invalidation scheme.
- Mirror fix in `frontend/apps/reality-web/` or `frontend/apps/mobile/` (track separately if those apps have an equivalent `AUTHED_QUERY_KEY_ROOTS` pattern).

## After-merge

- Move this file to `plans/_archive/code-review-ppt-web-core-querykeys-accounting-leak.md`
- Mark the matching `backlog.json` row as `status: "done"`
