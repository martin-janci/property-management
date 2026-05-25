#!/bin/bash
# =============================================================================
# check-discarded-principal CI lint
# =============================================================================
# Flags mutating (non-GET) Axum handlers that bind `_principal: RequestPrincipal`
# — i.e. extract the principal for middleware/authn but immediately discard it
# without using it for authz or tenant-scoping in the handler body.
#
# This pattern was the root cause of the ai.rs IDOR cluster:
#   - security-voice-device-idor (fixed PR #461): unlink_voice_device bound
#     _principal but called deactivate_voice_device(id) with no owner check.
#   - security-equipment-idor: delete_equipment / update_equipment /
#     update_maintenance all bound _principal and ignored it.
#   - security-inquiry-read-idor (reality-server): mark_as_read same pattern.
#
# A handler that only reads data (GET equivalent) may legitimately discard the
# principal when the data is non-sensitive or publicly scoped — these are
# allowed via the ALLOWLIST below.  Any write (POST/PUT/PATCH/DELETE) that
# discards the principal is almost certainly an IDOR.
#
# Detection strategy
# ------------------
# We look for async fn declarations in route files where:
#   1. The function parameter list contains `_principal: RequestPrincipal`
#      (underscore-prefixed -> intentionally unused), AND
#   2. The function name suggests a mutating operation (prefix/suffix heuristic).
#
# "Mutating" heuristics (fn name contains any of):
#   create_, update_, delete_, patch_, add_, remove_, set_, clear_, reset_,
#   enable_, disable_, cancel_, archive_, restore_, deactivate_, activate_,
#   unlink_, link_, revoke_, approve_, reject_, publish_, unpublish_, mark_,
#   trigger_, send_, assign_, unassign_, upload_, process_, bulk_
#
# The grep approach avoids needing a full Rust parser: we scan each .rs file
# for lines matching `_principal: RequestPrincipal` and then check whether the
# nearest preceding `async fn` declaration in the file has a mutating name.
#
# Allowlist (file:fn pairs where the discard is intentional):
#   Empty -- no legitimate cases identified yet.  Add with a comment if one
#   arises (e.g. a write that is scoped entirely by a separate extractor).
#
# Usage: ./scripts/lints/check-discarded-principal.sh [--strict]
#   --strict: exit 1 on any violation (default: exit 0 with output for review)
#   Without --strict the lint is informational; add --strict to backend.yml
#   once the existing violations have been cleaned up.
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

STRICT_MODE=false
if [[ "${1:-}" == "--strict" ]]; then
    STRICT_MODE=true
fi

# ---------------------------------------------------------------------------
# Mutating fn-name prefixes/substrings (lower-case; matched against fn name).
# ---------------------------------------------------------------------------
MUTATING_KEYWORDS=(
    create_
    update_
    delete_
    patch_
    add_
    remove_
    set_
    clear_
    reset_
    enable_
    disable_
    cancel_
    archive_
    restore_
    deactivate_
    activate_
    unlink_
    link_
    revoke_
    approve_
    reject_
    publish_
    unpublish_
    mark_
    trigger_
    send_
    assign_
    unassign_
    upload_
    process_
    bulk_
)

# ---------------------------------------------------------------------------
# Allowlist: "<relative-file>:<fn-name>" pairs that are known-safe discards.
# ---------------------------------------------------------------------------
ALLOWLIST=(
    # Example (remove if adding real entries):
    # "servers/api-server/src/routes/health.rs:health_check"
)

