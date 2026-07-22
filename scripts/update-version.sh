#!/bin/bash
#
# update-version.sh - Synchronize version across all projects
#
# Single source of truth: VERSION file
#
# Updates:
# - backend/Cargo.toml (workspace.package.version)
# - backend/Cargo.lock (first-party workspace member versions)
# - frontend/package.json
# - frontend/apps/*/package.json
# - frontend/packages/*/package.json
# - mobile-native/gradle.properties (versionName + versionCode)
# - docs/api/typespec/main.tsp (API service version)
#
# Usage: ./scripts/update-version.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
VERSION_FILE="$ROOT_DIR/VERSION"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

# Check VERSION file exists
if [[ ! -f "$VERSION_FILE" ]]; then
    echo -e "${RED}ERROR: VERSION file not found at $VERSION_FILE${NC}"
    exit 1
fi

# Read and validate version
VERSION=$(cat "$VERSION_FILE" | tr -d '[:space:]')

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo -e "${RED}ERROR: Invalid version format '$VERSION'. Expected X.Y.Z (semantic versioning)${NC}"
    exit 1
fi

# Parse version components
IFS='.' read -r MAJOR MINOR PATCH <<< "$VERSION"

# Validate version components for versionCode calculation
# Using formula: MAJOR * 1000000 + MINOR * 1000 + PATCH
# Max int32: 2147483647
# This allows: MAJOR 0-2147, MINOR 0-999, PATCH 0-999
if [[ $MAJOR -gt 2147 ]]; then
    echo -e "${RED}ERROR: MAJOR version $MAJOR exceeds maximum 2147 for Android versionCode${NC}"
    exit 1
fi
if [[ $MINOR -gt 999 ]]; then
    echo -e "${RED}ERROR: MINOR version $MINOR exceeds maximum 999 for Android versionCode${NC}"
    exit 1
fi
if [[ $PATCH -gt 999 ]]; then
    echo -e "${RED}ERROR: PATCH version $PATCH exceeds maximum 999 for Android versionCode${NC}"
    exit 1
fi

# Calculate Android versionCode: MAJOR * 1000000 + MINOR * 1000 + PATCH
# This gives each component proper space without overflow:
# - MAJOR: millions place (0-2147)
# - MINOR: thousands place (0-999)
# - PATCH: ones place (0-999)
# Example: 1.2.3 -> 1002003, 2.15.128 -> 2015128
VERSION_CODE=$((MAJOR * 1000000 + MINOR * 1000 + PATCH))

# Final safety check for int32 overflow
if [[ $VERSION_CODE -gt 2147483647 ]]; then
    echo -e "${RED}ERROR: Calculated versionCode $VERSION_CODE exceeds Android maximum 2147483647${NC}"
    exit 1
fi

echo -e "${GREEN}Version: $VERSION${NC}"
echo -e "${GREEN}Version Code: $VERSION_CODE${NC}"
echo ""

# Function to update package.json version
update_package_json() {
    local file="$1"
    if [[ -f "$file" ]]; then
        # Only update if file has a version field
        if grep -q '"version"' "$file"; then
            # Use temporary file to avoid sed -i portability issues
            local tmp_file="${file}.tmp"
            sed "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" "$file" > "$tmp_file"
            mv "$tmp_file" "$file"
            echo -e "  ${GREEN}✓${NC} Updated $file"
        fi
    fi
}

# ==================== Root package.json ====================
echo "Updating root package.json..."
update_package_json "$ROOT_DIR/package.json"

# ==================== Backend (Rust) ====================
echo "Updating backend..."
CARGO_TOML="$ROOT_DIR/backend/Cargo.toml"
if [[ -f "$CARGO_TOML" ]]; then
    # Update workspace.package.version in Cargo.toml
    sed "s/^version = \"[^\"]*\"/version = \"$VERSION\"/" "$CARGO_TOML" > "$CARGO_TOML.tmp"
    mv "$CARGO_TOML.tmp" "$CARGO_TOML"
    echo -e "  ${GREEN}✓${NC} Updated $CARGO_TOML"
fi

# Sync first-party workspace member versions in Cargo.lock so the lock stays
# consistent with Cargo.toml (otherwise `cargo build --locked` / CI fails on a
# dirty tree). Only the [[package]] entries whose `name` matches a workspace
# member are touched — third-party crates are left alone.
CARGO_LOCK="$ROOT_DIR/backend/Cargo.lock"
if [[ -f "$CARGO_LOCK" && -f "$CARGO_TOML" ]]; then
    # Derive crate names from the workspace member paths (basename of each member).
    MEMBERS=$(sed -n '/^\[workspace\]/,/^\]/p' "$CARGO_TOML" \
        | grep -oE '"[^"]+"' | tr -d '"' | xargs -n1 basename 2>/dev/null)
    if [[ -n "$MEMBERS" ]]; then
        MEMBERS="$MEMBERS" VERSION="$VERSION" awk '
            BEGIN { split(ENVIRON["MEMBERS"], a, /[[:space:]]+/); for (i in a) member[a[i]] = 1 }
            /^name = "/ { cur = $0; gsub(/^name = "|"$/, "", cur); is_member = (cur in member) }
            is_member && /^version = "/ { print "version = \"" ENVIRON["VERSION"] "\""; next }
            { print }
        ' "$CARGO_LOCK" > "$CARGO_LOCK.tmp"
        mv "$CARGO_LOCK.tmp" "$CARGO_LOCK"
        echo -e "  ${GREEN}✓${NC} Updated $CARGO_LOCK (workspace members)"
    fi
