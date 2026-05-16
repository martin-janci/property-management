-- Phase 0: RLS Foundation Hardening
-- Adds Row-Level Security policy to tenant-scoped table from migration 00084
-- (Epic 109: User Type Feature Experience).
--
--   feature_usage_events -> own organization_id (NULLABLE -- rows with a NULL
--                           organization_id are not visible to non-super-admins)

-- ============================================
-- feature_usage_events (own-org; organization_id is nullable)
-- ============================================
ALTER TABLE feature_usage_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE feature_usage_events FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS feature_usage_events_tenant_isolation ON feature_usage_events;
CREATE POLICY feature_usage_events_tenant_isolation ON feature_usage_events
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());
