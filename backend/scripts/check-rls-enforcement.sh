#!/bin/bash
# RLS Enforcement Checker
# Detects direct database pool access in handlers that should use RlsConnection
#
# Usage: ./scripts/check-rls-enforcement.sh [--strict]
#   --strict: Fail on any direct pool access (default: warn only)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_DIR="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

STRICT_MODE=false
if [[ "${1:-}" == "--strict" ]]; then
    STRICT_MODE=true
fi

echo "🔍 Checking for direct database pool access in handlers..."
echo ""

# Patterns that indicate direct pool access (should use RlsConnection instead).
# Note: Current architecture uses repository pattern (state.*_repo), which is fine.
# The risk is request-handling code touching the raw pool without RLS context.
#
# Two server idioms are covered:
#   api-server    : `state.db` is the raw pool  -> state.db.acquire / &state.db / ...
#   reality-server: `self.db` is the raw pool on AppState, and handlers also reach
#                   it via `&state.db`. The ONLY sanctioned raw acquire is inside
#                   `acquire_public_conn()` (state.rs), which clears RLS context
#                   itself — see SANCTIONED_HELPER allow-list below.
VIOLATION_PATTERNS=(
    'state\.db\.acquire'
    'state\.db\.pool'
    '&state\.db[^_]'
    'state\.db\)'
    'self\.db\.acquire'
    'self\.db\.pool'
    '&self\.db[^_]'
)

# Directories to check (request-handling code that should use RLS).
# reality-server's raw pool also lives on AppState in state.rs, and its request
# guards live in extractors/ — both are scanned.
#
# routes/admin/ is intentionally excluded: platform-admin handlers operate
# across tenants and bypass per-tenant RLS by design (admin-level credentials
# + role gate + audit logging handle security there). See ADR for admin arch.
CHECK_DIRS=(
    "servers/api-server/src/handlers"
    "servers/api-server/src/routes"
    "servers/api-server/src/services"
    "servers/reality-server/src/handlers"
    "servers/reality-server/src/routes"
    "servers/reality-server/src/extractors"
)

# Individual files (not whole dirs) to scan for raw-pool access.
CHECK_FILES=(
    "servers/reality-server/src/state.rs"
)

# Name of the sanctioned raw-acquire helper on reality-server's AppState.
# A raw-pool match is allow-listed if it sits inside this fn's body OR on a
# line that simply calls it.
SANCTIONED_HELPER="acquire_public_conn"

VIOLATIONS=0
WARNINGS=0

# is_sanctioned <file> <line>
# Returns 0 (true) when the matched line is an allow-listed raw-pool access:
#   - the line itself calls the sanctioned helper (acquire_public_conn), or
#   - the line lives inside the sanctioned helper's own fn body, or
#   - the file is an RlsConnection/RlsPool implementation.
# This is line-context aware: it does NOT blanket-allow an entire file just
# because the sanctioned helper happens to be defined somewhere in it.
is_sanctioned() {
    local file="$1" line="$2"
    local content
    content=$(sed -n "${line}p" "$file" 2>/dev/null || true)

    # Line directly calls the sanctioned helper.
    if [[ "$content" == *"$SANCTIONED_HELPER"* ]]; then
        return 0
    fi

    # RLS plumbing implementations are allowed to touch the raw pool.
    if grep -qE 'impl .*RlsConnection|impl .*RlsPool' "$file" 2>/dev/null; then
        return 0
    fi

    # Inside the sanctioned helper's fn body: find the nearest preceding
    # `fn <name>` and check whether it is the sanctioned helper.
    local fn_decl
    fn_decl=$(sed -n "1,${line}p" "$file" 2>/dev/null \
        | grep -nE '^[[:space:]]*(pub )?(async )?fn [a-zA-Z_]' \
        | tail -1 || true)
    if [[ "$fn_decl" == *"fn ${SANCTIONED_HELPER}"* ]]; then
        return 0
    fi

    return 1
}

