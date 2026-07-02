#!/usr/bin/env bash
# Smoke test for backlog-refill.sh — deterministic, offline (no git, no network).
# Builds fixtures in a temp dir and exercises: honest-claimable gating,
# score-based priority mapping (+ low-confidence downgrade), id/stem dedup
# against action-list AND assignments, per-run cap, additive-count guard, and
# idempotency.
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$HERE/backlog-refill.sh"
FAIL=0
ok()   { printf '  ok    %s\n' "$1"; }
bad()  { printf '  FAIL  %s\n' "$1"; FAIL=1; }

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

AL="$TMP/action-list.json"
ASG="$TMP/assignments.json"
ASGA="$TMP/assignments-archive.json"
BL="$TMP/backlog.json"
NOW="2026-07-02T12:00:00Z"

seed() {
  # action-list: one real open row + two GHOSTS (ids present in assignments)
  cat > "$AL" <<'JSON'
{"generated":"2026-07-01T00:00:00Z","items":[
  {"id":"real-open","action":"real","owner_role":"pm-backend","priority":"medium","status":"open","source":"seed","deadline":null,"dependency":null,"depends_on":[]},
  {"id":"ghost-merged","action":"ghost","owner_role":"pm-backend","priority":"low","status":"open","source":"seed","deadline":null,"dependency":null,"depends_on":[]},
  {"id":"ghost-arch","action":"ghost2","owner_role":"pm-backend","priority":"low","status":"open","source":"seed","deadline":null,"dependency":null,"depends_on":[]}
]}
JSON
  cat > "$ASG" <<'JSON'
{"generated":"2026-07-01T00:00:00Z","assignments":[
  {"task_id":"ghost-merged","branch":"auto-impl/ghost-merged","status":"merged","claimed_at":"2026-06-01T00:00:00Z","last_updated":"2026-06-02T00:00:00Z","status_changed_at":"2026-06-02T00:00:00Z"}
]}
JSON
  cat > "$ASGA" <<'JSON'
{"generated":"2026-07-01T00:00:00Z","assignments":[
  {"task_id":"ghost-arch","branch":"auto-impl/ghost-arch","status":"merged","claimed_at":"2026-05-01T00:00:00Z","last_updated":"2026-05-02T00:00:00Z","status_changed_at":"2026-05-02T00:00:00Z"}
]}
JSON
  # backlog: 4 promotable (distinct scores) + non-promotable (done, dup-of-al, dup-of-assignment)
  # Scores on the routine's real 0..8 scale (act-bar 3): 6->high 4->medium
  # 3+lowconf->low(downgrade) 1->low. Distinct scores keep ordering deterministic.
  cat > "$BL" <<'JSON'
{"version":1,"items":[
  {"id":"bl-sec-high","title":"Security fix","vector":"security-idor","confidence":"high","score":6,"status":"open","plan":"plans/bl-sec-high.md"},
  {"id":"bl-test-med","title":"Test gap","vector":"test-gap","confidence":"high","score":4,"status":"ready"},
  {"id":"bl-med-lowconf","title":"Downgrade me","vector":"bug","confidence":"low","score":3,"status":"open"},
  {"id":"bl-refactor-low","title":"Refactor","vector":"refactor","confidence":"high","score":1,"status":"open"},
  {"id":"bl-done","title":"Already done","vector":"bug","confidence":"high","score":6,"status":"done"},
  {"id":"real-open","title":"dup of action-list open","vector":"bug","confidence":"high","score":5,"status":"open"},
  {"id":"ghost-merged","title":"dup of assignment","vector":"bug","confidence":"high","score":5,"status":"open"}
]}
JSON
}

run() {
  BACKLOG_REFILL_NOW="$NOW" ACTION_LIST="$AL" ASSIGN="$ASG" ASSIGN_ARCHIVE="$ASGA" BACKLOG_FILE="$BL" \
    BUFFER_FLOOR="${BUFFER_FLOOR:-2}" BUFFER_TARGET="${BUFFER_TARGET:-5}" BUFFER_CEIL="${BUFFER_CEIL:-8}" \
    BACKLOG_REFILL_CAP="${BACKLOG_REFILL_CAP:-24}" \
    bash "$SCRIPT" "$@"
}

echo "== backlog-refill smoke test =="

