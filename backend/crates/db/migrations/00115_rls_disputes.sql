-- Phase 0: RLS Foundation Hardening
-- Adds Row-Level Security policies to tenant-scoped tables from migration 00076
-- (Epic 77: Dispute Resolution).
--
-- Org-reach chains traced from 00076_create_disputes.sql:
--   disputes             -> own organization_id
--   dispute_parties      -> disputes.organization_id (fk dispute_id)
--   dispute_evidence     -> disputes.organization_id (fk dispute_id)
--   dispute_activities   -> disputes.organization_id (fk dispute_id)
--   mediation_sessions   -> disputes.organization_id (fk dispute_id)
--   session_attendances  -> mediation_sessions -> disputes (fk session_id;
--                           mediation_sessions has no organization_id)
--   party_submissions    -> disputes.organization_id (fk dispute_id)
--   dispute_resolutions  -> disputes.organization_id (fk dispute_id)
--   resolution_votes     -> dispute_resolutions -> disputes (fk resolution_id;
--                           dispute_resolutions has no organization_id)
--   action_items         -> disputes.organization_id (fk dispute_id)
--   escalations          -> disputes.organization_id (fk dispute_id)

-- ============================================
-- disputes (own-org)
-- ============================================
ALTER TABLE disputes ENABLE ROW LEVEL SECURITY;
ALTER TABLE disputes FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS disputes_tenant_isolation ON disputes;
CREATE POLICY disputes_tenant_isolation ON disputes
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

-- ============================================
-- dispute_parties (child of disputes, fk dispute_id)
-- ============================================
ALTER TABLE dispute_parties ENABLE ROW LEVEL SECURITY;
ALTER TABLE dispute_parties FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS dispute_parties_tenant_isolation ON dispute_parties;
CREATE POLICY dispute_parties_tenant_isolation ON dispute_parties
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM disputes p WHERE p.id = dispute_parties.dispute_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM disputes p WHERE p.id = dispute_parties.dispute_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- dispute_evidence (child of disputes, fk dispute_id)
-- ============================================
ALTER TABLE dispute_evidence ENABLE ROW LEVEL SECURITY;
ALTER TABLE dispute_evidence FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS dispute_evidence_tenant_isolation ON dispute_evidence;
CREATE POLICY dispute_evidence_tenant_isolation ON dispute_evidence
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM disputes p WHERE p.id = dispute_evidence.dispute_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM disputes p WHERE p.id = dispute_evidence.dispute_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- dispute_activities (child of disputes, fk dispute_id)
-- ============================================
ALTER TABLE dispute_activities ENABLE ROW LEVEL SECURITY;
ALTER TABLE dispute_activities FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS dispute_activities_tenant_isolation ON dispute_activities;
CREATE POLICY dispute_activities_tenant_isolation ON dispute_activities
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM disputes p WHERE p.id = dispute_activities.dispute_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM disputes p WHERE p.id = dispute_activities.dispute_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- mediation_sessions (child of disputes, fk dispute_id)
-- ============================================
ALTER TABLE mediation_sessions ENABLE ROW LEVEL SECURITY;
ALTER TABLE mediation_sessions FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS mediation_sessions_tenant_isolation ON mediation_sessions;
CREATE POLICY mediation_sessions_tenant_isolation ON mediation_sessions
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM disputes p WHERE p.id = mediation_sessions.dispute_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM disputes p WHERE p.id = mediation_sessions.dispute_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- session_attendances (child of mediation_sessions, fk session_id)
-- mediation_sessions has no organization_id; reach disputes via two-hop.
-- ============================================
ALTER TABLE session_attendances ENABLE ROW LEVEL SECURITY;
ALTER TABLE session_attendances FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS session_attendances_tenant_isolation ON session_attendances;
CREATE POLICY session_attendances_tenant_isolation ON session_attendances
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM mediation_sessions ms
        JOIN disputes p ON p.id = ms.dispute_id
        WHERE ms.id = session_attendances.session_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM mediation_sessions ms
        JOIN disputes p ON p.id = ms.dispute_id
        WHERE ms.id = session_attendances.session_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- party_submissions (child of disputes, fk dispute_id)
-- ============================================
ALTER TABLE party_submissions ENABLE ROW LEVEL SECURITY;
ALTER TABLE party_submissions FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS party_submissions_tenant_isolation ON party_submissions;
CREATE POLICY party_submissions_tenant_isolation ON party_submissions
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM disputes p WHERE p.id = party_submissions.dispute_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM disputes p WHERE p.id = party_submissions.dispute_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- dispute_resolutions (child of disputes, fk dispute_id)
-- ============================================
ALTER TABLE dispute_resolutions ENABLE ROW LEVEL SECURITY;
ALTER TABLE dispute_resolutions FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS dispute_resolutions_tenant_isolation ON dispute_resolutions;
CREATE POLICY dispute_resolutions_tenant_isolation ON dispute_resolutions
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM disputes p WHERE p.id = dispute_resolutions.dispute_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM disputes p WHERE p.id = dispute_resolutions.dispute_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- resolution_votes (child of dispute_resolutions, fk resolution_id)
-- dispute_resolutions has no organization_id; reach disputes via two-hop.
-- ============================================
ALTER TABLE resolution_votes ENABLE ROW LEVEL SECURITY;
ALTER TABLE resolution_votes FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS resolution_votes_tenant_isolation ON resolution_votes;
CREATE POLICY resolution_votes_tenant_isolation ON resolution_votes
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM dispute_resolutions dr
        JOIN disputes p ON p.id = dr.dispute_id
        WHERE dr.id = resolution_votes.resolution_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM dispute_resolutions dr
        JOIN disputes p ON p.id = dr.dispute_id
        WHERE dr.id = resolution_votes.resolution_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- action_items (child of disputes, fk dispute_id)
-- ============================================
ALTER TABLE action_items ENABLE ROW LEVEL SECURITY;
ALTER TABLE action_items FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS action_items_tenant_isolation ON action_items;
CREATE POLICY action_items_tenant_isolation ON action_items
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM disputes p WHERE p.id = action_items.dispute_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM disputes p WHERE p.id = action_items.dispute_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- escalations (child of disputes, fk dispute_id)
-- ============================================
ALTER TABLE escalations ENABLE ROW LEVEL SECURITY;
ALTER TABLE escalations FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS escalations_tenant_isolation ON escalations;
CREATE POLICY escalations_tenant_isolation ON escalations
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM disputes p WHERE p.id = escalations.dispute_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM disputes p WHERE p.id = escalations.dispute_id AND p.organization_id = get_current_org_id()));
