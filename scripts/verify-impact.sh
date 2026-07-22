#!/usr/bin/env bash
# Deterministic impact-scoped verify gate (`just verify`).
#
# The plan is a PURE FUNCTION of:
#   1. the merge-base with origin/dev (or --base <ref>),
#   2. the sorted set of changed paths (committed merge-base..HEAD + uncommitted),
#   3. the escalation table below (from .research/build-experience-report-2026-07-22.md).
#
# Escalation table:
#   backend/Cargo.{toml,lock}, rust-toolchain*, backend/.cargo/**, any build.rs
#                                  -> full backend (fmt + clippy --workspace + tests --workspace)
#   backend/crates/db/migrations/** -> full backend + WARN (SQL truth needs live DB on :5433)
#   docs/api/typespec/**            -> tsp compile + full backend clippy + full frontend typecheck
#   frontend/pnpm-lock.yaml, pnpm-workspace.yaml, root package.json/tsconfig*, biome.json*
#                                  -> full frontend (check + typecheck + test)
#   other backend/**                -> owning crate + reverse dependents (one cargo metadata call)
#   other frontend/**               -> pnpm --filter "...[<merge-base>]" (dependent selection)
#   mobile-native gradle files      -> full gradle build; other mobile-native/** -> spotlessCheck + test
#   docs/, .research/, .claude/, .github/, scripts/, other non-build paths
#                                  -> no commands (empty plan, exit 0)
#
# Backend chain per crate: cargo fmt --all -- --check, then
# cargo clippy -p <crate> --all-targets -- -D warnings, then
# cargo nextest run -p <crate> (fallback cargo test -p <crate>).
# NO `cargo check` step (clippy subsumes it; they don't share artifacts).
# NEVER `--release`, NEVER `cargo build`.
#
# Every cargo invocation is serialized across agents on this host via
# flock on /tmp/ppt-cargo.lock (one heavy compile per host).
#
# Output contract (greppable, quoted verbatim in PR bodies):
#   VERIFY-PLAN base=<sha> files=<n>
#     <one indented line per command; lines starting with '# ' are notes>
#   VERIFY OK <sha1-of-plan-text>        (success / empty plan)
#   VERIFY FAIL: <cmd>                   (first failing check)
#
# Usage: verify-impact.sh [--plan-only] [--base <ref>] [--full]
# Exit codes: 0 green/empty plan, 1 first failing check, 2 usage/env error.
#
# TEST-ONLY: VERIFY_FILES="path1,path2" bypasses git diff and uses the given
# comma-separated paths as the changed set (used by scripts/test-verify-impact.sh).
# Never set it in real verification runs.

set -euo pipefail

PLAN_ONLY=0
BASE_REF="origin/dev"
FORCE_FULL=0
CARGO_LOCK=/tmp/ppt-cargo.lock
FLOCK_TIMEOUT=7200   # generous: two hours; heavy workspace compiles queue behind each other

usage() {
  echo "usage: verify-impact.sh [--plan-only] [--base <ref>] [--full]" >&2
  exit 2
}

while [ $# -gt 0 ]; do
  case "$1" in
    --plan-only) PLAN_ONLY=1; shift;;
    --base) [ $# -ge 2 ] || usage; BASE_REF="$2"; shift 2;;
    --full) FORCE_FULL=1; shift;;
    -h|--help) sed -n '2,40p' "$0"; exit 0;;
    *) echo "unknown arg: $1" >&2; usage;;
  esac
done

# --- repo root (NEVER hardcode mount roots: this repo is checked out under
# --- both /mnt/sda4/... and /home/mjanci/... worktrees) -----------------------
ROOT="$(git rev-parse --show-toplevel 2>/dev/null)" || { echo "error: not in a git repo" >&2; exit 2; }
cd "$ROOT"

# --- merge base ---------------------------------------------------------------
if ! git rev-parse --verify -q "$BASE_REF" >/dev/null; then
  if [ "$BASE_REF" = "origin/dev" ] && git rev-parse --verify -q dev >/dev/null; then
    BASE_REF="dev"
  else
    echo "error: base ref '$BASE_REF' not found (git fetch origin dev?)" >&2
    exit 2
  fi
fi
MB="$(git merge-base "$BASE_REF" HEAD)" || { echo "error: no merge-base with $BASE_REF" >&2; exit 2; }

# --- changed paths: committed (merge-base..HEAD) + uncommitted, sorted --------
if [ -n "${VERIFY_FILES:-}" ]; then
  # TEST-ONLY path (see header).
  CHANGED="$(printf '%s' "$VERIFY_FILES" | tr ',' '\n' | sed '/^$/d' | sort -u)"
  TEST_MODE=1
else
  CHANGED="$( { git diff --name-only "$MB"..HEAD 2>/dev/null;
                git diff --name-only 2>/dev/null;
                git diff --name-only --cached 2>/dev/null;
              } | sed '/^$/d' | sort -u )"
  TEST_MODE=0
