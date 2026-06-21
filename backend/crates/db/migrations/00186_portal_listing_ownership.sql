-- Epic 15.1/15.2: Portal-user listing ownership (owner/realtor edit flow).
--
-- The `listings` table was originally org-scoped only (organization_id NOT NULL).
-- This migration adds first-class portal-user ownership so authenticated portal
-- users can create and edit their own listings without belonging to a PPT org.
--
-- Changes:
--   1. `organization_id` becomes nullable (NULL = portal-direct listing).
--   2. `portal_owner_id UUID REFERENCES users(id)` added as the portal owner FK.
--   3. An ownership CHECK ensures every row is owned by exactly one of the two paths.
--   4. SECURITY DEFINER functions let portal routes write without an RLS org context:
--        portal_create_listing(...)  — INSERT with portal_owner_id
--        portal_update_listing(...)  — ownership-gated UPDATE
--        portal_get_listing(...)     — ownership-gated SELECT
--   5. The existing `listings_four_context` RLS policy is extended to cover portal-
--      owner reads/writes.

-- ===========================================================================
-- 1. Make organization_id nullable for portal-direct listings.
-- ===========================================================================
ALTER TABLE listings ALTER COLUMN organization_id DROP NOT NULL;

-- ===========================================================================
-- 2. Add portal_owner_id.
-- ===========================================================================
ALTER TABLE listings ADD COLUMN IF NOT EXISTS portal_owner_id UUID
    REFERENCES users(id) ON DELETE CASCADE;

CREATE INDEX IF NOT EXISTS idx_listings_portal_owner
    ON listings(portal_owner_id)
    WHERE portal_owner_id IS NOT NULL;

-- ===========================================================================
-- 3. Ownership invariant.
-- ===========================================================================
ALTER TABLE listings ADD CONSTRAINT listings_owner_invariant
    CHECK (organization_id IS NOT NULL OR portal_owner_id IS NOT NULL);

-- ===========================================================================
-- 4a. Helper: get_current_portal_user_id()
--     Reads app.current_user_id set by portal routes.
-- ===========================================================================
CREATE OR REPLACE FUNCTION get_current_portal_user_id()
RETURNS UUID
LANGUAGE plpgsql
STABLE
AS $$
BEGIN
    RETURN NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID;
EXCEPTION
    WHEN OTHERS THEN
        RETURN NULL;
END;
$$;

-- ===========================================================================
-- 4b. Update listings RLS to cover portal-owner reads and writes.
--
-- Policy becomes 5 contexts:
--   1. Super-admin            — is_super_admin()
--   2. Org-staff write        — organization_id = get_current_org_id()
--   3. Agency-subdomain       — same predicate (app-layer differentiates)
--   4. Global-read (portal)   — is_published AND is_global_read_context()
--   5. Portal-owner (new)     — portal_owner_id = get_current_portal_user_id()
-- ===========================================================================
DROP POLICY IF EXISTS listings_four_context ON listings;
DROP POLICY IF EXISTS listings_five_context ON listings;

CREATE POLICY listings_five_context ON listings
    FOR ALL
    USING (
        is_super_admin()
        OR organization_id = get_current_org_id()
        OR (is_published AND is_global_read_context())
        OR portal_owner_id = get_current_portal_user_id()
    )
    WITH CHECK (
        is_super_admin()
        OR organization_id = get_current_org_id()
        OR portal_owner_id = get_current_portal_user_id()
    );

COMMENT ON POLICY listings_five_context ON listings IS
    'Five-context RLS: (1) super-admin, (2/3) org-staff and agency subdomain '
    'gated by get_current_org_id(), (4) PlatformHost global read-only of '
    'is_published rows, (5) portal-owner CRUD gated by portal_owner_id = '
    'get_current_portal_user_id(). WITH CHECK omits global-read (read-only by '
    'design) and adds portal-owner for direct portal-user writes.';

-- listing_photos follows listings via FK; portal users need to manage photos.
-- Replaces the tenant-isolation policy (00110 / 00140) with one that also
-- covers the portal-owner arm and the global-read arm.
DROP POLICY IF EXISTS listing_photos_tenant_isolation ON listing_photos;
CREATE POLICY listing_photos_five_context ON listing_photos
    FOR ALL
    USING (
        is_super_admin()
        OR (
            EXISTS (
                SELECT 1 FROM listings p
                WHERE p.id = listing_photos.listing_id
                  AND p.organization_id = get_current_org_id()
            ) AND get_current_org_not_deleted()
        )
        OR EXISTS (
            SELECT 1 FROM listings p
            WHERE p.id = listing_photos.listing_id
              AND p.is_published AND is_global_read_context()
        )
        OR EXISTS (
            SELECT 1 FROM listings p
            WHERE p.id = listing_photos.listing_id
              AND p.portal_owner_id = get_current_portal_user_id()
        )
    )
    WITH CHECK (
        is_super_admin()
        OR (
            EXISTS (
                SELECT 1 FROM listings p
                WHERE p.id = listing_photos.listing_id
                  AND p.organization_id = get_current_org_id()
            ) AND get_current_org_not_deleted()
        )
        OR EXISTS (
            SELECT 1 FROM listings p
            WHERE p.id = listing_photos.listing_id
              AND p.portal_owner_id = get_current_portal_user_id()
        )
    );

