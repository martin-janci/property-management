# code-review-ppt-web-core-registries-root-logout-leak

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review ppt-web-core 2026-10-07 (routine Phase 1.5)
**Confidence:** high

## Hypothesis

The ppt-web logout flow purges TanStack Query cache keys scoped to the `AUTHED_QUERY_KEY_ROOTS` allowlist in `frontend/apps/ppt-web/src/lib/queryKeys.ts`. That allowlist does **not** include the `'registries'` root (note the plural — the queryKeys factory at `frontend/packages/api-client/src/registry/hooks.ts:23` uses `['registries'] as const`). On a shared workstation, user A's pet and vehicle registry rows — names, license plates, pet medical notes, all tenant-scoped PII — survive logout in the in-memory cache and are visible to user B on next login until first-fetch invalidation. This is the exact same bug class as the already-fixed `code-review-ppt-web-core-authed-roots-session-leak`, just a different feature folder that was missed by that sweep. The fix is a one-line allowlist addition plus a regression entry in the auto-discovery test.

## Evidence

- `frontend/packages/api-client/src/registry/hooks.ts:23` — `all: ['registries'] as const` (plural noun — do not confuse with any `registry` singular key elsewhere).
- `frontend/apps/ppt-web/src/lib/queryKeys.ts:354-380` — `AUTHED_QUERY_KEY_ROOTS` lists accounting, announcements, faults, documents, votes, messages, neighbors, forms, person-months, self-readings, user, buildings, notifications, meters, reports, developer, ocr, actionQueue, executionLogs, executionStats, ai-chat, financial, rentals (and more down-list) — but **not `registries`**.
- Prior analogous bug class: `code-review-ppt-web-core-authed-roots-session-leak` (PR not yet landed, plan at `plans/code-review-ppt-web-core-authed-roots-session-leak.md`) fixes the same hole for community/voting roots; this is the Epic-57 registry equivalent.
- `frontend/apps/ppt-web/src/lib/queryKeys.test.ts` — the auto-discovery coverage test (PR #2952) exists but did not catch this because `registryKeys` lives in the shared `@ppt/api-client` package, outside the glob that walks `apps/ppt-web/src/features/**/hooks/`.

## Files

- `frontend/apps/ppt-web/src/lib/queryKeys.ts:354`
- `frontend/packages/api-client/src/registry/hooks.ts`
- `frontend/apps/ppt-web/src/lib/queryKeys.test.ts`

## Dependencies

<!-- no blocking dependencies; independent from the sibling registry-api-no-auth-token plan -->

## Required capabilities

- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps

1. Start ppt-web locally with a seeded database containing two residents (user A and user B) in the same building with distinct pet/vehicle registrations.
2. Log in as user A. Open `/registry` → Pets tab. Wait for the pets list to populate.
3. Open DevTools → React Query devtools (or inspect the `queryClient` directly in console). Confirm `['registries','pets','list',...]` keys are populated.
4. Click Logout. Confirm the login screen renders.
5. Log in as user B. Immediately open `/registry` → Pets tab (before the first fetch resolves).
6. **Expected:** empty cache, loading spinner, then user B's rows.
7. **Actual:** user A's pet rows render in the first paint before user B's data arrives — the cache under `['registries', ...]` was never purged.

## Suggested approach

1. In `frontend/apps/ppt-web/src/lib/queryKeys.ts:354`, add `'registries'` to the `AUTHED_QUERY_KEY_ROOTS` array. Keep the array alphabetised in whichever section (shared @ppt/api-client factory roots or ad-hoc) the comment style indicates — in this case, the shared-factory section just below `'reports'`.
2. In `frontend/apps/ppt-web/src/lib/queryKeys.test.ts`, extend the auto-discovery glob to also walk `../../../../packages/api-client/src/**/hooks.ts`, so any shared factory's `*Keys` export is discovered. (If the current glob rules forbid crossing the pnpm workspace boundary, hard-code `registryKeys` into the imported-factories list and leave a comment pointing at this plan.)
3. Add a focused test: import `registryKeys` + `AUTHED_QUERY_KEY_ROOTS`, assert `AUTHED_QUERY_KEY_ROOTS.includes(registryKeys.all[0])`. This is the regression anchor that would have caught both this and the earlier `authed-roots-session-leak` finding.
4. Run `pnpm --filter @ppt/web test -- queryKeys` and `pnpm --filter @ppt/web typecheck`.
5. Verify at least one logout-purge test (if one exists in `AuthContext.test.tsx` or similar) still passes.
6. No other code changes — the allowlist is the whole fix.

## Alternatives considered

- **Clear the entire query cache on logout** — rejected because that regresses Issue #712 (overly aggressive clear invalidated unrelated public data too); the current scoped-purge model is intentional.
- **Rename `registryKeys.all[0]` from `'registries'` to `'registry'` to match a presumed singular convention** — rejected because the convention in this codebase is pluralised collection roots (`buildings`, `announcements`, `faults`, `meters`, `reports`), so plural is correct; it's the allowlist that's incomplete.

## Root-cause trace

1. Symptom: pet/vehicle registry rows from user A visible in user B's session after logout → login on the same browser.
2. ← Immediate cause: `queryKeys.ts:354 AUTHED_QUERY_KEY_ROOTS` lacks `'registries'`, so the logout-purge helper (which filters cache entries by `queryKey[0] in AUTHED_QUERY_KEY_ROOTS`) misses the registry subtree.
3. ← Upstream cause: `registryKeys` lives in `@ppt/api-client` (shared, auto-generated), not in `apps/ppt-web/src/features/**/hooks/`, so the auto-discovery test from PR #2952 doesn't walk it.
4. Origin: Epic 57 (building registries) was authored after the queryKeys allowlist pattern was established; the author added the keys factory in the shared package but didn't propagate the root into the app-level allowlist. Same oversight class as the community/voting roots the `authed-roots-session-leak` plan fixes.

## Test plan

- [ ] `frontend/apps/ppt-web/src/lib/queryKeys.test.ts` — new assertion that `registryKeys.all[0]` is contained in `AUTHED_QUERY_KEY_ROOTS`. This fails on main today.
- [ ] (If feasible without over-widening the glob) Extend the auto-discovery to walk `@ppt/api-client/src/**/hooks.ts`; add a shared-factory list that would have surfaced `registryKeys` on PR #2952.
- [ ] Local verify: `pnpm --filter @ppt/web test -- queryKeys` + `pnpm --filter @ppt/web typecheck`.

## Out of scope

- Fixing the sibling `code-review-ppt-web-core-registry-api-no-auth-token` finding — separate plan, separate PR.
- Rewriting the logout-purge helper itself; the mechanism is correct, only the allowlist is incomplete.
- A broader audit of every `*Keys` factory across all shared packages — Issue #2948 / PR #2952 already covered the apps side; a follow-up could widen the auto-discovery, but that belongs in its own plan.

## After-merge

- Move this file to `plans/_archive/code-review-ppt-web-core-registries-root-logout-leak.md`
- Mark the matching `backlog.json` row as `status: "done"`
