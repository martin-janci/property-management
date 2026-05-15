#!/bin/bash
# =============================================================================
# RLS Policy-Coverage Gate  (Phase 0 — RLS Foundation Hardening)
# =============================================================================
# This script is the TENANT-DATA MANIFEST (design-session leak #17): it is the
# single authoritative answer to the question "which tables hold tenant-scoped
# data, and are they all protected by an RLS policy?".
#
# It complements `check-rls-enforcement.sh` (which greps Rust source for raw
# pool access). This one inspects the *database itself*:
#
#   1. Spin up an ephemeral PostgreSQL 16 (Docker), or reuse $DATABASE_URL.
#   2. Apply init-db.sql + every migration in crates/db/migrations/*.sql in order.
#   3. Walk the foreign-key graph to find every table that is tenant-scoped:
#        - has an `organization_id` (or `org_id`) column, OR
#        - is FK-reachable (transitively) to such a table.
#   4. Assert every tenant-scoped table has >= 1 row-level-security POLICY
#      (pg_policies), not merely RLS *enabled* (rowsecurity) — the existing
#      `validate_rls_coverage()` view only checks the latter, which is the gap
#      this gate closes.
#
# Tables in EXEMPT_TABLES are intentionally NOT org-isolated (they ARE the
# tenant, or are pre-auth/global infrastructure) and are excluded from the
# requirement.
#
# Usage: ./scripts/check-rls-coverage.sh [--strict]
#   --strict : exit 1 on any uncovered tenant-scoped table (use in CI)
#   (default): report only, exit 0
#
# Env:
#   DATABASE_URL : if set AND reachable, used instead of spinning up Docker.
#   RLS_PG_IMAGE : postgres image for the ephemeral DB (default: postgres:16).
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_DIR="$(dirname "$SCRIPT_DIR")"
MIGRATIONS_DIR="$BACKEND_DIR/crates/db/migrations"
INIT_SQL="$SCRIPT_DIR/init-db.sql"

RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m'

STRICT_MODE=false
if [[ "${1:-}" == "--strict" ]]; then
    STRICT_MODE=true
fi

# -----------------------------------------------------------------------------
# EXEMPT TABLES — intentionally have no org-isolation policy.
#   organizations            : IS the tenant boundary
#   users                    : global identity (cross-org membership)
#   organization_members     : the org<->user join, evaluated by RLS helpers
#   roles                    : global role catalog
#   email_verification_tokens, refresh_tokens, password_reset_tokens,
#   login_attempts           : pre-authentication, no org context exists yet
# Keep this list in sync with validate_rls_coverage() in 00006.
# -----------------------------------------------------------------------------
EXEMPT_TABLES=(
    organizations
    users
    organization_members
    roles
    email_verification_tokens
    refresh_tokens
    password_reset_tokens
    login_attempts
)

echo "🔍 RLS policy-coverage gate (tenant-data manifest)"
echo ""

# -----------------------------------------------------------------------------
# Establish a psql-able connection. Prefer an externally provided DATABASE_URL
# (e.g. the postgres service in CI's test job); otherwise boot a throwaway
# Docker container.
# -----------------------------------------------------------------------------
PSQL_BASE=()        # command prefix that runs psql against the target DB
CLEANUP_CONTAINER=""

cleanup() {
    if [[ -n "$CLEANUP_CONTAINER" ]]; then
        docker rm -f "$CLEANUP_CONTAINER" >/dev/null 2>&1 || true
    fi
}
trap cleanup EXIT

try_external_db() {
    [[ -n "${DATABASE_URL:-}" ]] || return 1
    command -v psql >/dev/null 2>&1 || return 1
    if psql "$DATABASE_URL" -c 'SELECT 1' >/dev/null 2>&1; then
        PSQL_BASE=(psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -qtA)
        echo "  Using DATABASE_URL (external Postgres)."
        return 0
    fi
    return 1
}

boot_docker_db() {
    command -v docker >/dev/null 2>&1 || {
        echo -e "${RED}✗ Neither a reachable DATABASE_URL nor Docker is available.${NC}"
        echo "  Set DATABASE_URL to a Postgres instance, or install Docker."
        exit 2
    }
    local image="${RLS_PG_IMAGE:-postgres:16}"
    CLEANUP_CONTAINER="rls-coverage-check-$$"
    echo "  Booting ephemeral Postgres ($image) as $CLEANUP_CONTAINER ..."
    docker run -d --rm \
        --name "$CLEANUP_CONTAINER" \
        -e POSTGRES_USER=postgres \
        -e POSTGRES_PASSWORD=postgres \
        -e POSTGRES_DB=rls_check \
        "$image" >/dev/null

    # Wait for readiness. pg_isready can report ready during the init phase
    # *before* POSTGRES_DB exists (Postgres restarts mid-init), so we probe an
    # actual SELECT against the target database.
    local tries=0
    until docker exec "$CLEANUP_CONTAINER" \
            psql -U postgres -d rls_check -c 'SELECT 1' >/dev/null 2>&1; do
        tries=$((tries + 1))
        if [[ $tries -gt 60 ]]; then
            echo -e "${RED}✗ Postgres container did not become ready in time.${NC}"
            docker logs "$CLEANUP_CONTAINER" 2>&1 | tail -20 >&2 || true
            exit 2
        fi
        sleep 1
    done
    # psql executed *inside* the container so we need no host psql client.
    PSQL_BASE=(docker exec -i "$CLEANUP_CONTAINER" psql -U postgres -d rls_check -v ON_ERROR_STOP=1 -qtA)
}