-- ===========================================================================
-- 5a. SECURITY DEFINER: create portal-user listing (bypasses org-scoped RLS).
-- ===========================================================================
CREATE OR REPLACE FUNCTION portal_create_listing(
    p_user_id        UUID,
    p_title          TEXT,
    p_description    TEXT,
    p_property_type  TEXT,
    p_transaction_type TEXT,
    p_price          NUMERIC,
    p_currency       TEXT,
    p_street         TEXT,
    p_city           TEXT,
    p_postal_code    TEXT,
    p_country        TEXT    DEFAULT 'SK',
    p_size_sqm       NUMERIC DEFAULT NULL,
    p_rooms          INTEGER DEFAULT NULL,
    p_floor          INTEGER DEFAULT NULL,
    p_total_floors   INTEGER DEFAULT NULL
)
RETURNS SETOF listings
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
BEGIN
    RETURN QUERY
    INSERT INTO listings (
        portal_owner_id, created_by,
        status, is_published, title, description, property_type, transaction_type,
        price, currency, street, city, postal_code, country,
        size_sqm, rooms, floor, total_floors, features
    )
    VALUES (
        p_user_id, p_user_id,
        'draft', FALSE, p_title, p_description, p_property_type, p_transaction_type,
        p_price, p_currency, p_street, p_city, p_postal_code, p_country,
        p_size_sqm, p_rooms, p_floor, p_total_floors, '[]'::jsonb
    )
    RETURNING *;
END;
$$;

COMMENT ON FUNCTION portal_create_listing IS
    'Create a portal-user-owned listing (no PPT org required). Ownership is '
    'enforced via portal_owner_id = p_user_id; status starts as draft.';

-- ===========================================================================
-- 5b. SECURITY DEFINER: get portal-user owned listing for editing.
-- ===========================================================================
CREATE OR REPLACE FUNCTION portal_get_listing(
    p_listing_id UUID,
    p_user_id    UUID
)
RETURNS SETOF listings
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
BEGIN
    RETURN QUERY
    SELECT * FROM listings
    WHERE id = p_listing_id
      AND (portal_owner_id = p_user_id OR created_by = p_user_id);
END;
$$;

COMMENT ON FUNCTION portal_get_listing IS
    'Return a listing owned by p_user_id for editing. Checks both portal_owner_id '
    'and created_by to cover listings migrated before this schema change.';

-- ===========================================================================
-- 5c. SECURITY DEFINER: update portal-user owned listing.
-- ===========================================================================
CREATE OR REPLACE FUNCTION portal_update_listing(
    p_listing_id       UUID,
    p_user_id          UUID,
    p_title            TEXT    DEFAULT NULL,
    p_description      TEXT    DEFAULT NULL,
    p_property_type    TEXT    DEFAULT NULL,
    p_transaction_type TEXT    DEFAULT NULL,
    p_price            NUMERIC DEFAULT NULL,
    p_currency         TEXT    DEFAULT NULL,
    p_street           TEXT    DEFAULT NULL,
    p_city             TEXT    DEFAULT NULL,
    p_postal_code      TEXT    DEFAULT NULL,
    p_country          TEXT    DEFAULT NULL,
    p_size_sqm         NUMERIC DEFAULT NULL,
    p_rooms            INTEGER DEFAULT NULL,
    p_floor            INTEGER DEFAULT NULL,
    p_total_floors     INTEGER DEFAULT NULL,
    p_status           TEXT    DEFAULT NULL,
    p_is_negotiable    BOOLEAN DEFAULT NULL
)
RETURNS SETOF listings
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
BEGIN
    RETURN QUERY
    UPDATE listings SET
        title            = COALESCE(p_title, title),
        description      = COALESCE(p_description, description),
        property_type    = COALESCE(p_property_type, property_type),
        transaction_type = COALESCE(p_transaction_type, transaction_type),
        price            = COALESCE(p_price, price),
        currency         = COALESCE(p_currency, currency),
        street           = COALESCE(p_street, street),
        city             = COALESCE(p_city, city),
        postal_code      = COALESCE(p_postal_code, postal_code),
        country          = COALESCE(p_country, country),
        size_sqm         = COALESCE(p_size_sqm, size_sqm),
        rooms            = COALESCE(p_rooms, rooms),
        floor            = COALESCE(p_floor, floor),
        total_floors     = COALESCE(p_total_floors, total_floors),
        status           = COALESCE(p_status, status),
        is_negotiable    = COALESCE(p_is_negotiable, is_negotiable)
    WHERE id = p_listing_id
      AND (portal_owner_id = p_user_id OR created_by = p_user_id)
    RETURNING *;
END;
$$;

COMMENT ON FUNCTION portal_update_listing IS
    'Patch a portal-user-owned listing. NULL arguments are skipped (COALESCE). '
    'Ownership enforced via portal_owner_id = p_user_id OR created_by = p_user_id.';
