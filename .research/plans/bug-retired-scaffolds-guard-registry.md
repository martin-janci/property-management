# bug-retired-scaffolds-guard-registry

**Vector:** bug
**Score:** 3
**Source:** PR #1713 (revert of #1690 — delegation frontend re-admission)
**Confidence:** high

## Hypothesis
PR #1690 re-wired the standalone `delegation` frontend into `ppt-web` — the router, nav, and `@ppt/api-client` module — because the delegation surface had been *deleted* under PAP-33 ("Retired 7 dead scaffolds"), not *guarded* under PAP-55's `UNWIRED_FEATURES` registry. PAP-33's group (`migration`, `integrations`, `multi-currency`, `api-ecosystem`, `delegation`, `data-residency`, `packages`) is the class of retired surfaces the guard test explicitly does **not** cover — it can only fail on a slug still present in `features/`. A gap-sweep PR (#1690) then silently re-created `features/delegations/`, and the guard could not see it because delegation isn't in `UNWIRED_FEATURES`. #1713 reconciled `dev` (BIT-213), but the class-of-bug remains: any of the other 6 PAP-33 retirements can be re-introduced the same way. The fix is a **retired-scaffolds registry** that mirrors `UNWIRED_FEATURES` but asserts the *opposite* — every slug in `RETIRED_FEATURES` must have **no** `features/<slug>/` directory and **no** router/nav references.

## Evidence
- `frontend/apps/ppt-web/src/features/unwired-features.ts:1-52` — the header comment names the 7 PAP-33 retirements explicitly (including `delegation`) and states "Those dirs no longer exist on `dev`, so the guard no longer tracks them (a deleted feature cannot be re-exposed)." PR #1690 falsified that assumption.
- `frontend/apps/ppt-web/src/test/unwired-features.test.ts:44-56` — the current guard only iterates `UNWIRED_FEATURES` (10 slugs); a retirement guard must be a sibling `describe(...)` block iterating `RETIRED_FEATURES` with the *inverse* assertion (dir absent, no imports).
- PR #1713 body (revert) — "The mechanic merge re-admitted a board-retired customer surface. Atlas: *'a gap-sweep does not silently overturn a board decision.'*"
- PR #1690 landed 2026-06-22 16:39, ~16 min before CEO's BIT-198 ruling at 16:55 — no CI mechanism caught the class-of-bug (the guard test has no coverage for it).

