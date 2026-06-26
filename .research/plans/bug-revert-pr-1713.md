# bug-revert-pr-1713

**Vector:** bug
**Score:** 3
**Source:** PR #1713
**Confidence:** high

## Hypothesis
PR #1690 ("feat(delegations): wire delegation feature into ppt-web (Story 3.4)") re-added
a delegations frontend feature that had previously been removed from the codebase, and
PR #1713 reverted it immediately. The PR #1690 title literally says "re-added", which
indicates this is the second cycle of add-then-revert for the same feature. Without a
decision recorded in the repo about whether delegations is intended to ship at all,
a third re-add is likely. The smallest fix is to (a) capture the decision (ship vs.
shelve) in `docs/use-cases.md` or a delegations-specific spec, and (b) put a guardrail
in place — either a tracking issue with the answer, or a CODEOWNERS / lint rule that
blocks another silent re-add — so the next contributor doesn't repeat the same loop.

## Evidence
- Revert: commit `62d4b6c` — `revert(delegations): remove re-added delegation frontend (#1690) (#1713)`; 13 files changed, 966 deletions
- Re-add: commit `2253201` — `feat(delegations): wire delegation feature into ppt-web (Story 3.4) (#1690)` — added `frontend/apps/ppt-web/src/features/delegations/`, `frontend/packages/api-client/src/delegation/*`, and route wiring
- Post-revert state on `dev`: no `frontend/apps/ppt-web/src/features/delegations/` directory exists; no `frontend/packages/api-client/src/delegation/` directory exists; `frontend/packages/api-client/src/index.ts` no longer re-exports a delegation module
- PR #1690 title explicitly says "re-added" — first add/revert cycle predates the routine's lookback window
- `docs/use-cases.md` references delegation use cases but the codebase has no implementation

## Files
- `frontend/apps/ppt-web/src/App.tsx`
- `frontend/apps/ppt-web/src/routes/AppRoutes.tsx`
- `frontend/apps/ppt-web/src/routes/lazyRoutes.tsx`
- `frontend/packages/api-client/src/index.ts`
- `docs/use-cases.md`

## Dependencies

(none — investigation can start immediately)

## Required capabilities
- [x] C1 — Systematic debugging (revert pattern needs backward tracing)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. `git log --all --oneline --grep="delegation" -i | head -20` to enumerate every add and revert
   commit for the delegations feature; expect to see at least two `feat(delegations): … re-added …`
   commits and two matching `revert(…)` commits, separated by other unrelated work
2. `git log --all --diff-filter=D --name-only --pretty="===%h %s" -- 'frontend/apps/ppt-web/src/features/delegations/**'`
   to find every commit that has *deleted* the delegations frontend directory; expected output
   is at least two such deletions (the two reverts)
3. Open each revert PR's description on GitHub and look for a stated reason; observed today: the
   revert PR descriptions cite "not in scope" or "premature" but no canonical decision was
   recorded in the repo (e.g. in `docs/use-cases.md` or a status block)
4. Expected after this plan ships: a single canonical decision recorded in `docs/use-cases.md`
   under the delegations use case, plus a tracking issue closed with the same decision, so a
   future contributor finds the answer before opening a third re-add

## Suggested approach
1. Enumerate the full add/revert history: `git log --all --pretty='%h %ai %s' -- 'frontend/apps/ppt-web/src/features/delegations/**' 'frontend/packages/api-client/src/delegation/**'`
2. Read the body of the original first-add PR and the first revert PR to find the original "why"; usually one of: feature spec changed, story de-scoped, dependency on backend story not landed
3. Cross-check against `_bmad-output/implementation-artifacts/sprint-status.yaml` and `docs/use-cases.md`: is the delegation story (Story 3.4 per PR #1690 title) marked done/in-progress/backlog?
4. Decide one of three outcomes with the user:
   - (a) **Ship it** — open a new feature PR with the implementation, the backend contract test, and a CODEOWNERS entry that requires a human approve any future delegation-touching diff
   - (b) **Shelve it** — record the decision in `docs/use-cases.md` ("Delegations: deferred until <story-id> ships") and open a tracking issue with the decision in its title so future searches surface it
   - (c) **Block it** — same as (b) plus a custom lint rule or a `CODEOWNERS` entry under `frontend/apps/ppt-web/src/features/delegations/` that fails any new PR adding the path
5. For (b) or (c), commit *only* the docs/CODEOWNERS change — no functional code touched
6. Re-run the routine: the `revert-pr-1713` signal should not re-fire (it's one-shot per id; idempotent)

## Alternatives considered
- **Open a GitHub issue and let the team decide async** — rejected because the same loop has already happened twice; the symptom is *no decision recorded*, and asking for another decision via async issue is exactly the pattern that produced the loop. A plan forces a synchronous choice and writes the answer down
- **Delete the open backlog row and move on** — rejected because the revert pattern is a real instability signal (it cost CI cycles and review bandwidth twice); ignoring it does not prevent the third re-add

## Root-cause trace
1. Symptom: `frontend/apps/ppt-web/src/features/delegations/` keeps being added and reverted (two cycles observed; second cycle within the routine's 10-day window)
2. ← Immediate cause: PR #1690 was authored without checking whether the prior revert had a documented "why" — commit `2253201` adds the directory back from scratch
3. ← Upstream cause: no stop-the-line marker in the codebase (no `CODEOWNERS` entry, no `// DO NOT RE-ADD: see issue #N` comment, no `docs/use-cases.md` annotation) on the delegations story
4. Origin: first add commit predates the routine's lookback; the original story (PR #1690 calls it "Story 3.4") may itself have been authored without the prerequisite backend contract

## Test plan
- [ ] Manual: search the repo for any reference to delegations (`rg -i delegation`) and confirm every hit is either documentation or a decision marker; no implementation files should exist after the plan ships (in outcomes b/c)
- [ ] Manual: open a throw-away branch that re-adds `frontend/apps/ppt-web/src/features/delegations/index.ts` and confirm CODEOWNERS / lint rule blocks the PR (outcome c only)
- [ ] Documentation diff: `docs/use-cases.md` contains exactly one delegations entry with the recorded decision and a link to the canonical tracking issue
- [ ] Command to run locally: `git log --all --oneline --grep="delegation" -i | head` (confirms no new add/revert after the plan lands)

## Out of scope
- Implementing the delegations feature itself (that is outcome (a), and outcome (a) opens a separate feature PR — not this plan)
- Refactoring the `frontend/packages/api-client/src/index.ts` re-export style
- Touching `frontend/apps/ppt-web/src/routes/lazyRoutes.tsx` other than to confirm it does not import a delegations route after the plan lands

## After-merge
- Move this file to `plans/_archive/bug-revert-pr-1713.md`
- Mark the matching `backlog.json` row (`bug-revert-pr-1713`) as `status: "done"`
- If outcome (c) was chosen, add a row to `_bmad-output/implementation-artifacts/sprint-status.yaml` marking Story 3.4 as `blocked` with a link to the decision
