#!/usr/bin/env bash
# mobile-native-gate.sh — claim-time gate for mobile-native / KMP-owned tasks
# (issue #2652).
#
# WHY: the cloud dispatcher's verify gate for any `mobile-native/` change is
#   `cd mobile-native && ./gradlew spotlessCheck test`, which fails at Gradle
#   configuration — AGP resolves from dl.google.com, and the org egress proxy
#   403-denies that host. Every mobile-native/KMP task is therefore
#   STRUCTURALLY UNLANDABLE in the cloud runner: a claimed one burns a claim
#   slot + an implementer subagent and can only ever return partial/blocked.
#   This is the exact analogue of the iOS/Swift `awaiting-macos-build` PR gate
#   (Phase 5.5) — except it must fire at CLAIM time, before a slot is spent.
#
# WHAT: classify whether a task is mobile-native/KMP-owned, using signals that
#   are available at claim time (id shape + the plan's `## Files` scope):
#     - `id`   — the id contains a `mobile-native` or `kmp` token
#                (e.g. `code-review-mobile-native-*`, `churn-hotspot-mobile-native-*`).
#                Note this deliberately does NOT match `mobile-rn` (React
#                Native, verifiable in cloud via jest) or `frontend/apps/mobile`.
#     - `files`— the plan at `.research/plans/<id>.md` lists ≥1 path under
#                `mobile-native/` AND every non-test source path it touches is
#                under `mobile-native/` (a pure-KMP plan). A cross-stack plan
#                with a verifiable slice is intentionally NOT gated.
#
# The dispatcher's Phase 3 uses this to SKIP such candidates (without consuming
# a claim slot) and surface them in Phase 7 under `Mobile-native gated:`.
#
# Modes:
#   --id <task_id> [--plan <path>]      single predicate; exit 0 gated / 1 not
#   --action-list <path> [--plans-dir]  batch; prints one `id<TAB>signal` line
#                                        per OPEN gated item; always exit 0
#
# Single-mode output (stdout, grep-friendly):
#   id=<id> gated=<true|false> signal=<id|files|none> reason=<short>
#
# Exit codes:
#   single mode:  0 — gated · 1 — not gated · 64 — usage · 65 — bad input
#   batch  mode:  0 — ran (regardless of how many rows matched) · 65 — bad input
#
# Usage:
#   mobile-native-gate.sh --id churn-hotspot-mobile-native-shared-foo
#   mobile-native-gate.sh --action-list .research/management/action-list.json

set -euo pipefail

MODE=""
ID=""
PLAN=""
ACTION_LIST=""
PLANS_DIR="${PLANS_DIR:-.research/plans}"

while [ $# -gt 0 ]; do
  case "$1" in
    --id)          MODE="single"; ID="$2"; shift 2;;
    --plan)        PLAN="$2"; shift 2;;
    --action-list) MODE="batch"; ACTION_LIST="$2"; shift 2;;
    --plans-dir)   PLANS_DIR="$2"; shift 2;;
    -h|--help)     sed -n '2,40p' "$0"; exit 0;;
    *) echo "unknown arg: $1" >&2; exit 64;;
  esac
done

# ---- id-signal predicate -------------------------------------------
# A `mobile-native` or `kmp` token bounded by start/end or a `-`/`_`
# separator. Case-insensitive. Kept in one place so the batch and single
# modes agree.
id_signal() {
  printf '%s' "$1" | grep -Eiq '(^|[-_])(mobile-native|kmp)([-_]|$)'
}

# ---- files-signal predicate ----------------------------------------
# Parse the plan's `## Files` section. Return 0 (gated) iff at least one
# listed path is under `mobile-native/` AND every non-test source path is
# under `mobile-native/`. Missing/empty plan → 1 (can't judge on files).
#
# Path extraction: lines between `## Files` and the next `## ` header;
# take backtick-quoted tokens first, else the `- <path>` bullet token;
# strip a trailing `:<line>` suffix.
files_signal() {
  local plan="$1"
  [ -n "$plan" ] && [ -f "$plan" ] || return 1

  local section total mn nontest_nonmn
  section=$(awk '
    /^## Files[[:space:]]*$/ { infiles=1; next }
    /^## / { infiles=0 }
    infiles { print }
  ' "$plan")

  total=0; mn=0; nontest_nonmn=0
  while IFS= read -r line; do
    local path
    # Prefer a backtick-quoted path; else the first token after a leading "- ".
    # shellcheck disable=SC2016  # single-quoted sed script is intentional (no shell expansion)
    path=$(printf '%s' "$line" | sed -n 's/.*`\([^`]*\)`.*/\1/p' | head -n1)
    if [ -z "$path" ]; then
      path=$(printf '%s' "$line" | sed -n 's/^[[:space:]]*-[[:space:]]*\([^[:space:]]*\).*/\1/p' | head -n1)
    fi
    [ -z "$path" ] && continue
    path="${path%%:*}"                 # strip :<line>
    case "$path" in
      \<*|'') continue;;               # skip template placeholders
    esac
    total=$((total + 1))
    case "$path" in
      mobile-native/*) mn=$((mn + 1)); continue;;
    esac
    # A non-mobile-native path. Test paths don't disqualify a KMP plan;
    # any non-test path under another root makes it cross-stack (not gated).
    case "$path" in
      */tests/*|*_test.rs|*.test.ts|*.test.tsx|*.test.js|*.test.jsx|*/__tests__/*) continue;;
      *) nontest_nonmn=$((nontest_nonmn + 1));;
    esac
  done <<EOF
$section
EOF

  [ "$mn" -ge 1 ] && [ "$nontest_nonmn" -eq 0 ] && [ "$total" -ge 1 ]
}

# ---- classify one id (+ optional plan) -----------------------------
# Echoes the signal name and returns 0 (gated) / 1 (not gated).
classify() {
  local id="$1" plan="$2"
  if id_signal "$id"; then echo "id"; return 0; fi
  if [ -z "$plan" ]; then plan="$PLANS_DIR/$id.md"; fi
  if files_signal "$plan"; then echo "files"; return 0; fi
  echo "none"; return 1
}

# ---- single mode ---------------------------------------------------
if [ "$MODE" = "single" ]; then
  [ -n "$ID" ] || { echo "error: --id requires a value" >&2; exit 64; }
  if signal=$(classify "$ID" "$PLAN"); then
    case "$signal" in
      id)    reason="id token (mobile-native|kmp)";;
      files) reason="plan Files scope is pure mobile-native/";;
    esac
    printf 'id=%s gated=true signal=%s reason=%s\n' "$ID" "$signal" "$reason"
    exit 0
  else
    printf 'id=%s gated=false signal=none reason=%s\n' "$ID" "no mobile-native/KMP signal"
    exit 1
  fi
fi

# ---- batch mode ----------------------------------------------------
if [ "$MODE" = "batch" ]; then
  [ -f "$ACTION_LIST" ] || { echo "error: missing action-list: $ACTION_LIST" >&2; exit 65; }
  command -v jq >/dev/null 2>&1 || { echo "error: jq required" >&2; exit 64; }
  # OPEN items only — a gated candidate only matters while it is claimable.
  while IFS= read -r id; do
    [ -z "$id" ] && continue
    if signal=$(classify "$id" ""); then
      printf '%s\t%s\n' "$id" "$signal"
    fi
  done < <(jq -r '.items[] | select(.status=="open") | .id' "$ACTION_LIST")
  exit 0
fi

echo "error: specify --id <task_id> or --action-list <path>" >&2
exit 64
