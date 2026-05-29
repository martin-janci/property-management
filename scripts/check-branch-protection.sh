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
#   1 — `gh` CLI missing or not authenticated
#   2 — unexpected API error
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

if ! command -v jq >/dev/null 2>&1; then
  echo "ERROR: jq required (install 'jq' to parse the GitHub API response)." >&2
  exit 1
fi

# Probe auth up-front so we can distinguish "not authenticated" (exit 1)
# from a genuinely unexpected API error (exit 2) later on.
#
# We deliberately do NOT use `gh auth status` here: it returns non-zero
# whenever any configured account has a stale token, even if the active
# account is fine. A cheap authenticated `gh api user` call is the real
# liveness check for the active account.
if ! gh api user >/dev/null 2>&1; then
  echo "ERROR: gh CLI missing or not authenticated. Run 'gh auth login'." >&2
  exit 1
fi

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
  # "Branch not protected" is the API's explicit message when the branch
  # exists but has no protection rule — treat as success (exit 0).
  if echo "$RAW" | grep -qi 'Branch not protected'; then
    echo "Branch '$BRANCH' has NO branch-protection rule."
    echo "Required status checks: (none)"
    exit 0
  fi
  # A generic 404 here usually means the branch itself does not exist —
  # surface that as an error rather than masking it as "unprotected".
  if echo "$RAW" | grep -qE 'Not Found|HTTP 404'; then
    echo "ERROR: branch '$BRANCH' not found on $REPO (or insufficient permissions)." >&2
    echo "$RAW" >&2
    exit 2
  fi
  echo "ERROR: unexpected response from GitHub API:" >&2
  echo "$RAW" >&2
  exit 2
fi

# Pretty-print the JSON, then list the contexts one per line.
echo "Raw API response:"
echo "$RAW" | jq . 2>/dev/null || echo "$RAW"
echo

echo "Required status checks:"
NAMES=$(echo "$RAW" | jq -r '
  (.checks // []) as $c
  | (.contexts // []) as $x
  | (if ($c | length) > 0 then ($c | map(.context)) else $x end)
  | .[]
' 2>/dev/null || true)

if [ -z "$NAMES" ]; then
  echo "  (none)"
else
  while IFS= read -r n; do
    [ -n "$n" ] && echo "  - $n"
  done <<<"$NAMES"
fi