## Files
- `frontend/apps/ppt-web/src/features/unwired-features.ts`
- `frontend/apps/ppt-web/src/test/unwired-features.test.ts`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser  · local-only
- [ ] C5 — ADB device  · local-only
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. On a clean `dev` checkout, run `pnpm --filter @ppt/web test -- unwired-features`. Expected: green (`UNWIRED_FEATURES` guard passes because `features/delegations/` no longer exists after #1713).
2. Create a spike branch. Add `frontend/apps/ppt-web/src/features/delegations/index.ts` with `export const X = 1`. Add a matching route group `routes/groups/delegations.tsx` that imports it and wires it into `AppRoutes.tsx`.
3. Re-run `pnpm --filter @ppt/web test -- unwired-features`. Expected today: green (the guard cannot detect the re-admission — the slug is not in `UNWIRED_FEATURES`). After this plan lands: **red** — the `RETIRED_FEATURES` guard fires with "features/delegations must remain deleted (PAP-33 retirement)".
4. Discard the spike branch.

## Suggested approach
1. In `frontend/apps/ppt-web/src/features/unwired-features.ts`, append a second exported constant:
   ```ts
   /** PAP-33 retirements — deleted scaffolds. Must not be re-created. */
   export const RETIRED_FEATURES = [
     'migration',
     'integrations',
     'multi-currency',
     'api-ecosystem',
     'delegation',
     'delegations',   // both singular + plural to match #1690's actual dir name
     'data-residency',
     'packages',
   ] as const;
   export type RetiredFeature = (typeof RETIRED_FEATURES)[number];
   ```
   Update the file's top JSDoc to explain the twin registries — unwired = keep-and-hide, retired = delete-and-block.
2. In `frontend/apps/ppt-web/src/test/unwired-features.test.ts`, add a sibling `describe('Retired-scaffolds registry (PAP-33)')` block that:
   - Asserts `features/<slug>/` **does not** exist for every slug in `RETIRED_FEATURES` (use the same `readdirSync` pattern as the existing `it('retains the code…')` block, inverted).
   - Asserts no file in `routingSurfaceSources()` imports `features/<slug>` (same `importRe` pattern the current guard uses, inverted expectation — no offenders).
   - Also asserts no file under `frontend/packages/api-client/src/<slug>/` exists (mirrors #1690's re-created `@ppt/api-client/src/delegation/` module).
3. Reuse `routingSurfaceSources()` — do not duplicate it. Extract the `importRe` construction into a small helper if the two guards share it.
4. Run `pnpm --filter @ppt/web test -- unwired-features` locally — confirm green on current `dev`, and red after a deliberate re-add (step 3 of *Repro*).
5. In the PR body, cite `PAP-33`, `PAP-55`, `BIT-213`, and PR #1713; mark this as the follow-up that closes the class-of-bug.

## Alternatives considered
- **A CI gate on `.github/workflows/` that greps for `features/delegation` etc. across the whole repo before merge** — rejected because grep-based CI checks live outside the codebase and are trivially bypassable ("just rename the dir"); Vitest inside `ppt-web` runs on every backend-and-frontend PR via `frontend.yml` and is closer to the source of truth (`RETIRED_FEATURES`).
- **Add the retired slugs to `UNWIRED_FEATURES` and change the existing guard's semantics to accept both retained + retired states** — rejected because it conflates two different board decisions (PAP-55 keep-and-hide vs PAP-33 delete-and-block) and weakens the current guard's meaning (`UNWIRED_FEATURES` is "retained but hidden"; adding retired entries would invert its `retains-the-code` assertion).

## Root-cause trace
1. Symptom: PR #1690 merged, re-wiring `features/delegations/` + `/delegations` route + nav + `@ppt/api-client/delegation` module. CEO ruled 16 min later (BIT-198) that delegation stays retired. #1713 reverted.
2. ← `frontend/apps/ppt-web/src/test/unwired-features.test.ts:44` — the guard iterates only `UNWIRED_FEATURES` (10 slugs), all of which have `features/<slug>/` present. It has no coverage for slugs whose directories are supposed to remain absent.
3. ← `frontend/apps/ppt-web/src/features/unwired-features.ts:26-38` — the file's own header comment names the 7 PAP-33 retirements and states "a deleted feature cannot be re-exposed" — an assumption a gap-sweep PR falsified.
4. Origin: PAP-33 (2026-05-13) executed as *code deletion only* with no CI guardrail; PAP-55 (2026-05-27) added a guard for the *hide-but-keep* class but not for the *delete-and-block* class. The gap was latent for ~40 days until PR #1690 tripped it.

## Test plan
- [ ] `frontend/apps/ppt-web/src/test/unwired-features.test.ts` — new `describe('Retired-scaffolds registry (PAP-33)')` block with three `it(...)`s: dir absent, no router/nav import, no `@ppt/api-client/<slug>/` module. Fails on the pre-fix code once `features/delegations/index.ts` is re-added in a spike; passes on current `dev` after this plan lands.
- [ ] Regression scenario: same spike as *Repro* step 2 — re-add `features/delegations/index.ts`, confirm the new test goes red. Delete the spike, confirm green.
- [ ] Exact command: `pnpm --filter @ppt/web test -- unwired-features`.

## Out of scope
- Extending the guard to `reality-web` or `mobile` — those apps have their own retirement classes; separate plans if/when their catalog is documented.
- A backend-side guard for retired routes (e.g. `/api/v1/delegations` — which stays live for voting delegation, Story 5.4). Backend retirement is context-dependent and not the pattern that broke here.
- Retro-fitting `@ppt/sitemap` to warn when it references a retired slug — the sitemap package's ownership boundary is separate; the ppt-web test is the enforcing checkpoint.

## After-merge
- Move this file to `plans/_archive/bug-retired-scaffolds-guard-registry.md`
- Mark the matching `backlog.json` row (`reverted-pr-1690`) as `status: "done"`
