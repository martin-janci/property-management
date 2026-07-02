#!/usr/bin/env bash
# backlog-refill.sh — in-repo, planner-independent buffer refill for the
# dispatcher. Promotes fresh, actionable items from the research routine's
# backlog.json into the dispatcher's action-list.json when the claimable
# buffer is starved.
#
# Why this script exists (dispatcher starvation, 2026-07-02):
#
#   The dispatcher's ONLY source of genuinely-fresh backlog is the planner
#   (Phase 2.6 Tier-2 `curl $DISPATCHER_URL`). That kick is a documented
#   no-op whenever `DISPATCHER_URL` is mis-set to an Anthropic/CCR proxy
#   (GH #1380 defect 2) — an operator-only secret the dispatcher cannot
#   self-heal. The in-repo fallbacks are exhaustible: re-opening `deferred`
#   rows (Tier-1a) drains to zero, and `coverage.json` (Tier-1) is a stale,
#   finite rubric (snapshot 2026-06-23: 37/49 stories already done, 5 fresh
#   left). Once all three are dry, `open_claimable_count` collapses toward 0
#   and the dispatcher claims nothing despite ample WIP headroom.
#
#   Meanwhile `.research/backlog.json` — the research routine's continuously
#   refreshed, SCORED, PLANNED output — carries dozens of `open`/`ready`
#   vectors the dispatcher never draws from. A live action-list row's own
#   `source` reads "dispatcher-tier1-refill … (backlog.json promote)": this
#   promotion path existed before coverage.json displaced it. This script
#   revives it as a first-class, deterministic, self-healing refill tier
#   that needs NO network and NO planner.
#
# Ghost-robust trigger (the metric blind spot, findings.json):
#
#   Phase 2.6's `open_claimable_count` is polluted by GHOST rows — open
#   action-list items whose id is already terminal/in-flight in assignments
#   but not yet reconciled closed. Those inflate the buffer metric so the
#   refill gate never fires even at 0 TRUE claimable. This script does NOT
#   trust that metric: it computes its OWN honest claimable count (open
#   action-list items whose id AND stem are absent from assignments, active
#   + archive, with satisfiable deps) and gates on that.
#
# Priority mapping (score-based, per operator decision 2026-07-02), calibrated
# to the research routine's ACTUAL score scale — 0..8, clamped at 8, with the
# routine's own "actionable" bar at score >= 3 (routine-prompt.md:121,628):
#   score >= 6            -> high
#   3 <= score <= 5       -> medium   (>=3 is the routine's confidence/act bar)
#   score <  3            -> low
#   confidence == "low"   -> downgrade one tier (min low)
#
# Bounded + additive (distinct from the reconcilers, which are count-invariant):
#   - Never lifts honest claimable above BUFFER_CEIL.
#   - Promotes at most BACKLOG_REFILL_CAP items per run (default 24) to keep
#     action-list.json well inside the MCP inline-push size limit (issue
#     #1014 — the same reason action-list-reconcile.sh exists).
#   - Only ever APPENDS new open rows; never mutates or removes existing
#     rows. Guarded: new active count == old active count + promoted_n.
#
# Idempotent + re-runnable: promoted ids are dedup'd against existing
# action-list ids/stems and assignment ids/stems, so a second --apply on a
# healthy or unchanged buffer is a no-op.
#
# Usage:
#   ./.research/backlog-refill.sh                 # dry-run: print promote set, write nothing
#   ./.research/backlog-refill.sh --apply         # promote fresh backlog items in place
#   BACKLOG_FILE=… ACTION_LIST=… ASSIGN=… ASSIGN_ARCHIVE=… \
#     BUFFER_FLOOR=… BUFFER_TARGET=… BUFFER_CEIL=… BACKLOG_REFILL_CAP=… \
#     BACKLOG_REFILL_NOW=… ./.research/backlog-refill.sh [--apply]

set -euo pipefail

ACTION_LIST="${ACTION_LIST:-.research/management/action-list.json}"
BACKLOG_FILE="${BACKLOG_FILE:-.research/backlog.json}"
ASSIGN="${ASSIGN:-.research/management/assignments.json}"
ASSIGN_ARCHIVE="${ASSIGN_ARCHIVE:-.research/management/assignments-archive.json}"

