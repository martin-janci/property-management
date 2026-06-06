#!/usr/bin/env bash
# action-list archive-move reconciler — sibling of archive-reconcile.sh.
#
# Sweeps active action-list.json for items whose status is terminal
# (done/dropped) and moves them to action-list-archive.json idempotently.
#
# Why this script exists (issue #1014):
#
#   dispatcher-prompt.md (Phase 1, step 4) mandates that action-list.json
#   hold NON-TERMINAL items only (status ∈ {open, in-progress}); done/dropped
#   items are supposed to live in action-list-archive.json. But unlike the
#   assignments split — which has archive-reconcile.sh enforcing the move
#   every Phase 6 — there was no equivalent reconciler for action-list, so
#   terminal rows accumulated in the live file (snapshot 2026-06-06: 25
#   terminal rows out of 82; the issue reports ~66 accreting over time).
#
#   The bloat is the ROOT CAUSE of issue #1014: when `git push` is HTTP-403'd
#   by the sandbox proxy, Phase 6 falls back to the GitHub MCP push_files
#   tool, which requires the full file content INLINE in the tool call. A
#   ~75-92 KB literal truncates on emission, so the file lands on dev as a
#   1-2 item stub — silent corruption. Keeping action-list.json small (only
#   live, non-terminal work) keeps it well inside the inline-push limit and
#   removes the corruption vector at the source.
#
# Idempotent + re-runnable: after --apply the moved items are gone from the
# active file so they no longer match the sweep set. Re-running on a clean
# state is a no-op. Concurrent runs that each re-archive the same id produce
# ONE row per id (group_by + map(last) keeps the freshest by file position).
#
# Combined-count guard (mirrors archive-reconcile.sh T17): the script refuses
# to write if the combined active+archive item count would change — items
# MOVE, they never disappear. This makes a buggy sweep fail closed instead of
# destroying backlog.
#
# Usage:
#   ./.research/action-list-reconcile.sh             # dry-run: print move set, write nothing
#   ./.research/action-list-reconcile.sh --apply     # actually move terminal items
#   ACTION_LIST=… ACTION_ARCHIVE=… ./.research/action-list-reconcile.sh [--apply]

set -euo pipefail

ACTION_LIST="${ACTION_LIST:-.research/management/action-list.json}"
ACTION_ARCHIVE="${ACTION_ARCHIVE:-.research/management/action-list-archive.json}"
APPLY=0
[ "${1:-}" = "--apply" ] && APPLY=1

if [ ! -f "$ACTION_LIST" ]; then
  echo "action-list-reconcile: active file missing — nothing to do: $ACTION_LIST" >&2
  exit 0
fi

# Build the move set from CURRENT active file state (sweep semantics — same as
# archive-reconcile.sh). Terminal = status in {done, dropped}. (open/in-progress
# stay; merged/failed are not action-list statuses but are tolerated as terminal
# for forward-compat.)
MOVE_IDS_JSON=$(jq '[.items[]
  | select(.status=="done" or .status=="dropped" or .status=="merged" or .status=="failed")
  | .id]' "$ACTION_LIST")

MOVE_COUNT=$(echo "$MOVE_IDS_JSON" | jq 'length')
ACTIVE_BEFORE=$(jq '.items | length' "$ACTION_LIST")
ARCHIVE_BEFORE=0
[ -f "$ACTION_ARCHIVE" ] && ARCHIVE_BEFORE=$(jq '.items | length' "$ACTION_ARCHIVE")
COMBINED_BEFORE=$((ACTIVE_BEFORE + ARCHIVE_BEFORE))

if [ "$MOVE_COUNT" = "0" ]; then
  echo "action-list-reconcile: no terminal items in active (clean)"
  echo "  active=$ACTIVE_BEFORE archive=$ARCHIVE_BEFORE combined=$COMBINED_BEFORE"
  exit 0
fi

echo "action-list-reconcile: $MOVE_COUNT terminal item(s) in active action-list.json"
jq -r --argjson ids "$MOVE_IDS_JSON" '
  .items[]
  | select(.id as $i | $ids | index($i))
  | "  \(.id) :: status=\(.status) owner=\(.owner_role // "-")"' "$ACTION_LIST"

if [ "$APPLY" = "0" ]; then
  echo "  (dry-run — pass --apply to move them)"
  exit 0
fi

# Ensure the archive file exists with a valid skeleton before we extend it.
if [ ! -f "$ACTION_ARCHIVE" ]; then
  echo '{"generated":"","items":[]}' > "$ACTION_ARCHIVE"
fi

TMP_ARCHIVE=$(mktemp)
TMP_ACTIVE=$(mktemp)
trap 'rm -f "$TMP_ARCHIVE" "$TMP_ACTIVE"' EXIT

# Append matching active items to archive, then dedup by id keeping the freshest
# item (group_by + map(last)). Stamp archived_at as archive-reconcile.sh does.
jq --argjson ids "$MOVE_IDS_JSON" --slurpfile active "$ACTION_LIST" '
  .items = (
    (.items + [ $active[0].items[] | select(.id as $i | $ids | index($i)) ])
    | group_by(.id)
    | map(last)
  )
  | .archived_at = (now | todate)
' "$ACTION_ARCHIVE" > "$TMP_ARCHIVE"

# Drop matching items from active.
jq --argjson ids "$MOVE_IDS_JSON" '
  .items |= map(select(.id as $i | $ids | index($i) | not))
' "$ACTION_LIST" > "$TMP_ACTIVE"

# Combined-count guard — refuse to write a destructive result. Items MOVE, so
# the combined count must stay exactly equal (a decrease means an item was lost;
# an increase means the dedup-on-append failed).
NEW_ACTIVE=$(jq '.items | length' "$TMP_ACTIVE")
NEW_ARCHIVE=$(jq '.items | length' "$TMP_ARCHIVE")
NEW_COMBINED=$((NEW_ACTIVE + NEW_ARCHIVE))

if [ "$NEW_COMBINED" -ne "$COMBINED_BEFORE" ]; then
  echo "action-list-reconcile: ABORT — combined count changed $COMBINED_BEFORE → $NEW_COMBINED" >&2
  echo "  refusing to write; leaving $ACTION_LIST and $ACTION_ARCHIVE untouched." >&2
  exit 2
fi

mv "$TMP_ARCHIVE" "$ACTION_ARCHIVE"
mv "$TMP_ACTIVE"  "$ACTION_LIST"
trap - EXIT

echo "action-list-reconcile: moved $MOVE_COUNT item(s)"
echo "  active=$ACTIVE_BEFORE → $NEW_ACTIVE  archive=$ARCHIVE_BEFORE → $NEW_ARCHIVE  combined=$COMBINED_BEFORE (unchanged)"
