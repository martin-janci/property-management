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

# Patterns that indicate direct pool access (should use RlsConnection instead)
# Note: Current architecture uses repository pattern (state.*_repo), which is fine.
# The risk is repositories using self.pool without RLS context.
VIOLATION_PATTERNS=(
    'state\.db\.acquire'
    'state\.db\.pool'
    '&state\.db[^_]'
    'state\.db\)'
)

# Directories to check (handler code that should use RLS)
CHECK_DIRS=(
    "servers/api-server/src/handlers"
    "servers/api-server/src/routes"
    "servers/api-server/src/services"
    "servers/reality-server/src/handlers"
    "servers/reality-server/src/routes"
)

VIOLATIONS=0
WARNINGS=0

for dir in "${CHECK_DIRS[@]}"; do
    FULL_DIR="$BACKEND_DIR/$dir"
    if [[ ! -d "$FULL_DIR" ]]; then
        continue
    fi

    for pattern in "${VIOLATION_PATTERNS[@]}"; do
        # Use ripgrep for fast search, exclude health.rs, mod.rs, test files
        while IFS= read -r match; do
            if [[ -n "$match" ]]; then
                FILE=$(echo "$match" | cut -d: -f1)
                LINE=$(echo "$match" | cut -d: -f2)
                CONTENT=$(echo "$match" | cut -d: -f3-)

                # Check if this is in a legitimate context (e.g., inside RlsConnection implementation)
                if grep -q "impl.*RlsConnection\|impl.*RlsPool\|pub async fn acquire_public" "$FILE" 2>/dev/null; then
                    continue
                fi

                ((VIOLATIONS+=1))
                echo -e "${RED}VIOLATION${NC} [$FILE:$LINE]"
                echo "  $CONTENT"
                echo "  → Use RlsConnection extractor or RlsPool::acquire_with_rls() instead"
                echo ""
            fi
        done < <(rg -n "$pattern" "$FULL_DIR" \
            --glob='!*health*.rs' \
            --glob='!*mod.rs' \
            --glob='!*_test.rs' \
            --glob='!*tests*' \
            2>/dev/null || true)
    done
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
