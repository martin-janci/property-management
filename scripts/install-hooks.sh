#!/bin/bash
#
# Install git hooks for code quality
#
# This script installs the pre-commit hook that:
#   1. Checks Rust formatting (cargo fmt)
#   2. Checks Kotlin formatting (spotless)
#   3. Checks TypeScript/JS linting (Biome)
#   4. Checks TypeScript types (tsc --noEmit)
#   5. Validates screen-maps (frontmatter + sitemap + relations)
#
# Version bumping happens automatically via CI after merge to main.
#
# Usage:
#   ./scripts/install-hooks.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
HOOKS_DIR="$ROOT_DIR/.git/hooks"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${CYAN}Installing git hooks...${NC}"

# Check if .git directory exists
if [[ ! -d "$ROOT_DIR/.git" ]]; then
    echo -e "${RED}ERROR: .git directory not found. Are you in a git repository?${NC}"
    exit 1
fi

# Create hooks directory if it doesn't exist
mkdir -p "$HOOKS_DIR"

# Install pre-commit hook
if [[ -f "$HOOKS_DIR/pre-commit" ]]; then
    echo -e "${YELLOW}Backing up existing pre-commit hook...${NC}"
    mv "$HOOKS_DIR/pre-commit" "$HOOKS_DIR/pre-commit.backup.$(date +%Y%m%d%H%M%S)"
fi

cp "$SCRIPT_DIR/pre-commit" "$HOOKS_DIR/pre-commit"
chmod +x "$HOOKS_DIR/pre-commit"

echo -e "${GREEN}✓ Pre-commit hook installed${NC}"

# Register the `merge=ours` driver used by .gitattributes for backend/Cargo.lock.
# Without this, the per-file merge attribute is a no-op (git treats an unknown
# driver name as the default merge). `true` is the conventional one-liner
# driver: "the merge is already done — keep our version, exit 0".
git config merge.ours.driver true
echo -e "${GREEN}✓ Registered merge.ours.driver (Cargo.lock churn absorber)${NC}"
echo ""
echo -e "${CYAN}The following checks are now active:${NC}"
echo ""
echo "  Pre-commit hook runs:"
echo "    1. Rust formatting check        (cargo fmt --check)"
echo "    2. Kotlin formatting check      (spotless)"
echo "    3. TypeScript/JS lint           (Biome)"
echo "    4. TypeScript type check        (tsc --noEmit)"
echo "    5. Screen-map validation        (frontmatter + sitemap + relations)"
echo ""
echo -e "${CYAN}If a check fails, the hook will show:${NC}"
echo "    - What failed"
echo "    - Exact command to fix it"
echo "    - How to commit again"
echo ""
echo -e "${CYAN}Version bumping (patch auto on merge to main via CI):${NC}"
echo "    ./scripts/bump-version.sh minor   # 0.1.x -> 0.2.0"
echo "    ./scripts/bump-version.sh major   # 0.x.y -> 1.0.0"
echo ""
echo -e "${GREEN}Done!${NC}"
