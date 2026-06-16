#!/bin/bash
# RLS Enforcement Checker
# Detects direct database pool access in handlers that should use RlsConnection
#
# Usage: ./scripts/check-rls-enforcement.sh [--strict]
#   --strict: Fail on any direct pool access (default: warn only)

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

# The handler scan below is built on ripgrep. Every rg call is suffixed with
# `|| true` (rg exits 1 on no-match), which also swallows exit 127 when rg is
# NOT INSTALLED — on a runner without ripgrep the whole script degrades to a
# silent no-op that prints "No RLS violations found" (this is how 68 handler
# violations merged to dev behind a green gate; see PAP-68). Hard-fail instead.
if ! command -v rg >/dev/null 2>&1; then
    echo "ERROR: ripgrep (rg) is required by this check but is not installed." >&2
    echo "       Install it (apt-get install -y ripgrep / brew install ripgrep)" >&2
    echo "       — refusing to pass silently without it." >&2
    exit 2
fi

TMP_PREFIX=$(mktemp -d)
trap 'rm -rf "$TMP_PREFIX"' EXIT

# File-level baseline of KNOWN handler-side raw-pool offenders (pre-existing
# debt that merged while the gate was a no-op). Paths are relative to
# backend/. Baselined files warn instead of failing; new offenders in any
# other file still fail --strict. Remove entries as files are cleaned up.
HANDLER_BASELINE_FILE="$SCRIPT_DIR/rls-handler-baseline.txt"
: > "$TMP_PREFIX/handler_baseline"
if [[ -f "$HANDLER_BASELINE_FILE" ]]; then
    { grep -vE '^[[:space:]]*(#|$)' "$HANDLER_BASELINE_FILE" || true; } | awk '{print $1}' | sort -u > "$TMP_PREFIX/handler_baseline"
fi
: > "$TMP_PREFIX/handler_hits"
HANDLER_BASELINED=0

echo "🔍 Checking for direct database pool access in handlers..."
echo ""

# Patterns that indicate direct pool access (should use RlsConnection instead).
# Note: Current architecture uses repository pattern (state.*_repo), which is fine.
# The risk is request-handling code touching the raw pool without RLS context.
#
# Two server idioms are covered:
#   api-server    : `state.db` is the raw pool  -> state.db.acquire / &state.db / ...
#   reality-server: `self.db` is the raw pool on AppState, and handlers also reach
#                   it via `&state.db`. The ONLY sanctioned raw acquire is inside
#                   `acquire_public_conn()` (state.rs), which clears RLS context
#                   itself — see SANCTIONED_HELPER allow-list below.
VIOLATION_PATTERNS=(
    'state\.db\.acquire'
    'state\.db\.pool'
    '&state\.db[^_]'
    'state\.db\)'
    'self\.db\.acquire'
    'self\.db\.pool'
    '&self\.db[^_]'
)

# Directories to check (request-handling code that should use RLS).
# reality-server's raw pool also lives on AppState in state.rs, and its request
# guards live in extractors/ — both are scanned.
#
# routes/admin/ is intentionally excluded: platform-admin handlers operate
# across tenants and bypass per-tenant RLS by design (admin-level credentials
# + role gate + audit logging handle security there). See ADR for admin arch.
CHECK_DIRS=(
    "servers/api-server/src/handlers"
    "servers/api-server/src/routes"
    "servers/api-server/src/services"
    "servers/reality-server/src/handlers"
    "servers/reality-server/src/routes"
    "servers/reality-server/src/extractors"
)

# Individual files (not whole dirs) to scan for raw-pool access.
CHECK_FILES=(
    "servers/reality-server/src/state.rs"
)

# Name of the sanctioned raw-acquire helper on reality-server's AppState.
# A raw-pool match is allow-listed if it sits inside this fn's body OR on a
# line that simply calls it.
SANCTIONED_HELPER="acquire_public_conn"

