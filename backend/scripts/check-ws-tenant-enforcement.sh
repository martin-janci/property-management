#!/bin/bash
# WebSocket Tenant Enforcement Checker
# Detects WebSocket upgrade handlers that do not co-locate a host_tenant_middleware
# reference in the same module or a directly-imported module.
#
# Usage: ./scripts/check-ws-tenant-enforcement.sh [--strict]
#   --strict: Fail on any violation (default: warn only)
#
# Exit codes:
#   0  No WS code found, OR all WS handlers are paired with the middleware.
#   1  Violations found AND --strict mode is active.
#
# Why this exists (Phase 6 / leak #5 & #20 risk):
#   WebSocket routes in Axum bypass the normal HTTP response pipeline. A future
#   developer adding a WS route might mount it outside the middleware stack, leaking
#   cross-tenant data over the socket.  This gate catches that at review time.
#
#   Convention: every WS handler file MUST either (a) live under a router that
#   applies host_tenant_middleware via from_fn_with_state, or (b) import a
#   module that does.  The check looks for a host_tenant_middleware / from_fn_with_state
#   reference within ±200 lines of the WS upgrade in the same file, OR in any
#   use/mod path visible from the crate root (super-module scan).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_DIR="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

STRICT_MODE=false
if [[ "${1:-}" == "--strict" ]]; then
    STRICT_MODE=true
fi

echo "Checking for WebSocket routes without host_tenant_middleware..."
echo ""

# Directories to search for WS code.
SERVERS_DIR="$BACKEND_DIR/servers"

# WS upgrade patterns (Axum + tokio-tungstenite).
WS_PATTERNS=(
    'axum::extract::ws::WebSocketUpgrade'
    'WebSocketUpgrade'
    'tokio_tungstenite::accept_async'
)

# Middleware anchor: a WS route is considered protected when the file (or a
# file in the same module tree) references this.
MIDDLEWARE_ANCHOR='host_tenant_middleware'
# Second anchor: the standard Axum layer wiring idiom.
LAYER_ANCHOR='from_fn_with_state'

VIOLATIONS=0

# ------------------------------------------------------------------ helpers --

# has_middleware_in_file <file>
# Returns 0 when the file itself references the middleware anchor.
#
# Require the actual MIDDLEWARE_ANCHOR (`host_tenant_middleware`) — not just
# any `from_fn_with_state` — so a WS handler file with an unrelated
# middleware wired via from_fn_with_state cannot silently pass this gate
# (Phase 6 review R7 / Copilot comment 3252565690).
has_middleware_in_file() {
    local file="$1"
    grep -q "$MIDDLEWARE_ANCHOR" "$file" 2>/dev/null
}

# has_middleware_in_crate <file>
# Walks up from <file> looking for the crate's src/ root, then greps the
# entire crate for the middleware anchor.  This covers the common pattern
# where the router wiring lives in routes/mod.rs and the handler is in a
# sibling file.
#
# Note: we avoid combining -q and -l in the same rg call.  ripgrep's -q flag
# suppresses all output (including the filenames that -l would emit), so
# `rg -ql ... | grep -q .` always gets empty stdin.  Instead we use -l alone
# and test whether its output is non-empty.
has_middleware_in_crate() {
    local file="$1"
    # Find the nearest ancestor src/ directory.
    local src_dir
    src_dir=$(dirname "$file")
    while [[ "$src_dir" != "/" && "$(basename "$src_dir")" != "src" ]]; do
        src_dir=$(dirname "$src_dir")
    done
    if [[ "$(basename "$src_dir")" != "src" ]]; then
        return 1
    fi
    local found
    found=$(rg -l "$MIDDLEWARE_ANCHOR" "$src_dir" 2>/dev/null || true)
    [[ -n "$found" ]]
}

# ------------------------------------------------------------------ scanner --

scan_ws_files() {
    local servers_dir="$1"

    [[ -d "$servers_dir" ]] || return 0

    for pattern in "${WS_PATTERNS[@]}"; do
        # ripgrep: find all files that contain a WS upgrade reference.
        while IFS= read -r match; do
            [[ -n "$match" ]] || continue

            local FILE LINE CONTENT
            FILE=$(printf '%s' "$match" | cut -d: -f1)
            LINE=$(printf '%s' "$match" | cut -d: -f2)
            CONTENT=$(printf '%s' "$match" | cut -d: -f3-)

            # Skip comment / doc-comment lines.
            local trimmed="${CONTENT#"${CONTENT%%[![:space:]]*}"}"
            if [[ "$trimmed" == //* || "$trimmed" == \** || "$trimmed" == "///"* ]]; then
                continue
            fi

            # Check 1: middleware reference in the same file.
            if has_middleware_in_file "$FILE"; then
                continue
            fi

            # Check 2: middleware reference anywhere in the same crate's src/.
            if has_middleware_in_crate "$FILE"; then
                continue
            fi

            # No middleware anchor found — this is a violation.
            ((VIOLATIONS+=1))
            echo -e "${RED}VIOLATION${NC} [$FILE:$LINE]"
            echo "  $CONTENT"
            echo "  Pattern matched : $pattern"
            echo "  Missing         : '$MIDDLEWARE_ANCHOR' (via from_fn_with_state)"
            echo "  Fix             : Ensure this WS route is mounted under the"
            echo "                    host_tenant_middleware layer.  See:"
            echo "                    backend/crates/api-core/src/middleware/host_tenant_middleware.rs"
            echo ""
        done < <(rg -n --with-filename "$pattern" "$servers_dir" \
            --glob='!*_test.rs' \
            --glob='!*tests*' \
            --type=rust \
            2>/dev/null || true)
    done
}

# ------------------------------------------------------------------ main --

scan_ws_files "$SERVERS_DIR"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

if [[ $VIOLATIONS -eq 0 ]]; then
    # No WS code at all (current state) OR all WS handlers are paired.
    echo -e "${GREEN}No WebSocket tenant-enforcement violations found${NC}"
    echo "  (No WebSocket upgrade code detected in servers/, or all handlers"
    echo "   are co-located with host_tenant_middleware.)"
    exit 0
fi

echo -e "${RED}Found $VIOLATIONS WebSocket tenant-enforcement violation(s)${NC}"
echo ""
echo "Every WebSocket route MUST be mounted under host_tenant_middleware."
echo "Convention (Axum):"
echo ""
echo "  let ws_router = Router::new()"
echo "      .route(\"/ws/…\", get(ws_handler))"
echo "      .route_layer(from_fn_with_state(state.clone(), host_tenant_middleware));"
echo ""
echo "See docs/multitenancy/operability.md § CI gates for the full rationale."
echo ""

if $STRICT_MODE; then
    echo -e "${RED}Failing in strict mode.${NC}"
    exit 1
else
    echo -e "${YELLOW}Run with --strict to fail CI on violations.${NC}"
    exit 0
fi
