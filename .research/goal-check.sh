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
       [ "$hard" = "true" ] && HARD_FAIL=1; return 0
  fi
}

echo "==> goal-check (coverage=$COVERAGE action-list=$ACTION_LIST assign=$ASSIGN enforce=$ENFORCE)" >&2

# --- GC1: coverage referential integrity (record-only) ---
# (a) every gap-* action-list item maps to a real coverage story
#     (a story id is a prefix of the gap-id with the leading "gap-" stripped);
# (b) no `done` story retains an open gap-* task.
GC1_ORPHANS=$(jq -n --slurpfile al "$ACTION_LIST" --slurpfile cov "$COVERAGE" '
  ($cov[0].stories | map(.id)) as $sids
  | [ $al[0].items[]
      | select(.id | startswith("gap-"))
      | (.id | ltrimstr("gap-")) as $rest
      | select([ $sids[] | . as $sid | select($rest | startswith($sid)) ] | length == 0)
      | .id ] | length')
GC1_DONE_OPEN=$(jq -n --slurpfile al "$ACTION_LIST" --slurpfile cov "$COVERAGE" '
  ($cov[0].stories | map(select(.status=="done") | .id)) as $done
  | [ $al[0].items[]
      | select(.status=="open" and (.id | startswith("gap-")))
      | (.id | ltrimstr("gap-")) as $rest
      | select([ $done[] | . as $sid | select($rest | startswith($sid)) ] | length > 0)
      | .id ] | length')
GC1_PASS=$([ "$GC1_ORPHANS" = "0" ] && [ "$GC1_DONE_OPEN" = "0" ] && echo true || echo false)
record "GC1-referential-integrity" "$GC1_PASS" \
  "orphans=$GC1_ORPHANS done_with_open_gap=$GC1_DONE_OPEN" "orphans=0 done_with_open_gap=0" false

# --- GC2: coverage progress — done-story count is monotonic non-decreasing
# vs the previously-committed coverage.json (HEAD). This is the HARD-FAIL
# check once enforcement is turned on (PR 2). Exempt deep-scan commits
# (a deliberate human refresh may legitimately re-classify done -> partial).
GC2_NOW=$(jq '[.stories[] | select(.status=="done")] | length' "$COVERAGE")
GC2_KIND=$(jq -r '.scan_kind // "upkeep"' "$COVERAGE")
GC2_HEAD=$(git show "HEAD:$COVERAGE" 2>/dev/null \
           | jq '[.stories[] | select(.status=="done")] | length' 2>/dev/null || echo "$GC2_NOW")
if [ "$GC2_KIND" = "deep" ]; then
  record "GC2-coverage-progress" true "done=$GC2_NOW (deep-scan exempt)" "done>=$GC2_HEAD" true
elif [ "$GC2_NOW" -ge "$GC2_HEAD" ]; then
  record "GC2-coverage-progress" true "done=$GC2_NOW head=$GC2_HEAD" "done>=$GC2_HEAD" true
else
  record "GC2-coverage-progress" false "done=$GC2_NOW head=$GC2_HEAD" "done>=$GC2_HEAD" true
fi

# --- emit + exit ---
if [ "$EMIT_JSON" = "1" ]; then echo "$RESULTS"; fi
if [ "$ENFORCE" = "1" ] && [ "$HARD_FAIL" = "1" ]; then
  echo "==> goal-check: HARD FAIL (enforce on)" >&2; exit 1
fi
echo "==> goal-check: done (record-only)" >&2
exit 0
