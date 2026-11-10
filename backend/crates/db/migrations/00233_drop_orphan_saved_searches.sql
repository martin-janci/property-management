-- Drop the orphaned `saved_searches` (int4) table introduced by migration
-- 00046 (`create_smart_search.sql`). Follow-up to #2816 / PR #2815.
--
-- Background
-- ----------
-- Two parallel saved-search implementations existed:
--
--   1. LIVE: `portal_saved_searches` (migration 00063), served by
--      `RealityPortalRepository`, wired into reality-server's
--      `/api/v1/saved-searches` routes and the `SavedSearchAlertWorker`.
--      Its `match_count` was hardened to bigint in migration 00232 (#2814).
--
--   2. ORPHAN (this migration removes it): `saved_searches` (migration 00046),
--      targeted by the `PortalRepository` saved-search cluster
--      (`create_saved_search`, `get_saved_search(es)`, `update_saved_search`,
--      `delete_saved_search`, `count_saved_searches`, `update_match_count`).
--      None of those methods had any caller anywhere in the tree, and the
--      table's actual columns (query/filters/alert_enabled) never even matched
--      the `SavedSearch` model (criteria/alerts_enabled/match_count) the
--      repository bound against — the path was dead on arrival.
--
-- Removing the model + repository methods leaves this table with no reader or
-- writer, so it is dropped here together with the RLS policy and updated-at
-- trigger created for it in 00046. `DROP TABLE` also removes the dependent
-- `idx_saved_searches_user` index.

DROP TRIGGER IF EXISTS update_saved_searches_updated_at ON saved_searches;
DROP POLICY IF EXISTS saved_searches_user_isolation ON saved_searches;
DROP TABLE IF EXISTS saved_searches;
