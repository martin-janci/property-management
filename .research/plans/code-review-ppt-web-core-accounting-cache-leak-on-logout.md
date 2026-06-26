# code-review-ppt-web-core-accounting-cache-leak-on-logout

**Vector:** security
**Score:** 3
**Source:** Phase 1.5 code review (ppt-web-core, frontend lens) on 2026-06-26
**Confidence:** high

## Hypothesis

`AUTHED_QUERY_KEY_ROOTS` in `frontend/apps/ppt-web/src/lib/queryKeys.ts` (lines 347–368) omits the `'accounting'` root even though `queryKeys.accounting.all = ['accounting']` (line 143) anchors the entire accounting subtree (invoices, contacts, statements, statement lines, payment-match results). `AuthContext.logout()` (AuthContext.tsx:542) iterates that list and calls `queryClient.removeQueries({ queryKey: [root] })` on each — `'accounting'` is silently skipped, so every cached statement/invoice/payment-match survives logout and is visible to the next user in the same browser session. This is the same cross-tenant cache-leak class issue #712 was filed to prevent; the existing test at `AuthContext.test.tsx:188` enumerates `['user','faults','announcements','ai-chat','developer']` for the sanity-loop and never touches accounting, which is why the regression went unnoticed when the native accounting MVP (#1453/#1454) landed.

## Evidence

