#!/usr/bin/env bash
# build-ios.sh - Reality Portal iOS CI build
# Epic 85 - Story 85.2: Build Configuration by Environment
# Usage: ./scripts/build-ios.sh [development|staging|production]
# Env vars: DEVELOPMENT_TEAM, STAGING_PROVISIONING_PROFILE, PROD_PROVISIONING_PROFILE
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
IOS_DIR="$ROOT_DIR/mobile-native/iosApp"
BUILD_DIR="$ROOT_DIR/build/ios"
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'; NC='\033[0m'
ENV="${1:-development}"
case "$ENV" in
    development|dev) SCHEME=RealityPortal-Dev; CONFIG=Development; LABEL=development; ARCHIVE=false ;;
    staging|stg)     SCHEME=RealityPortal-Staging; CONFIG=Staging; LABEL=staging; ARCHIVE=true ;;
    production|prod) SCHEME=RealityPortal-Prod; CONFIG=Production; LABEL=production; ARCHIVE=true ;;
    *) echo -e "${RED}ERROR: Unknown env '$ENV'${NC}"; exit 1 ;;
esac
OUT_DIR="$BUILD_DIR/$LABEL"; mkdir -p "$OUT_DIR"
echo -e "${BLUE}=== iOS Build: $LABEL ===${NC}"; echo ""
echo -e "${BLUE}[1/3] KMP shared framework...${NC}"
( cd "$ROOT_DIR/mobile-native" && ./gradlew :shared:linkPodReleaseFrameworkIosArm64 )
echo -e "${GREEN}  KMP framework OK${NC}"
ARGS=(-project "$IOS_DIR/iosApp.xcodeproj" -scheme "$SCHEME" -configuration "$CONFIG" -destination "generic/platform=iOS" CODE_SIGN_STYLE=Automatic)
[ -n "${DEVELOPMENT_TEAM:-}" ] && ARGS+=( DEVELOPMENT_TEAM="$DEVELOPMENT_TEAM" )
echo -e "${BLUE}[2/3] xcodebuild $CONFIG...${NC}"
if [ "$ARCHIVE" = true ]; then
    xcodebuild archive "${ARGS[@]}" -archivePath "$OUT_DIR/iosApp.xcarchive" | xcpretty --color || true
    echo -e "${BLUE}[3/3] Export IPA...${NC}"
    METHOD=$([ "$LABEL" = production ] && echo app-store || echo ad-hoc)
    PLIST="$OUT_DIR/ExportOptions.plist"
    python3 -c "print(open('/dev/stdin').read())" << PLIST_EOF > "$PLIST"
<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd"><plist version="1.0"><dict><key>method</key><string>$METHOD</string><key>compileBitcode</key><false/></dict></plist>
PLIST_EOF
    xcodebuild -exportArchive -archivePath "$OUT_DIR/iosApp.xcarchive" -exportPath "$OUT_DIR" -exportOptionsPlist "$PLIST" | xcpretty --color || true
    ls "$OUT_DIR"/*.ipa 2>/dev/null && echo -e "${GREEN}  IPA ready${NC}" || echo -e "${YELLOW}  No IPA (check signing)${NC}"
else
    xcodebuild build "${ARGS[@]}" -derivedDataPath "$OUT_DIR/DerivedData" | xcpretty --color || true
    echo -e "${GREEN}  Build output: $OUT_DIR/DerivedData${NC}"
fi
echo ""; echo -e "${GREEN}=== iOS $LABEL complete ===${NC}"
