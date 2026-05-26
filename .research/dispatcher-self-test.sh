#!/usr/bin/env bash
# Deterministic self-test for the dispatcher invariants.
#
# Validates the schema and state-machine discipline encoded in
# .research/dispatcher-prompt.md against the current
# .research/management/assignments.json.
#
# Exits non-zero on any violation. Safe to run anywhere — read-only.
#
# Usage:
#   ./.research/dispatcher-self-test.sh                  # uses repo root
#   ./.research/dispatcher-self-test.sh path/to/file.json # custom file

set -euo pipefail

ASSIGN="${1:-.research/management/assignments.json}"
PROMPT="${DISPATCHER_PROMPT:-.research/dispatcher-prompt.md}"
SKILLS_DIR="${SKILLS_DIR:-.claude/skills}"
# Cutoff for hardening-era checks. Rows claimed before this date are legacy
# (pre-merged_at, pre-hardening-fields) and are exempted from T5 + T11.
HARDENING_DATE="${HARDENING_DATE:-2026-05-25T00:00:00Z}"

FAIL=0
note() { printf '  ok    %s\n' "$1"; }
fail() { printf '  FAIL  %s\n' "$1" >&2; FAIL=1; }

echo "==> dispatcher-self-test"
echo "    assignments: $ASSIGN"
echo "    prompt:      $PROMPT"
echo

# --- T0: required files exist -----------------------------------------------
echo "T0  required files exist"
for f in "$ASSIGN" "$PROMPT" \
         "$SKILLS_DIR/ppt-implement/SKILL.md" \
         "$SKILLS_DIR/ppt-pr-merge/SKILL.md" \
         "$SKILLS_DIR/ppt-review-merged/SKILL.md"; do
  if [ -f "$f" ]; then note "exists: $f"; else fail "missing: $f"; fi
done
echo

# --- T1: assignments.json parses as JSON -----------------------------------
echo "T1  assignments.json parses + top-level shape"
if jq -e '.generated and .assignments' "$ASSIGN" >/dev/null 2>&1; then
  note "valid JSON with .generated and .assignments[]"
else
  fail "assignments.json missing .generated or .assignments[]"
fi
echo

