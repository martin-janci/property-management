-- Epic 15: Fix per-listing analytics reads for portal-owned listings (#2199).
--
-- Background
-- ----------
-- PR #2183 shipped `GET /api/v1/my/listings/{id}/analytics`, whose read path
-- calls `RealityPortalRepository::get_listing_analytics`, a plain
--   SELECT * FROM listing_analytics WHERE listing_id = $1 ...
-- on the RLS-subject reality-server pool. `listing_analytics` is
-- `FORCE ROW LEVEL SECURITY` (migration 00113) and its only policy,
-- `listing_analytics_tenant_isolation` (00113 / 00140), grants rows solely via:
--
--     is_super_admin()
--     OR EXISTS (SELECT 1 FROM listings p
--                WHERE p.id = listing_analytics.listing_id
--                  AND p.organization_id = get_current_org_id())
--
-- Portal-owned listings have `organization_id = NULL` (migration 00186 moved
-- ownership to `portal_owner_id`) and there is **no portal-owner branch** on the
-- analytics policy. So for every portal listing the `EXISTS` is false and the
-- reality-server role (RLS-subject by design) reads zero analytics rows — the
-- endpoint returns a zeroed summary and an empty daily series for every
-- portal-owned listing, even one with recorded views.
--
-- Fix
-- ---
-- Mirror the existing portal SECURITY DEFINER pattern (migration 00186:
-- `portal_get_listing` / `portal_create_listing` / `portal_update_listing`).
-- Add `portal_get_listing_analytics(...)`, a SECURITY DEFINER function that is
-- ownership-gated on `portal_owner_id = p_user_id OR created_by = p_user_id`
-- (identical to `portal_get_listing`) and returns the `listing_analytics` rows
-- for the owned listing within the optional date window. Running as the definer
-- (a superuser, like the other portal_* functions) bypasses the org-scoped
-- `listing_analytics` RLS policy while the in-function ownership check keeps the
-- read scoped to the caller's own listing — never cross-user.

CREATE OR REPLACE FUNCTION portal_get_listing_analytics(
    p_listing_id UUID,
    p_user_id    UUID,
    p_from       DATE DEFAULT NULL,
    p_to         DATE DEFAULT NULL
)
RETURNS SETOF listing_analytics
LANGUAGE plpgsql
STABLE
SECURITY DEFINER
SET search_path = public
AS $$
BEGIN
    -- Ownership gate: only the portal owner (or original creator, to cover
    -- listings migrated before 00186) may read their listing's analytics.
    -- When the listing is not owned by p_user_id we return no rows, exactly
    -- like the direct RLS-filtered SELECT would for a non-owner.
    IF NOT EXISTS (
        SELECT 1 FROM listings l
        WHERE l.id = p_listing_id
          AND (l.portal_owner_id = p_user_id OR l.created_by = p_user_id)
    ) THEN
        RETURN;
    END IF;

    RETURN QUERY
    SELECT * FROM listing_analytics la
    WHERE la.listing_id = p_listing_id
      AND (p_from IS NULL OR la.date >= p_from)
      AND (p_to   IS NULL OR la.date <= p_to)
    ORDER BY la.date DESC;
END;
$$;

COMMENT ON FUNCTION portal_get_listing_analytics IS
    'Return listing_analytics rows for a portal-user-owned listing, gated on '
    'portal_owner_id = p_user_id OR created_by = p_user_id (mirrors '
    'portal_get_listing, migration 00186). SECURITY DEFINER bypasses the '
    'org-scoped listing_analytics RLS policy, which has no portal-owner branch, '
    'so portal listings no longer read back an empty analytics series (#2199).';
