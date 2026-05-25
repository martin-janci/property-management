#!/usr/bin/env bash
# build-mobile.sh
# Reality Portal – Mobile CI orchestrator (Android + iOS).
# Epic 85 - Story 85.2: Build Configuration by Environment
#
# Usage:
#   ./scripts/build-mobile.sh [development|staging|production] [android|ios|both]
#
# Examples:
#   ./scripts/build-mobile.sh development both        # dev build, both platforms
#   ./scripts/build-mobile.sh staging android         # staging Android only
#   ./scripts/build-mobile.sh production ios          # prod iOS only
#   ./scripts/build-mobile.sh staging                 # defaults to "both"
#
# Delegates to:
#   scripts/build-android.sh
#   scripts/build-ios.sh
#
# Exit codes:
#   0 — all requested builds succeeded
#   1 — one or more builds failed (details printed inline)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

ENV="${1:-development}"
PLATFORM="${2:-both}"

# Validate environment
case "$ENV" in
    development|dev|staging|stg|production|prod) ;;
    *)
        echo -e "${RED}ERROR: Unknown environment '$ENV'${NC}"
        echo "Usage: $0 [development|staging|production] [android|ios|both]"
        exit 1
        ;;
esac

# Validate platform
case "$PLATFORM" in
    android|ios|both) ;;
    *)
        echo -e "${RED}ERROR: Unknown platform '$PLATFORM'${NC}"
        echo "Usage: $0 [development|staging|production] [android|ios|both]"
        exit 1
        ;;
esac

echo -e "${BOLD}${BLUE}=== Reality Portal Mobile CI Build ===${NC}"
echo -e "  Environment : ${YELLOW}${ENV}${NC}"
echo -e "  Platform    : ${YELLOW}${PLATFORM}${NC}"
echo ""

ANDROID_STATUS="skipped"
IOS_STATUS="skipped"
EXIT_CODE=0

# --- Android ---
if [[ "$PLATFORM" == "android" || "$PLATFORM" == "both" ]]; then
    echo -e "${BLUE}>>> Starting Android build...${NC}"
    if "$SCRIPT_DIR/build-android.sh" "$ENV"; then
        ANDROID_STATUS="${GREEN}success${NC}"
    else
        ANDROID_STATUS="${RED}FAILED${NC}"
        EXIT_CODE=1
    fi
fi

# --- iOS ---
if [[ "$PLATFORM" == "ios" || "$PLATFORM" == "both" ]]; then
    echo -e "${BLUE}>>> Starting iOS build...${NC}"
    if "$SCRIPT_DIR/build-ios.sh" "$ENV"; then
        IOS_STATUS="${GREEN}success${NC}"
    else
        IOS_STATUS="${RED}FAILED${NC}"
        EXIT_CODE=1
    fi
fi

# --- Summary ---
echo ""
echo -e "${BOLD}=== Build Summary ===${NC}"
echo -e "  Environment : ${YELLOW}${ENV}${NC}"
echo -e "  Android     : ${ANDROID_STATUS}"
echo -e "  iOS         : ${IOS_STATUS}"
echo ""

if [ $EXIT_CODE -eq 0 ]; then
    echo -e "${GREEN}All builds completed successfully.${NC}"
else
    echo -e "${RED}One or more builds FAILED. Check output above.${NC}"
fi

exit $EXIT_CODE
