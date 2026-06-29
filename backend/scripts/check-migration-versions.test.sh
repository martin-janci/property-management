#!/bin/bash
# Regression test for check-migration-versions.sh.
#
# The collision guard is pure shell wired into the required `check` CI job. With
# no test it could silently rot — a future edit to the `num="${base%%[!0-9]*}"`
# prefix parse or the `sed 's/^0*//' | sort -n | uniq -d` normalization could
# break detection, and nothing would fail until a real collision slipped through
# to `dev` (GH #1904). This test pins the guard's behavior by pointing it at
# temp fixture directories (the checker takes explicit dirs as arguments).
#
# Usage: ./scripts/check-migration-versions.test.sh
# Exit 0 if every case passes; exit 1 on the first failure.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHECKER="$SCRIPT_DIR/check-migration-versions.sh"

failures=0

fail() {
    echo "  ✗ $1"
    failures=$((failures + 1))
}

pass() {
    echo "  ✓ $1"
}

# --- Case 1: a same-version collision exits 1 and names both files ----------
echo "case: collision is detected"
tmp_collide="$(mktemp -d)"
trap 'rm -rf "$tmp_collide" "$tmp_clean" "$tmp_padded"' EXIT
: > "$tmp_collide/00192_a.sql"
: > "$tmp_collide/00192_b.sql"
out="$("$CHECKER" "$tmp_collide" 2>&1)"
code=$?
if [[ "$code" -ne 1 ]]; then
    fail "expected exit 1 on collision, got $code"
else
    pass "exit 1"
fi
if grep -q '00192_a.sql' <<< "$out" && grep -q '00192_b.sql' <<< "$out"; then
    pass "both colliding filenames reported"
else
    fail "expected both filenames in output; got: $out"
fi

# --- Case 2: a clean directory exits 0 --------------------------------------
echo "case: clean directory passes"
tmp_clean="$(mktemp -d)"
: > "$tmp_clean/00001_first.sql"
: > "$tmp_clean/00002_second.sql"
: > "$tmp_clean/00003_third.sql"
if "$CHECKER" "$tmp_clean" >/dev/null 2>&1; then
    pass "exit 0 on unique prefixes"
else
    fail "expected exit 0 on a clean directory"
fi

# --- Case 3: zero-pad normalization (0007 vs 7 are the same version) --------
echo "case: zero-padding is normalized"
tmp_padded="$(mktemp -d)"
: > "$tmp_padded/0007_padded.sql"
: > "$tmp_padded/7_bare.sql"
if "$CHECKER" "$tmp_padded" >/dev/null 2>&1; then
    fail "expected collision between 0007_* and 7_* (same numeric version)"
else
    pass "0007 and 7 treated as the same version"
fi

echo
if [[ "$failures" -gt 0 ]]; then
    echo "FAILED: $failures assertion(s) failed"
    exit 1
fi
echo "All migration-guard self-tests passed."
exit 0
