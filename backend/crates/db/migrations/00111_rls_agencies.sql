-- Phase 0: RLS Foundation Hardening
-- Adds Row-Level Security policies to tenant-scoped tables from migration 00050
-- (Epic 17: Agency & Realtor Management).
--
-- agency_listings and listing_edit_history are CHILD tables of listings
-- (both reach the org through listings.organization_id via fk listing_id).

-- ============================================
-- agency_listings (child of listings, fk listing_id)
-- ============================================
ALTER TABLE agency_listings ENABLE ROW LEVEL SECURITY;
ALTER TABLE agency_listings FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS agency_listings_tenant_isolation ON agency_listings;
CREATE POLICY agency_listings_tenant_isolation ON agency_listings
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM listings p WHERE p.id = agency_listings.listing_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM listings p WHERE p.id = agency_listings.listing_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- listing_edit_history (child of listings, fk listing_id)
-- ============================================
ALTER TABLE listing_edit_history ENABLE ROW LEVEL SECURITY;
ALTER TABLE listing_edit_history FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS listing_edit_history_tenant_isolation ON listing_edit_history;
CREATE POLICY listing_edit_history_tenant_isolation ON listing_edit_history
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM listings p WHERE p.id = listing_edit_history.listing_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM listings p WHERE p.id = listing_edit_history.listing_id AND p.organization_id = get_current_org_id()));
