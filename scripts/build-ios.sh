#!/usr/bin/env bash
# build-ios.sh
# Reality Portal iOS – CI build script for dev / staging / prod environments.
# Epic 85 - Story 85.2: Build Configuration by Environment
#
# Usage:
#   ./scripts/build-ios.sh [development|staging|production]
#
# Environment variables (set by CI or export before running):
#   STAGING_PROVISIONING_PROFILE   — provisioning profile name for Staging
#   PROD_PROVISIONING_PROFILE      — provisioning profile name for Production
#   DEVELOPMENT_TEAM               — Apple team ID (e.g. "ABC1234DEF")
#   APPLE_CERTIFICATE_P12_BASE64   — base64-encoded .p12 (CI signing)
#   APPLE_CERTIFICATE_PASSWORD     — p12 password
#
# Output:
#   build/ios/<env>/RealityPortal-<env>.ipa  (staging + prod)
#   build/ios/<env>/RealityPortal-<env>.app  (development simulator)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
IOS_DIR="$ROOT_DIR/mobile-native/iosApp"
BUILD_DIR="$ROOT_DIR/build/ios"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

ENV="${1:-development}"

case "$ENV" in
    development|dev)
        SCHEME="RealityPortal-Dev"
        CONFIG="Development"
        ENV_LABEL="development"
        ARCHIVE=false
        ;;
    staging|stg)
        SCHEME="RealityPortal-Staging"
        CONFIG="Staging"
        ENV_LABEL="staging"
        ARCHIVE=true
        ;;
    production|prod)
        SCHEME="RealityPortal-Prod"
        CONFIG="Production"
        ENV_LABEL="production"
        ARCHIVE=true
        ;;
    *)
        echo -e "${RED}ERROR: Unknown environment '$ENV'${NC}"
        echo "Usage: $0 [development|staging|production]"
        exit 1
        ;;
esac

OUT_DIR="$BUILD_DIR/$ENV_LABEL"
ARCHIVE_PATH="$OUT_DIR/RealityPortal-${ENV_LABEL}.xcarchive"
IPA_PATH="$OUT_DIR/RealityPortal-${ENV_LABEL}.ipa"

echo -e "${BLUE}=== Reality Portal iOS Build ===${NC}"
echo -e "  Environment : ${YELLOW}${ENV_LABEL}${NC}"
echo -e "  Scheme      : ${SCHEME}"
echo -e "  Config      : ${CONFIG}"
echo -e "  Output dir  : ${OUT_DIR}"
echo ""

mkdir -p "$OUT_DIR"

# --- Step 1: Build KMP shared framework (required before xcodebuild) ---
echo -e "${BLUE}[1/3] Building KMP shared framework...${NC}"
(
    cd "$ROOT_DIR/mobile-native"
    ./gradlew :shared:linkPodReleaseFrameworkIosArm64
)
echo -e "${GREEN}  KMP framework built.${NC}"

# --- Step 2: xcodebuild ---
echo -e "${BLUE}[2/3] Running xcodebuild (${CONFIG})...${NC}"

XCODEBUILD_ARGS=(
    -project "$IOS_DIR/iosApp.xcodeproj"
    -scheme "$SCHEME"
    -configuration "$CONFIG"
    -destination "generic/platform=iOS"
    CODE_SIGN_STYLE=Automatic
)

# Add team ID if available
if [ -n "${DEVELOPMENT_TEAM:-}" ]; then
    XCODEBUILD_ARGS+=( DEVELOPMENT_TEAM="$DEVELOPMENT_TEAM" )
fi

if [ "$ARCHIVE" = "true" ]; then
    # Staging + Production: produce .xcarchive then export IPA
    xcodebuild archive \
        "${XCODEBUILD_ARGS[@]}" \
        -archivePath "$ARCHIVE_PATH" | xcpretty --color || true

    echo -e "${BLUE}[3/3] Exporting IPA...${NC}"
    EXPORT_OPTIONS_PLIST="$OUT_DIR/ExportOptions.plist"
    cat > "$EXPORT_OPTIONS_PLIST" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>method</key>
    <string>$([ "$ENV_LABEL" = "production" ] && echo "app-store" || echo "ad-hoc")</string>
    <key>teamID</key>
    <string>${DEVELOPMENT_TEAM:-YOURTEAMID}</string>
    <key>uploadSymbols</key>
    <true/>
    <key>compileBitcode</key>
    <false/>
</dict>
</plist>
PLIST

    xcodebuild -exportArchive \
        -archivePath "$ARCHIVE_PATH" \
        -exportPath "$OUT_DIR" \
        -exportOptionsPlist "$EXPORT_OPTIONS_PLIST" | xcpretty --color || true

    if ls "$OUT_DIR"/*.ipa 1>/dev/null 2>&1; then
        mv "$OUT_DIR"/*.ipa "$IPA_PATH" 2>/dev/null || true
        echo -e "${GREEN}  IPA: $IPA_PATH${NC}"
    else
        echo -e "${YELLOW}  No IPA found (may need manual signing or code-sign env vars).${NC}"
    fi
else
    # Development: build only (simulator or device)
    xcodebuild build \
        "${XCODEBUILD_ARGS[@]}" \
        -derivedDataPath "$OUT_DIR/DerivedData" | xcpretty --color || true
    echo -e "${GREEN}  Build output: $OUT_DIR/DerivedData${NC}"
fi

echo ""
echo -e "${GREEN}=== iOS ${ENV_LABEL} build complete ===${NC}"
echo ""
echo "Install scheme xcschemes into the project:"
echo "  cp $ROOT_DIR/mobile-native/iosApp/xcschemes/*.xcscheme \\"
echo "     $IOS_DIR/iosApp.xcodeproj/xcshareddata/xcschemes/"
