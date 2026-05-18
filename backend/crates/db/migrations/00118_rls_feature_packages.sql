-- Phase 0: RLS Foundation Hardening
-- Adds Row-Level Security policy to tenant-scoped table from migration 00083
-- (Epic 108: Feature Packages & Bundles).
--
--   organization_packages -> own organization_id

-- ============================================
-- organization_packages (own-org)
-- ============================================
ALTER TABLE organization_packages ENABLE ROW LEVEL SECURITY;
ALTER TABLE organization_packages FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS organization_packages_tenant_isolation ON organization_packages;
CREATE POLICY organization_packages_tenant_isolation ON organization_packages
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());
