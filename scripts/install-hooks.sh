#!/bin/bash
#
# Install git hooks for code quality
#
# This script installs the pre-commit and pre-push hooks.
#
# Usage:
#   ./scripts/install-hooks.sh
#

set -e

# Hardcode paths to avoid command substitution
ROOT_DIR=".."
HOOKS_DIR="$ROOT_DIR/.git/hooks"
SCRIPT_DIR="."

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
    mv "$HOOKS_DIR/pre-commit" "$HOOKS_DIR/pre-commit.backup"
fi
cp "$SCRIPT_DIR/pre-commit" "$HOOKS_DIR/pre-commit"
chmod +x "$HOOKS_DIR/pre-commit"
echo -e "${GREEN}✓ Pre-commit hook installed${NC}"

# Install pre-push hook
if [[ -f "$HOOKS_DIR/pre-push" ]]; then
    echo -e "${YELLOW}Backing up existing pre-push hook...${NC}"
    mv "$HOOKS_DIR/pre-push" "$HOOKS_DIR/pre-push.backup"
fi
cp "$SCRIPT_DIR/pre-push" "$HOOKS_DIR/pre-push"
chmod +x "$HOOKS_DIR/pre-push"
echo -e "${GREEN}✓ Pre-push hook installed${NC}"

# Register the merge driver
git config merge.ours.driver true
echo -e "${GREEN}✓ Registered merge.ours.driver (Cargo.lock churn absorber)${NC}"

echo -e "${GREEN}Done!${NC}"
