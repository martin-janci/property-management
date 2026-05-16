-- Phase 0: RLS Foundation Hardening
-- Adds Row-Level Security policies to tenant-scoped tables from migration 00063
-- (Epic 31-34: Reality Portal Professional).
--
-- Direct children of listings (fk listing_id):
--   portal_favorites, listing_price_history, realtor_listings,
--   listing_inquiries, listing_analytics, external_listing_ids
-- Children of listing_inquiries (fk inquiry_id) -- listing_inquiries itself has
-- no organization_id, so these need a two-hop reach to listings.organization_id:
--   inquiry_messages, viewing_schedules

-- ============================================
-- portal_favorites (child of listings, fk listing_id)
-- ============================================
ALTER TABLE portal_favorites ENABLE ROW LEVEL SECURITY;
ALTER TABLE portal_favorites FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS portal_favorites_tenant_isolation ON portal_favorites;
CREATE POLICY portal_favorites_tenant_isolation ON portal_favorites
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM listings p WHERE p.id = portal_favorites.listing_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM listings p WHERE p.id = portal_favorites.listing_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- listing_price_history (child of listings, fk listing_id)
-- ============================================
ALTER TABLE listing_price_history ENABLE ROW LEVEL SECURITY;
ALTER TABLE listing_price_history FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS listing_price_history_tenant_isolation ON listing_price_history;
CREATE POLICY listing_price_history_tenant_isolation ON listing_price_history
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM listings p WHERE p.id = listing_price_history.listing_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM listings p WHERE p.id = listing_price_history.listing_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- realtor_listings (child of listings, fk listing_id)
-- ============================================
ALTER TABLE realtor_listings ENABLE ROW LEVEL SECURITY;
ALTER TABLE realtor_listings FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS realtor_listings_tenant_isolation ON realtor_listings;
CREATE POLICY realtor_listings_tenant_isolation ON realtor_listings
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM listings p WHERE p.id = realtor_listings.listing_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM listings p WHERE p.id = realtor_listings.listing_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- listing_inquiries (child of listings, fk listing_id)
-- ============================================
ALTER TABLE listing_inquiries ENABLE ROW LEVEL SECURITY;
ALTER TABLE listing_inquiries FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS listing_inquiries_tenant_isolation ON listing_inquiries;
CREATE POLICY listing_inquiries_tenant_isolation ON listing_inquiries
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM listings p WHERE p.id = listing_inquiries.listing_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM listings p WHERE p.id = listing_inquiries.listing_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- listing_analytics (child of listings, fk listing_id)
-- ============================================
ALTER TABLE listing_analytics ENABLE ROW LEVEL SECURITY;
ALTER TABLE listing_analytics FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS listing_analytics_tenant_isolation ON listing_analytics;
CREATE POLICY listing_analytics_tenant_isolation ON listing_analytics
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM listings p WHERE p.id = listing_analytics.listing_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM listings p WHERE p.id = listing_analytics.listing_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- external_listing_ids (child of listings, fk listing_id)
-- ============================================
ALTER TABLE external_listing_ids ENABLE ROW LEVEL SECURITY;
ALTER TABLE external_listing_ids FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS external_listing_ids_tenant_isolation ON external_listing_ids;
CREATE POLICY external_listing_ids_tenant_isolation ON external_listing_ids
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM listings p WHERE p.id = external_listing_ids.listing_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM listings p WHERE p.id = external_listing_ids.listing_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- inquiry_messages (child of listing_inquiries, fk inquiry_id)
-- listing_inquiries has no organization_id; reach listings via two-hop.
-- ============================================
ALTER TABLE inquiry_messages ENABLE ROW LEVEL SECURITY;
ALTER TABLE inquiry_messages FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS inquiry_messages_tenant_isolation ON inquiry_messages;
CREATE POLICY inquiry_messages_tenant_isolation ON inquiry_messages
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM listing_inquiries li
        JOIN listings p ON p.id = li.listing_id
        WHERE li.id = inquiry_messages.inquiry_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM listing_inquiries li
        JOIN listings p ON p.id = li.listing_id
        WHERE li.id = inquiry_messages.inquiry_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- viewing_schedules (child of listing_inquiries, fk inquiry_id)
-- listing_inquiries has no organization_id; reach listings via two-hop.
-- ============================================
ALTER TABLE viewing_schedules ENABLE ROW LEVEL SECURITY;
ALTER TABLE viewing_schedules FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS viewing_schedules_tenant_isolation ON viewing_schedules;
CREATE POLICY viewing_schedules_tenant_isolation ON viewing_schedules
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM listing_inquiries li
        JOIN listings p ON p.id = li.listing_id
        WHERE li.id = viewing_schedules.inquiry_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM listing_inquiries li
        JOIN listings p ON p.id = li.listing_id
        WHERE li.id = viewing_schedules.inquiry_id AND p.organization_id = get_current_org_id()));
