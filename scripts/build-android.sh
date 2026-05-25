#!/usr/bin/env bash
# build-android.sh
# Reality Portal Android – CI build script for dev / staging / prod environments.
# Epic 85 - Story 85.2: Build Configuration by Environment
#
# Usage:
#   ./scripts/build-android.sh [development|staging|production]
#
# Environment variables (set by CI or export before running):
#   KEYSTORE_FILE      — path to release.jks (production / staging signing)
#   KEYSTORE_PASSWORD  — keystore password
#   KEY_ALIAS          — key alias in the keystore
#   KEY_PASSWORD       — key password
#
# Output:
#   build/android/<env>/RealityPortal-<env>.apk   (debug/development)
#   build/android/<env>/RealityPortal-<env>.aab   (staging/production)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
MOBILE_DIR="$ROOT_DIR/mobile-native"
BUILD_DIR="$ROOT_DIR/build/android"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

ENV="${1:-development}"

case "$ENV" in
    development|dev)
        FLAVOR="development"
        BUILD_TYPE="debug"
        TASK="assembleDevelopmentDebug"
        ARTIFACT_EXT="apk"
        ENV_LABEL="development"
        ;;
    staging|stg)
        FLAVOR="staging"
        BUILD_TYPE="release"
        TASK="assembleStagingRelease"
        ARTIFACT_EXT="apk"
        ENV_LABEL="staging"
        ;;
    production|prod)
        FLAVOR="production"
        BUILD_TYPE="release"
        TASK="bundleProductionRelease"
        ARTIFACT_EXT="aab"
        ENV_LABEL="production"
        ;;
    *)
        echo -e "${RED}ERROR: Unknown environment '$ENV'${NC}"
        echo "Usage: $0 [development|staging|production]"
        exit 1
        ;;
esac

OUT_DIR="$BUILD_DIR/$ENV_LABEL"
mkdir -p "$OUT_DIR"

echo -e "${BLUE}=== Reality Portal Android Build ===${NC}"
echo -e "  Environment : ${YELLOW}${ENV_LABEL}${NC}"
echo -e "  Flavor      : ${FLAVOR}"
echo -e "  Build type  : ${BUILD_TYPE}"
echo -e "  Gradle task : ${TASK}"
echo -e "  Output dir  : ${OUT_DIR}"
echo ""

# Validate signing env vars for release builds
if [ "$BUILD_TYPE" = "release" ]; then
    if [ -z "${KEYSTORE_FILE:-}" ] || [ -z "${KEYSTORE_PASSWORD:-}" ]; then
        echo -e "${YELLOW}WARNING: KEYSTORE_FILE or KEYSTORE_PASSWORD not set.${NC}"
        echo -e "${YELLOW}         Release build will use unsigned APK/AAB.${NC}"
    fi
fi

# --- Step 1: Run Spotless check ---
echo -e "${BLUE}[1/3] Running Spotless formatting check...${NC}"
(
    cd "$MOBILE_DIR"
    ./gradlew spotlessCheck
)
echo -e "${GREEN}  Spotless clean.${NC}"

# --- Step 2: Run tests (skip for staging/prod to keep CI fast) ---
if [ "$ENV_LABEL" = "development" ]; then
    echo -e "${BLUE}[2/3] Running unit tests...${NC}"
    (
        cd "$MOBILE_DIR"
        ./gradlew :shared:allTests
    )
    echo -e "${GREEN}  Tests passed.${NC}"
else
    echo -e "${YELLOW}[2/3] Skipping unit tests for ${ENV_LABEL} build (use development for test run).${NC}"
fi

# --- Step 3: Build ---
echo -e "${BLUE}[3/3] Building ${FLAVOR} ${BUILD_TYPE}...${NC}"
(
    cd "$MOBILE_DIR"
    ./gradlew "$TASK"
)

# Find and copy the artifact to our build dir
ARTIFACT_SEARCH="$MOBILE_DIR/androidApp/build/outputs"
if [ "$ARTIFACT_EXT" = "aab" ]; then
    FOUND_ARTIFACT=$(find "$ARTIFACT_SEARCH/bundle" -name "*.aab" | head -1)
else
    FOUND_ARTIFACT=$(find "$ARTIFACT_SEARCH/apk" -name "*.apk" | grep -v unaligned | head -1)
fi

if [ -n "$FOUND_ARTIFACT" ]; then
    DEST="$OUT_DIR/RealityPortal-${ENV_LABEL}.${ARTIFACT_EXT}"
    cp "$FOUND_ARTIFACT" "$DEST"
    echo -e "${GREEN}  Artifact: $DEST${NC}"
else
    echo -e "${YELLOW}  Artifact not found at expected path (build may have succeeded in-place).${NC}"
    echo -e "${YELLOW}  Check: $ARTIFACT_SEARCH${NC}"
fi

echo ""
echo -e "${GREEN}=== Android ${ENV_LABEL} build complete ===${NC}"
