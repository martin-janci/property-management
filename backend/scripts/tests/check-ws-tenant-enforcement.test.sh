#!/bin/bash
# Self-tests for check-ws-tenant-enforcement.sh
#
# Usage: bash backend/scripts/tests/check-ws-tenant-enforcement.test.sh
#
# Creates isolated temporary directory trees that mimic crate src/ layouts,
# runs the gate script against them, and asserts the expected exit code.
#
# Note: The gate is invoked via symlinks so that its BACKEND_DIR resolution
# (dirname of its own path) points at the temp tree rather than the real repo.

set -uo pipefail
# Intentionally NOT using -e: we need to capture non-zero exits from the gate.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GATE="$SCRIPT_DIR/../check-ws-tenant-enforcement.sh"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

PASS=0
FAIL=0

# Accumulate temp dirs for cleanup.
TMPDIRS=()
cleanup() { rm -rf "${TMPDIRS[@]:-}"; }
trap cleanup EXIT

assert_exit() {
    local description="$1"
    local expected_exit="$2"
    local actual_exit="$3"

    if [[ "$actual_exit" -eq "$expected_exit" ]]; then
        echo -e "${GREEN}PASS${NC}  $description"
        ((PASS+=1))
    else
        echo -e "${RED}FAIL${NC}  $description"
        echo "       expected exit $expected_exit, got $actual_exit"
        ((FAIL+=1))
    fi
}

# make_fake_backend <tmpdir>
# Creates <tmpdir>/backend/scripts/ with a symlink to the gate and returns
# the path to <tmpdir>/backend so callers can populate servers/ beneath it.
make_fake_backend() {
    local tmpdir="$1"
    local fake_backend="$tmpdir/backend"
    mkdir -p "$fake_backend/scripts"
    ln -s "$GATE" "$fake_backend/scripts/check-ws-tenant-enforcement.sh"
    printf '%s' "$fake_backend"
}

# run_gate <fake_backend> [--strict]
# Runs the gate symlinked inside <fake_backend>/scripts/ and captures exit.
run_gate() {
    local fb="$1"; shift
    local actual
    "$fb/scripts/check-ws-tenant-enforcement.sh" "$@" > /dev/null 2>&1
    actual=$?
    printf '%d' "$actual"
}

# ------------------------------------------------------------------ Test 1 --
# No WS code at all in the real repo → exit 0.
echo "Running test 1: real repo (no WS code)..."
actual1=0
"$GATE" --strict > /dev/null 2>&1 || actual1=$?
assert_exit "no-ws-code in real repo exits 0" 0 "$actual1"

# ------------------------------------------------------------------ Test 2 --
# Violating case: WebSocketUpgrade with NO middleware in file or crate.
echo "Running test 2: ws-without-middleware (expect violation)..."
T2=$(mktemp -d); TMPDIRS+=("$T2")
fb2=$(make_fake_backend "$T2")
mkdir -p "$fb2/servers/api-server/src"

cat > "$fb2/servers/api-server/src/ws_handler.rs" << 'RUST'
use axum::extract::ws::WebSocketUpgrade;

pub async fn ws_handler(upgrade: WebSocketUpgrade) {
    upgrade.on_upgrade(|_socket| async {});
}
RUST

actual2=0
run_gate "$fb2" --strict > /dev/null 2>&1 || true
# run_gate already redirects; we need a direct call to capture exit.
"$fb2/scripts/check-ws-tenant-enforcement.sh" --strict > /dev/null 2>&1 && actual2=0 || actual2=$?
assert_exit "ws-without-middleware fails with --strict (exit 1)" 1 "$actual2"

# ------------------------------------------------------------------ Test 3 --
# Passing case: WS upgrade WITH host_tenant_middleware reference in same file.
echo "Running test 3: ws-with-middleware in same file (expect pass)..."
T3=$(mktemp -d); TMPDIRS+=("$T3")
fb3=$(make_fake_backend "$T3")
mkdir -p "$fb3/servers/api-server/src"

cat > "$fb3/servers/api-server/src/ws_handler.rs" << 'RUST'
use axum::extract::ws::WebSocketUpgrade;
use axum::middleware::from_fn_with_state;
use crate::middleware::host_tenant_middleware;

pub fn ws_router(state: AppState) -> Router {
    Router::new()
        .route("/ws/notifications", axum::routing::get(ws_handler_inner))
        .route_layer(from_fn_with_state(state.clone(), host_tenant_middleware))
}

pub async fn ws_handler_inner(upgrade: WebSocketUpgrade) {
    upgrade.on_upgrade(|_socket| async {});
}
RUST

actual3=0
"$fb3/scripts/check-ws-tenant-enforcement.sh" --strict > /dev/null 2>&1 && actual3=0 || actual3=$?
assert_exit "ws-with-middleware-in-same-file exits 0" 0 "$actual3"

# ------------------------------------------------------------------ Test 4 --
# Passing case: WS in handler file, middleware in a sibling routes/mod.rs
# (same crate src/ tree, different file — crate-wide scan must find it).
echo "Running test 4: ws-in-handler-middleware-in-sibling (expect pass)..."
T4=$(mktemp -d); TMPDIRS+=("$T4")
fb4=$(make_fake_backend "$T4")
mkdir -p "$fb4/servers/api-server/src/routes"

# Handler: WS only, no middleware reference here.
cat > "$fb4/servers/api-server/src/routes/ws.rs" << 'RUST'
use axum::extract::ws::WebSocketUpgrade;

pub async fn ws_handler(upgrade: WebSocketUpgrade) {
    upgrade.on_upgrade(|_socket| async {});
}
RUST

# Router module: wires the middleware (sibling file in same crate src/).
cat > "$fb4/servers/api-server/src/routes/mod.rs" << 'RUST'
use axum::middleware::from_fn_with_state;
use crate::middleware::host_tenant_middleware;

pub mod ws;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .merge(ws_router(state.clone()))
        .route_layer(from_fn_with_state(state, host_tenant_middleware))
}
RUST

actual4=0
"$fb4/scripts/check-ws-tenant-enforcement.sh" --strict > /dev/null 2>&1 && actual4=0 || actual4=$?
assert_exit "ws-in-handler-middleware-in-sibling-mod exits 0 (crate-wide scan)" 0 "$actual4"

# ------------------------------------------------------------------ summary --
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Results: $PASS passed, $FAIL failed"
echo ""

if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
exit 0
