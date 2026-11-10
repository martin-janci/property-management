#!/usr/bin/env bash
# signals-promote.sh — ingest the Tier-1d generator's orphaned signal sink
# (issue #3357 part 2). Promotes fresh `code-review-finding` signals written to
# `.research/signals/*tier1d*.json` into the dispatcher's action-list.json —
# de-duped and mobile-native-gated — so the demand-driven generator stops being
# a silent dead-end.
#
# Why this script exists (the Tier-1d dead-end, 2026-11-08):
#
#   When the claim buffer is starved, Phase 2.6 Tier-1d spawns a `ppt-dev-review`
#   slice. That slice writes findings to `.research/signals/<date>-<segment>-
#   tier1d.json`. But NO refill/ingest tier reads `.research/signals/`:
#   backlog-refill.sh reads backlog.json, issue-ingest.sh reads GitHub issues,
#   retry-remint.sh reads the archive. So every Tier-1d finding was orphaned and
#   never became a claimable action-list item — the generator burned a subagent
#   per starved run for nothing, and the buffer stayed pinned at claimable<floor
#   with 0 claimed (issue #3357). This script closes that loop: it is the reader
#   the Tier-1d sink never had.
#
# Dead-end exclusion ("stop emitting dead-end vectors", issue #3357):
#
#   The dev-review segment rotation also reviews `mobile-native` / KMP code, so
#   its signals include `code-review-mobile-native-kmp-*` findings. Those are
#   STRUCTURALLY UNLANDABLE in the cloud runner (AGP egress-blocked, issue
#   #2652): promoting them only mints action-list rows the Phase-3 claim gate
#   then skips, burning a backlog slot and polluting the claimable metric. This
#   script excludes them AT PROMOTION TIME using the same predicate the claim
#   gate uses — `mobile-native-gate.sh --id` — so a dead-end signal never
#   becomes a dead-end action-list row. `mobile-rn` (React Native, jest-
#   verifiable in cloud) is deliberately NOT gated by that helper, so RN
#   findings still promote.
#
# De-dup (never double-source a finding):
#
#   A signal is skipped when its id OR stem already appears as (a) an action-list
#   row (any non-terminal status — T24 one-open-per-stem), (b) an assignment id
#   (active + archive, so anything ever claimed/merged is excluded), or (c) a
#   backlog.json id (the research routine's scored sink that backlog-refill.sh
#   already drains). (c) makes this a true LAST-RESORT tier: it only promotes
#   signals that fell through every other crack — exactly the orphaned remainder
#   issue #3357 describes.
#
# Bounded + additive + fail-closed (mirrors backlog-refill.sh):
#   - Buffer-gated: only fires when honest claimable < BUFFER_FLOOR; promotes up
#     to BUFFER_TARGET; never lifts honest claimable above BUFFER_CEIL.
#   - At most SIGNALS_PROMOTE_CAP items per run (default 12) to keep
#     action-list.json inside the MCP inline-push size limit.
#   - Only APPENDS open rows; item count must grow by EXACTLY promoted_n or the
#     write is refused.
#   - Newest-first (signal `generated` date DESC, then score DESC, id ASC) so a
#     capped run promotes the freshest findings.
#
# Idempotent: promoted ids dedup against the sets above, so a second --apply on
# an unchanged buffer is a no-op.
#
# Usage:
#   ./.research/signals-promote.sh                 # dry-run: print promote set
#   ./.research/signals-promote.sh --apply         # promote in place
#   SIGNALS_GLOB=… ACTION_LIST=… ASSIGN=… ASSIGN_ARCHIVE=… BACKLOG_FILE=… \
#     BUFFER_FLOOR=… BUFFER_TARGET=… BUFFER_CEIL=… SIGNALS_PROMOTE_CAP=… \
#     SIGNALS_PROMOTE_NOW=… GATE_SCRIPT=… ./.research/signals-promote.sh [--apply]

set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"