scan_target() {
    # scan_target <absolute-path-or-dir>
    local target="$1"
    for pattern in "${VIOLATION_PATTERNS[@]}"; do
        # ripgrep: fast search, skip health.rs / mod.rs / test files.
        while IFS= read -r match; do
            [[ -n "$match" ]] || continue
            local FILE LINE CONTENT
            FILE=$(echo "$match" | cut -d: -f1)
            LINE=$(echo "$match" | cut -d: -f2)
            CONTENT=$(echo "$match" | cut -d: -f3-)

            # Skip doc-comment / comment lines — these reference the idioms in
            # prose (e.g. state.rs documents why `&self.db` is flagged).
            local trimmed="${CONTENT#"${CONTENT%%[![:space:]]*}"}"
            if [[ "$trimmed" == //* || "$trimmed" == \** ]]; then
                continue
            fi

            if is_sanctioned "$FILE" "$LINE"; then
                continue
            fi

            ((VIOLATIONS+=1))
            echo -e "${RED}VIOLATION${NC} [$FILE:$LINE]"
            echo "  $CONTENT"
            echo "  → Use RlsConnection extractor or RlsPool::acquire_with_rls() instead"
            echo ""
        done < <(rg -n --with-filename "$pattern" "$target" \
            --glob='!*health*.rs' \
            --glob='!*mod.rs' \
            --glob='!*_test.rs' \
            --glob='!*tests*' \
            --glob='!**/admin/**' \
            --glob='!**/admin_*.rs' \
            2>/dev/null || true)
    done
}

for dir in "${CHECK_DIRS[@]}"; do
    FULL_DIR="$BACKEND_DIR/$dir"
    [[ -d "$FULL_DIR" ]] || continue
    scan_target "$FULL_DIR"
done

for f in "${CHECK_FILES[@]}"; do
    FULL_FILE="$BACKEND_DIR/$f"
    [[ -f "$FULL_FILE" ]] || continue
    scan_target "$FULL_FILE"
done

# Also check for repository methods that take raw pool instead of RlsConnection
echo "🔍 Checking repository patterns..."

REPO_DIR="$BACKEND_DIR/crates/db/src/repositories"
if [[ -d "$REPO_DIR" ]]; then
    # Look for methods that take &DbPool but don't have RLS context
    while IFS= read -r match; do
        if [[ -n "$match" ]]; then
            FILE=$(echo "$match" | cut -d: -f1)
            LINE=$(echo "$match" | cut -d: -f2)
            CONTENT=$(echo "$match" | cut -d: -f3-)

            ((WARNINGS+=1))
            echo -e "${YELLOW}WARNING${NC} [$FILE:$LINE]"
            echo "  $CONTENT"
            echo "  → Consider if this method needs RLS context injection"
            echo ""
        fi
    done < <(rg -n 'pub async fn.*\(&self.*pool.*DbPool' "$REPO_DIR" 2>/dev/null || true)
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

if [[ $VIOLATIONS -gt 0 ]]; then
    echo -e "${RED}✗ Found $VIOLATIONS RLS violation(s)${NC}"
    if [[ $WARNINGS -gt 0 ]]; then
        echo -e "${YELLOW}⚠ Found $WARNINGS warning(s) in repositories${NC}"
    fi
    echo ""
    echo "Handlers should use:"
    echo "  • RlsConnection extractor for request-scoped RLS"
    echo "  • RlsPool::acquire_with_rls() for explicit context"
    echo "  • RlsPool::acquire_public() for unauthenticated routes"
    echo ""
    if $STRICT_MODE; then
        echo -e "${RED}Failing in strict mode.${NC}"
        exit 1
    else
        echo -e "${YELLOW}Run with --strict to fail CI on violations.${NC}"
        exit 0
    fi
else
    echo -e "${GREEN}✓ No RLS violations found${NC}"
    if [[ $WARNINGS -gt 0 ]]; then
        echo -e "${YELLOW}⚠ Found $WARNINGS warning(s) in repositories (review recommended)${NC}"
    fi
    exit 0
fi