fi
NFILES="$(printf '%s' "$CHANGED" | grep -c . || true)"

# --- classification -----------------------------------------------------------
FULL_BACKEND=0
WARN_MIGRATION=0
TYPESPEC=0
FULL_FRONTEND=0
FULL_GRADLE=0
MOBILE_LIGHT=0
BE_FILES=""          # backend files needing owning-crate resolution
FE_FILES=""          # frontend files for scoped pnpm run

while IFS= read -r f; do
  [ -z "$f" ] && continue
  case "$f" in
    backend/Cargo.toml|backend/Cargo.lock|rust-toolchain*|backend/rust-toolchain*|backend/.cargo/*|backend/*/build.rs)
      FULL_BACKEND=1;;
    backend/crates/db/migrations/*)
      FULL_BACKEND=1; WARN_MIGRATION=1;;
    docs/api/typespec/*)
      TYPESPEC=1;;
    backend/docs/*)
      : ;;                                   # backend docs: no build impact
    backend/*)
      BE_FILES="$BE_FILES$f"$'\n';;
    frontend/pnpm-lock.yaml|frontend/pnpm-workspace.yaml|frontend/package.json|frontend/biome.json*|frontend/tsconfig*.json)
      FULL_FRONTEND=1;;
    frontend/*)
      FE_FILES="$FE_FILES$f"$'\n';;
    mobile-native/gradle*|mobile-native/*.gradle*|mobile-native/gradle/*|mobile-native/*/build.gradle*|mobile-native/settings.gradle*)
      FULL_GRADLE=1;;
    mobile-native/*)
      MOBILE_LIGHT=1;;
    *)
      : ;;                                   # docs/, .research/, .claude/, scripts/, .github/, … — no build impact
  esac
done <<< "$CHANGED"

if [ "$FORCE_FULL" -eq 1 ]; then
  FULL_BACKEND=1; FULL_FRONTEND=1; FULL_GRADLE=1
fi

# --- backend owning crates + reverse dependents (one cargo metadata call) -----
BE_CRATES=""
if [ "$FULL_BACKEND" -eq 0 ] && [ -n "$BE_FILES" ]; then
  command -v jq >/dev/null 2>&1 || { echo "error: jq required for crate-impact resolution" >&2; exit 2; }
  META="$(cd backend && cargo metadata --no-deps --format-version=1 2>/dev/null)" \
    || { echo "error: cargo metadata failed in backend/" >&2; exit 2; }

  # name<TAB>repo-relative-crate-dir (paths normalized against $ROOT, not hardcoded)
  CRATE_MAP="$(printf '%s' "$META" | jq -r --arg root "$ROOT/" \
    '.packages[] | "\(.name)\t\(.manifest_path | sub("^" + $root; "") | sub("/Cargo\\.toml$"; ""))"')"

  # workspace-internal reverse edges: "dep depender"
  MEMBER_NAMES="$(printf '%s\n' "$CRATE_MAP" | cut -f1 | sort -u)"
  REV_EDGES="$(printf '%s' "$META" | jq -r \
    '.packages as $p | ($p | map(.name)) as $names
     | $p[] | .name as $n | .dependencies[] | select(.name as $d | $names | index($d)) | "\(.name) \($n)"' \
    | sort -u)"

  # owning crate per changed file: longest matching crate-dir prefix
  SEEDS=""
  while IFS= read -r f; do
    [ -z "$f" ] && continue
    best=""; bestlen=0
    while IFS=$'\t' read -r name dir; do
      case "$f" in
        "$dir"/*)
          if [ "${#dir}" -gt "$bestlen" ]; then best="$name"; bestlen="${#dir}"; fi;;
      esac
    done <<< "$CRATE_MAP"
    if [ -n "$best" ]; then
      SEEDS="$SEEDS$best"$'\n'
    else
      FULL_BACKEND=1                        # backend file outside any crate — be conservative
    fi
  done <<< "$BE_FILES"

  if [ "$FULL_BACKEND" -eq 0 ]; then
    # transitive reverse-dependent closure over workspace edges
    IMPACTED="$(printf '%s' "$SEEDS" | sed '/^$/d' | sort -u)"
    while :; do
      NEXT="$IMPACTED"
      while IFS= read -r c; do
        [ -z "$c" ] && continue
        ADD="$(printf '%s\n' "$REV_EDGES" | awk -v d="$c" '$1==d{print $2}')"
        [ -n "$ADD" ] && NEXT="$(printf '%s\n%s' "$NEXT" "$ADD")"
      done <<< "$IMPACTED"
      NEXT="$(printf '%s' "$NEXT" | sed '/^$/d' | sort -u)"
      [ "$NEXT" = "$IMPACTED" ] && break
      IMPACTED="$NEXT"
    done
    BE_CRATES="$IMPACTED"
  fi
fi

# --- test runner selection (deterministic per host) ---------------------------
if command -v cargo-nextest >/dev/null 2>&1; then
  TEST_CMD="cargo nextest run"
else
  TEST_CMD="cargo test"
fi

# --- build the plan -----------------------------------------------------------
PLAN_LINES=""
add() { PLAN_LINES="$PLAN_LINES  $1"$'\n'; }

if [ "$WARN_MIGRATION" -eq 1 ]; then
  add "# WARN: migrations touched — SQL truth needs a live DB (localhost:5433); runtime sqlx compiles without one, so run DB-backed tests before merge"
fi

if [ "$TYPESPEC" -eq 1 ]; then
  add "cd docs/api/typespec && npx tsp compile ."
  add "# NOTE: if the OpenAPI spec changed, regenerate clients (just generate-api) and commit the diff"
fi

if [ "$FULL_BACKEND" -eq 1 ]; then
  add "cd backend && cargo fmt --all -- --check"
  add "cd backend && cargo clippy --workspace --all-targets -- -D warnings"
  add "cd backend && $TEST_CMD --workspace"
elif [ "$TYPESPEC" -eq 1 ]; then
  add "cd backend && cargo fmt --all -- --check"
  add "cd backend && cargo clippy --workspace --all-targets -- -D warnings"
elif [ -n "$BE_CRATES" ]; then
  add "cd backend && cargo fmt --all -- --check"
  while IFS= read -r c; do
    [ -z "$c" ] && continue
    add "cd backend && cargo clippy -p $c --all-targets -- -D warnings"
  done <<< "$BE_CRATES"
  while IFS= read -r c; do
    [ -z "$c" ] && continue
    add "cd backend && $TEST_CMD -p $c"
  done <<< "$BE_CRATES"
fi

if [ "$FULL_FRONTEND" -eq 1 ]; then
  add "cd frontend && pnpm check"
  add "cd frontend && pnpm typecheck"
  add "cd frontend && pnpm test"
elif [ "$TYPESPEC" -eq 1 ] && [ -z "$FE_FILES" ]; then
  add "cd frontend && pnpm typecheck"
elif [ -n "$FE_FILES" ]; then
  # biome on the changed files (paths relative to frontend/), skipping deleted ones
  BIOME_PATHS=""
  while IFS= read -r f; do
    [ -z "$f" ] && continue
    case "$f" in
      *.ts|*.tsx|*.js|*.jsx|*.json|*.css)
        rel="${f#frontend/}"
        if [ "$TEST_MODE" -eq 1 ] || [ -e "$f" ]; then
          BIOME_PATHS="$BIOME_PATHS $rel"
        fi;;
    esac
  done <<< "$FE_FILES"
  [ -n "$BIOME_PATHS" ] && add "cd frontend && pnpm exec biome check$BIOME_PATHS"
  add "cd frontend && pnpm --filter \"...[$MB]\" typecheck"
  add "cd frontend && pnpm --filter \"...[$MB]\" test"
  if [ "$TYPESPEC" -eq 1 ]; then
    add "cd frontend && pnpm typecheck"
  fi
fi

if [ "$FULL_GRADLE" -eq 1 ]; then
  add "cd mobile-native && ./gradlew build"
elif [ "$MOBILE_LIGHT" -eq 1 ]; then
  add "cd mobile-native && ./gradlew spotlessCheck test"
fi

PLAN_TEXT="VERIFY-PLAN base=$MB files=$NFILES"$'\n'"$PLAN_LINES"
PLAN_HASH="$(printf '%s' "$PLAN_TEXT" | sha1sum | awk '{print $1}')"

printf '%s' "$PLAN_TEXT"

if [ "$PLAN_ONLY" -eq 1 ]; then
  exit 0
fi

# --- execute ------------------------------------------------------------------
run_one() {
  # cargo invocations serialize on a host-wide lock; note on stderr if we wait
  local cmd="$1"
  case "$cmd" in
    *"cargo "*)
      if ! flock -n "$CARGO_LOCK" true 2>/dev/null; then
        echo "verify-impact: waiting for $CARGO_LOCK (another compile is running on this host)" >&2
      fi
      flock -w "$FLOCK_TIMEOUT" "$CARGO_LOCK" bash -c "$cmd"
      ;;
    *)
      bash -c "$cmd"
      ;;
  esac
}

while IFS= read -r line; do
  [ -z "$line" ] && continue
  cmd="${line#  }"
  case "$cmd" in
    "#"*) continue;;    # notes/warnings are not executable
  esac
  echo "==> $cmd" >&2
  if ! run_one "$cmd"; then
    echo "VERIFY FAIL: $cmd"
    exit 1
  fi
done <<< "$PLAN_LINES"

echo "VERIFY OK $PLAN_HASH"
exit 0
