#!/usr/bin/env bash
# Detects whether a newline-delimited list of changed paths contains at
# least one "test file" — used by the security-test-gate workflow to
# decide whether a security-labelled PR satisfies the
# every-security-fix-needs-a-test contract (see workflow header).
#
# Two modes:
#   - Default: reads paths from stdin, prints "true" or "false" to
#     stdout, exits 0.
#   - --self-test: runs the built-in fixture suite and exits 0 iff
#     every fixture produces the expected verdict. Used by the
#     `self-test` job in the workflow to catch matcher regressions
#     before they reach a live security PR.
#
# Test-file patterns recognised:
#   Rust integration   backend/<crate>/tests/<file>.rs      (any depth — see note)
#   Rust unit modules  *_test.rs | *_tests.rs | test_*.rs | tests.rs
#   TypeScript         **/__tests__/**, top-level __tests__/**, *.test.ts(x), *.spec.ts(x)
#   End-to-end         e2e/**
#
# Note on bash `case` globs: `*` in a `case` pattern matches any string
# INCLUDING `/`. So `backend/*/tests/*.rs` already matches paths at
# arbitrary depth (e.g. `backend/servers/api-server/tests/foo.rs`).
# Earlier comments in the workflow claimed `case` globs are
# single-level; that was wrong (finding #3). The patterns below are
# kept tight against the current repo layout but the underlying glob
# semantics impose no depth limit.

set -euo pipefail

# ---- detection ------------------------------------------------------

detect() {
  local input="$1"
  local has_test=false
  local f
  while IFS= read -r f; do
    [ -z "$f" ] && continue
    case "$f" in
      # Rust integration test files anywhere under a crate's tests/ dir.
      backend/*/tests/*.rs)
        has_test=true
        ;;
      # Rust files with _test / _tests / test_ naming conventions.
      *_test.rs | *_tests.rs | */test_*.rs | test_*.rs | */tests.rs | tests.rs)
        has_test=true
        ;;
      # TypeScript __tests__ directories — both nested AND top-level
      # (finding #4 — top-level __tests__/ paths not recognised).
      */__tests__/* | __tests__/*)
        has_test=true
        ;;
      # TypeScript test / spec files.
      *.test.ts | *.spec.ts | *.test.tsx | *.spec.tsx)
        has_test=true
        ;;
      # End-to-end tests.
      e2e/*)
        has_test=true
        ;;
    esac
  done <<< "$input"
  printf '%s' "$has_test"
}

# ---- self-test fixtures --------------------------------------------

self_test() {
  local failures=0
  local label expected got input
  # Each fixture: label | expected | newline-separated paths (use ;; as
  # the line separator so the array is one entry per fixture).
  run() {
    label="$1"
    expected="$2"
    input="$3"
    got=$(detect "$input")
    if [ "$got" != "$expected" ]; then
      printf 'FAIL: %s — expected=%s got=%s\n' "$label" "$expected" "$got" >&2
      failures=$((failures + 1))
    else
      printf 'ok:   %s\n' "$label"
    fi
  }

  # Positive cases — each should detect a test.
  run "rust integration test" "true" "backend/servers/api-server/tests/foo.rs"
  run "rust integration deep" "true" "backend/servers/api-server/tests/sub/dir/foo.rs"
  run "rust unit _test.rs" "true" "backend/crates/db/src/queries_test.rs"
  run "rust unit _tests.rs" "true" "backend/crates/db/src/queries_tests.rs"
  run "rust unit test_*.rs nested" "true" "backend/crates/db/src/test_queries.rs"
  run "rust unit test_*.rs top-level" "true" "test_queries.rs"
  run "rust unit tests.rs" "true" "backend/crates/db/src/tests.rs"
  run "ts __tests__ nested" "true" "frontend/apps/ppt-web/src/__tests__/foo.test.ts"
  run "ts __tests__ top-level" "true" "__tests__/foo.test.ts"
  run "ts *.test.ts" "true" "frontend/apps/ppt-web/src/foo.test.ts"
  run "ts *.spec.tsx" "true" "frontend/apps/ppt-web/src/foo.spec.tsx"
  run "e2e" "true" "e2e/auth-mfa.spec.ts"
  run "mixed (one test among many)" "true" "$(printf '%s\n' "backend/servers/api-server/src/handlers/auth.rs" "backend/servers/api-server/tests/auth_test.rs" "frontend/apps/ppt-web/src/components/Login.tsx")"

  # Negative cases — none should detect a test.
  run "rust impl only" "false" "backend/servers/api-server/src/handlers/auth.rs"
  run "ts impl only" "false" "frontend/apps/ppt-web/src/api/client.ts"
  run "rust file with 'test' in path but not name" "false" "backend/crates/contests/src/foo.rs"
  run "docs only" "false" "docs/architecture.md"
  run "empty list" "false" ""
  run "multiple non-tests" "false" "$(printf '%s\n' "README.md" "frontend/apps/ppt-web/src/foo.ts" "backend/Cargo.toml")"

  if [ "$failures" -gt 0 ]; then
    printf '\n%s self-test fixture(s) failed\n' "$failures" >&2
    return 1
  fi
  printf '\nAll self-test fixtures passed.\n'
  return 0
}

# ---- entry ----------------------------------------------------------

case "${1:-}" in
  --self-test)
    self_test
    ;;
  "")
    detect "$(cat)"
    ;;
  *)
    printf 'usage: %s [--self-test]\n' "$0" >&2
    exit 64
    ;;
esac
