#!/usr/bin/env bash
# build-mobile.sh - Reality Portal Mobile CI orchestrator
# Epic 85 - Story 85.2: Build Configuration by Environment
# Usage: ./scripts/build-mobile.sh [development|staging|production] [android|ios|both]
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'; BOLD='\033[1m'; NC='\033[0m'
ENV="${1:-development}"; PLATFORM="${2:-both}"
case "$ENV" in development|dev|staging|stg|production|prod) ;; *) echo -e "${RED}Unknown env${NC}"; exit 1 ;; esac
case "$PLATFORM" in android|ios|both) ;; *) echo -e "${RED}Unknown platform${NC}"; exit 1 ;; esac
echo -e "${BOLD}${BLUE}=== Reality Portal Mobile CI Build ===${NC}"
echo -e "  Environment : ${YELLOW}${ENV}${NC} | Platform : ${YELLOW}${PLATFORM}${NC}"; echo ""
ANDROID_STATUS="skipped"; IOS_STATUS="skipped"; EXIT_CODE=0
if [[ "$PLATFORM" == "android" || "$PLATFORM" == "both" ]]; then
    echo -e "${BLUE}>>> Android...${NC}"
    if "$SCRIPT_DIR/build-android.sh" "$ENV"; then ANDROID_STATUS="${GREEN}success${NC}"
    else ANDROID_STATUS="${RED}FAILED${NC}"; EXIT_CODE=1; fi
fi
if [[ "$PLATFORM" == "ios" || "$PLATFORM" == "both" ]]; then
    echo -e "${BLUE}>>> iOS...${NC}"
    if "$SCRIPT_DIR/build-ios.sh" "$ENV"; then IOS_STATUS="${GREEN}success${NC}"
    else IOS_STATUS="${RED}FAILED${NC}"; EXIT_CODE=1; fi
fi
echo ""; echo -e "${BOLD}=== Summary ===${NC}"
echo -e "  Android : ${ANDROID_STATUS}"; echo -e "  iOS     : ${IOS_STATUS}"; echo ""
[ $EXIT_CODE -eq 0 ] && echo -e "${GREEN}All builds OK.${NC}" || echo -e "${RED}Build(s) FAILED.${NC}"
exit $EXIT_CODE
