#!/usr/bin/env bash
# pick-target-epic.sh — finish-first depth-first epic selector for the
# dispatcher's Phase 3 claim, behind DISPATCHER_FINISH_FIRST=1.
#
# Rule (matches the PR 3/5 design decision):
#   1. Load current target from objective.json.
#   2. If the current target still has ≥1 claimable open task →
#      KEEP. Print + exit 0.
#   3. Else: pick the epic with the fewest remaining open claimable
#      tasks (tie-break: max priority among that epic's open items).
#      "Closest-to-done" semantics — bias toward finishing what's
#      almost done.
#   4. Write the new target to objective.json (idempotent — same
#      epic_prefix → same file content).
#
# "Claimable" = open in action-list, NOT in active assignments,
#               NOT in archive, NOT dep-blocked.
#
# Output (default — line-oriented, grep-friendly):
#   epic_prefix=<prefix> open=<n> claimable=<m> action=<keep|repick|empty> reason=<short>
#
# Output (--json):
#   {"epic_prefix":..., "open":N, "claimable":M, "action":..., "reason":...}
#
# Exit codes:
#   0 — target selected (or no work available; epic_prefix=NONE)
#  64 — usage error
#  65 — input files missing or unparseable
#
# Usage:
#   pick-target-epic.sh [--update] [--json]
#                       [--action-list <path>] [--assignments <path>]
#                       [--archive <path>] [--objective <path>]
#
# Flags:
#   --update     write the chosen target to <objective>. Without it,
#                the script is read-only (dry-run; prints the choice).
#   --json       emit one-line JSON instead of key=value pairs.
#   --action-list / --assignments / --archive / --objective
#                override the default paths (under .research/).

set -euo pipefail

ACTION_LIST="${ACTION_LIST:-.research/management/action-list.json}"
ASSIGN="${ASSIGN:-.research/management/assignments.json}"
ARCHIVE="${ARCHIVE:-.research/management/assignments-archive.json}"
OBJECTIVE="${OBJECTIVE:-.research/management/objective.json}"
UPDATE=0
EMIT_JSON=0

while [ $# -gt 0 ]; do
  case "$1" in
    --update) UPDATE=1; shift;;
    --json)   EMIT_JSON=1; shift;;
    --action-list) ACTION_LIST="$2"; shift 2;;
    --assignments) ASSIGN="$2"; shift 2;;
    --archive)     ARCHIVE="$2"; shift 2;;
    --objective)   OBJECTIVE="$2"; shift 2;;
    -h|--help) sed -n '2,40p' "$0"; exit 0;;
    *) echo "unknown arg: $1" >&2; exit 64;;
  esac
done

if ! command -v jq >/dev/null 2>&1; then
  echo "error: jq required" >&2; exit 64
fi
for f in "$ACTION_LIST" "$ASSIGN" "$ARCHIVE"; do
  if [ ! -f "$f" ]; then
    echo "error: missing input: $f" >&2; exit 65
  fi
done

# ---- helpers -------------------------------------------------------

# epic_prefix(task_id) — mirror the definition in dispatcher-prompt.md
# Phase 3 "Same-epic burst-claim guard":
#   ^(gap-\d+[a-z]?)
#   ^(pm-[a-z-]+?)-
#   else: full task_id
#
# The result is lowercased (ascii_downcase). The gap-/pm- branches already
# yield lowercase by construction, but the else-branch returns the full
# task_id verbatim — and task_ids derived from camelCase source filenames
# (e.g. churn-hotspot-…-useOfflineSupport-ts) carry uppercase characters.
# An uppercase epic_prefix written to objective.json trips dispatcher
# self-test T21 (epic_prefix must be all-lowercase kebab), which ABORTS the
# finish-first Phase-6 commit (issue #2651). Normalising here keeps the
# internal grouping self-consistent (STATS group-by and the KEEP comparison
# both see the lowercased form) and guarantees the persisted target always
# satisfies the T21 invariant.
epic_prefix_jq='
  def epic_prefix:
    ( if test("^gap-[0-9]+[a-z]?") then capture("^(?<e>gap-[0-9]+[a-z]?)").e
      elif test("^pm-[a-z-]+?-") then capture("^(?<e>pm-[a-z-]+?)-").e
      else . end
    ) | ascii_downcase;
'

