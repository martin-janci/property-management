#!/bin/bash
#
# update-version.sh - Synchronize version across all projects
#
# Single source of truth: the repo-root VERSION file.
#
# Modes
#   ./scripts/update-version.sh           propagate VERSION to every carrier (default)
#   ./scripts/update-version.sh --list    print the carrier inventory, one repo-relative
#                                         path per line, and exit
#   ./scripts/update-version.sh --check   verify every carrier already carries VERSION;
#                                         writes nothing, exits non-zero on drift
#
# `--list` is the ONE carrier inventory in this repo. It is consumed by
#   * .github/workflows/version-bump.yml  - `git add --pathspec-from-file=-`, so a
#     newly added carrier can never be silently dropped from the bump commit;
#   * .github/workflows/version-drift.yml - the required PR gate;
#   * `just check-version`                - the same gate, locally.
# Add a carrier to `carrier_paths()` below and all three pick it up. Do not re-type
# the list anywhere else.
#
# Carriers (see carrier_paths):
# - package.json (root)
# - backend/Cargo.toml (workspace.package.version)
# - backend/Cargo.lock (first-party workspace member versions)
# - backend/servers/deploy-server/Cargo.toml + Cargo.lock (out-of-workspace crate)
# - frontend/package.json, frontend/apps/*/package.json, frontend/packages/*/package.json
# - mobile-native/gradle.properties (versionName + versionCode)
# - docs/api/typespec/main.tsp (API service version, inside @info)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
VERSION_FILE="$ROOT_DIR/VERSION"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

MODE="update"
case "${1:-}" in
    "")        MODE="update" ;;
    --list)    MODE="list" ;;
    --check)   MODE="check" ;;
    -h|--help)
        sed -n '3,20p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
        exit 0
        ;;
    *)
        echo -e "${RED}ERROR: unknown argument '$1' (expected nothing, --list or --check)${NC}" >&2
        exit 2
        ;;
esac

# Packages that intentionally carry no version field. Listing a repo-relative path
# here makes "unversioned" a recorded decision instead of an accident: --list omits
# it and --check ignores it. Keep this empty unless there is a stated reason.
UNVERSIONED_ALLOWLIST=()

is_allowlisted() {
    local candidate="$1" entry
    for entry in ${UNVERSIONED_ALLOWLIST[@]+"${UNVERSIONED_ALLOWLIST[@]}"}; do
        [[ "$entry" == "$candidate" ]] && return 0
    done
    return 1
}

# The carrier inventory. Prints repo-relative paths, one per line, for every carrier
# that exists in this checkout.
carrier_paths() {
    local p
    for p in \
        package.json \
        backend/Cargo.toml \
        backend/Cargo.lock \
        backend/servers/deploy-server/Cargo.toml \
        backend/servers/deploy-server/Cargo.lock \
        frontend/package.json \
        mobile-native/gradle.properties \
        docs/api/typespec/main.tsp
    do
        [[ -f "$ROOT_DIR/$p" ]] || continue
        is_allowlisted "$p" && continue
        echo "$p"
    done
    local abs rel
    for abs in "$ROOT_DIR"/frontend/apps/*/package.json "$ROOT_DIR"/frontend/packages/*/package.json; do
        [[ -f "$abs" ]] || continue
        rel="${abs#"$ROOT_DIR"/}"
        is_allowlisted "$rel" && continue
        echo "$rel"
    done
}

if [[ "$MODE" == "list" ]]; then
    carrier_paths
    exit 0
fi

# Check VERSION file exists
if [[ ! -f "$VERSION_FILE" ]]; then
    echo -e "${RED}ERROR: VERSION file not found at $VERSION_FILE${NC}"
    exit 1
fi

# Read and validate version
VERSION=$(tr -d '[:space:]' < "$VERSION_FILE")

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo -e "${RED}ERROR: Invalid version format '$VERSION'. Expected X.Y.Z (semantic versioning)${NC}"
    exit 1
fi

# Parse version components
IFS='.' read -r MAJOR MINOR PATCH <<< "$VERSION"

# Validate version components for versionCode calculation
# Using formula: MAJOR * 1000000 + MINOR * 1000 + PATCH
# Max int32: 2147483647
# This allows: MAJOR 0-2147, MINOR 0-999, PATCH 0-999
if [[ $MAJOR -gt 2147 ]]; then
    echo -e "${RED}ERROR: MAJOR version $MAJOR exceeds maximum 2147 for Android versionCode${NC}"
    exit 1
fi
if [[ $MINOR -gt 999 ]]; then
    echo -e "${RED}ERROR: MINOR version $MINOR exceeds maximum 999 for Android versionCode${NC}"
    exit 1
fi
if [[ $PATCH -gt 999 ]]; then
    echo -e "${RED}ERROR: PATCH version $PATCH exceeds maximum 999 for Android versionCode${NC}"
    exit 1