# Additional sanctioned functions that legitimately acquire raw pool connections
# and manually set RLS context (e.g. webhook handlers with no user auth).
# Each entry is a Rust function name. The line must be inside the fn body.
SANCTIONED_WEBHOOK_FNS=(
    "store_signed_document"  # signatures.rs: webhook handler sets RLS context explicitly
)

VIOLATIONS=0
WARNINGS=0

# is_sanctioned <file> <line>
# Returns 0 (true) when the matched line is an allow-listed raw-pool access:
#   - the line itself calls the sanctioned helper (acquire_public_conn), or
#   - the line lives inside the sanctioned helper's own fn body, or
#   - the line lives inside a SANCTIONED_WEBHOOK_FNS fn body, or
#   - the file is an RlsConnection/RlsPool implementation.
# This is line-context aware: it does NOT blanket-allow an entire file just
# because the sanctioned helper happens to be defined somewhere in it.
is_sanctioned() {
    local file="$1" line="$2"
    local content
    content=$(sed -n "${line}p" "$file" 2>/dev/null || true)

    # Line directly calls the sanctioned helper.
    if [[ "$content" == *"$SANCTIONED_HELPER"* ]]; then
        return 0
    fi

    # RLS plumbing implementations are allowed to touch the raw pool.
    if grep -qE 'impl .*RlsConnection|impl .*RlsPool' "$file" 2>/dev/null; then
        return 0
    fi

    # Find the nearest preceding fn declaration to determine function context.
    local fn_decl
    fn_decl=$(sed -n "1,${line}p" "$file" 2>/dev/null \
        | grep -nE '^[[:space:]]*(pub )?(async )?fn [a-zA-Z_]' \
        | tail -1 || true)

    # Inside the sanctioned helper's fn body.
    if [[ "$fn_decl" == *"fn ${SANCTIONED_HELPER}"* ]]; then
        return 0
    fi

    # Inside a webhook/manual-RLS fn body that explicitly sets tenant context.
    for webhook_fn in "${SANCTIONED_WEBHOOK_FNS[@]}"; do
        if [[ "$fn_decl" == *"fn ${webhook_fn}"* ]]; then
            return 0
        fi
    done

    return 1
}