# Directories to scan (route/handler code only).
SCAN_DIRS=(
    "servers/api-server/src/routes"
    "servers/reality-server/src/routes"
    "servers/api-server/src/handlers"
    "servers/reality-server/src/handlers"
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

is_mutating_fn() {
    local fn_name="${1,,}"   # lower-case
    for kw in "${MUTATING_KEYWORDS[@]}"; do
        if [[ "$fn_name" == *"$kw"* ]]; then
            return 0
        fi
    done
    return 1
}

is_allowlisted() {
    local rel_file="$1" fn_name="$2"
    local key="${rel_file}:${fn_name}"
    for entry in "${ALLOWLIST[@]}"; do
        if [[ "$entry" == "$key" ]]; then
            return 0
        fi
    done
    return 1
}

# Given a file and a line number where `_principal: RequestPrincipal` appears,
# walk backwards to find the nearest `async fn <name>` declaration.
# Prints the fn name, or empty string if not found.
nearest_fn_name() {
    local file="$1"
    local target_line="$2"
    local fn_name
    fn_name=$(sed -n "1,${target_line}p" "$file" 2>/dev/null \
        | grep -nE '^[[:space:]]*(pub )?(pub\(crate\) )?(async )?fn [a-zA-Z_][a-zA-Z0-9_]*' \
        | tail -1 \
        | sed -E 's/.*fn ([a-zA-Z_][a-zA-Z0-9_]*).*/\1/' \
        || true)
    echo "$fn_name"
}

# ---------------------------------------------------------------------------
# Main scan
# ---------------------------------------------------------------------------

echo "Checking for mutating handlers that discard _principal: RequestPrincipal ..."
echo ""

VIOLATIONS=0

cd "$BACKEND_DIR"

for scan_dir in "${SCAN_DIRS[@]}"; do
    [[ -d "$scan_dir" ]] || continue
    while IFS= read -r -d '' rs_file; do
        rel_file="${rs_file#./}"

        # Skip test files.
        if [[ "$rel_file" == *tests* || "$rel_file" == *_test.rs ]]; then
            continue
        fi

        while IFS= read -r match; do
            [[ -n "$match" ]] || continue
            line_num=$(echo "$match" | cut -d: -f1)
            line_text=$(echo "$match" | cut -d: -f2-)

            # Skip comment lines.
            trimmed="${line_text#"${line_text%%[![:space:]]*}"}"
            if [[ "$trimmed" == //* || "$trimmed" == \** ]]; then
                continue
            fi

            fn_name=$(nearest_fn_name "$rs_file" "$line_num")
            [[ -n "$fn_name" ]] || continue

            if ! is_mutating_fn "$fn_name"; then
                continue
            fi

            if is_allowlisted "$rel_file" "$fn_name"; then
                continue
            fi

            VIOLATIONS=$((VIOLATIONS + 1))
            if [[ $VIOLATIONS -eq 1 ]]; then
                echo -e "${RED}Mutating handlers with discarded _principal (potential IDOR):${NC}"
                echo ""
            fi
            echo "  ${rel_file}:${line_num}  fn ${fn_name}"
            echo "    ${trimmed}"
        done < <(grep -n '_principal: RequestPrincipal' "$rs_file" 2>/dev/null || true)
    done < <(find "$scan_dir" -name '*.rs' -type f -print0 2>/dev/null)
done

# ---------------------------------------------------------------------------
# Result
# ---------------------------------------------------------------------------

echo ""
echo "--------------------------------------------"
echo ""

if [[ $VIOLATIONS -gt 0 ]]; then
    echo -e "${RED}X $VIOLATIONS discarded-principal violation(s) in mutating handlers.${NC}"
    echo ""
    echo "Each listed handler extracts RequestPrincipal but does not use it."
    echo "This is the pattern behind the ai.rs IDOR cluster (voice-device,"
    echo "equipment). Fix options:"
    echo "  1. Pass principal.user_id / principal.org_id to the repository"
    echo "     query so it scopes the mutation to the caller's resource."
    echo "  2. If the discard is genuinely safe (e.g. a separate extractor"
    echo "     already enforces ownership), add to ALLOWLIST in this script"
    echo "     with a comment explaining why."
    echo ""
    if $STRICT_MODE; then
        echo -e "${RED}Failing in strict mode (--strict).${NC}"
        exit 1
    else
        echo -e "${YELLOW}Run with --strict to fail CI.  Informational mode -- exit 0.${NC}"
        exit 0
    fi
fi

echo -e "${GREEN}OK No discarded-principal violations in mutating handlers.${NC}"
exit 0
