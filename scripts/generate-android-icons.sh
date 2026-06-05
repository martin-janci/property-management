#!/usr/bin/env bash
# generate-android-icons.sh - Generate Android launcher icons with DEV/STG badges
# Epic 85 - Story 85.2: Build Configuration by Environment
#
# Mirrors scripts/generate-ios-icons.sh for the Android product flavors so that
# DEV / STG / PROD builds are visually distinguishable on-device (AC-4).
#
# Production icons land in the `main` source set; the development and staging
# flavors get badged variants in their flavor source sets, which AGP merges
# over `main` at build time (no manifest/Gradle change needed — the flavors are
# already declared in androidApp/build.gradle.kts).
#
# Requires: ImageMagick (`brew install imagemagick` / `apt-get install imagemagick`)
#
# Usage:
#   ./scripts/generate-android-icons.sh <source-1024.png>
#
# Output (per density mipmap dir, ic_launcher.png + ic_launcher_round.png):
#   mobile-native/androidApp/src/main/res/mipmap-*/         (production)
#   mobile-native/androidApp/src/development/res/mipmap-*/  (DEV badge, red)
#   mobile-native/androidApp/src/staging/res/mipmap-*/      (STG badge, amber)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
ANDROID_RES_BASE="$ROOT_DIR/mobile-native/androidApp/src"

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'

# ImageMagick 7 ships `magick`; older installs ship `convert`. Support both.
if command -v magick &>/dev/null; then
    CONVERT="magick"
elif command -v convert &>/dev/null; then
    CONVERT="convert"
else
    echo -e "${RED}ERROR: ImageMagick not found. brew install imagemagick (or apt-get install imagemagick)${NC}" >&2
    exit 1
fi

[ $# -lt 1 ] && { echo "Usage: $0 <source-1024.png>" >&2; exit 1; }
SRC="$1"
[ ! -f "$SRC" ] && { echo -e "${RED}ERROR: Not found: $SRC${NC}" >&2; exit 1; }

# Android launcher icon densities (px) — standard ldpi..xxxhdpi ladder.
# Round icons use the same pixel sizes.
DENSITIES=(
    "mdpi:48"
    "hdpi:72"
    "xhdpi:96"
    "xxhdpi:144"
    "xxxhdpi:192"
)

# write_icon <src-square-png> <dest-mipmap-parent-dir>
# Emits ic_launcher.png + ic_launcher_round.png at every density.
write_icon() {
    local square="$1" dest_base="$2"
    local entry density size dir
    for entry in "${DENSITIES[@]}"; do
        density="${entry%%:*}"
        size="${entry##*:}"
        dir="$dest_base/mipmap-${density}"
        mkdir -p "$dir"
        # Square launcher icon
        "$CONVERT" "$square" -resize "${size}x${size}!" "$dir/ic_launcher.png"
        # Round launcher icon (circular mask)
        "$CONVERT" "$square" -resize "${size}x${size}!" \
            \( +clone -alpha extract \
               -draw "fill black polygon 0,0 0,${size} ${size},0 fill white circle $((size/2)),$((size/2)) $((size/2)),0" \
               \( +clone -flip \) -compose Multiply -composite \
               \( +clone -flop \) -compose Multiply -composite \) \
            -alpha off -compose CopyOpacity -composite "$dir/ic_launcher_round.png"
    done
}

# badge <src-1024> <label> <color> <out-1024>
# Composites a labelled band across the bottom of the icon (same visual
# language as generate-ios-icons.sh).
badge() {
    local in="$1" label="$2" color="$3" out="$4"
    local size=1024 band=200 tmp font_args
    tmp="$(mktemp --suffix=.png)"
    # Prefer a bundled bold font; fall back to ImageMagick's default if absent.
    if "$CONVERT" -list font 2>/dev/null | grep -q "DejaVu-Sans-Bold"; then
        font_args=(-font DejaVu-Sans-Bold)
    else
        font_args=()
    fi
    "$CONVERT" -size "${size}x${band}" xc:"$color" -gravity Center \
        "${font_args[@]}" -pointsize 100 -fill white -draw "text 0,0 '$label'" "$tmp"
    "$CONVERT" "$in" -resize "${size}x${size}!" "$tmp" -gravity South -composite "$out"
    rm -f "$tmp"
}

echo -e "${GREEN}Generating Android launcher icons from ${SRC}...${NC}"

# --- Production (main source set, no badge) ---
write_icon "$SRC" "$ANDROID_RES_BASE/main/res"
echo -e "${GREEN}  [prod]  main/res/mipmap-* OK${NC}"

# --- Development (DEV badge, red) ---
DEV_SRC="$(mktemp --suffix=.png)"
badge "$SRC" "DEV" "#E53E3E" "$DEV_SRC"
write_icon "$DEV_SRC" "$ANDROID_RES_BASE/development/res"
rm -f "$DEV_SRC"
echo -e "${GREEN}  [dev]   development/res/mipmap-* OK${NC}"

# --- Staging (STG badge, amber) ---
STG_SRC="$(mktemp --suffix=.png)"
badge "$SRC" "STG" "#D97706" "$STG_SRC"
write_icon "$STG_SRC" "$ANDROID_RES_BASE/staging/res"
rm -f "$STG_SRC"
echo -e "${GREEN}  [stg]   staging/res/mipmap-* OK${NC}"

echo ""
echo -e "${GREEN}Done.${NC} Flavor icons are merged over main automatically by AGP."
echo -e "${YELLOW}Tip:${NC} ensure AndroidManifest.xml uses android:icon=\"@mipmap/ic_launcher\""
echo -e "      and android:roundIcon=\"@mipmap/ic_launcher_round\"."
