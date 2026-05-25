#!/usr/bin/env bash
# generate-ios-icons.sh
# Reality Portal iOS – Generate app icon sets with environment badges.
# Epic 85 - Story 85.2: Build Configuration by Environment
#
# Requirements: ImageMagick (brew install imagemagick)
#
# Usage:
#   ./scripts/generate-ios-icons.sh <source-icon-1024.png>
#
# Outputs (in mobile-native/iosApp/iosApp/Resources/Assets.xcassets/):
#   AppIcon.appiconset/Icon-1024.png         — Production (no badge)
#   AppIcon-Dev.appiconset/Icon-Dev-1024.png — Development  (DEV banner)
#   AppIcon-Staging.appiconset/Icon-Staging-1024.png — Staging (STG banner)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
ASSETS_DIR="$ROOT_DIR/mobile-native/iosApp/iosApp/Resources/Assets.xcassets"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

if ! command -v convert &>/dev/null; then
    echo -e "${RED}ERROR: ImageMagick not found. Install with: brew install imagemagick${NC}"
    exit 1
fi

if [ $# -lt 1 ]; then
    echo -e "${YELLOW}Usage: $0 <source-icon-1024.png>${NC}"
    echo ""
    echo "  Generates production, dev (DEV badge), and staging (STG badge) icon sets."
    exit 1
fi

SOURCE_ICON="$1"

if [ ! -f "$SOURCE_ICON" ]; then
    echo -e "${RED}ERROR: Source icon not found: $SOURCE_ICON${NC}"
    exit 1
fi

ICON_SIZE=1024
BANNER_HEIGHT=200
DEV_BADGE_COLOR="#E53E3E"    # red for DEV
STG_BADGE_COLOR="#D97706"    # amber for STG
TEXT_COLOR="white"
FONT_SIZE=100

echo -e "${GREEN}Generating iOS app icons from: $SOURCE_ICON${NC}"

# --- 1. Production icon (no badge) ---
echo "  [prod] Copying clean icon..."
convert "$SOURCE_ICON" \
    -resize "${ICON_SIZE}x${ICON_SIZE}!" \
    "$ASSETS_DIR/AppIcon.appiconset/Icon-1024.png"
echo -e "${GREEN}  [prod] Done: AppIcon.appiconset/Icon-1024.png${NC}"

# --- Helper: overlay a banner on the bottom of the icon ---
# Args: <input> <label> <banner-color> <output>
overlay_banner() {
    local input="$1"
    local label="$2"
    local bg_color="$3"
    local output="$4"

    # Create a banner image
    local banner_tmp
    banner_tmp="$(mktemp --suffix=.png)"

    convert \
        -size "${ICON_SIZE}x${BANNER_HEIGHT}" \
        xc:"${bg_color}" \
        -gravity Center \
        -font DejaVu-Sans-Bold \
        -pointsize "${FONT_SIZE}" \
        -fill "${TEXT_COLOR}" \
        -draw "text 0,0 '${label}'" \
        "${banner_tmp}"

    # Composite banner onto bottom of icon
    convert "$input" \
        -resize "${ICON_SIZE}x${ICON_SIZE}!" \
        "${banner_tmp}" \
        -gravity South \
        -composite \
        "$output"

    rm -f "${banner_tmp}"
}

# --- 2. Development icon (DEV banner) ---
echo "  [dev] Overlaying DEV badge..."
overlay_banner \
    "$SOURCE_ICON" \
    "DEV" \
    "$DEV_BADGE_COLOR" \
    "$ASSETS_DIR/AppIcon-Dev.appiconset/Icon-Dev-1024.png"
echo -e "${GREEN}  [dev] Done: AppIcon-Dev.appiconset/Icon-Dev-1024.png${NC}"

# --- 3. Staging icon (STG banner) ---
echo "  [stg] Overlaying STG badge..."
overlay_banner \
    "$SOURCE_ICON" \
    "STG" \
    "$STG_BADGE_COLOR" \
    "$ASSETS_DIR/AppIcon-Staging.appiconset/Icon-Staging-1024.png"
echo -e "${GREEN}  [stg] Done: AppIcon-Staging.appiconset/Icon-Staging-1024.png${NC}"

echo ""
echo -e "${GREEN}All icon sets generated successfully.${NC}"
echo ""
echo "Next steps:"
echo "  1. Open iosApp.xcodeproj in Xcode."
echo "  2. In the target General settings, set the App Icon source per scheme:"
echo "     - RealityPortal-Dev:     AppIcon-Dev"
echo "     - RealityPortal-Staging: AppIcon-Staging"
echo "     - RealityPortal-Prod:    AppIcon"
