#!/usr/bin/env bash
# sandbox-reclaim.sh — classify a stalled in-progress assignments row and
# decide whether it is reclaimable (sandbox-died with no branch push) or
# terminally failed.
#
# Inputs (env or args):
#   BRANCH                — branch name we expect the implementer to push
#   STATUS_CHANGED_AT     — ISO-8601 timestamp for status_changed_at
#   MODE_TAG              — plan's `Mode:` tag (e.g. "cloud-ok", "local-only")
#   RECLAIM_ATTEMPTS      — current reclaim_attempts count on the row (default 0)
#   NOW                   — optional ISO-8601 override (for testing); default = now (UTC)
#
# Outputs ONE line to stdout, dispatcher-friendly:
#   action=<wait|reclaim|fail> reason=<short> branch_state=<missing|empty|present>
#
# Exit codes: 0 = decision printed, 2 = bad invocation.
#
# State machine (matches dispatcher-prompt.md Phase 2):
#
#   timeout = 60m  if MODE_TAG == "cloud-ok"
#           = 120m otherwise (legacy default)
#
#   grace not yet elapsed                                  -> action=wait
#   timeout elapsed AND branch missing AND attempts == 0   -> action=reclaim reason=sandbox-timeout
#   timeout elapsed AND branch missing AND attempts >= 1   -> action=fail    reason=sandbox-failure-after-reclaim
#   timeout elapsed AND branch present AND 0 commits ahead -> action=fail    reason=empty-branch
#   timeout elapsed AND branch present AND commits ahead   -> action=fail    reason=agent-gave-up-with-commits (caller should still try to PR it)
#
# Reclaim cap is one (RECLAIM_ATTEMPTS < 1).

set -u

BRANCH="${BRANCH:-${1:-}}"
STATUS_CHANGED_AT="${STATUS_CHANGED_AT:-${2:-}}"
MODE_TAG="${MODE_TAG:-${3:-}}"
RECLAIM_ATTEMPTS="${RECLAIM_ATTEMPTS:-${4:-0}}"
NOW="${NOW:-$(date -u -Iseconds)}"

if [ -z "$BRANCH" ] || [ -z "$STATUS_CHANGED_AT" ]; then
  echo "usage: BRANCH=<name> STATUS_CHANGED_AT=<iso> [MODE_TAG=<tag>] [RECLAIM_ATTEMPTS=<n>] $0" >&2
  exit 2
fi

# --- timeout selection
case "$MODE_TAG" in
  cloud-ok|cloud|cloud-only) TIMEOUT_MIN=60 ;;
  *)                          TIMEOUT_MIN=120 ;;
esac

# --- elapsed minutes (portable: GNU date)
NOW_EPOCH=$(date -u -d "$NOW" +%s 2>/dev/null) || NOW_EPOCH=$(date -u +%s)
THEN_EPOCH=$(date -u -d "$STATUS_CHANGED_AT" +%s 2>/dev/null) || {
  echo "action=wait reason=bad-timestamp branch_state=unknown"
  exit 0
}
ELAPSED_MIN=$(( (NOW_EPOCH - THEN_EPOCH) / 60 ))

if [ "$ELAPSED_MIN" -lt "$TIMEOUT_MIN" ]; then
  echo "action=wait reason=grace-period-${ELAPSED_MIN}m-of-${TIMEOUT_MIN}m branch_state=unchecked"
  exit 0
fi

# --- branch state probe (mirrors dispatcher-prompt.md Phase 2 Branch-state probe)
git fetch origin "$BRANCH" 2>/dev/null || true
if git rev-parse --verify "origin/$BRANCH" >/dev/null 2>&1; then
  COMMITS_AHEAD=$(git rev-list --count origin/dev..origin/"$BRANCH" 2>/dev/null || echo 0)
  if [ "$COMMITS_AHEAD" -eq 0 ]; then
    echo "action=fail reason=empty-branch branch_state=empty"
    exit 0
  fi
  echo "action=fail reason=agent-gave-up-with-commits branch_state=present commits_ahead=$COMMITS_AHEAD"
  exit 0
fi

# --- no branch on origin: sandbox most likely died
if [ "$RECLAIM_ATTEMPTS" -ge 1 ]; then
  echo "action=fail reason=sandbox-failure-after-reclaim branch_state=missing"
  exit 0
fi

echo "action=reclaim reason=sandbox-timeout branch_state=missing"
exit 0
