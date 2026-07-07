#!/usr/bin/env bash
# Smoke test for retry-remint.sh — deterministic, offline.
# Exercises: retryable classification (pr_number=null only), cool-down gate,
# retry-round cap, landed-stem exclusion, active-stem exclusion, live
# action-list stem exclusion (T24), minted-id idempotency, retry_of/round
# stamping, per-run cap + ceil headroom, append-only count guard.
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$HERE/retry-remint.sh"
FAIL=0
ok()   { printf '  ok    %s\n' "$1"; }
bad()  { printf '  FAIL  %s\n' "$1"; FAIL=1; }

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

AL="$TMP/action-list.json"
ALA="$TMP/action-list-archive.json"
ASG="$TMP/assignments.json"
ASGA="$TMP/assignments-archive.json"
NOW="2026-07-07T12:00:00Z"          # fixtures fail at 2026-06-20 → 17d ago

seed() {
  cat > "$AL" <<'EOF'
{ "version": 1, "items": [
  { "id": "unrelated-open", "action": "x", "status": "open", "priority": "low", "depends_on": [] },
  { "id": "live-stem-task", "action": "y", "status": "in-progress", "priority": "low", "depends_on": [] }
] }
EOF
  cat > "$ALA" <<'EOF'
{ "version": 1, "items": [
  { "id": "bug-retryable", "action": "Fix the retryable bug", "owner_role": "pm-backend", "priority": "high", "status": "dropped" }
] }
EOF
  cat > "$ASG" <<'EOF'
{ "assignments": [
  { "task_id": "active-stem-task", "status": "review", "pr_number": 5,
    "last_updated": "2026-06-20T00:00:00Z" }
] }
EOF
  cat > "$ASGA" <<'EOF'
{ "assignments": [
  { "task_id": "bug-retryable",       "status": "failed", "pr_number": null,
    "last_updated": "2026-06-20T00:00:00Z" },
  { "task_id": "bug-had-pr",          "status": "failed", "pr_number": 42,
    "last_updated": "2026-06-20T00:00:00Z" },
  { "task_id": "bug-too-fresh",       "status": "failed", "pr_number": null,
    "last_updated": "2026-07-06T00:00:00Z" },
  { "task_id": "bug-landed",          "status": "failed", "pr_number": null,
    "last_updated": "2026-06-20T00:00:00Z" },
  { "task_id": "bug-landed-retry1",   "status": "merged", "pr_number": 43,
    "last_updated": "2026-06-21T00:00:00Z" },
  { "task_id": "bug-exhausted",        "status": "failed", "pr_number": null,
    "last_updated": "2026-06-01T00:00:00Z" },
  { "task_id": "bug-exhausted-retry1", "status": "failed", "pr_number": null,
    "last_updated": "2026-06-08T00:00:00Z" },
  { "task_id": "bug-exhausted-retry2", "status": "failed", "pr_number": null,
    "last_updated": "2026-06-15T00:00:00Z" },
  { "task_id": "active-stem-task-v2", "status": "failed", "pr_number": null,
    "last_updated": "2026-06-20T00:00:00Z" },
  { "task_id": "live-stem-task-v2",   "status": "failed", "pr_number": null,
    "last_updated": "2026-06-20T00:00:00Z" },
  { "task_id": "bug-second-round",        "status": "failed", "pr_number": null,
    "last_updated": "2026-06-10T00:00:00Z" },
  { "task_id": "bug-second-round-retry1", "status": "failed", "pr_number": null,
    "last_updated": "2026-06-15T00:00:00Z" }
] }
EOF
}

run() {
  ACTION_LIST="$AL" ACTION_ARCHIVE="$ALA" ASSIGN="$ASG" ASSIGN_ARCHIVE="$ASGA" \
  RETRY_NOW="$NOW" bash "$SCRIPT" "$@"
}

