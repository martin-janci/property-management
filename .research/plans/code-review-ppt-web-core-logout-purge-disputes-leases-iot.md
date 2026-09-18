# code-review-ppt-web-core-logout-purge-disputes-leases-iot

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review (dispatcher Tier-1d 2026-09-18 ppt-web-core queryKeys/AuthContext logout purge)
**Confidence:** high

## Hypothesis
On session change the ppt-web logout path calls `queryClient.removeQueries` only for the roots enumerated in `AUTHED_QUERY_KEY_ROOTS` — it does NOT call `queryClient.clear()`. Six query roots consumed by routed authed pages are missing from that allow-list, so their cached data (mediation notes, lease financials, IoT telemetry, resident own-unit data, document templates) survives logout and is served to the next user on the same browser until React Query refetches. This is the same defect class that PR #2650 and its follow-ups partially fixed twice already; new features keep adding roots that are never appended to the list. The smallest safe fix is to append the six missing roots now; a structural remedy (auto-derive the purge set from every registered key factory, or invert to a small non-session DENY list) removes the recurrence.

## Evidence
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:354-392` — `AUTHED_QUERY_KEY_ROOTS` is the explicit allow-list iterated on logout at `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:624-626`; no `queryClient.clear()` fallback (issue #712 removed it deliberately, per the module doc-comment at :331-352).
- Prior fixes of the same class already landed and are `done` in `.research/backlog.json`: `code-review-ppt-web-core-logout-cache-purge-gap` (added predictive-maintenance/sentiment/notification-analytics) and `code-review-ppt-web-core-logout-purge-notif-triggers` (added notification-triggers). Each fix patched only the roots then known.
- Confirmed still-missing routed roots (each with an authed `@ppt/api-client` query hook consumed by a routed ppt-web page): `disputes` (features/disputes/pages/MediationWorkspacePage.tsx, features/disputes/components/MediationChatThread.tsx, routes/groups/disputes.tsx; api-client `all: ["disputes"]` at packages/api-client/src/disputes/hooks.ts:29), `leases` + `violations` (routes/groups/leases.tsx; packages/api-client/src/leases/hooks.ts:23 & :39), `iot` (routes/groups/iot.tsx, features/iot/pages/IotAlertsPage.tsx, features/iot/hooks/useIotWebSocket.ts; packages/api-client/src/iot/hooks.ts:42), `my-units` (features/my-unit/pages/MyUnitPage.tsx, routes/groups/buildings.tsx; packages/api-client/src/my-units/hooks.ts:9), `templates` (features/document-templates/pages/DocumentTemplatesPage.tsx; packages/api-client/src/templates/hooks.ts:16).
- The regression contract already exists at `frontend/apps/ppt-web/src/lib/queryKeys.test.ts` — PRs #2943/#2952/#2956 hardened this file exactly to catch new *Keys factories drifting out of the allow-list, so a failing-on-main test is a natural extension there.

## Files
- `frontend/apps/ppt-web/src/lib/queryKeys.ts`
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx`
- `frontend/apps/ppt-web/src/lib/queryKeys.test.ts`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [x] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. In a clean profile, sign in to ppt-web as user A whose org has active mediation/lease/IoT data; open `/disputes`, `/leases`, `/iot`, `/my-unit`, `/document-templates` in that order so each root is cached in TanStack Query devtools.
2. Log out (AuthContext.logout runs the allow-list purge).
3. Sign in as user B in the SAME browser tab; navigate to `/disputes` before user B's data has time to refetch.
4. Expected: user B sees only their own data (or a loading state). Actual: TanStack Query devtools still lists cached entries under the missing roots (`disputes`, `leases`, `violations`, `iot`, `my-units`, `templates`) with user A's payload — user B's `MediationWorkspacePage` may momentarily render user A's mediation notes if the query resolves from cache before the refetch settles.

