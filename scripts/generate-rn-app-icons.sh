#!/usr/bin/env bash
# generate-rn-app-icons.sh
#
# Epic 85 — Coverage 85-2: Generate all required app icon sizes for the React
# Native Property Management mobile app (frontend/apps/mobile) from a single
# source PNG or SVG.
#
# Produces:
#   Expo / React Native managed-workflow assets (1024x1024 PNG):
#     assets/images/icon.png                      Launcher icon — production
#     assets/images/icon-dev.png                  Launcher icon — DEV badge (red)
#     assets/images/icon-staging.png              Launcher icon — STG badge (amber)
#     assets/images/adaptive-icon.png             Android adaptive foreground — production
#     assets/images/adaptive-icon-dev.png         Android adaptive foreground — DEV
#     assets/images/adaptive-icon-staging.png     Android adaptive foreground — STG
#     assets/images/notification-icon.png          96x96 monochrome notification icon
#
#   iOS AppIcon set (under assets/ios-icons/):
#     All sizes required by Apple for a Universal app submission:
#     20pt @1x/2x/3x, 29pt @1x/2x/3x, 40pt @1x/2x/3x,
#     60pt @2x/3x, 76pt @1x/2x, 83.5pt @2x, 1024pt @1x (App Store)
#     Plus iPad sizes: 20/29/40/76/83.5pt variants.
#
#   NOTE: The Expo managed workflow does NOT use an ios/AppIcon.appiconset
#   directory directly — Expo regenerates the Xcode project from app.config.ts
#   at `expo prebuild` time and derives the iOS icon from `app.config.ts:icon`
#   (the 1024x1024 launcher PNG). However, the AppIcon.appiconset + Contents.json
#   are included here for:
#     a) Reference / manual Xcode inspection after prebuild.
#     b) Bare-workflow forks or teams that check in the native ios/ directory.
#     c) CI artifact archiving.
#
# HOW EXPO MANAGED WORKFLOW USES ICONS
# -------------------------------------
# app.config.ts `icon` field  =>  1024x1024 PNG  =>  Expo resizes to every
# required iOS/Android density at `expo prebuild` / EAS Build time.
# The six 1024x1024 PNGs this script produces (icon*.png / adaptive-icon*.png)
# are the ONLY binary assets you need to drop into assets/images/ — Expo does
# the rest.  The AppIcon.appiconset output of this script is supplementary.
#
# PREREQUISITES
# -------------
# ImageMagick 7 (`magick`) or 6 (`convert`).
#   macOS:  brew install imagemagick
#   Debian: apt-get install imagemagick
#
# USAGE
# -----
#   ./scripts/generate-rn-app-icons.sh [source]
#
#   source  Path to a 1024x1024+ PNG or SVG.
#           Defaults to frontend/apps/mobile/assets/images/icon-source.svg
#           (the canonical source committed in the repo).
#
# EXAMPLES
#   ./scripts/generate-rn-app-icons.sh
#   ./scripts/generate-rn-app-icons.sh path/to/custom-icon.png
#
# NOTE ON BINARY ASSETS
# ---------------------
# This script generates binary PNG files. They CANNOT be pushed via
# mcp__github__push_files (UTF-8 only). A maintainer must:
#   1. Run this script locally.
#   2. `git add frontend/apps/mobile/assets/images/*.png`
#   3. `git add frontend/apps/mobile/assets/ios-icons/`
#   4. Commit and push normally.
# See the PR body for details.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
MOBILE_DIR="$ROOT_DIR/frontend/apps/mobile"
ASSETS_DIR="$MOBILE_DIR/assets/images"
# NOTE: ios/ is gitignored (Expo managed: prebuild generates it).
# AppIcon sets land in assets/ios-icons/ instead so they can be committed.
IOS_ICONS_DIR="$MOBILE_DIR/assets/ios-icons"
IOS_APPICONSET_DIR="$IOS_ICONS_DIR/AppIcon.appiconset"
IOS_APPICONSET_DEV_DIR="$IOS_ICONS_DIR/AppIcon-Dev.appiconset"
IOS_APPICONSET_STG_DIR="$IOS_ICONS_DIR/AppIcon-Staging.appiconset"

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BOLD='\033[1m'; NC='\033[0m'

# ---------------------------------------------------------------------------
# Dependency check — support both ImageMagick 7 (magick) and 6 (convert)
# ---------------------------------------------------------------------------
if command -v magick &>/dev/null; then
    CONVERT="magick"
elif command -v convert &>/dev/null; then
    CONVERT="convert"
else
    echo -e "${RED}ERROR: ImageMagick not found.${NC}" >&2
    echo "  macOS:  brew install imagemagick" >&2
    echo "  Debian: apt-get install imagemagick" >&2
    exit 1
