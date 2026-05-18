-- Phase 0: RLS Foundation Hardening
-- Adds Row-Level Security policies to tenant-scoped tables from migration 00103
-- (Epic 68: Service Provider Marketplace).
--
-- NOTE: 00103 already gave `rfqs`, `provider_reviews` and `favorite_providers`
-- RLS + policies (legacy current_setting pattern, left untouched -- not gap
-- tables). The three child tables below had NO RLS at all.
--
--   provider_quotes      -> rfqs.organization_id (fk rfq_id)
--   rfq_invitations      -> rfqs.organization_id (fk rfq_id)
--   review_helpful_votes -> provider_reviews.organization_id (fk review_id)

-- ============================================
-- provider_quotes (child of rfqs, fk rfq_id)
-- ============================================
ALTER TABLE provider_quotes ENABLE ROW LEVEL SECURITY;
ALTER TABLE provider_quotes FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS provider_quotes_tenant_isolation ON provider_quotes;
CREATE POLICY provider_quotes_tenant_isolation ON provider_quotes
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM rfqs p WHERE p.id = provider_quotes.rfq_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM rfqs p WHERE p.id = provider_quotes.rfq_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- rfq_invitations (child of rfqs, fk rfq_id)
-- ============================================
ALTER TABLE rfq_invitations ENABLE ROW LEVEL SECURITY;
ALTER TABLE rfq_invitations FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rfq_invitations_tenant_isolation ON rfq_invitations;
CREATE POLICY rfq_invitations_tenant_isolation ON rfq_invitations
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM rfqs p WHERE p.id = rfq_invitations.rfq_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM rfqs p WHERE p.id = rfq_invitations.rfq_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- review_helpful_votes (child of provider_reviews, fk review_id)
-- ============================================
ALTER TABLE review_helpful_votes ENABLE ROW LEVEL SECURITY;
ALTER TABLE review_helpful_votes FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS review_helpful_votes_tenant_isolation ON review_helpful_votes;
CREATE POLICY review_helpful_votes_tenant_isolation ON review_helpful_votes
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM provider_reviews p WHERE p.id = review_helpful_votes.review_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM provider_reviews p WHERE p.id = review_helpful_votes.review_id AND p.organization_id = get_current_org_id()));
