#!/usr/bin/env bash
# Deterministic GOAL checks for the dispatcher — convergence, not just schema.
# Sibling of dispatcher-self-test.sh. Reads .research/management/*.json.
#
# Output: human lines on stderr; a goal_checks JSON array on stdout with --json.
# Modes:
#   (default)            record-only: prints results, ALWAYS exits 0.
#   GOAL_CHECK_ENFORCE=1 hard-fail subset (GC2) exits non-zero on violation.
#                        (PR 1 leaves this off everywhere — observe-only.)
#
# Usage:
#   ./.research/goal-check.sh            # against repo .research/management/*
#   ./.research/goal-check.sh --json     # emit goal_checks[] to stdout
#   COVERAGE=fixt/coverage.json ACTION_LIST=… ASSIGN=… ./.research/goal-check.sh
set -euo pipefail

COVERAGE="${COVERAGE:-.research/management/coverage.json}"
ACTION_LIST="${ACTION_LIST:-.research/management/action-list.json}"
ASSIGN="${ASSIGN:-.research/management/assignments.json}"
ENFORCE="${GOAL_CHECK_ENFORCE:-0}"
EMIT_JSON=0
[ "${1:-}" = "--json" ] && EMIT_JSON=1

RESULTS='[]'           # accumulates {check,passed,observed,expected,hard}
HARD_FAIL=0

# record <check> <passed:true|false> <observed> <expected> <hard:true|false>
record() {
  local check="$1" passed="$2" observed="$3" expected="$4" hard="$5"
  RESULTS=$(jq -c --arg c "$check" --argjson p "$passed" --arg o "$observed" \
              --arg e "$expected" --argjson h "$hard" \
              '. += [{check:$c,passed:$p,observed:$o,expected:$e,hard:$h}]' <<<"$RESULTS")
  if [ "$passed" = "true" ]; then printf '  ok    %s — %s\n' "$check" "$observed" >&2
  else printf '  FAIL  %s — observed=%s expected=%s\n' "$check" "$observed" "$expected" >&2
       [ "$hard" = "true" ] && HARD_FAIL=1
  fi
}

echo "==> goal-check (coverage=$COVERAGE action-list=$ACTION_LIST assign=$ASSIGN enforce=$ENFORCE)" >&2

# --- checks inserted by later tasks ---

# --- emit + exit ---
if [ "$EMIT_JSON" = "1" ]; then echo "$RESULTS"; fi
if [ "$ENFORCE" = "1" ] && [ "$HARD_FAIL" = "1" ]; then
  echo "==> goal-check: HARD FAIL (enforce on)" >&2; exit 1
fi
echo "==> goal-check: done (record-only)" >&2
exit 0
