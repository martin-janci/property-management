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
RAW=$(gh api "$API_PATH" 2>&1) || RC=$? && RC=${RC:-0}

if [ "${RC:-0}" -ne 0 ]; then
  # Branch unprotected or no required checks configured — common after a
  # fresh repo or before branch-protection-setup.yml has been run.
  if echo "$RAW" | grep -qE 'Branch not protected|Not Found|404'; then
    echo "Branch '$BRANCH' has NO branch-protection rule."
    echo "Required status checks: (none)"
    exit 0
  fi
  echo "ERROR: unexpected response from GitHub API:" >&2
  echo "$RAW" >&2
  exit 2
fi

# Pretty-print the JSON, then list the contexts one per line.
echo "Raw API response:"
echo "$RAW" | python3 -m json.tool --no-ensure-ascii 2>/dev/null || echo "$RAW"
echo

echo "Required status checks:"
echo "$RAW" | python3 -c "
import json, sys
d = json.load(sys.stdin)
checks = d.get('checks') or []
contexts = d.get('contexts') or []
names = [c['context'] for c in checks] if checks else contexts
if not names:
    print('  (none)')
else:
    for n in names:
        print(f'  - {n}')
"
