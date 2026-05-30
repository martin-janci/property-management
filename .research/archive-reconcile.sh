#!/usr/bin/env bash
# Archive-move reconciler — sibling of gc1-reconcile.sh.
#
# Sweeps active assignments.json for rows whose status is terminal
# (merged/failed/done) and moves them to assignments-archive.json
# idempotently. This is the same operation Phase 6 of dispatcher-prompt.md
# performs at the end of every dispatcher run; this script extracts it so
# it can also be invoked at merge-time on a dispatcher-state branch.
#
# Why a standalone script:
#
#   When a stacked dispatcher PR merges `origin/dev` into its branch, dev's
#   newer terminal rows for ids the branch still has as `review` land in
#   the branch's active assignments.json. Phase 6 only sweeps inside a
#   dispatcher run, so the CI self-test (T19: active contains no terminal
#   rows) goes red on the PR before the next dispatcher run gets a chance
#   to clean up. Issue #759 (PR #734 hit this with two leaked rows:
#   gap-82-5-ios-keychain-push-v2 and gap-82-3-search-enhancements-fix).
#
#   Running this script after the dev-merge resolves the leak in one shot
#   and preserves the T17 combined-count invariant (active+archive total
#   unchanged — rows move, never disappear).
#
# Idempotent + re-runnable: after --apply, the moved rows are gone from
# active so they no longer match the sweep set. Concurrent runs that each
# re-archive the same id produce ONE row per task_id in the archive
# (group_by + map(last) keeps the freshest row by file position; same
# behaviour as Phase 6).
#
# Usage:
#   ./.research/archive-reconcile.sh             # dry-run: print move set, write nothing
#   ./.research/archive-reconcile.sh --apply     # actually move terminal rows
#   ASSIGN=… ARCHIVE=… ./.research/archive-reconcile.sh [--apply]

set -euo pipefail

ASSIGN="${ASSIGN:-.research/management/assignments.json}"
ARCHIVE="${ARCHIVE:-.research/management/assignments-archive.json}"
APPLY=0
[ "${1:-}" = "--apply" ] && APPLY=1

if [ ! -f "$ASSIGN" ]; then
  echo "archive-reconcile: active file missing — nothing to do: $ASSIGN" >&2
  exit 0
fi

# Build the move set from CURRENT active file state (sweep semantics — same as
# dispatcher-prompt.md Phase 6). Terminal = status in {merged, failed, done}.
MOVE_IDS_JSON=$(jq '[.assignments[]
  | select(.status=="merged" or .status=="failed" or .status=="done")
  | .task_id]' "$ASSIGN")

MOVE_COUNT=$(echo "$MOVE_IDS_JSON" | jq 'length')
ACTIVE_BEFORE=$(jq '.assignments | length' "$ASSIGN")
ARCHIVE_BEFORE=0
[ -f "$ARCHIVE" ] && ARCHIVE_BEFORE=$(jq '.assignments | length' "$ARCHIVE")
COMBINED_BEFORE=$((ACTIVE_BEFORE + ARCHIVE_BEFORE))

if [ "$MOVE_COUNT" = "0" ]; then
  echo "archive-reconcile: no terminal rows in active (clean)"
  echo "  active=$ACTIVE_BEFORE archive=$ARCHIVE_BEFORE combined=$COMBINED_BEFORE"
  exit 0
fi

echo "archive-reconcile: $MOVE_COUNT terminal row(s) leaked into active"
jq -r --argjson ids "$MOVE_IDS_JSON" '
  .assignments[]
  | select(.task_id as $t | $ids | index($t))
  | "  \(.task_id) :: status=\(.status) pr=\(.pr_number // "-") merged_at=\(.merged_at // "-")"' "$ASSIGN"

if [ "$APPLY" = "0" ]; then
  echo "  (dry-run — pass --apply to move them)"
  exit 0
fi

# Ensure the archive file exists with a valid skeleton before we extend it.
if [ ! -f "$ARCHIVE" ]; then
  echo '{"assignments":[]}' > "$ARCHIVE"
fi

TMP_ARCHIVE=$(mktemp)
TMP_ACTIVE=$(mktemp)
trap 'rm -f "$TMP_ARCHIVE" "$TMP_ACTIVE"' EXIT

# Append matching active rows to archive, then dedup by task_id keeping the
# freshest row (group_by + map(last)). Stamp archived_at as Phase 6 does.
jq --argjson ids "$MOVE_IDS_JSON" --slurpfile active "$ASSIGN" '
  .assignments = (
    (.assignments + [ $active[0].assignments[] | select(.task_id as $t | $ids | index($t)) ])
    | group_by(.task_id)
    | map(last)
  )
  | .archived_at = (now | todate)
' "$ARCHIVE" > "$TMP_ARCHIVE"

# Drop matching rows from active.
jq --argjson ids "$MOVE_IDS_JSON" '
  .assignments |= map(select(.task_id as $t | $ids | index($t) | not))
' "$ASSIGN" > "$TMP_ACTIVE"

# T17 combined-count guard — refuse to write a destructive result.
NEW_ACTIVE=$(jq '.assignments | length' "$TMP_ACTIVE")
NEW_ARCHIVE=$(jq '.assignments | length' "$TMP_ARCHIVE")
NEW_COMBINED=$((NEW_ACTIVE + NEW_ARCHIVE))

# Combined count can only DECREASE if the same id existed in both files before
# the sweep (impossible by T18 + T19 invariants), so a drop is a hard refuse.
# An increase by MOVE_COUNT is also wrong — append happened without dedup.
# Expected: combined stays exactly equal (move, not copy).
if [ "$NEW_COMBINED" -ne "$COMBINED_BEFORE" ]; then
  echo "archive-reconcile: ABORT — combined count changed $COMBINED_BEFORE → $NEW_COMBINED" >&2
  echo "  refusing to write; leaving $ASSIGN and $ARCHIVE untouched." >&2
  exit 2
fi

mv "$TMP_ARCHIVE" "$ARCHIVE"
mv "$TMP_ACTIVE"  "$ASSIGN"
trap - EXIT

echo "archive-reconcile: moved $MOVE_COUNT row(s)"
echo "  active=$ACTIVE_BEFORE → $NEW_ACTIVE  archive=$ARCHIVE_BEFORE → $NEW_ARCHIVE  combined=$COMBINED_BEFORE (unchanged)"
