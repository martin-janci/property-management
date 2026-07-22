#!/usr/bin/env bash
# setup-dev-host.sh — idempotent, OPT-IN dev-host tuning for PPT builds.
#
# What it does (machine-level, ~/.cargo/config.toml — NEVER the repo's
# committed backend/.cargo/config.toml):
#   1. Installs/locates sccache and sets it as rustc-wrapper. Local-disk
#      backend: SCCACHE_DIR=$HOME/.cache/sccache, SCCACHE_CACHE_SIZE=50G.
#      Cargo incremental stays ON (we do NOT set CARGO_INCREMENTAL) —
#      cargo only does incremental for workspace members, which sccache
#      skips anyway; sccache caches the ~790-crate dependency graph.
#   2. With --mold: configures mold as the linker for x86_64-unknown-linux-gnu
#      (only if `command -v mold` succeeds).
#
# Usage: ./scripts/setup-dev-host.sh [--mold] [--dry-run]
# Safe to re-run; each step reports "already configured" when nothing to do.

set -euo pipefail

WANT_MOLD=0
DRY_RUN=0
for a in "$@"; do
  case "$a" in
    --mold) WANT_MOLD=1;;
    --dry-run) DRY_RUN=1;;
    -h|--help) sed -n '2,16p' "$0"; exit 0;;
    *) echo "unknown arg: $a" >&2; exit 2;;
  esac
done

CARGO_HOME_DIR="${CARGO_HOME:-$HOME/.cargo}"
CFG="$CARGO_HOME_DIR/config.toml"

# Refuse to ever touch the repo's committed cargo config.
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [ -n "$REPO_ROOT" ] && [ "$(readlink -f "$CFG" 2>/dev/null || echo "$CFG")" = "$REPO_ROOT/backend/.cargo/config.toml" ]; then
  echo "error: refusing to modify the repo's committed backend/.cargo/config.toml" >&2
  exit 2
fi

say() { echo "setup-dev-host: $*"; }
doit() { if [ "$DRY_RUN" -eq 1 ]; then say "[dry-run] $*"; else eval "$*"; fi; }

mkdir -p "$CARGO_HOME_DIR"
touch "$CFG"

# --- 1. sccache ---------------------------------------------------------------
SCCACHE_BIN="$(command -v sccache || true)"
if [ -z "$SCCACHE_BIN" ]; then
  say "sccache not found — installing (cargo install sccache --locked)…"
  doit "cargo install sccache --locked"
  SCCACHE_BIN="$(command -v sccache || echo "$CARGO_HOME_DIR/bin/sccache")"
else
  say "sccache found at $SCCACHE_BIN"
fi

if grep -q 'rustc-wrapper' "$CFG" 2>/dev/null; then
  say "rustc-wrapper already configured in $CFG — leaving as-is"
else
  say "writing [build] rustc-wrapper = \"$SCCACHE_BIN\" into $CFG"
  if [ "$DRY_RUN" -eq 0 ]; then
    printf '\n[build]\nrustc-wrapper = "%s"\n' "$SCCACHE_BIN" >> "$CFG"
  fi
fi

if grep -q 'SCCACHE_DIR' "$CFG" 2>/dev/null; then
  say "SCCACHE_DIR already configured in $CFG — leaving as-is"
else
  say "writing [env] SCCACHE_DIR=$HOME/.cache/sccache, SCCACHE_CACHE_SIZE=50G into $CFG"
  if [ "$DRY_RUN" -eq 0 ]; then
    printf '\n[env]\nSCCACHE_DIR = "%s"\nSCCACHE_CACHE_SIZE = "50G"\n' "$HOME/.cache/sccache" >> "$CFG"
  fi
fi
mkdir -p "$HOME/.cache/sccache"
say "NOTE: cargo incremental stays ON (no CARGO_INCREMENTAL override) — intentional; see .research/build-experience-report-2026-07-22.md"

# --- 2. mold (opt-in) ---------------------------------------------------------
if [ "$WANT_MOLD" -eq 1 ]; then
  if ! command -v mold >/dev/null 2>&1; then
    say "--mold requested but mold is not installed — skipping (install it via your package manager)"
  elif grep -q 'link-arg=-fuse-ld=mold' "$CFG" 2>/dev/null; then
    say "mold linker already configured in $CFG — leaving as-is"
  else
    say "writing mold linker config for x86_64-unknown-linux-gnu into $CFG"
    if [ "$DRY_RUN" -eq 0 ]; then
      cat >> "$CFG" <<'EOF'

[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
EOF
    fi
    say "NOTE: the repo's committed backend/.cargo/config.toml may set its own linker flags; repo config wins for repo builds"
  fi
fi

say "done. Verify with: sccache --show-stats (after a build)"
