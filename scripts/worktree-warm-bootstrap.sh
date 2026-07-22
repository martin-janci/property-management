#!/usr/bin/env bash
# worktree-warm-bootstrap.sh — warm a fresh worktree so the first agent build
# isn't a cold build.
#
#   1. If a template target dir exists ($PPT_TARGET_TEMPLATE, default
#      $HOME/.cache/ppt-target-template/dev), hardlink-clone it (`cp -al`)
#      into <worktree>/backend/target. Hardlinks cost ~no disk and cargo
#      breaks links on rewrite, so worktrees stay independent.
#   2. If <worktree>/frontend/node_modules is missing, run `pnpm install`.
#
# Usage: ./scripts/worktree-warm-bootstrap.sh <worktree-path>
#
# Template refresh (suggested nightly cron on the dev host):
#   # refresh the dev target template at 03:30 from a dedicated clean checkout
#   30 3 * * * cd $HOME/.cache/ppt-target-template/checkout && \
#     git fetch origin dev && git reset --hard origin/dev && \
#     (cd backend && cargo clippy --workspace --all-targets -- -D warnings) && \
#     rsync -a --delete backend/target/ $HOME/.cache/ppt-target-template/dev/
#
# Idempotent: skips each step when the destination already exists.

set -euo pipefail

WT="${1:-}"
[ -n "$WT" ] || { echo "usage: worktree-warm-bootstrap.sh <worktree-path>" >&2; exit 2; }
[ -d "$WT" ] || { echo "error: worktree path not found: $WT" >&2; exit 2; }

TEMPLATE="${PPT_TARGET_TEMPLATE:-$HOME/.cache/ppt-target-template/dev}"

# --- 1. hardlink-clone the backend target template ---------------------------
if [ -d "$WT/backend/target" ]; then
  echo "warm-bootstrap: $WT/backend/target already exists — skipping target clone"
elif [ -d "$TEMPLATE" ]; then
  echo "warm-bootstrap: hardlink-cloning $TEMPLATE -> $WT/backend/target (cp -al)…"
  mkdir -p "$WT/backend"
  cp -al "$TEMPLATE" "$WT/backend/target"
  echo "warm-bootstrap: target clone done ($(du -sh "$WT/backend/target" 2>/dev/null | cut -f1) apparent, hardlinked)"
else
  echo "warm-bootstrap: no template at $TEMPLATE — skipping (set PPT_TARGET_TEMPLATE or seed it; see header for the nightly cron)"
fi

# --- 2. frontend deps ---------------------------------------------------------
if [ -d "$WT/frontend/node_modules" ]; then
  echo "warm-bootstrap: frontend/node_modules present — skipping pnpm install"
elif [ -f "$WT/frontend/package.json" ]; then
  echo "warm-bootstrap: running pnpm install in $WT/frontend…"
  (cd "$WT/frontend" && pnpm install)
else
  echo "warm-bootstrap: no frontend/package.json — skipping pnpm install"
fi

echo "warm-bootstrap: done."
