#!/usr/bin/env bash
# build-android.sh - Reality Portal Android CI build
# Epic 85 - Story 85.2: Build Configuration by Environment
# Usage: ./scripts/build-android.sh [development|staging|production]
# Env vars: KEYSTORE_FILE, KEYSTORE_PASSWORD, KEY_ALIAS, KEY_PASSWORD
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
MOBILE_DIR="$ROOT_DIR/mobile-native"
BUILD_DIR="$ROOT_DIR/build/android"
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'; NC='\033[0m'
ENV="${1:-development}"
case "$ENV" in
    development|dev) FLAVOR=development; BUILD_TYPE=debug; TASK=assembleDevelopmentDebug; EXT=apk; LABEL=development ;;
    staging|stg)     FLAVOR=staging; BUILD_TYPE=release; TASK=assembleStagingRelease; EXT=apk; LABEL=staging ;;
    production|prod) FLAVOR=production; BUILD_TYPE=release; TASK=bundleProductionRelease; EXT=aab; LABEL=production ;;
    *) echo -e "${RED}ERROR: Unknown env '$ENV'${NC}"; exit 1 ;;
esac
OUT_DIR="$BUILD_DIR/$LABEL"; mkdir -p "$OUT_DIR"
echo -e "${BLUE}=== Android Build: $LABEL ===${NC}"; echo ""
[ "$BUILD_TYPE" = "release" ] && [ -z "${KEYSTORE_FILE:-}" ] && echo -e "${YELLOW}WARNING: KEYSTORE_FILE not set${NC}"
echo -e "${BLUE}[1/3] Spotless check...${NC}"; ( cd "$MOBILE_DIR" && ./gradlew spotlessCheck ); echo -e "${GREEN}  OK${NC}"
if [ "$LABEL" = "development" ]; then
    echo -e "${BLUE}[2/3] Unit tests...${NC}"; ( cd "$MOBILE_DIR" && ./gradlew :shared:allTests ); echo -e "${GREEN}  OK${NC}"
else echo -e "${YELLOW}[2/3] Tests skipped for $LABEL${NC}"; fi
echo -e "${BLUE}[3/3] Build: $TASK...${NC}"; ( cd "$MOBILE_DIR" && ./gradlew "$TASK" )
SEARCH="$MOBILE_DIR/androidApp/build/outputs"
[ "$EXT" = "aab" ] && FOUND=$(find "$SEARCH/bundle" -name "*.aab" 2>/dev/null | head -1) || FOUND=$(find "$SEARCH/apk" -name "*.apk" 2>/dev/null | grep -v unaligned | head -1)
[ -n "${FOUND:-}" ] && cp "$FOUND" "$OUT_DIR/RealityPortal-${LABEL}.${EXT}" && echo -e "${GREEN}  Artifact: $OUT_DIR/RealityPortal-${LABEL}.${EXT}${NC}" || echo -e "${YELLOW}  Artifact not found at $SEARCH${NC}"
echo ""; echo -e "${GREEN}=== Android $LABEL complete ===${NC}"
