---
name: ppt-goal-gate
description: Deterministic goal/convergence checks for the PPT dispatcher — coverage referential integrity, coverage-progress monotonicity, and buffer bounds. Run in dispatcher Phase 6 and CI. Use when adding/auditing the dispatcher's goal-verification layer.
mode: cloud-ok
---

# ppt-goal-gate

## When to invoke
- The dispatcher's Phase 6 runs `goal-check.sh` to record convergence health in the commit.
- CI runs it on PRs touching the dispatcher prompt / scripts.
- A human auditing whether the autonomous loop is making *measured* progress (not just merging PRs).
- As of PR 2, `goal-check.sh` runs with `GOAL_CHECK_ENFORCE=1 in Phase 6` + CI; `T20` (in `dispatcher-self-test.sh`) and `GC7` (routine `G16`) also enforce.

## What it gives you
`.research/goal-check.sh` emits `{check,passed,observed,expected,hard}` rows:
- **GC1** coverage referential integrity — every `gap-*` action-list id maps to a real `coverage.json` story; no `done` story retains an open gap task.
- **GC2** coverage progress — `done`-story count is monotonic non-decreasing vs HEAD (deep-scan commits exempt). **ENFORCING (PR 2):** a regression ABORTS the dispatcher run and fails CI.
- **GC3** buffer bounds — `open_claimable` in `[18, 60]` (catches both starvation and the 102/36 overflow).

Stem-uniqueness is enforced separately as `T20` in `dispatcher-self-test.sh`.

### Quarantine + pre-merge autofix + claim-time dedup (PR 5/5)

**Quarantine** (`status=quarantined`): set by Phase 5.7 when
`fix_rounds >= 3` and the reviewer still returns `verdict=changes`.
Excluded from claim selection AND WIP count. Operator un-quarantines by
editing `status` back to `review`. Self-test `T23` enforces
`quarantined_at` + (`fix_rounds >= 3` OR `quarantine_reason`).

**Pre-merge autofix** (Phase 5.4, between Phase 5 and Phase 5.5): for
approved PRs failing CI with delta entirely inside the mechanical-only
path set (SQLx offline, `Cargo.lock`, generated clients, lockfiles),
spawn a subagent that runs `ppt-pr-merge` Step 2 (rebase + regenerate),
force-pushes, re-triggers CI, and returns. Avoids routing mechanical-
only failures through the expensive Phase 5.7 respawn. Cap: 1 mechanical
autofix per row per 24h.

**Claim-time dedup tightening:**
- Phase 3 claim predicate now includes a stem-aware **active-or-quarantined**
  check (parallel to the existing stem-aware terminal check), so an
  `<id>-impl` variant cannot be claimed while `<id>` is in-flight or
  under operator triage.
- Routine `Phase 3 promote` extends `slug-stem uniqueness` to include
  open action-list items, not just plans+assignments — closes the gap
  where the planner emits both `<id>-retry` and `<id>-v2`.
- Self-test `T24` enforces at-most-one-OPEN-per-stem in action-list.
  Hard-fail; the pre-commit gate (PR 2/5) blocks the dispatcher commit
  on violation, so the bad state never reaches `dev`.

### WIP throttle (PR 4/5 — `DISPATCHER_WIP_CAP`)

Phase 3's `free_slots` is no longer a constant 3. It's
`min(3, max(0, DISPATCHER_WIP_CAP - WIP_NOW))` where `WIP_NOW` is the
count of `{in-progress, review}` rows in `assignments.json`. Default
cap is **8**; set `DISPATCHER_WIP_CAP=0` to disable and restore the
legacy uncapped behavior.

Goal: bound the approved-but-unmergeable pool. When the pool grows
faster than merges drain it, the throttle creates back-pressure on new
claims so Phase 5/5.5/5.6 can catch up.

Self-test `T22` enforces a soft bound (note when over cap, hard-fail
only at `WIP > 2 × cap`).

The WIP throttle composes with finish-first: WIP bounds the *quantity*
of in-flight work; finish-first bounds the *quality* (one epic at a
time). Both apply uniformly; finish-first does not override the WIP cap.

### Finish-first picker (PR 3/5 — `pick-target-epic.sh`)

`.research/pick-target-epic.sh` selects ONE target epic per dispatcher run
(behind `DISPATCHER_FINISH_FIRST=1`) and writes it to
`.research/management/objective.json`. The dispatcher's Phase 3 filters
claim candidates to that target epic, biasing all 3 slots toward
finishing one epic before starting another. Rule: **closest-to-done** —
prefer the epic with fewest claimable open tasks; tie-break by max
priority. Idempotent: KEEPs the current target until exhausted.

Schema for `objective.json`:

```json
{ "schema_version": 1,
  "epic_prefix": "gap-10a",
  "selected_at": "2026-05-28T18:50:52Z",
  "last_action": "select|keep|repick",
  "reason": "<one-line explanation>",
  "stats_at_selection": { "open": 2, "claimable": 1 } }
```

Enforced by self-test `T21` (epic_prefix matches `gap-N[a-z]?` or
`pm-<role>` or the `NONE` sentinel for no-claimable-work runs).

## Inputs
- `.research/management/coverage.json`, `action-list.json`, `assignments.json` (override via `COVERAGE` / `ACTION_LIST` / `ASSIGN` env).
- `GOAL_CHECK_ENFORCE=1` makes the hard-fail subset (GC2) exit non-zero. Default: record-only (exit 0).
- `DISPATCHER_FINISH_FIRST=1` makes Phase 3 invoke `pick-target-epic.sh` and filter claims to one epic.
- `DISPATCHER_WIP_CAP=<n>` (default 8) caps in-flight work; `=0` disables. Phase 3 derives `free_slots` from `cap - WIP_NOW`.

## Steps
1. `./.research/goal-check.sh` — human-readable, record-only.
2. `./.research/goal-check.sh --json` — emit the `goal_checks[]` array (the dispatcher embeds the summary in its commit).
3. `./.research/pick-target-epic.sh --json` — dry-run; `--update` writes `objective.json`. Dispatcher Phase 3 always runs with `--update` under `DISPATCHER_FINISH_FIRST=1`.

## Deterministic verification
- `test -x .research/goal-check.sh`
- `./.research/goal-check.sh --json | jq -e 'type == "array"'` exits 0.
- Against the fixtures, GC1 reports `orphans=1 done_with_open_gap=1` and T20 reports a duplicate stem.

## Smoke check
- `COVERAGE=.research/fixtures/goal-check/coverage.json ACTION_LIST=.research/fixtures/goal-check/action-list.json ASSIGN=.research/fixtures/goal-check/assignments.json ./.research/goal-check.sh --json | jq -e 'length >= 3'`
- `bash .research/test-pick-target-epic.sh` (5-case synthetic-fixture smoke)

## Cross-references
- `.research/dispatcher-prompt.md` Phase 3 (WIP-throttle preamble, finish-first preamble + filter), Phase 6 (goal-check invocation), Phase 7 (WIP-throttle + Target-epic lines), HARD RULES.
- `.research/dispatcher-self-test.sh` T20 (active stem-uniqueness), T21 (objective.json shape), T22 (WIP bounds), T23 (quarantine invariants), T24 (action-list stem-uniqueness).
- `.research/pick-target-epic.sh` + `.research/test-pick-target-epic.sh`.
- `docs/superpowers/specs/2026-05-28-dispatcher-goal-convergence-design.md` (Pillar 1).