fi

# Calculate Android versionCode: MAJOR * 1000000 + MINOR * 1000 + PATCH
# This gives each component proper space without overflow:
# - MAJOR: millions place (0-2147)
# - MINOR: thousands place (0-999)
# - PATCH: ones place (0-999)
# Example: 1.2.3 -> 1002003, 2.15.128 -> 2015128
VERSION_CODE=$((MAJOR * 1000000 + MINOR * 1000 + PATCH))

# Final safety check for int32 overflow
if [[ $VERSION_CODE -gt 2147483647 ]]; then
    echo -e "${RED}ERROR: Calculated versionCode $VERSION_CODE exceeds Android maximum 2147483647${NC}"
    exit 1
fi

# ---------------------------------------------------------------------------
# Carrier readers - used by --check and by the post-conditions of a real run.
# Each prints the version a carrier currently declares (empty if it declares none).
# ---------------------------------------------------------------------------

# First "version": "X" of a JSON file. package.json declares the package's own
# version first, which is the one every consumer reads.
read_package_json_version() {
    grep -m1 -oE '"version"[[:space:]]*:[[:space:]]*"[^"]*"' "$1" 2>/dev/null \
        | sed -E 's/.*"([^"]*)"$/\1/'
}

# First `version = "X"` at column 0 of a Cargo.toml.
read_cargo_toml_version() {
    grep -m1 -oE '^version[[:space:]]*=[[:space:]]*"[^"]*"' "$1" 2>/dev/null \
        | sed -E 's/.*"([^"]*)"$/\1/'
}

# Workspace member crate names, derived from backend/Cargo.toml's [workspace] block.
workspace_members() {
    local cargo_toml="$1"
    [[ -f "$cargo_toml" ]] || return 0
    sed -n '/^\[workspace\]/,/^\]/p' "$cargo_toml" \
        | grep -oE '"[^"]+"' | tr -d '"' | xargs -n1 basename 2>/dev/null
}

# Every distinct version the named [[package]] entries declare in a Cargo.lock,
# one per line. Drift shows up as a line that is not $VERSION.
read_cargo_lock_member_versions() {
    local lock="$1" members="$2"
    MEMBERS="$members" awk '
        BEGIN { split(ENVIRON["MEMBERS"], a, /[[:space:]]+/); for (i in a) if (a[i] != "") member[a[i]] = 1 }
        /^name = "/ { cur = $0; gsub(/^name = "|"$/, "", cur); is_member = (cur in member) }
        is_member && /^version = "/ { v = $0; gsub(/^version = "|"$/, "", v); print v }
    ' "$lock" 2>/dev/null | sort -u
}

read_gradle_property() {
    grep -m1 -oE "^$2=.*" "$1" 2>/dev/null | cut -d= -f2-
}

# `version: "X"` inside main.tsp's @info(...).
read_tsp_version() {
    grep -m1 -oE 'version:[[:space:]]*"[^"]*"' "$1" 2>/dev/null \
        | sed -E 's/.*"([^"]*)"$/\1/'
}

# Verify one carrier. Prints one line for it; returns 1 on drift.
check_carrier() {
    local rel="$1" abs="$ROOT_DIR/$1" observed="" drift=0
    case "$rel" in
        *package.json)
            observed="$(read_package_json_version "$abs")"
            if [[ -z "$observed" ]]; then
                echo -e "  ${RED}✗${NC} $rel - no \"version\" field (add one, or add the path to UNVERSIONED_ALLOWLIST in scripts/update-version.sh)"
                return 1
            fi
            ;;
        */Cargo.lock)
            local members="deploy-server" versions
            [[ "$rel" == "backend/Cargo.lock" ]] && members="$(workspace_members "$ROOT_DIR/backend/Cargo.toml")"
            versions="$(read_cargo_lock_member_versions "$abs" "$members")"
            if [[ -z "$versions" ]]; then
                echo -e "  ${RED}✗${NC} $rel - no first-party [[package]] entry found"
                return 1
            fi
            while IFS= read -r observed; do
                if [[ "$observed" != "$VERSION" ]]; then
                    echo -e "  ${RED}✗${NC} $rel - a first-party crate declares '$observed' (expected $VERSION)"
                    drift=1
                fi
            done <<< "$versions"
            [[ $drift -eq 0 ]] && echo -e "  ${GREEN}✓${NC} $rel"
            return $drift
            ;;
        */Cargo.toml)
            observed="$(read_cargo_toml_version "$abs")"
            ;;
        mobile-native/gradle.properties)
            local code
            observed="$(read_gradle_property "$abs" "app.versionName")"
            code="$(read_gradle_property "$abs" "app.versionCode")"
            if [[ "$code" != "$VERSION_CODE" ]]; then
                echo -e "  ${RED}✗${NC} $rel - app.versionCode is '${code:-<missing>}' (expected $VERSION_CODE)"
                drift=1
            fi
            ;;
        docs/api/typespec/main.tsp)
            observed="$(read_tsp_version "$abs")"
            if [[ -z "$observed" ]]; then
                echo -e "  ${RED}✗${NC} $rel - no version: \"X.Y.Z\" found (its @info(#{ ... }) block must declare one)"
                return 1
            fi
            ;;
        *)
            echo -e "  ${RED}✗${NC} $rel - no checker for this carrier type; teach check_carrier() about it"
            return 1
            ;;
    esac
    if [[ "$observed" != "$VERSION" ]]; then
        echo -e "  ${RED}✗${NC} $rel - declares '${observed:-<missing>}' (expected $VERSION)"
        drift=1
    elif [[ $drift -eq 0 ]]; then
        echo -e "  ${GREEN}✓${NC} $rel"
    fi
    return $drift
}

