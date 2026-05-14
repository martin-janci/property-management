-- Phase 0: RLS Foundation Hardening
-- Adds Row-Level Security policies to tenant-scoped tables from migration 00049
-- (Epic 15: Property Listings & Multi-Portal Sync).
--
-- listings is an OWN-ORG table (has organization_id).
-- listing_photos and listing_syndications are CHILD tables of listings.

-- ============================================
-- listings (own-org)
-- ============================================
ALTER TABLE listings ENABLE ROW LEVEL SECURITY;
ALTER TABLE listings FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS listings_tenant_isolation ON listings;
CREATE POLICY listings_tenant_isolation ON listings
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

-- ============================================
-- listing_photos (child of listings, fk listing_id)
-- ============================================
ALTER TABLE listing_photos ENABLE ROW LEVEL SECURITY;
ALTER TABLE listing_photos FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS listing_photos_tenant_isolation ON listing_photos;
CREATE POLICY listing_photos_tenant_isolation ON listing_photos
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM listings p WHERE p.id = listing_photos.listing_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM listings p WHERE p.id = listing_photos.listing_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- listing_syndications (child of listings, fk listing_id)
-- ============================================
ALTER TABLE listing_syndications ENABLE ROW LEVEL SECURITY;
ALTER TABLE listing_syndications FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS listing_syndications_tenant_isolation ON listing_syndications;
CREATE POLICY listing_syndications_tenant_isolation ON listing_syndications
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM listings p WHERE p.id = listing_syndications.listing_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM listings p WHERE p.id = listing_syndications.listing_id AND p.organization_id = get_current_org_id()));
