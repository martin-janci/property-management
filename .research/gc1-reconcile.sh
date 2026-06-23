#!/usr/bin/env bash
# GC1 referential-integrity reconciler — sibling of goal-check.sh.
#
# Drains the two ACTIONABLE classes of GC1 violation (see goal-check.sh GC1):
#
#   (1) archive-terminal LEAK — an OPEN action-list item of ANY id shape whose
#       exact task_id is already terminal in assignments-archive.json. The work
#       is settled; the action-list row was never closed. SAFE to auto-close:
#       merged/done → status=done; failed → status=dropped, stamping the
#       archive evidence. This is the same leak as finding
#       reclaim-of-already-merged-task-id, drained at the coverage layer.
#       (issue #1747, #1739: the old pass only matched gap-* ids, so
#       code-review-*, test-gap-*, screen-map-*, churn-hotspot-*, triage-* etc.
#       leaked — terminal in the archive yet still status=open, inflating the
#       claimable pool and near-re-claiming merged work.)
#
#   (2) stem ORPHAN — a coverage-keyed gap item (id ~ gap-<epic>-<story>-…)
#       whose <epic>-<story> stem maps to NO coverage story. These are real
#       roadmap stories missing from coverage.json (e.g. 9-2, 10a-4, 82-6..82-9).
#       NOT auto-modified — emitted to a triage doc for a coverage author to
#       add the story (relink). Auto-pruning would destroy live work.
#
# Legitimate open follow-ups under a done coverage story (exact task NOT yet
# merged) are deliberately left untouched — they are real work, not a leak.
#
# Idempotent + re-runnable: after --apply, the closed rows are status=done so
# they no longer match the open-leak set; orphans persist until coverage is
# authored. Default DRY-RUN; pass --apply to write.
#
# Usage:
#   ./.research/gc1-reconcile.sh            # dry-run: print both sets, write nothing
#   ./.research/gc1-reconcile.sh --apply    # close leak rows + (re)write triage doc
#   ACTION_LIST=… COVERAGE=… ARCHIVE=… TRIAGE=… ./.research/gc1-reconcile.sh [--apply]
set -euo pipefail

ACTION_LIST="${ACTION_LIST:-.research/management/action-list.json}"
COVERAGE="${COVERAGE:-.research/management/coverage.json}"
ARCHIVE="${ARCHIVE:-.research/management/assignments-archive.json}"
TRIAGE="${TRIAGE:-.research/management/gc1-orphan-triage.md}"
NOW="${GC1_NOW:-$(date -u +%Y-%m-%dT%H:%M:%SZ)}"
APPLY=0
[ "${1:-}" = "--apply" ] && APPLY=1

# Same <epic>-<story> stem extractor as goal-check.sh GC1 — keep in lockstep.
STEM='capture("^(?<s>[0-9]+[a-z]?-[0-9]+)").s'