fi

# deploy-server is excluded from the workspace (sqlite feature isolation —
# see .research/build-experience-report-2026-07-22.md) so it carries an
# explicit version + its own Cargo.lock; keep both in sync with VERSION.
DEPLOY_TOML="$ROOT_DIR/backend/servers/deploy-server/Cargo.toml"
if [[ -f "$DEPLOY_TOML" ]]; then
    sed "0,/^version = \"[^\"]*\"/s//version = \"$VERSION\"/" "$DEPLOY_TOML" > "$DEPLOY_TOML.tmp"
    mv "$DEPLOY_TOML.tmp" "$DEPLOY_TOML"
    echo -e "  ${GREEN}✓${NC} Updated $DEPLOY_TOML"
fi
DEPLOY_LOCK="$ROOT_DIR/backend/servers/deploy-server/Cargo.lock"
if [[ -f "$DEPLOY_LOCK" ]]; then
    MEMBERS="deploy-server" VERSION="$VERSION" awk '
        BEGIN { split(ENVIRON["MEMBERS"], a, /[[:space:]]+/); for (i in a) member[a[i]] = 1 }
        /^name = "/ { cur = $0; gsub(/^name = "|"$/, "", cur); is_member = (cur in member) }
        is_member && /^version = "/ { print "version = \"" ENVIRON["VERSION"] "\""; next }
        { print }
    ' "$DEPLOY_LOCK" > "$DEPLOY_LOCK.tmp"
    mv "$DEPLOY_LOCK.tmp" "$DEPLOY_LOCK"
    echo -e "  ${GREEN}✓${NC} Updated $DEPLOY_LOCK (deploy-server)"
fi

# ==================== Frontend (TypeScript) ====================
echo "Updating frontend packages..."
update_package_json "$ROOT_DIR/frontend/package.json"

for app_dir in "$ROOT_DIR/frontend/apps"/*; do
    if [[ -d "$app_dir" ]]; then
        update_package_json "$app_dir/package.json"
    fi
done

for pkg_dir in "$ROOT_DIR/frontend/packages"/*; do
    if [[ -d "$pkg_dir" ]]; then
        update_package_json "$pkg_dir/package.json"
    fi
done

# ==================== Mobile Native (Kotlin) ====================
echo "Updating mobile-native..."
GRADLE_PROPS="$ROOT_DIR/mobile-native/gradle.properties"
if [[ -f "$GRADLE_PROPS" ]]; then
    # Remove existing version properties AND the comment line, then remove consecutive empty lines
    grep -v -E "^app\.version|^# App version" "$GRADLE_PROPS" | awk 'NF || !blank {print; blank=!NF}' > "$GRADLE_PROPS.tmp"

    # Remove trailing empty lines using awk (portable)
    awk '/^$/{blank++; next} {for(i=0;i<blank;i++){print ""}; blank=0; print}' "$GRADLE_PROPS.tmp" > "$GRADLE_PROPS"
    rm -f "$GRADLE_PROPS.tmp"

    # Append version properties with single blank line separator
    {
        echo ""
        echo "# App version (synced from VERSION file)"
        echo "app.versionName=$VERSION"
        echo "app.versionCode=$VERSION_CODE"
    } >> "$GRADLE_PROPS"

    echo -e "  ${GREEN}✓${NC} Updated $GRADLE_PROPS"
fi

# ==================== API Specs (TypeSpec) ====================
echo "Updating API specs..."
TYPESPEC_MAIN="$ROOT_DIR/docs/api/typespec/main.tsp"
if [[ -f "$TYPESPEC_MAIN" ]]; then
    # Update service version in TypeSpec (version: "X.Y.Z")
    sed "s/version: \"[^\"]*\"/version: \"$VERSION\"/" "$TYPESPEC_MAIN" > "$TYPESPEC_MAIN.tmp"
    mv "$TYPESPEC_MAIN.tmp" "$TYPESPEC_MAIN"
    echo -e "  ${GREEN}✓${NC} Updated $TYPESPEC_MAIN"
fi

echo ""
echo -e "${GREEN}Version synchronization complete!${NC}"
echo ""
echo "Summary:"
echo "  Version:      $VERSION"
echo "  Version Code: $VERSION_CODE"
echo "  Major:        $MAJOR"
echo "  Minor:        $MINOR"
echo "  Patch:        $PATCH"
echo ""
echo "Files updated:"
echo "  - backend/Cargo.toml (workspace version)"
echo "  - backend/Cargo.lock (workspace member versions)"
echo "  - frontend/**/package.json"
echo "  - mobile-native/gradle.properties"
echo "  - docs/api/typespec/main.tsp (API version)"
