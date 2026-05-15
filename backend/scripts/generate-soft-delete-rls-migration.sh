#!/bin/bash
# =============================================================================
# Generate migration 00140_rls_soft_delete_filter.sql from the tenant-data
# manifest produced by check-rls-coverage.sh --emit-manifest.
#
# Phase 5.5 — Tenant Lifecycle & Operability.
#
# Strategy
# --------
# 1. Apply all migrations to an ephemeral Postgres (or DATABASE_URL).
# 2. Introspect pg_policies for every tenant-scoped table from the manifest,
#    pulling out the existing policy name + qual + with_check expressions.
# 3. Emit a DROP POLICY + CREATE POLICY per table, AND-ing
#    `get_current_org_not_deleted()` into the existing predicates so that
#    soft-deleted orgs disappear from tenant queries (super-admin still sees
#    them).
# 4. Prepend the helper function that reads the live `organizations.deleted_at`
#    flag for the current tenant.
#
# This is invasive (one ALTER per table × 360+ tables) but mechanical — the
# script *is* the migration spec. Re-run after a schema change to regenerate.
#
# Usage:
#   bash backend/scripts/generate-soft-delete-rls-migration.sh \
#     [--manifest backend/manifests/tenant-data-manifest.json] \
#     [--out backend/crates/db/migrations/00140_rls_soft_delete_filter.sql]
#
# ============================================================================
# GOTCHA — multi-context policies (e.g. `listings_four_context` from 00136)
# ============================================================================
# This generator works PER POLICY NAME: it pulls the policy currently attached
# to the table from `pg_policies` and emits a DROP/CREATE for each name. If
# the live schema has already been upgraded to a richer multi-context policy
# (for example, Phase 4's `listings_four_context` replaces the legacy
# `listings_tenant_isolation`), this script will:
#
#   (a) Faithfully rewrite that multi-context policy with the soft-delete
#       filter AND-ed in — IF it runs against a base that already includes
#       the upgrade migration. Good.
#   (b) Quietly miss the upgrade and re-emit the LEGACY 2-context policy
#       name — IF it runs against a base that pre-dates the upgrade
#       migration. Bad: Postgres ORs PERMISSIVE policies, so the resulting
#       table ends up with BOTH policies and the soft-delete filter is
#       bypassed via the still-present multi-context policy that this script
#       never touched.
#
# Mitigation:
#   * The TENANT_TABLES_SKIP allowlist below is the single source of truth
#     for tables whose policies are managed elsewhere and MUST NOT be
#     rewritten by this script. Add a table here if you upgrade its policy
#     to a multi-context shape that this generator can't faithfully diff.
#     The integration fix migration (00141) takes care of `listings`
#     post-hoc; a future regeneration won't reintroduce the regression
#     because `listings` is on the skip list.
#   * Anyone regenerating 00140 against a fresh schema MUST also re-run
#     `check-rls-coverage.sh --emit-manifest` first so the manifest is
#     fresh. The `manifest-fresh.sh` lint catches the common skew.
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
MIGRATIONS_DIR="$BACKEND_DIR/crates/db/migrations"
INIT_SQL="$SCRIPT_DIR/init-db.sql"

MANIFEST="$BACKEND_DIR/manifests/tenant-data-manifest.json"
OUT="$MIGRATIONS_DIR/00140_rls_soft_delete_filter.sql"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --manifest) MANIFEST="$2"; shift 2 ;;
        --out)      OUT="$2"; shift 2 ;;
        *) echo "Unknown arg $1" >&2; exit 2 ;;
    esac
done

if [[ ! -f "$MANIFEST" ]]; then
    echo "✗ Manifest not found at $MANIFEST" >&2
    echo "  Run: bash backend/scripts/check-rls-coverage.sh --emit-manifest $MANIFEST" >&2
    exit 2
fi

# ---- Boot Postgres (same dance as check-rls-coverage.sh) --------------------
PSQL_BASE=()
CLEANUP_CONTAINER=""
cleanup() {
    if [[ -n "$CLEANUP_CONTAINER" ]]; then
        docker rm -f "$CLEANUP_CONTAINER" >/dev/null 2>&1 || true
    fi
}
trap cleanup EXIT

