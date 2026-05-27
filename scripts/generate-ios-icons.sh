#!/usr/bin/env bash
# generate-ios-icons.sh - Generate iOS app icon sets with DEV/STG badges
# Epic 85 - Story 85.2: Build Configuration by Environment
# Requires: ImageMagick (brew install imagemagick)
# Usage: ./scripts/generate-ios-icons.sh <source-1024.png>
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
ASSETS="$ROOT_DIR/mobile-native/iosApp/iosApp/Resources/Assets.xcassets"
RED='\033[0;31m'; GREEN='\033[0;32m'; NC='\033[0m'
command -v convert &>/dev/null || { echo -e "${RED}ERROR: ImageMagick not found. brew install imagemagick${NC}"; exit 1; }
[ $# -lt 1 ] && { echo "Usage: $0 <source-1024.png>"; exit 1; }
[ ! -f "$1" ] && { echo -e "${RED}ERROR: Not found: $1${NC}"; exit 1; }
SRC="$1"; SIZE=1024; BH=200
overlay() {
    local in="$1" label="$2" color="$3" out="$4"
    local tmp; tmp="$(mktemp --suffix=.png)"
    convert -size "${SIZE}x${BH}" xc:"$color" -gravity Center -font DejaVu-Sans-Bold -pointsize 100 -fill white -draw "text 0,0 '$label'" "$tmp"
    convert "$in" -resize "${SIZE}x${SIZE}!" "$tmp" -gravity South -composite "$out"
    rm -f "$tmp"
}
echo -e "${GREEN}Generating icon sets...${NC}"
convert "$SRC" -resize "${SIZE}x${SIZE}!" "$ASSETS/AppIcon.appiconset/Icon-1024.png" && echo -e "${GREEN}  [prod] OK${NC}"
overlay "$SRC" "DEV" "#E53E3E" "$ASSETS/AppIcon-Dev.appiconset/Icon-Dev-1024.png" && echo -e "${GREEN}  [dev] OK${NC}"
overlay "$SRC" "STG" "#D97706" "$ASSETS/AppIcon-Staging.appiconset/Icon-Staging-1024.png" && echo -e "${GREEN}  [stg] OK${NC}"
echo ""; echo -e "${GREEN}Done. Assign in Xcode per scheme: AppIcon-Dev / AppIcon-Staging / AppIcon${NC}"
