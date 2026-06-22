#!/usr/bin/env bash
# build-mobile.sh — Property Management (React Native / Expo) per-environment build helper.
# Epic 85 - Story 85.2: Build Configuration by Environment (React Native side).
#
# This is the RN/Expo counterpart to the top-level scripts/build-*.sh, which
# target the *mobile-native* KMP Reality Portal app (xcodegen + Gradle flavors).
# The Property Management app under frontend/apps/mobile/ is a managed-workflow
# Expo app: it has no checked-in native ios/ or android/ project, so per-env
# builds go through EAS Build (profiles in eas.json) rather than xcodebuild /
# gradlew directly. APP_ENV (development|staging|production) is set by the EAS
# profile; this wrapper just selects the right profile + platform and forwards.
#
# Usage:
#   ./scripts/build-mobile.sh [development|staging|production] [android|ios|both] [extra eas args...]
#
# Examples:
#   ./scripts/build-mobile.sh staging both           # eas build staging, android + ios
#   ./scripts/build-mobile.sh production ios          # eas build production, ios only
#   ./scripts/build-mobile.sh staging android --local # local EAS build (no cloud)
#   ./scripts/build-mobile.sh development             # dev-client build, both platforms
#
# Environment / prerequisites:
#   - eas-cli installed globally (npm install -g eas-cli) — see CLAUDE.md (#715).
#   - eas login / EXPO_TOKEN in the environment for cloud builds.
#   - Production: API_BASE_URL / WS_BASE_URL must be provided as EAS project
#     secrets (the shipped .env.production carries a __SET_BY_CI__ sentinel that
#     app.config.ts rejects if it reaches the build unchanged — see eas.json).
#
# Exit codes:
#   0 — all requested builds succeeded
#   1 — a build failed, or a prerequisite (eas-cli) is missing
#   2 — bad usage (unknown environment / platform)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$(dirname "$SCRIPT_DIR")"

# Colors (disabled when not a TTY so CI logs stay clean)
if [ -t 1 ]; then
    RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'; BOLD='\033[1m'; NC='\033[0m'
else
    RED=''; GREEN=''; YELLOW=''; BLUE=''; BOLD=''; NC=''
fi

ENV_ARG="${1:-development}"
PLATFORM="${2:-both}"
shift 2 2>/dev/null || shift "$#" 2>/dev/null || true
EXTRA_ARGS=("$@")

# Normalise + validate the environment, map to the eas.json build profile.
case "$ENV_ARG" in
    development|dev) PROFILE="development" ;;
    staging|stg)     PROFILE="staging" ;;
    production|prod) PROFILE="production" ;;
    *)
        echo -e "${RED}ERROR: Unknown environment '$ENV_ARG'${NC}" >&2
        echo "Usage: $0 [development|staging|production] [android|ios|both] [extra eas args...]" >&2
        exit 2
        ;;
esac

case "$PLATFORM" in
    android|ios|both) ;;
    *)
        echo -e "${RED}ERROR: Unknown platform '$PLATFORM'${NC}" >&2
        echo "Usage: $0 [development|staging|production] [android|ios|both] [extra eas args...]" >&2
        exit 2
        ;;
esac

if ! command -v eas >/dev/null 2>&1; then
    echo -e "${RED}ERROR: eas-cli not found on PATH.${NC}" >&2
    echo "  Install it globally (it is intentionally NOT a devDependency — see CLAUDE.md / #715):" >&2
    echo "    npm install -g eas-cli && eas login" >&2
    exit 1
fi

echo -e "${BOLD}${BLUE}=== Property Management (RN/Expo) build ===${NC}"
echo -e "  Environment : ${YELLOW}${ENV_ARG}${NC} (eas profile: ${PROFILE})"
echo -e "  Platform    : ${YELLOW}${PLATFORM}${NC}"
[ "${#EXTRA_ARGS[@]}" -gt 0 ] && echo -e "  Extra args  : ${EXTRA_ARGS[*]}"
echo ""

run_eas() {
    local plat="$1"
    echo -e "${BLUE}>>> eas build --platform ${plat} --profile ${PROFILE}${NC}"
    ( cd "$APP_DIR" && eas build --platform "$plat" --profile "$PROFILE" "${EXTRA_ARGS[@]}" )
}

EXIT_CODE=0
ANDROID_STATUS="skipped"
IOS_STATUS="skipped"

if [[ "$PLATFORM" == "android" || "$PLATFORM" == "both" ]]; then
    if run_eas android; then ANDROID_STATUS="${GREEN}success${NC}"; else ANDROID_STATUS="${RED}FAILED${NC}"; EXIT_CODE=1; fi
fi

if [[ "$PLATFORM" == "ios" || "$PLATFORM" == "both" ]]; then
    if run_eas ios; then IOS_STATUS="${GREEN}success${NC}"; else IOS_STATUS="${RED}FAILED${NC}"; EXIT_CODE=1; fi
fi

echo ""
echo -e "${BOLD}=== Build summary ===${NC}"
echo -e "  Environment : ${YELLOW}${ENV_ARG}${NC}"
echo -e "  Android     : ${ANDROID_STATUS}"
echo -e "  iOS         : ${IOS_STATUS}"
echo ""

if [ "$EXIT_CODE" -eq 0 ]; then
    echo -e "${GREEN}All requested builds completed.${NC}"
else
    echo -e "${RED}One or more builds FAILED. Check output above.${NC}" >&2
fi

exit "$EXIT_CODE"