if [[ -n "${DATABASE_URL:-}" ]] && command -v psql >/dev/null 2>&1 \
        && psql "$DATABASE_URL" -c 'SELECT 1' >/dev/null 2>&1; then
    PSQL_BASE=(psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -qtA)
    echo "  Using DATABASE_URL."
else
    command -v docker >/dev/null 2>&1 || { echo "✗ docker required" >&2; exit 2; }
    CLEANUP_CONTAINER="rls-soft-delete-gen-$$"
    echo "  Booting ephemeral Postgres as $CLEANUP_CONTAINER ..."
    docker run -d --rm --name "$CLEANUP_CONTAINER" \
        -e POSTGRES_USER=postgres -e POSTGRES_PASSWORD=postgres \
        -e POSTGRES_DB=rls_softdel postgres:16 >/dev/null
    tries=0
    until docker exec "$CLEANUP_CONTAINER" \
            psql -U postgres -d rls_softdel -c 'SELECT 1' >/dev/null 2>&1; do
        tries=$((tries + 1))
        [[ $tries -gt 60 ]] && { echo "✗ Postgres not ready" >&2; exit 2; }
        sleep 1
    done
    PSQL_BASE=(docker exec -i "$CLEANUP_CONTAINER" psql -U postgres -d rls_softdel \
               -v ON_ERROR_STOP=1 -qtA)
fi

echo "  Applying init-db.sql + $(ls "$MIGRATIONS_DIR"/*.sql | wc -l | tr -d ' ') migrations ..."
[[ -f "$INIT_SQL" ]] && "${PSQL_BASE[@]}" -f - < "$INIT_SQL" >/dev/null

