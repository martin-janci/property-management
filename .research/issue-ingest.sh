#!/usr/bin/env bash
# issue-ingest.sh — pull OPEN GitHub issues into the dispatcher's action-list
# so they get claimed, implemented, and closed. Target: get things done.
#
# Why this script exists (2026-07-02):
#
#   The dispatcher's backlog is synthetic (research-routine vectors +
#   coverage). Real work filed as GitHub issues never entered the pipeline,
#   so open issues sat untouched. This ingests them as first-class
#   action-list rows (`gh-issue-<N>`), deterministically and idempotently.
#
# Division of labour (the cron can't run `gh` — proxy-403'd, #958):
#
#   The DISPATCHER fetches open issues via the GitHub MCP and writes them to
#   a JSON file; THIS script does the deterministic merge. Same offline-
#   injection contract as dev-reconcile.sh's DEV_LOG_FILE — so it is unit-
#   testable with a fixture and a run with no issues degrades to a no-op.
#
#   GH_ISSUES_FILE must contain a JSON array of issue objects, each with at
#   least: { "number": int, "title": str, "labels": [ {"name": str} | str ],
#   "html_url"?: str, "pull_request"?: any }. This is exactly the shape of
#   `mcp__github__list_issues` / the REST `GET /issues?state=open` payload.
#
# Filtering:
#   - PRs are skipped (any object carrying a `pull_request` key).
#   - Issues whose labels intersect EXCLUDE_LABELS are skipped
#     (default: epic,discussion,question,wontfix,duplicate,blocked,needs-triage).
#   - Already-tracked issues are skipped: `gh-issue-<N>` present as an
#     action-list id/stem or an assignment id/stem, OR any existing
#     action-list row that already references `#<N>` in its action/source
#     (best-effort dup guard for issues tracked under a different slug).
#
# Priority from labels: security|critical|bug -> high; enhancement|backend|
#   frontend|mobile|follow-up|from-merged-review -> medium; else -> low.
#
# UNLIKE backlog-refill (a buffer top-up gated on BUFFER_FLOOR), issue-ingest
# runs EVERY cycle: any untracked open issue is pulled in regardless of buffer
# level — real deliverables should always enter the pipeline. Still bounded:
#   - never lifts the active item count past BUFFER_CEIL headroom,
#   - at most ISSUE_INGEST_CAP issues per run (default 12),
#   - append-only; fail-closed if the count doesn't grow by exactly N.
#
# The dispatcher closes issue #N via the GitHub MCP when the `gh-issue-<N>`
# PR merges (a dev-targeted "Closes #N" does NOT auto-close, since `main` —
# not `dev` — is the default branch). The `Closes #<N>` in the action text is
# for the PR body; the explicit MCP close is the real done-signal.
#
# Usage:
#   GH_ISSUES_FILE=issues.json ./.research/issue-ingest.sh            # dry-run
#   GH_ISSUES_FILE=issues.json ./.research/issue-ingest.sh --apply    # ingest
#   ACTION_LIST=… ASSIGN=… ASSIGN_ARCHIVE=… BUFFER_CEIL=… \
#     ISSUE_INGEST_CAP=… EXCLUDE_LABELS=… ISSUE_INGEST_NOW=… … [--apply]

set -euo pipefail

ACTION_LIST="${ACTION_LIST:-.research/management/action-list.json}"
ASSIGN="${ASSIGN:-.research/management/assignments.json}"
ASSIGN_ARCHIVE="${ASSIGN_ARCHIVE:-.research/management/assignments-archive.json}"

BUFFER_CEIL="${BUFFER_CEIL:-120}"
ISSUE_INGEST_CAP="${ISSUE_INGEST_CAP:-12}"
EXCLUDE_LABELS="${EXCLUDE_LABELS:-epic,discussion,question,wontfix,duplicate,blocked,needs-triage}"
NOW="${ISSUE_INGEST_NOW:-$(date -u +%Y-%m-%dT%H:%M:%SZ)}"

APPLY=0
[ "${1:-}" = "--apply" ] && APPLY=1

if [ ! -f "$ACTION_LIST" ]; then
  echo "issue-ingest: action-list missing — nothing to do: $ACTION_LIST" >&2
  exit 0
fi
if [ -z "${GH_ISSUES_FILE:-}" ] || [ ! -f "${GH_ISSUES_FILE:-}" ]; then
  echo "issue-ingest: no GH_ISSUES_FILE — dispatcher must fetch open issues via GitHub MCP first (no-op)" >&2
  exit 0
fi

# Assignment id set (active + archive), multi-file-safe via `jq -s`.
# Guard BOTH inputs symmetrically with `[ -f ]`: a missing PRIMARY
# assignments.json makes `jq -s "$ASSIGN"` exit 2 ("Could not open file"),
# which under `set -euo pipefail` aborts the whole ingest — the `.assignments[]?`
# optional operator only tolerates a missing *key*, not a missing *file* (#2078).
# Default to an empty id set when neither file exists.
ASSIGN_INPUTS=()
[ -f "$ASSIGN" ]         && ASSIGN_INPUTS+=("$ASSIGN")
[ -f "$ASSIGN_ARCHIVE" ] && ASSIGN_INPUTS+=("$ASSIGN_ARCHIVE")
if [ "${#ASSIGN_INPUTS[@]}" -eq 0 ]; then
  ASSIGN_IDS='[]'
else
  ASSIGN_IDS=$(jq -s '[ .[].assignments[]?.task_id ] | unique' "${ASSIGN_INPUTS[@]}")
fi

