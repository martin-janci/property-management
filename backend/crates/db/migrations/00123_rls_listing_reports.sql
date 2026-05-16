-- Phase 0: RLS Foundation Hardening
-- Adds Row-Level Security policy to tenant-scoped table from migration 00106
-- (Reality Portal: listing reports, UC-23).
--
--   listing_reports -> listings.organization_id (fk listing_id)

-- ============================================
-- listing_reports (child of listings, fk listing_id)
-- ============================================
ALTER TABLE listing_reports ENABLE ROW LEVEL SECURITY;
ALTER TABLE listing_reports FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS listing_reports_tenant_isolation ON listing_reports;
CREATE POLICY listing_reports_tenant_isolation ON listing_reports
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM listings p WHERE p.id = listing_reports.listing_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM listings p WHERE p.id = listing_reports.listing_id AND p.organization_id = get_current_org_id()));
