#!/bin/bash
# Regression test for check-no-orphan-tests.sh.
#
# The orphan-test guard is pure shell wired into the required `check` CI job.
# With no test it could silently rot — a future edit to the `mod <name>;` parse
# or the subdir discovery could break detection and the GH #2158 class of bug
# (a whole tests/ subtree that never compiles) would slip back in unnoticed.
# This test pins the guard's behavior against temp fixture crate roots (the
# checker takes explicit crate roots as arguments).
#
# Usage: ./scripts/check-no-orphan-tests.test.sh
# Exit 0 if every case passes; exit 1 on the first failure.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHECKER="$SCRIPT_DIR/check-no-orphan-tests.sh"

failures=0
tmpdirs=()

cleanup() { for d in "${tmpdirs[@]}"; do rm -rf "$d"; done; }
trap cleanup EXIT

fail() { echo "  ✗ $1"; failures=$((failures + 1)); }
pass() { echo "  ✓ $1"; }

new_crate() {
    # Create a temp crate root with a Cargo.toml + tests/ dir; echo its path.
    local d
    d="$(mktemp -d)"
    tmpdirs+=("$d")
    printf '[package]\nname = "fixture"\n' > "$d/Cargo.toml"
    mkdir -p "$d/tests"
    printf '%s' "$d"
}

# --- Case 1: an un-mod-included subdir is flagged (exit 1) -------------------
echo "case: orphan subtree (no 'mod integration;') is detected"
c1="$(new_crate)"
: > "$c1/tests/some_top_level_tests.rs"           # a real binary, declares nothing
mkdir -p "$c1/tests/integration"
: > "$c1/tests/integration/auth_tests.rs"
: > "$c1/tests/integration/mod.rs"
out="$("$CHECKER" "$c1" 2>&1)"; code=$?
if [[ "$code" -ne 1 ]]; then fail "expected exit 1, got $code"; else pass "exit 1"; fi
if grep -q 'integration/auth_tests.rs' <<< "$out"; then
    pass "orphaned file reported"
else
    fail "expected orphaned file in output; got: $out"
fi

# --- Case 2: a mod-included helper subdir passes (exit 0) --------------------
echo "case: 'mod common;'-included helper subdir passes"
c2="$(new_crate)"
printf 'mod common;\n' > "$c2/tests/foo_tests.rs"
mkdir -p "$c2/tests/common"
: > "$c2/tests/common/mod.rs"
if "$CHECKER" "$c2" >/dev/null 2>&1; then
    pass "exit 0 when subdir is mod-included"
else
    fail "expected exit 0 for a mod common;-included helper dir"
fi

# --- Case 3: only top-level test files, no subdirs (exit 0) ------------------
echo "case: flat tests/ dir passes"
c3="$(new_crate)"
: > "$c3/tests/a_tests.rs"
: > "$c3/tests/b_tests.rs"
if "$CHECKER" "$c3" >/dev/null 2>&1; then
    pass "exit 0 on a flat tests/ dir"
else
    fail "expected exit 0 with no subdirs"
fi

# --- Case 4: 'pub mod' declaration is honored (exit 0) -----------------------
echo "case: 'pub mod helpers;' is recognized as reachable"
c4="$(new_crate)"
printf 'pub mod helpers;\n' > "$c4/tests/entry_tests.rs"
mkdir -p "$c4/tests/helpers"
: > "$c4/tests/helpers/mod.rs"
if "$CHECKER" "$c4" >/dev/null 2>&1; then
    pass "exit 0 for a pub mod-included dir"
else
    fail "expected exit 0 for 'pub mod helpers;'"
fi

# --- Case 5: non-crate dir (no Cargo.toml) is skipped (exit 0) ---------------
# Mirrors backend/scripts/lints/tests/** — .rs files are lint fixtures, not
# cargo targets, so an un-mod-included subdir there must NOT be flagged.
echo "case: tests/ dir without a Cargo.toml sibling is skipped"
c5="$(mktemp -d)"; tmpdirs+=("$c5")
mkdir -p "$c5/tests/positive"
: > "$c5/tests/positive/fixture.rs"   # no Cargo.toml at $c5 -> not a crate
if "$CHECKER" "$c5" >/dev/null 2>&1; then
    pass "exit 0 — non-crate tests/ tree ignored"
else
    fail "expected exit 0 for a tests/ dir with no Cargo.toml sibling"
fi

# --- Case 6: an empty subdir (no .rs) is not flagged (exit 0) ----------------
echo "case: subdir with no .rs files is ignored"
c6="$(new_crate)"
: > "$c6/tests/x_tests.rs"
mkdir -p "$c6/tests/snapshots"
: > "$c6/tests/snapshots/out.snap"    # non-.rs artifact dir
if "$CHECKER" "$c6" >/dev/null 2>&1; then
    pass "exit 0 — .rs-free subdir ignored"
else
    fail "expected exit 0 for an .rs-free subdir"
fi

echo
if [[ "$failures" -gt 0 ]]; then
    echo "FAILED: $failures assertion(s) failed"
    exit 1
fi
echo "All orphan-test-guard self-tests passed."
exit 0
