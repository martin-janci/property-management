#!/bin/bash
# =============================================================================
# manifest-fresh CI lint  (Phase 5.5 — Tenant Lifecycle & Operability)
# =============================================================================
# Defense for brainstorming leak #17 ("GDPR purge incomplete: tenant data
# silently left behind in tables that the purge routine never knew about").
#
# The tenant-data manifest at backend/manifests/tenant-data-manifest.json is
# the SINGLE source of truth for which tables hold tenant-scoped rows. Every
# Phase 5.5 export/purge/restore plan walks it. If a developer adds a
# migration that introduces a new tenant-scoped table without regenerating
# (and committing) the manifest, the purge silently leaves that table behind.
#
# This grep-based gate is intentionally simple:
#   * Look at the migration count baked into the manifest
#     (manifest.migration_count).
#   * Compare against the number of *.sql files in the migrations directory.
#   * Fail if migrations got ahead of the manifest.
#
# The full source-of-truth regeneration requires Postgres — see
# `backend/scripts/check-rls-coverage.sh --emit-manifest`. This lint just
# catches the "I forgot to regenerate" case at PR time.
#
# Usage: ./scripts/lints/manifest-fresh.sh
#   exits 0 if manifest is fresh, 1 if it's behind the migrations.
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

MANIFEST="$BACKEND_DIR/manifests/tenant-data-manifest.json"
MIGRATIONS_DIR="$BACKEND_DIR/crates/db/migrations"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "🔒 manifest-fresh lint (Phase 5.5 leak #17 defense)"
echo ""

if [[ ! -f "$MANIFEST" ]]; then
    echo -e "${RED}✗ Manifest not found at $MANIFEST${NC}"
    echo "  Generate it via: bash backend/scripts/check-rls-coverage.sh --emit-manifest $MANIFEST"
    exit 1
fi

if [[ ! -d "$MIGRATIONS_DIR" ]]; then
    echo -e "${RED}✗ Migrations directory not found at $MIGRATIONS_DIR${NC}"
    exit 1
fi

# Pull the manifest's migration_count without requiring jq (which isn't
# guaranteed in CI minimal images). The line shape is:
#   "migration_count": 140,
MANIFEST_COUNT=$(grep -oE '"migration_count":[[:space:]]*[0-9]+' "$MANIFEST" \
                 | head -1 \
                 | grep -oE '[0-9]+' \
                 | head -1 || echo "0")

if [[ -z "$MANIFEST_COUNT" || "$MANIFEST_COUNT" == "0" ]]; then
    echo -e "${RED}✗ Could not read migration_count from manifest${NC}"
    exit 1
fi

# Latest migration number = the numeric prefix of the highest-sorted *.sql
# file. Migration filenames are zero-padded (00140_*.sql), so lexical sort =
# numeric sort. We use this rather than a raw file count so renames /
# squashes / accidental gaps don't silently drift the gate.
LATEST_MIGRATION=$(ls "$MIGRATIONS_DIR"/*.sql 2>/dev/null \
                   | sort \
                   | tail -1 \
                   | xargs -n1 basename \
                   | grep -oE '^[0-9]+' \
                   | sed 's/^0*//' \
                   || echo "0")

if [[ -z "$LATEST_MIGRATION" || "$LATEST_MIGRATION" == "0" ]]; then
    echo -e "${RED}✗ Could not find any migrations in $MIGRATIONS_DIR${NC}"
    exit 1
fi

echo "  Manifest migration_count : $MANIFEST_COUNT"
echo "  Latest migration number  : $LATEST_MIGRATION"
echo ""

if [[ "$MANIFEST_COUNT" -lt "$LATEST_MIGRATION" ]]; then
    echo -e "${RED}✗ Manifest is stale.${NC}"
    echo ""
    echo "  The manifest was generated at migration_count=$MANIFEST_COUNT but"
    echo "  the migrations directory now contains a migration numbered"
    echo "  $LATEST_MIGRATION. Any tenant-scoped table introduced in the gap is"
    echo "  invisible to the Phase 5.5 export/purge plans — leak #17 risk."
    echo ""
    echo "  Regenerate the manifest:"
    echo "    bash backend/scripts/check-rls-coverage.sh \\"
    echo "      --emit-manifest backend/manifests/tenant-data-manifest.json"
    echo ""
    echo "  Then commit the updated JSON alongside your migration."
    exit 1
fi

if [[ "$MANIFEST_COUNT" -gt "$LATEST_MIGRATION" ]]; then
    echo -e "${YELLOW}⚠ Manifest claims more migrations ($MANIFEST_COUNT) than exist ($LATEST_MIGRATION).${NC}"
    echo "  This usually means a migration was deleted after the manifest was"
    echo "  generated. Regenerate the manifest to keep them in sync."
    exit 1
fi

echo -e "${GREEN}✓ Manifest is fresh (covers all $LATEST_MIGRATION migrations).${NC}"
exit 0