ACTIVE_BEFORE=$(jq '.items | length' "$ACTION_LIST")
# CEIL headroom is measured against the active item count (append-bound).
HEADROOM=$(( BUFFER_CEIL - ACTIVE_BEFORE ))
[ "$HEADROOM" -lt 0 ] && HEADROOM=0
LIMIT=$(( HEADROOM < ISSUE_INGEST_CAP ? HEADROOM : ISSUE_INGEST_CAP ))

# Candidate issues → action-list rows. Filter PRs + excluded labels + already
# tracked, map labels→priority, stamp issue_ref + Closes #N, sort by number ASC
# (deterministic; lower/older issue numbers first).
CANDIDATES=$(jq -n \
  --slurpfile iss "$GH_ISSUES_FILE" \
  --slurpfile al "$ACTION_LIST" \
  --argjson aids "$ASSIGN_IDS" \
  --arg now "$NOW" \
  --arg excl "$EXCLUDE_LABELS" '
  def stem: sub("-(impl|fix|v2|retry|followup|wip)[0-9]*$";"");
  def lname: if type=="object" then .name else . end;       # label may be obj or string
  ($excl | split(",") | map(gsub("^\\s+|\\s+$";"")) | map(select(length>0))) as $xl
  | ([ $al[0].items[]?.id ] | unique) as $al_ids
  | ([ $al[0].items[]? | select(.status != "done" and .status != "dropped") | (.id | stem) ] | unique) as $al_active_stems
  | ($aids | unique) as $a_ids
  | ($aids | map(stem) | unique) as $a_stems
  # existing "#<N>" references in any action-list row (dup guard for issues
  # tracked under a non gh-issue-<N> slug).
  | ([ $al[0].items[]? | ((.action // "") + " " + (.source // "")) ] | join("  ")) as $al_text

  | [ $iss[0][]
      | select(has("pull_request") | not)                    # drop PRs
      | . as $i
      | ([ (.labels // [])[] | lname ]) as $lbls
      | select( ($lbls | map(. as $l | $xl | index($l)) | map(select(. != null)) | length) == 0 )  # no excluded label
      | ("gh-issue-" + (.number|tostring)) as $tid
      | select(($tid | IN($al_ids[])) | not)                 # not already an action-list id
      | select(($tid | stem | IN($al_active_stems[])) | not) # nor a non-terminal stem
      | select(($tid | IN($a_ids[])) | not)                  # nor an assignment id
      | select(($tid | stem | IN($a_stems[])) | not)
      | select( ($al_text | test("(^|[^0-9])#" + ($i.number|tostring) + "([^0-9]|$)")) | not )  # not referenced already
      | {
          id:         $tid,
          action:     ((.title // $tid) + " (Closes #" + (.number|tostring) + ")"),
          owner_role: (
            if   ($lbls | any(. == "security" or . == "dependencies")) then "pm-security"
            elif ($lbls | any(. == "frontend" or . == "mobile"))       then "pm-frontend"
            elif ($lbls | any(. == "backend"))                          then "pm-backend"
            else "pm-tech-lead" end),
          priority: (
            if   ($lbls | any(. == "security" or . == "critical" or . == "bug")) then "high"
            elif ($lbls | any(. == "enhancement" or . == "backend" or . == "frontend"
                              or . == "mobile" or . == "follow-up" or . == "from-merged-review")) then "medium"
            else "low" end),
          status:     "open",
          source:     ("dispatcher-issue-ingest " + $now + " (#" + (.number|tostring) + ")"),
          deadline:   null,
          dependency: null,
          depends_on: [],
          first_open_at: $now,
          issue_ref:  { number: .number, url: (.html_url // null), labels: $lbls }
        }
    ]
  | sort_by(.issue_ref.number)
')

AVAIL=$(echo "$CANDIDATES" | jq 'length')
INGEST_N=$(( LIMIT < AVAIL ? LIMIT : AVAIL ))
[ "$INGEST_N" -lt 0 ] && INGEST_N=0
CAPPED=0
[ "$AVAIL" -gt "$LIMIT" ] && CAPPED=1

echo "issue-ingest: untracked open issues=$AVAIL ceil-headroom=$HEADROOM cap=$ISSUE_INGEST_CAP -> ingest=$INGEST_N (capped=$CAPPED)"

if [ "$INGEST_N" -le 0 ]; then
  echo "issue-ingest: nothing to ingest"
  exit 0
fi

INGEST=$(echo "$CANDIDATES" | jq --argjson n "$INGEST_N" '.[0:$n]')
echo "$INGEST" | jq -r '.[] | "  + \(.id)  [\(.priority)]  \(.action)"'

if [ "$APPLY" = "0" ]; then
  echo "  (dry-run — pass --apply to ingest them)"
  exit 0
fi

TMP=$(mktemp)
trap 'rm -f "$TMP"' EXIT
jq --argjson ingest "$INGEST" '.items += $ingest' "$ACTION_LIST" > "$TMP"

ACTIVE_AFTER=$(jq '.items | length' "$TMP")
if [ "$ACTIVE_AFTER" -ne "$(( ACTIVE_BEFORE + INGEST_N ))" ]; then
  echo "issue-ingest: ABORT — item count $ACTIVE_BEFORE + $INGEST_N != $ACTIVE_AFTER; refusing to write" >&2
  exit 2
fi
mv "$TMP" "$ACTION_LIST"
trap - EXIT
echo "issue-ingest: ingested $INGEST_N issue(s); action-list items $ACTIVE_BEFORE → $ACTIVE_AFTER"