scan_target() {
    # scan_target <absolute-path-or-dir>
    local target="$1"
    for pattern in "${VIOLATION_PATTERNS[@]}"; do
        # ripgrep: fast search, skip health.rs / mod.rs / test files.
        while IFS= read -r match; do
            [[ -n "$match" ]] || continue
            local FILE LINE CONTENT
            FILE=$(echo "$match" | cut -d: -f1)
            LINE=$(echo "$match" | cut -d: -f2)
            CONTENT=$(echo "$match" | cut -d: -f3-)

            # Skip doc-comment / comment lines — these reference the idioms in
            # prose (e.g. state.rs documents why `&self.db` is flagged).
            local trimmed="${CONTENT#"${CONTENT%%[![:space:]]*}"}"
            if [[ "$trimmed" == //* || "$trimmed" == \** ]]; then
                continue
            fi

            if is_sanctioned "$FILE" "$LINE"; then
                continue
            fi

            local REL="${FILE#"$BACKEND_DIR"/}"
            if grep -qxF "$REL" "$TMP_PREFIX/handler_baseline"; then
                ((HANDLER_BASELINED+=1))
                echo "$REL" >> "$TMP_PREFIX/handler_hits"
                echo -e "${YELLOW}KNOWN OFFENDER${NC} [$FILE:$LINE] (baselined, cleanup pending)"
                echo "  $CONTENT"
                echo ""
                continue
            fi

            ((VIOLATIONS+=1))
            echo -e "${RED}VIOLATION${NC} [$FILE:$LINE]"
            echo "  $CONTENT"
            echo "  → Use RlsConnection extractor or RlsPool::acquire_with_rls() instead"
            echo ""
        done < <(rg -n --with-filename "$pattern" "$target" \
            --glob='!*health*.rs' \
            --glob='!*mod.rs' \
            --glob='!*_test.rs' \
            --glob='!*tests*' \
            --glob='!**/admin/**' \
            --glob='!**/admin_*.rs' \
            2>/dev/null || true)
    done
}

for dir in "${CHECK_DIRS[@]}"; do
    FULL_DIR="$BACKEND_DIR/$dir"
    [[ -d "$FULL_DIR" ]] || continue
    scan_target "$FULL_DIR"
done

for f in "${CHECK_FILES[@]}"; do
    FULL_FILE="$BACKEND_DIR/$f"
    [[ -f "$FULL_FILE" ]] || continue
    scan_target "$FULL_FILE"
done

# Stale handler-baseline entries: listed but no longer flagged — remove them
# so the ratchet only ever tightens.
sort -u "$TMP_PREFIX/handler_hits" > "$TMP_PREFIX/handler_hits_sorted"
while IFS= read -r stale; do
    [[ -n "$stale" ]] || continue
    ((WARNINGS+=1))
    echo -e "${YELLOW}WARNING${NC} stale baseline entry '$stale' — file no longer flagged; remove it from $(basename "$HANDLER_BASELINE_FILE")"
    echo ""
done < <(comm -23 "$TMP_PREFIX/handler_baseline" "$TMP_PREFIX/handler_hits_sorted")

if [[ $HANDLER_BASELINED -gt 0 ]]; then
    echo -e "${YELLOW}⚠ $HANDLER_BASELINED known handler-side raw-pool access(es) pending cleanup (baselined)${NC}"
    echo ""
fi

# Also check for repository methods that take raw pool instead of RlsConnection
echo "🔍 Checking repository patterns..."

REPO_DIR="$BACKEND_DIR/crates/db/src/repositories"
if [[ -d "$REPO_DIR" ]]; then
    # Look for methods that take &DbPool but don't have RLS context
    while IFS= read -r match; do
        if [[ -n "$match" ]]; then
            FILE=$(echo "$match" | cut -d: -f1)
            LINE=$(echo "$match" | cut -d: -f2)
            CONTENT=$(echo "$match" | cut -d: -f3-)

            ((WARNINGS+=1))
            echo -e "${YELLOW}WARNING${NC} [$FILE:$LINE]"
            echo "  $CONTENT"
            echo "  → Consider if this method needs RLS context injection"
            echo ""
        fi
    done < <(rg -n 'pub async fn.*\(&self.*pool.*DbPool' "$REPO_DIR" 2>/dev/null || true)
fi

# ──────────────────────────────────────────────────────────────────────────────
# Pool-field detector (PAP-68): repositories that OWN a raw pool and query
# FORCE-RLS tables without ever setting RLS context.
#
# The missed pattern behind PAP-67: a repository struct holds `pool: PgPool` /
# `db: DbPool`, is built once at startup with the raw pool, and runs every
# query via `&self.pool`. Under `FORCE ROW LEVEL SECURITY` (migration 00179
# et al.) the owner connection is no longer exempt, `get_current_org_id()`
# returns NULL, and the policy collapses to deny-all. The detectors above only
# catch handler-side raw acquires and pool-typed *parameters* — not pool
# *fields* — so this class merged green.
#
# A repo is flagged when ALL of the following hold:
#   1. a struct field `pool:`/`db:` typed PgPool/DbPool (incl. pub / qualified)
#   2. at least one `&self.pool` / `&self.db` use (a query or helper pass-through)
#   3. zero `set_request_context` calls anywhere in the file
#   4. the file's SQL references at least one table that a migration put under
#      FORCE ROW LEVEL SECURITY
#
# Known-but-not-yet-converted repos live in rls-pool-field-baseline.txt next to
# this script (one basename per line, '#' comments). Baselined hits warn;
# anything NOT in the baseline is a violation (fails --strict). Fixing a repo
# without removing its baseline entry produces a stale-baseline warning so the
# ratchet only ever tightens.
# ──────────────────────────────────────────────────────────────────────────────
echo "🔍 Checking repository structs holding a raw pool field (FORCE-RLS guard)..."

MIGRATIONS_DIR="$BACKEND_DIR/crates/db/migrations"
BASELINE_FILE="$SCRIPT_DIR/rls-pool-field-baseline.txt"

POOL_FIELD_VIOLATIONS=0
POOL_FIELD_BASELINED=0

if [[ -d "$REPO_DIR" && -d "$MIGRATIONS_DIR" ]]; then
    # FORCE-RLS table set, derived from migrations (source of truth). Tables
    # later relaxed with NO FORCE are subtracted.
    { grep -hE 'ALTER TABLE[[:space:]]+[A-Za-z0-9_."]+[[:space:]]+FORCE ROW LEVEL SECURITY' \
        "$MIGRATIONS_DIR"/*.sql 2>/dev/null || true; } \
        | sed -E 's/.*ALTER TABLE[[:space:]]+([A-Za-z0-9_."]+)[[:space:]]+FORCE ROW LEVEL SECURITY.*/\1/' \
        | tr -d '"' | sed 's/^public\.//' | sort -u > "$TMP_PREFIX/force"
    { grep -hE 'ALTER TABLE[[:space:]]+[A-Za-z0-9_."]+[[:space:]]+NO FORCE ROW LEVEL SECURITY' \
        "$MIGRATIONS_DIR"/*.sql 2>/dev/null || true; } \
        | sed -E 's/.*ALTER TABLE[[:space:]]+([A-Za-z0-9_."]+)[[:space:]]+NO FORCE ROW LEVEL SECURITY.*/\1/' \
        | tr -d '"' | sed 's/^public\.//' | sort -u > "$TMP_PREFIX/noforce"
    comm -23 "$TMP_PREFIX/force" "$TMP_PREFIX/noforce" > "$TMP_PREFIX/force_final"

    FORCE_COUNT=$(wc -l < "$TMP_PREFIX/force_final")
    echo "   ($FORCE_COUNT FORCE-RLS tables derived from migrations)"

    # Baseline of known offenders (pending conversion, each tracked by issue).
    : > "$TMP_PREFIX/baseline"
    if [[ -f "$BASELINE_FILE" ]]; then
        { grep -vE '^[[:space:]]*(#|$)' "$BASELINE_FILE" || true; } | awk '{print $1}' | sort -u > "$TMP_PREFIX/baseline"
    fi
    : > "$TMP_PREFIX/hit_names"

    POOL_FIELD_RE='^[[:space:]]*(pub([[:space:]]*\([^)]*\))?[[:space:]]+)?(pool|db):[[:space:]]*(sqlx::|crate::)?(PgPool|DbPool)'

    for repo_file in "$REPO_DIR"/*.rs; do
        [[ -f "$repo_file" ]] || continue
        base=$(basename "$repo_file")
        case "$base" in mod.rs|*_test.rs|*_tests.rs) continue ;; esac

        # Strip line comments so doc-prose about pools/tables can't false-positive.
        grep -vE '^[[:space:]]*//' "$repo_file" > "$TMP_PREFIX/code" || true

        grep -qE "$POOL_FIELD_RE" "$TMP_PREFIX/code" || continue
        grep -qE '&self\.(pool|db)\b' "$TMP_PREFIX/code" || continue
        grep -q 'set_request_context' "$TMP_PREFIX/code" && continue

        # Tables this file's SQL touches, intersected with the FORCE-RLS set.
        grep -ohiE '\b(from|join|into|update)[[:space:]]+[A-Za-z_][A-Za-z0-9_]*' "$TMP_PREFIX/code" \
            | awk '{print tolower($2)}' | sort -u > "$TMP_PREFIX/touched" || true
        comm -12 "$TMP_PREFIX/touched" "$TMP_PREFIX/force_final" > "$TMP_PREFIX/hit_tables"
        [[ -s "$TMP_PREFIX/hit_tables" ]] || continue

        echo "$base" >> "$TMP_PREFIX/hit_names"
        tables_short=$(head -5 "$TMP_PREFIX/hit_tables" | paste -sd, -)
        total_tables=$(wc -l < "$TMP_PREFIX/hit_tables")
        field_line=$(grep -nE "$POOL_FIELD_RE" "$repo_file" | head -1 | cut -d: -f1)

        if grep -qxF "$base" "$TMP_PREFIX/baseline"; then
            ((POOL_FIELD_BASELINED+=1))
            echo -e "${YELLOW}KNOWN OFFENDER${NC} [$repo_file:${field_line:-?}] (baselined, conversion pending)"
            echo "  raw pool field + 0 set_request_context; FORCE-RLS tables ($total_tables): $tables_short"
            echo ""
        else
            ((POOL_FIELD_VIOLATIONS+=1))
            ((VIOLATIONS+=1))
            echo -e "${RED}VIOLATION${NC} [$repo_file:${field_line:-?}]"
            echo "  Repository holds a raw pool field and queries FORCE-RLS tables"
            echo "  with zero set_request_context calls → deny-all under FORCE RLS"
            echo "  FORCE-RLS tables touched ($total_tables): $tables_short"
            echo "  → Convert to the executor pattern (take impl Executor / &mut PgConnection"
            echo "    from the RlsConnection extractor) like document.rs / board_meetings.rs,"
            echo "    or add a baseline entry with a tracking issue."
            echo ""
        fi
    done

    # Stale baseline entries: listed but no longer flagged — tighten the ratchet.
    sort -u "$TMP_PREFIX/hit_names" > "$TMP_PREFIX/hit_names_sorted"
    while IFS= read -r stale; do
        [[ -n "$stale" ]] || continue
        ((WARNINGS+=1))
        echo -e "${YELLOW}WARNING${NC} stale baseline entry '$stale' — repo no longer flagged; remove it from $(basename "$BASELINE_FILE")"
        echo ""
    done < <(comm -23 "$TMP_PREFIX/baseline" "$TMP_PREFIX/hit_names_sorted")

    if [[ $POOL_FIELD_VIOLATIONS -eq 0 ]]; then
        if [[ $POOL_FIELD_BASELINED -gt 0 ]]; then
            echo -e "${YELLOW}⚠ $POOL_FIELD_BASELINED known pool-field offender(s) pending conversion (baselined)${NC}"
        else
            echo -e "${GREEN}✓ No repositories holding raw pool fields against FORCE-RLS tables${NC}"
        fi
        echo ""
    fi
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

if [[ $VIOLATIONS -gt 0 ]]; then
    echo -e "${RED}✗ Found $VIOLATIONS RLS violation(s)${NC}"
    if [[ $WARNINGS -gt 0 ]]; then
        echo -e "${YELLOW}⚠ Found $WARNINGS warning(s) in repositories${NC}"
    fi
    echo ""
    echo "Handlers should use:"
    echo "  • RlsConnection extractor for request-scoped RLS"
    echo "  • RlsPool::acquire_with_rls() for explicit context"
    echo "  • RlsPool::acquire_public() for unauthenticated routes"
    echo ""
    if $STRICT_MODE; then
        echo -e "${RED}Failing in strict mode.${NC}"
        exit 1
    else
        echo -e "${YELLOW}Run with --strict to fail CI on violations.${NC}"
        exit 0
    fi
else
    echo -e "${GREEN}✓ No RLS violations found${NC}"
    if [[ $WARNINGS -gt 0 ]]; then
        echo -e "${YELLOW}⚠ Found $WARNINGS warning(s) in repositories (review recommended)${NC}"
    fi
    exit 0
fi
