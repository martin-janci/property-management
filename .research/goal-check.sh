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
ARCHIVE="${ARCHIVE:-.research/management/assignments-archive.json}"
ENFORCE="${GOAL_CHECK_ENFORCE:-0}"
EMIT_JSON=0
[ "${1:-}" = "--json" ] && EMIT_JSON=1

# Buffer bounds — SINGLE SOURCE OF TRUTH, shared with dispatcher Phase 2.6.
# GC3 checks claimable ∈ [FLOOR, CEIL]; Phase 2.6 refills toward TARGET and
# drains anything above CEIL. Override via env to keep the two in lockstep
# (the dispatcher exports the same three before reading coverage). Changing a
# bound here must be mirrored in dispatcher-prompt.md Phase 2.6, never forked.
# Bounds are sized in DAYS OF THROUGHPUT (12 cron runs/day x DISPATCHER_CLAIM_CAP
# claims/run). Doubled on 2026-06-02 to track the claim-cap raise 3->6:
# TARGET = 1 day (12x6=72), FLOOR = half day (36), CEIL ~ 1.7 days (120). Keeping
# the buffer measured in time-of-work means the faster dispatcher always has
# enough ready items queued to fill its higher claim cap, instead of starving or
# being force-drained by a ceiling sized for the old slower rate.
BUFFER_FLOOR="${BUFFER_FLOOR:-36}"   # below this = starving → Tier-1 refill
BUFFER_TARGET="${BUFFER_TARGET:-72}" # refill brings claimable up to this
BUFFER_CEIL="${BUFFER_CEIL:-120}"    # above this = overflow → Phase 2.6 drains

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

# Observe-only: a missing input file must never abort a dispatcher run.
for f in "$COVERAGE" "$ACTION_LIST" "$ASSIGN"; do
  if [ ! -f "$f" ]; then
    echo "==> goal-check: SKIP (missing $f)" >&2
    [ "$EMIT_JSON" = "1" ] && echo "[]"
    exit 0
  fi
done