check_all_carriers() {
    local rel failures=0
    echo "Checking every carrier against VERSION ($VERSION, versionCode $VERSION_CODE)..."
    while IFS= read -r rel; do
        check_carrier "$rel" || failures=$((failures + 1))
    done < <(carrier_paths)
    if [[ $failures -ne 0 ]]; then
        echo ""
        echo -e "${RED}Version drift: $failures carrier(s) disagree with VERSION ($VERSION).${NC}" >&2
        echo "Fix with: ./scripts/update-version.sh   (then commit the result)" >&2
        return 1
    fi
    echo ""
    echo -e "${GREEN}All carriers agree with VERSION ($VERSION).${NC}"
    return 0
}

if [[ "$MODE" == "check" ]]; then
    check_all_carriers
    exit $?
fi

echo -e "${GREEN}Version: $VERSION${NC}"
echo -e "${GREEN}Version Code: $VERSION_CODE${NC}"
echo ""

# Function to update package.json version
update_package_json() {
    local file="$1"
    if [[ -f "$file" ]]; then
        # Only update if file has a version field
        if grep -q '"version"' "$file"; then
            # Use temporary file to avoid sed -i portability issues
            local tmp_file="${file}.tmp"
            sed "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" "$file" > "$tmp_file"
            mv "$tmp_file" "$file"
            echo -e "  ${GREEN}✓${NC} Updated $file"
        fi
    fi
}

# ==================== package.json carriers ====================
# Driven by the carrier inventory itself, so the inventory and the writes cannot
# diverge (the root, frontend/package.json, apps/* and packages/* all come from it).
echo "Updating package.json carriers..."
while IFS= read -r CARRIER; do
    case "$CARRIER" in
        *package.json) update_package_json "$ROOT_DIR/$CARRIER" ;;
    esac
done < <(carrier_paths)

# ==================== Backend (Rust) ====================
echo "Updating backend..."
CARGO_TOML="$ROOT_DIR/backend/Cargo.toml"
if [[ -f "$CARGO_TOML" ]]; then
    # Update workspace.package.version in Cargo.toml
    sed "s/^version = \"[^\"]*\"/version = \"$VERSION\"/" "$CARGO_TOML" > "$CARGO_TOML.tmp"
    mv "$CARGO_TOML.tmp" "$CARGO_TOML"
    echo -e "  ${GREEN}✓${NC} Updated $CARGO_TOML"
fi

# Sync first-party workspace member versions in Cargo.lock so the lock stays
# consistent with Cargo.toml (otherwise `cargo build --locked` / CI fails on a
# dirty tree). Only the [[package]] entries whose `name` matches a workspace
# member are touched - third-party crates are left alone.
CARGO_LOCK="$ROOT_DIR/backend/Cargo.lock"
if [[ -f "$CARGO_LOCK" && -f "$CARGO_TOML" ]]; then
    # Derive crate names from the workspace member paths (basename of each member).
    MEMBERS=$(workspace_members "$CARGO_TOML")
    if [[ -n "$MEMBERS" ]]; then
        MEMBERS="$MEMBERS" VERSION="$VERSION" awk '
            BEGIN { split(ENVIRON["MEMBERS"], a, /[[:space:]]+/); for (i in a) member[a[i]] = 1 }
            /^name = "/ { cur = $0; gsub(/^name = "|"$/, "", cur); is_member = (cur in member) }
            is_member && /^version = "/ { print "version = \"" ENVIRON["VERSION"] "\""; next }
            { print }
        ' "$CARGO_LOCK" > "$CARGO_LOCK.tmp"
        mv "$CARGO_LOCK.tmp" "$CARGO_LOCK"
        echo -e "  ${GREEN}✓${NC} Updated $CARGO_LOCK (workspace members)"
    fi
fi