# --- 1. dry-run reports candidates, writes nothing ---
seed
BEFORE=$(jq -c . "$AL")
OUT=$(run 2>&1)
[ "$(jq -c . "$AL")" = "$BEFORE" ] && ok "dry-run wrote nothing" || bad "dry-run mutated action-list"
echo "$OUT" | grep -q "honest_claimable=1" && ok "honest claimable excludes both ghosts (=1)" || bad "honest claimable wrong: $(echo "$OUT" | grep honest_claimable)"

# --- 2. --apply promotes to TARGET with correct priorities ---
seed
run --apply >/dev/null 2>&1
N_ADDED=$(jq '[.items[]|select(.source|startswith("dispatcher-backlog-refill"))]|length' "$AL")
# honest=1, target=5 -> need 4; 4 candidates available -> promote 4
[ "$N_ADDED" = "4" ] && ok "promoted 4 to reach target" || bad "expected 4 promoted, got $N_ADDED"
p() { jq -r --arg i "$1" '.items[]|select(.id==$i)|.priority' "$AL"; }
[ "$(p bl-sec-high)"     = "high" ]   && ok "score 6 -> high"                || bad "bl-sec-high priority=$(p bl-sec-high)"
[ "$(p bl-test-med)"     = "medium" ] && ok "score 4 -> medium"              || bad "bl-test-med priority=$(p bl-test-med)"
[ "$(p bl-med-lowconf)"  = "low" ]    && ok "score 3 + conf low -> low (downgrade)" || bad "bl-med-lowconf priority=$(p bl-med-lowconf)"
[ "$(p bl-refactor-low)" = "low" ]    && ok "score 1 -> low"                 || bad "bl-refactor-low priority=$(p bl-refactor-low)"

# --- 3. schema of promoted rows ---
ROW=$(jq -c '.items[]|select(.id=="bl-sec-high")' "$AL")
echo "$ROW" | jq -e '.status=="open" and .depends_on==[] and .first_open_at=="2026-07-02T12:00:00Z" and .owner_role=="pm-security"' >/dev/null \
  && ok "promoted row schema (open, depends_on:[], first_open_at, role)" || bad "promoted row schema wrong: $ROW"

# --- 4. dedup: none of the excluded ids were promoted ---
for x in bl-done real-open ghost-merged ghost-arch; do
  CNT=$(jq --arg i "$x" '[.items[]|select(.id==$i)]|length' "$AL")
  if [ "$x" = "real-open" ] || [ "$x" = "ghost-merged" ] || [ "$x" = "ghost-arch" ]; then
    [ "$CNT" = "1" ] && ok "no duplicate row for pre-existing $x" || bad "$x count=$CNT (dup created?)"
  else
    [ "$CNT" = "0" ] && ok "done backlog item $x not promoted" || bad "$x wrongly promoted"
  fi
done

# --- 5. idempotency: second --apply is a no-op (buffer now >= floor) ---
AFTER1=$(jq -c . "$AL")
run --apply >/dev/null 2>&1
[ "$(jq -c . "$AL")" = "$AFTER1" ] && ok "second --apply is a no-op (idempotent)" || bad "second --apply mutated"

# --- 6. no-op when buffer already healthy ---
seed
BEFORE=$(jq -c . "$AL")
BUFFER_FLOOR=1 run --apply >/dev/null 2>&1
[ "$(jq -c . "$AL")" = "$BEFORE" ] && ok "healthy buffer (honest 1 >= floor 1) -> no-op" || bad "refilled a healthy buffer"

# --- 7. per-run cap bounds promotions ---
seed
BACKLOG_REFILL_CAP=2 run --apply >/dev/null 2>&1
N=$(jq '[.items[]|select(.source|startswith("dispatcher-backlog-refill"))]|length' "$AL")
[ "$N" = "2" ] && ok "per-run cap=2 promotes exactly 2 (highest score first)" || bad "cap expected 2, got $N"
jq -e '[.items[]|select(.source|startswith("dispatcher-backlog-refill"))|.id]|sort==["bl-sec-high","bl-test-med"]' "$AL" >/dev/null \
  && ok "cap picks top-2 by score" || bad "cap picked wrong items"

echo
if [ "$FAIL" = "0" ]; then echo "==> backlog-refill smoke test PASSED"; exit 0
else echo "==> backlog-refill smoke test FAILED"; exit 1; fi
