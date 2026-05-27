-- Story 6.4: Pinned announcements — optimise pinned-first sort query.
--
-- The list_published_rls and list_rls queries sort by:
--   ORDER BY pinned DESC, published_at DESC / COALESCE(published_at, created_at) DESC
--
-- The existing partial index (idx_announcements_pinned) covers only the case
-- WHERE pinned = TRUE and does not help the ORDER BY on the full result set.
-- This new composite index covers the common "list published announcements
-- for an org, pinned ones first" pattern used by every announcement feed.

CREATE INDEX IF NOT EXISTS idx_announcements_org_pinned_published
    ON announcements (organization_id, pinned DESC, published_at DESC)
    WHERE status = 'published';