# --- T2: row schema --------------------------------------------------------
echo "T2  every row has required fields (task_id, branch, status, claimed_at, last_updated, status_changed_at)"
MISSING=$(jq -r '
  .assignments
  | map(select(
      (.task_id == null) or
      (.branch  == null) or
      (.status  == null) or
      (.claimed_at == null) or
      (.last_updated == null) or
      (.status_changed_at == null)
    ))
  | length' "$ASSIGN")
if [ "$MISSING" = "0" ]; then note "all rows have core schema"
else fail "$MISSING rows missing one or more required fields"; fi
echo

# --- T3: status enum -------------------------------------------------------
echo "T3  status ∈ {in-progress, review, merged, failed, done}  (done = legacy compat)"
BAD=$(jq -r '
  .assignments
  | map(select((.status as $s | ["in-progress","review","merged","failed","done"] | index($s) | not)))
  | length' "$ASSIGN")
if [ "$BAD" = "0" ]; then note "all status values in allowed set"
else
  fail "$BAD rows with disallowed status"
  jq -r '.assignments[] | select((.status as $s | ["in-progress","review","merged","failed","done"] | index($s) | not)) | "    \(.task_id) :: status=\(.status)"' "$ASSIGN" >&2
fi
echo

# --- T4: no duplicate task_id ----------------------------------------------
echo "T4  task_id unique"
DUPES=$(jq -r '
  .assignments
  | group_by(.task_id)
  | map(select(length>1) | .[0].task_id)
  | join(",")' "$ASSIGN")
if [ -z "$DUPES" ]; then note "all task_id values unique"
else fail "duplicate task_id: $DUPES"; fi
echo

# --- T5: terminal-state discipline -----------------------------------------
# Only enforced for rows claimed on/after the hardening cutoff. Older rows
# pre-date the merged_at field being wired into Phase 2 and were merged via
# legacy logic; we don't rewrite them.
echo "T5  merged rows claimed on/after $HARDENING_DATE must have merged_at AND pr_number"
BAD=$(jq --arg d "$HARDENING_DATE" '
  .assignments
  | map(select(.status == "merged"
               and .claimed_at >= $d
               and ((.merged_at == null) or (.pr_number == null))))
  | length' "$ASSIGN")
if [ "$BAD" = "0" ]; then note "all merged rows in window have merged_at + pr_number"
else
  fail "$BAD merged rows missing merged_at or pr_number (post-cutoff)"
  jq --arg d "$HARDENING_DATE" -r '.assignments[] | select(.status=="merged" and .claimed_at >= $d and ((.merged_at==null) or (.pr_number==null))) | "    \(.task_id) :: claimed=\(.claimed_at) merged_at=\(.merged_at) pr=\(.pr_number)"' "$ASSIGN" >&2
fi
echo

# --- T6: failed rows must not have pr_number set to a still-open PR --------
echo "T6  failed rows: pr_number may be null OR a closed/merged PR (not enforced — informational only)"
FAILED_WITH_PR=$(jq -r '.assignments | map(select(.status=="failed" and .pr_number != null)) | length' "$ASSIGN")
if [ "$FAILED_WITH_PR" = "0" ]; then note "no failed rows carry a pr_number"
else
  printf '  info  %s failed rows carry a pr_number (Phase 2 sets these when PR is CLOSED unmerged)\n' "$FAILED_WITH_PR"
fi
echo

# --- T7: branch naming convention ------------------------------------------
echo "T7  branch starts with auto-impl/"
BAD=$(jq -r '.assignments | map(select((.branch // "") | startswith("auto-impl/") | not)) | length' "$ASSIGN")
if [ "$BAD" = "0" ]; then note "all branches use auto-impl/ prefix"
else
  fail "$BAD rows with non-conforming branch"
  jq -r '.assignments[] | select((.branch // "") | startswith("auto-impl/") | not) | "    \(.task_id) :: branch=\(.branch)"' "$ASSIGN" >&2
fi
echo

# --- T8: status_changed_at <= last_updated ---------------------------------
echo "T8  status_changed_at <= last_updated"
BAD=$(jq -r '
  .assignments
  | map(select(.status_changed_at > .last_updated))
  | length' "$ASSIGN")
if [ "$BAD" = "0" ]; then note "monotonic timestamps OK"
else
  fail "$BAD rows where status_changed_at > last_updated"
  jq -r '.assignments[] | select(.status_changed_at > .last_updated) | "    \(.task_id) :: status_changed_at=\(.status_changed_at) > last_updated=\(.last_updated)"' "$ASSIGN" >&2
fi
echo

# --- T9: dispatcher prompt references all expected skills -------------------
echo "T9  dispatcher-prompt.md references ppt-implement, ppt-review-merged, ppt-pr-merge skills"
for s in ppt-implement ppt-review-merged ppt-pr-merge; do
  if grep -q "$s" "$PROMPT"; then note "references $s"
  else fail "$PROMPT does not reference $s"; fi
done
echo

# --- T10: dispatcher prompt encodes the 7 hardening items -------------------
echo "T10 dispatcher-prompt.md encodes the 2026-05-25 hardening items"
declare -a MARKERS=(
  "item #1"  # empty-branch detection
  "item #2"  # same-epic guard
  "item #3"  # scope drift
  "item #4"  # code reuse
  "item #5"  # JSON key case
  "item #6"  # auto rebase
  "item #7"  # disk preflight
  "item #9"  # always-print hang lines
  "item #10" # reviewer re-run gating
)
for m in "${MARKERS[@]}"; do
  if grep -q "$m" "$PROMPT"; then note "$m present"
  else fail "$m missing from $PROMPT"; fi
done
echo

# --- T11: per-row hardening fields backfilled (allow null, just must exist) -
# Only checks rows claimed AFTER the hardening date; older rows are legacy.
echo "T11 rows claimed on/after $HARDENING_DATE carry hardening fields"
NEW_ROWS=$(jq --arg d "$HARDENING_DATE" '.assignments | map(select(.claimed_at >= $d)) | length' "$ASSIGN")
if [ "$NEW_ROWS" = "0" ]; then
  printf '  skip  no rows claimed on/after %s yet (will check on next dispatcher run)\n' "$HARDENING_DATE"
else
  BAD=$(jq --arg d "$HARDENING_DATE" '
    .assignments
    | map(select(.claimed_at >= $d
                 and ((has("last_reviewed_oid") | not)
                   or (has("scope_drift") | not)
                   or (has("code_reuse_warn") | not)
                   or (has("empty_branch") | not)
                   or (has("rebase_attempts") | not))))
    | length' "$ASSIGN")
  if [ "$BAD" = "0" ]; then note "all $NEW_ROWS new rows carry hardening fields"
  else fail "$BAD new rows missing one or more hardening fields"; fi
fi
echo

# --- T12: gap 1 invariant — reviewer_summary != null => last_reviewed_oid != null
echo "T12 reviewer_summary != null implies last_reviewed_oid != null (gap 1; informational)"
# Informational only. Pre-existing rows in this state are exactly what the
# gap-1 Phase 1 backfill rule targets — they self-heal on the next cycle.
# Becomes a hard fail only for rows created AFTER the gap-1 cutoff.
GAP1_CUTOFF="${GAP1_CUTOFF:-2026-05-27T00:00:00Z}"
INFO=$(jq --arg d "$HARDENING_DATE" '
  .assignments
  | map(select(.claimed_at >= $d
               and .reviewer_summary != null
               and .last_reviewed_oid == null))
  | length' "$ASSIGN")
BAD=$(jq --arg d "$GAP1_CUTOFF" '
  .assignments
  | map(select(.claimed_at >= $d
               and .reviewer_summary != null
               and .last_reviewed_oid == null))
  | length' "$ASSIGN")
if [ "$INFO" = "0" ]; then
  note "all reviewed rows carry last_reviewed_oid"
elif [ "$BAD" = "0" ]; then
  printf '  info  %s pre-gap-1 rows lack last_reviewed_oid (will force re-review next cycle)\n' "$INFO"
else
  fail "$BAD post-gap-1-cutoff rows have reviewer_summary but null last_reviewed_oid"
  jq --arg d "$GAP1_CUTOFF" -r '.assignments[] | select(.claimed_at >= $d and .reviewer_summary != null and .last_reviewed_oid == null) | "    \(.task_id) pr=\(.pr_number)"' "$ASSIGN" >&2
fi
echo

# --- T13: gap 3 — every action-list row has depends_on: array --------------
echo "T13 action-list.json items carry depends_on: array (gap 3)"
ACTION_LIST="${ACTION_LIST:-.research/management/action-list.json}"
if [ -f "$ACTION_LIST" ]; then
  BAD=$(jq -r '
    [.items[] | select((.depends_on | type) != "array")] | length' "$ACTION_LIST")
  if [ "$BAD" = "0" ]; then note "all action-list items carry depends_on array"
  else
    fail "$BAD action-list items missing depends_on or wrong type"
    jq -r '.items[] | select((.depends_on | type) != "array") | "    \(.id) depends_on=\(.depends_on)"' "$ACTION_LIST" >&2
  fi
else
  printf '  skip  %s not found\n' "$ACTION_LIST"
fi
echo

# --- T14: gap 4 — merge_attempted_at is iso-8601 or null --------------------
echo "T14 merge_attempted_at is iso-8601 or null (gap 4)"
BAD=$(jq -r '
  .assignments
  | map(select(
      has("merge_attempted_at")
      and .merge_attempted_at != null
      and ((.merge_attempted_at | type) != "string"
           or (.merge_attempted_at | test("^[0-9]{4}-[0-9]{2}-[0-9]{2}T") | not))
    ))
  | length' "$ASSIGN")
if [ "$BAD" = "0" ]; then note "merge_attempted_at values are well-formed"
else
  fail "$BAD rows with malformed merge_attempted_at"
  jq -r '.assignments[] | select(has("merge_attempted_at") and .merge_attempted_at != null and ((.merge_attempted_at | type) != "string" or (.merge_attempted_at | test("^[0-9]{4}-[0-9]{2}-[0-9]{2}T") | not))) | "    \(.task_id) merge_attempted_at=\(.merge_attempted_at)"' "$ASSIGN" >&2
fi
echo

# --- T15: gap 2 — open_count / open_claimable_count / dep_blocked smoke ----
echo "T15 buffer metrics computable (gap 2 smoke)"
if [ -f "$ACTION_LIST" ]; then
  # Compute open_count = open items not in assignments
  OPEN_COUNT=$(jq --slurpfile a "$ASSIGN" '
    [.items[] | select(.status == "open")
                | .id as $id
                | select(($a[0].assignments | map(.task_id) | index($id)) == null)
    ] | length' "$ACTION_LIST")
  # Compute dep_blocked = of those, the ones whose depends_on has any
  # element NOT pointing at an assignment in {merged, done}.
  DEP_BLOCKED=$(jq --slurpfile a "$ASSIGN" '
    [.items[]
      | select(.status == "open")
      | .id as $id
      | select(($a[0].assignments | map(.task_id) | index($id)) == null)
      | select(
          ((.depends_on // []) | length) > 0
          and any(
            (.depends_on // [])[];
            . as $dep
            | (($a[0].assignments | map(select(.task_id == $dep)) | .[0].status // "missing") | IN("merged","done") | not)
          )
        )
    ] | length' "$ACTION_LIST")
  CLAIMABLE=$((OPEN_COUNT - DEP_BLOCKED))
  note "open_count=$OPEN_COUNT dep_blocked=$DEP_BLOCKED open_claimable=$CLAIMABLE"
else
  printf '  skip  %s not found\n' "$ACTION_LIST"
fi
echo

# --- Summary ---------------------------------------------------------------
if [ "$FAIL" = "0" ]; then
  echo "==> dispatcher-self-test: PASS"
  exit 0
else
  echo "==> dispatcher-self-test: FAIL" >&2
  exit 1
fi
