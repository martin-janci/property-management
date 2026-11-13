#!/usr/bin/env bash
# lag-floor-guard.test.sh — offline unit test for lag-floor-guard.sh.
# No gh / network / real state.json needed: drives the guard with a STATE_FILE
# fixture (only .last_run_iso matters) and a frozen NOW_EPOCH. Sibling of
# stale-pr-guard.test.sh. Run locally: `bash .research/lag-floor-guard.test.sh`.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GUARD="$HERE/lag-floor-guard.sh"
# Frozen "today" matches the task that motivated the guard (2026-11-13).
NOW=$(date -u -d '2026-11-13T11:30:00Z' +%s)

FIX="$(mktemp)"
BAD="$(mktemp)"
EMPTY="$(mktemp)"
trap 'rm -f "$FIX" "$BAD" "$EMPTY"' EXIT

fail() { echo "FAIL: $*" >&2; exit 1; }
state() { printf '{"last_run_iso":"%s","paused":false}' "$1" > "$FIX"; }

# 1. Regression scenario: last run 2026-11-09, 4-day gap, floor=2 -> BREACH,
#    exit 1, loud banner. This is the exact situation the guard was created for;
#    before the guard the routine would have silently caught up (exit 0 echo).
state "2026-11-09T11:30:00Z"
if NOW_EPOCH="$NOW" STATE_FILE="$FIX" LAG_FLOOR_DAYS=2 "$GUARD" >/dev/null 2>"$BAD"; then
  fail "4-day gap over floor=2 must exit non-zero (loud fail)"
fi
grep -q "LAG FLOOR BREACHED" "$BAD" || fail "breach banner missing from stderr"

# 2. JSON reflects the breach.
OUT=$(NOW_EPOCH="$NOW" STATE_FILE="$FIX" LAG_FLOOR_DAYS=2 "$GUARD" --json 2>/dev/null || true)
echo "$OUT" | jq -e '.breached == true and .acknowledged == false and .lagDays == 4 and .floorDays == 2' \
  >/dev/null || fail "json breach fields wrong: $OUT"

# 3. Acknowledged catch-up (deep sweep): same gap, LAG_FLOOR_ACK=1 -> exit 0,
#    but still a loud ack banner (not silent).
state "2026-11-09T11:30:00Z"
ACKOUT="$(mktemp)"; trap 'rm -f "$FIX" "$BAD" "$EMPTY" "$ACKOUT"' EXIT
NOW_EPOCH="$NOW" STATE_FILE="$FIX" LAG_FLOOR_DAYS=2 LAG_FLOOR_ACK=1 "$GUARD" >/dev/null 2>"$ACKOUT" \
  || fail "acknowledged catch-up must exit 0"
grep -q "lag_floor_ack" "$ACKOUT" || fail "ack banner missing"

# 4. Within floor: 1-day gap, floor=2 -> exit 0, no breach.
state "2026-11-12T11:30:00Z"
NOW_EPOCH="$NOW" STATE_FILE="$FIX" LAG_FLOOR_DAYS=2 "$GUARD" >/dev/null 2>&1 \
  || fail "1-day gap under floor=2 must exit 0"
OUT=$(NOW_EPOCH="$NOW" STATE_FILE="$FIX" LAG_FLOOR_DAYS=2 "$GUARD" --json 2>/dev/null)
echo "$OUT" | jq -e '.breached == false and .lagDays == 1' >/dev/null \
  || fail "1-day json wrong: $OUT"

# 5. Exactly at the floor (2-day gap, floor=2) is NOT a breach ("exceeds" = >).
state "2026-11-11T11:30:00Z"
NOW_EPOCH="$NOW" STATE_FILE="$FIX" LAG_FLOOR_DAYS=2 "$GUARD" >/dev/null 2>&1 \
  || fail "2-day gap at floor=2 must not breach"

# 6. Custom floor: 4-day gap but floor=5 -> no breach.
state "2026-11-09T11:30:00Z"
NOW_EPOCH="$NOW" STATE_FILE="$FIX" LAG_FLOOR_DAYS=5 "$GUARD" >/dev/null 2>&1 \
  || fail "4-day gap under floor=5 must exit 0"

# 7. Clock skew / future cursor -> clamp to 0, no breach.
state "2026-11-20T00:00:00Z"
NOW_EPOCH="$NOW" STATE_FILE="$FIX" LAG_FLOOR_DAYS=2 "$GUARD" >/dev/null 2>&1 \
  || fail "future last_run_iso must clamp to 0 and exit 0"

# 8. Missing last_run_iso -> usage error, exit 64.
echo '{"paused":false}' > "$BAD"
set +e
NOW_EPOCH="$NOW" STATE_FILE="$BAD" "$GUARD" >/dev/null 2>&1
rc=$?
set -e
[ "$rc" = "64" ] || fail "missing last_run_iso should exit 64, got $rc"

# 9. Unreadable state file -> usage error, exit 64.
set +e
NOW_EPOCH="$NOW" STATE_FILE="/nonexistent/state.json" "$GUARD" >/dev/null 2>&1
rc=$?
set -e
[ "$rc" = "64" ] || fail "unreadable state file should exit 64, got $rc"

echo "lag-floor-guard.test.sh: all 9 assertions passed"
