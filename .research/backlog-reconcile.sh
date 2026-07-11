#!/usr/bin/env bash
# backlog-reconcile.sh — make backlog.json honest against the dispatcher's
# assignment ledger (active + archive).
#
# Why this script exists (dispatcher starvation, 2026-07-07):
#
#   backlog-refill.sh (Tier 1b) promotes `open`/`ready` backlog vectors into
#   the action-list, excluding any id/stem already present in assignments
#   (active + archive). But nothing ever reconciled backlog.json AGAINST that
#   ledger: rows whose assignment terminated weeks ago (merged/done/failed)
#   still read `open`/`ready`. Observed 2026-07-07: ALL 20 open/ready backlog
#   rows were already terminal in assignments-archive (mostly `failed` with
#   pr_number=null), so Tier-1b evaluated and rejected the same 20 ghosts
#   every run, logged `avail=0`, and the buffer stayed starved while the
#   backlog LOOKED full. The lie also hides true supply-side starvation from
#   the research routine and the operator.
#
# What it does (idempotent, count-invariant, append-nothing):
#
#   For each backlog item with status open|ready whose EXACT id has a
#   terminal row in assignments.json / assignments-archive.json:
#     - assignment merged/done  -> backlog status "done"
#     - assignment failed       -> backlog status "dropped"
#   Every flipped row gets a `reconciled` evidence stamp:
#     { at, assignment_status, pr_number } — never a silent flip.
#   Rows are NEVER added or removed; non-open/ready rows are never touched.
#   Retry candidacy is NOT lost by the drop: retry-remint.sh reads the
#   ARCHIVE (not backlog.json) to decide what deserves a -v2.
#
# Usage:
#   ./.research/backlog-reconcile.sh            # dry-run: print planned flips
#   ./.research/backlog-reconcile.sh --apply    # write backlog.json in place
#   Injectables (tests): BACKLOG_FILE, ASSIGN, ASSIGN_ARCHIVE, BACKLOG_RECONCILE_NOW

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BACKLOG_FILE="${BACKLOG_FILE:-$REPO_ROOT/.research/backlog.json}"
ASSIGN="${ASSIGN:-$REPO_ROOT/.research/management/assignments.json}"
ASSIGN_ARCHIVE="${ASSIGN_ARCHIVE:-$REPO_ROOT/.research/management/assignments-archive.json}"
NOW="${BACKLOG_RECONCILE_NOW:-$(date -u +%Y-%m-%dT%H:%M:%SZ)}"
APPLY=0
[ "${1:-}" = "--apply" ] && APPLY=1

if [ ! -f "$BACKLOG_FILE" ]; then
  echo "backlog-reconcile: backlog.json missing — nothing to do: $BACKLOG_FILE" >&2
  exit 0
fi

# Guard BOTH assignment inputs with [ -f ] (same multi-file-safe read as
# backlog-refill.sh — a missing PRIMARY file makes `jq -s` exit 2, #2078).
ASSIGN_INPUTS=()
[ -f "$ASSIGN" ]         && ASSIGN_INPUTS+=("$ASSIGN")
[ -f "$ASSIGN_ARCHIVE" ] && ASSIGN_INPUTS+=("$ASSIGN_ARCHIVE")
if [ "${#ASSIGN_INPUTS[@]}" -eq 0 ]; then
  echo "backlog-reconcile: no assignment files — nothing to reconcile against"
  exit 0
fi

# Scratch dir — large intermediates go through FILES, never argv:
# `--argjson` with a multi-100KB ledger blows ARG_MAX ("Argument list too long").
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

# task_id -> terminal fate map. Later files win per id (archive is the
# authoritative long-term ledger; active file rarely holds terminal rows).
jq -s '
  [ .[].assignments[]?
    | select(.status=="merged" or .status=="done" or .status=="failed")
    | { key: .task_id,
        value: { status: .status, pr_number: (.pr_number // null) } } ]
  | from_entries' "${ASSIGN_INPUTS[@]}" > "$WORK/terminal-map.json"

COUNT_BEFORE=$(jq '.items | length' "$BACKLOG_FILE")

jq \
  --slurpfile term "$WORK/terminal-map.json" \
  --arg now "$NOW" '
  $term[0] as $t
  | .items |= map(
    if (.status == "open" or .status == "ready") and ($t[.id] != null) then
      . + {
        status: (if $t[.id].status == "failed" then "dropped" else "done" end),
        reconciled: {
          at: $now,
          assignment_status: $t[.id].status,
          pr_number: $t[.id].pr_number
        }
      }
    else . end
  )' "$BACKLOG_FILE" > "$WORK/result.json"

COUNT_AFTER=$(jq '.items | length' "$WORK/result.json")
if [ "$COUNT_BEFORE" != "$COUNT_AFTER" ]; then
  echo "backlog-reconcile: ABORT — item count changed $COUNT_BEFORE -> $COUNT_AFTER (must be invariant)" >&2
  exit 1
fi

# Flips = actual status diff between input and output (never inferred from
# the `reconciled` stamp — a fixed-NOW rerun would recount old stamps).
FLIPS=$(jq -n --slurpfile before "$BACKLOG_FILE" --slurpfile after "$WORK/result.json" '
  ($before[0].items | map({key: .id, value: .status}) | from_entries) as $old
  | [ $after[0].items[]
      | select($old[.id] != .status)
      | {id, status, was: .reconciled.assignment_status} ]')
N=$(echo "$FLIPS" | jq 'length')

if [ "$N" = "0" ]; then
  echo "backlog-reconcile: backlog already honest — 0 flips"
  exit 0
fi

echo "backlog-reconcile: $N stale open/ready row(s) terminal in assignments:"
echo "$FLIPS" | jq -r '.[] | "  \(.id): -> \(.status) (assignment=\(.was))"'

if [ "$APPLY" = "1" ]; then
  mv "$WORK/result.json" "$BACKLOG_FILE"
  echo "backlog-reconcile: applied $N flip(s) to $BACKLOG_FILE"
else
  echo "backlog-reconcile: dry-run — pass --apply to write"
fi
