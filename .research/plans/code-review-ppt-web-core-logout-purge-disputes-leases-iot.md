# code-review-ppt-web-core-logout-purge-disputes-leases-iot

**Vector:** security
**Score:** 3
**Source:** dispatcher Tier-1d rotating-expert-review 2026-09-18 (ppt-web-core queryKeys/AuthContext logout purge) — signal `code-review-ppt-web-core-logout-purge-disputes-leases-iot`
**Confidence:** high

## Hypothesis
`logout()` in `AuthContext.tsx` no longer calls `queryClient.clear()` (issue #712 replaced it with a scoped removal that walks the hand-curated `AUTHED_QUERY_KEY_ROOTS` allow-list in `lib/queryKeys.ts`). Six routed authed roots — `disputes`, `leases`, `violations`, `iot`, `my-units`, `templates` — are consumed by ppt-web pages via `@ppt/api-client` query hooks but are absent from that allow-list, so on shared/kiosk workstations the outgoing user's cached mediation notes, dispute details, lease financials, tenant PII, IoT telemetry, and resident's own-unit data survive logout and are served to the next user until a refetch. This is the third recurrence of the same defect class (`logout-cache-purge-gap`, `logout-purge-notif-triggers` — both already fixed by appending roots one at a time). Immediate fix: append the six missing roots to `AUTHED_QUERY_KEY_ROOTS`. Structural fix (recommended, still cloud-landable): derive the purge set at runtime from the registered `keyFactory.all[0]` roots, or invert to a small session-safe DENY-list — so any new feature root is purged by default instead of leaking until someone remembers to append.

## Evidence
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:331-352` — the doc-comment explicitly warns the allow-list is hand-maintained and every new authed feature root must be added or it will leak across sessions.
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:354-392` — `AUTHED_QUERY_KEY_ROOTS` today omits `disputes`, `leases`, `violations`, `iot`, `my-units`, `templates`.
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:624-626` — `logout()` iterates only `AUTHED_QUERY_KEY_ROOTS` and calls `queryClient.removeQueries({ queryKey: [root] })`; there is no `queryClient.clear()` and no fall-through.
- `@ppt/api-client` root proof: `packages/api-client/src/disputes/hooks.ts:29` (`all: ['disputes']`), `.../leases/hooks.ts:23` (`all: ['leases']`), `.../leases/hooks.ts:39` (`all: ['violations']`), `.../iot/hooks.ts:42` (`all: ['iot']`), `.../my-units/hooks.ts:9` (`all: ['my-units']`), `.../templates/hooks.ts:16` (`all: ['templates']`).
- Prior fixes of the same class: `code-review-ppt-web-core-logout-cache-purge-gap` (done — added `predictive-maintenance`, `sentiment`, `notification-analytics`), `code-review-ppt-web-core-logout-purge-notif-triggers` (done, PR #2650 — added `notification-triggers`); each fix patched only the then-known roots.

## Files
- `frontend/apps/ppt-web/src/lib/queryKeys.ts`
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug/security)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Log in as tenant A in ppt-web on a shared browser. Navigate to the Disputes page — `useDispute()` populates the `['disputes', …]` query cache with A's mediation notes.
2. Call `logout()` via the account menu. `AuthContext.logout()` walks `AUTHED_QUERY_KEY_ROOTS` and calls `queryClient.removeQueries({queryKey:[root]})` for each listed root — `disputes` is NOT in the list.
3. Log in as tenant B in the same browser tab (no reload). Navigate to Disputes and read from `queryClient.getQueryData(['disputes', <A's id>])`.
4. **Actual:** the query cache still contains A's mediation-note payload; a component that reads from the cache (or a `useQuery(['disputes',...], ...)` before its `staleTime` expires) surfaces A's data to B until an explicit refetch resolves. **Expected:** the cache entry is gone (purged on logout).
5. Repeat with `leases`, `violations`, `iot`, `my-units`, `templates` — same class of leak, six distinct root families.

## Suggested approach
1. Append the six missing roots to `AUTHED_QUERY_KEY_ROOTS` in `frontend/apps/ppt-web/src/lib/queryKeys.ts` (alphabetical insertion around the existing `disputes`-neighbourhood in the array): `'disputes'`, `'leases'`, `'violations'`, `'iot'`, `'my-units'`, `'templates'`. Immediate defense; single-line adds.
2. Add a Vitest suite `frontend/apps/ppt-web/src/lib/queryKeys.test.ts` (or extend the existing `queryKeys.test.ts` from PR #2956) that:
   a. Imports the `@ppt/api-client` root factories (`disputesKeys`, `leasesKeys`, `violationsKeys`, `iotKeys`, `myUnitsKeys`, `templatesKeys`) and asserts each `factory.all[0]` string is present in `AUTHED_QUERY_KEY_ROOTS`.
   b. Runs the same assertion for every future *`Keys` factory discovered by the auto-discovery hook that PR #2956 shipped — so a new feature root fails CI on the day it's added, not on the day a real user is leaked.
3. Author a companion regression test `frontend/apps/ppt-web/src/contexts/AuthContext.logout-purge-leaks.test.tsx` that pre-populates the query cache with a dummy entry under each of the six roots, calls `logout()`, and asserts `queryClient.getQueryData([root, ...])` returns `undefined` for each. This is the failing-on-main test (IG3): drops back to red the moment any root is dropped from the allow-list.
4. Update the doc-comment at `lib/queryKeys.ts:331-352` to point at the new discovery test and the class of prior recurrences (`#2650`, `#2943`, this PR) so the next reader knows the pattern.
5. Structural follow-up (optional in this PR, otherwise a new plan): replace the hand-curated allow-list with a derivation from the registered `keyFactory.all[0]` roots — either an autoregister side-effect on each factory or a build-time codegen step. Cite the alternative in the PR description; leave the DENY-list variant as a rejected alternative below.
6. Run `pnpm --filter @ppt/web typecheck && pnpm --filter @ppt/web test` locally (or in the cloud runner) to prove the new tests fail on pre-fix code and pass on post-fix code.
7. Post-merge: confirm `@ppt/api-client`'s CI still emits the `*Keys` factory files unchanged (no wire-format drift).

## Alternatives considered
- **Restore `queryClient.clear()` in `logout()`** — rejected because issue #712 replaced `clear()` specifically to preserve select non-session caches (public unauthed lookups, i18n, feature flags) across a re-login; blanket clear regresses that.
- **Switch to a session-safe DENY-list (purge everything except an explicit whitelist)** — rejected as the sole change for this PR because the whitelist is not fully mapped today (would need an inventory pass across `@ppt/api-client` + local `queryKeys` roots) and getting it wrong drops legitimate non-session cache. Documented as the recommended structural follow-up so the recurrence stops for good.

## Root-cause trace
1. Symptom: tenant A's `disputes`/`leases`/`violations`/`iot`/`my-units`/`templates` cache entries persist through `logout()` and are readable by tenant B in the same browser tab, breaking multi-tenant isolation on shared workstations.
2. ← `contexts/AuthContext.tsx:624-626` — `logout()` calls only `queryClient.removeQueries({ queryKey: [root] })` for each `root` in `AUTHED_QUERY_KEY_ROOTS`, with no fall-through and no `queryClient.clear()`.
3. ← `lib/queryKeys.ts:354-392` — `AUTHED_QUERY_KEY_ROOTS` is a hand-curated allow-list missing the six new roots.
4. ← `@ppt/api-client` factories at `packages/api-client/src/{disputes,leases,iot,my-units,templates}/hooks.ts` register new roots as feature routes ship, without a linking mechanism to the ppt-web allow-list.
5. Origin: issue #712 replaced `queryClient.clear()` with scoped removal to preserve non-session caches; that decision assumed the allow-list would stay in sync as features shipped. Every subsequent feature route (`disputes` UC-DSP-*, `leases` UC-LEASE-*, `iot` UC-IOT-*, `my-units` UC-MYUNIT-*, `templates` UC-TPL-*) added a query root without updating the allow-list, so the isolation guarantee eroded silently.

## Test plan
- [ ] `frontend/apps/ppt-web/src/contexts/AuthContext.logout-purge-leaks.test.tsx` — new; pre-populates the six roots and asserts each is purged by `logout()`. Fails on main today.
- [ ] `frontend/apps/ppt-web/src/lib/queryKeys.test.ts` — extend PR #2956's auto-discovery to also cross-check that every `@ppt/api-client` `*Keys` factory's `all[0]` root appears in `AUTHED_QUERY_KEY_ROOTS`. Fails on main today.
- [ ] Regression command: `pnpm --filter @ppt/web test -- --run queryKeys logout-purge-leaks && pnpm --filter @ppt/web typecheck`

## Out of scope
- Migrating the whole purge model to a derived/DENY-list (see *Alternatives considered*); this PR is the immediate defense + coverage lock.
- Auditing `@ppt/api-client`'s own logout hooks in reality-web or the mobile-native/RN clients — those apps have their own session lifecycle; if they share the same class of gap, they are separate plans.
- Rotating the `queryKeys.ts` structure to co-locate roots by feature; a cosmetic change that would obscure the diff.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-logout-purge-disputes-leases-iot.md`
- Mark `backlog.json` row `code-review-ppt-web-core-logout-purge-disputes-leases-iot` as `status: "done"`