# --- (1) archive-terminal leak: open action-list items whose EXACT id is
#         terminal in the archive — ANY id shape, not just gap-* (issue #1747,
#         #1739). The archive is the source of truth for shipped/abandoned
#         work: merged/done → close as done; failed → close as dropped. The
#         old pass only matched gap-* ids, so code-review-*, test-gap-*,
#         screen-map-*, churn-hotspot-*, triage-* items whose work was already
#         terminal stayed status=open, inflating open_claimable_count and
#         near-re-claiming merged work. We map the close target per terminal
#         status: a failed archive row means the task was abandoned, so the
#         open action-list item is dropped (not re-presented as claimable),
#         while merged/done means the work shipped (done).
#
# Each LEAK row carries `to` = the status we close the open item to, so the
# apply pass needs no second lookup. PR/date evidence is best-effort (failed
# rows usually have neither).
LEAK_ROWS=$(jq -c --slurpfile arc "$ARCHIVE" '
  ($arc[0].assignments
    | map(select(.status=="merged" or .status=="done" or .status=="failed"))
    # Dedup by task_id keeping the freshest (last) archive row, mirroring the
    # group_by/last semantics archive-reconcile.sh uses, so a task that failed
    # then later merged closes as done, not dropped.
    | group_by(.task_id) | map(last)
    | map({key:.task_id,
           value:{pr:.pr_number, at:.merged_at, st:.status,
                  to:(if .status=="failed" then "dropped" else "done" end)}})
    | from_entries) as $term
  | [ .items[]
      | select(.status=="open")
      | select($term[.id] != null)
      | {id, pr:$term[.id].pr, at:$term[.id].at, st:$term[.id].st, to:$term[.id].to} ]' "$ACTION_LIST")
LEAK_N=$(jq 'length' <<<"$LEAK_ROWS")

# --- (2) stem orphans: coverage-keyed gaps whose stem has no coverage story ---
ORPHAN_ROWS=$(jq -c --slurpfile cov "$COVERAGE" "
  (\$cov[0].stories | map(.id | $STEM) | unique) as \$cstems
  | [ .items[]
      | select(.id | test(\"^gap-[0-9]+[a-z]?-[0-9]+\"))
      | . as \$it | ((.id|ltrimstr(\"gap-\")) | $STEM) as \$g
      | select((\$cstems | index(\$g)) == null)
      | {id, stem:\$g, status:\$it.status} ]" "$ACTION_LIST")
ORPHAN_N=$(jq 'length' <<<"$ORPHAN_ROWS")

echo "==> gc1-reconcile (apply=$APPLY)  leak=$LEAK_N  orphans=$ORPHAN_N" >&2
echo "--- archive-terminal leak (open → done|dropped, any id shape) ---" >&2
jq -r '.[] | "  \(.id)  archive=\(.st) → \(.to)  (PR#\(.pr // "?") at \(.at // "?"))"' <<<"$LEAK_ROWS" >&2
echo "--- stem orphans (coverage missing — triage, NOT auto-closed) ---" >&2
jq -r '.[] | "  \(.id)  stem=\(.stem) status=\(.status)"' <<<"$ORPHAN_ROWS" >&2

if [ "$APPLY" != "1" ]; then
  echo "==> dry-run: nothing written. Re-run with --apply to close leak rows + write $TRIAGE." >&2
  exit 0
fi

# --- apply (1): close the leak rows in place, stamping archive evidence ---
# Close ANY open item whose exact id is terminal in the archive (issue #1747,
# #1739): merged/done → done, failed → dropped. Idempotent — only `open` rows
# are touched, and the per-row close target comes from `$l.to` computed above.
if [ "$LEAK_N" -gt 0 ]; then
  jq --argjson leak "$LEAK_ROWS" --arg now "$NOW" '
    (reduce $leak[] as $l ({}; .[$l.id] = $l)) as $by
    | .items |= map(
        if ($by[.id] != null and .status=="open")
        then .status = $by[.id].to
           | .source = ((.source // "") | if .=="" then "gc1-reconcile \($now)" else . + " | gc1-reconcile \($now)" end)
           | .gc1_closed = {reason:"archive-terminal", archive_status:$by[.id].st, merged_pr:$by[.id].pr, merged_at:$by[.id].at, at:$now}
        else . end)
  ' "$ACTION_LIST" > "$ACTION_LIST.tmp" && mv "$ACTION_LIST.tmp" "$ACTION_LIST"
  echo "==> closed $LEAK_N leak row(s) in $ACTION_LIST (status open→done|dropped + gc1_closed evidence)." >&2
fi

# --- apply (2): (re)write the orphan triage doc (overwrite — current snapshot) ---
{
  echo "# GC1 orphan triage — coverage-authoring candidates"
  echo ""
  echo "_Generated by \`gc1-reconcile.sh --apply\` at $NOW. Re-generated each run._"
  echo ""
  echo "These action-list items are coverage-keyed (\`gap-<epic>-<story>-…\`) but"
  echo "their \`<epic>-<story>\` stem has **no matching story in coverage.json**."
  echo "They are NOT orphaned work — they are real stories coverage never scanned."
  echo "Action: add the missing coverage story (relink), then re-run goal-check."
  echo "Pruning the action-list item is wrong unless the story is truly abandoned."
  echo ""
  echo "| action-list id | stem | status | suggested coverage story |"
  echo "|---|---|---|---|"
  jq -r '.[] | "| \(.id) | \(.stem) | \(.status) | (author \(.stem)-… in coverage.json) |"' <<<"$ORPHAN_ROWS"
  echo ""
  echo "_Count: $ORPHAN_N. When this list reaches 0, GC1 orphans=0._"
} > "$TRIAGE"
echo "==> wrote orphan triage → $TRIAGE ($ORPHAN_N item(s))." >&2