fi

# Prefer DejaVu-Sans-Bold for badge text; fall back to ImageMagick built-in.
if "$CONVERT" -list font 2>/dev/null | grep -q "DejaVu-Sans-Bold"; then
    FONT_ARGS=(-font DejaVu-Sans-Bold)
else
    FONT_ARGS=()
fi

# ---------------------------------------------------------------------------
# Source resolution
# ---------------------------------------------------------------------------
DEFAULT_SOURCE="$ASSETS_DIR/icon-source.svg"
SRC="${1:-$DEFAULT_SOURCE}"

if [ ! -f "$SRC" ]; then
    echo -e "${RED}ERROR: Source file not found: $SRC${NC}" >&2
    echo "  Pass a 1024x1024+ PNG or SVG as the first argument, or place the" >&2
    echo "  canonical SVG at: $DEFAULT_SOURCE" >&2
    exit 1
fi

echo -e "${BOLD}PPT React Native app icon generator${NC}"
echo "  Source : $SRC"
echo "  Target : $ASSETS_DIR"
echo ""

# ---------------------------------------------------------------------------
# Helper: render source to a 1024x1024 PNG temp file
# ---------------------------------------------------------------------------
rasterise() {
    # $1 = destination PNG (caller manages lifetime)
    "$CONVERT" -background none "$SRC" -resize 1024x1024! "$1"
}

# ---------------------------------------------------------------------------
# Helper: composite badge band over a 1024x1024 PNG
# badge <in-1024.png> <label> <color-hex> <out-1024.png>
# ---------------------------------------------------------------------------
badge() {
    local in="$1" label="$2" color="$3" out="$4"
    local band_h=200 size=1024
    local tmp_band; tmp_band="$(mktemp --suffix=.png)"
    # Render the coloured ribbon with centred label
    "$CONVERT" -size "${size}x${band_h}" xc:"$color" \
        -gravity Center "${FONT_ARGS[@]}" -pointsize 110 \
        -fill white -draw "text 0,0 '$label'" "$tmp_band"
    # Composite ribbon on the bottom of the icon
    "$CONVERT" "$in" "$tmp_band" -gravity South -composite "$out"
    rm -f "$tmp_band"
}

# ---------------------------------------------------------------------------
# Step 1 — Rasterise source once into a temp 1024x1024 base PNG
# ---------------------------------------------------------------------------
BASE_TMP="$(mktemp --suffix=_base.png)"
echo -e "  ${YELLOW}Rasterising source...${NC}"
rasterise "$BASE_TMP"
echo -e "  ${GREEN}Done (1024x1024 base ready)${NC}"

# ---------------------------------------------------------------------------
# Step 2 — Badged variant temp files
# ---------------------------------------------------------------------------
DEV_TMP="$(mktemp --suffix=_dev.png)"
STG_TMP="$(mktemp --suffix=_stg.png)"
badge "$BASE_TMP" "DEV" "#E53E3E" "$DEV_TMP"
badge "$BASE_TMP" "STG" "#D97706" "$STG_TMP"

# ---------------------------------------------------------------------------
# Step 3 — Production launcher + adaptive icons (1024x1024)
# ---------------------------------------------------------------------------
echo ""
echo -e "  ${BOLD}[1/4] Production launcher icons (1024x1024)${NC}"
mkdir -p "$ASSETS_DIR"

cp "$BASE_TMP" "$ASSETS_DIR/icon.png"
echo -e "    ${GREEN}assets/images/icon.png${NC}"

cp "$BASE_TMP" "$ASSETS_DIR/adaptive-icon.png"
echo -e "    ${GREEN}assets/images/adaptive-icon.png${NC}"

# ---------------------------------------------------------------------------
# Step 4 — DEV-badged launcher + adaptive icons
# ---------------------------------------------------------------------------
echo ""
echo -e "  ${BOLD}[2/4] DEV-badged launcher icons${NC}"

cp "$DEV_TMP" "$ASSETS_DIR/icon-dev.png"
echo -e "    ${GREEN}assets/images/icon-dev.png${NC}"

cp "$DEV_TMP" "$ASSETS_DIR/adaptive-icon-dev.png"
echo -e "    ${GREEN}assets/images/adaptive-icon-dev.png${NC}"

# ---------------------------------------------------------------------------
# Step 5 — STG-badged launcher + adaptive icons
# ---------------------------------------------------------------------------
echo ""
echo -e "  ${BOLD}[3/4] STG-badged launcher icons${NC}"

cp "$STG_TMP" "$ASSETS_DIR/icon-staging.png"
echo -e "    ${GREEN}assets/images/icon-staging.png${NC}"