if ! try_external_db; then
    boot_docker_db
fi

psql_run() { "${PSQL_BASE[@]}" "$@"; }

# -----------------------------------------------------------------------------
# Apply schema: init-db.sql first (extensions + helper fns), then every
# migration in lexical order — identical to sqlx::migrate!.
# -----------------------------------------------------------------------------
echo "  Applying init-db.sql + $(ls "$MIGRATIONS_DIR"/*.sql | wc -l | tr -d ' ') migrations ..."

if [[ -f "$INIT_SQL" ]]; then
    psql_run -f - < "$INIT_SQL" >/dev/null
fi

for mig in $(ls "$MIGRATIONS_DIR"/*.sql | sort); do
    if ! psql_run -f - < "$mig" >/dev/null 2>/tmp/rls_mig_err.$$; then
        echo -e "${RED}✗ Migration failed: $(basename "$mig")${NC}"
        cat /tmp/rls_mig_err.$$ >&2 || true
        rm -f /tmp/rls_mig_err.$$
        exit 2
    fi
done
rm -f /tmp/rls_mig_err.$$
echo "  Schema applied."
echo ""

# -----------------------------------------------------------------------------
# Compute the tenant-scoped table set via the FK graph, entirely in SQL.
#
#   org_rooted   : tables with an organization_id / org_id column
#   reachable    : recursive closure — table T is tenant-scoped if it has a
#                  FK to any tenant-scoped table.
# Then LEFT JOIN against pg_policies to find the ones with zero policies.
# -----------------------------------------------------------------------------
EXEMPT_SQL=$(printf "'%s'," "${EXEMPT_TABLES[@]}")
EXEMPT_SQL="${EXEMPT_SQL%,}"

COVERAGE_SQL=$(cat <<SQL
WITH org_rooted AS (
    -- cast away information_schema.sql_identifier so the type lines up with the
    -- text columns produced by ::regclass::text in the recursive term below.
    SELECT DISTINCT c.table_name::text AS table_name
    FROM information_schema.columns c
    WHERE c.table_schema = 'public'
      AND c.column_name IN ('organization_id', 'org_id')
),
fk_edges AS (
    -- child depends on parent
    SELECT
        con.conrelid::regclass::text  AS child,
        con.confrelid::regclass::text AS parent
    FROM pg_constraint con
    WHERE con.contype = 'f'
      AND con.connamespace = 'public'::regnamespace
),
tenant_scoped AS (
    SELECT table_name FROM org_rooted
    UNION
    -- transitive FK closure: any table reaching a tenant-scoped table
    SELECT fk.child
    FROM fk_edges fk
    JOIN tenant_scoped ts ON ts.table_name = fk.parent
),
candidate AS (
    SELECT t.tablename
    FROM pg_tables t
    JOIN tenant_scoped ts ON ts.table_name = t.tablename
    WHERE t.schemaname = 'public'
      AND t.tablename NOT IN ($EXEMPT_SQL)
)
SELECT
    c.tablename,
    COALESCE(p.cnt, 0) AS policy_count
FROM candidate c
LEFT JOIN (
    SELECT tablename, COUNT(*) AS cnt
    FROM pg_policies
    WHERE schemaname = 'public'
    GROUP BY tablename
) p ON p.tablename = c.tablename
ORDER BY policy_count, c.tablename;
SQL
)

# Postgres' recursive CTE requires the RECURSIVE keyword; emit it correctly.
COVERAGE_SQL="WITH RECURSIVE${COVERAGE_SQL#WITH}"

RESULT=$(psql_run -F'|' -c "$COVERAGE_SQL")

TOTAL=0
UNCOVERED=0
UNCOVERED_LIST=()

while IFS='|' read -r tbl cnt; do
    [[ -z "$tbl" ]] && continue
    TOTAL=$((TOTAL + 1))
    if [[ "$cnt" -eq 0 ]]; then
        UNCOVERED=$((UNCOVERED + 1))
        UNCOVERED_LIST+=("$tbl")
    fi
done <<< "$RESULT"

echo "📋 Tenant-scoped tables evaluated: $TOTAL  (exempt: ${#EXEMPT_TABLES[@]})"
echo ""

if [[ $UNCOVERED -gt 0 ]]; then
    echo -e "${RED}✗ $UNCOVERED tenant-scoped table(s) have NO RLS policy:${NC}"
    for t in "${UNCOVERED_LIST[@]}"; do
        echo -e "  ${RED}MISSING POLICY${NC}  $t"
    done
    echo ""
    echo "Every tenant-scoped table must have at least one CREATE POLICY"
    echo "enforcing organization isolation. Add the policy in a migration, or"
    echo "— if the table is genuinely not tenant-scoped — add it to"
    echo "EXEMPT_TABLES in this script (and validate_rls_coverage() in 00006)."
else
    echo -e "${GREEN}✓ All $TOTAL tenant-scoped tables have at least one RLS policy.${NC}"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [[ $UNCOVERED -gt 0 ]]; then
    if $STRICT_MODE; then
        echo -e "${RED}Failing in strict mode.${NC}"
        exit 1
    else
        echo -e "${YELLOW}Run with --strict to fail CI on uncovered tables.${NC}"
        exit 0
    fi
fi
exit 0