# Apply every migration EXCEPT 00140 itself (we're regenerating it).
for mig in $(ls "$MIGRATIONS_DIR"/*.sql | sort); do
    if [[ "$(basename "$mig")" == "00140_rls_soft_delete_filter.sql" ]]; then
        continue
    fi
    "${PSQL_BASE[@]}" -f - < "$mig" >/dev/null
done

# ---- Tables whose policies must NOT be rewritten by this generator ---------
# See the GOTCHA comment at the top of the file. Each entry is a table whose
# RLS policy uses a multi-context shape (more than the standard 2-context
# `is_super_admin() OR organization_id = get_current_org_id()` pattern) and
# is therefore managed by a hand-authored migration. Skipping them here keeps
# this generator from reintroducing the legacy 2-context policy name and
# leaving Postgres to OR the two PERMISSIVE policies together.
TENANT_TABLES_SKIP=(
    # Phase 4: `listings_four_context` adds a 4th OR-leg
    # (`is_published AND is_global_read_context()`) for the unauthenticated
    # PlatformHost reader. The soft-delete filter is applied by 00141.
    listings
)

is_skipped() {
    local tbl="$1"
    for skip in "${TENANT_TABLES_SKIP[@]}"; do
        if [[ "$tbl" == "$skip" ]]; then
            return 0
        fi
    done
    return 1
}

# ---- Extract tenant tables from manifest -----------------------------------
# Use grep+sed since jq isn't guaranteed in CI.
TABLES=$(grep -oE '"table": "[^"]+"' "$MANIFEST" | sed 's/.*"table": "\([^"]*\)"/\1/' | sort -u)

# ---- Emit the migration ----------------------------------------------------
{
    cat <<'HEADER'
-- ============================================================================
-- Phase 5.5: Tenant Lifecycle & Operability
-- Migration 00140 — soft-delete filter on every tenant-scoped RLS policy.
-- ============================================================================
-- GENERATED FILE — do not hand-edit. Regenerate with:
--   bash backend/scripts/generate-soft-delete-rls-migration.sh
--
-- Defense for brainstorming leak #16: once an organization is soft-deleted
-- (organizations.deleted_at IS NOT NULL), every tenant context sees zero
-- rows from that org via the RLS policy below. Super-admin context still
-- sees them — so a soft-deleted tenant can be inspected and restored.
--
-- The helper reads the LIVE organizations.deleted_at flag (no caching) —
-- because RLS evaluates the policy expression on every query, an admin
-- restore is visible to subsequent tenant queries within the same session.
-- ============================================================================

-- ----------------------------------------------------------------------------
-- Helper: returns TRUE iff the current org context is alive (or there is
-- no org context — bypass paths like /tenant-config still work).
-- ----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION get_current_org_not_deleted() RETURNS BOOLEAN AS $$
DECLARE
    org_id_val UUID;
BEGIN
    org_id_val := get_current_org_id();
    IF org_id_val IS NULL THEN
        RETURN TRUE;
    END IF;
    RETURN NOT EXISTS (
        SELECT 1 FROM organizations
        WHERE id = org_id_val
          AND deleted_at IS NOT NULL
    );
END;
$$ LANGUAGE plpgsql STABLE;

COMMENT ON FUNCTION get_current_org_not_deleted() IS
    'Phase 5.5: returns TRUE if the request''s tenant org is not soft-deleted. '
    'Used as an AND filter on every tenant-scoped RLS policy. Super-admin paths '
    'bypass this via the is_super_admin() OR-leg in the same policies.';

HEADER

    # For every tenant table in the manifest, look up its existing policies
    # via pg_policies and emit DROP/CREATE pairs that AND in the helper.
    for tbl in $TABLES; do
        # Skip tables whose policies are multi-context and managed elsewhere
        # (see the GOTCHA at the top of this file).
        if is_skipped "$tbl"; then
            echo ""
            echo "-- ----------------------------------------------------------------------------"
            echo "-- $tbl (SKIPPED — multi-context policy managed by a hand-authored migration)"
            echo "-- ----------------------------------------------------------------------------"
            continue
        fi
        # Pull (policyname, cmd, qual, with_check) for every policy on this
        # table. We collapse newlines and tabs in qual/with_check to single
        # spaces so each record fits on one line for the bash parser. The
        # final SQL is still well-formed because Postgres treats whitespace
        # uniformly inside expressions.
        rows=$("${PSQL_BASE[@]}" -F '|' -c "
            SELECT policyname,
                   cmd,
                   COALESCE(regexp_replace(qual, '[[:space:]]+', ' ', 'g'), ''),
                   COALESCE(regexp_replace(with_check, '[[:space:]]+', ' ', 'g'), '')
            FROM pg_policies
            WHERE schemaname = 'public'
              AND tablename = '$tbl'
            ORDER BY policyname;
        " 2>/dev/null) || rows=""

        [[ -z "$rows" ]] && continue

        echo ""
        echo "-- ----------------------------------------------------------------------------"
        echo "-- $tbl"
        echo "-- ----------------------------------------------------------------------------"
        while IFS='|' read -r polname cmd qual wcheck; do
            [[ -z "$polname" ]] && continue
            echo "DROP POLICY IF EXISTS $polname ON $tbl;"

            # Add the soft-delete filter to whichever predicates exist.
            new_qual=""
            if [[ -n "$qual" ]]; then
                new_qual="($qual) AND get_current_org_not_deleted()"
            fi
            new_wcheck=""
            if [[ -n "$wcheck" ]]; then
                new_wcheck="($wcheck) AND get_current_org_not_deleted()"
            fi

            # Build the CREATE POLICY statement. cmd is one of
            # ALL/SELECT/INSERT/UPDATE/DELETE.
            line="CREATE POLICY $polname ON $tbl"
            line="$line FOR $cmd"
            if [[ -n "$new_qual" ]]; then
                line="$line USING ($new_qual)"
            fi
            if [[ -n "$new_wcheck" ]]; then
                line="$line WITH CHECK ($new_wcheck)"
            fi
            line="$line;"
            echo "$line"
        done <<< "$rows"
    done

    cat <<'FOOTER'

-- ============================================================================
-- End of generated soft-delete RLS update.
-- ============================================================================
FOOTER

} > "$OUT"

n_tables=$(grep -c '^-- ----' "$OUT" || echo 0)
echo "✓ Wrote $OUT ($(wc -l < "$OUT") lines, ~$((n_tables - 1)) tables touched)"
