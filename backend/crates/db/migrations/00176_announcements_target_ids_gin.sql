-- Issue #999: index the announcement targeting predicate.
--
-- The published-feed predicate (announcement_targeting_predicate! in
-- announcement.rs, shared by list_published_rls + count_published_rls) matches
-- a candidate row by extracting its `target_ids` JSONB array and testing the
-- RLS-context user's building / unit / role against it via:
--   ... IN (SELECT jsonb_array_elements_text(announcements.target_ids))
--
-- A GIN index on the JSONB column lets the planner use a containment/extraction
-- path instead of a per-row correlated scan as the published set grows. The
-- supporting indexes for the EXISTS sub-lookups already exist:
--   - unit_residents:  idx_unit_residents_user (user_id)            [00009]
--   - user_memberships: idx_user_memberships_active
--                       (user_id, organization_id) WHERE revoked_at IS NULL [00128]

CREATE INDEX IF NOT EXISTS idx_announcements_target_ids_gin
    ON announcements USING GIN (target_ids);
