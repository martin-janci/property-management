# code-review-ppt-web-core-logout-purge-disputes-leases-iot

**Vector:** security
**Score:** 3
**Source:** dispatcher Tier-1d rotating-expert-review 2026-09-18 (ppt-web-core: queryKeys/AuthContext logout purge)
**Confidence:** high

## Hypothesis
`AUTHED_QUERY_KEY_ROOTS` in `frontend/apps/ppt-web/src/lib/queryKeys.ts` is an explicit allow-list that `logout()` iterates via `queryClient.removeQueries({ queryKey: [root] })` at `contexts/AuthContext.tsx:624-626`. Since issue #712 replaced `queryClient.clear()` with this scoped removal, any authed TanStack Query root not present in the list survives logout and is served to the next user who logs in on the same browser until a natural refetch — a cross-user tenant / PII data exposure on shared workstations. Six new feature roots (`disputes`, `leases`, `violations`, `iot`, `my-units`, `templates`) added since the last purge patch are missing from the list. Smallest fix: append the six missing roots. Structural fix: replace the hand-curated allow-list with a derived registry so newly-added feature roots are purged by default.

## Evidence
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:354-392` — `AUTHED_QUERY_KEY_ROOTS` allow-list definition.
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:624-626` — logout iterates the list, no `queryClient.clear()` fallback.
- Recurring bug: two prior fixes (`code-review-ppt-web-core-logout-cache-purge-gap`, `code-review-ppt-web-core-logout-purge-notif-triggers` / PR #2650) patched the specific roots then known; the list has drifted again.
- Missing roots confirmed used by authed pages via `@ppt/api-client` query hooks: `disputes` (features/disputes/pages/MediationWorkspacePage.tsx; api-client `disputes/hooks.ts:29`), `leases` + `violations` (routes/groups/leases.tsx; api-client `leases/hooks.ts:23,39`), `iot` (routes/groups/iot.tsx; api-client `iot/hooks.ts:42`), `my-units` (features/my-unit/pages/MyUnitPage.tsx; api-client `my-units/hooks.ts:9`), `templates` (features/document-templates/pages/DocumentTemplatesPage.tsx; api-client `templates/hooks.ts:16`).
- Impact: HIGH — cross-session mediation notes, lease financials, tenant PII, resident own-unit data leak on shared/kiosk workstations.

## Files
- `frontend/apps/ppt-web/src/lib/queryKeys.ts`
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx`

## Required capabilities
- [x] C1 — Systematic debugging (bug/security)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception (structural fix option is controversial)

Mode: cloud-ok

## Repro steps
1. `pnpm -F ppt-web dev`; log in as user A in Chrome, navigate to `/disputes/<id>` so `useDispute` populates the TanStack cache under root `['disputes']`.
2. Click "Log out". Confirm the auth session is cleared.
3. Log in as user B (different tenant) in the same browser tab; navigate to `/disputes/<same-id>` before any network refetch.
4. Expected: request goes out, no cached data served, 403/404 rendered per B's scope. Actual: A's cached dispute detail flashes in — the root `'disputes'` was never purged because it's absent from `AUTHED_QUERY_KEY_ROOTS`.
5. Repeat with `/leases`, `/iot`, `/my-unit`, `/document-templates` for the other four missing roots.

## Suggested approach
1. Add the six missing string constants to `AUTHED_QUERY_KEY_ROOTS` in `frontend/apps/ppt-web/src/lib/queryKeys.ts` in alphabetical position: `'disputes'`, `'iot'`, `'leases'`, `'my-units'`, `'templates'`, `'violations'`.
2. Extend the existing coverage-check test (added by PR #2952 — `queryKeys.test.ts` auto-discovery of feature-local `*Keys` factories) to also cover `@ppt/api-client` root factories: enumerate `packages/api-client/src/*/hooks.ts` for exported `all: [<literal>]` roots and assert each appears in `AUTHED_QUERY_KEY_ROOTS` (unless explicitly listed on a small non-auth allow-list — e.g. i18n locale bundles).
3. In `AuthContext.tsx`, add a defensive `queryClient.clear()` fallback wrapped in a feature-flag `PPT_LOGOUT_STRICT_PURGE` (default `true`) so the allow-list becomes belt-and-suspenders, not the only line of defence. Comment references this plan for future readers.
4. Update the doc-comment at `queryKeys.ts:331-352` to state the coverage-test invariant so drift is caught at CI, not at runtime by the next user.
5. Add a regression test that populates each of the six missing roots, calls `logout()`, and asserts `queryClient.getQueryCache().findAll({ queryKey: [<root>] })` is empty afterwards.

## Alternatives considered
- **Full `queryClient.clear()` on logout** — rejected because issue #712 explicitly moved off it (breaks non-auth caches like locale bundles and static feature-flag responses that survive across sessions).
- **Invert to a small DENY-list of non-auth roots** — attractive long-term (default-purge); rejected as the primary fix because it changes semantics for every existing root and needs a wider audit than this plan is scoped to. Left as a follow-up.

## Root-cause trace
1. Symptom: user B, freshly logged in on the same browser, briefly sees user A's dispute / lease / IoT / my-unit / document-template data before refetch.
2. ← `logout()` at `contexts/AuthContext.tsx:624-626` iterates only `AUTHED_QUERY_KEY_ROOTS` (`lib/queryKeys.ts:354-392`) and never calls `queryClient.clear()`.
3. ← New feature hooks under `@ppt/api-client` (disputes, leases, iot, my-units, templates, violations) added a query root but nobody amended `AUTHED_QUERY_KEY_ROOTS`.
4. Origin: issue #712 (`queryClient.clear()` → scoped `removeQueries`) traded structural safety for cache preservation, without a compiler / test check that every registered root is enumerated. Two prior patch cycles (`logout-cache-purge-gap`, `logout-purge-notif-triggers` / PR #2650) confirmed the drift pattern.

## Test plan
- [ ] `frontend/apps/ppt-web/src/lib/queryKeys.test.ts` — assert `AUTHED_QUERY_KEY_ROOTS` contains each of the six missing roots.
- [ ] Extend the existing auto-discovery in `queryKeys.test.ts` (added by PR #2952) to also traverse `packages/api-client/src/*/hooks.ts` for `all: [<literal>]` roots.
- [ ] New `frontend/apps/ppt-web/src/contexts/AuthContext.test.tsx` case: populate each new root in a mock `queryClient`, call `logout()`, assert `queryClient.getQueryCache().findAll({ queryKey: [root] })` is empty.
- [ ] `pnpm -F ppt-web test` and `pnpm -F ppt-web typecheck` locally.

## Out of scope
- The structural rewrite to a derived registry or DENY-list (alternative above).
- Any changes to the SecureStore / NFC purge paths (covered by PRs #2955, #2950 already merged).
- Reality-web logout purge (`frontend/apps/reality-web/`) — different queryClient config, separate audit.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-logout-purge-disputes-leases-iot.md`
- Mark the matching `backlog.json` row as `status: "done"`
- File follow-up issue for the derived-registry / DENY-list structural fix so the drift class ends.
