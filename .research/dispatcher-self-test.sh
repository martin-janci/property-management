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
# Archive split (issue #9). Terminal rows live in a sibling file. If absent
# (pre-split repo state), tests degrade gracefully — they use ASSIGN only.
ASSIGN_ARCHIVE="${ASSIGN_ARCHIVE:-.research/management/assignments-archive.json}"
PROMPT="${DISPATCHER_PROMPT:-.research/dispatcher-prompt.md}"
SKILLS_DIR="${SKILLS_DIR:-.claude/skills}"
# Cutoff for hardening-era checks. Rows claimed before this date are legacy
# (pre-merged_at, pre-hardening-fields) and are exempted from T5 + T11.
HARDENING_DATE="${HARDENING_DATE:-2026-05-25T00:00:00Z}"

# Build the list of files to scan for combined-row checks (T2/T3/T4/T5/T11).
# T7 (non-terminal branch convention) and T16 (ingestion guard) stay
# active-only — terminal rows in the archive are historical and exempt.
ASSIGN_FILES=("$ASSIGN")
if [ -f "$ASSIGN_ARCHIVE" ]; then
  ASSIGN_FILES+=("$ASSIGN_ARCHIVE")
fi

# Cross-file slurp helper: emits one flat array of all rows across active +
# archive (or just active if archive is absent). Use as input to jq filters
# that want to scan every row regardless of which file it lives in.
combined_rows_jq() {
  jq -s '[.[].assignments[]]' "${ASSIGN_FILES[@]}"
}

FAIL=0
note() { printf '  ok    %s\n' "$1"; }
fail() { printf '  FAIL  %s\n' "$1" >&2; FAIL=1; }
# warn() — advisory only: prints but does NOT set FAIL. Used for invariants
# that are self-healing on the next run (e.g. T28 gc1 archive-terminal drift,
# which gc1-reconcile.sh sweeps) so introducing the check doesn't retroactively
# hard-fail an already-drifted dev snapshot. (T26 was formerly a warn of this
# kind but was promoted to fail in #2102 once Phase 6 began running the
# action-list reconcile before the pre-commit self-test gate.)
warn() { printf '  WARN  %s\n' "$1" >&2; }

echo "==> dispatcher-self-test"
echo "    assignments: $ASSIGN"
if [ -f "$ASSIGN_ARCHIVE" ]; then
  echo "    archive:     $ASSIGN_ARCHIVE"
else
  echo "    archive:     (absent — pre-split repo state)"
