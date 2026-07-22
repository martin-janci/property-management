#!/usr/bin/env bash
# Fixture self-test for scripts/verify-impact.sh.
#
# Uses the TEST-ONLY VERIFY_FILES override to feed synthetic changed-path
# sets into the escalation table and asserts the emitted VERIFY-PLAN
# (via --plan-only — no command is ever executed here).
#
# Run: ./scripts/test-verify-impact.sh
# Exit: 0 all scenarios pass, 1 any assertion failed.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
VI="$ROOT/scripts/verify-impact.sh"
[ -x "$VI" ] || { echo "FAIL: $VI not executable" >&2; exit 1; }

pass=0
fail=0
CURRENT=""
PLAN=""
RC=0

scenario() {
  CURRENT="$1"
  shift
  set +e
  PLAN="$(VERIFY_FILES="$1" "$VI" --plan-only 2>/dev/null)"
  RC=$?
  set -e
}

assert_contains() {
  if printf '%s\n' "$PLAN" | grep -qF -- "$1"; then
    printf '  ok   [%s] contains: %s\n' "$CURRENT" "$1"; pass=$((pass+1))
  else
    printf '  FAIL [%s] missing:  %s\n' "$CURRENT" "$1"; fail=$((fail+1))
    printf '%s\n' "$PLAN" | sed 's/^/       | /'
  fi
}

assert_not_contains() {
  if printf '%s\n' "$PLAN" | grep -qF -- "$1"; then
    printf '  FAIL [%s] must-not-contain: %s\n' "$CURRENT" "$1"; fail=$((fail+1))
    printf '%s\n' "$PLAN" | sed 's/^/       | /'
  else
    printf '  ok   [%s] absent:   %s\n' "$CURRENT" "$1"; pass=$((pass+1))
  fi
}

assert_rc() {
  if [ "$RC" -eq "$1" ]; then
    printf '  ok   [%s] exit=%s\n' "$CURRENT" "$1"; pass=$((pass+1))
  else
    printf '  FAIL [%s] exit=%s expected=%s\n' "$CURRENT" "$RC" "$1"; fail=$((fail+1))
  fi
}

assert_cmd_count() {
  local n
  n="$(printf '%s\n' "$PLAN" | grep -cE '^  [^#]' || true)"
  if [ "$n" -eq "$1" ]; then
    printf '  ok   [%s] command-count=%s\n' "$CURRENT" "$1"; pass=$((pass+1))
  else
    printf '  FAIL [%s] command-count=%s expected=%s\n' "$CURRENT" "$n" "$1"; fail=$((fail+1))
    printf '%s\n' "$PLAN" | sed 's/^/       | /'
  fi
}

echo "==> test-verify-impact"

# --- 1. docs/.research only → empty plan, exit 0 ------------------------------
scenario docs-only "docs/spec1.0.md,.research/backlog.json,CLAUDE.md"
assert_rc 0
assert_contains "VERIFY-PLAN base="
assert_contains "files=3"
assert_cmd_count 0

# docs-only must also run green end-to-end (no commands → safe to execute)
CURRENT=docs-only-exec
set +e
OUT="$(VERIFY_FILES="docs/spec1.0.md" "$VI" 2>/dev/null)"
RC=$?
set -e
PLAN="$OUT"
assert_rc 0
assert_contains "VERIFY OK "

# --- 2. single-crate backend change (crates/common) ---------------------------
scenario backend-common "backend/crates/common/src/lib.rs"
assert_rc 0
assert_contains "cd backend && cargo fmt --all -- --check"
assert_contains "cargo clippy -p common --all-targets -- -D warnings"
assert_not_contains "--workspace"
assert_not_contains "cargo check"
assert_not_contains "cargo build"
assert_not_contains "--release"
assert_not_contains "pnpm"

# tests are emitted per crate with the same crate list as clippy
CURRENT=backend-common-tests
if printf '%s\n' "$PLAN" | grep -qE '^  cd backend && cargo (nextest run|test) -p common$'; then
  printf '  ok   [%s] test line for common present\n' "$CURRENT"; pass=$((pass+1))
else
  printf '  FAIL [%s] no per-crate test line for common\n' "$CURRENT"; fail=$((fail+1))
  printf '%s\n' "$PLAN" | sed 's/^/       | /'
fi

# --- 3. db migration path → full backend + WARN -------------------------------
scenario migration "backend/crates/db/migrations/20260722000000_add_thing.sql"
assert_rc 0
assert_contains "# WARN: migrations touched"
assert_contains "5433"
assert_contains "cargo clippy --workspace --all-targets -- -D warnings"
assert_not_contains "cargo build"
assert_not_contains "--release"

# --- 4. typespec path → tsp + full backend clippy + full frontend typecheck ---
scenario typespec "docs/api/typespec/main.tsp"
assert_rc 0
assert_contains "cd docs/api/typespec && npx tsp compile ."
assert_contains "cargo clippy --workspace --all-targets -- -D warnings"
assert_contains "cd frontend && pnpm typecheck"
assert_not_contains "cargo nextest run --workspace"
assert_not_contains "cargo test --workspace"

# --- 5. frontend lockfile → full frontend -------------------------------------
scenario fe-lockfile "frontend/pnpm-lock.yaml"
assert_rc 0
assert_contains "cd frontend && pnpm check"
assert_contains "cd frontend && pnpm typecheck"
assert_contains "cd frontend && pnpm test"
assert_not_contains "--filter"
assert_not_contains "cargo"

# --- 6. single frontend package → scoped biome + filtered typecheck/test ------
scenario fe-package "frontend/apps/ppt-web/src/main.tsx"
assert_rc 0
assert_contains "pnpm exec biome check apps/ppt-web/src/main.tsx"
assert_contains 'pnpm --filter "...['
assert_contains '" typecheck'
assert_contains '" test'
assert_not_contains "cargo"

# --- 7. mixed backend + frontend + docs ---------------------------------------
scenario mixed "backend/crates/common/src/lib.rs,frontend/apps/ppt-web/src/main.tsx,docs/spec1.0.md"
assert_rc 0
assert_contains "cargo clippy -p common --all-targets -- -D warnings"
assert_contains 'pnpm --filter "...['
assert_contains "pnpm exec biome check apps/ppt-web/src/main.tsx"
assert_not_contains "--workspace"
assert_not_contains "gradlew"

# --- 8. determinism: same input → same plan text ------------------------------
CURRENT=determinism
P1="$(VERIFY_FILES="backend/crates/common/src/lib.rs" "$VI" --plan-only 2>/dev/null)"
P2="$(VERIFY_FILES="backend/crates/common/src/lib.rs" "$VI" --plan-only 2>/dev/null)"
if [ "$P1" = "$P2" ]; then
  printf '  ok   [%s] identical plans across runs\n' "$CURRENT"; pass=$((pass+1))
else
  printf '  FAIL [%s] plans differ across runs\n' "$CURRENT"; fail=$((fail+1))
fi

echo
if [ "$fail" -eq 0 ]; then
  echo "==> test-verify-impact: PASS ($pass assertions)"
  exit 0
else
  echo "==> test-verify-impact: FAIL ($fail failed, $pass passed)"
  exit 1
fi
