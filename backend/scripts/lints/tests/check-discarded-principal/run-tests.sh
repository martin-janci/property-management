#!/usr/bin/env bash
# Self-tests for backend/scripts/lints/check-discarded-principal.sh.
# Issue #528 (2): a fixture-based harness so a future refactor of the
# regex / `nearest_fn_name` walk / keyword list cannot silently regress
# detection.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LINT_SCRIPT="$SCRIPT_DIR/../../check-discarded-principal.sh"
POSITIVE_DIR="$SCRIPT_DIR/positive"
NEGATIVE_DIR="$SCRIPT_DIR/negative"

if [[ ! -x "$LINT_SCRIPT" ]]; then
    chmod +x "$LINT_SCRIPT" 2>/dev/null || true
fi

fail=0

# Positive: every file under positive/ must produce at least one violation.
for fix in "$POSITIVE_DIR"/*.rs; do
    name=$(basename "$fix")
    tmpdir=$(mktemp -d)
    cp "$fix" "$tmpdir/$name"
    out=$(CHECK_DISCARDED_SCAN_DIRS="$tmpdir" "$LINT_SCRIPT" 2>&1 || true)
    if ! grep -qE '[0-9]+ discarded-principal violation' <<<"$out"; then
        echo "FAIL positive/$name — lint did not flag it"
        echo "$out"
        fail=1
    else
        echo "ok   positive/$name"
    fi
    rm -rf "$tmpdir"
done

# Negative: every file under negative/ must produce zero violations.
for fix in "$NEGATIVE_DIR"/*.rs; do
    name=$(basename "$fix")
    tmpdir=$(mktemp -d)
    cp "$fix" "$tmpdir/$name"
    out=$(CHECK_DISCARDED_SCAN_DIRS="$tmpdir" "$LINT_SCRIPT" 2>&1 || true)
    if grep -qE '[0-9]+ discarded-principal violation' <<<"$out"; then
        echo "FAIL negative/$name — lint flagged a legitimate case"
        echo "$out"
        fail=1
    else
        echo "ok   negative/$name"
    fi
    rm -rf "$tmpdir"
done

if [[ $fail -ne 0 ]]; then
    echo ""
    echo "check-discarded-principal self-tests FAILED"
    exit 1
fi
echo ""
echo "check-discarded-principal self-tests passed"
