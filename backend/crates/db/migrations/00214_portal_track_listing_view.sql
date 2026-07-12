-- Epic 15 follow-up: fix the per-listing analytics **write** path for
-- portal-owned listings (#2244, the write-path counterpart of #2199 / #2218).
--
-- Background
-- ----------
-- `track_listing_view` (migration 00063) is `LANGUAGE plpgsql` with no security
-- clause, i.e. **SECURITY INVOKER**. It `INSERT … ON CONFLICT DO UPDATE`s into
-- `listing_analytics`, which is `FORCE ROW LEVEL SECURITY` (migration 00113)
-- whose only policy is org-scoped:
--
--     is_super_admin()
--     OR EXISTS (SELECT 1 FROM listings p
--                WHERE p.id = listing_analytics.listing_id
--                  AND p.organization_id = get_current_org_id())
--
-- with **no portal-owner branch**. Portal listings carry `organization_id = NULL`
-- (migration 00186 moved ownership to `portal_owner_id`), and the public
-- reality-server pool is RLS-subject with no org context. So the write's
-- `WITH CHECK` fails for portal listings exactly as the read's `USING` did
-- (#2199): the view increment errors out (swallowed as best-effort by the
-- route), the `listing_analytics` table stays empty for portal listings, and
-- `GET /api/v1/my/listings/{id}/analytics` keeps returning a zeroed summary
-- end-to-end even after the #2218 read fix. Net: reads were fixed but no data
-- is ever recorded.
--
-- Fix
-- ---
-- Mirror the portal SECURITY DEFINER pattern (migration 00186 / 00213). Add
-- `portal_track_listing_view(p_listing_id, p_source)`, a SECURITY DEFINER
-- function with the same INSERT/UPSERT body as `track_listing_view` but running
-- as the definer (a superuser), so it bypasses the org-scoped
-- `listing_analytics` RLS policy and records the view for portal listings.
--
-- No ownership gate is applied (unlike the read function 00213): recording a
-- view is an inherently public, unauthenticated action — any visitor of a
-- public listing legitimately increments its view counter, and the function
-- only ever touches the aggregate counters of the listing being viewed. There
-- is no cross-tenant data exposure: the function neither returns rows nor lets
-- the caller read another tenant's data.

CREATE OR REPLACE FUNCTION portal_track_listing_view(
    p_listing_id UUID,
    p_source     VARCHAR DEFAULT 'website'
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
BEGIN
    INSERT INTO listing_analytics (listing_id, date, views, source_website, source_mobile, source_search, source_direct)
    VALUES (
        p_listing_id,
        CURRENT_DATE,
        1,
        CASE WHEN p_source = 'website' THEN 1 ELSE 0 END,
        CASE WHEN p_source = 'mobile' THEN 1 ELSE 0 END,
        CASE WHEN p_source = 'search' THEN 1 ELSE 0 END,
        CASE WHEN p_source = 'direct' THEN 1 ELSE 0 END
    )
    ON CONFLICT (listing_id, date) DO UPDATE SET
        views = listing_analytics.views + 1,
        source_website = listing_analytics.source_website + CASE WHEN p_source = 'website' THEN 1 ELSE 0 END,
        source_mobile = listing_analytics.source_mobile + CASE WHEN p_source = 'mobile' THEN 1 ELSE 0 END,
        source_search = listing_analytics.source_search + CASE WHEN p_source = 'search' THEN 1 ELSE 0 END,
        source_direct = listing_analytics.source_direct + CASE WHEN p_source = 'direct' THEN 1 ELSE 0 END;
END;
$$;

COMMENT ON FUNCTION portal_track_listing_view IS
    'Record a listing view (INSERT/UPSERT into listing_analytics) for a portal '
    'listing. SECURITY DEFINER bypasses the org-scoped listing_analytics RLS '
    'policy (which has no portal-owner branch), so views on portal listings '
    '(organization_id IS NULL) are actually recorded instead of failing the '
    'FORCE-RLS WITH CHECK. Write-path counterpart of portal_get_listing_analytics '
    '(migration 00213); no ownership gate — view tracking is public (#2244).';
