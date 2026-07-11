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
    # BIT-78 (GH #1302/#1304): two evasions the patterns above missed.
    # `state.db.begin()` opens a raw-pool transaction with NO RLS context (the
    # mfa.rs disable_mfa / verify_recovery_code defect). `state.db.clone()` hands
    # the raw pool to ad-hoc code. The constructor idiom
    # (`Repo::new(state.db.clone())`) and struct-field init (`db: self.db.clone(),`)
    # are allow-listed in is_sanctioned(); a *bare* `let x = state.db.clone();`
    # raw-pool extraction is flagged.
    'state\.db\.begin'
    'self\.db\.begin'
    'state\.db\.clone'
    'self\.db\.clone'
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

    # BIT-78: `.clone()` of the raw pool is the canonical way to construct a
    # repository / service / RlsPool (`Repo::new(state.db.clone())`) or to seed
    # a struct field (`db: self.db.clone(),`). Those hand the pool to an
    # RLS-aware abstraction, so they are sanctioned. The evasion this guards
    # against is *binding the raw pool to a local* and querying it directly
    # (`let pool = state.db.clone();`). Sanction the wrapped/arg/field form;
    # flag a bare binding whose entire right-hand side is the clone.
    if [[ "$content" == *".db.clone("* ]]; then
        if [[ "$content" =~ =[[:space:]]*(state|self)\.db\.clone\(\)[[:space:]]*\;?[[:space:]]*$ ]]; then
            return 1   # bare `let x = state.db.clone();` → not sanctioned
        fi
        return 0       # passed into a constructor/call or a struct field
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
    if $STRICT_MODE; then ((VIOLATIONS+=1)); fi
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
# First-class allowlist of service-role-by-design repos. These are NOT
# conversion-pending debt: their table RLS policy is a service-role *allowance*
# (permits the unset-GUC service pool), so the executor pattern is inapplicable
# and converting them would flip the policy to deny. Kept distinct from the
# baseline so they are reported as ALLOWED, never "conversion pending", and a
# PAP-80-style sweep can't mis-classify them. See the file header for the
# access-model invariant.
SERVICE_ROLE_ALLOWLIST_FILE="$SCRIPT_DIR/rls-service-role-allowlist.txt"

