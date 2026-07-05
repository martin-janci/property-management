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
-- `id DESC` as a third index key makes the whole ORDER BY (and therefore the
-- LIMIT/OFFSET slice) index-satisfiable *on the strict-org leg*
-- (`organization_id = $1`, i.e. include_system=false): that query is a single
-- equality on the leading column, so the planner walks the index range in
-- order and the read becomes a pure index range scan with a stable,
-- deterministic order and no top-level sort.
--
-- This does NOT eliminate the sort on the default include_system=true leg.
-- `WHERE (organization_id = $1 OR organization_id IS NULL)
--  ORDER BY updated_at DESC, id DESC` is an OR over two disjoint ranges of the
-- leading index column; Postgres resolves it via BitmapOr -> Bitmap Heap Scan
-- (which discards index order) and then a top-level Sort. Each individual leg
-- (`organization_id = $1` and `organization_id IS NULL`, with NULLs indexed by
-- btree) is an ordered index range this index can serve, but the union of the
-- two still requires a sort unless the query is rewritten as a UNION ALL /
-- MergeAppend of the ordered legs. The `id DESC` tie-break itself remains
-- deterministic on both legs regardless of the access path.
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