cp "$STG_TMP" "$ASSETS_DIR/adaptive-icon-staging.png"
echo -e "    ${GREEN}assets/images/adaptive-icon-staging.png${NC}"

# ---------------------------------------------------------------------------
# Step 6 — Notification icon (96x96, monochrome white on transparent bg)
# Android requires a white/transparent icon for the notification shade.
# See: https://developer.android.com/guide/topics/ui/notifiers/notifications#icon
# ---------------------------------------------------------------------------
echo ""
echo -e "  ${BOLD}[4/4] Notification icon (96x96 monochrome)${NC}"

# Desaturate + threshold to pure white, keep alpha channel
"$CONVERT" "$BASE_TMP" -resize 96x96! \
    -colorspace Gray -threshold 50% \
    -channel Alpha -fx 'p{0,0}.g > 0.5 ? 0 : 1' +channel \
    -fill white -colorize 100 \
    "$ASSETS_DIR/notification-icon.png"
echo -e "    ${GREEN}assets/images/notification-icon.png${NC}"

# ---------------------------------------------------------------------------
# Step 7 — iOS AppIcon.appiconset (all required sizes)
# Expo managed workflow generates these at prebuild from the 1024px icon,
# but the appiconsets are shipped here for bare-workflow and reference use.
# ---------------------------------------------------------------------------
echo ""
echo -e "  ${BOLD}[Supplementary] iOS AppIcon sets${NC}"

# Icon size spec: "ptSize scale" entries covering iPhone + iPad Universal.
# Apple requires every size listed in Contents.json to be present in the .ipa.
# Source: Apple Human Interface Guidelines + Xcode asset catalog spec.
declare -a IOS_SIZES=(
    # format: <pixel_size> <filename_stem>
    # iPhone notification
    "40  Icon-20@2x"
    "60  Icon-20@3x"
    # iPhone settings
    "58  Icon-29@2x"
    "87  Icon-29@3x"
    # iPhone spotlight
    "80  Icon-40@2x"
    "120 Icon-40@3x"
    # iPhone app
    "120 Icon-60@2x"
    "180 Icon-60@3x"
    # iPad notification
    "20  Icon-20@1x-ipad"
    "40  Icon-20@2x-ipad"
    # iPad settings
    "29  Icon-29@1x-ipad"
    "58  Icon-29@2x-ipad"
    # iPad spotlight
    "40  Icon-40@1x-ipad"
    "80  Icon-40@2x-ipad"
    # iPad app
    "76  Icon-76@1x-ipad"
    "152 Icon-76@2x-ipad"
    # iPad Pro app
    "167 Icon-83.5@2x-ipad"
    # App Store
    "1024 Icon-1024@1x"
)

emit_ios_iconset() {
    local dest_dir="$1" src_png="$2"
    mkdir -p "$dest_dir"
    local entry pixels stem
    for entry in "${IOS_SIZES[@]}"; do
        pixels="${entry%% *}"
        stem="${entry##* }"
        "$CONVERT" "$src_png" -resize "${pixels}x${pixels}!" "$dest_dir/${stem}.png"
        echo -e "    ${GREEN}$(basename "$dest_dir")/${stem}.png${NC} (${pixels}px)"
    done
}

emit_ios_iconset "$IOS_APPICONSET_DIR"     "$BASE_TMP"
emit_ios_iconset "$IOS_APPICONSET_DEV_DIR" "$DEV_TMP"
emit_ios_iconset "$IOS_APPICONSET_STG_DIR" "$STG_TMP"

# ---------------------------------------------------------------------------
# Cleanup temp files
# ---------------------------------------------------------------------------
rm -f "$BASE_TMP" "$DEV_TMP" "$STG_TMP"

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo ""
echo -e "${GREEN}${BOLD}All icons generated successfully.${NC}"
echo ""
echo -e "${YELLOW}Next steps:${NC}"
echo "  1. Review the generated icons visually (open assets/images/*.png)."
echo "  2. Stage and commit the binary PNGs:"
echo "       git add frontend/apps/mobile/assets/images/*.png"
echo "       git add frontend/apps/mobile/assets/ios-icons/"
echo "       git commit -m 'chore(mobile): add generated app icons for all sizes'"
echo "  3. The Expo managed workflow (EAS Build / expo prebuild) will use"
echo "     assets/images/icon.png (and the badged variants) automatically."
echo "     The assets/ios-icons/AppIcon*.appiconset PNGs are supplementary (bare-workflow/reference)."
echo ""
echo -e "  ${YELLOW}Tip:${NC} Re-run this script whenever the brand icon changes."
echo "  Source: $SRC"
