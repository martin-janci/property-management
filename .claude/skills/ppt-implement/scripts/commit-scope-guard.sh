#!/usr/bin/env bash
# Dispatcher commit-scope guard (#526).
#
# Pre-flight check that refuses to commit a working tree whose changes
# stray outside the agent's declared scope. This is the fix for the
# class of scope-creep failure observed in PR #496, where the
# dispatcher's stop hook auto-committed 1088 lines of unrelated
# parallel-agent work (gap-85-1 + gap-85-2) onto an ADR-008 doc-edit
# branch.
#
# WHEN TO RUN
#   Before ANY `git commit` invoked by a stop-hook / dispatcher /
#   auto-commit codepath. The implementer's normal interactive
#   commits already go through human review; the guard exists to
#   protect the automated commit path where a stop hook fires while
#   multiple parallel agents share a worktree (or where staged work
#   from a sibling task slipped in via a shared index).
#
# WHAT IT DOES
#   1. Gathers the set of paths the commit would touch (default:
#      `git diff --cached --name-only`; with `--worktree`, includes
#      `git diff --name-only` and `git ls-files --others
#      --exclude-standard` so a stop hook checking BEFORE `git add`
#      gets the same answer).
#   2. Compares each path against the agent's allow-list. The
#      allow-list comes from one of:
#        a. `--owner <role>` — looks up `owner-areas.json` (same file
#           that drives `scope-check.sh`).
#        b. `--allow <pattern>` (repeatable) — explicit pathspec list
#           passed by the caller for tasks that don't map cleanly to
#           an owner role (e.g. dispatcher's own self-commit, which
#           is scoped to `.research/management/`).
#   3. Exit code:
#        0 — every changed path is inside the allow-list. Safe to
#            commit.
#        2 — at least one path is outside. Prints the offending paths
#            and a remediation hint (`git stash push -- <path>` or
#            `git reset HEAD <path>`) on stderr. The caller MUST NOT
#            commit. The dispatcher's stop hook should `git stash` the
#            off-scope paths, commit the rest, and surface the stash
#            ref in the dispatcher log so a follow-up agent can pick
#            them up.
#        1 — unrecognised owner_role (logged, treat as fail-closed:
#            unknown scope is not safely committable from an
#            automated codepath).
#        64 — usage error.
#
# WHY FAIL-CLOSED ON UNKNOWN OWNER
#   The stop hook is automation: it has no human in the loop to
#   adjudicate. Better to skip the auto-commit and leave the work
#   uncommitted (the agent can surface it on resume) than to silently
#   bundle unrelated changes into the wrong PR — which is exactly
#   what bit PR #496.
#
# USAGE
#   commit-scope-guard.sh --owner pm-backend [--base dev] [--worktree]
#   commit-scope-guard.sh --allow 'docs/architecture.md' \
#                         --allow '.research/management/**' [--worktree]
#
# The script lives under `.claude/skills/ppt-implement/scripts/` so it
# travels with the skill bundle and is discoverable from the same
# location as `scope-check.sh`. Documentation lives in the skill's
# SKILL.md (see "Step 2.5 — Scope-drift check" — this guard is the
# stop-hook companion that refuses the commit instead of just
# tagging it).

set -euo pipefail

OWNER=""
BASE="dev"
WORKTREE=0
ALLOW=()

while [ $# -gt 0 ]; do
  case "$1" in
    --owner)    OWNER="$2"; shift 2;;
    --base)     BASE="$2"; shift 2;;
    --allow)    ALLOW+=("$2"); shift 2;;
    --worktree) WORKTREE=1; shift;;
    -h|--help)
      sed -n '1,/^set -euo pipefail$/p' "$0" | sed -n 's/^# \{0,1\}//p'
      exit 0
      ;;
    *) echo "unknown arg: $1" >&2; exit 64;;
  esac
done

if [ -z "$OWNER" ] && [ "${#ALLOW[@]}" -eq 0 ]; then
  echo "error: --owner <role> or at least one --allow <pattern> is required" >&2
  exit 64
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AREAS_FILE="$SCRIPT_DIR/owner-areas.json"

PATTERNS=()

if [ -n "$OWNER" ]; then
  if ! command -v jq >/dev/null 2>&1; then
    echo "error: jq is required for --owner lookup" >&2
    exit 64
  fi
  if [ ! -f "$AREAS_FILE" ]; then
    echo "error: missing $AREAS_FILE" >&2
    exit 64
  fi
  # Read newline-separated patterns from the owner's allow-list.
  while IFS= read -r p; do
    [ -z "$p" ] && continue
    PATTERNS+=("$p")
  done < <(jq -r --arg o "$OWNER" '.[$o] // empty | .[]?' "$AREAS_FILE")
  if [ "${#PATTERNS[@]}" -eq 0 ]; then
    # Fail-closed: see header rationale.
    echo "commit-scope-guard: unknown owner_role '$OWNER' (not in owner-areas.json) — fail-closed" >&2
    exit 1
  fi
fi

# Add any explicit --allow patterns.
for p in "${ALLOW[@]}"; do
  PATTERNS+=("$p")
done

# Collect the candidate paths.
if [ "$WORKTREE" -eq 1 ]; then
  # Stop hook before `git add`: include staged, unstaged, AND untracked.
  CHANGED=$(
    {
      git diff --cached --name-only
      git diff --name-only
      git ls-files --others --exclude-standard
    } | sort -u
  )
else
  CHANGED=$(git diff --cached --name-only --diff-filter=ACMRTUXB | sort -u)
fi

if [ -z "$CHANGED" ]; then
  exit 0
fi

# Convert pattern lines to anchored egrep alternation. Mirrors the
# translation in scope-check.sh so an allow-list shared across the two
# scripts has identical semantics.
#   **  -> .*
#   *   -> [^/]*
#   .   -> \.
to_regex() {
  printf '%s' "$1" \
    | sed -E 's#\.#\\.#g' \
    | sed -E 's#\*\*#__GLOBSTAR__#g' \
    | sed -E 's#\*#[^/]*#g' \
    | sed -E 's#__GLOBSTAR__#.*#g'
}

ALTS=""
for p in "${PATTERNS[@]}"; do
  rx=$(to_regex "$p")
  if [ -z "$ALTS" ]; then
    ALTS="$rx"
  else
    ALTS="$ALTS|$rx"
  fi
done
REGEX="^($ALTS)$"

DRIFT=$(printf '%s\n' "$CHANGED" | grep -E -v -- "$REGEX" || true)

if [ -n "$DRIFT" ]; then
  {
    echo "commit-scope-guard: REFUSING commit — paths outside declared scope:"
    printf '  %s\n' $DRIFT
    echo ""
    echo "Allow-list patterns:"
    printf '  %s\n' "${PATTERNS[@]}"
    echo ""
    echo "Remediation (pick one):"
    echo "  - git stash push -m 'off-scope: <task>' -- <path>..."
    echo "    (preserves the work; surface the stash ref in dispatcher log)"
    echo "  - git reset HEAD -- <path>     (drop from index; leave on disk)"
    echo "  - git restore --staged <path>  (newer git, same effect)"
    echo ""
    echo "Why this guard exists: PR #496 auto-committed 1088 lines of"
    echo "unrelated parallel-agent work onto an ADR-008 doc-edit branch"
    echo "via the dispatcher stop hook. The guard catches that class of"
    echo "scope-creep at the commit boundary so each agent's PR stays"
    echo "scoped to its declared owner area."
  } >&2
  exit 2
fi

exit 0
