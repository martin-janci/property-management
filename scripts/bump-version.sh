#!/bin/bash
#
# bump-version.sh - Bump version number
#
# Usage:
#   ./scripts/bump-version.sh patch   # 0.1.0 -> 0.1.1 (auto on merge to main via CI)
#   ./scripts/bump-version.sh minor   # 0.1.1 -> 0.2.0 (manual)
#   ./scripts/bump-version.sh major   # 0.2.0 -> 1.0.0 (manual)
#
# After bumping, automatically runs update-version.sh to sync all projects.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
VERSION_FILE="$ROOT_DIR/VERSION"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check argument
BUMP_TYPE="${1:-patch}"

if [[ ! "$BUMP_TYPE" =~ ^(major|minor|patch)$ ]]; then
    echo -e "${RED}ERROR: Invalid bump type '$BUMP_TYPE'${NC}"
    echo "Usage: $0 [major|minor|patch]"
    exit 1
fi

# Check VERSION file exists
if [[ ! -f "$VERSION_FILE" ]]; then
    echo -e "${RED}ERROR: VERSION file not found at $VERSION_FILE${NC}"
    exit 1
fi

# Read current version
CURRENT_VERSION=$(cat "$VERSION_FILE" | tr -d '[:space:]')

if [[ ! "$CURRENT_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo -e "${RED}ERROR: Invalid current version format '$CURRENT_VERSION'${NC}"
    exit 1
fi

# Parse version components
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"

# Bump version based on type
case "$BUMP_TYPE" in
    major)
        MAJOR=$((MAJOR + 1))
        MINOR=0
        PATCH=0
        ;;
    minor)
        MINOR=$((MINOR + 1))
        PATCH=0
        ;;
    patch)
        PATCH=$((PATCH + 1))
        ;;
esac

NEW_VERSION="$MAJOR.$MINOR.$PATCH"

# Validate new version won't overflow Android versionCode
# Formula: MAJOR * 1000000 + MINOR * 1000 + PATCH
# Max int32: 2147483647 -> MAJOR 0-2147, MINOR 0-999, PATCH 0-999
if [[ $MAJOR -gt 2147 ]]; then
    echo -e "${RED}ERROR: Cannot bump - MAJOR version $MAJOR would exceed maximum 2147${NC}"
    exit 1
fi
if [[ $MINOR -gt 999 ]]; then
    echo -e "${RED}ERROR: Cannot bump - MINOR version $MINOR would exceed maximum 999${NC}"
    exit 1
fi
if [[ $PATCH -gt 999 ]]; then
    echo -e "${RED}ERROR: Cannot bump - PATCH version $PATCH would exceed maximum 999${NC}"
    exit 1
fi

echo -e "${YELLOW}Bumping version: $CURRENT_VERSION -> $NEW_VERSION ($BUMP_TYPE)${NC}"

# Write new version
echo "$NEW_VERSION" > "$VERSION_FILE"

echo -e "${GREEN}✓ VERSION file updated${NC}"

# Run update-version.sh to sync all projects
echo ""
echo "Synchronizing version across all projects..."
"$SCRIPT_DIR/update-version.sh"
