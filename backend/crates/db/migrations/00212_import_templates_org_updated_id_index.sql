-- Migration: 00212_import_templates_org_updated_id_index
-- Tighten the list_templates pagination index so the full ORDER BY tie-break
-- is index-satisfiable, and drop the now-redundant single-column index
-- (#2051, follow-up to PR #2038 / #1994).
--
-- MigrationRepository::list_templates pages with:
--   SELECT * FROM import_templates
--   WHERE (organization_id = $1 [OR organization_id IS NULL]) [AND data_type = $2]
--   ORDER BY updated_at DESC, id DESC
--   LIMIT $ OFFSET $
--
-- Migration 00211 added idx_import_templates_org_updated on
-- (organization_id, updated_at DESC). That covers the org filter and the
-- leading sort key, but NOT the `id DESC` tie-break: Postgres can range-scan
-- the index for the first two keys, yet rows sharing an `updated_at` value
-- still need an extra in-memory sort to resolve the `id DESC` order. Adding
-- `id DESC` as a third index key makes the entire ORDER BY (and therefore the
-- LIMIT/OFFSET slice) satisfiable directly from the index, so paginated reads
-- become a pure index range scan with a stable, deterministic order.
--
-- The system-template leg of the OR (organization_id IS NULL) is still served
-- by this index: NULLs are indexed by btree, and the leading key plus the two
-- sort keys match the query's ordering for that branch too.
CREATE INDEX IF NOT EXISTS idx_import_templates_org_updated_id
    ON import_templates (organization_id, updated_at DESC, id DESC);

-- Replace the (organization_id, updated_at DESC) index from 00211: the new
-- three-column index is a strict superset (same leading keys), so keeping both
-- only wastes write bandwidth and planner consideration.
DROP INDEX IF EXISTS idx_import_templates_org_updated;

-- Drop the original single-column index from 00198. The composite index above
-- has organization_id as its leading column, so it already serves every
-- equality lookup on organization_id that idx_import_templates_organization_id
-- used to cover. Keeping it is pure redundancy (extra index maintenance on
-- every insert/update/delete for no read benefit).
DROP INDEX IF EXISTS idx_import_templates_organization_id;