ACTION_LIST="${ACTION_LIST:-.research/management/action-list.json}"
ASSIGN="${ASSIGN:-.research/management/assignments.json}"
ASSIGN_ARCHIVE="${ASSIGN_ARCHIVE:-.research/management/assignments-archive.json}"
BACKLOG_FILE="${BACKLOG_FILE:-.research/backlog.json}"
SIGNALS_GLOB="${SIGNALS_GLOB:-.research/signals/*tier1d*.json}"
GATE_SCRIPT="${GATE_SCRIPT:-$HERE/mobile-native-gate.sh}"

BUFFER_FLOOR="${BUFFER_FLOOR:-36}"
BUFFER_TARGET="${BUFFER_TARGET:-72}"
BUFFER_CEIL="${BUFFER_CEIL:-120}"
SIGNALS_PROMOTE_CAP="${SIGNALS_PROMOTE_CAP:-12}"
NOW="${SIGNALS_PROMOTE_NOW:-$(date -u +%Y-%m-%dT%H:%M:%SZ)}"

APPLY=0
[ "${1:-}" = "--apply" ] && APPLY=1

command -v jq >/dev/null 2>&1 || { echo "signals-promote: jq required" >&2; exit 64; }
if [ ! -f "$ACTION_LIST" ]; then
  echo "signals-promote: action-list missing — nothing to do: $ACTION_LIST" >&2
  exit 0
fi

# Expand the signal glob (nullglob so a no-match degrades to "no source").
shopt -s nullglob
# shellcheck disable=SC2206  # word-split is intentional: SIGNALS_GLOB is a glob
SIGNAL_FILES=( $SIGNALS_GLOB )
shopt -u nullglob
if [ "${#SIGNAL_FILES[@]}" -eq 0 ]; then
  echo "signals-promote: no signal files match '$SIGNALS_GLOB' — no source" >&2
  exit 0
fi

# ---- assignment id set (active + archive), for ghost/dup exclusion ----------
# Same multi-file-safe read as backlog-refill.sh: guard both inputs with -f so a
# missing primary assignments.json doesn't abort jq under `set -e` (#2078).
ASSIGN_INPUTS=()
[ -f "$ASSIGN" ]         && ASSIGN_INPUTS+=("$ASSIGN")
[ -f "$ASSIGN_ARCHIVE" ] && ASSIGN_INPUTS+=("$ASSIGN_ARCHIVE")
if [ "${#ASSIGN_INPUTS[@]}" -eq 0 ]; then
  ASSIGN_IDS='[]'
else
  ASSIGN_IDS=$(jq -s '[ .[].assignments[]?.task_id ] | unique' "${ASSIGN_INPUTS[@]}")
fi

# Backlog.json id set (the sink backlog-refill.sh already drains). Optional —
# a missing file degrades to no exclusion.
if [ -f "$BACKLOG_FILE" ]; then
  BACKLOG_IDS=$(jq '[ .items[]?.id ] | unique' "$BACKLOG_FILE")
else
  BACKLOG_IDS='[]'
fi

