#!/usr/bin/env bash
#
# check-branch-protection.sh — print the current required status checks
# for a given branch on this repository.
#
# Usage:
#   ./scripts/check-branch-protection.sh [branch]
#   ./scripts/check-branch-protection.sh dev
#
# Exit codes:
#   0 — required checks fetched and printed (including empty / unprotected)
#   1 — `gh` CLI missing or not authenticated (incl. API auth/authz rejection)
#   2 — unexpected API error (incl. branch not found / typo'd branch name)
#
# JSON parsing degrades gracefully: prefers `jq`, falls back to `python3`,
# and if neither is present still exits 0 after printing the raw response.
#
# Does NOT require an admin token. The default `gh auth` user only needs
# read access to the repo. If the branch is unprotected, the script prints
# a clear message and exits 0 so it can be used in informational checks.
#
# Designed for follow-up audit of issue #683 — verifies that
# `security-gate-conclusion` (or any other gate) is registered as a
# required status check after running `.github/workflows/branch-protection-setup.yml`.

set -euo pipefail

BRANCH="${1:-dev}"

if ! command -v gh >/dev/null 2>&1; then
  echo "ERROR: 'gh' CLI is not installed. See https://cli.github.com/" >&2
  exit 1
fi

# gh auth status may report non-zero if any configured account has a stale
# token, even when the active account is fine. Treat the script's ability
# to call `gh api` as the real auth check (done below).

# Resolve owner/repo from the current git remote so the script works in any clone.
REPO=$(gh repo view --json nameWithOwner -q .nameWithOwner 2>/dev/null || true)
if [ -z "$REPO" ]; then
  echo "ERROR: could not resolve owner/repo via 'gh repo view'." >&2
  echo "       Run this script from inside a git checkout with a GitHub remote." >&2
  exit 1
fi

echo "Repository: $REPO"
echo "Branch:     $BRANCH"
echo

API_PATH="repos/$REPO/branches/$BRANCH/protection/required_status_checks"
RC=0
RAW=$(gh api "$API_PATH" 2>&1) || RC=$?

if [ "$RC" -ne 0 ]; then
  # Authentication / authorization failures are the documented exit-1 case
  # (same contract as "gh missing"). Detect them before the unprotected path
  # so a stale token isn't misreported as "no protection rule".
  if echo "$RAW" | grep -qiE 'auth|credentials|401|Bad credentials|Requires authentication|Must have admin'; then
    echo "ERROR: GitHub API rejected the request (authentication/authorization)." >&2
    echo "       Run 'gh auth status' and re-authenticate if needed." >&2
    echo "$RAW" >&2
    exit 1
  fi
  # "Branch not protected" is the expected 404 for a real branch with no
  # protection rule — common after a fresh repo or before
  # branch-protection-setup.yml has been run. Report it as success.
  if echo "$RAW" | grep -qE 'Branch not protected'; then
    echo "Branch '$BRANCH' has NO branch-protection rule."
    echo "Required status checks: (none)"
    exit 0
  fi
  # A generic 404 / "Not Found" most likely means the branch itself does not
  # exist (e.g. a typo'd name) — do NOT mask that as "unprotected".
  if echo "$RAW" | grep -qE 'Not Found|404'; then
    echo "ERROR: branch '$BRANCH' not found on $REPO (or no access)." >&2
    echo "       Check the branch name; this is NOT the same as 'unprotected'." >&2
    echo "$RAW" >&2
    exit 2
  fi
  echo "ERROR: unexpected response from GitHub API:" >&2
  echo "$RAW" >&2
  exit 2
fi

# Pretty-print the JSON, then list the contexts one per line. Parse with
# whatever is available — prefer jq, fall back to python3, else print raw.
# This helper is meant to run in arbitrary local shells, so it must not hard
# require a single interpreter.
echo "Raw API response:"
if command -v jq >/dev/null 2>&1; then
  echo "$RAW" | jq .
elif command -v python3 >/dev/null 2>&1; then
  echo "$RAW" | python3 -m json.tool --no-ensure-ascii
else
  echo "$RAW"
fi
echo

echo "Required status checks:"
if command -v jq >/dev/null 2>&1; then
  names=$(echo "$RAW" | jq -r '(.checks // [] | map(.context)) as $c
    | (if ($c | length) > 0 then $c else (.contexts // []) end)[]')
elif command -v python3 >/dev/null 2>&1; then
  names=$(echo "$RAW" | python3 -c "
import json, sys
d = json.load(sys.stdin)
checks = d.get('checks') or []
contexts = d.get('contexts') or []
for n in ([c['context'] for c in checks] if checks else contexts):
    print(n)
")
else
  echo "  (cannot list — install 'jq' or 'python3'; see raw response above)"
  exit 0
fi
if [ -z "${names:-}" ]; then
  echo "  (none)"
else
  while IFS= read -r n; do
    [ -n "$n" ] && echo "  - $n"
  done <<< "$names"
fi