echo "T1 dry-run: eligibility filter"
seed
OUT=$(run)
grep -q "bug-retryable-retry1"    <<<"$OUT" && ok "retryable no-PR failure minted" || bad "bug-retryable missing: $OUT"
grep -q "bug-second-round-retry2" <<<"$OUT" && ok "second round minted as -retry2" || bad "bug-second-round-retry2 missing"
grep -q "bug-had-pr"       <<<"$OUT" && bad "failure WITH a PR must not be retried" || ok "pr_number!=null excluded"
grep -q "bug-too-fresh"    <<<"$OUT" && bad "fresh failure must wait out cool-down" || ok "cool-down enforced"
grep -q "bug-landed"       <<<"$OUT" && bad "landed stem must not re-mint" || ok "merged-in-stem excluded"
grep -q "bug-exhausted"    <<<"$OUT" && bad "exhausted retry budget must stop" || ok "retry cap (2 rounds) enforced"
grep -q "active-stem-task" <<<"$OUT" && bad "active assignment stem must block" || ok "active-stem excluded"
grep -q "live-stem-task"   <<<"$OUT" && bad "live action-list stem must block (T24)" || ok "live action-list stem excluded"
[ "$(jq '.items | length' "$AL")" = "2" ] && ok "dry-run leaves file untouched" || bad "dry-run mutated action-list"

echo "T2 apply: row shape + metadata inheritance + append-only"
run --apply >/dev/null
ROW=$(jq '.items[] | select(.id=="bug-retryable-retry1")' "$AL")
[ -n "$ROW" ] && ok "row appended" || bad "row not appended"
[ "$(jq -r '.retry_of'    <<<"$ROW")" = "bug-retryable" ] && ok "retry_of stamped"    || bad "retry_of wrong"
[ "$(jq -r '.retry_round' <<<"$ROW")" = "1" ]             && ok "retry_round stamped" || bad "retry_round wrong"
[ "$(jq -r '.priority'    <<<"$ROW")" = "high" ]          && ok "priority inherited from archive" || bad "priority not inherited"
[ "$(jq -r '.owner_role'  <<<"$ROW")" = "pm-backend" ]    && ok "owner_role inherited" || bad "owner_role not inherited"
[ "$(jq -r '.status'      <<<"$ROW")" = "open" ]          && ok "minted open"          || bad "status wrong"
[ "$(jq -r '.first_open_at' <<<"$ROW")" = "$NOW" ]        && ok "first_open_at=NOW"    || bad "first_open_at wrong"
[ "$(jq -r '.depends_on | length' <<<"$ROW")" = "0" ]     && ok "depends_on empty"     || bad "depends_on wrong"
ROW2=$(jq '.items[] | select(.id=="bug-second-round-retry2")' "$AL")
[ "$(jq -r '.priority' <<<"$ROW2")" = "medium" ] && ok "no-archive-metadata defaults to medium" || bad "default priority wrong"
[ "$(jq '.items | length' "$AL")" = "4" ] && ok "count grew by exactly the mint count" || bad "count wrong"

echo "T3 idempotent: second apply mints nothing"
OUT=$(run --apply)
grep -q "nothing to re-mint" <<<"$OUT" && ok "second run no-op (minted ids now live stems)" || bad "not idempotent: $OUT"

echo "T4 per-run cap"
seed
OUT=$(RETRY_REMINT_CAP=1 ACTION_LIST="$AL" ACTION_ARCHIVE="$ALA" ASSIGN="$ASG" ASSIGN_ARCHIVE="$ASGA" RETRY_NOW="$NOW" bash "$SCRIPT" --apply)
grep -q "re-minting 1 of 2" <<<"$OUT" && ok "cap=1 mints exactly 1" || bad "cap not enforced: $OUT"
grep -q "bug-second-round-retry2" <<<"$OUT" && ok "oldest failure wins under cap" || bad "deterministic order broken"

echo "T5 ceil headroom"
seed
OUT=$(BUFFER_CEIL=1 ACTION_LIST="$AL" ACTION_ARCHIVE="$ALA" ASSIGN="$ASG" ASSIGN_ARCHIVE="$ASGA" RETRY_NOW="$NOW" bash "$SCRIPT" --apply)
grep -q "no headroom" <<<"$OUT" && ok "ceil blocks minting" || bad "ceil not enforced: $OUT"

echo "T6 missing files no-op"
OUT=$(ACTION_LIST="$TMP/nope.json" bash "$SCRIPT" 2>&1); [ $? -eq 0 ] \
  && ok "missing action-list exits 0" || bad "missing action-list errored"
seed
OUT=$(ACTION_LIST="$AL" ASSIGN="$TMP/no1.json" ASSIGN_ARCHIVE="$TMP/no2.json" bash "$SCRIPT" 2>&1)
grep -q "no failures to retry" <<<"$OUT" && ok "missing assignments exits clean" || bad "missing assignments: $OUT"

echo
[ "$FAIL" = "0" ] && { echo "ALL PASS"; exit 0; } || { echo "FAILURES"; exit 1; }
