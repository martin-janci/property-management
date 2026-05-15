-- Phase 0: RLS Foundation Hardening
-- Adds Row-Level Security policies to tenant-scoped tables from migration 00077
-- (Epic 74: Owner Investment Analytics).
--
-- Org-reach chains traced from 00077_create_owner_analytics.sql + 00008_create_units.sql:
--   expense_auto_approval_rules -> own organization_id
--   property_valuations         -> units -> buildings.organization_id (fk unit_id;
--                                  units has no organization_id, it reaches org
--                                  through buildings.building_id)
--   property_value_history      -> units -> buildings.organization_id (fk unit_id)
--   expense_approval_requests   -> units -> buildings.organization_id (fk unit_id)
--   comparable_properties       -> property_valuations -> units -> buildings
--                                  (fk valuation_id; property_valuations has no
--                                  organization_id)

-- ============================================
-- expense_auto_approval_rules (own-org)
-- ============================================
ALTER TABLE expense_auto_approval_rules ENABLE ROW LEVEL SECURITY;
ALTER TABLE expense_auto_approval_rules FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS expense_auto_approval_rules_tenant_isolation ON expense_auto_approval_rules;
CREATE POLICY expense_auto_approval_rules_tenant_isolation ON expense_auto_approval_rules
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

-- ============================================
-- property_valuations (child of units, fk unit_id)
-- units reaches org through buildings; chain to buildings.organization_id.
-- ============================================
ALTER TABLE property_valuations ENABLE ROW LEVEL SECURITY;
ALTER TABLE property_valuations FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS property_valuations_tenant_isolation ON property_valuations;
CREATE POLICY property_valuations_tenant_isolation ON property_valuations
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM units u
        JOIN buildings b ON b.id = u.building_id
        WHERE u.id = property_valuations.unit_id AND b.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM units u
        JOIN buildings b ON b.id = u.building_id
        WHERE u.id = property_valuations.unit_id AND b.organization_id = get_current_org_id()));

-- ============================================
-- property_value_history (child of units, fk unit_id)
-- ============================================
ALTER TABLE property_value_history ENABLE ROW LEVEL SECURITY;
ALTER TABLE property_value_history FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS property_value_history_tenant_isolation ON property_value_history;
CREATE POLICY property_value_history_tenant_isolation ON property_value_history
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM units u
        JOIN buildings b ON b.id = u.building_id
        WHERE u.id = property_value_history.unit_id AND b.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM units u
        JOIN buildings b ON b.id = u.building_id
        WHERE u.id = property_value_history.unit_id AND b.organization_id = get_current_org_id()));

-- ============================================
-- expense_approval_requests (child of units, fk unit_id)
-- ============================================
ALTER TABLE expense_approval_requests ENABLE ROW LEVEL SECURITY;
ALTER TABLE expense_approval_requests FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS expense_approval_requests_tenant_isolation ON expense_approval_requests;
CREATE POLICY expense_approval_requests_tenant_isolation ON expense_approval_requests
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM units u
        JOIN buildings b ON b.id = u.building_id
        WHERE u.id = expense_approval_requests.unit_id AND b.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM units u
        JOIN buildings b ON b.id = u.building_id
        WHERE u.id = expense_approval_requests.unit_id AND b.organization_id = get_current_org_id()));

-- ============================================
-- comparable_properties (child of property_valuations, fk valuation_id)
-- property_valuations has no organization_id; chain through units -> buildings.
-- ============================================
ALTER TABLE comparable_properties ENABLE ROW LEVEL SECURITY;
ALTER TABLE comparable_properties FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS comparable_properties_tenant_isolation ON comparable_properties;
CREATE POLICY comparable_properties_tenant_isolation ON comparable_properties
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM property_valuations pv
        JOIN units u ON u.id = pv.unit_id
        JOIN buildings b ON b.id = u.building_id
        WHERE pv.id = comparable_properties.valuation_id AND b.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM property_valuations pv
        JOIN units u ON u.id = pv.unit_id
        JOIN buildings b ON b.id = u.building_id
        WHERE pv.id = comparable_properties.valuation_id AND b.organization_id = get_current_org_id()));
