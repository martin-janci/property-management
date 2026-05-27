#!/usr/bin/env bash
# dispatcher-followup.sh — decide and apply the dispatcher-mode follow-up
# action for one PR. Reads `.research/management/assignments.json`, mutates
# the matching row per the SKILL.md decision table, and prints the spawn
# brief to stdout when the action is "respawned" (the dispatcher then feeds
# that brief into a Task spawn — this script does not call Claude tools).
#
# Usage:
#   dispatcher-followup.sh --pr <n> [--dry-run]
#
# Output (last line, machine-parseable):
#   followup=<respawned|skip|failed> pr=<n> [specialist=<sp>] [round=<k>] note=<short>
#
# Exit codes:
#   0 = respawned or skip
#   2 = failed (round cap reached or reviewer rejected)
#   3 = bad invocation / missing state files

set -euo pipefail

PR=""
DRY_RUN=0
ASSIGNMENTS="${ASSIGNMENTS_PATH:-.research/management/assignments.json}"
ACTION_LIST="${ACTION_LIST_PATH:-.research/management/action-list.json}"
MAX_FIX_ROUNDS="${MAX_FIX_ROUNDS:-3}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --pr)        PR="$2"; shift 2 ;;
    --dry-run)   DRY_RUN=1; shift ;;
    -h|--help)
      sed -n '2,18p' "$0"; exit 0 ;;
    *) echo "unknown arg: $1" >&2; exit 3 ;;
  esac
done

[[ -n "$PR" ]] || { echo "missing --pr <n>" >&2; exit 3; }
[[ -f "$ASSIGNMENTS" ]] || { echo "assignments.json not found at $ASSIGNMENTS" >&2; exit 3; }
command -v jq >/dev/null 2>&1 || { echo "jq required" >&2; exit 3; }

NOW="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

# --- Pull the matching row out of assignments.json
ROW=$(jq --argjson n "$PR" '
  .assignments[] | select(.pr_number == $n)
' "$ASSIGNMENTS")

if [[ -z "$ROW" ]]; then
  echo "no row in $ASSIGNMENTS for pr_number=$PR" >&2
  echo "followup=skip pr=$PR note=no-assignment-row"
  exit 0
fi

TASK_ID=$(jq -r '.task_id // ""'           <<<"$ROW")
BRANCH=$(jq -r  '.branch // ""'            <<<"$ROW")
SPECIALIST=$(jq -r '.specialist // "generic"' <<<"$ROW")
STATUS=$(jq -r  '.status // ""'            <<<"$ROW")
RS=$(jq -r      '.reviewer_summary // ""'  <<<"$ROW")
FR=$(jq -r      '.fix_rounds // 0'         <<<"$ROW")
[[ "$FR" =~ ^[0-9]+$ ]] || FR=0

# --- Parse verdict + note out of `reviewer_summary` string
# Reviewer_summary shape: "verdict=<v> head_oid=<oid> note=<rest>" (note can contain spaces).
VERDICT=$(grep -oP '(?<=verdict=)[a-z]+' <<<"$RS" || true)
NOTE=$(grep -oP '(?<=note=).*' <<<"$RS" || true)

emit() {
  # emit "<line>"  — prints the machine-parseable result line on stdout.
  printf '%s\n' "$1"
}

mutate_row() {
  # mutate_row '<jq update expression on .assignments[] | select(.pr_number == $n)>'
  local upd="$1"
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "[dry-run] would apply: $upd" >&2
    return 0
  fi
  local tmp
  tmp="$(mktemp)"
  jq --argjson n "$PR" --arg now "$NOW" "
    .assignments |= map(
      if .pr_number == \$n then ($upd) else . end
    )
    | .generated = \$now
  " "$ASSIGNMENTS" > "$tmp"
  mv "$tmp" "$ASSIGNMENTS"
}

# ============================================================================
# Decision table — see SKILL.md "Dispatcher mode — state machine"
# ============================================================================

# 1. No verdict yet — wait.
if [[ -z "$VERDICT" ]]; then
  emit "followup=skip pr=$PR note=no-verdict-yet"
  exit 0
fi

# 2. Approve — Phase 5.5 / ppt-pr-merge handles it.
if [[ "$VERDICT" == "approve" ]]; then
  emit "followup=skip pr=$PR note=approved-call-pr-merge"
  exit 0
fi

# 3. Reviewer rejected outright.
if [[ "$VERDICT" == "block" || "$VERDICT" == "reject" ]]; then
  mutate_row '. + {status: "failed", reason: "reviewer-rejected", last_updated: $now, status_changed_at: $now}'
  emit "followup=failed pr=$PR note=reviewer-rejected"
  exit 2
fi

# 4. Anything other than "changes" at this point is unexpected — treat as skip.
if [[ "$VERDICT" != "changes" ]]; then
  emit "followup=skip pr=$PR note=unknown-verdict:$VERDICT"
  exit 0
fi

# --- VERDICT == "changes" path ---

# 5. Round cap.
if (( FR >= MAX_FIX_ROUNDS )); then
  mutate_row '. + {status: "failed", reason: "fix-rounds-exhausted", last_updated: $now, status_changed_at: $now}'
  emit "followup=failed pr=$PR note=fix-rounds-exhausted round=$FR"
  exit 2
fi

# 6. Idempotency guard — row already in-progress means a prior call respawned.
if [[ "$STATUS" == "in-progress" ]]; then
  emit "followup=skip pr=$PR note=in-progress-already round=$FR"
  exit 0
fi

# 7. The real respawn path. Bump fix_rounds, flip status, clear reviewer_summary.
NEXT_ROUND=$((FR + 1))
mutate_row "$(cat <<JQ
. + {
  status: "in-progress",
  fix_rounds: ($FR + 1),
  reviewer_summary: null,
  last_updated: \$now,
  status_changed_at: \$now
}
JQ
)"

# Look up the original action text for context (best-effort).
ACTION=""
if [[ -f "$ACTION_LIST" ]]; then
  ACTION=$(jq -r --arg id "$TASK_ID" '
    (.. | objects | select(.id? == $id) | .action? // "") // ""
  ' "$ACTION_LIST" 2>/dev/null | head -1)
fi

# Print the spawn brief on stdout so the calling agent can feed it to Task.
cat <<BRIEF
=== ppt-pr-followup respawn brief ===
You are a $SPECIALIST implementer addressing reviewer fix-round $NEXT_ROUND/$MAX_FIX_ROUNDS
on PR #$PR (branch $BRANCH). Do NOT open a new PR. Check out $BRANCH and push
commits on top.

Reviewer's blocking findings (must be addressed):

    $NOTE

Original task context for reference only: $TASK_ID: $ACTION

Follow .claude/skills/ppt-implement/agents/$SPECIALIST.md. Verify per the
specialist's verify command before pushing. Return EXACTLY:
  pr=$PR status=<done|blocked> specialist=$SPECIALIST note=<short>
=== end brief ===
BRIEF

emit "followup=respawned pr=$PR specialist=$SPECIALIST round=$NEXT_ROUND"
exit 0