- `frontend/apps/ppt-web/src/lib/queryKeys.ts:347-368` — `AUTHED_QUERY_KEY_ROOTS` literal: `'announcements','faults','documents','votes','messages','neighbors','forms','person-months','self-readings','user','buildings','notifications','developer','ocr','actionQueue','executionLogs','executionStats','ai-chat'`. `'accounting'` is absent.
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:142-151` — `queryKeys.accounting.all = ['accounting']` is the first segment used by `accounting.invoices()`, `accounting.contacts()`, `accounting.statements()`, `accounting.statementLines(id)`, `accounting.lineMatches(id)`.
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:342-344` — file header comment: *"When you add a new auth-scoped query root, add it here too — otherwise the cached data will leak into the next user's session."*
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:532-555` — `logout()` purge loop only touches `AUTHED_QUERY_KEY_ROOTS` entries.
- `frontend/apps/ppt-web/src/contexts/AuthContext.test.tsx:185-244` — existing logout cache-purge test never seeds or asserts `accounting`; the sanity-loop at line 188 enumerates 5 roots and accounting is not one of them.
- Issue #712 — historical context for why blanket `queryClient.clear()` was replaced with this list; that regression-prevention contract is what's currently broken.

## Files

- `frontend/apps/ppt-web/src/lib/queryKeys.ts:347`
- `frontend/apps/ppt-web/src/contexts/AuthContext.test.tsx:185`

## Dependencies

(none)

## Required capabilities

- [x] C1 — Systematic debugging (security/cross-tenant leak — trace cache lifecycle)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps

1. Run `pnpm -F @ppt/web test src/contexts/AuthContext.test.tsx` against `dev` HEAD with `AUTHED_QUERY_KEY_ROOTS` untouched — the existing "removes auth-scoped subtrees" test passes (because it doesn't exercise accounting).
2. Add the failing assertion below to `AuthContext.test.tsx` and re-run:
   ```ts
   queryClient.setQueryData(['accounting', 'invoices'], [{ id: 'inv-1', amount: 12345 }]);
   queryClient.setQueryData(['accounting', 'statements', 's-1', 'lines'], [{ id: 'l-1' }]);
   // … later, after logout …
   expect(queryClient.getQueryData(['accounting', 'invoices'])).toBeUndefined();
   expect(queryClient.getQueryData(['accounting', 'statements', 's-1', 'lines'])).toBeUndefined();
   ```
3. Expected: both `getQueryData` calls return `undefined` (cache cleared). Actual on `dev`: both return the seeded objects — the cross-tenant accounting data survives logout.

## Suggested approach

1. **One-line fix** in `frontend/apps/ppt-web/src/lib/queryKeys.ts:347-368`: add `'accounting',` to `AUTHED_QUERY_KEY_ROOTS` (alphabetically near the top, alongside `'announcements'`, to match the comment grouping "queryKeys factory roots").
2. **Extend the failing-on-main regression test** in `frontend/apps/ppt-web/src/contexts/AuthContext.test.tsx:188`: add `'accounting'` to the sanity-loop list (so future drops fail loudly), and add the two `setQueryData` seeds + two `toBeUndefined` assertions from step 2 of Repro to the "removes auth-scoped subtrees" test body (around lines 198–211 and 226–230).
3. **Audit for sibling drift**: grep the codebase for any other factory root added to `queryKeys` since the last list update — `grep -rn "queryKey:\s*\[\s*['\"][a-z][a-z0-9-]*['\"]" frontend/apps/ppt-web/src` and cross-check first-segment literals against `AUTHED_QUERY_KEY_ROOTS`. If anything else is missing (e.g. a feature added in the same window), add it in the same PR — but stay disciplined and don't bundle unrelated cleanups.
4. **Do not** convert the list to a generated/derived array — the explicit list is the load-bearing contract that catches new-root drift in code review. A computed list would silently re-introduce the same class of bug.
5. Run `pnpm -F @ppt/web typecheck && pnpm -F @ppt/web test src/contexts` locally; CI will run the full `frontend.yml` workflow.
6. PR title: `fix(ppt-web): clear accounting query cache on logout (cross-tenant leak)`. Reference issue #712 in the body for historical context.

## Alternatives considered

- **Revert to `queryClient.clear()`** — rejected because that's exactly what issue #712 documented as too aggressive (wipes router/internal caches that legitimately survive logout). The explicit-list contract is the right shape; the bug is a missed update, not a wrong design.
- **Derive the list from `queryKeys` object keys at runtime** — rejected because (a) ad-hoc roots used directly in feature hooks (`developer`, `ocr`, `actionQueue`, `executionLogs`, `executionStats`, `ai-chat`) aren't in the `queryKeys` factory and would be silently dropped, and (b) a derived list trades a one-line maintenance burden for a class of cache-drift bugs that's much harder to spot in code review.

## Root-cause trace

1. Symptom: after User A logs out and User B logs in on the same browser, User B sees a flash of (or a stale-render of) User A's invoices/statements/payment-match data from the TanStack Query cache.
2. ← `AuthContext.logout()` at `AuthContext.tsx:542-544` iterates `AUTHED_QUERY_KEY_ROOTS` and calls `queryClient.removeQueries({ queryKey: [root] })`. Because `'accounting'` is not in the list, the accounting subtree isn't removed.
3. ← `AUTHED_QUERY_KEY_ROOTS` was last updated when prior factory roots were added (faults/announcements/messages/etc.). The native accounting MVP (PR #1453 N5 + #1454 N1–N4) introduced `queryKeys.accounting.*` at `queryKeys.ts:142-151` but did not extend `AUTHED_QUERY_KEY_ROOTS`. The existing logout test at `AuthContext.test.tsx:188` enumerates 5 roots and never touched accounting, so CI was silent.
4. Origin: PR #1454 "feat: native accounting MVP (N1-N4) + @hey-api migration" (merged in the 2026-06-16..2026-06-26 window) — added the accounting `queryKeys` subtree without the matching `AUTHED_QUERY_KEY_ROOTS` entry. The companion PR #1453 (N5 — bank statement upload + payment matching) carried `accounting.lineMatches` keys through the same gap.

## Test plan

- [ ] `frontend/apps/ppt-web/src/contexts/AuthContext.test.tsx` — add `'accounting'` to the sanity-loop list (line 188 area) AND add `setQueryData`/`getQueryData` pair for `['accounting','invoices']` + `['accounting','statements','s-1','lines']` (lines 198–211 seed + 226–230 asserts). The test must fail on `dev` HEAD before the queryKeys.ts change, and pass after.
- [ ] Regression: confirm the existing assertion at line 188 (`expect(AUTHED_QUERY_KEY_ROOTS).toContain(root)`) loop still passes for `accounting` after the queryKeys.ts edit.
- [ ] Run command: `pnpm -F @ppt/web test src/contexts/AuthContext.test.tsx` (Vitest 4). Expect green after the fix. Then `pnpm -F @ppt/web typecheck` to confirm no type regressions from the list edit.

## Out of scope

- Server-side cache / session invalidation (handled separately by `tokenStorage.clear()` + `authApi.logout()` at AuthContext.tsx:546-554).
- A broader audit of all `AUTHED_QUERY_KEY_ROOTS` entries vs `queryKeys.*` factory (worthwhile but separate refactor; this PR fixes the specific leak that's live in production).
- Changing the contract design from explicit-list to derived (see *Alternatives considered*).
- Mobile app's `frontend/apps/mobile/` cache-purge audit — different surface (no AuthContext.tsx parallel yet), separate plan if needed.

## After-merge

- Move this file to `plans/_archive/code-review-ppt-web-core-accounting-cache-leak-on-logout.md`
- Mark the matching `backlog.json` row as `status: "done"`
