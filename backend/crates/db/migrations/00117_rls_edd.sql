-- Phase 0: RLS Foundation Hardening
-- Adds Row-Level Security policies to tenant-scoped tables from migration 00078
-- (Epic 100: Enhanced Due Diligence / AML / DSA compliance).
--
--   aml_risk_assessments -> own organization_id (NOT NULL)
--   edd_records          -> own organization_id (NOT NULL)
--   moderation_cases     -> own organization_id (NULLABLE -- rows with a NULL
--                           organization_id are not visible to non-super-admins,
--                           which is the desired conservative behaviour)
--   suspicious_activities-> own organization_id (NOT NULL)
--   edd_documents        -> edd_records.organization_id (fk edd_id)

-- ============================================
-- aml_risk_assessments (own-org)
-- ============================================
ALTER TABLE aml_risk_assessments ENABLE ROW LEVEL SECURITY;
ALTER TABLE aml_risk_assessments FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS aml_risk_assessments_tenant_isolation ON aml_risk_assessments;
CREATE POLICY aml_risk_assessments_tenant_isolation ON aml_risk_assessments
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

-- ============================================
-- edd_records (own-org)
-- ============================================
ALTER TABLE edd_records ENABLE ROW LEVEL SECURITY;
ALTER TABLE edd_records FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS edd_records_tenant_isolation ON edd_records;
CREATE POLICY edd_records_tenant_isolation ON edd_records
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

-- ============================================
-- moderation_cases (own-org; organization_id is nullable)
-- ============================================
ALTER TABLE moderation_cases ENABLE ROW LEVEL SECURITY;
ALTER TABLE moderation_cases FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS moderation_cases_tenant_isolation ON moderation_cases;
CREATE POLICY moderation_cases_tenant_isolation ON moderation_cases
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

-- ============================================
-- suspicious_activities (own-org)
-- ============================================
ALTER TABLE suspicious_activities ENABLE ROW LEVEL SECURITY;
ALTER TABLE suspicious_activities FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS suspicious_activities_tenant_isolation ON suspicious_activities;
CREATE POLICY suspicious_activities_tenant_isolation ON suspicious_activities
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

-- ============================================
-- edd_documents (child of edd_records, fk edd_id)
-- ============================================
ALTER TABLE edd_documents ENABLE ROW LEVEL SECURITY;
ALTER TABLE edd_documents FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS edd_documents_tenant_isolation ON edd_documents;
CREATE POLICY edd_documents_tenant_isolation ON edd_documents
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM edd_records p WHERE p.id = edd_documents.edd_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM edd_records p WHERE p.id = edd_documents.edd_id AND p.organization_id = get_current_org_id()));
