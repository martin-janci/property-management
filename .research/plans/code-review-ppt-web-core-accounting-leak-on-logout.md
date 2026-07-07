# code-review-ppt-web-core-accounting-leak-on-logout

**Vector:** security
**Score:** 3
**Source:** code review (ppt-web-core segment, 2026-07-07); hotspot in `frontend/apps/ppt-web/src/lib/queryKeys.ts`
**Confidence:** medium

## Hypothesis
`AUTHED_QUERY_KEY_ROOTS` (queryKeys.ts:347-368) enumerates the TanStack Query cache roots that `AuthContext.logout()` (AuthContext.tsx:532-544) clears on sign-out, but the `'accounting'` root is missing from the list even though `queryKeys.accounting.*` (invoices, contacts, statements, statementLines, lineMatches — queryKeys.ts:142-151) is keyed under `['accounting', ...]`. On a shared browser, after User A logs out and User B logs in, User B's session inherits User A's cached accounting subtree (invoices, contacts, bank statements, payment-line matches) until it is manually invalidated or refetched. The exact class of cross-session leak that PR #712 introduced this pattern to prevent — accounting was simply forgotten when the module landed later. The smallest change is a one-line insertion into `AUTHED_QUERY_KEY_ROOTS` plus a regression test that asserts the sentinel is present.

## Evidence
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:347-368` — `AUTHED_QUERY_KEY_ROOTS` contains 20 entries; `'accounting'` is absent while every other feature root (announcements, faults, documents, votes, messages, neighbors, forms, person-months, self-readings, user, buildings, notifications) is present.
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:140-151` — `queryKeys.accounting.all = ['accounting'] as const`; the five accounting sub-keys (`invoices`, `contacts`, `statements`, `statementLines`, `lineMatches`) all extend this root.
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:532-544` — `logout()` iterates `AUTHED_QUERY_KEY_ROOTS` and calls `queryClient.removeQueries({ queryKey: [root] })` for each; anything not in the list survives.
- `frontend/apps/ppt-web/src/contexts/AuthContext.test.tsx:188` (per Phase 1.5 review) — the sanity list asserting AUTHED_QUERY_KEY_ROOTS contents does not include `'accounting'`, so the omission is not caught by the current test.

## Files
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:347-368`
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:532-544`
- `frontend/apps/ppt-web/src/contexts/AuthContext.test.tsx`

## Dependencies
(none)

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
1. Start ppt-web dev server; log in as User A with an org membership that has accounting access.
2. Navigate to the Accounting section so `queryKeys.accounting.invoices()` and `queryKeys.accounting.contacts()` queries populate the TanStack Query cache.
3. Log out via the account menu (drives `AuthContext.logout()`).
4. In the browser devtools, inspect the query cache: entries keyed under `['accounting', 'invoices']`, `['accounting', 'contacts']`, etc. are still present.
5. Log in as User B (different org / different accounting role).
6. Expected: the accounting screen shows a loading state and re-fetches for User B's org. Actual: the previously-cached User A subtree is returned by the cache before User B's fetch resolves, briefly (or persistently, for cached-only queries) exposing User A's accounting data to User B.

## Suggested approach
1. Edit `frontend/apps/ppt-web/src/lib/queryKeys.ts:347-368` — add `'accounting',` to `AUTHED_QUERY_KEY_ROOTS`, grouped with the other queryKeys-factory roots (before the "Ad-hoc roots" comment).
2. Add an assertion to `frontend/apps/ppt-web/src/contexts/AuthContext.test.tsx` (near the existing AUTHED_QUERY_KEY_ROOTS sanity list around line 188) that fails when a queryKeys-factory root is missing: iterate the `queryKeys` factory keys and assert every root produced by `queryKeys.<feature>.all` appears in `AUTHED_QUERY_KEY_ROOTS`. This turns the class of bug into a compile-time-adjacent guard so future feature roots can't ship without the logout cleanup.
3. Add a regression test that seeds the cache with an `['accounting', 'invoices']` entry, calls `logout()`, and asserts `queryClient.getQueryData(['accounting', 'invoices'])` is `undefined`.
4. Run `pnpm -F @ppt/ppt-web test src/contexts/AuthContext.test.tsx` — the sanity test must fail before step 1 lands and pass after.
5. `pnpm -F @ppt/ppt-web typecheck` and `pnpm -F @ppt/ppt-web test` for the full ppt-web target.
6. Verify the fix by driving the repro-steps flow above manually against a dev server if time allows (optional; the unit test is the load-bearing assertion).

## Alternatives considered
- **Blanket `queryClient.clear()` on logout** — rejected because it wipes anonymous public caches (e.g. reality-web listings previews) and non-user caches, and it's what PR #712 explicitly avoided; the enumerated-roots pattern is the intentional design.
- **Auto-derive `AUTHED_QUERY_KEY_ROOTS` from the `queryKeys` factory shape** — rejected because ad-hoc roots (`developer`, `ocr`, `actionQueue`, `executionLogs`, `executionStats`, `ai-chat`) are declared inline in feature hooks, not through the factory; a full auto-derivation would require refactoring those hooks first, which is out of scope for this fix. The unit-test guard proposed in step 2 gives most of the safety at a fraction of the churn.

## Root-cause trace
1. Symptom: cross-user accounting cache leak on shared-browser logout/login.
2. ← `AuthContext.tsx:542` loops `AUTHED_QUERY_KEY_ROOTS` and only removes those subtrees on logout.
3. ← `queryKeys.ts:347-368` list omits `'accounting'` even though the `queryKeys.accounting` factory (queryKeys.ts:142-151) exists and is used by accounting feature hooks.
4. Origin: the native accounting MVP (PAP-232 comment at queryKeys.ts:140) landed after PR #712 introduced the enumerated-roots pattern; the follow-up to add `'accounting'` to the sentinel list was missed.

## Test plan
- [ ] `frontend/apps/ppt-web/src/contexts/AuthContext.test.tsx` — assertion that every `queryKeys.<feature>.all[0]` appears in `AUTHED_QUERY_KEY_ROOTS`; fails on `main`, passes after fix.
- [ ] `frontend/apps/ppt-web/src/contexts/AuthContext.test.tsx` — regression: seed `queryClient.setQueryData(['accounting', 'invoices'], [...])`, call `logout()`, expect `queryClient.getQueryData(['accounting', 'invoices'])` to be `undefined`.
- [ ] `pnpm -F @ppt/ppt-web test` and `pnpm -F @ppt/ppt-web typecheck` — both exit 0.

## Out of scope
- Refactoring feature hooks to route the ad-hoc roots (`developer`, `ocr`, `actionQueue`, `executionLogs`, `executionStats`, `ai-chat`) through the `queryKeys` factory. Track separately if the auto-derivation alternative becomes attractive.
- Auditing other apps (admin-web, reality-web, mobile RN) for the same pattern. Their auth flows are decoupled; a separate segment review can pick that up.
- Any change to the accounting feature itself.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-accounting-leak-on-logout.md`
- Mark the matching `backlog.json` row as `status: "done"`
