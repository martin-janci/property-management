#!/usr/bin/env bash
# MCP-push size guard (issue #1014) — defence-in-depth for the Phase 6 push.
#
# When `git push` is HTTP-403'd by the sandbox proxy, dispatcher Phase 6 falls
# back to the GitHub MCP push_files tool. That tool requires the FULL file
# content inline in the tool call; a large literal (~tens of KB and up)
# truncates on emission, so the file lands on dev silently corrupted (issue
# #1014: action-list.json was stubbed to 1-2 items).
#
# The structural fix is action-list-reconcile.sh, which keeps action-list.json
# small by archiving terminal items every run. THIS guard is the belt-and-
# suspenders backstop: before the MCP push path runs, it checks every file the
# dispatcher is about to push and FAILS CLOSED if any exceeds a safe inline
# size. A blocked push is recoverable (next run retries on a fixed base); a
# silently-truncated push is not.
#
# It is a no-op when PUSH_METHOD != mcp (direct `git push` has no inline limit).
#
# Usage:
#   PUSH_METHOD=mcp ./.research/mcp-push-size-guard.sh <file> [<file>...]
#   # or read the staged set from git:
#   PUSH_METHOD=mcp ./.research/mcp-push-size-guard.sh --staged
#
# Tunables (env):
#   PUSH_METHOD            mcp (default) | git. Guard only enforces under mcp.
#   MCP_INLINE_MAX_BYTES   per-file hard ceiling, default 65536 (64 KiB).
#                          Files above this are NOT safe to inline-push.
#
# Exit codes:
#   0  all files safe to inline-push (or PUSH_METHOD != mcp → skipped).
#   3  at least one file exceeds the inline ceiling — DO NOT MCP-push.
#      Remediation printed on stderr (run the reconcilers / use git push).
#  64  usage error.

set -euo pipefail

PUSH_METHOD="${PUSH_METHOD:-mcp}"
MCP_INLINE_MAX_BYTES="${MCP_INLINE_MAX_BYTES:-65536}"

if [ "$#" -eq 0 ]; then
  echo "usage: $0 <file> [<file>...] | --staged" >&2
  exit 64
fi

# Direct git push has no inline-content limit — nothing to guard.
if [ "$PUSH_METHOD" != "mcp" ]; then
  echo "mcp-push-size-guard: PUSH_METHOD=$PUSH_METHOD (not mcp) — skipped"
  exit 0
fi

FILES=()
if [ "${1:-}" = "--staged" ]; then
  while IFS= read -r f; do
    [ -n "$f" ] && FILES+=("$f")
  done < <(git diff --cached --name-only --diff-filter=ACM)
else
  FILES=("$@")
fi

if [ "${#FILES[@]}" -eq 0 ]; then
  echo "mcp-push-size-guard: no files to check — OK"
  exit 0
fi

file_size() { wc -c < "$1" | tr -d '[:space:]'; }

OVERSIZE=0
for f in "${FILES[@]}"; do
  [ -f "$f" ] || continue
  sz=$(file_size "$f")
  if [ "$sz" -gt "$MCP_INLINE_MAX_BYTES" ]; then
    echo "mcp-push-size-guard: OVERSIZE $f = ${sz}B > ${MCP_INLINE_MAX_BYTES}B ceiling" >&2
    OVERSIZE=$((OVERSIZE + 1))
  else
    echo "mcp-push-size-guard: ok       $f = ${sz}B"
  fi
done

if [ "$OVERSIZE" -gt 0 ]; then
  cat >&2 <<EOF

mcp-push-size-guard: ABORT — $OVERSIZE file(s) exceed the MCP inline-push ceiling.
  Inline MCP push_files would TRUNCATE these and corrupt them on dev (issue #1014).
  Remediation (in order):
    1. Run the archive reconcilers to shrink active state, then re-stage:
         bash .research/action-list-reconcile.sh --apply
         bash .research/archive-reconcile.sh --apply
    2. If a file is still oversize after archiving, do NOT MCP-push it. Land
       this run via direct 'git push' (PUSH_METHOD=git) where the proxy allows,
       or defer — the next run retries on a corrected base. A blocked push is
       recoverable; a truncated one is not.
EOF
  exit 3
fi

echo "mcp-push-size-guard: all ${#FILES[@]} file(s) within ${MCP_INLINE_MAX_BYTES}B inline ceiling — safe to MCP-push"
exit 0
