# security-mobile-layout-cache-tenant-scope-issue-2486

**Vector:** security
**Score:** 2
**Source:** Issue #2486 | PR #2432
**Confidence:** high

## Hypothesis

The React Native dashboard layout cache in `frontend/apps/mobile/src/services/localCacheKeys.ts` uses `LAYOUT_CACHE_KEY(screen) = ` `` `ppt_layout_${screen}` `` — keyed only by screen, with no org or user id. On mount, `useDashboardLayout` hydrates from the cache (Phase 1) and calls `setLayout(cached)` before the background refresh (Phase 2) lands, so on a shared device org B briefly renders org A's tenant-scoped dashboard layout after account switch until Phase 2 completes. The `resetLocalData` logout sweep does not include the layout key (its filter lists `WIDGET_CONFIG_KEY` etc. statically but not the dynamic `ppt_layout_*` prefix), so the stale layout also survives explicit logout. Fix by (1) threading the current `orgId` into the key so a foreign tenant cannot read it in the first place, and (2) adding a prefix sweep to `resetLocalData` so stale entries are purged on logout even if a caller forgets the new arg.

## Evidence

- `frontend/apps/mobile/src/services/localCacheKeys.ts:27` — `export const LAYOUT_CACHE_KEY = (screen: string) => `` `ppt_layout_${screen.replace(/\//g, '_')}` `` — no tenant scope.
- `frontend/apps/mobile/src/services/localCacheKeys.ts:8` — adjacent doc comment for `WIDGET_CONFIG_KEY` explicitly flags this class of cross-tenant stale-data problem (issue #2361, #2399), showing the codebase is already aware of it.
- `frontend/apps/mobile/src/features/layout/layoutCache.ts:6,33` — `writeCachedLayout` / `readCachedLayout` shape-guard only asserts `screen` + `sections is array`; cannot detect foreign tenant.
- `frontend/apps/mobile/src/services/resetLocalData.ts:38-45` — `keys.filter` is a **fixed-key** allowlist; a dynamic-suffix key like `ppt_layout_ppt_dashboard` is not matched, so logout does not purge it.
- Issue #2486 (opened 2026-07-23 by post-merge review of PR #2432) documents the reproduction and the proposed prefix-sweep + org-scoping fix.

## Files

- `frontend/apps/mobile/src/services/localCacheKeys.ts`
- `frontend/apps/mobile/src/features/layout/layoutCache.ts`
- `frontend/apps/mobile/src/services/resetLocalData.ts`
- `frontend/apps/mobile/src/features/layout/layout.test.tsx`

## Dependencies

None — post-#2432 change on the mobile app.

## Required capabilities

- [x] C1 — Systematic debugging (security bug)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [x] C5 — ADB device (mobile-touching plan, verification benefits from a real device switch flow)
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
- Mode: local-only (reason: C5 — plan touches React Native cache lifecycle; test needs Android device switch to verify the sweep clears real AsyncStorage across account changes)

## Repro steps

1. Log in as user A (org A) on the mobile app; navigate to `ppt/dashboard`; wait for the resolved layout to render and the cache to populate (`AsyncStorage.getItem('ppt_layout_ppt_dashboard')` returns the org A layout JSON).
2. Log out (currently invokes `resetLocalData`) and immediately re-log in as user B (org B).
3. Expected: dashboard renders org B's layout without ever showing org A's; `AsyncStorage.getItem('ppt_layout_ppt_dashboard')` returns null or an org-B-prefixed key value.
4. Actual (today): after step 2, `useDashboardLayout`'s Phase 1 reads the cached org A payload and `setLayout(cached)` briefly renders org A's tenant-scoped section order/visibility before Phase 2's fetch replaces it. `AsyncStorage.getItem('ppt_layout_ppt_dashboard')` still holds org A's payload after logout — the sweep did not clear it.

## Suggested approach

1. Change `LAYOUT_CACHE_KEY` to accept an `orgId` argument: ``export const LAYOUT_CACHE_KEY = (orgId: string, screen: string) => `ppt_layout_${orgId}_${screen.replace(/\//g, '_')}`;`` and update the constant's doc comment to flag it as tenant-scoped.
2. Thread `orgId` through `layoutCache.ts` (`readCachedLayout`, `writeCachedLayout`) and `useDashboardLayout` — the hook already runs inside an authenticated screen, so the current user/org context is available (mirror the reader in `useOfflineSupport` for the token pattern).
3. In `resetLocalData.ts`, extend the `keys.filter` predicate with a prefix rule: `|| key.startsWith('ppt_layout_')`. This is a defense-in-depth belt-and-suspenders that also handles any residual entries written before the migration.
4. Add a `layoutCache.test.tsx` case: seed cache under `ppt_layout_ORGA_ppt_dashboard`; `readCachedLayout('ORGB', 'ppt/dashboard')` returns `null`; `resetLocalData()` removes all `ppt_layout_*` keys.
5. Extend `layout.test.tsx` with an integration scenario proving org A's cached layout is not activated for org B after account switch.
6. Update the mobile-side `docs/screens/ppt/dashboard.md` Agent Log entry (per screen-map integration protocol).
7. Verify locally with `pnpm --filter @ppt/mobile test -- layout.test.tsx layoutCache.test.ts`.

## Alternatives considered

- **Clear-on-logout only (keep the key unprefixed)** — rejected because the risk isn't only logout: a stale entry from a previous session that never went through `resetLocalData` (e.g. crash, app upgrade) still leaks; prefixing the key at write time is the primary fix.
- **Encrypt the payload with a per-user key** — rejected as over-engineered for a layout config (no PII; the exposure is UX not data). A key rename + prefix sweep is a two-line change with a full behavioral fix.

## Root-cause trace

1. Symptom: after switching accounts on the same device, org B briefly sees org A's dashboard layout on `ppt/dashboard` mount.
2. ← `useDashboardLayout` at `frontend/apps/mobile/src/features/layout/useDashboardLayout.ts` Phase 1 reads cache and `setLayout(cached)` before the background fetch (Phase 2) reconciles.
3. ← `readCachedLayout` at `frontend/apps/mobile/src/features/layout/layoutCache.ts:33` looks up `LAYOUT_CACHE_KEY(screen)` — the key is not tenant-scoped, so an org-B session reads org-A's entry.
4. ← `resetLocalData` at `frontend/apps/mobile/src/services/resetLocalData.ts:38` filters a fixed list of static keys and does not include a `ppt_layout_*` prefix rule, so logout does not evict the stale entry.
5. Origin: PR #2432 shipped `LAYOUT_CACHE_KEY` without tenant scoping and without updating `resetLocalData`.

## Test plan

- [ ] `frontend/apps/mobile/src/features/layout/layoutCache.test.ts` — `readCachedLayout('org-b-id', 'ppt/dashboard')` returns `null` when the only cached entry was written under `orgId=org-a-id`.
- [ ] `frontend/apps/mobile/src/features/layout/layout.test.tsx` — mount → account switch → mount cycle shows `setLayout` never called with the previous org's cached payload.
- [ ] `frontend/apps/mobile/src/services/resetLocalData.test.ts` — after `resetLocalData()`, `AsyncStorage.getAllKeys()` contains no key with `ppt_layout_` prefix (add fixture and assert).
- [ ] `pnpm --filter @ppt/mobile test -- layoutCache layout.test.tsx resetLocalData`

## Out of scope

- Migrating existing on-device entries (users are re-authenticated on the next launch and `resetLocalData` prefix sweep clears leftovers).
- Refactoring `useDashboardLayout`'s two-phase read model (the Phase 1 hydrate → Phase 2 refresh pattern is intentional for offline UX).
- The reality-web / ppt-web layout caches (this plan is mobile-scoped).

## After-merge

- Move this file to `plans/_archive/security-mobile-layout-cache-tenant-scope-issue-2486.md`
- Mark the matching `backlog.json` row (`security-mobile-layout-cache-tenant-scope-issue-2486`) as `status: "done"`