POOL_FIELD_VIOLATIONS=0
POOL_FIELD_BASELINED=0
POOL_FIELD_SERVICE_ROLE=0

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

    # Service-role-by-design allowlist (permanent, distinct semantics from the
    # baseline — see SERVICE_ROLE_ALLOWLIST_FILE comment above).
    : > "$TMP_PREFIX/service_role"
    if [[ -f "$SERVICE_ROLE_ALLOWLIST_FILE" ]]; then
        { grep -vE '^[[:space:]]*(#|$)' "$SERVICE_ROLE_ALLOWLIST_FILE" || true; } | awk '{print $1}' | sort -u > "$TMP_PREFIX/service_role"
    fi
    : > "$TMP_PREFIX/hit_names"
    : > "$TMP_PREFIX/service_role_hits"

    POOL_FIELD_RE='^[[:space:]]*(pub([[:space:]]*\([^)]*\))?[[:space:]]+)?(pool|db):[[:space:]]*(sqlx::|crate::)?(PgPool|DbPool)'

    # Classify ONE logical repository unit against the 4 pool-field criteria.
    # A "unit" is either a single top-level repo file (e.g. agency.rs) OR a
    # split-repo module directory (e.g. rental/) whose struct field lives in
    # mod.rs while the &self.pool queries + SQL live in sibling sub-modules. For
    # a directory the caller passes the COMBINED, comment-stripped source of every
    # *.rs in it, so the field-in-mod.rs / queries-in-sub-modules split is
    # analysed as one repository — exactly the shape it had before the move-only
    # refactor. Baseline / allowlist matching is by the unit's NAME: a basename
    # for files, "<dir>/mod.rs" for directory modules.
    #   $1 name   $2 comment-stripped code file   $3 field-declaring report file
    classify_pool_field_unit() {
        local name="$1" codefile="$2" report_file="$3"

        grep -qE "$POOL_FIELD_RE" "$codefile" || return 0
        grep -qE '&self\.(pool|db)\b' "$codefile" || return 0
        grep -q 'set_request_context' "$codefile" && return 0

        # Tables this unit's SQL touches, intersected with the FORCE-RLS set.
        grep -ohiE '\b(from|join|into|update)[[:space:]]+[A-Za-z_][A-Za-z0-9_]*' "$codefile" \
            | awk '{print tolower($2)}' | sort -u > "$TMP_PREFIX/touched" || true
        comm -12 "$TMP_PREFIX/touched" "$TMP_PREFIX/force_final" > "$TMP_PREFIX/hit_tables"
        [[ -s "$TMP_PREFIX/hit_tables" ]] || return 0

        echo "$name" >> "$TMP_PREFIX/hit_names"
        local tables_short total_tables field_line
        tables_short=$(head -5 "$TMP_PREFIX/hit_tables" | paste -sd, -)
        total_tables=$(wc -l < "$TMP_PREFIX/hit_tables")
        # Located only now that the unit is a confirmed hit, so the grep always
        # matches (avoids a pipefail-under-set-e abort on non-matching files).
        field_line=$(grep -nE "$POOL_FIELD_RE" "$report_file" 2>/dev/null | head -1 | cut -d: -f1) || true
        field_line="${field_line:-?}"

        if grep -qxF "$name" "$TMP_PREFIX/service_role"; then
            # Service-role-by-design: the table RLS policy permits the unset-GUC
            # service pool. Correct as written; the executor pattern does NOT
            # apply (a request-scoped user GUC would flip the policy to deny).
            # Reported as ALLOWED, never "conversion pending".
            ((POOL_FIELD_SERVICE_ROLE+=1))
            echo "$name" >> "$TMP_PREFIX/service_role_hits"
            echo -e "${GREEN}ALLOWED${NC} [$report_file:${field_line}] (service-role-by-design)"
            echo "  raw pool field on the off-request service path; table policy permits the"
            echo "  unset-GUC service pool by design — FORCE-RLS tables ($total_tables): $tables_short"
            echo ""
        elif grep -qxF "$name" "$TMP_PREFIX/baseline"; then
            ((POOL_FIELD_BASELINED+=1))
            echo -e "${YELLOW}KNOWN OFFENDER${NC} [$report_file:${field_line}] (baselined, conversion pending)"
            echo "  raw pool field + 0 set_request_context; FORCE-RLS tables ($total_tables): $tables_short"
            echo ""
        else
            ((POOL_FIELD_VIOLATIONS+=1))
            ((VIOLATIONS+=1))
            echo -e "${RED}VIOLATION${NC} [$report_file:${field_line}]"
            echo "  Repository holds a raw pool field and queries FORCE-RLS tables"
            echo "  with zero set_request_context calls → deny-all under FORCE RLS"
            echo "  FORCE-RLS tables touched ($total_tables): $tables_short"
            echo "  → Convert to the executor pattern (take impl Executor / &mut PgConnection"
            echo "    from the RlsConnection extractor) like document.rs / board_meetings.rs,"
            echo "    or add a baseline entry with a tracking issue. If the table policy is a"
            echo "    service-role allowance (unset-GUC permitted, off-request path), add it to"
            echo "    $(basename "$SERVICE_ROLE_ALLOWLIST_FILE") instead — see that file's invariant."
            echo ""
        fi
    }

    # (1) Top-level single-file repositories.
    for repo_file in "$REPO_DIR"/*.rs; do
        [[ -f "$repo_file" ]] || continue
        base=$(basename "$repo_file")
        case "$base" in mod.rs|*_test.rs|*_tests.rs) continue ;; esac

        # Strip line comments so doc-prose about pools/tables can't false-positive.
        grep -vE '^[[:space:]]*//' "$repo_file" > "$TMP_PREFIX/code" || true

        classify_pool_field_unit "$base" "$TMP_PREFIX/code" "$repo_file"
    done

    # (2) Split-repo module directories (move-only refactors: rental/, document/,
    # subscription/, sensor/, …). The raw pool field lives in <dir>/mod.rs while
    # the &self.pool queries live in sub-modules, so NEITHER the top-level glob
    # above NOR the mod.rs skip would ever see them — a blind spot introduced by
    # the directory split. Analyse the whole directory as ONE logical repository
    # from its combined source, tracked in the baseline under key "<dir>/mod.rs".
    for repo_subdir in "$REPO_DIR"/*/; do
        [[ -d "$repo_subdir" ]] || continue
        dir_base=$(basename "$repo_subdir")

        # Combined, comment-stripped source of every non-test .rs in the dir
        # (mod.rs + sub-modules).
        : > "$TMP_PREFIX/code"
        for unit_file in "$repo_subdir"*.rs; do
            [[ -f "$unit_file" ]] || continue
            case "$(basename "$unit_file")" in *_test.rs|*_tests.rs) continue ;; esac
            grep -vE '^[[:space:]]*//' "$unit_file" >> "$TMP_PREFIX/code" || true
        done

        # Report against the file that actually declares the pool field (mod.rs by
        # convention); fall back to <dir>/mod.rs for the location string.
        field_file=$( { grep -lE "$POOL_FIELD_RE" "$repo_subdir"*.rs 2>/dev/null | head -1; } || true )
        field_file="${field_file:-${repo_subdir}mod.rs}"
        classify_pool_field_unit "$dir_base/mod.rs" "$TMP_PREFIX/code" "$field_file"
    done

    # Stale baseline entries: listed but no longer flagged — tighten the ratchet.
    sort -u "$TMP_PREFIX/hit_names" > "$TMP_PREFIX/hit_names_sorted"
    while IFS= read -r stale; do
        [[ -n "$stale" ]] || continue
        ((WARNINGS+=1))
        if $STRICT_MODE; then ((VIOLATIONS+=1)); fi
        echo -e "${YELLOW}WARNING${NC} stale baseline entry '$stale' — repo no longer flagged; remove it from $(basename "$BASELINE_FILE")"
        echo ""
    done < <(comm -23 "$TMP_PREFIX/baseline" "$TMP_PREFIX/hit_names_sorted")

    # Stale service-role allowlist entries: listed but no longer flagged by the
    # detector (e.g. the repo was refactored away from a raw pool field). These
    # are by-design permanent allowances, not a ratchet, so this is purely
    # informational — it NEVER fails --strict.
    sort -u "$TMP_PREFIX/service_role_hits" > "$TMP_PREFIX/service_role_hits_sorted"
    while IFS= read -r stale; do
        [[ -n "$stale" ]] || continue
        ((WARNINGS+=1))
        echo -e "${YELLOW}INFO${NC} service-role allowlist entry '$stale' is no longer flagged by the detector; it can be removed from $(basename "$SERVICE_ROLE_ALLOWLIST_FILE") (informational — does not fail CI)"
        echo ""
    done < <(comm -23 "$TMP_PREFIX/service_role" "$TMP_PREFIX/service_role_hits_sorted")

    if [[ $POOL_FIELD_SERVICE_ROLE -gt 0 ]]; then
        echo -e "${GREEN}✓ $POOL_FIELD_SERVICE_ROLE service-role-by-design repo(s) allowed (policy permits the unset-GUC service pool; not conversion debt)${NC}"
        echo ""
    fi

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