# ---- honest effective-claimable count (mirror of backlog-refill.sh) ---------
HONEST=$(jq -n \
  --slurpfile al "$ACTION_LIST" \
  --argjson aids "$ASSIGN_IDS" '
  def stem: sub("-(impl|fix|v2|retry|followup|wip)[0-9]*$";"");
  ($aids | map(stem) | unique) as $astems
  | [ $al[0].items[]
      | select(.status=="open")
      | select(((.depends_on // []) | length) == 0)
      | select((.id | IN($aids[])) | not)
      | select(((.retry_of? // null) != null) or ((.id | stem | IN($astems[])) | not))
    ] | length
')

NEED=$(( BUFFER_TARGET - HONEST ))
HEADROOM_TO_CEIL=$(( BUFFER_CEIL - HONEST ))
[ "$NEED" -gt "$HEADROOM_TO_CEIL" ] && NEED="$HEADROOM_TO_CEIL"
[ "$NEED" -gt "$SIGNALS_PROMOTE_CAP" ] && NEED="$SIGNALS_PROMOTE_CAP" && CAPPED=1 || CAPPED=0

echo "signals-promote: honest_claimable=$HONEST floor=$BUFFER_FLOOR target=$BUFFER_TARGET ceil=$BUFFER_CEIL (sources=${#SIGNAL_FILES[@]})"

if [ "$HONEST" -ge "$BUFFER_FLOOR" ]; then
  echo "signals-promote: buffer healthy (>= floor) — no promotion needed"
  exit 0
fi
if [ "$NEED" -le 0 ]; then
  echo "signals-promote: at/above ceil headroom — no room to promote"
  exit 0
fi

# ---- flatten every signal file into one normalized pool ---------------------
# Signal files come in three shapes (verified across the sink): a bare array, a
# `{segment,generated,signals:[…]}` object, and a `{…,findings:[…]}` object.
# Doc-level `generated`/`segment` are captured and attached to each signal (a
# signal may also carry its own). Only objects with an `id` survive.
POOL_FILE=$(mktemp)
ASSIGN_IDS_FILE=$(mktemp)
BACKLOG_IDS_FILE=$(mktemp)
trap 'rm -f "$POOL_FILE" "$ASSIGN_IDS_FILE" "$BACKLOG_IDS_FILE"' EXIT
printf '%s' "$ASSIGN_IDS"  > "$ASSIGN_IDS_FILE"
printf '%s' "$BACKLOG_IDS" > "$BACKLOG_IDS_FILE"

jq -s '
  [ .[]
    | (if type=="object" then (.generated // .generated_at // "") else "" end) as $g
    | (if type=="object" then (.segment // "") else "" end) as $dseg
    | ( if type=="array" then .
        elif (type=="object" and has("signals"))  then .signals
        elif (type=="object" and has("findings")) then .findings
        else [] end )
    | .[]
    | select(type=="object" and has("id"))
    | { id:         .id,
        title:      (.title // .summary // .short_summary // null),
        vector:     (.candidate_vector // .vector // null),
        score:      (.score_delta // .score // 0),
        confidence: (.confidence // null),
        segment:    (.segment // $dseg),
        gen:        (._gen // $g) }
  ]' "${SIGNAL_FILES[@]}" > "$POOL_FILE"

# ---- de-dup + schema-map + order (mirror backlog-refill.sh) -----------------
# All bulk JSON (pool, id sets, action-list) is passed via --slurpfile, never
# --argjson: a multi-hundred-element array on the command line blows ARG_MAX.
CANDIDATES=$(jq -n \
  --slurpfile pool "$POOL_FILE" \
  --slurpfile al "$ACTION_LIST" \
  --slurpfile aids "$ASSIGN_IDS_FILE" \
  --slurpfile blids "$BACKLOG_IDS_FILE" \
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
  def deslug: sub("^code-review-";"") | gsub("-";" ");

  ($pool[0])  as $pool
  | ($aids[0])  as $aids
  | ($blids[0]) as $blids
  | ( [ $al[0].items[]?.id ] | unique) as $al_ids
  | ([ $al[0].items[]? | select(.status != "done" and .status != "dropped") | (.id | stem) ] | unique) as $al_active_stems
  | ($aids  | unique)          as $a_ids
  | ($aids  | map(stem) | unique) as $a_stems
  | ($blids | unique)          as $bl_ids
  | ($blids | map(stem) | unique) as $bl_stems

  | [ $pool[]
      | select((.id | IN($al_ids[])) | not)
      | select((.id | stem | IN($al_active_stems[])) | not)
      | select((.id | IN($a_ids[])) | not)
      | select((.id | stem | IN($a_stems[])) | not)
      | select((.id | IN($bl_ids[])) | not)
      | select((.id | stem | IN($bl_stems[])) | not)
    ]
  # Collapse intra-pool duplicate ids (same finding written by multiple runs):
  # keep the highest-scoring occurrence.
  | group_by(.id) | map(max_by(.score // 0))
  # Order: signal date DESC (freshest first), score DESC, id ASC. gen is a
  # "YYYY-MM-DD" string → digits-only number, negated for DESC; missing gen → 0
  # (sorts last). All keys ascending with numeric negation = fully deterministic.
  | sort_by([ ( (.gen // "") | gsub("[^0-9]";"") | (if . == "" then 0 else tonumber end) | -. ),
              (-(.score // 0)),
              .id ])
  | map({
      id:         .id,
      action:     (.title // (.id | deslug)),
      owner_role: role(.vector),
      priority:   prio(.score; .confidence),
      status:     "open",
      source:     ("dispatcher-signals-promote " + $now
                   + " (tier1d " + (.segment // "?")
                   + " score=" + ((.score // 0) | tostring)
                   + " vector=" + (.vector // "?") + ")"),
      deadline:   null,
      dependency: null,
      depends_on: [],
      first_open_at: $now,
      signal_ref: { segment: (.segment // null), score: (.score // null), vector: (.vector // null), generated: (.gen // null) }
    })
')

# ---- exclude mobile-native/KMP dead-ends (reuse the claim-time gate) --------
# Loop the (already small, post-dedup) candidate ids through the SAME predicate
# the Phase-3 claim gate uses. `--id` exits 0 when GATED.
GATED_IDS='[]'
if [ -x "$GATE_SCRIPT" ]; then
  while IFS= read -r cid; do
    [ -z "$cid" ] && continue
    if "$GATE_SCRIPT" --id "$cid" >/dev/null 2>&1; then
      GATED_IDS=$(jq -n --argjson g "$GATED_IDS" --arg i "$cid" '$g + [$i]')
    fi
  done < <(echo "$CANDIDATES" | jq -r '.[].id')
else
  echo "signals-promote: WARN gate script not executable ($GATE_SCRIPT) — mobile-native dead-ends NOT excluded" >&2
fi

CANDIDATES=$(echo "$CANDIDATES" | jq --argjson gated "$GATED_IDS" '
  [ .[] | select((.id | IN($gated[])) | not) ]')

GATED_N=$(echo "$GATED_IDS" | jq 'length')
AVAIL=$(echo "$CANDIDATES" | jq 'length')
PROMOTE_N=$(( NEED < AVAIL ? NEED : AVAIL ))

echo "signals-promote: candidates=$AVAIL (mobile-native-gated excluded=$GATED_N) need=$NEED cap=$SIGNALS_PROMOTE_CAP capped=$CAPPED"

if [ "$PROMOTE_N" -le 0 ]; then
  echo "signals-promote: no landable signal candidates available — buffer stays starved"
  exit 0
fi

PROMOTE=$(echo "$CANDIDATES" | jq --argjson n "$PROMOTE_N" '.[0:$n]')
echo "signals-promote: promoting $PROMOTE_N of $AVAIL candidate(s)"
echo "$PROMOTE" | jq -r '.[] | "  + \(.id)  [\(.priority)]  \(.action)"'

if [ "$APPLY" = "0" ]; then
  echo "  (dry-run — pass --apply to promote them)"
  exit 0
fi

ITEMS_BEFORE=$(jq '.items | length' "$ACTION_LIST")
TMP=$(mktemp)
trap 'rm -f "$TMP"' EXIT

jq --argjson promote "$PROMOTE" '.items += $promote' "$ACTION_LIST" > "$TMP"

ITEMS_AFTER=$(jq '.items | length' "$TMP")
if [ "$ITEMS_AFTER" -ne "$(( ITEMS_BEFORE + PROMOTE_N ))" ]; then
  echo "signals-promote: ABORT — item count $ITEMS_BEFORE + $PROMOTE_N != $ITEMS_AFTER; refusing to write" >&2
  exit 2
fi

mv "$TMP" "$ACTION_LIST"
trap - EXIT
echo "signals-promote: promoted $PROMOTE_N item(s); action-list items $ITEMS_BEFORE → $ITEMS_AFTER (honest_claimable $HONEST → $(( HONEST + PROMOTE_N )))"
