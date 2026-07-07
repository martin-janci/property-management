#!/usr/bin/env bash
# dev-reconcile.sh — close OPEN action-list items whose work already landed on
# the integration branch but which left NO dispatcher archive row.
#
# Why this script exists (GH #1380 defect 1 — "stale gap-scan buffer"):
#
#   gc1-reconcile.sh closes an open gap item only when its EXACT task_id is
#   already merged/done in assignments-archive.json — i.e. the dispatcher's own
#   merge ledger. But work also lands on dev OUT OF BAND: a coverage feature is
#   implemented under a different branch/PR, a sibling race-merges, or an
#   implementer squash-merges without the dispatcher ever persisting an archive
#   row. Those merges leave the action-list row `open`, so the next buffer-low
#   refill re-claims work that is already shipped — the dispatcher logs
#   `0 claimed … 0 merged` while no-op implementers churn.
#
#   This pass reconciles open action-list items against the ACTUAL integration
#   branch ($BASE, default origin/dev), independent of the archive ledger, and
#   closes the leak the same way gc1-reconcile does: status open -> done, with a
#   `dev_reconciled` evidence stamp (landing commit + subject).
#
# Join key — subject-prefix, NOT substring:
#
#   A commit on $BASE whose SUBJECT begins with "<id>:" is the squash-merge
#   convention used by implementer PRs (e.g.
#   `feat-airbnb-...-backend: Airbnb OAuth token-exchange ... (#1388)`).
#   Matching the subject PREFIX (anchored, up to the first colon) — never a
#   loose body grep — is what keeps a dispatcher `chore(research)` commit that
#   merely lists an id in its body from being a false positive. (A naive
#   `git log --grep=<id>` matches those chore bodies; this does not.)
#
# Idempotent + re-runnable: after --apply the closed rows are status=done so
# they no longer match the open set; re-running on a clean state is a no-op.
# Fail-safe: this pass only ever flips status open->done and never adds/removes
# rows, so the item count is invariant — it is guarded below.
#
# Usage:
#   ./.research/dev-reconcile.sh                 # dry-run: print landed-but-open set, write nothing
#   ./.research/dev-reconcile.sh --apply         # close landed rows in place + stamp evidence
#   BASE=origin/main ./.research/dev-reconcile.sh [--apply]
#   DEV_LOG_FILE=fixture.tsv ./.research/dev-reconcile.sh [--apply]   # offline test injection
set -euo pipefail

ACTION_LIST="${ACTION_LIST:-.research/management/action-list.json}"
BASE="${BASE:-origin/dev}"
NOW="${DEV_RECONCILE_NOW:-$(date -u +%Y-%m-%dT%H:%M:%SZ)}"
APPLY=0
[ "${1:-}" = "--apply" ] && APPLY=1

if [ ! -f "$ACTION_LIST" ]; then
  echo "dev-reconcile: active file missing — nothing to do: $ACTION_LIST" >&2
  exit 0
fi

# Source of landed commit subjects: one "<sha>\t<subject>" per line. Default is
# `git log` on $BASE; DEV_LOG_FILE overrides it so the script is unit-testable
# offline (and so a run with an unreachable remote degrades to "nothing landed"
# rather than aborting the dispatcher).
if [ -n "${DEV_LOG_FILE:-}" ]; then
  LOG_SRC=$(cat "$DEV_LOG_FILE")
else
  LOG_SRC=$(git log "$BASE" --pretty='%H%x09%s' 2>/dev/null || true)
fi

# Build the landed-but-open set: for each OPEN item id, find the first commit
# whose subject begins with "<id>:". (Only `open` is reconciled — in-progress
# rows have a live implementer in assignments.json and are left alone, mirroring
# gc1-reconcile's open-only scope.)
LANDED_ROWS='[]'
OPEN_IDS=$(jq -r '.items[] | select(.status=="open") | .id' "$ACTION_LIST")
while IFS= read -r id; do
  [ -z "$id" ] && continue
  # Anchored, literal match of "<sha>\t<id>:" — subject prefix only.
  line=$(printf '%s\n' "$LOG_SRC" | grep -m1 -E "^[0-9a-f]+	${id}:" || true)
  [ -z "$line" ] && continue
  sha=${line%%	*}
  subj=${line#*	}
  LANDED_ROWS=$(jq -c --arg id "$id" --arg sha "$sha" --arg subj "$subj" \
    '. + [{id:$id, commit:$sha, subject:$subj}]' <<<"$LANDED_ROWS")
done <<<"$OPEN_IDS"

LANDED_N=$(jq 'length' <<<"$LANDED_ROWS")
ITEMS_BEFORE=$(jq '.items | length' "$ACTION_LIST")

echo "==> dev-reconcile (apply=$APPLY, base=$BASE)  landed-but-open=$LANDED_N" >&2
jq -r '.[] | "  \(.id)  <- \(.commit[0:9]) \(.subject)"' <<<"$LANDED_ROWS" >&2

if [ "$LANDED_N" = "0" ]; then
  echo "==> clean: no open action-list item has already landed on $BASE." >&2
  exit 0
fi

if [ "$APPLY" != "1" ]; then
  echo "==> dry-run: nothing written. Re-run with --apply to close the rows above." >&2
  exit 0
fi

TMP=$(mktemp)
trap 'rm -f "$TMP"' EXIT

# -c (compact, one-line) to match action-list.json's on-disk convention and keep
# the diff minimal against the dispatcher-written file (the LLM emits compact).
jq -c --argjson landed "$LANDED_ROWS" --arg now "$NOW" '
  (reduce $landed[] as $l ({}; .[$l.id] = $l)) as $by
  | .items |= map(
      if ($by[.id] != null and .status=="open")
      then .status = "done"
         | .source = ((.source // "") | if .=="" then "dev-reconcile \($now)" else . + " | dev-reconcile \($now)" end)
         | .dev_reconciled = {reason:"landed-on-dev", commit:$by[.id].commit, subject:$by[.id].subject, at:$now}
      else . end)
' "$ACTION_LIST" > "$TMP"

# Fail-safe: this pass only flips status; the item count must be invariant.
ITEMS_AFTER=$(jq '.items | length' "$TMP")
if [ "$ITEMS_AFTER" -ne "$ITEMS_BEFORE" ]; then
  echo "dev-reconcile: ABORT — item count changed $ITEMS_BEFORE -> $ITEMS_AFTER; refusing to write." >&2
  exit 2
fi

mv "$TMP" "$ACTION_LIST"
trap - EXIT
echo "==> closed $LANDED_N landed-but-open row(s) in $ACTION_LIST (status open->done + dev_reconciled evidence)." >&2
echo "    (terminal rows are swept to the archive by action-list-reconcile.sh in Phase 6.)" >&2
