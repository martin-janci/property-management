#!/bin/bash
# =============================================================================
# no-raw-redis CI lint  (Phase 5.5 — Tenant Lifecycle & Operability)
# =============================================================================
# Defense for brainstorming leak #20 ("Shared Redis cross-tenant"):
# every Redis read/write must go through `TenantedRedis`, which prefixes
# every key with `t:<org_id>:`. Direct use of the underlying `redis` crate
# (or the lower-level `RedisClient` from the integrations crate) outside
# the tenanted wrapper is forbidden.
#
# This grep-based gate is intentionally simple: a developer who reaches
# for `redis::Client`, `redis::Cmd`, or `RedisClient::set` outside of an
# explicit allowlist gets stopped at PR time.
#
# Allowlist (files where raw Redis is intentional):
#   - integrations/src/redis.rs       — defines RedisClient itself
#   - api-core/src/cache/tenanted_redis.rs  — the wrapper (uses raw client internally)
#   - any file under tests/ or examples/
#
# Usage: ./scripts/lints/no-raw-redis.sh
#   exits 0 if clean, 1 if any disallowed raw-redis call is found.
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Patterns considered "raw Redis use".
PATTERNS=(
    'redis::Client'
    'redis::Cmd'
    'redis::cmd\b'
    'use redis::'
    'RedisClientInner'
    'ConnectionManager::new'
)

# Files (relative to BACKEND_DIR) that are EXPLICITLY allowed to use raw Redis.
ALLOWLIST=(
    'crates/integrations/src/redis.rs'
    'crates/api-core/src/cache/tenanted_redis.rs'
)

echo "🔒 no-raw-redis lint (Phase 5.5 leak #20 defense)"
echo ""

is_allowlisted() {
    local path="$1"
    for allowed in "${ALLOWLIST[@]}"; do
        if [[ "$path" == "$allowed" ]]; then
            return 0
        fi
    done
    # Allow tests + examples.
    if [[ "$path" == */tests/* ]] || [[ "$path" == */examples/* ]] \
            || [[ "$path" == tests/* ]] || [[ "$path" == examples/* ]]; then
        return 0
    fi
    return 1
}

VIOLATIONS=0
TMPFILE="$(mktemp)"
trap 'rm -f "$TMPFILE"' EXIT

# Build a single grep alternation.
PATTERN_ALT="$(IFS='|'; echo "${PATTERNS[*]}")"

# Walk every Rust file under crates/ and servers/.
cd "$BACKEND_DIR"
find crates servers -name '*.rs' -type f -print0 \
    | xargs -0 grep -nE "$PATTERN_ALT" 2>/dev/null > "$TMPFILE" || true

while IFS=: read -r file line text; do
    [[ -z "$file" ]] && continue
    if is_allowlisted "$file"; then
        continue
    fi
    if [[ $VIOLATIONS -eq 0 ]]; then
        echo -e "${RED}✗ Raw Redis use detected outside the tenanted wrapper:${NC}"
        echo ""
    fi
    VIOLATIONS=$((VIOLATIONS + 1))
    echo "  $file:$line"
    echo "    $(echo "$text" | sed -e 's/^[[:space:]]*//')"
done < "$TMPFILE"

if [[ $VIOLATIONS -gt 0 ]]; then
    echo ""
    echo -e "${RED}$VIOLATIONS violation(s).${NC}"
    echo ""
    echo "Use api_core::cache::TenantedRedis instead. It transparently prefixes"
    echo "every key with t:<org_id>: so cross-tenant cache collisions are"
    echo "impossible by construction (Phase 5.5 defense, leak #20)."
    echo ""
    echo "If a file is genuinely allowed to use raw Redis (e.g. a new"
    echo "primitive that the wrapper depends on), add it to ALLOWLIST in"
    echo "this script, with a comment explaining why."
    exit 1
fi

echo -e "${GREEN}✓ No raw Redis use outside the tenanted wrapper.${NC}"
exit 0
