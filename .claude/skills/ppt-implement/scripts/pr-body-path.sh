#!/usr/bin/env bash
# Print a TASK-UNIQUE scratchpad path for an implementer's PR-body file.
#
# Why this exists (observed 2026-11-17): parallel implementer subagents
# spawned in the same dispatcher run each materialise their `gh pr create` /
# MCP `create_pull_request` body into a scratchpad file before handing it to
# the tool. When that path is a bare, shared `pr-body.md`, concurrent siblings
# clobber each other's file (authed-query-roots vs validation-i18n collided;
# one only recovered by switching to a task-unique filename). The diffs are
# safe (separate worktrees) — only this transient body file collides.
#
# This helper is the single source of truth for that path. It folds the
# task_id (or, as a last resort, the caller's PID) into the filename so two
# implementers running at once can never resolve to the same file.
#
# Guarantees:
#   * Same task_id            -> same path   (idempotent: write then read it).
#   * Different task_ids       -> different paths (no sibling collision).
#   * No task_id and no $TASK_ID -> falls back to the PID ($$), which is still
#     process-unique and is NEVER the bare shared `pr-body.md`.
#
# Usage:
#   pr-body-path.sh [--task-id <id>] [--dir <scratchdir>] [--stem <stem>]
#
# Resolution order for the uniqueness token:
#   1. --task-id <id>
#   2. $TASK_ID environment variable
#   3. the caller's PID ($$)   (printed to stderr as a warning)
#
# Resolution order for the directory:
#   1. --dir <scratchdir>
#   2. $PPT_SCRATCH_DIR
#   3. $SCRATCHPAD
#   4. $TMPDIR
#   5. /tmp
#
# Output (stdout): the absolute-or-given path, e.g.
#   /tmp/pr-body.gap-9-1-mfa-frontend-integration.md
#
# Exit codes:
#   0  = path printed
#   64 = usage error

set -euo pipefail

TASK_ID_ARG=""
DIR_ARG=""
STEM="pr-body"

usage() {
  sed -n '2,45p' "$0" >&2
  exit 64
}

while [ $# -gt 0 ]; do
  case "$1" in
    --task-id) TASK_ID_ARG="${2:-}"; shift 2 || usage ;;
    --dir)     DIR_ARG="${2:-}";     shift 2 || usage ;;
    --stem)    STEM="${2:-}";        shift 2 || usage ;;
    -h|--help) usage ;;
    *) echo "pr-body-path.sh: unknown arg '$1'" >&2; usage ;;
  esac
done

# --- uniqueness token ---------------------------------------------------------
token="$TASK_ID_ARG"
if [ -z "$token" ]; then
  token="${TASK_ID:-}"
fi
if [ -z "$token" ]; then
  token="$$"
  echo "pr-body-path.sh: no --task-id and no \$TASK_ID — falling back to PID '$token' (still collision-safe, but prefer a task_id)" >&2
fi

# Sanitise to a safe filename fragment: keep [A-Za-z0-9._-], collapse the rest
# to '-'. This keeps distinct task_ids distinct (no lossy truncation) while
# never emitting a path separator or shell-hostile char.
safe_token="$(printf '%s' "$token" | tr -c 'A-Za-z0-9._-' '-' | sed -E 's/-+/-/g; s/^-|-$//g')"
if [ -z "$safe_token" ]; then
  # token was entirely non-safe chars — degrade to PID rather than a bare stem.
  safe_token="$$"
fi

if [ -z "$STEM" ]; then
  echo "pr-body-path.sh: --stem must be non-empty" >&2
  exit 64
fi

# --- directory ----------------------------------------------------------------
dir="$DIR_ARG"
if [ -z "$dir" ]; then
  dir="${PPT_SCRATCH_DIR:-${SCRATCHPAD:-${TMPDIR:-/tmp}}}"
fi
# Strip a single trailing slash for a clean join.
dir="${dir%/}"

printf '%s/%s.%s.md\n' "$dir" "$STEM" "$safe_token"
