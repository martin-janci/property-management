-- Phase 0: RLS Foundation Hardening
-- Adds Row-Level Security policy to tenant-scoped table from migration 00105
-- (Reality Portal: compare lists, UC-48).
--
--   compare_lists -> listings.organization_id (fk listing_id)

-- ============================================
-- compare_lists (child of listings, fk listing_id)
-- ============================================
ALTER TABLE compare_lists ENABLE ROW LEVEL SECURITY;
ALTER TABLE compare_lists FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS compare_lists_tenant_isolation ON compare_lists;
CREATE POLICY compare_lists_tenant_isolation ON compare_lists
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM listings p WHERE p.id = compare_lists.listing_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM listings p WHERE p.id = compare_lists.listing_id AND p.organization_id = get_current_org_id()));