BUFFER_FLOOR="${BUFFER_FLOOR:-36}"
BUFFER_TARGET="${BUFFER_TARGET:-72}"
BUFFER_CEIL="${BUFFER_CEIL:-120}"
BACKLOG_REFILL_CAP="${BACKLOG_REFILL_CAP:-24}"
NOW="${BACKLOG_REFILL_NOW:-$(date -u +%Y-%m-%dT%H:%M:%SZ)}"

APPLY=0
[ "${1:-}" = "--apply" ] && APPLY=1

if [ ! -f "$ACTION_LIST" ]; then
  echo "backlog-refill: action-list missing — nothing to do: $ACTION_LIST" >&2
  exit 0
fi
if [ ! -f "$BACKLOG_FILE" ]; then
  echo "backlog-refill: backlog.json missing — no refill source: $BACKLOG_FILE" >&2
  exit 0
fi

# Assignment id + stem sets (active + archive), for ghost/dup exclusion.
# stem() mirrors dispatcher-prompt.md Phase 3 + dispatcher-self-test.sh T24:
# strip a trailing -(impl|fix|v2|retry|followup|wip)<digits> suffix. Bare
# action-list/backlog slugs carry no branch prefix, so no prefix strip.
ASSIGN_INPUTS=("$ASSIGN")
[ -f "$ASSIGN_ARCHIVE" ] && ASSIGN_INPUTS+=("$ASSIGN_ARCHIVE")

# Assignment id set (active + archive). `jq -s` slurps every input file into an
# array of root docs; `.[].assignments[]?` then streams all rows regardless of
# which file (or how many) held them — this is the multi-file-safe read that
# --slurpfile (one file only) cannot do.
ASSIGN_IDS=$(jq -s '[ .[].assignments[]?.task_id ] | unique' "${ASSIGN_INPUTS[@]}")

