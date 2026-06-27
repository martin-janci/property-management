# security-accounting-query-cache-leak-on-logout

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review 2026-06-27 (Phase 1.5 follow-up, ppt-web-core segment); fire-payload buffer-low trigger
**Confidence:** high

## Hypothesis
The `accounting` query subtree (invoices, contacts, statements, statement lines, line matches, payments) is registered in `queryKeys.accounting` and used by `features/accounting/**` pages but is **not** listed in `AUTHED_QUERY_KEY_ROOTS`. On logout, `AuthContext.logout()` iterates that list and calls `queryClient.removeQueries({ queryKey: [root] })` per root — accounting cache survives the purge. If the next user signs in on the same tab before the GC sweep, their accounting pages render the previous user's cached data on first paint. This is the exact bug class issue #712 was meant to prevent — adding `'accounting'` to the array closes it. One-line fix, narrow blast radius, mechanical regression test (logout → next-user fetch must MISS the cache).

## Evidence
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:347-368` — `AUTHED_QUERY_KEY_ROOTS` does NOT contain `'accounting'`. Roots listed: `announcements`, `faults`, `documents`, `votes`, `messages`, `neighbors`, `forms`, `person-months`, `self-readings`, `user`, `buildings`, `notifications`, `developer`, `ocr`, `actionQueue`, `executionLogs`, `executionStats`, `ai-chat`.
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:140-148` — `queryKeys.accounting = { all: ['accounting'], invoices, contacts, statements, statementLines, … }` — first segment is `'accounting'`, which is the prefix the logout sweep targets.
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:540-543` — `for (const root of AUTHED_QUERY_KEY_ROOTS) { queryClient.removeQueries({ queryKey: [root] }); }` — prefix-match purge by root; anything not in the list survives.
- Active usage (proving the cache actually holds data): `frontend/apps/ppt-web/src/features/accounting/pages/AccountingInvoiceManagementPage.tsx:25,30` and `features/accounting/pages/PaymentMatchingPage.tsx:38,43,49` both call `useQuery({ queryKey: queryKeys.accounting.* })`.
- Module-level JSDoc on `queryKeys.ts:344` explicitly warns: *"When you add a new auth-scoped query root, add it here too — otherwise the cached data will leak into the next user's session. @see Issue #712 — logout `queryClient.clear()` was too aggressive"*. The accounting addition (PAP-232) violated this contract.

## Files
- `frontend/apps/ppt-web/src/lib/queryKeys.ts`
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx`

## Dependencies
<!-- No prior task_ids block this fix. -->

## Required capabilities
- [x] C1 — Systematic debugging (security vector; trace the missing entry)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Mode: cloud-ok**

## Repro steps
1. Open the app, log in as user A (tenant α). Navigate to the accounting invoices list; let it populate the React-Query cache for keys prefixed by `['accounting', 'invoices']`.
2. Log out (the logout path runs `AUTHED_QUERY_KEY_ROOTS` purge).
3. Log in as user B (tenant β) in the same browser tab. Navigate to the accounting invoices list immediately.
4. **Expected:** user B sees a loading state followed by user B's tenant data (cache MISS, fresh fetch).
5. **Actual:** user B sees user A's invoice list on first paint until the new fetch resolves and overwrites it (cache HIT on the stale entry). Confirm via React Query DevTools — the `['accounting', 'invoices', …]` entries are still present after step 2.

## Suggested approach
1. In `frontend/apps/ppt-web/src/lib/queryKeys.ts:347-368`, append `'accounting'` to the `AUTHED_QUERY_KEY_ROOTS` array. Keep the list sorted by feature affinity (it currently groups factory roots first, then ad-hoc roots — drop `'accounting'` in with the other factory roots between `'forms'` and `'person-months'`, or wherever maintains the existing pattern).
2. Add a brief inline comment next to the new line — e.g. `// accounting (PAP-232)` — so the next contributor sees the cross-reference.
3. Run the existing test file for `queryKeys.ts` if one exists; if not, add a focused unit test in `frontend/apps/ppt-web/src/lib/__tests__/queryKeys.test.ts` (or sibling) that asserts every key in `queryKeys` whose `all[0]` is a string appears in `AUTHED_QUERY_KEY_ROOTS`. This prevents the next feature from re-introducing the same gap.
4. Add an integration-level test in `frontend/apps/ppt-web/src/contexts/__tests__/AuthContext.test.tsx` (or equivalent existing logout test) that seeds the accounting cache, runs `logout()`, and asserts `queryClient.getQueryData(queryKeys.accounting.invoices())` returns `undefined`.
5. If issue #712 has a tracking thread, reference it in the commit message and close it if outstanding.

## Alternatives considered
- **Replace the per-root loop with `queryClient.clear()`** — rejected: that's exactly what #712 reverted because it nuked legitimately public/anonymous caches (translations, route metadata) and forced cold reloads on every login switch. The allowlist exists by design.
- **Auto-derive `AUTHED_QUERY_KEY_ROOTS` from `Object.keys(queryKeys)`** — rejected at this scope: there are ad-hoc roots used directly (e.g. `'developer'`, `'ocr'`, `'ai-chat'`) that bypass the factory; auto-derive would either miss them or require a second registry. The lint-style unit test in step 3 catches drift at the right cost.

## Root-cause trace
1. Symptom: previous user's accounting invoice list flashes on screen when a second user logs in on the same tab; React Query DevTools shows `['accounting', 'invoices', …]` entries surviving logout.
2. ← `AuthContext.tsx:540-543` logout sweep iterates `AUTHED_QUERY_KEY_ROOTS` and only purges those prefixes; entries whose first key segment is not in the list are not touched.
3. ← `lib/queryKeys.ts:347-368` `AUTHED_QUERY_KEY_ROOTS` literal omits `'accounting'`.
4. Origin: PAP-232 (native accounting MVP) added `queryKeys.accounting` at `lib/queryKeys.ts:140` without updating the allowlist at `:347` — the module-level JSDoc explicitly required this. Issue #712 already documented the same hazard at logout-time.

## Test plan
- [ ] Unit test in `frontend/apps/ppt-web/src/lib/queryKeys.test.ts` (new or existing) — assert every top-level key in `queryKeys` whose `.all` starts with a string is present in `AUTHED_QUERY_KEY_ROOTS`. Today this fails (accounting missing); after the fix, it passes; it also guards against future regressions.
- [ ] Integration test in `frontend/apps/ppt-web/src/contexts/AuthContext.test.tsx` — seed `queryClient.setQueryData(queryKeys.accounting.invoices(), [{...}])`, call `logout()`, assert `queryClient.getQueryData(queryKeys.accounting.invoices()) === undefined`.
- [ ] Run locally: `pnpm -F @ppt/web test queryKeys` and `pnpm -F @ppt/web test AuthContext`. Both must FAIL on `main`, PASS after the patch.

## Out of scope
- Refactoring `AUTHED_QUERY_KEY_ROOTS` into an auto-derived registry (covered as a follow-up if churn shows this gap recurs).
- Auditing other recently-added query roots for the same omission (covered by the lint-style unit test in the *Test plan*).
- The WebSocket-refresh-token bug also surfaced in the same review (`code-review-ppt-web-core-ws-refreshed-token-not-applied`) — separate plan when it scores up.

## After-merge
- Move this file to `plans/_archive/security-accounting-query-cache-leak-on-logout.md`
- Mark `code-review-ppt-web-core-accounting-query-leak-on-logout` in `backlog.json` as `status: "done"`