# --- GC1: coverage referential integrity (record-only) ---
# Match on the <epic>-<story> NUMERIC STEM, never the full slug. Coverage story
# ids and action-list gap ids carry DIFFERENT descriptive slugs for the same
# story (cov "10b-3-platform-health-monitoring" vs gap "gap-10b-3-admin-health-ui"),
# so the old full-slug prefix match false-flagged ~95 live items as orphans.
# The stem (e.g. "10b-3", "8a-3", "7a-1") is the stable join key.
#
# (a) orphans: a coverage-keyed gap item (id matches gap-<num>-<num>…) whose
#     stem maps to NO coverage story. Ad-hoc gaps (e.g. gap-security-*) are not
#     coverage-keyed and are exempt. A true orphan means coverage is missing
#     that story — author it (relink), do not prune live work.
# (b) leak: an OPEN gap item whose EXACT task_id is already merged/done in the
#     archive — shipped-but-never-closed. This replaces the old story-level
#     "done story has open gap" check, which false-alarmed on legitimate open
#     follow-ups under a done story (tests, retries, v2). The per-task archive
#     signal is precise and is the same leak as finding reclaim-of-already-merged-task-id.
GC1_STEM='capture("^(?<s>[0-9]+[a-z]?-[0-9]+)").s'
GC1_ORPHANS=$(jq -n --slurpfile al "$ACTION_LIST" --slurpfile cov "$COVERAGE" "
  (\$cov[0].stories | map(.id | $GC1_STEM) | unique) as \$cstems
  | [ \$al[0].items[]
      | select(.id | test(\"^gap-[0-9]+[a-z]?-[0-9]+\"))
      | ((.id | ltrimstr(\"gap-\")) | $GC1_STEM) as \$g
      | select((\$cstems | index(\$g)) == null)
      | .id ] | length")
GC1_LEAK=$(jq -n --slurpfile al "$ACTION_LIST" --slurpfile arc "$ARCHIVE" '
  ($arc[0].assignments | map(select(.status=="merged" or .status=="done") | .task_id)) as $term
  | [ $al[0].items[]
      | select(.status=="open" and (.id | startswith("gap-")))
      | .id as $id | select($term | index($id))
      | $id ] | length')
GC1_PASS=$([ "$GC1_ORPHANS" = "0" ] && [ "$GC1_LEAK" = "0" ] && echo true || echo false)
record "GC1-referential-integrity" "$GC1_PASS" \
  "orphans=$GC1_ORPHANS archive_terminal_leak=$GC1_LEAK" "orphans=0 archive_terminal_leak=0" false

# --- GC2: coverage progress — done-story count is monotonic non-decreasing
# vs the previously-committed coverage.json (HEAD). This is the HARD-FAIL
# check once enforcement is turned on (PR 2). Exempt deep-scan commits
# (a deliberate human refresh may legitimately re-classify done -> partial).
GC2_NOW=$(jq '[.stories[] | select(.status=="done")] | length' "$COVERAGE")
GC2_KIND=$(jq -r '.scan_kind // "upkeep"' "$COVERAGE")
# Read HEAD's done-story count. Distinguish three states explicitly (issue #700):
#   GC2_BASELINE=present   — git show succeeded, count is the prior committed value
#   GC2_BASELINE=missing   — git show failed (first commit of coverage.json,
#                            shallow clone where the blob isn't fetched, detached
#                            HEAD). Treated as monotonic pass but the head=N
#                            marker is suffixed `(no-baseline)` so it's auditable.
#   GC2_BASELINE=corrupt   — git show returned bytes but jq couldn't parse them.
#                            Also treated as monotonic pass + marked.
GC2_HEAD_RAW=$(git show "HEAD:$COVERAGE" 2>/dev/null || true)
if [ -z "$GC2_HEAD_RAW" ]; then
  GC2_HEAD="$GC2_NOW"
  GC2_BASELINE="missing"
else
  GC2_HEAD=$(printf '%s' "$GC2_HEAD_RAW" | jq '[.stories[] | select(.status=="done")] | length' 2>/dev/null || true)
  if [ -z "$GC2_HEAD" ]; then
    GC2_HEAD="$GC2_NOW"
    GC2_BASELINE="corrupt"
  else
    GC2_BASELINE="present"
  fi
fi
# Audit suffix: when the baseline is absent the line reads e.g.
# `done=24 head=24(no-baseline)`, surfacing the gap in the commit log instead of
# pretending the monotonicity check ran.
if [ "$GC2_BASELINE" = "present" ]; then
  GC2_HEAD_FMT="$GC2_HEAD"
else
  GC2_HEAD_FMT="$GC2_HEAD(no-baseline:$GC2_BASELINE)"
fi
if [ "$GC2_KIND" = "deep" ]; then
  record "GC2-coverage-progress" true "done=$GC2_NOW (deep-scan exempt)" "done>=$GC2_HEAD_FMT" true
elif [ "$GC2_NOW" -ge "$GC2_HEAD" ]; then
  record "GC2-coverage-progress" true "done=$GC2_NOW head=$GC2_HEAD_FMT" "done>=$GC2_HEAD_FMT" true
else
  record "GC2-coverage-progress" false "done=$GC2_NOW head=$GC2_HEAD_FMT" "done>=$GC2_HEAD_FMT" true
fi

# --- GC3: buffer bounds. open_claimable = open action-list items not already
# in active assignments, minus dep-blocked (a depends_on entry not pointing at
# a merged/done assignment). Healthy band [BUFFER_FLOOR, BUFFER_CEIL]; below
# FLOOR = starving, above CEIL = overflow (the 102/36 explosion). Bounds are
# the shared constants above so the Phase 2.6 refill/drain and this check can
# never disagree. Record-only here.
GC3_OPEN=$(jq --slurpfile a "$ASSIGN" '
  [.items[] | select(.status=="open")
            | .id as $id
            | select(($a[0].assignments | map(.task_id) | index($id)) == null)] | length' "$ACTION_LIST")
GC3_BLOCKED=$(jq --slurpfile a "$ASSIGN" '
  [.items[] | select(.status=="open")
            | .id as $id
            | select(($a[0].assignments | map(.task_id) | index($id)) == null)
            | select(((.depends_on // []) | length) > 0
                     and any((.depends_on // [])[];
                             . as $dep
                             | (($a[0].assignments | map(select(.task_id==$dep)) | .[0].status // "missing")
                                | IN("merged","done") | not)))] | length' "$ACTION_LIST")
GC3_CLAIMABLE=$((GC3_OPEN - GC3_BLOCKED))
GC3_PASS=$([ "$GC3_CLAIMABLE" -ge "$BUFFER_FLOOR" ] && [ "$GC3_CLAIMABLE" -le "$BUFFER_CEIL" ] && echo true || echo false)
record "GC3-buffer-bounds" "$GC3_PASS" \
  "claimable=$GC3_CLAIMABLE (open=$GC3_OPEN dep_blocked=$GC3_BLOCKED)" \
  "$BUFFER_FLOOR<=claimable<=$BUFFER_CEIL" false

# --- emit + exit ---
if [ "$EMIT_JSON" = "1" ]; then echo "$RESULTS"; fi
if [ "$ENFORCE" = "1" ] && [ "$HARD_FAIL" = "1" ]; then
  echo "==> goal-check: HARD FAIL (enforce on)" >&2; exit 1
fi
echo "==> goal-check: done (record-only)" >&2
exit 0