## Suggested approach
1. Extend `AUTHED_QUERY_KEY_ROOTS` in `frontend/apps/ppt-web/src/lib/queryKeys.ts` with `"disputes"`, `"leases"`, `"violations"`, `"iot"`, `"my-units"`, `"templates"` — one root per new line, preserve alphabetical grouping if the file uses it.
2. Extend `frontend/apps/ppt-web/src/lib/queryKeys.test.ts` so the auto-discovery test asserts each of the six new roots is present in `AUTHED_QUERY_KEY_ROOTS`. Write the assertion BEFORE editing step 1 so `pnpm -F @ppt/ppt-web test queryKeys` fails on the current tree (IG3 — a failing-on-main test).
3. Consumed `@ppt/api-client` factories the test auto-discovery must reach: `packages/api-client/src/{disputes,leases,iot,my-units,templates}/hooks.ts` — verify the current glob depth catches them; if not, widen the glob (PR #2956 already touched this same knob).
4. Add a comment above the AUTHED_QUERY_KEY_ROOTS array pointing at the (planned) structural follow-up so the next reviewer sees the recurrence pattern.
5. Run `pnpm -F @ppt/ppt-web test queryKeys` and `pnpm -F @ppt/ppt-web typecheck`; both must be green.
6. Open a follow-up issue for the structural fix (auto-derived purge set or DENY-list inversion) and reference it in the PR body under Out-of-scope — do NOT bundle it into this PR; a two-line append is the smallest change that closes the current leak, and the structural work touches every feature module.

## Alternatives considered
- **Replace the allow-list with `queryClient.clear()` on logout** — rejected because #712's original replacement was deliberate: `clear()` also nuked non-authed caches (public listings, i18n dictionaries, redesign preview state) that the app relies on across sessions. Restoring `clear()` regresses that fix; the STRUCTURAL fix (auto-derived roots) preserves both invariants.
- **Auto-derive the roots this PR** — rejected because it touches every feature module's key factory; that's a separate refactor with its own review surface. The two-line allow-list append is the smallest change that stops the current leak and the follow-up issue keeps the structural work visible.

## Root-cause trace
1. Symptom: after logout on a shared browser, the next user's routed pages under `/disputes`, `/leases`, `/iot`, `/my-unit`, `/document-templates` momentarily render the previous user's cached data.
2. ← `AuthContext.logout` (`frontend/apps/ppt-web/src/contexts/AuthContext.tsx:624-626`) iterates only `AUTHED_QUERY_KEY_ROOTS`; anything not listed is left in cache.
3. ← `AUTHED_QUERY_KEY_ROOTS` (`frontend/apps/ppt-web/src/lib/queryKeys.ts:354-392`) is a hand-maintained allow-list; six later-added feature roots (`disputes`, `leases`, `violations`, `iot`, `my-units`, `templates`) were never appended.
4. Origin: the allow-list pattern was introduced when `queryClient.clear()` was removed in issue #712; each subsequent feature added its own `queryKeys.all` root but the purge list was updated only when a reviewer noticed. PRs #2650 (notification-triggers), then the two `code-review-ppt-web-core-logout-*` fixes, patched the specific roots then known — exactly the pattern this plan is preventing again.

## Test plan
- [ ] Extend `frontend/apps/ppt-web/src/lib/queryKeys.test.ts` so the auto-discovery loop asserts each of the six new roots (`disputes`, `leases`, `violations`, `iot`, `my-units`, `templates`) is present in `AUTHED_QUERY_KEY_ROOTS`. Test must fail on the current `dev` head before step 1 of the approach lands (IG3).
- [ ] Regression: seed a cache entry under each of the six roots via `queryClient.setQueryData(["disputes"], ...)` and assert `queryClient.getQueryData(["disputes"])` is `undefined` after `logout()` in the AuthContext unit test.
- [ ] `pnpm -F @ppt/ppt-web test queryKeys` and `pnpm -F @ppt/ppt-web typecheck` — both green.

## Out of scope
- The STRUCTURAL fix (auto-derive purge set from every registered key factory, or invert to a small non-session DENY list). File a follow-up issue and reference it; do NOT bundle it into this PR — the structural work touches every feature module and needs its own review surface.
- Any change to the `@ppt/api-client` key factories themselves; this plan only touches the consumer allow-list and the coverage test.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-logout-purge-disputes-leases-iot.md`
- Mark the matching `backlog.json` row as `status: "done"`
