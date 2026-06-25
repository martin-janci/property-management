# code-review-ppt-web-core-accounting-query-leak

**Vector:** security
**Score:** 3
**Source:** Phase 1.5 code-review (segment `ppt-web-core`, 2026-06-25)
**Confidence:** high

## Hypothesis
`AUTHED_QUERY_KEY_ROOTS` in `lib/queryKeys.ts` enumerates the TanStack Query root keys that `AuthContext.logout()` clears from the cache, but the recently-added `accounting` root key is **missing** from the list. As a result, when a manager logs out, their cached invoices, payment statements, and other accounting data remain in the in-memory query cache and are immediately visible to the next user who logs in on the same browser tab (shared device, kiosk, demo machine). Adding `'accounting'` to the array closes the leak; a regression test should pin the contract going forward.

## Evidence
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:142-151` — defines `queryKeys.accounting` (the root that the accounting feature subscribes to).
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:347-368` — defines `AUTHED_QUERY_KEY_ROOTS`; `'accounting'` is **not** in the list.
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:542` — `logout()` calls `queryClient.removeQueries` only for the roots in `AUTHED_QUERY_KEY_ROOTS`, so anything outside that list survives logout.
- Discovered by Phase 1.5 rotating expert review on the `ppt-web-core` segment, 2026-06-25; confidence=high (file + line cited, mechanism observed in code).

## Files
- `frontend/apps/ppt-web/src/lib/queryKeys.ts`
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx`
- `frontend/apps/ppt-web/src/contexts/AuthContext.test.tsx`

## Dependencies
<!-- no upstream dependencies; the fix is isolated to the web app -->

## Required capabilities
- [x] C1 — Systematic debugging (security bug, need to confirm cache state transitions)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser  · local-only
- [ ] C5 — ADB device  · local-only
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception (security-tagged change)

**Mode: cloud-ok** — the regression test runs under Vitest in jsdom, no real browser needed.

## Repro steps
1. Run the ppt-web app locally; sign in as a manager whose org has the accounting feature enabled.
2. Navigate to `/accounting/invoices` (or any accounting route). Confirm invoices populate; React Query has now cached `['accounting', 'invoices', ...]` entries.
3. Click "Sign out". `AuthContext.logout()` runs and calls `queryClient.removeQueries` for each root in `AUTHED_QUERY_KEY_ROOTS`.
4. Open DevTools → React Query devtools (or `queryClient.getQueryCache().getAll()`); accounting entries are still present.
5. Sign in as a different user (different tenant). Navigate to `/accounting/invoices`. The cached invoices from step 2 flash briefly before the new fetch resolves — **data leakage between sessions**. *Expected:* no cached accounting data after logout. *Actual:* cached data survives.

## Suggested approach
1. In `frontend/apps/ppt-web/src/lib/queryKeys.ts`, add the string `'accounting'` to the `AUTHED_QUERY_KEY_ROOTS` array (line ~347-368). Keep alphabetical ordering if the existing list maintains it; otherwise append.
2. In `frontend/apps/ppt-web/src/contexts/AuthContext.test.tsx`, add a test that:
   - Mounts the provider, prefills the query cache with `queryClient.setQueryData(['accounting', 'invoices'], <fixture>)`.
   - Calls the exposed `logout()` action.
   - Asserts `queryClient.getQueryData(['accounting', 'invoices']) === undefined` after the call.
3. Optional but recommended: convert `AUTHED_QUERY_KEY_ROOTS` into a derivation from `Object.keys(queryKeys)` (excluding intentionally-public roots) so future feature additions can't reintroduce the gap. If that's larger than a one-liner, file as a separate refactor and keep this PR to the array entry + test.

## Alternatives considered
- **Clear *all* queries on logout** (`queryClient.clear()`) — rejected because the existing design intentionally preserves some public roots (e.g. config, feature flags) across the sign-out boundary; switching to a blanket clear regresses those non-auth caches and is a larger behavioural change than this bug needs.
- **Move accounting queries to a separate `QueryClient` instance scoped to authenticated sessions** — rejected because the rest of the auth-scoped data already lives on the same shared client and uses the allowlist mechanism; introducing a second client just to fix one missing entry creates more surface area than it removes.

## Root-cause trace
1. Symptom: cached accounting query results survive logout and are visible to the next signed-in user on the same tab.
2. ← `AuthContext.logout()` (`frontend/apps/ppt-web/src/contexts/AuthContext.tsx:542`) iterates only over `AUTHED_QUERY_KEY_ROOTS` when removing cached queries.
3. ← `AUTHED_QUERY_KEY_ROOTS` (`frontend/apps/ppt-web/src/lib/queryKeys.ts:347-368`) is a hand-maintained allowlist that does not include `'accounting'`.
4. Origin: the `accounting` root key was added to `queryKeys` (line 142) when the accounting feature shipped, but the allowlist was never updated alongside it. The omission is a maintenance gap, not a design choice — every other auth-scoped root key in `queryKeys` IS in the allowlist.

## Test plan
- [ ] New test in `frontend/apps/ppt-web/src/contexts/AuthContext.test.tsx`: prefill `queryClient` with an accounting query, call `logout()`, assert the cache entry is gone. This test fails on `main` today (proves the bug exists) and passes after the one-line allowlist fix.
- [ ] Run the existing test suite: `pnpm --filter ppt-web test contexts/AuthContext` (must stay green).
- [ ] Manual smoke: log in as user A, view `/accounting/invoices`, sign out, log in as user B, observe no flash of A's invoices.

## Out of scope
- Auditing other query roots for similar omissions (a follow-up `dx`/`refactor` plan can cover deriving the allowlist from `queryKeys`).
- Any backend or RLS work; this is purely a client-side cache hygiene fix.
- Cross-tab logout propagation (`storage` event listener) — orthogonal concern.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-accounting-query-leak.md`
- Mark the matching `backlog.json` row as `status: "done"`
