#!/usr/bin/env bash
# Smoke test for signals-promote.sh — deterministic, offline (no git, no
# network). Builds fixtures in a temp dir and exercises: the three signal-file
# shapes (bare array / {signals:[…]} / {findings:[…]}), honest-claimable
# gating, mobile-native/KMP dead-end EXCLUSION (the issue #3357 fix) vs
# mobile-rn promotion, id/stem dedup against action-list + assignments
# (active+archive) + backlog.json, per-run cap, title/deslug fallback,
# additive-count guard, and idempotency.
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$HERE/signals-promote.sh"
# Reuse the REAL claim-time gate — the integration is part of what we verify.
GATE="$HERE/mobile-native-gate.sh"
FAIL=0
ok()  { printf '  ok    %s\n' "$1"; }
bad() { printf '  FAIL  %s\n' "$1"; FAIL=1; }

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

AL="$TMP/action-list.json"
ASG="$TMP/assignments.json"
ASGA="$TMP/assignments-archive.json"
BL="$TMP/backlog.json"
SIGDIR="$TMP/signals"
NOW="2026-11-09T12:00:00Z"
mkdir -p "$SIGDIR"

seed() {
  rm -f "$SIGDIR"/*.json
  # action-list: 1 real open row + 1 ghost (in assignments) + 1 dedup target.
  cat > "$AL" <<'JSON'
{"version":1,"items":[
  {"id":"real-open","action":"real","owner_role":"pm-backend","priority":"medium","status":"open","source":"seed","deadline":null,"dependency":null,"depends_on":[]},
  {"id":"code-review-dup-in-actionlist","action":"already tracked","owner_role":"pm-backend","priority":"low","status":"open","source":"seed","deadline":null,"dependency":null,"depends_on":[]}
]}
JSON
  cat > "$ASG" <<'JSON'
{"generated":"2026-11-01T00:00:00Z","assignments":[
  {"task_id":"code-review-dup-in-assign","branch":"auto-impl/x","status":"merged","claimed_at":"2026-10-01T00:00:00Z","last_updated":"2026-10-02T00:00:00Z","status_changed_at":"2026-10-02T00:00:00Z"}
]}
JSON
  cat > "$ASGA" <<'JSON'
{"generated":"2026-11-01T00:00:00Z","assignments":[
  {"task_id":"code-review-dup-in-archive","branch":"auto-impl/y","status":"merged","claimed_at":"2026-09-01T00:00:00Z","last_updated":"2026-09-02T00:00:00Z","status_changed_at":"2026-09-02T00:00:00Z"}
]}
JSON
  cat > "$BL" <<'JSON'
{"version":1,"items":[
  {"id":"code-review-dup-in-backlog","title":"already in backlog","vector":"bug","score":3,"status":"open"}
]}
JSON

  # Shape A — {segment,generated,signals:[…]}: two fresh, landable findings
  # (one security, one bug) + one dedup-against-action-list.
  cat > "$SIGDIR/2026-11-09-reality-server-tier1d.json" <<'JSON'
{"generator":"dev-review","generated":"2026-11-09","segment":"reality-server","signals":[
  {"id":"code-review-reality-server-sso-err-leak","title":"SSO callback leaks raw DB error","type":"code-review-finding","segment":"reality-server","score_delta":4,"confidence":"high","candidate_vector":"security-info-leak","evidence":["x"]},
  {"id":"code-review-reality-server-price-mode-unvalidated","type":"code-review-finding","segment":"reality-server","score_delta":2,"confidence":"medium","candidate_vector":"bug","evidence":["y"]},
  {"id":"code-review-dup-in-actionlist","title":"dup","type":"code-review-finding","segment":"reality-server","score_delta":5,"confidence":"high","candidate_vector":"bug"}
]}
JSON

  # Shape B — bare array (older files): one mobile-rn (LANDABLE, must promote)
  # + one mobile-native-kmp (DEAD-END, must be excluded).
  cat > "$SIGDIR/2026-11-08-mobile-tier1d.json" <<'JSON'
[
  {"id":"code-review-mobile-rn-announcements-i18n","title":"RN announcements screen untranslated","type":"code-review-finding","segment":"mobile-rn","score_delta":2,"confidence":"high","candidate_vector":"bug"},
  {"id":"code-review-mobile-native-kmp-sso-service-no-tests","title":"KMP SsoService untested","type":"code-review-finding","segment":"mobile-native-kmp","score_delta":3,"confidence":"high","candidate_vector":"test-gap"}
]
JSON

  # Shape C — {segment,generated,findings:[…]}: a test-gap with NO title
  # (exercises the deslug fallback) + the four cross-sink dedup targets.
  cat > "$SIGDIR/2026-11-07-api-core-tier1d.json" <<'JSON'
{"generator":"dev-review","generated":"2026-11-07","segment":"api-core","findings":[
  {"id":"code-review-api-core-digest-worker-no-replica-claim","type":"code-review-finding","segment":"api-core","score_delta":2,"confidence":"medium","candidate_vector":"bug"},
  {"id":"code-review-dup-in-assign","type":"code-review-finding","segment":"api-core","score_delta":3,"candidate_vector":"bug"},
  {"id":"code-review-dup-in-archive","type":"code-review-finding","segment":"api-core","score_delta":3,"candidate_vector":"bug"},
  {"id":"code-review-dup-in-backlog","type":"code-review-finding","segment":"api-core","score_delta":3,"candidate_vector":"bug"}
]}
JSON
}

run() {
  SIGNALS_PROMOTE_NOW="$NOW" ACTION_LIST="$AL" ASSIGN="$ASG" ASSIGN_ARCHIVE="$ASGA" \
    BACKLOG_FILE="$BL" SIGNALS_GLOB="$SIGDIR/*tier1d*.json" GATE_SCRIPT="$GATE" \
    BUFFER_FLOOR="${BUFFER_FLOOR:-10}" BUFFER_TARGET="${BUFFER_TARGET:-12}" BUFFER_CEIL="${BUFFER_CEIL:-20}" \
    SIGNALS_PROMOTE_CAP="${SIGNALS_PROMOTE_CAP:-12}" \
    bash "$SCRIPT" "$@"
}

echo "== signals-promote smoke test =="

# --- 1. dry-run writes nothing, reports honest claimable ---
seed
BEFORE=$(jq -c . "$AL")
OUT=$(run 2>&1)
[ "$(jq -c . "$AL")" = "$BEFORE" ] && ok "dry-run wrote nothing" || bad "dry-run mutated action-list"
# honest claimable = 'real-open' + 'code-review-dup-in-actionlist' = 2 open, both
# dep-free, neither in assignments -> 2.
echo "$OUT" | grep -q "honest_claimable=2" && ok "honest claimable = 2" || bad "honest wrong: $(echo "$OUT"|grep -o 'honest_claimable=[0-9]*')"

# --- 2. --apply promotes the landable fresh signals ---
seed
APPLY_OUT=$(run --apply 2>&1)
promoted() { jq -r '[.items[]|select(.source|startswith("dispatcher-signals-promote"))|.id]|sort|.[]' "$AL"; }
P="$(promoted)"
# Landable & fresh: reality-server sso + price-mode, mobile-rn, api-core digest.
echo "$P" | grep -q "code-review-reality-server-sso-err-leak" && ok "promoted reality-server security finding" || bad "missing sso finding"
echo "$P" | grep -q "code-review-mobile-rn-announcements-i18n" && ok "promoted mobile-RN finding (jest-verifiable, NOT gated)" || bad "mobile-rn finding not promoted"

# --- 3. THE FIX: mobile-native/KMP dead-end excluded ---
echo "$P" | grep -q "mobile-native-kmp" && bad "KMP dead-end wrongly promoted" || ok "mobile-native-kmp dead-end EXCLUDED (issue #3357 fix)"
echo "$APPLY_OUT" | grep -q "gated excluded=1" && ok "gate reports exactly 1 excluded" || bad "gate exclusion count wrong: $(echo "$APPLY_OUT"|grep -o 'gated excluded=[0-9]*')"

# --- 4. cross-sink dedup: none of the 4 dup ids promoted as a 2nd row ---
for d in code-review-dup-in-actionlist code-review-dup-in-assign code-review-dup-in-archive code-review-dup-in-backlog; do
  N=$(jq --arg i "$d" '[.items[]|select(.id==$i)]|length' "$AL")
  # dup-in-actionlist pre-exists (count 1); the other three never existed (count 0).
  if [ "$d" = "code-review-dup-in-actionlist" ]; then
    [ "$N" = "1" ] && ok "no duplicate row for action-list dup $d" || bad "$d count=$N"
  else
    [ "$N" = "0" ] && ok "dedup excluded $d (in assign/archive/backlog)" || bad "$d wrongly promoted (count=$N)"
  fi
done

# --- 5. security finding gets pm-security role + high-ish priority ---
ROLE=$(jq -r '.items[]|select(.id=="code-review-reality-server-sso-err-leak")|.owner_role' "$AL")
[ "$ROLE" = "pm-security" ] && ok "security vector -> pm-security role" || bad "sso role=$ROLE"

# --- 6. title fallback: no-title finding gets deslugified action text ---
ACT=$(jq -r '.items[]|select(.id=="code-review-api-core-digest-worker-no-replica-claim")|.action' "$AL")
[ "$ACT" = "api core digest worker no replica claim" ] && ok "no-title finding deslugs id into action" || bad "deslug wrong: '$ACT'"

# --- 7. freshest-first ordering under cap=1 picks a 2026-11-09 signal ---
seed
SIGNALS_PROMOTE_CAP=1 run --apply >/dev/null 2>&1
N=$(jq '[.items[]|select(.source|startswith("dispatcher-signals-promote"))]|length' "$AL")
[ "$N" = "1" ] && ok "per-run cap=1 promotes exactly 1" || bad "cap expected 1, got $N"
GEN=$(jq -r '.items[]|select(.source|startswith("dispatcher-signals-promote"))|.signal_ref.generated' "$AL")
[ "$GEN" = "2026-11-09" ] && ok "freshest-first: capped run picks the 2026-11-09 signal" || bad "cap picked gen=$GEN (not freshest)"

# --- 8. idempotency: second --apply adds nothing new (same ids already present) ---
seed
run --apply >/dev/null 2>&1
AFTER1=$(jq -c . "$AL")
run --apply >/dev/null 2>&1
[ "$(jq -c . "$AL")" = "$AFTER1" ] && ok "second --apply is a no-op (idempotent)" || bad "second --apply mutated"

# --- 9. healthy buffer -> no-op ---
seed
BEFORE=$(jq -c . "$AL")
BUFFER_FLOOR=1 run --apply >/dev/null 2>&1
[ "$(jq -c . "$AL")" = "$BEFORE" ] && ok "healthy buffer (honest 2 >= floor 1) -> no-op" || bad "promoted into a healthy buffer"

# --- 10. missing assignments.json: clean exit, no jq abort (#2078 parity) ---
seed
rm -f "$ASG" "$ASGA"
if OUT=$(run --apply 2>&1); then ok "missing assignments exits cleanly"; else bad "missing assignments aborted: $OUT"; fi

# --- 11. no signal files -> clean exit, no source ---
seed
rm -f "$SIGDIR"/*.json
if OUT=$(run --apply 2>&1); then
  echo "$OUT" | grep -q "no signal files" && ok "empty signal sink exits cleanly (no source)" || bad "empty sink unexpected: $OUT"
else bad "empty signal sink aborted: $OUT"; fi

echo
if [ "$FAIL" = "0" ]; then echo "==> signals-promote smoke test PASSED"; exit 0
else echo "==> signals-promote smoke test FAILED"; exit 1; fi
