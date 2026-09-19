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
# Append-only-archive routing (issue #1162 / #2743): two files legitimately
# grow without bound and CANNOT be shrunk without losing history — the
# dispatcher's own append-only ledgers `assignments-archive.json` and
# `action-list-archive.json`. #1162 originally kept the pipeline moving when
# those crossed the ceiling by NOT treating them as a fail-closed #1014 hazard:
# they are append-only and the dispatcher is their sole writer, so a direct
# `git push` (which has NO inline-content limit) lands them intact. That routing
# regressed — the guard began fail-closing on ALL oversize files — and a single
# archive append could wedge every Phase 6 push (#2743). This restores it: when
# the ONLY oversize staged files are these known append-only archives, the guard
# exits 4 ("route via git push") instead of 3 ("abort"). An oversize ACTIVE
# state file (action-list.json / assignments.json — the #1014 corruption class)
# still fail-closes hard (exit 3): those must be shrunk by the reconcilers, never
# inline-pushed.
#
# Tunables (env):
#   PUSH_METHOD            mcp (default) | git. Guard only enforces under mcp.
#   MCP_INLINE_MAX_BYTES   per-file hard ceiling, default 65536 (64 KiB).
#                          Files above this are NOT safe to inline-push.
#   MCP_ARCHIVE_BASENAMES  space-separated basenames of append-only archives
#                          that are safe to route via git push when oversize.
#                          Default: "assignments-archive.json action-list-archive.json".
#
# Exit codes:
#   0  all files safe to inline-push (or PUSH_METHOD != mcp → skipped).
#   3  at least one NON-archive (active-state) file exceeds the ceiling — DO NOT
#      push at all this run; the reconcilers must shrink it first (#1014).
#   4  the ONLY oversize files are known append-only archives — NOT safe to
#      MCP-inline-push, but SAFE to route this commit via direct `git push`
#      (no inline-content limit). Caller should set PUSH_METHOD=git (#1162/#2743).
#  64  usage error.

set -euo pipefail

PUSH_METHOD="${PUSH_METHOD:-mcp}"
MCP_INLINE_MAX_BYTES="${MCP_INLINE_MAX_BYTES:-65536}"
MCP_ARCHIVE_BASENAMES="${MCP_ARCHIVE_BASENAMES:-assignments-archive.json action-list-archive.json}"

# Is $1 one of the known append-only archives (matched by basename)?
is_archive() {
  local b; b="$(basename "$1")"
  local a
  for a in $MCP_ARCHIVE_BASENAMES; do
    [ "$b" = "$a" ] && return 0
  done
  return 1
}

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

ACTIVE_OVERSIZE=0     # non-archive state files over the ceiling — #1014 hazard
ARCHIVE_OVERSIZE=0    # known append-only archives over the ceiling — git-routable
for f in "${FILES[@]}"; do
  [ -f "$f" ] || continue
  sz=$(file_size "$f")
  if [ "$sz" -gt "$MCP_INLINE_MAX_BYTES" ]; then
    if is_archive "$f"; then
      echo "mcp-push-size-guard: OVERSIZE(archive) $f = ${sz}B > ${MCP_INLINE_MAX_BYTES}B — route via git push" >&2
      ARCHIVE_OVERSIZE=$((ARCHIVE_OVERSIZE + 1))
    else
      echo "mcp-push-size-guard: OVERSIZE $f = ${sz}B > ${MCP_INLINE_MAX_BYTES}B ceiling" >&2
      ACTIVE_OVERSIZE=$((ACTIVE_OVERSIZE + 1))
    fi
  else
    echo "mcp-push-size-guard: ok       $f = ${sz}B"
  fi
done

# An oversize ACTIVE state file is the #1014 corruption class — fail closed,
# regardless of any archive that is also oversize. It must be shrunk first.
if [ "$ACTIVE_OVERSIZE" -gt 0 ]; then
  cat >&2 <<EOF

mcp-push-size-guard: ABORT — $ACTIVE_OVERSIZE active-state file(s) exceed the MCP inline-push ceiling.
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

# Only append-only archives are oversize — they cannot be shrunk without losing
# history and are safe to git-push (no inline limit). Route, do NOT wedge (#1162/#2743).
if [ "$ARCHIVE_OVERSIZE" -gt 0 ]; then
  cat >&2 <<EOF

mcp-push-size-guard: ROUTE=git — $ARCHIVE_OVERSIZE append-only archive(s) exceed the ${MCP_INLINE_MAX_BYTES}B inline ceiling.
  These are append-only ledgers (dispatcher is the sole writer); they cannot be
  shrunk without losing history and MUST NOT be inline-MCP-pushed (would truncate,
  #1014). Land this commit via direct 'git push' (PUSH_METHOD=git) — it has no
  inline-content limit (#1162/#2743). Caller: set PUSH_METHOD=git for this push.
EOF
  exit 4
fi

echo "mcp-push-size-guard: all ${#FILES[@]} file(s) within ${MCP_INLINE_MAX_BYTES}B inline ceiling — safe to MCP-push"
exit 0
