#!/usr/bin/env bash
# lag-floor-guard.sh — fail the research routine LOUDLY when it has fallen too
# far behind, instead of letting it silently "catch up" on a stale cursor.
#
# Why this script exists (pm-devops / stale-routine-watch, 2026-11-13):
#
#   The daily research routine keys all of its "since last run" scans off
#   state.json:last_run_iso. The cadence block in routine-prompt.md only ever
#   *echoed* an advisory `lag_warning` (>36h) and a `stale_routine_alert`
#   (>=3d) — both soft log lines that the pipeline ignores. So when the cloud
#   routine scheduler drifts or dies (observed: last_run_iso=2026-11-09 vs
#   today 2026-11-13, a 4-day gap), the next run just widens its window and
#   quietly back-fills four days of activity in one pass, advancing the cursor
#   as if nothing happened. The silent catch-up HIDES the scheduler failure —
#   nobody notices the cron is broken, and a wide one-shot sweep is far more
#   likely to drop or mis-dedup signals than four normal daily runs.
#
#   This guard is the missing alarm: a deterministic floor on lag-days. When
#   the gap since the last *successful* run exceeds LAG_FLOOR_DAYS (default 2)
#   it prints a loud, visible banner and exits non-zero, so the next run fails
#   loudly at the top of the pipeline instead of silently catching up. The
#   operator fixes the scheduler (or acknowledges an intentional catch-up with
#   LAG_FLOOR_ACK=1 — e.g. a `deep` 30-day sweep) and re-runs.
#
#   It does NOT touch state.json, cursors, or any backlog/management state —
#   it only reads last_run_iso and reports. Pair with the advisory lag_warning
#   lines already in routine-prompt.md (which stay, for the <=floor case).
#
# Usage:
#   ./.research/lag-floor-guard.sh                  # banner on stderr; exit 1 if breached
#   ./.research/lag-floor-guard.sh --json           # status object to stdout
#   LAG_FLOOR_DAYS=3 ./.research/lag-floor-guard.sh  # custom floor
#   LAG_FLOOR_ACK=1 ./.research/lag-floor-guard.sh   # intentional catch-up: warn, exit 0
#   STATE_FILE=fixture.json NOW_EPOCH=... ./.research/lag-floor-guard.sh  # offline test
#
# Offline test injection:
#   STATE_FILE — path to a state.json-shaped file (only .last_run_iso is read).
#   NOW_EPOCH  — overrides the clock (unix seconds, UTC).
#
# Exit codes:
#   0  — lag within floor, OR breach acknowledged (LAG_FLOOR_ACK=1).
#   1  — lag exceeds floor and was not acknowledged (loud fail).
#   64 — usage / unreadable state (last_run_iso missing or unparseable).
set -euo pipefail

STATE_FILE="${STATE_FILE:-.research/state.json}"
LAG_FLOOR_DAYS="${LAG_FLOOR_DAYS:-2}"
ACK="${LAG_FLOOR_ACK:-0}"
EMIT_JSON=0
[ "${1:-}" = "--json" ] && EMIT_JSON=1

NOW="${NOW_EPOCH:-$(date -u +%s)}"

die_usage() { echo "lag-floor-guard: $*" >&2; exit 64; }

[ -r "$STATE_FILE" ] || die_usage "state file not readable: $STATE_FILE"

LAST_ISO="$(jq -r '.last_run_iso // empty' "$STATE_FILE" 2>/dev/null || true)"
[ -n "$LAST_ISO" ] || die_usage "state.json has no last_run_iso ($STATE_FILE)"

# Parse the ISO timestamp with GNU date, falling back to BSD date (same two-way
# shim the routine-prompt cadence block uses).
LAST_EPOCH="$(date -u -d "$LAST_ISO" +%s 2>/dev/null \
  || date -u -j -f "%Y-%m-%dT%H:%M:%SZ" "$LAST_ISO" +%s 2>/dev/null \
  || echo "")"
[ -n "$LAST_EPOCH" ] || die_usage "could not parse last_run_iso='$LAST_ISO'"

LAG_SECS=$(( NOW - LAST_EPOCH ))
[ "$LAG_SECS" -lt 0 ] && LAG_SECS=0          # clock skew / future cursor -> clamp
LAG_HOURS=$(( LAG_SECS / 3600 ))
LAG_DAYS=$(( LAG_SECS / 86400 ))
NOW_ISO="$(date -u -d "@$NOW" +%Y-%m-%dT%H:%M:%SZ 2>/dev/null \
  || date -u -r "$NOW" +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || echo "?")"

# Breach = gap strictly EXCEEDS the floor. floor=2 => 3+ full days trips it,
# matching the existing 3-day stale_routine_alert threshold.
BREACHED=0
[ "$LAG_DAYS" -gt "$LAG_FLOOR_DAYS" ] && BREACHED=1
ACKED=0
[ "$ACK" = "1" ] && ACKED=1

if [ "$EMIT_JSON" -eq 1 ]; then
  jq -cn \
    --arg last "$LAST_ISO" --arg now "$NOW_ISO" \
    --argjson days "$LAG_DAYS" --argjson hours "$LAG_HOURS" \
    --argjson floor "$LAG_FLOOR_DAYS" \
    --argjson breached "$BREACHED" --argjson acked "$ACKED" \
    '{lastRunIso:$last, nowIso:$now, lagDays:$days, lagHours:$hours, floorDays:$floor, breached:($breached==1), acknowledged:($acked==1)}'
fi

if [ "$BREACHED" -eq 1 ]; then
  if [ "$ACKED" -eq 1 ]; then
    {
      echo "####################################################################"
      echo "# lag_floor_ack: routine is ${LAG_DAYS}d ${LAG_HOURS}h behind"
      echo "#   last_run_iso=${LAST_ISO}  floor=${LAG_FLOOR_DAYS}d"
      echo "#   LAG_FLOOR_ACK=1 set — proceeding with INTENTIONAL catch-up."
      echo "#   (deep/manual sweep acknowledged; cursor will advance on success.)"
      echo "####################################################################"
    } >&2
    exit 0
  fi
  {
    echo "####################################################################"
    echo "# LAG FLOOR BREACHED — research routine is STALE, refusing to"
    echo "# silently catch up."
    echo "#"
    echo "#   last_run_iso = ${LAST_ISO}"
    echo "#   now          = ${NOW_ISO}"
    echo "#   lag          = ${LAG_DAYS}d ${LAG_HOURS}h   (floor = ${LAG_FLOOR_DAYS}d)"
    echo "#"
    echo "# The cloud routine scheduler has very likely drifted or died: a gap"
    echo "# this large means missed runs, not a normal daily cadence. A single"
    echo "# wide back-fill sweep risks dropped / mis-deduped signals."
    echo "#"
    echo "# ACTION:"
    echo "#   1. Check the cloud routine scheduler (cron / trigger) for a silent"
    echo "#      failure or pause; confirm it is running again."
    echo "#   2. If this catch-up is INTENTIONAL (e.g. a deep 30-day sweep),"
    echo "#      re-run with LAG_FLOOR_ACK=1 to proceed and advance the cursor."
    echo "#   3. Raise LAG_FLOOR_DAYS only if a wider cadence is genuinely"
    echo "#      expected for this deployment."
    echo "####################################################################"
  } >&2
  exit 1
fi

echo "lag-floor-guard: ok — ${LAG_DAYS}d ${LAG_HOURS}h behind (floor ${LAG_FLOOR_DAYS}d, last_run_iso=${LAST_ISO})" >&2
exit 0
