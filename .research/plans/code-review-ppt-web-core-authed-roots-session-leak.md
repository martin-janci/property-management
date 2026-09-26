# code-review-ppt-web-core-authed-roots-session-leak

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review (dispatcher Tier-1d 2026-09-26 ppt-web-core query-cache)
**Confidence:** high

## Hypothesis
`AUTHED_QUERY_KEY_ROOTS` in `frontend/apps/ppt-web/src/lib/queryKeys.ts` is the allow-list `logout()` iterates to purge per-user TanStack Query cache. Multiple authed roots for mounted ppt-web route groups (`community`, `disputes`, `iot`, `outages`, `leases`, `voting`) are missing, and the list has a naming drift (`'votes'` vs the actual live factory root `'voting'`). On a shared workstation, User A's tenant-/user-scoped cache for those roots survives `logout()` and is served to User B until each query happens to refetch — the exact cross-session data-leak class this list was created to prevent (issue #712). The smallest safe change: extend the allow-list to cover every mounted authed root, and add a Vitest guard that asserts every imported key-factory `all` root appears in `AUTHED_QUERY_KEY_ROOTS` so silent drift can't reappear.

## Evidence
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:354-392` — current allow-list; docstring on lines 346-350 states it "must cover ALL tenant-/user-scoped caches" and reference issue #712.
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:614-637` — `logout()` prefix-purges only roots in the allow-list via `queryClient.removeQueries({ queryKey: [root] })`.
- Mounted route groups whose caches are NOT purged: `frontend/apps/ppt-web/src/routes/groups/{community,disputes,iot,leases,voting}.tsx` (verified on disk).
- Drift example: allow-list has `'votes'` (line 360) but the live factory root used by `/voting` pages is `['voting']` — the entry looks like it covers voting but does not.
- Prior partial fix: PR #2650 addressed one missed root (`notification-triggers`, line 391) but the systemic drift persisted.

## Files
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:354`
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:614`
- `frontend/apps/ppt-web/src/routes/groups/community.tsx`
- `frontend/apps/ppt-web/src/routes/groups/disputes.tsx`
- `frontend/apps/ppt-web/src/routes/groups/iot.tsx`
- `frontend/apps/ppt-web/src/routes/groups/leases.tsx`
- `frontend/apps/ppt-web/src/routes/groups/voting.tsx`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. In a browser with two tenant accounts on the same origin (or two users of the same tenant), sign in as User A.
2. Navigate to `/community`, `/disputes`, `/leases`, or `/voting` and populate the TanStack Query cache (e.g. open a mediation thread, a lease, a poll).
3. Click Logout — `logout()` purges only the current `AUTHED_QUERY_KEY_ROOTS`, leaving those routes' caches in place.
4. Sign in as User B on the same browser.
5. Navigate immediately to the same route: expected — empty state / fresh fetch. Actual — cached results from User A's session are rendered until the individual queries refetch.

## Suggested approach
1. Enumerate every key-factory root actually imported into `ppt-web` (grep for `all: [` in `frontend/packages/api-client/src/**/hooks.ts` and `frontend/apps/ppt-web/src/**` — the query-key factories all define `all: ['<root>']` as their first entry).
2. Extend `AUTHED_QUERY_KEY_ROOTS` in `queryKeys.ts:354` to cover the missing mounted roots (`community`, `disputes`, `iot`, `outages`, `leases`, `voting`) and correct the `votes` → `voting` drift (keep `votes` if any legacy hook still uses it; add `voting` regardless).
3. Add a Vitest guard test at `frontend/apps/ppt-web/src/lib/queryKeys.test.ts` (new file — mirror the pattern of nearby `*.test.tsx` files in `apps/ppt-web/src/lib/`). Import every mounted key-factory's `all` root and assert each string appears in `AUTHED_QUERY_KEY_ROOTS`. The test fails today for the missing roots (this is the IG3 regression test).
4. Add a Vitest scenario for `logout()` that (a) seeds queries under each authed root via `queryClient.setQueryData`, (b) invokes `logout()`, (c) asserts `queryClient.getQueryCache().findAll({ queryKey: [root] })` is empty for every root in the list.
5. Verify locally: `pnpm --filter @ppt/web typecheck && pnpm --filter @ppt/web test`.

## Alternatives considered
- **Revert to `queryClient.clear()`** — rejected because #712 explicitly replaced the blanket clear (it also nukes public/unauthenticated caches, causing avoidable refetches for logged-out surfaces) and this change would regress that intent.
- **Namespace all authed queries under a single `['authed', …]` root** — rejected because it requires touching every hook in `@ppt/api-client` and every feature-local hook in `ppt-web` (dozens of call sites) whereas the allow-list fix is one file plus a guard test.

## Root-cause trace
1. Symptom: after logout, `/community`, `/disputes`, `/iot`, `/leases`, `/voting` render User A's data to User B until per-query refetch.
2. ← `logout()` at `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:624-626` only calls `queryClient.removeQueries({ queryKey: [root] })` for roots in `AUTHED_QUERY_KEY_ROOTS`.
3. ← `AUTHED_QUERY_KEY_ROOTS` at `frontend/apps/ppt-web/src/lib/queryKeys.ts:354-392` omits the roots for those mounted route groups; contains `'votes'` where the live factory root is `'voting'`.
4. Origin: the list was hand-maintained (docstring says "add it here too"). Each new mounted route group added a new authed root without a corresponding allow-list update; PR #2650 fixed one instance (`notification-triggers`) but did not address the systemic drift.

## Test plan
- [ ] New Vitest guard at `frontend/apps/ppt-web/src/lib/queryKeys.test.ts` — asserts every imported key-factory `all` root is present in `AUTHED_QUERY_KEY_ROOTS`. Fails on `dev` before the fix.
- [ ] New Vitest scenario in the same file — seeds queries for each authed root, calls `logout()` (or the underlying purge helper), asserts `getQueryCache().findAll({ queryKey: [root] })` returns `[]` for every root.
- [ ] `pnpm --filter @ppt/web typecheck` — no new type errors.
- [ ] `pnpm --filter @ppt/web test frontend/apps/ppt-web/src/lib/queryKeys.test.ts` — both new tests pass at HEAD after the fix.

## Out of scope
- Renaming or namespacing existing query-key factory roots (e.g. moving everything under `['authed', …]`) — that is the alternative rejected above and requires broad refactor.
- Server-side session invalidation or refresh-token rotation (issue #2650 covered that path; this plan is client-cache purging only).
- Auditing `admin-web` or `reality-web` — this finding is scoped to `ppt-web`; the sibling apps use their own logout paths.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-authed-roots-session-leak.md`
- Mark the matching `backlog.json` row as `status: "done"`
