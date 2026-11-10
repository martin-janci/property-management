-- Epic 15 follow-up: fix the public listing-detail **view-count read** for
-- portal-owned listings (the read-back counterpart of the write fix in
-- migration 00214 / #2244).
--
-- Background
-- ----------
-- `GET /api/v1/listings/{id}` (the public listing-detail handler) records a
-- view via the SECURITY DEFINER `portal_track_listing_view` (00214), so views
-- ARE persisted into `listing_analytics`. But the handler built its response
-- with a hardcoded `view_count: 0` — the read side was never wired up, so the
-- public listing page and analytics under-reported every listing's views to
-- zero.
--
-- A naive read fix (a plain `SELECT SUM(views) FROM listing_analytics` on the
-- reality-server pool) does not work: `listing_analytics` is
-- `FORCE ROW LEVEL SECURITY` (migration 00113) with an org-only policy and no
-- portal-owner branch. Portal listings carry `organization_id = NULL`
-- (migration 00186) and the public reality-server pool is RLS-subject with no
-- org context, so a direct SELECT reads back zero rows for portal listings —
-- exactly the RLS trap documented in 00213 (read) and 00214 (write).
--
-- Fix
-- ---
-- Mirror the portal SECURITY DEFINER pattern (migrations 00186 / 00213 / 00214).
-- Add `portal_get_listing_view_count(p_listing_id)`, a SECURITY DEFINER function
-- that runs as the definer (a superuser) and thus bypasses the org-scoped
-- `listing_analytics` RLS policy, returning the listing's aggregate view total.
--
-- No ownership gate is applied (unlike the per-owner read function 00213):
-- the total view count of a public listing is itself public — any visitor of
-- the public listing-detail page legitimately sees it. The function only ever
-- aggregates the `views` counter of the single requested listing; it exposes no
-- per-tenant rows and lets the caller read no other tenant's data. This matches
-- the public, ownership-free contract of the write path 00214.

CREATE OR REPLACE FUNCTION portal_get_listing_view_count(
    p_listing_id UUID
)
RETURNS BIGINT
LANGUAGE plpgsql
STABLE
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
    v_total BIGINT;
BEGIN
    SELECT COALESCE(SUM(la.views), 0)::bigint
    INTO v_total
    FROM listing_analytics la
    WHERE la.listing_id = p_listing_id;

    RETURN v_total;
END;
$$;

COMMENT ON FUNCTION portal_get_listing_view_count IS
    'Return the aggregate view total for a listing from listing_analytics. '
    'SECURITY DEFINER bypasses the org-scoped listing_analytics RLS policy '
    '(which has no portal-owner branch), so the public listing-detail read '
    'reports real recorded views for portal listings (organization_id IS NULL) '
    'instead of a hardcoded zero. No ownership gate — a public listing''s total '
    'view count is public (read-side counterpart of portal_track_listing_view, '
    'migration 00214).';