# Honest effective-claimable count: open action-list items whose id AND stem
# are absent from assignments (active+archive) and whose deps are empty
# (promoted rows always carry depends_on:[]; pre-existing dep-blocked rows are
# conservatively excluded from "claimable" here, matching Phase 2.6).
HONEST=$(jq -n \
  --slurpfile al "$ACTION_LIST" \
  --argjson aids "$ASSIGN_IDS" '
  def stem: sub("-(impl|fix|v2|retry|followup|wip)[0-9]*$";"");
  ($aids | map(stem) | unique) as $astems
  | [ $al[0].items[]
      | select(.status=="open")
      | select(((.depends_on // []) | length) == 0)
      | select((.id | IN($aids[])) | not)
      | select((.id | stem | IN($astems[])) | not)
    ] | length
')

# What the buffer needs to reach TARGET, clamped so we never cross CEIL.
NEED=$(( BUFFER_TARGET - HONEST ))
HEADROOM_TO_CEIL=$(( BUFFER_CEIL - HONEST ))
[ "$NEED" -gt "$HEADROOM_TO_CEIL" ] && NEED="$HEADROOM_TO_CEIL"
[ "$NEED" -gt "$BACKLOG_REFILL_CAP" ] && NEED="$BACKLOG_REFILL_CAP" && CAPPED=1 || CAPPED=0

echo "backlog-refill: honest_claimable=$HONEST floor=$BUFFER_FLOOR target=$BUFFER_TARGET ceil=$BUFFER_CEIL"

if [ "$HONEST" -ge "$BUFFER_FLOOR" ]; then
  echo "backlog-refill: buffer healthy (>= floor) — no refill needed"
  exit 0
fi
if [ "$NEED" -le 0 ]; then
  echo "backlog-refill: at/above ceil headroom — no room to promote"
  exit 0
fi

# Candidate pool from backlog.json: status open|ready, not already tracked as
# an action-list id/stem (T24 one-open-per-stem) nor an assignment id/stem.
# Sorted score DESC, id ASC (deterministic). Mapped to action-list schema.
CANDIDATES=$(jq -n \
  --slurpfile bl "$BACKLOG_FILE" \
  --slurpfile al "$ACTION_LIST" \
  --argjson aids "$ASSIGN_IDS" \
  --arg now "$NOW" '
  def stem: sub("-(impl|fix|v2|retry|followup|wip)[0-9]*$";"");
  def prio(score; conf):
    ( if (score // 0) >= 6 then 3 elif (score // 0) >= 3 then 2 else 1 end ) as $base
    | ( if (conf // "") == "low" then ($base - 1) else $base end )
    | ( if . < 1 then 1 else . end )
    | ( if . == 3 then "high" elif . == 2 then "medium" else "low" end );
  def role(vector):
    ( vector // "" )
    | if   test("^security") then "pm-security"
      elif test("^bug")      then "pm-backend"
      elif test("^test-gap") then "pm-qa"
      elif test("^dx")       then "pm-devops"
      elif test("^refactor") then "pm-tech-lead"
      else "pm-tech-lead" end;

  ( [ $al[0].items[]?.id ]                 | unique) as $al_ids
  | ([ $al[0].items[]? | select(.status=="open") | (.id | stem) ] | unique) as $al_open_stems
  | ($aids                                 | unique) as $a_ids
  | ($aids | map(stem)                     | unique) as $a_stems

  | [ $bl[0].items[]
      | select(.status == "open" or .status == "ready")
      | select((.id | IN($al_ids[])) | not)
      | select((.id | stem | IN($al_open_stems[])) | not)
      | select((.id | IN($a_ids[])) | not)
      | select((.id | stem | IN($a_stems[])) | not)
    ]
  | sort_by([ (-(.score // 0)), .id ])
  | map({
      id:         .id,
      action:     (.title // .id),
      owner_role: role(.vector),
      priority:   prio(.score; .confidence),
      status:     "open",
      source:     ("dispatcher-backlog-refill " + $now
                   + " (score=" + ((.score // 0) | tostring)
                   + " conf=" + (.confidence // "?")
                   + " vector=" + (.vector // "?") + ")"),
      deadline:   null,
      dependency: null,
      depends_on: [],
      first_open_at: $now,
      backlog_ref: { plan: (.plan // null), score: (.score // null), vector: (.vector // null) }
    })
')

AVAIL=$(echo "$CANDIDATES" | jq 'length')
PROMOTE_N=$(( NEED < AVAIL ? NEED : AVAIL ))

if [ "$PROMOTE_N" -le 0 ]; then
  echo "backlog-refill: no fresh backlog.json candidates available (avail=$AVAIL) — buffer stays starved"
  exit 0
fi

PROMOTE=$(echo "$CANDIDATES" | jq --argjson n "$PROMOTE_N" '.[0:$n]')

echo "backlog-refill: promoting $PROMOTE_N of $AVAIL candidate(s) (need=$NEED, per-run-cap=$BACKLOG_REFILL_CAP, capped=$CAPPED)"
echo "$PROMOTE" | jq -r '.[] | "  + \(.id)  [\(.priority)]  \(.action)"'

if [ "$APPLY" = "0" ]; then
  echo "  (dry-run — pass --apply to promote them)"
  exit 0
fi

ACTIVE_BEFORE=$(jq '.items | length' "$ACTION_LIST")
TMP=$(mktemp)
trap 'rm -f "$TMP"' EXIT

jq --argjson promote "$PROMOTE" '.items += $promote' "$ACTION_LIST" > "$TMP"

# Additive guard: item count must grow by EXACTLY promoted_n. A different delta
# means the append malformed the file — fail closed, write nothing.
ACTIVE_AFTER=$(jq '.items | length' "$TMP")
if [ "$ACTIVE_AFTER" -ne "$(( ACTIVE_BEFORE + PROMOTE_N ))" ]; then
  echo "backlog-refill: ABORT — item count $ACTIVE_BEFORE + $PROMOTE_N != $ACTIVE_AFTER; refusing to write" >&2
  exit 2
fi

mv "$TMP" "$ACTION_LIST"
trap - EXIT
echo "backlog-refill: promoted $PROMOTE_N item(s); action-list items $ACTIVE_BEFORE → $ACTIVE_AFTER (honest_claimable $HONEST → $(( HONEST + PROMOTE_N )))"