# Build the set of task_ids that are NOT claimable: in active assignments
# (any non-terminal status) OR in archive (terminal).
NOT_CLAIMABLE=$(jq -r '
  .assignments[]
  | select(.status=="in-progress" or .status=="review" or .status=="merged" or .status=="failed" or .status=="done")
  | .task_id' "$ASSIGN" "$ARCHIVE" | sort -u)

# Build per-epic stats over action-list open items.
# Each row: epic_prefix \t total_open \t claimable_open \t max_priority_rank
# priority_rank: critical=4 high=3 medium=2 low=1 (default 0)
STATS=$(jq -r --argjson not_claimable "$(printf '%s\n' "$NOT_CLAIMABLE" | jq -R . | jq -s .)" '
  '"$epic_prefix_jq"'
  def pri_rank:
    if . == "critical" then 4
    elif . == "high" then 3
    elif . == "medium" then 2
    elif . == "low" then 1
    else 0 end;
  # Items with non-empty depends_on are conservatively treated as
  # not-claimable here (Phase 2.6 computes the precise dep_blocked set).
  # The dispatcher will refine this; the picker only needs a useful
  # approximation for finish-first.
  [ .items[]
    | select(.status=="open")
    | .id as $tid
    | { ep: ($tid | epic_prefix),
        id: $tid,
        claimable: ((($not_claimable | index($tid)) | not)
                    and ((.depends_on // []) | length == 0)),
        pr: (.priority | pri_rank) }
  ]
  | group_by(.ep)
  | map({
      ep: .[0].ep,
      total_open: length,
      claimable_open: (map(select(.claimable)) | length),
      max_priority: (map(.pr) | max)
    })
  | sort_by([.claimable_open, -.max_priority])
  | .[]
  | "\(.ep)\t\(.total_open)\t\(.claimable_open)\t\(.max_priority)"
' "$ACTION_LIST")

# ---- decide --------------------------------------------------------

CURRENT_EP=""
if [ -f "$OBJECTIVE" ]; then
  CURRENT_EP=$(jq -r '.epic_prefix // empty' "$OBJECTIVE" 2>/dev/null || true)
fi

ACTION=""
CHOSEN_EP=""
CHOSEN_OPEN=0
CHOSEN_CLAIMABLE=0
REASON=""

# 1. If we have a current target, check whether it still has claimable work.
if [ -n "$CURRENT_EP" ]; then
  cur_row=$(printf '%s\n' "$STATS" | awk -F'\t' -v ep="$CURRENT_EP" '$1==ep {print; exit}')
  if [ -n "$cur_row" ]; then
    cur_claimable=$(echo "$cur_row" | awk -F'\t' '{print $3}')
    cur_open=$(echo "$cur_row" | awk -F'\t' '{print $2}')
    if [ "$cur_claimable" -ge 1 ]; then
      ACTION="keep"
      CHOSEN_EP="$CURRENT_EP"
      CHOSEN_OPEN="$cur_open"
      CHOSEN_CLAIMABLE="$cur_claimable"
      REASON="current target still has claimable work"
    fi
  fi
fi

# 2. No current target, or current target exhausted → pick closest-to-done.
if [ -z "$ACTION" ]; then
  # First epic with claimable_open >= 1 in the sorted stats (sorted ascending
  # by claimable_open, then descending by max_priority for tie-break).
  pick=$(printf '%s\n' "$STATS" | awk -F'\t' '$3 >= 1 {print; exit}')
  if [ -z "$pick" ]; then
    ACTION="empty"
    CHOSEN_EP="NONE"
    REASON="no epic has claimable open work"
  else
    ACTION=$([ -n "$CURRENT_EP" ] && echo "repick" || echo "select")
    CHOSEN_EP=$(echo "$pick" | awk -F'\t' '{print $1}')
    CHOSEN_OPEN=$(echo "$pick" | awk -F'\t' '{print $2}')
    CHOSEN_CLAIMABLE=$(echo "$pick" | awk -F'\t' '{print $3}')
    if [ "$ACTION" = "repick" ]; then
      REASON="prior target '$CURRENT_EP' exhausted; closest-to-done is '$CHOSEN_EP' (open=$CHOSEN_OPEN claimable=$CHOSEN_CLAIMABLE)"
    else
      REASON="initial selection: closest-to-done is '$CHOSEN_EP' (open=$CHOSEN_OPEN claimable=$CHOSEN_CLAIMABLE)"
    fi
  fi
fi

# ---- emit ----------------------------------------------------------

if [ "$EMIT_JSON" = "1" ]; then
  jq -nc \
    --arg ep "$CHOSEN_EP" \
    --argjson open "$CHOSEN_OPEN" \
    --argjson claimable "$CHOSEN_CLAIMABLE" \
    --arg action "$ACTION" \
    --arg reason "$REASON" \
    '{epic_prefix:$ep, open:$open, claimable:$claimable, action:$action, reason:$reason}'
else
  printf 'epic_prefix=%s open=%s claimable=%s action=%s reason=%s\n' \
    "$CHOSEN_EP" "$CHOSEN_OPEN" "$CHOSEN_CLAIMABLE" "$ACTION" "$REASON"
fi

# ---- persist (if --update) ----------------------------------------

if [ "$UPDATE" = "1" ] && [ "$ACTION" != "empty" ]; then
  mkdir -p "$(dirname "$OBJECTIVE")"
  # Idempotent: keep the existing selected_at when we KEEP, refresh when
  # we SELECT or REPICK.
  if [ "$ACTION" = "keep" ] && [ -f "$OBJECTIVE" ]; then
    PRIOR_SELECTED_AT=$(jq -r '.selected_at // empty' "$OBJECTIVE" 2>/dev/null || true)
  else
    PRIOR_SELECTED_AT=""
  fi
  SELECTED_AT="${PRIOR_SELECTED_AT:-$(date -u +%Y-%m-%dT%H:%M:%SZ)}"
  jq -n \
    --arg ep "$CHOSEN_EP" \
    --arg sel "$SELECTED_AT" \
    --argjson open "$CHOSEN_OPEN" \
    --argjson claimable "$CHOSEN_CLAIMABLE" \
    --arg action "$ACTION" \
    --arg reason "$REASON" \
    '{schema_version: 1,
      epic_prefix: $ep,
      selected_at: $sel,
      last_action: $action,
      reason: $reason,
      stats_at_selection: { open: $open, claimable: $claimable }
     }' > "$OBJECTIVE.tmp"
  mv "$OBJECTIVE.tmp" "$OBJECTIVE"
fi

exit 0