# deploy-server is excluded from the workspace (sqlite feature isolation -
# see .research/build-experience-report-2026-07-22.md) so it carries an
# explicit version + its own Cargo.lock; keep both in sync with VERSION.
DEPLOY_TOML="$ROOT_DIR/backend/servers/deploy-server/Cargo.toml"
if [[ -f "$DEPLOY_TOML" ]]; then
    sed "0,/^version = \"[^\"]*\"/s//version = \"$VERSION\"/" "$DEPLOY_TOML" > "$DEPLOY_TOML.tmp"
    mv "$DEPLOY_TOML.tmp" "$DEPLOY_TOML"
    echo -e "  ${GREEN}✓${NC} Updated $DEPLOY_TOML"
fi
DEPLOY_LOCK="$ROOT_DIR/backend/servers/deploy-server/Cargo.lock"
if [[ -f "$DEPLOY_LOCK" ]]; then
    MEMBERS="deploy-server" VERSION="$VERSION" awk '
        BEGIN { split(ENVIRON["MEMBERS"], a, /[[:space:]]+/); for (i in a) member[a[i]] = 1 }
        /^name = "/ { cur = $0; gsub(/^name = "|"$/, "", cur); is_member = (cur in member) }
        is_member && /^version = "/ { print "version = \"" ENVIRON["VERSION"] "\""; next }
        { print }
    ' "$DEPLOY_LOCK" > "$DEPLOY_LOCK.tmp"
    mv "$DEPLOY_LOCK.tmp" "$DEPLOY_LOCK"
    echo -e "  ${GREEN}✓${NC} Updated $DEPLOY_LOCK (deploy-server)"
fi

# ==================== Mobile Native (Kotlin) ====================
echo "Updating mobile-native..."
GRADLE_PROPS="$ROOT_DIR/mobile-native/gradle.properties"
if [[ -f "$GRADLE_PROPS" ]]; then
    # Remove existing version properties AND the comment line, then remove consecutive empty lines
    grep -v -E "^app\.version|^# App version" "$GRADLE_PROPS" | awk 'NF || !blank {print; blank=!NF}' > "$GRADLE_PROPS.tmp"

    # Remove trailing empty lines using awk (portable)
    awk '/^$/{blank++; next} {for(i=0;i<blank;i++){print ""}; blank=0; print}' "$GRADLE_PROPS.tmp" > "$GRADLE_PROPS"
    rm -f "$GRADLE_PROPS.tmp"

    # Append version properties with single blank line separator
    {
        echo ""
        echo "# App version (synced from VERSION file)"
        echo "app.versionName=$VERSION"
        echo "app.versionCode=$VERSION_CODE"
    } >> "$GRADLE_PROPS"

    echo -e "  ${GREEN}✓${NC} Updated $GRADLE_PROPS"
fi

# ==================== API Specs (TypeSpec) ====================
echo "Updating API specs..."
TYPESPEC_MAIN="$ROOT_DIR/docs/api/typespec/main.tsp"
if [[ -f "$TYPESPEC_MAIN" ]]; then
    # Update service version in TypeSpec (version: "X.Y.Z")
    sed "s/version: \"[^\"]*\"/version: \"$VERSION\"/" "$TYPESPEC_MAIN" > "$TYPESPEC_MAIN.tmp"
    mv "$TYPESPEC_MAIN.tmp" "$TYPESPEC_MAIN"
    # Post-condition: the sed above is a silent no-op when @info(...) declares no
    # version at all - which is exactly how main.tsp shipped an empty @info block
    # while this script reported success. Fail instead of pretending.
    if [[ "$(read_tsp_version "$TYPESPEC_MAIN")" != "$VERSION" ]]; then
        echo -e "${RED}ERROR: $TYPESPEC_MAIN declares no version: \"X.Y.Z\" to update - add one to its @info(#{ ... }) block${NC}" >&2
        exit 1
    fi
    echo -e "  ${GREEN}✓${NC} Updated $TYPESPEC_MAIN"
fi

echo ""

# Post-condition for the whole run: every carrier the inventory names must now
# declare $VERSION. This is the same assertion `--check` and the drift gate make, so
# a path that is listed but not actually written fails here, at the source.
if ! check_all_carriers; then
    echo -e "${RED}ERROR: update-version.sh finished but some carriers still disagree (see above).${NC}" >&2
    exit 1
fi

echo ""
echo -e "${GREEN}Version synchronization complete!${NC}"
echo ""
echo "Summary:"
echo "  Version:      $VERSION"
echo "  Version Code: $VERSION_CODE"
echo "  Major:        $MAJOR"
echo "  Minor:        $MINOR"
echo "  Patch:        $PATCH"
echo ""
echo "Carriers written (./scripts/update-version.sh --list):"
carrier_paths | sed 's/^/  - /'