fi
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
# Combined active + archive — invariant holds regardless of which file holds the row.
MISSING=$(jq -s -r '
  [.[].assignments[]]
  | map(select(
      (.task_id == null) or
      (.branch  == null) or
      (.status  == null) or
      (.claimed_at == null) or
      (.last_updated == null) or
      (.status_changed_at == null)
    ))
  | length' "${ASSIGN_FILES[@]}")
if [ "$MISSING" = "0" ]; then note "all rows have core schema (across ${#ASSIGN_FILES[@]} file(s))"
else fail "$MISSING rows missing one or more required fields"; fi
echo

# --- T3: status enum -------------------------------------------------------
echo "T3  status ∈ {in-progress, review, merged, failed, done, quarantined}  (done = legacy compat; quarantined = semi-terminal, PR 5/5)"
BAD=$(jq -s -r '
  [.[].assignments[]]
  | map(select((.status as $s | ["in-progress","review","merged","failed","done","quarantined"] | index($s) | not)))
  | length' "${ASSIGN_FILES[@]}")
if [ "$BAD" = "0" ]; then note "all status values in allowed set"
else
  fail "$BAD rows with disallowed status"
  jq -s -r '[.[].assignments[]] | .[] | select((.status as $s | ["in-progress","review","merged","failed","done","quarantined"] | index($s) | not)) | "    \(.task_id) :: status=\(.status)"' "${ASSIGN_FILES[@]}" >&2
fi
echo

# --- T4: no duplicate task_id (cross-file — issue #9) ----------------------
echo "T4  task_id unique across active + archive"
DUPES=$(jq -s -r '
  [.[].assignments[]]
  | group_by(.task_id)
  | map(select(length>1) | .[0].task_id)
  | join(",")' "${ASSIGN_FILES[@]}")
if [ -z "$DUPES" ]; then note "all task_id values unique across all files"
else fail "duplicate task_id: $DUPES (a row exists in both active and archive — Phase 6 move bug)"; fi
echo

# --- T5: terminal-state discipline -----------------------------------------
# Only enforced for rows claimed on/after the hardening cutoff. Older rows
# pre-date the merged_at field being wired into Phase 2 and were merged via
# legacy logic; we don't rewrite them.
echo "T5  merged rows claimed on/after $HARDENING_DATE must have merged_at AND pr_number"
BAD=$(jq -s --arg d "$HARDENING_DATE" '
  [.[].assignments[]]
  | map(select(.status == "merged"
               and .claimed_at >= $d
               and ((.merged_at == null) or (.pr_number == null))))
  | length' "${ASSIGN_FILES[@]}")
if [ "$BAD" = "0" ]; then note "all merged rows in window have merged_at + pr_number"
else
  fail "$BAD merged rows missing merged_at or pr_number (post-cutoff)"
  jq -s --arg d "$HARDENING_DATE" -r '[.[].assignments[]] | .[] | select(.status=="merged" and .claimed_at >= $d and ((.merged_at==null) or (.pr_number==null))) | "    \(.task_id) :: claimed=\(.claimed_at) merged_at=\(.merged_at) pr=\(.pr_number)"' "${ASSIGN_FILES[@]}" >&2
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
# Only enforced for non-terminal rows. Merged/done/failed rows are historical
# artifacts — if the dispatcher claimed under the auto-impl/ prefix but the
# implementer used a manually-named branch (fix/…, feat/…) the merge already
# happened and we don't rewrite history.
echo "T7  non-terminal branch starts with auto-impl/"
BAD=$(jq -r '
  .assignments
  | map(select(.status == "in-progress" or .status == "review"))
  | map(select((.branch // "") | startswith("auto-impl/") | not))
  | length' "$ASSIGN")
if [ "$BAD" = "0" ]; then note "all non-terminal branches use auto-impl/ prefix"
else
  fail "$BAD non-terminal rows with non-conforming branch"
  jq -r '.assignments[] | select((.status == "in-progress" or .status == "review") and ((.branch // "") | startswith("auto-impl/") | not)) | "    \(.task_id) :: branch=\(.branch) status=\(.status)"' "$ASSIGN" >&2
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
# Combined active + archive (issue #9).
echo "T11 rows claimed on/after $HARDENING_DATE carry hardening fields"
NEW_ROWS=$(jq -s --arg d "$HARDENING_DATE" '[.[].assignments[]] | map(select(.claimed_at >= $d)) | length' "${ASSIGN_FILES[@]}")
if [ "$NEW_ROWS" = "0" ]; then
  printf '  skip  no rows claimed on/after %s yet (will check on next dispatcher run)\n' "$HARDENING_DATE"
else
  BAD=$(jq -s --arg d "$HARDENING_DATE" '
    [.[].assignments[]]
    | map(select(.claimed_at >= $d
                 and ((has("last_reviewed_oid") | not)
                   or (has("scope_drift") | not)
                   or (has("code_reuse_warn") | not)
                   or (has("empty_branch") | not)
                   or (has("rebase_attempts") | not))))
    | length' "${ASSIGN_FILES[@]}")
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

# --- T16: branch-prefix guard (issue #573) ---------------------------------
# Hard fail for any non-terminal row (in-progress or review) that does NOT
# carry an auto-impl/ prefix -- T7 already covers this but T16 also checks
# newly-ingested terminal rows (merged/failed) claimed on/after the guard
# cutoff, where the ingestion guard should have prevented the bad row from
# ever being written.
#
# Terminal rows (merged/failed/done) with claimed_at < T16_GUARD_CUTOFF are
# historical manual-PR rows added before the guard existed -- reported as info
# but not a hard fail.
#
# T16_GUARD_CUTOFF is the date the ingestion guard was introduced (this PR,
# 2026-05-27). Only rows claimed by the dispatcher AFTER this date are held
# to the auto-impl/ contract.
T16_GUARD_CUTOFF="${T16_GUARD_CUTOFF:-2026-05-27T18:00:00Z}"
echo "T16 branch starts with auto-impl/ -- ingestion prefix guard (issue #573; guard cutoff=$T16_GUARD_CUTOFF)"
# Hard fail: non-terminal rows with wrong prefix (all dates).
BAD_NONTERMINAL=$(jq -r '
  .assignments
  | map(select(
      (.status == "in-progress" or .status == "review")
      and ((.branch // "") | startswith("auto-impl/") | not)
    ))
  | length' "$ASSIGN")
if [ "$BAD_NONTERMINAL" = "0" ]; then
  note "no non-terminal rows with non-auto-impl/ branch"
else
  fail "$BAD_NONTERMINAL non-terminal rows ingested with non-auto-impl/ branch (ingestion guard violated -- issue #573)"
  jq -r '.assignments[] | select((.status == "in-progress" or .status == "review") and ((.branch // "") | startswith("auto-impl/") | not)) | "    \(.task_id) :: branch=\(.branch) status=\(.status)"' "$ASSIGN" >&2
fi
# Hard fail: terminal rows claimed on/after the guard cutoff with wrong prefix.
BAD_TERMINAL=$(jq --arg d "$T16_GUARD_CUTOFF" '
  .assignments
  | map(select(
      (.status | IN("merged","failed","done"))
      and .claimed_at >= $d
      and ((.branch // "") | startswith("auto-impl/") | not)
    ))
  | length' "$ASSIGN")
if [ "$BAD_TERMINAL" = "0" ]; then
  note "no post-guard-cutoff terminal rows with non-auto-impl/ branch"
else
  fail "$BAD_TERMINAL post-guard-cutoff terminal rows carry non-auto-impl/ branch (should have been blocked at ingestion)"
  jq --arg d "$T16_GUARD_CUTOFF" -r '.assignments[] | select((.status | IN("merged","failed","done")) and .claimed_at >= $d and ((.branch // "") | startswith("auto-impl/") | not)) | "    \(.task_id) :: branch=\(.branch) status=\(.status) claimed_at=\(.claimed_at)"' "$ASSIGN" >&2
fi
# Informational: historical terminal rows (pre-guard-cutoff) with non-auto-impl/ branch.
INFO_LEGACY=$(jq --arg d "$T16_GUARD_CUTOFF" '
  .assignments
  | map(select(
      (.status | IN("merged","failed","done"))
      and .claimed_at < $d
      and ((.branch // "") | startswith("auto-impl/") | not)
    ))
  | length' "$ASSIGN")
if [ "$INFO_LEGACY" -gt "0" ]; then
  printf '  info  %s historical terminal rows carry non-auto-impl/ branch (pre-guard cutoff; manual PRs exempt)\n' "$INFO_LEGACY"
fi
echo

# --- T17: row-count regression guard (issue #8, adapted for split — #9) ---
# Combined active + archive count should not drop by more than 2 vs HEAD.
# Catches the stale-read → Edit race (commit 4def18ce, 118→2 rows). Rows
# legitimately move from active to archive during Phase 6, so we must
# compare the COMBINED total — not per-file — to avoid false alarms.
echo "T17 combined assignments row count (active + archive) vs HEAD (issue #8 — destructive-write guard)"
if git rev-parse --is-inside-work-tree >/dev/null 2>&1 \
   && git cat-file -e "HEAD:$ASSIGN" 2>/dev/null; then
  NEW_ACTIVE=$(jq '.assignments | length' "$ASSIGN")
  NEW_ARCH=0
  if [ -f "$ASSIGN_ARCHIVE" ]; then
    NEW_ARCH=$(jq '.assignments | length' "$ASSIGN_ARCHIVE")
  fi
  NEW_COMBINED=$((NEW_ACTIVE + NEW_ARCH))
  OLD_ACTIVE=$(git show "HEAD:$ASSIGN" | jq '.assignments | length' 2>/dev/null || echo 0)
  OLD_ARCH=0
  if git cat-file -e "HEAD:$ASSIGN_ARCHIVE" 2>/dev/null; then
    OLD_ARCH=$(git show "HEAD:$ASSIGN_ARCHIVE" | jq '.assignments | length' 2>/dev/null || echo 0)
  fi
  OLD_COMBINED=$((OLD_ACTIVE + OLD_ARCH))
  if [ "$NEW_COMBINED" -lt "$((OLD_COMBINED - 2))" ]; then
    fail "combined row count regressed: HEAD=$OLD_COMBINED current=$NEW_COMBINED (loss > 2; active=$OLD_ACTIVE→$NEW_ACTIVE, archive=$OLD_ARCH→$NEW_ARCH)"
  else
    note "combined row count OK: HEAD=$OLD_COMBINED current=$NEW_COMBINED (active=$NEW_ACTIVE, archive=$NEW_ARCH)"
  fi
else
  printf '  skip  not in git tree or HEAD missing %s\n' "$ASSIGN"
fi
echo

# --- T18: archive contains only terminal rows (issue #9) -------------------
# Invariant of the active/archive split: archive holds merged/failed/done ONLY.
# Any in-progress or review row in archive means Phase 6 archived prematurely.
if [ -f "$ASSIGN_ARCHIVE" ]; then
  echo "T18 archive contains only terminal rows (issue #9)"
  BAD=$(jq -r '
    .assignments
    | map(select(.status != "merged" and .status != "failed" and .status != "done"))
    | length' "$ASSIGN_ARCHIVE")
  if [ "$BAD" = "0" ]; then note "all archive rows are terminal"
  else
    fail "$BAD archive rows with non-terminal status"
    jq -r '.assignments[] | select(.status != "merged" and .status != "failed" and .status != "done") | "    \(.task_id) :: status=\(.status)"' "$ASSIGN_ARCHIVE" >&2
  fi
  echo
fi

# --- T18b: archive contains AT MOST ONE row per task_id --------------------
# Phase 6 archive write must upsert by task_id. Concurrent runs that each
# computed the same move set used to append duplicate rows into the archive
# (T4 caught it cross-file; T18b is the in-archive invariant proper). Plain
# append → duplicate rows; group_by | map(last) → idempotent upsert.
if [ -f "$ASSIGN_ARCHIVE" ]; then
  echo "T18b archive has at most one row per task_id (in-archive dedup)"
  DUPES=$(jq -r '
    [.assignments | group_by(.task_id) | .[] | select(length>1) | .[0].task_id]
    | length' "$ASSIGN_ARCHIVE")
  if [ "$DUPES" = "0" ]; then note "no duplicate task_id rows in archive"
  else
    fail "$DUPES task_id(s) duplicated within archive (Phase 6 archive-append bug)"
    jq -r '.assignments | group_by(.task_id) | .[] | select(length>1) | "    \(.[0].task_id) :: \(length) rows"' "$ASSIGN_ARCHIVE" >&2
  fi
  echo
fi

# --- T19: active contains NO terminal rows (issue #9) ----------------------
# Inverse of T18: terminal rows must be moved to archive in Phase 6.
echo "T19 active assignments contains NO terminal rows (issue #9)"
BAD=$(jq -r '
  .assignments
  | map(select(.status == "merged" or .status == "failed" or .status == "done"))
  | length' "$ASSIGN")
if [ "$BAD" = "0" ]; then note "no terminal rows leaked into active"
else
  fail "$BAD terminal rows still in active assignments.json (Phase 6 archive-move bug)"
  jq -r '.assignments[] | select(.status=="merged" or .status=="failed" or .status=="done") | "    \(.task_id) :: status=\(.status)"' "$ASSIGN" >&2
fi
echo

# --- T20: stem-uniqueness among active rows (closes #641/#644 double-land) ---
# At most one non-terminal (in-progress|review) row per stem(task_id), where
# stem strips the auto-impl/|impl/ prefix and a trailing -(impl|fix|v2|retry|
# followup|wip)<digits> suffix. Two active rows sharing a stem = duplicate work.
echo "T20 stem-uniqueness among active (in-progress|review) rows"
T20_DUPES=$(jq -r '
  def stem: sub("^(auto-impl|impl)/";"") | sub("-(impl|fix|v2|retry|followup|wip)[0-9]*$";"");
  .assignments
  | map(select(.status=="in-progress" or .status=="review"))
  | group_by(.task_id | stem)
  | map(select(length>1) | (.[0].task_id | stem))
  | join(",")' "$ASSIGN")
if [ -z "$T20_DUPES" ]; then note "no two active rows share a stem"
else
  # Duplicate active work is a data-integrity violation (the #641/#644
  # double-land class). Enforcing as of PR 2.
  fail "duplicate active stems: $T20_DUPES (two non-terminal units of the same work)"
  jq -r '
    def stem: sub("^(auto-impl|impl)/";"") | sub("-(impl|fix|v2|retry|followup|wip)[0-9]*$";"");
    .assignments | map(select(.status=="in-progress" or .status=="review"))
    | group_by(.task_id | stem) | map(select(length>1))[] | .[]
    | "    \(.task_id) :: stem=\(.task_id|stem) status=\(.status)"' "$ASSIGN" >&2
fi
echo

# --- T21: objective.json sanity (PR 3/5 finish-first) ----------------------
# When DISPATCHER_FINISH_FIRST=1, the dispatcher reads/writes
# .research/management/objective.json. T21 enforces:
#   * if the file exists, it's valid JSON with the expected schema
#   * epic_prefix is a non-empty string and either matches one of the
#     epic-prefix regexes used by the dispatcher (gap-N[a-z]? or pm-<role>)
#     OR equals "NONE" (the picker's "no claimable work" sentinel).
# Soft-fail: if the file doesn't exist, T21 is a no-op (the picker
# creates it on first finish-first run; absence is the default state).
OBJECTIVE_FILE="${OBJECTIVE_FILE:-.research/management/objective.json}"
if [ -f "$OBJECTIVE_FILE" ]; then
  echo "T21 objective.json schema + epic_prefix shape (PR 3/5)"
  if ! jq -e '.schema_version and .epic_prefix and .selected_at' "$OBJECTIVE_FILE" >/dev/null 2>&1; then
    fail "$OBJECTIVE_FILE missing one of {schema_version, epic_prefix, selected_at}"
  else
    EP=$(jq -r '.epic_prefix' "$OBJECTIVE_FILE")
    case "$EP" in
      NONE) note "objective=NONE (no claimable work)";;
      gap-[0-9]*|pm-*) note "objective epic_prefix='$EP' is well-formed";;
      "") fail "objective.epic_prefix is empty";;
      # The dispatcher's epic_prefix() else-branch returns the full task_id,
      # so the finish-first picker legitimately writes a flat kebab id here
      # (feat-*/test-*/refactor-*/screen-*/triage-issue-* …). Accept any
      # non-empty lowercase kebab token; reject malformed shapes (uppercase,
      # spaces, leading separator) that signal a corrupt objective.json.
      *)
        if printf '%s' "$EP" | grep -Eq '^[a-z0-9][a-z0-9._-]*$'; then
          note "objective epic_prefix='$EP' is a full task_id (epic_prefix else-branch)"
        else
          fail "objective.epic_prefix='$EP' is not NONE, gap-N[a-z]?, pm-<role>, or a kebab task_id"
        fi
        ;;
    esac
  fi
  echo
fi

# --- T22: WIP throttle bounds (PR 4/5) -------------------------------------
# Soft invariant: WIP (count of {in-progress, review} rows) should stay
# within DISPATCHER_WIP_CAP. State on disk may already be over cap when the
# throttle is first enabled, so being-over-cap is a `note`, not a `fail`.
# Hard-fail only when WIP exceeds 2 × cap (clear runaway).
echo "T22 WIP throttle bounds (PR 4/5)"
WIP_CAP_FOR_TEST="${DISPATCHER_WIP_CAP:-16}"
WIP_NOW=$(jq '[.assignments[] | select(.status=="in-progress" or .status=="review")] | length' "$ASSIGN")
if [ "$WIP_CAP_FOR_TEST" = "0" ]; then
  note "WIP throttle disabled (DISPATCHER_WIP_CAP=0); current WIP=$WIP_NOW"
elif [ "$WIP_NOW" -gt $(( WIP_CAP_FOR_TEST * 2 )) ]; then
  fail "WIP=$WIP_NOW exceeds 2× cap (cap=$WIP_CAP_FOR_TEST) — clear runaway, merge throughput collapsed"
elif [ "$WIP_NOW" -gt "$WIP_CAP_FOR_TEST" ]; then
  note "WIP=$WIP_NOW over cap=$WIP_CAP_FOR_TEST (drain by merges; Phase 3 will claim 0 until under cap)"
else
  note "WIP=$WIP_NOW within cap=$WIP_CAP_FOR_TEST"
fi
echo

# --- T23: quarantine invariants (PR 5/5) -----------------------------------
# Every quarantined row MUST have quarantined_at set AND either
# fix_rounds >= 3 OR a non-null quarantine_reason. Hard-fail on violation.
echo "T23 quarantine invariants (PR 5/5)"
BAD23=$(jq -r '
  [ .assignments[] | select(.status == "quarantined")
    | select((.quarantined_at == null) or
             (((.fix_rounds // 0) < 3) and ((.quarantine_reason // null) == null)))
    | .task_id ] | length' "$ASSIGN")
if [ "$BAD23" = "0" ]; then
  note "all quarantined rows have quarantined_at + (fix_rounds>=3 OR reason)"
else
  fail "$BAD23 quarantined rows missing quarantined_at or fix_rounds/reason"
  jq -r '.assignments[]
    | select(.status == "quarantined")
    | select((.quarantined_at == null) or
             (((.fix_rounds // 0) < 3) and ((.quarantine_reason // null) == null)))
    | "    \(.task_id) quarantined_at=\(.quarantined_at) fix_rounds=\(.fix_rounds // 0) reason=\(.quarantine_reason // "null")"' "$ASSIGN" >&2
fi
echo

# --- T24: action-list stem-uniqueness (PR 5/5 claim-time dedup) ------------
# At most one OPEN action-list item per stem. Suffix variants (-impl, -v2,
# -fix, etc.) sharing a stem indicate dup work that bypassed the routine's
# promotion-time stem guard.
if [ -f "$ACTION_LIST" ]; then
  echo "T24 action-list.json: at most one OPEN item per stem (PR 5/5)"
  DUPES24=$(jq -r '
    # Canonical stem(): suffix-strip only — same definition as routine-prompt.md
    # line ~670, dispatcher-prompt.md Phase 3, and ppt-pr-create Step 3.5.
    # Keep these in sync (no branch-prefix strip — action-list IDs are bare slugs).
    def stem: sub("-(impl|fix|v2|retry|followup|wip)[0-9]*$";"");
    [ .items[] | select(.status == "open") | (.id | stem) ]
    | group_by(.) | map(select(length > 1)) | length' "$ACTION_LIST")
  if [ "$DUPES24" = "0" ]; then
    note "no two open action-list items share a stem"
  else
    fail "$DUPES24 stem collision(s) among open action-list items"
    jq -r '
      def stem: sub("-(impl|fix|v2|retry|followup|wip)[0-9]*$";"");
      [ .items[] | select(.status == "open") | {id, s: (.id|stem)} ]
      | group_by(.s) | map(select(length > 1))[] | .[]
      | "    \(.id) :: stem=\(.s)"' "$ACTION_LIST" >&2
  fi
  echo
fi

# --- T25: unparseable-legacy-dep guard (issue #583) -----------------------
# When an action-list item carries a non-empty, non-"none" legacy `dependency`
# free-text field, `depends_on` MUST NOT be empty -- it must either contain
# real task_id(s) (when parseable) or a poisoned sentinel
# `["UNRESOLVED:<truncated>"]` (when unparseable). An empty `depends_on` with a
# meaningful legacy `dependency` value silently makes the item claimable while
# its real dependency is unresolved (the gap-3 migration trap; PR #562 fallout).
if [ -f "$ACTION_LIST" ]; then
  echo "T25 action-list items: non-empty legacy 'dependency' => depends_on non-empty (issue #583)"
  BAD25=$(jq -r '
    [ .items[]
      | select(((.dependency // "") | ascii_downcase) as $d
               | ($d != "" and $d != "none"))
      | select(((.depends_on // []) | length) == 0)
    ] | length' "$ACTION_LIST")
  if [ "$BAD25" = "0" ]; then
    note "no action-list rows with non-empty legacy dependency and empty depends_on"
  else
    fail "$BAD25 action-list rows have legacy 'dependency' set but empty depends_on (gap-3 migration must emit poisoned sentinel \"UNRESOLVED:<text>\" -- issue #583)"
    jq -r '
      .items[]
      | select(((.dependency // "") | ascii_downcase) as $d
               | ($d != "" and $d != "none"))
      | select(((.depends_on // []) | length) == 0)
      | "    \(.id) :: dependency=\(.dependency) depends_on=[]"' "$ACTION_LIST" >&2
  fi
  echo
fi

# --- T26: action-list.json holds non-terminal items only (issue #1014) ------
# Spec (dispatcher-prompt.md Phase 1 step 4): action-list.json carries only
# open/in-progress items; done/dropped live in action-list-archive.json. Bloat
# from un-archived terminal rows is what pushed the file past the MCP inline
# push limit and corrupted it on dev. ENFORCED (fail) as of #2102: Phase 6 now
# runs `bash .research/action-list-reconcile.sh --apply` BEFORE the pre-commit
# self-test gate, so a clean action-list is guaranteed on the commit path. A
# terminal row surviving to this check therefore means the reconcile was
# skipped or failed — a real invariant breach that must block the MCP push
# (a bloated action-list.json is the #1014 truncation vector), not transient
# self-healing bloat. Promoting warn→fail is only safe BECAUSE of that wiring.
echo "T26 action-list.json holds non-terminal items only (issue #1014; enforced)"
if [ -f "$ACTION_LIST" ]; then
  TERM=$(jq -r '[.items[] | select(.status=="done" or .status=="dropped" or .status=="merged" or .status=="failed")] | length' "$ACTION_LIST")
  if [ "$TERM" = "0" ]; then note "no terminal items in action-list.json (archive split clean)"
  else
    fail "$TERM terminal item(s) in action-list.json — run: bash .research/action-list-reconcile.sh --apply"
  fi
else
  printf '  skip  %s not found\n' "$ACTION_LIST"
fi
echo

# --- T27: merge-path subagent prompt extraction integrity (PAP-164) --------
# PAP-164 moved the verbatim Phase 5 / 5.4 / 5.6 / 5.7 subagent prompts out of
# dispatcher-prompt.md into .research/management/*-prompt.md (token economy).
# These sit on the merge-decision hot path, so the extraction MUST stay intact:
#   (a) each extracted file exists and carries its anchor strings (a truncated,
#       empty, or corrupted file would silently strip the subagent's rubric —
#       exactly the failure a live dispatcher dry-run would otherwise catch); AND
#   (b) dispatcher-prompt.md still POINTS at each file (a spawn site that lost
#       its pointer would spawn a subagent with no instructions).
# Hard-fail on any violation — this is the merge hot path.
echo "T27 merge-path subagent prompt extraction integrity (PAP-164)"
MGMT_DIR="${MGMT_DIR:-.research/management}"
# file :: anchor1|anchor2|anchor3  (all anchors must be present, grep -F literal)
declare -a T27_SPECS=(
  "pr-reviewer-prompt.md::You are a code reviewer::verdict=<approve|changes>::head_oid"
  "premerge-autofix-prompt.md::pre-merge mechanical autofixer::premerge=<applied|skipped|failed>::ppt-pr-merge"
  "rebaser-prompt.md::PR rebaser::rebased=<true|false>"
  "pr-followup-prompt.md::PR follow-up driver::ppt-pr-followup::followup="
)
for spec in "${T27_SPECS[@]}"; do
  f="${spec%%::*}"
  rest="${spec#*::}"
  fpath="$MGMT_DIR/$f"
  if [ ! -f "$fpath" ]; then fail "missing extracted prompt: $fpath"; continue; fi
  if ! grep -qF -- "$f" "$PROMPT"; then
    fail "dispatcher-prompt.md no longer points at $f (spawn site lost its pointer)"
  fi
  ok_anchors=1
  while [ -n "$rest" ]; do
    anchor="${rest%%::*}"
    if [ "$anchor" = "$rest" ]; then rest=""; else rest="${rest#*::}"; fi
    if ! grep -qF -- "$anchor" "$fpath"; then
      fail "$fpath missing anchor: '$anchor' (extraction corrupted/truncated)"
      ok_anchors=0
    fi
  done
  [ "$ok_anchors" = "1" ] && note "$f intact + referenced by dispatcher-prompt.md"
done
echo

# --- T28: no OPEN action-list item is already terminal in the archive -------
# (issue #1747, #1739) The gc1-reconcile.sh archive-terminal pass closes any
# OPEN action-list item whose exact id is terminal (merged/done/failed) in
# assignments-archive.json. If any survive, they inflate the claimable pool
# and the claim predicate can near-re-claim already-merged work (only T4 caught
# it before). Advisory (warn, not fail) because gc1-reconcile.sh --apply is
# self-healing on the next Phase 2.6 — but it surfaces the leak every run.
echo "T28 no OPEN action-list item is terminal in the archive (issue #1747/#1739; advisory)"
if [ -f "$ACTION_LIST" ] && [ -f "$ASSIGN_ARCHIVE" ]; then
  LEAK28=$(jq -n --slurpfile al "$ACTION_LIST" --slurpfile arc "$ASSIGN_ARCHIVE" '
    ($arc[0].assignments
      | map(select(.status=="merged" or .status=="done" or .status=="failed"))
      | map(.task_id) | unique) as $term
    | [ $al[0].items[] | select(.status=="open") | select(.id as $i | $term | index($i)) | .id ]')
  N28=$(jq 'length' <<<"$LEAK28")
  if [ "$N28" = "0" ]; then note "no open action-list item is terminal in the archive"
  else
    warn "$N28 open action-list item(s) already terminal in archive — run: bash .research/gc1-reconcile.sh --apply"
    jq -r '.[] | "    \(.)"' <<<"$LEAK28" >&2
  fi
else
  printf '  skip  %s or %s not found\n' "$ACTION_LIST" "$ASSIGN_ARCHIVE"
fi
echo

# --- T29: anti-starvation wiring (aging + backlog self-refill; 2026-07-02) --
# Two starvation fixes must stay encoded and well-formed:
#   (a) every action-list item's first_open_at is iso-8601 or null/absent
#       (the Phase 3 aging term reads it; a malformed value would mis-age);
#   (b) the prompt encodes the aging boost AND the Tier-1b backlog self-refill,
#       and the backlog-refill.sh helper exists and is executable.
echo "T29 anti-starvation wiring: first_open_at well-formed + aging/backlog-refill encoded"
if [ -f "$ACTION_LIST" ]; then
  BAD29=$(jq -r '
    [ .items[]
      | select(has("first_open_at") and .first_open_at != null
               and ((.first_open_at | type) != "string"
                    or (.first_open_at | test("^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}([.][0-9]+)?Z$") | not))) ]
    | length' "$ACTION_LIST")
  if [ "$BAD29" = "0" ]; then note "all first_open_at values are iso-8601 or null"
  else
    fail "$BAD29 action-list item(s) with malformed first_open_at"
    jq -r '.items[] | select(has("first_open_at") and .first_open_at != null and ((.first_open_at|type)!="string" or (.first_open_at|test("^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}([.][0-9]+)?Z$")|not))) | "    \(.id) first_open_at=\(.first_open_at)"' "$ACTION_LIST" >&2
  fi
else
  printf '  skip  %s not found\n' "$ACTION_LIST"
fi
PROMPT="${PROMPT:-.research/dispatcher-prompt.md}"
if [ -f "$PROMPT" ]; then
  grep -q 'Anti-starvation aging' "$PROMPT"     && note "prompt encodes Phase 3 aging" || fail "prompt missing Phase 3 aging block"
  grep -q 'backlog-refill.sh' "$PROMPT"          && note "prompt wires Tier-1b backlog-refill" || fail "prompt missing Tier-1b backlog-refill wiring"
fi
REFILL="${REFILL:-.research/backlog-refill.sh}"
if [ -x "$REFILL" ]; then note "backlog-refill.sh present + executable"
else fail "backlog-refill.sh missing or not executable ($REFILL)"; fi
echo

# --- T30: open-issue ingestion wiring (2026-07-02 — get things done) --------
# gh-issue-<N> rows must carry issue_ref.number (drives the post-merge MCP
# close); the prompt must encode the ingest + explicit-close loop; the helper
# must exist + be executable.
echo "T30 issue-ingest wiring: gh-issue-<N> rows carry issue_ref + ingest/close encoded"
if [ -f "$ACTION_LIST" ]; then
  BAD30=$(jq -r '
    [ .items[]
      | select(.id | type=="string" and startswith("gh-issue-"))
      | select((.issue_ref // {} | .number | type) != "number") ]
    | length' "$ACTION_LIST")
  if [ "$BAD30" = "0" ]; then note "all gh-issue-<N> rows carry issue_ref.number"
  else
    fail "$BAD30 gh-issue-<N> row(s) missing issue_ref.number"
    jq -r '.items[] | select(.id|type=="string" and startswith("gh-issue-")) | select((.issue_ref//{}|.number|type)!="number") | "    \(.id)"' "$ACTION_LIST" >&2
  fi
else
  printf '  skip  %s not found\n' "$ACTION_LIST"
fi
if [ -f "$PROMPT" ]; then
  grep -q 'issue-ingest.sh' "$PROMPT"        && note "prompt wires open-issue ingestion" || fail "prompt missing issue-ingest wiring"
  grep -q 'mcp__github__update_issue' "$PROMPT" && note "prompt encodes explicit issue-close loop" || fail "prompt missing post-merge issue-close loop"
fi
INGEST="${INGEST:-.research/issue-ingest.sh}"
if [ -x "$INGEST" ]; then note "issue-ingest.sh present + executable"
else fail "issue-ingest.sh missing or not executable ($INGEST)"; fi
echo

# --- T31: supply-chain hardening wiring (2026-07-07) ------------------------
# Four starvation/stall fixes must stay encoded:
#   (a) backlog-reconcile.sh + retry-remint.sh exist, are executable, and the
#       prompt wires them (Phase 2.6 backlog-honesty pass + Tier 1c);
#   (b) the prompt encodes the Phase 5.8 merge-confirm sweep and the Phase 2
#       escalation SLA (stale red reviews + quarantine exit);
#   (c) every retry_of-carrying action-list row is well-formed: retry_of is a
#       string, retry_round a number >= 1, and the id ends in -retry<N>
#       (the suffix is what satisfies the exact-id archive exclusion);
#   (d) an OPEN retry row's ORIGINAL (retry_of) must NOT be merged/done in the
#       archive — retrying landed work is duplicate work.
echo "T31 supply-chain hardening: reconcile/retry/merge-confirm/SLA wiring"
for s in backlog-reconcile.sh retry-remint.sh; do
  if [ -x ".research/$s" ]; then note "$s present + executable"
  else fail "$s missing or not executable"; fi
done
if [ -f "$PROMPT" ]; then
  grep -q 'backlog-reconcile.sh' "$PROMPT" && note "prompt wires backlog-honesty pass" || fail "prompt missing backlog-reconcile wiring"
  grep -q 'retry-remint.sh' "$PROMPT"      && note "prompt wires Tier 1c retry re-mint" || fail "prompt missing retry-remint wiring"
  grep -q 'Phase 5.8 — Merge-confirm sweep' "$PROMPT" && note "prompt encodes merge-confirm sweep" || fail "prompt missing Phase 5.8 merge-confirm sweep"
  grep -q 'quarantine escalation SLA' "$PROMPT" && note "prompt encodes escalation SLA" || fail "prompt missing escalation SLA"
fi
if [ -f "$ACTION_LIST" ]; then
  BAD31=$(jq -r '
    [ .items[]
      | select(has("retry_of"))
      | select((.retry_of | type) != "string"
               or ((.retry_round // 0) | type) != "number"
               or (.retry_round // 0) < 1
               or ((.id | type) != "string")
               or ((.id | test("-retry[0-9]+$")) | not)) ]
    | length' "$ACTION_LIST")
  if [ "$BAD31" = "0" ]; then note "all retry_of rows well-formed"
  else fail "$BAD31 malformed retry_of row(s) in action-list"; fi
  if [ -f "$ASSIGN_ARCHIVE" ]; then
    BAD31b=$(jq -n --slurpfile al "$ACTION_LIST" --slurpfile arc "$ASSIGN_ARCHIVE" '
      ([ $arc[0].assignments[]? | select(.status=="merged" or .status=="done") | .task_id ] | unique) as $landed
      | [ $al[0].items[]
          | select(has("retry_of") and .status=="open")
          | select(.retry_of as $o | $landed | index($o)) ]
      | length')
    if [ "$BAD31b" = "0" ]; then note "no open retry row targets landed work"
    else fail "$BAD31b open retry row(s) whose original already merged/done"; fi
  fi
fi
echo

# --- T32: mobile-native/KMP claim-time gate wiring (2026-08-04, issue #2652) -
# The cloud runner cannot verify any `mobile-native/` change (the `./gradlew`
# verify gate 403s on AGP from dl.google.com), so such candidates must be
# skipped at CLAIM time — the analogue of the Phase 5.5 awaiting-macos-build PR
# gate. Assert three things stay encoded:
#   (a) the helper + its smoke test exist and are executable;
#   (b) the prompt wires the helper into the Phase 3 claim predicate AND
#       surfaces gated items in Phase 7 (Mobile-native gated: line);
#   (c) the helper is behaviorally correct on a canonical id (positive) and a
#       React-Native id (negative) — a cheap invariant guard so a future edit
#       to the id-token regex can't silently start gating (or stop gating) the
#       wrong thing.
echo "T32 mobile-native gate wiring: helper present + Phase 3/7 encoded + predicate sane"
MNGATE="${MNGATE:-.research/mobile-native-gate.sh}"
if [ -x "$MNGATE" ]; then note "mobile-native-gate.sh present + executable"
else fail "mobile-native-gate.sh missing or not executable ($MNGATE)"; fi
if [ -x ".research/test-mobile-native-gate.sh" ]; then note "test-mobile-native-gate.sh present + executable"
else fail "test-mobile-native-gate.sh missing or not executable"; fi
if [ -f "$PROMPT" ]; then
  grep -q 'mobile-native-gate.sh' "$PROMPT"        && note "prompt wires Phase 3 mobile-native gate" || fail "prompt missing mobile-native-gate wiring"
  grep -q 'MOBILE_NATIVE_GATED' "$PROMPT"          && note "prompt excludes gated ids from candidates" || fail "prompt missing MOBILE_NATIVE_GATED claim exclusion"
  grep -q 'Mobile-native gated:' "$PROMPT"         && note "prompt surfaces Phase 7 Mobile-native gated line" || fail "prompt missing Phase 7 Mobile-native gated line"
fi
if [ -x "$MNGATE" ]; then
  if bash "$MNGATE" --id code-review-mobile-native-kmp-foo >/dev/null 2>&1; then
    note "predicate gates a canonical mobile-native id"
  else fail "predicate did NOT gate code-review-mobile-native-kmp-foo"; fi
  if bash "$MNGATE" --id code-review-mobile-rn-report-fault >/dev/null 2>&1; then
    fail "predicate wrongly gated a mobile-rn (React Native) id"
  else note "predicate does NOT gate mobile-rn (React Native) ids"; fi
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
