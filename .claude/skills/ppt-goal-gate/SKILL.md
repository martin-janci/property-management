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
- `.research/dispatcher-prompt.md` Phase 3 (finish-first preamble + filter), Phase 6 (goal-check invocation), Phase 7 (Target-epic line), HARD RULES.
- `.research/dispatcher-self-test.sh` T20 (stem-uniqueness), T21 (objective.json shape).
- `.research/pick-target-epic.sh` + `.research/test-pick-target-epic.sh`.
- `docs/superpowers/specs/2026-05-28-dispatcher-goal-convergence-design.md` (Pillar 1).
