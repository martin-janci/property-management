#!/usr/bin/env bash
# stale-pr-guard.sh — flag "stranded done" PRs so `done` can't silently mean
# "open forever, never merged."
#
# Why this script exists (BIT-325, audit remediation of BIT-324 F1/F2):
#
#   A subtask gets marked `done` in Paperclip the moment its implementer opens
#   a green PR. But `done` != merged: if the dispatcher merge pipeline stalls
#   (auto-merge enabled but never fires, flaky `test`, silent merge conflicts),
#   the PR sits open for days and the "done" coverage is never realized on
#   `dev`. The BIT-324 Audit of Done found ~36 BIT-258/268 backfill PRs in
#   exactly this state — `done` in Paperclip, 0% landed on `dev`.
#
#   There was no alarm for this. `auto-rebase-stale-drafts.yml` only unsticks
#   *draft* conflicts; nothing watches *ready* PRs that age out. This guard is
#   that missing alarm: a read-only, periodic sweep that flags any open
#   non-draft PR against the integration branch that is either
#     - open  > STALE_DAYS  (default 3)   days, OR
#     - more  > BEHIND_LIMIT (default 200) commits behind $BASE.
#
#   It does NOT merge, close, or rebase anything — flag-only, by design. The
#   operator (or a follow-up drain task) decides per PR. Pair with
#   auto-rebase-stale-drafts.yml (mechanical un-sticking) and the dispatcher
#   merge phase (the actual lander).
#
# Correlation to "done subtasks": the Paperclip `done` status is not reachable
# from CI, so this guard keys off the GitHub-observable proxy — an open PR IS
# the landing vehicle for an in-flight/`done` subtask. The BIT-<n> id is parsed
# from the branch/title into `subtask` so the operator can cross-reference the
# board.
#
# Idempotent + read-only: computes and prints; writes nothing to git or the
# board. Safe to run on any schedule.
#
# Usage:
#   ./.research/stale-pr-guard.sh                 # human table on stderr, exit 0
#   ./.research/stale-pr-guard.sh --json          # flagged[] JSON to stdout
#   STALE_DAYS=2 BEHIND_LIMIT=100 ./.research/stale-pr-guard.sh
#   GUARD_ENFORCE=1 ./.research/stale-pr-guard.sh  # exit 1 if anything flagged
#   PRS_FILE=fixture.json ./.research/stale-pr-guard.sh --json   # offline test
#
# Offline test injection:
#   PRS_FILE — a JSON array shaped like `gh pr list --json number,title,
#   headRefName,headRefOid,createdAt,isDraft`. Each row MAY carry a precomputed
#   integer `commitsBehind`; when present the script trusts it and skips git
#   (so the unit test needs no repo history). NOW_EPOCH overrides the clock.
set -euo pipefail

BASE="${BASE:-origin/dev}"
STALE_DAYS="${STALE_DAYS:-3}"
BEHIND_LIMIT="${BEHIND_LIMIT:-200}"
ENFORCE="${GUARD_ENFORCE:-0}"
REPO="${REPO:-}"
EMIT_JSON=0
[ "${1:-}" = "--json" ] && EMIT_JSON=1

NOW="${NOW_EPOCH:-$(date -u +%s)}"
STALE_SECS=$(( STALE_DAYS * 86400 ))

# 1. Source the open, non-draft PR set — from a fixture (offline) or live gh.
if [ -n "${PRS_FILE:-}" ]; then
  PRS=$(cat "$PRS_FILE")
else
  repo_flag=()
  [ -n "$REPO" ] && repo_flag=(--repo "$REPO")
  PRS=$(gh pr list "${repo_flag[@]}" --base "${BASE#origin/}" --state open --limit 200 \
    --json number,title,headRefName,headRefOid,createdAt,isDraft)
fi

# 2. Resolve commits-behind for each non-draft PR. Trust a fixture-provided
#    commitsBehind; otherwise ask git (merge-base..BASE). A head sha we cannot
#    resolve locally yields -1 (reported as "unknown", never flagged on behind).
behind_for() {
  local sha="$1"
  [ -z "$sha" ] && { echo -1; return; }
  if git rev-parse --verify --quiet "$sha^{commit}" >/dev/null 2>&1; then
    git rev-list --count "$sha..$BASE" 2>/dev/null || echo -1
  else
    echo -1
  fi
}

ROWS='[]'
while read -r row; do
  [ -z "$row" ] && continue
  number=$(echo "$row"   | jq -r '.number')
  title=$(echo "$row"    | jq -r '.title')
  head=$(echo "$row"     | jq -r '.headRefName')
  sha=$(echo "$row"      | jq -r '.headRefOid // ""')
  created=$(echo "$row"  | jq -r '.createdAt')
  preset=$(echo "$row"   | jq -r '.commitsBehind // empty')

  created_epoch=$(date -u -d "$created" +%s 2>/dev/null || echo 0)
  age_days=$(( (NOW - created_epoch) / 86400 ))

  if [ -n "$preset" ]; then behind="$preset"; else behind=$(behind_for "$sha"); fi

  # Subtask id from branch then title (first BIT-<n> / story / epic token).
  subtask=$(printf '%s %s' "$head" "$title" \
    | grep -oiE '(BIT-[0-9]+|epic-[0-9]+|story-[0-9.]+)' | head -1 || true)

  reasons='[]'
  (( (NOW - created_epoch) > STALE_SECS )) && \
    reasons=$(echo "$reasons" | jq -c --arg d "$STALE_DAYS" '. + ["open>" + $d + "d"]')
  if [ "$behind" -ge 0 ] 2>/dev/null && [ "$behind" -gt "$BEHIND_LIMIT" ]; then
    reasons=$(echo "$reasons" | jq -c --arg l "$BEHIND_LIMIT" '. + ["behind>" + $l]')
  fi

  if [ "$(echo "$reasons" | jq 'length')" -gt 0 ]; then
    ROWS=$(echo "$ROWS" | jq -c \
      --argjson n "$number" --arg t "$title" --arg b "$head" \
      --argjson age "$age_days" --argjson beh "$behind" \
      --arg sub "${subtask:-}" --argjson r "$reasons" \
      '. + [{number:$n, subtask:$sub, ageDays:$age, commitsBehind:$beh, branch:$b, title:$t, reasons:$r}]')
  fi
done < <(echo "$PRS" | jq -c '.[] | select(.isDraft == false)')

# 3. Emit. Always sorted oldest-first so the worst offenders lead.
ROWS=$(echo "$ROWS" | jq -c 'sort_by(-.ageDays)')
COUNT=$(echo "$ROWS" | jq 'length')

if [ "$EMIT_JSON" -eq 1 ]; then
  echo "$ROWS"
fi

{
  echo "stale-pr-guard: $COUNT flagged (open>${STALE_DAYS}d or >${BEHIND_LIMIT} behind $BASE)"
  echo "$ROWS" | jq -r '.[] | "  #\(.number)  \(.subtask // "—")  age=\(.ageDays)d  behind=\(if .commitsBehind < 0 then "?" else .commitsBehind|tostring end)  [\(.reasons|join(","))]  \(.title[0:48])"'
} >&2

if [ "$ENFORCE" = "1" ] && [ "$COUNT" -gt 0 ]; then
  exit 1
fi
exit 0
