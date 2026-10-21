# code-review-ppt-web-core-authed-roots-session-leak

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review (dispatcher Tier-1d 2026-09-26 ppt-web-core query-cache) — signal `code-review-ppt-web-core-authed-roots-session-leak`
**Confidence:** high

## Hypothesis
`AUTHED_QUERY_KEY_ROOTS` in `frontend/apps/ppt-web/src/lib/queryKeys.ts` is the allow-list that `logout()` iterates to purge per-user TanStack Query caches on a shared workstation (Issue #712 fix). Multiple mounted `ppt-web` route groups populate caches under roots that are absent from this list — most notably `community`, `disputes`, `iot`, `outages`, `leases`, plus a live drift where the list carries `votes` but the actual factory root is `voting`. On logout, prefix-match `queryClient.removeQueries({ queryKey: [root] })` skips these caches; the next user on the same browser is served the previous user's tenant/user-scoped data until each query happens to refetch. Extending the allow-list (and correcting `votes` → `voting`) plus a guard test that keeps it in sync with the imported factory roots fixes the leak without behavioural risk.

## Evidence
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:354-392` — the current `AUTHED_QUERY_KEY_ROOTS` array; docstring at :346-350 explicitly says "must cover ALL tenant-/user-scoped caches" and cites Issue #712.
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:614-637` — `logout()` iterates the allow-list and calls `queryClient.removeQueries({ queryKey: [root] })`; anything not in the list survives logout.
- `frontend/packages/api-client/src/community/hooks.ts:29` — `communityKeys.all = ['community']`; `community.tsx` is a mounted route group (`frontend/apps/ppt-web/src/routes/groups/community.tsx`), yet `community` is **not** in the allow-list.
- `frontend/packages/api-client/src/voting/hooks.ts:24` — `votingKeys.all = ['voting']`; the allow-list has `'votes'` (:360) but no `'voting'` — a drift where the list *looks* like it covers the domain but never matches the live root.
- `frontend/apps/ppt-web/src/routes/groups/{community,disputes,iot,outages,leases,voting}.tsx` — all mounted route groups whose caches populate under the missing roots.

## Files
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:354`
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:614`
- `frontend/packages/api-client/src/community/hooks.ts`
- `frontend/packages/api-client/src/voting/hooks.ts`
- `frontend/apps/ppt-web/src/routes/groups/community.tsx`
- `frontend/apps/ppt-web/src/routes/groups/voting.tsx`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug vector, cache-purge trace)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Log in as User A, visit `/community`, `/voting`, `/disputes`, `/leases` (each populates its query cache with tenant-/user-scoped data).
2. Call `logout()` via the sidebar / AuthContext.
3. In DevTools, inspect `queryClient.getQueryCache().findAll()` — queries under `['community', …]`, `['voting', …]`, `['disputes', …]`, `['leases', …]` are still present.
4. Expected: after logout every authed cache root is empty. Actual: only the roots listed in `AUTHED_QUERY_KEY_ROOTS` are purged; the rest leak into the next login until a refetch.

## Suggested approach
1. Extend `AUTHED_QUERY_KEY_ROOTS` in `frontend/apps/ppt-web/src/lib/queryKeys.ts` with the mounted-route roots currently missing: at minimum `community`, `disputes`, `iot`, `outages`, `leases`, and add `voting` (keep `votes` too if any legacy hook still uses it; a quick grep of `frontend/apps/ppt-web/src` for `['votes'` decides).
2. Sweep `frontend/packages/api-client/src/*/hooks.ts` for every `all: [<root>]` factory that ppt-web imports and confirm each root is present in the allow-list — add any strays surfaced by the sweep (visitors, packages, violations, my-units, compliance, esignature, portfolio-performance, registries, templates, integrations, syndication, ecosystem, oauth-grants, mfa are candidates cited in the source signal; verify each is actually mounted in ppt-web before adding).
3. Add a guard test at `frontend/apps/ppt-web/src/lib/queryKeys.test.ts` (new file, or an existing suite) that imports every `*Keys` factory used by ppt-web, extracts `.all[0]`, and asserts each is present in `AUTHED_QUERY_KEY_ROOTS`. The test should fail today for the missing roots and pass after step 1.
4. Add a Vitest integration-style test in `AuthContext` that renders `AuthProvider`, seeds `queryClient` with `queryClient.setQueryData(['community', 'posts'], …)` (and one entry per missing root), calls `logout()`, then asserts `queryClient.getQueryCache().findAll()` no longer contains any of the seeded keys.
5. Run `pnpm --filter @ppt/web typecheck && pnpm --filter @ppt/web test` to confirm the new tests pass and nothing regresses.

## Alternatives considered
- **Restore `queryClient.clear()` on logout** — rejected because Issue #712 documented that the blunt clear also nuked query caches that survive login boundaries (SSR bootstrap data, `['app-config']`), causing a re-fetch storm and briefly-blank UI on the next login. The scoped-removal approach is correct; the fix is to keep it complete.
- **Runtime `mount → register root` API** — rejected as over-engineered: TanStack Query factories are statically imported and the allow-list is short. A guard test that enforces the invariant at CI time gives the same safety without a new registration layer.

## Root-cause trace
1. Symptom: After User A logs out on a shared workstation, User B's next login briefly renders User A's community feed / voting board / dispute threads until each list refetches.
2. ← `AuthContext.tsx:624-626` iterates `AUTHED_QUERY_KEY_ROOTS` and prefix-purges only those roots — anything not listed survives.
3. ← `lib/queryKeys.ts:354-392` — the allow-list is out of sync with the live query-key factories in `frontend/packages/api-client/src/{community,voting,disputes,iot,outages,leases,…}/hooks.ts` (each `all: [<root>]` is the first cache-key segment).
4. Origin: Issue #712 (PR replaced `queryClient.clear()` with the scoped loop) established the allow-list; every subsequent feature that mounted a new route group (community, disputes, iot, outages, leases, voting, and the `@ppt/api-client` roots enumerated above) landed without extending it. The `votes` → `voting` drift is a naming rename that never propagated to `AUTHED_QUERY_KEY_ROOTS`.

## Test plan
- [ ] `frontend/apps/ppt-web/src/lib/queryKeys.test.ts` — new guard test asserting every `*.all` root from the ppt-web-imported factories is in `AUTHED_QUERY_KEY_ROOTS` (failing on `main`, passing after the fix).
- [ ] `frontend/apps/ppt-web/src/contexts/AuthContext.test.tsx` — new / extended test that seeds queries under each affected root, calls `logout()`, asserts `queryClient.getQueryCache().findAll()` returns nothing carrying those roots.
- [ ] Local command: `pnpm --filter @ppt/web typecheck && pnpm --filter @ppt/web test -- lib/queryKeys AuthContext`

## Out of scope
- Rewriting the query-key factories themselves.
- Changing `logout()`'s server-side token-revocation semantics.
- Handling the `WebSocketContext.tsx` `categoryToQueryKeys` gap for `community` / `system` — that is a separate signal (`code-review-ppt-web-core-ws-category-map-gap`) and gets its own backlog row.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-authed-roots-session-leak.md`
- Mark the matching `backlog.json` row as `status: "done"`
