#!/usr/bin/env bash
# Smoke test for backlog-reconcile.sh — deterministic, offline.
# Exercises: merged->done flip, failed->dropped flip, untouched rows
# (non-terminal assignment, no assignment, already-terminal backlog status),
# evidence stamp, count invariance, idempotency, missing-file no-ops.
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$HERE/backlog-reconcile.sh"
FAIL=0
ok()   { printf '  ok    %s\n' "$1"; }
bad()  { printf '  FAIL  %s\n' "$1"; FAIL=1; }

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

BL="$TMP/backlog.json"
ASG="$TMP/assignments.json"
ASGA="$TMP/assignments-archive.json"
NOW="2026-07-07T12:00:00Z"

seed() {
  cat > "$BL" <<'EOF'
{ "version": 1, "items": [
  { "id": "vec-merged",     "status": "open",  "title": "merged upstream" },
  { "id": "vec-failed",     "status": "ready", "title": "failed upstream" },
  { "id": "vec-active",     "status": "open",  "title": "still in review" },
  { "id": "vec-untracked",  "status": "ready", "title": "never assigned" },
  { "id": "vec-done-already","status": "done", "title": "terminal already" }
] }
EOF
  cat > "$ASG" <<'EOF'
{ "assignments": [
  { "task_id": "vec-active", "status": "review", "pr_number": 10 }
] }
EOF
  cat > "$ASGA" <<'EOF'
{ "assignments": [
  { "task_id": "vec-merged",       "status": "merged", "pr_number": 11 },
  { "task_id": "vec-failed",       "status": "failed", "pr_number": null },
  { "task_id": "vec-done-already", "status": "failed", "pr_number": null }
] }
EOF
}

run() { BACKLOG_FILE="$BL" ASSIGN="$ASG" ASSIGN_ARCHIVE="$ASGA" BACKLOG_RECONCILE_NOW="$NOW" bash "$SCRIPT" "$@"; }

echo "T1 dry-run flips nothing"
seed
OUT=$(run)
grep -q "2 stale" <<<"$OUT" && ok "reports 2 flips" || bad "expected 2 flips in: $OUT"
[ "$(jq -r '.items[] | select(.id=="vec-merged").status' "$BL")" = "open" ] \
  && ok "dry-run leaves file untouched" || bad "dry-run mutated the file"

echo "T2 apply: merged->done, failed->dropped, stamps, count invariant"
run --apply >/dev/null
[ "$(jq -r '.items[] | select(.id=="vec-merged").status' "$BL")" = "done" ] \
  && ok "vec-merged -> done" || bad "vec-merged not done"
[ "$(jq -r '.items[] | select(.id=="vec-failed").status' "$BL")" = "dropped" ] \
  && ok "vec-failed -> dropped" || bad "vec-failed not dropped"
[ "$(jq -r '.items[] | select(.id=="vec-merged").reconciled.pr_number' "$BL")" = "11" ] \
  && ok "evidence stamp carries pr_number" || bad "missing/wrong reconciled stamp"
[ "$(jq -r '.items[] | select(.id=="vec-active").status' "$BL")" = "open" ] \
  && ok "active assignment left open" || bad "vec-active wrongly flipped"
[ "$(jq -r '.items[] | select(.id=="vec-untracked").status' "$BL")" = "ready" ] \
  && ok "untracked row untouched" || bad "vec-untracked wrongly flipped"
[ "$(jq -r '.items[] | select(.id=="vec-done-already").status' "$BL")" = "done" ] \
  && ok "already-terminal backlog row untouched" || bad "vec-done-already rewritten"
[ "$(jq '.items | length' "$BL")" = "5" ] \
  && ok "item count invariant" || bad "item count changed"

echo "T3 idempotent: second apply is a no-op"
OUT=$(run --apply)
grep -q "0 flips" <<<"$OUT" && ok "second run flips nothing" || bad "not idempotent: $OUT"

echo "T4 missing files no-op"
OUT=$(BACKLOG_FILE="$TMP/nope.json" ASSIGN="$ASG" ASSIGN_ARCHIVE="$ASGA" bash "$SCRIPT" 2>&1)
[ $? -eq 0 ] && ok "missing backlog exits 0" || bad "missing backlog errored"
OUT=$(BACKLOG_FILE="$BL" ASSIGN="$TMP/no1.json" ASSIGN_ARCHIVE="$TMP/no2.json" bash "$SCRIPT" 2>&1)
grep -q "no assignment files" <<<"$OUT" && ok "missing assignments exits clean" || bad "missing assignments: $OUT"

echo
[ "$FAIL" = "0" ] && { echo "ALL PASS"; exit 0; } || { echo "FAILURES"; exit 1; }
