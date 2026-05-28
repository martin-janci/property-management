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

## What it gives you
`.research/goal-check.sh` emits `{check,passed,observed,expected,hard}` rows:
- **GC1** coverage referential integrity — every `gap-*` action-list id maps to a real `coverage.json` story; no `done` story retains an open gap task.
- **GC2** coverage progress — `done`-story count is monotonic non-decreasing vs HEAD (deep-scan commits exempt). HARD-FAIL once `GOAL_CHECK_ENFORCE=1`.
- **GC3** buffer bounds — `open_claimable` in `[18, 60]` (catches both starvation and the 102/36 overflow).

Stem-uniqueness is enforced separately as `T20` in `dispatcher-self-test.sh`.

## Inputs
- `.research/management/coverage.json`, `action-list.json`, `assignments.json` (override via `COVERAGE` / `ACTION_LIST` / `ASSIGN` env).
- `GOAL_CHECK_ENFORCE=1` makes the hard-fail subset (GC2) exit non-zero. Default: record-only (exit 0).

## Steps
1. `./.research/goal-check.sh` — human-readable, record-only.
2. `./.research/goal-check.sh --json` — emit the `goal_checks[]` array (the dispatcher embeds the summary in its commit).

## Deterministic verification
- `test -x .research/goal-check.sh`
- `./.research/goal-check.sh --json | jq -e 'type == "array"'` exits 0.
- Against the fixtures, GC1 reports `orphans=1 done_with_open_gap=1` and T20 reports a duplicate stem.

## Smoke check
`COVERAGE=.research/fixtures/goal-check/coverage.json ACTION_LIST=.research/fixtures/goal-check/action-list.json ASSIGN=.research/fixtures/goal-check/assignments.json ./.research/goal-check.sh --json | jq -e 'length >= 3'`

## Cross-references
- `.research/dispatcher-prompt.md` Phase 6 (invocation), HARD RULES.
- `.research/dispatcher-self-test.sh` T20.
- `docs/superpowers/specs/2026-05-28-dispatcher-goal-convergence-design.md` (Pillar 1).
