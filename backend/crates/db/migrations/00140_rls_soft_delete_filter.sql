-- ============================================================================
-- Phase 5.5: Tenant Lifecycle & Operability
-- Migration 00140 — soft-delete filter on every tenant-scoped RLS policy.
-- ============================================================================
-- GENERATED FILE — do not hand-edit. Regenerate with:
--   bash backend/scripts/generate-soft-delete-rls-migration.sh
--
-- Defense for brainstorming leak #16: once an organization is soft-deleted
-- (organizations.deleted_at IS NOT NULL), every tenant context sees zero
-- rows from that org via the RLS policy below. Super-admin context still
-- sees them — so a soft-deleted tenant can be inspected and restored.
--
-- The helper reads the LIVE organizations.deleted_at flag (no caching) —
-- because RLS evaluates the policy expression on every query, an admin
-- restore is visible to subsequent tenant queries within the same session.
-- ============================================================================

-- ----------------------------------------------------------------------------
-- Helper: returns TRUE iff the current org context is alive (or there is
-- no org context — bypass paths like /tenant-config still work).
-- ----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION get_current_org_not_deleted() RETURNS BOOLEAN AS $$
DECLARE
    org_id_val UUID;
BEGIN
    org_id_val := get_current_org_id();
    IF org_id_val IS NULL THEN
        RETURN TRUE;
    END IF;
    RETURN NOT EXISTS (
        SELECT 1 FROM organizations
        WHERE id = org_id_val
          AND deleted_at IS NOT NULL
    );
END;
$$ LANGUAGE plpgsql STABLE;

COMMENT ON FUNCTION get_current_org_not_deleted() IS
    'Phase 5.5: returns TRUE if the request''s tenant org is not soft-deleted. '
    'Used as an AND filter on every tenant-scoped RLS policy. Super-admin paths '
    'bypass this via the is_super_admin() OR-leg in the same policies.';


-- ----------------------------------------------------------------------------
-- account_transactions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS account_transactions_manage ON account_transactions;
CREATE POLICY account_transactions_manage ON account_transactions FOR ALL USING ((((EXISTS ( SELECT 1 FROM (financial_accounts fa JOIN organization_members om ON ((om.organization_id = fa.organization_id))) WHERE ((fa.id = account_transactions.account_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted()) WITH CHECK ((((EXISTS ( SELECT 1 FROM (financial_accounts fa JOIN organization_members om ON ((om.organization_id = fa.organization_id))) WHERE ((fa.id = account_transactions.account_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS account_transactions_select ON account_transactions;
CREATE POLICY account_transactions_select ON account_transactions FOR SELECT USING ((((EXISTS ( SELECT 1 FROM (financial_accounts fa JOIN organization_members om ON ((om.organization_id = fa.organization_id))) WHERE ((fa.id = account_transactions.account_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid)))) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- action_items
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS action_items_tenant_isolation ON action_items;
CREATE POLICY action_items_tenant_isolation ON action_items FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM disputes p WHERE ((p.id = action_items.dispute_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM disputes p WHERE ((p.id = action_items.dispute_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- agency_domains
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS agency_domains_tenant_isolation ON agency_domains;
CREATE POLICY agency_domains_tenant_isolation ON agency_domains FOR ALL USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- agency_listings
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS agency_listings_tenant_isolation ON agency_listings;
CREATE POLICY agency_listings_tenant_isolation ON agency_listings FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM listings p WHERE ((p.id = agency_listings.listing_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM listings p WHERE ((p.id = agency_listings.listing_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- ai_chat_messages
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS ai_chat_messages_tenant_isolation ON ai_chat_messages;
CREATE POLICY ai_chat_messages_tenant_isolation ON ai_chat_messages FOR ALL USING (((session_id IN ( SELECT ai_chat_sessions.id FROM ai_chat_sessions WHERE (ai_chat_sessions.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- ai_chat_sessions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS ai_chat_sessions_tenant_isolation ON ai_chat_sessions;
CREATE POLICY ai_chat_sessions_tenant_isolation ON ai_chat_sessions FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- ai_risk_scoring_models
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS ai_risk_models_delete ON ai_risk_scoring_models;
CREATE POLICY ai_risk_models_delete ON ai_risk_scoring_models FOR DELETE USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS ai_risk_models_insert ON ai_risk_scoring_models;
CREATE POLICY ai_risk_models_insert ON ai_risk_scoring_models FOR INSERT WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS ai_risk_models_select ON ai_risk_scoring_models;
CREATE POLICY ai_risk_models_select ON ai_risk_scoring_models FOR SELECT USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS ai_risk_models_update ON ai_risk_scoring_models;
CREATE POLICY ai_risk_models_update ON ai_risk_scoring_models FOR UPDATE USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- ai_training_feedback
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS ai_training_feedback_tenant_isolation ON ai_training_feedback;
CREATE POLICY ai_training_feedback_tenant_isolation ON ai_training_feedback FOR ALL USING (((message_id IN ( SELECT m.id FROM (ai_chat_messages m JOIN ai_chat_sessions s ON ((s.id = m.session_id))) WHERE (s.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- aml_risk_assessments
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS aml_risk_assessments_tenant_isolation ON aml_risk_assessments;
CREATE POLICY aml_risk_assessments_tenant_isolation ON aml_risk_assessments FOR ALL USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- announcement_attachments
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS announcement_attachments_tenant_isolation ON announcement_attachments;
CREATE POLICY announcement_attachments_tenant_isolation ON announcement_attachments FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM announcements a WHERE ((a.id = announcement_attachments.announcement_id) AND (a.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM announcements a WHERE ((a.id = announcement_attachments.announcement_id) AND (a.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- announcement_comments
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS announcement_comments_tenant_isolation ON announcement_comments;
CREATE POLICY announcement_comments_tenant_isolation ON announcement_comments FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM announcements a WHERE ((a.id = announcement_comments.announcement_id) AND (a.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM announcements a WHERE ((a.id = announcement_comments.announcement_id) AND (a.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- announcement_reads
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS announcement_reads_tenant_isolation ON announcement_reads;
CREATE POLICY announcement_reads_tenant_isolation ON announcement_reads FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM announcements a WHERE ((a.id = announcement_reads.announcement_id) AND (a.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM announcements a WHERE ((a.id = announcement_reads.announcement_id) AND (a.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- announcements
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS announcements_tenant_isolation ON announcements;
CREATE POLICY announcements_tenant_isolation ON announcements FOR ALL USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- api_key_usage_logs
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS api_key_usage_policy ON api_key_usage_logs;
CREATE POLICY api_key_usage_policy ON api_key_usage_logs FOR ALL USING ((((api_key_id IN ( SELECT dk.id FROM (developer_api_keys dk JOIN developer_accounts da ON ((dk.developer_id = da.id))) WHERE (da.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid))) OR (COALESCE((current_setting('app.is_super_admin'::text, true))::boolean, false) = true))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- article_comment_reactions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS article_comment_reactions_tenant_isolation ON article_comment_reactions;
CREATE POLICY article_comment_reactions_tenant_isolation ON article_comment_reactions FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (article_comments c JOIN news_articles a ON ((a.id = c.article_id))) WHERE ((c.id = article_comment_reactions.comment_id) AND (a.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (article_comments c JOIN news_articles a ON ((a.id = c.article_id))) WHERE ((c.id = article_comment_reactions.comment_id) AND (a.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- article_comments
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS article_comments_tenant_isolation ON article_comments;
CREATE POLICY article_comments_tenant_isolation ON article_comments FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM news_articles a WHERE ((a.id = article_comments.article_id) AND (a.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM news_articles a WHERE ((a.id = article_comments.article_id) AND (a.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- article_media
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS article_media_tenant_isolation ON article_media;
CREATE POLICY article_media_tenant_isolation ON article_media FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM news_articles a WHERE ((a.id = article_media.article_id) AND (a.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM news_articles a WHERE ((a.id = article_media.article_id) AND (a.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- article_reactions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS article_reactions_tenant_isolation ON article_reactions;
CREATE POLICY article_reactions_tenant_isolation ON article_reactions FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM news_articles a WHERE ((a.id = article_reactions.article_id) AND (a.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM news_articles a WHERE ((a.id = article_reactions.article_id) AND (a.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- article_views
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS article_views_tenant_isolation ON article_views;
CREATE POLICY article_views_tenant_isolation ON article_views FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM news_articles a WHERE ((a.id = article_views.article_id) AND (a.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM news_articles a WHERE ((a.id = article_views.article_id) AND (a.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- audit_logs
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS audit_logs_insert_only ON audit_logs;
CREATE POLICY audit_logs_insert_only ON audit_logs FOR INSERT WITH CHECK ((true) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS audit_logs_own_actions ON audit_logs;
CREATE POLICY audit_logs_own_actions ON audit_logs FOR SELECT USING (((user_id = (current_setting('app.current_user_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS audit_logs_super_admin ON audit_logs;
CREATE POLICY audit_logs_super_admin ON audit_logs FOR ALL USING ((is_super_admin()) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- avm_property_valuations
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS valuations_tenant_isolation ON avm_property_valuations;
CREATE POLICY valuations_tenant_isolation ON avm_property_valuations FOR ALL USING (((organization_id = (current_setting('app.current_tenant'::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- avm_property_value_history
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS value_history_tenant_isolation ON avm_property_value_history;
CREATE POLICY value_history_tenant_isolation ON avm_property_value_history FOR ALL USING (((organization_id = (current_setting('app.current_tenant'::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- background_job_executions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS background_job_executions_tenant_isolation ON background_job_executions;
CREATE POLICY background_job_executions_tenant_isolation ON background_job_executions FOR ALL USING (((is_super_admin() OR (COALESCE(NULLIF(current_setting('app.is_system'::text, true), ''::text), 'false'::text) = 'true'::text) OR (EXISTS ( SELECT 1 FROM background_jobs bj WHERE ((bj.id = background_job_executions.job_id) AND ((bj.org_id IS NULL) OR (bj.org_id = get_current_org_id()))))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (COALESCE(NULLIF(current_setting('app.is_system'::text, true), ''::text), 'false'::text) = 'true'::text))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- background_jobs
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS background_jobs_org_insert ON background_jobs;
CREATE POLICY background_jobs_org_insert ON background_jobs FOR INSERT WITH CHECK ((((org_id IS NULL) OR (org_id = (current_setting('app.current_org_id'::text, true))::uuid) OR is_current_user_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS background_jobs_org_select ON background_jobs;
CREATE POLICY background_jobs_org_select ON background_jobs FOR SELECT USING ((((org_id IS NULL) OR (org_id = (current_setting('app.current_org_id'::text, true))::uuid) OR is_current_user_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS background_jobs_system_update ON background_jobs;
CREATE POLICY background_jobs_system_update ON background_jobs FOR UPDATE USING (((is_current_user_super_admin() OR (COALESCE(NULLIF(current_setting('app.is_system'::text, true), ''::text), 'false'::text) = 'true'::text))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- benchmark_comparisons
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS benchmark_comparisons_tenant_isolation ON benchmark_comparisons;
CREATE POLICY benchmark_comparisons_tenant_isolation ON benchmark_comparisons FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM performance_portfolios p WHERE ((p.id = benchmark_comparisons.portfolio_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM performance_portfolios p WHERE ((p.id = benchmark_comparisons.portfolio_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- board_meetings
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS board_meetings_tenant_isolation ON board_meetings;
CREATE POLICY board_meetings_tenant_isolation ON board_meetings FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- board_members
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS board_members_tenant_isolation ON board_members;
CREATE POLICY board_members_tenant_isolation ON board_members FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- budget_actuals
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS budget_actuals_tenant_isolation ON budget_actuals;
CREATE POLICY budget_actuals_tenant_isolation ON budget_actuals FOR ALL USING (((budget_item_id IN ( SELECT bi.id FROM (budget_items bi JOIN budgets b ON ((b.id = bi.budget_id))) WHERE (b.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- budget_categories
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS budget_categories_tenant_isolation ON budget_categories;
CREATE POLICY budget_categories_tenant_isolation ON budget_categories FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- budget_items
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS budget_items_tenant_isolation ON budget_items;
CREATE POLICY budget_items_tenant_isolation ON budget_items FOR ALL USING (((budget_id IN ( SELECT budgets.id FROM budgets WHERE (budgets.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- budget_variance_alerts
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS budget_alerts_tenant_isolation ON budget_variance_alerts;
CREATE POLICY budget_alerts_tenant_isolation ON budget_variance_alerts FOR ALL USING (((budget_item_id IN ( SELECT bi.id FROM (budget_items bi JOIN budgets b ON ((b.id = bi.budget_id))) WHERE (b.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- budgets
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS budgets_tenant_isolation ON budgets;
CREATE POLICY budgets_tenant_isolation ON budgets FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- building_certifications
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS building_certifications_org_policy ON building_certifications;
CREATE POLICY building_certifications_org_policy ON building_certifications FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- building_package_settings
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS building_package_settings_tenant_isolation ON building_package_settings;
CREATE POLICY building_package_settings_tenant_isolation ON building_package_settings FOR ALL USING (((tenant_id = (current_setting('app.current_tenant_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- building_registry_rules
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS building_registry_rules_insert ON building_registry_rules;
CREATE POLICY building_registry_rules_insert ON building_registry_rules FOR INSERT WITH CHECK (((tenant_id = (current_setting('app.current_tenant_id'::text))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS building_registry_rules_tenant_isolation ON building_registry_rules;
CREATE POLICY building_registry_rules_tenant_isolation ON building_registry_rules FOR ALL USING (((tenant_id = (current_setting('app.current_tenant_id'::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- building_visitor_settings
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS building_visitor_settings_tenant_isolation ON building_visitor_settings;
CREATE POLICY building_visitor_settings_tenant_isolation ON building_visitor_settings FOR ALL USING (((tenant_id = (current_setting('app.current_tenant_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- buildings
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS buildings_tenant_isolation ON buildings;
CREATE POLICY buildings_tenant_isolation ON buildings FOR ALL USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- capital_calls
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS capital_calls_tenant_isolation ON capital_calls;
CREATE POLICY capital_calls_tenant_isolation ON capital_calls FOR ALL USING (((organization_id = (current_setting('app.current_tenant_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- capital_plans
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS capital_plans_tenant_isolation ON capital_plans;
CREATE POLICY capital_plans_tenant_isolation ON capital_plans FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- carbon_footprints
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS carbon_footprints_insert ON carbon_footprints;
CREATE POLICY carbon_footprints_insert ON carbon_footprints FOR INSERT WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS carbon_footprints_select ON carbon_footprints;
CREATE POLICY carbon_footprints_select ON carbon_footprints FOR SELECT USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS carbon_footprints_update ON carbon_footprints;
CREATE POLICY carbon_footprints_update ON carbon_footprints FOR UPDATE USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- certification_audit_logs
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS certification_audit_logs_org_policy ON certification_audit_logs;
CREATE POLICY certification_audit_logs_org_policy ON certification_audit_logs FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- certification_benchmarks
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS certification_benchmarks_org_policy ON certification_benchmarks;
CREATE POLICY certification_benchmarks_org_policy ON certification_benchmarks FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- certification_costs
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS certification_costs_org_policy ON certification_costs;
CREATE POLICY certification_costs_org_policy ON certification_costs FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- certification_credits
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS certification_credits_org_policy ON certification_credits;
CREATE POLICY certification_credits_org_policy ON certification_credits FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- certification_documents
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS certification_documents_org_policy ON certification_documents;
CREATE POLICY certification_documents_org_policy ON certification_documents FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- certification_milestones
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS certification_milestones_org_policy ON certification_milestones;
CREATE POLICY certification_milestones_org_policy ON certification_milestones FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- certification_reminders
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS certification_reminders_org_policy ON certification_reminders;
CREATE POLICY certification_reminders_org_policy ON certification_reminders FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- cma_property_comparisons
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS cma_comparisons_tenant_isolation ON cma_property_comparisons;
CREATE POLICY cma_comparisons_tenant_isolation ON cma_property_comparisons FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM comparative_market_analyses cma WHERE ((cma.id = cma_property_comparisons.cma_id) AND (cma.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM comparative_market_analyses cma WHERE ((cma.id = cma_property_comparisons.cma_id) AND (cma.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- community_comment_reactions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS community_comment_reactions_tenant_isolation ON community_comment_reactions;
CREATE POLICY community_comment_reactions_tenant_isolation ON community_comment_reactions FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (((community_comments c JOIN community_posts po ON ((po.id = c.post_id))) JOIN community_groups g ON ((g.id = po.group_id))) JOIN buildings b ON ((b.id = g.building_id))) WHERE ((c.id = community_comment_reactions.comment_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (((community_comments c JOIN community_posts po ON ((po.id = c.post_id))) JOIN community_groups g ON ((g.id = po.group_id))) JOIN buildings b ON ((b.id = g.building_id))) WHERE ((c.id = community_comment_reactions.comment_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- community_comments
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS community_comments_tenant_isolation ON community_comments;
CREATE POLICY community_comments_tenant_isolation ON community_comments FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM ((community_posts po JOIN community_groups g ON ((g.id = po.group_id))) JOIN buildings b ON ((b.id = g.building_id))) WHERE ((po.id = community_comments.post_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM ((community_posts po JOIN community_groups g ON ((g.id = po.group_id))) JOIN buildings b ON ((b.id = g.building_id))) WHERE ((po.id = community_comments.post_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- community_event_comments
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS community_event_comments_tenant_isolation ON community_event_comments;
CREATE POLICY community_event_comments_tenant_isolation ON community_event_comments FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (community_events e JOIN buildings b ON ((b.id = e.building_id))) WHERE ((e.id = community_event_comments.event_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (community_events e JOIN buildings b ON ((b.id = e.building_id))) WHERE ((e.id = community_event_comments.event_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- community_event_rsvps
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS community_event_rsvps_tenant_isolation ON community_event_rsvps;
CREATE POLICY community_event_rsvps_tenant_isolation ON community_event_rsvps FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (community_events e JOIN buildings b ON ((b.id = e.building_id))) WHERE ((e.id = community_event_rsvps.event_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (community_events e JOIN buildings b ON ((b.id = e.building_id))) WHERE ((e.id = community_event_rsvps.event_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- community_events
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS community_events_tenant_isolation ON community_events;
CREATE POLICY community_events_tenant_isolation ON community_events FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM buildings b WHERE ((b.id = community_events.building_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM buildings b WHERE ((b.id = community_events.building_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- community_group_members
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS community_group_members_tenant_isolation ON community_group_members;
CREATE POLICY community_group_members_tenant_isolation ON community_group_members FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (community_groups g JOIN buildings b ON ((b.id = g.building_id))) WHERE ((g.id = community_group_members.group_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (community_groups g JOIN buildings b ON ((b.id = g.building_id))) WHERE ((g.id = community_group_members.group_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- community_groups
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS community_groups_tenant_isolation ON community_groups;
CREATE POLICY community_groups_tenant_isolation ON community_groups FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM buildings b WHERE ((b.id = community_groups.building_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM buildings b WHERE ((b.id = community_groups.building_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- community_join_requests
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS community_join_requests_tenant_isolation ON community_join_requests;
CREATE POLICY community_join_requests_tenant_isolation ON community_join_requests FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (community_groups g JOIN buildings b ON ((b.id = g.building_id))) WHERE ((g.id = community_join_requests.group_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (community_groups g JOIN buildings b ON ((b.id = g.building_id))) WHERE ((g.id = community_join_requests.group_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- community_poll_votes
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS community_poll_votes_tenant_isolation ON community_poll_votes;
CREATE POLICY community_poll_votes_tenant_isolation ON community_poll_votes FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM ((community_posts po JOIN community_groups g ON ((g.id = po.group_id))) JOIN buildings b ON ((b.id = g.building_id))) WHERE ((po.id = community_poll_votes.post_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM ((community_posts po JOIN community_groups g ON ((g.id = po.group_id))) JOIN buildings b ON ((b.id = g.building_id))) WHERE ((po.id = community_poll_votes.post_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- community_post_reactions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS community_post_reactions_tenant_isolation ON community_post_reactions;
CREATE POLICY community_post_reactions_tenant_isolation ON community_post_reactions FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM ((community_posts po JOIN community_groups g ON ((g.id = po.group_id))) JOIN buildings b ON ((b.id = g.building_id))) WHERE ((po.id = community_post_reactions.post_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM ((community_posts po JOIN community_groups g ON ((g.id = po.group_id))) JOIN buildings b ON ((b.id = g.building_id))) WHERE ((po.id = community_post_reactions.post_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- community_posts
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS community_posts_tenant_isolation ON community_posts;
CREATE POLICY community_posts_tenant_isolation ON community_posts FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (community_groups g JOIN buildings b ON ((b.id = g.building_id))) WHERE ((g.id = community_posts.group_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (community_groups g JOIN buildings b ON ((b.id = g.building_id))) WHERE ((g.id = community_posts.group_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- community_rules
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS community_rules_tenant_isolation ON community_rules;
CREATE POLICY community_rules_tenant_isolation ON community_rules FOR ALL USING (((organization_id = (current_setting('app.current_tenant'::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- comparable_adjustments
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS adjustments_tenant_isolation ON comparable_adjustments;
CREATE POLICY adjustments_tenant_isolation ON comparable_adjustments FOR ALL USING (((comparable_id IN ( SELECT valuation_comparables.id FROM valuation_comparables WHERE (valuation_comparables.organization_id = (current_setting('app.current_tenant'::text))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- comparable_properties
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS comparable_properties_tenant_isolation ON comparable_properties;
CREATE POLICY comparable_properties_tenant_isolation ON comparable_properties FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM ((property_valuations pv JOIN units u ON ((u.id = pv.unit_id))) JOIN buildings b ON ((b.id = u.building_id))) WHERE ((pv.id = comparable_properties.valuation_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM ((property_valuations pv JOIN units u ON ((u.id = pv.unit_id))) JOIN buildings b ON ((b.id = u.building_id))) WHERE ((pv.id = comparable_properties.valuation_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- comparative_market_analyses
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS cma_tenant_isolation ON comparative_market_analyses;
CREATE POLICY cma_tenant_isolation ON comparative_market_analyses FOR ALL USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- compare_lists
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS compare_lists_tenant_isolation ON compare_lists;
CREATE POLICY compare_lists_tenant_isolation ON compare_lists FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM listings p WHERE ((p.id = compare_lists.listing_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM listings p WHERE ((p.id = compare_lists.listing_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- compliance_audit_trail
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS compliance_audit_tenant_isolation ON compliance_audit_trail;
CREATE POLICY compliance_audit_tenant_isolation ON compliance_audit_trail FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- compliance_requirements
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS compliance_req_tenant_isolation ON compliance_requirements;
CREATE POLICY compliance_req_tenant_isolation ON compliance_requirements FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- compliance_templates
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS compliance_templates_tenant_isolation ON compliance_templates;
CREATE POLICY compliance_templates_tenant_isolation ON compliance_templates FOR ALL USING ((((organization_id IS NULL) OR (organization_id = (current_setting('app.current_organization_id'::text, true))::uuid))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- compliance_verifications
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS compliance_verifications_tenant_isolation ON compliance_verifications;
CREATE POLICY compliance_verifications_tenant_isolation ON compliance_verifications FOR ALL USING (((requirement_id IN ( SELECT compliance_requirements.id FROM compliance_requirements WHERE (compliance_requirements.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- connector_execution_logs
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS connector_logs_policy ON connector_execution_logs;
CREATE POLICY connector_logs_policy ON connector_execution_logs FOR ALL USING (((org_connector_id IN ( SELECT organization_connectors.id FROM organization_connectors WHERE (organization_connectors.organization_id = (NULLIF(current_setting('app.current_organization_id'::text, true), ''::text))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- consumption_aggregates
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS consumption_aggregates_manage ON consumption_aggregates;
CREATE POLICY consumption_aggregates_manage ON consumption_aggregates FOR ALL USING ((is_super_admin()) AND get_current_org_not_deleted()) WITH CHECK ((is_super_admin()) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS consumption_aggregates_select ON consumption_aggregates;
CREATE POLICY consumption_aggregates_select ON consumption_aggregates FOR SELECT USING ((((EXISTS ( SELECT 1 FROM (meters m JOIN organization_members om ON ((om.organization_id = m.organization_id))) WHERE ((m.id = consumption_aggregates.meter_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid)))) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- coupon_redemptions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS coupon_redemptions_tenant_isolation ON coupon_redemptions;
CREATE POLICY coupon_redemptions_tenant_isolation ON coupon_redemptions FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- critical_notification_acknowledgments
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS critical_acks_insert ON critical_notification_acknowledgments;
CREATE POLICY critical_acks_insert ON critical_notification_acknowledgments FOR INSERT WITH CHECK ((((user_id = COALESCE((current_setting('request.user_id'::text, true))::uuid, '00000000-0000-0000-0000-000000000000'::uuid)) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS critical_acks_read ON critical_notification_acknowledgments;
CREATE POLICY critical_acks_read ON critical_notification_acknowledgments FOR SELECT USING ((((user_id = COALESCE((current_setting('request.user_id'::text, true))::uuid, '00000000-0000-0000-0000-000000000000'::uuid)) OR (EXISTS ( SELECT 1 FROM critical_notifications cn WHERE ((cn.id = critical_notification_acknowledgments.notification_id) AND (cn.organization_id = COALESCE((current_setting('request.org_id'::text, true))::uuid, '00000000-0000-0000-0000-000000000000'::uuid))))) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- critical_notifications
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS critical_notifications_insert ON critical_notifications;
CREATE POLICY critical_notifications_insert ON critical_notifications FOR INSERT WITH CHECK ((((created_by = COALESCE((current_setting('request.user_id'::text, true))::uuid, '00000000-0000-0000-0000-000000000000'::uuid)) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS critical_notifications_org_read ON critical_notifications;
CREATE POLICY critical_notifications_org_read ON critical_notifications FOR SELECT USING ((((organization_id = COALESCE((current_setting('request.org_id'::text, true))::uuid, '00000000-0000-0000-0000-000000000000'::uuid)) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- cross_border_leases
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS "Tenant isolation for cross_border_leases" ON cross_border_leases;
CREATE POLICY "Tenant isolation for cross_border_leases" ON cross_border_leases FOR ALL USING (((organization_id = (current_setting('app.current_tenant'::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- currency_conversion_audit
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS "Tenant isolation for currency_conversion_audit" ON currency_conversion_audit;
CREATE POLICY "Tenant isolation for currency_conversion_audit" ON currency_conversion_audit FOR ALL USING (((transaction_id IN ( SELECT multi_currency_transactions.id FROM multi_currency_transactions WHERE (multi_currency_transactions.organization_id = (current_setting('app.current_tenant'::text))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- currency_exposure_analysis
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS "Tenant isolation for currency_exposure_analysis" ON currency_exposure_analysis;
CREATE POLICY "Tenant isolation for currency_exposure_analysis" ON currency_exposure_analysis FOR ALL USING (((organization_id = (current_setting('app.current_tenant'::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- delegation_audit_log
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS delegation_audit_select ON delegation_audit_log;
CREATE POLICY delegation_audit_select ON delegation_audit_log FOR SELECT USING (((EXISTS ( SELECT 1 FROM delegations d WHERE ((d.id = delegation_audit_log.delegation_id) AND ((d.owner_user_id = (current_setting('app.current_user_id'::text, true))::uuid) OR (d.delegate_user_id = (current_setting('app.current_user_id'::text, true))::uuid)))))) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS delegation_audit_super_admin ON delegation_audit_log;
CREATE POLICY delegation_audit_super_admin ON delegation_audit_log FOR ALL USING ((((current_setting('app.is_super_admin'::text, true))::boolean = true)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- delegations
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS delegations_delete_owner ON delegations;
CREATE POLICY delegations_delete_owner ON delegations FOR DELETE USING (((owner_user_id = (current_setting('app.current_user_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS delegations_insert_owner ON delegations;
CREATE POLICY delegations_insert_owner ON delegations FOR INSERT WITH CHECK (((owner_user_id = (current_setting('app.current_user_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS delegations_select_delegate ON delegations;
CREATE POLICY delegations_select_delegate ON delegations FOR SELECT USING (((delegate_user_id = (current_setting('app.current_user_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS delegations_select_owner ON delegations;
CREATE POLICY delegations_select_owner ON delegations FOR SELECT USING (((owner_user_id = (current_setting('app.current_user_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS delegations_super_admin ON delegations;
CREATE POLICY delegations_super_admin ON delegations FOR ALL USING ((((current_setting('app.is_super_admin'::text, true))::boolean = true)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS delegations_update_delegate ON delegations;
CREATE POLICY delegations_update_delegate ON delegations FOR UPDATE USING ((((delegate_user_id = (current_setting('app.current_user_id'::text, true))::uuid) AND (status = 'pending'::delegation_status))) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS delegations_update_owner ON delegations;
CREATE POLICY delegations_update_owner ON delegations FOR UPDATE USING (((owner_user_id = (current_setting('app.current_user_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- developer_accounts
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS developer_accounts_policy ON developer_accounts;
CREATE POLICY developer_accounts_policy ON developer_accounts FOR ALL USING ((((user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) OR (COALESCE((current_setting('app.is_super_admin'::text, true))::boolean, false) = true))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- developer_api_keys
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS api_keys_policy ON developer_api_keys;
CREATE POLICY api_keys_policy ON developer_api_keys FOR ALL USING ((((developer_id IN ( SELECT developer_accounts.id FROM developer_accounts WHERE (developer_accounts.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid))) OR (COALESCE((current_setting('app.is_super_admin'::text, true))::boolean, false) = true))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- developer_oauth_apps
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS oauth_apps_policy ON developer_oauth_apps;
CREATE POLICY oauth_apps_policy ON developer_oauth_apps FOR ALL USING ((((developer_id IN ( SELECT developer_accounts.id FROM developer_accounts WHERE (developer_accounts.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid))) OR (COALESCE((current_setting('app.is_super_admin'::text, true))::boolean, false) = true))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- developer_oauth_grants
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS oauth_grants_policy ON developer_oauth_grants;
CREATE POLICY oauth_grants_policy ON developer_oauth_grants FOR ALL USING ((((user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) OR (COALESCE((current_setting('app.is_super_admin'::text, true))::boolean, false) = true))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- developer_sandboxes
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS sandboxes_policy ON developer_sandboxes;
CREATE POLICY sandboxes_policy ON developer_sandboxes FOR ALL USING ((((developer_id IN ( SELECT developer_accounts.id FROM developer_accounts WHERE (developer_accounts.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid))) OR (COALESCE((current_setting('app.is_super_admin'::text, true))::boolean, false) = true))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- dispute_activities
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS dispute_activities_tenant_isolation ON dispute_activities;
CREATE POLICY dispute_activities_tenant_isolation ON dispute_activities FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM disputes p WHERE ((p.id = dispute_activities.dispute_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM disputes p WHERE ((p.id = dispute_activities.dispute_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- dispute_evidence
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS dispute_evidence_tenant_isolation ON dispute_evidence;
CREATE POLICY dispute_evidence_tenant_isolation ON dispute_evidence FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM disputes p WHERE ((p.id = dispute_evidence.dispute_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM disputes p WHERE ((p.id = dispute_evidence.dispute_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- dispute_parties
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS dispute_parties_tenant_isolation ON dispute_parties;
CREATE POLICY dispute_parties_tenant_isolation ON dispute_parties FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM disputes p WHERE ((p.id = dispute_parties.dispute_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM disputes p WHERE ((p.id = dispute_parties.dispute_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- dispute_resolutions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS dispute_resolutions_tenant_isolation ON dispute_resolutions;
CREATE POLICY dispute_resolutions_tenant_isolation ON dispute_resolutions FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM disputes p WHERE ((p.id = dispute_resolutions.dispute_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM disputes p WHERE ((p.id = dispute_resolutions.dispute_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- disputes
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS disputes_tenant_isolation ON disputes;
CREATE POLICY disputes_tenant_isolation ON disputes FOR ALL USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- distributed_spans
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS distributed_spans_select ON distributed_spans;
CREATE POLICY distributed_spans_select ON distributed_spans FOR SELECT USING (((EXISTS ( SELECT 1 FROM distributed_traces t WHERE (t.id = distributed_spans.trace_id)))) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS distributed_spans_system_insert ON distributed_spans;
CREATE POLICY distributed_spans_system_insert ON distributed_spans FOR INSERT WITH CHECK (((is_current_user_super_admin() OR (COALESCE(NULLIF(current_setting('app.is_system'::text, true), ''::text), 'false'::text) = 'true'::text))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- distributed_traces
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS distributed_traces_org_select ON distributed_traces;
CREATE POLICY distributed_traces_org_select ON distributed_traces FOR SELECT USING ((((org_id IS NULL) OR (org_id = (current_setting('app.current_org_id'::text, true))::uuid) OR is_current_user_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS distributed_traces_system_insert ON distributed_traces;
CREATE POLICY distributed_traces_system_insert ON distributed_traces FOR INSERT WITH CHECK (((is_current_user_super_admin() OR (COALESCE(NULLIF(current_setting('app.is_system'::text, true), ''::text), 'false'::text) = 'true'::text))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- document_classification_history
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS document_classification_history_tenant_isolation ON document_classification_history;
CREATE POLICY document_classification_history_tenant_isolation ON document_classification_history FOR ALL USING (((EXISTS ( SELECT 1 FROM documents d WHERE ((d.id = document_classification_history.document_id) AND (d.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- document_embeddings
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS document_embeddings_org_isolation ON document_embeddings;
CREATE POLICY document_embeddings_org_isolation ON document_embeddings FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted()) WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- document_folders
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS folder_tenant_isolation ON document_folders;
CREATE POLICY folder_tenant_isolation ON document_folders FOR ALL USING (((organization_id = (current_setting('app.tenant_id'::text, true))::uuid)) AND get_current_org_not_deleted()) WITH CHECK (((organization_id = (current_setting('app.tenant_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- document_intelligence_stats
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS document_intelligence_stats_tenant_isolation ON document_intelligence_stats;
CREATE POLICY document_intelligence_stats_tenant_isolation ON document_intelligence_stats FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- document_ocr_queue
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS document_ocr_queue_tenant_isolation ON document_ocr_queue;
CREATE POLICY document_ocr_queue_tenant_isolation ON document_ocr_queue FOR ALL USING (((EXISTS ( SELECT 1 FROM documents d WHERE ((d.id = document_ocr_queue.document_id) AND (d.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- document_share_access_log
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS access_log_tenant_isolation ON document_share_access_log;
CREATE POLICY access_log_tenant_isolation ON document_share_access_log FOR ALL USING (((share_id IN ( SELECT ds.id FROM (document_shares ds JOIN documents d ON ((d.id = ds.document_id))) WHERE (d.organization_id = (current_setting('app.tenant_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- document_shares
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS share_tenant_isolation ON document_shares;
CREATE POLICY share_tenant_isolation ON document_shares FOR ALL USING (((document_id IN ( SELECT documents.id FROM documents WHERE (documents.organization_id = (current_setting('app.tenant_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- document_summarization_queue
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS document_summarization_queue_tenant_isolation ON document_summarization_queue;
CREATE POLICY document_summarization_queue_tenant_isolation ON document_summarization_queue FOR ALL USING (((EXISTS ( SELECT 1 FROM documents d WHERE ((d.id = document_summarization_queue.document_id) AND (d.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- document_templates
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS document_templates_read ON document_templates;
CREATE POLICY document_templates_read ON document_templates FOR SELECT USING (((((organization_id)::text = current_setting('app.tenant_id'::text, true)) OR (organization_id IN ( SELECT om.organization_id FROM organization_members om WHERE ((om.user_id = (current_setting('request.user_id'::text, true))::uuid) AND ((om.status)::text = 'active'::text)))))) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS document_templates_write ON document_templates;
CREATE POLICY document_templates_write ON document_templates FOR ALL USING ((((organization_id)::text = current_setting('app.tenant_id'::text, true))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- documents
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS document_tenant_isolation ON documents;
CREATE POLICY document_tenant_isolation ON documents FOR ALL USING (((organization_id = (current_setting('app.tenant_id'::text, true))::uuid)) AND get_current_org_not_deleted()) WITH CHECK (((organization_id = (current_setting('app.tenant_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- edd_documents
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS edd_documents_tenant_isolation ON edd_documents;
CREATE POLICY edd_documents_tenant_isolation ON edd_documents FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM edd_records p WHERE ((p.id = edd_documents.edd_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM edd_records p WHERE ((p.id = edd_documents.edd_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- edd_records
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS edd_records_tenant_isolation ON edd_records;
CREATE POLICY edd_records_tenant_isolation ON edd_records FOR ALL USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- emergency_broadcast_acknowledgments
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS emergency_broadcast_acks_tenant_isolation ON emergency_broadcast_acknowledgments;
CREATE POLICY emergency_broadcast_acks_tenant_isolation ON emergency_broadcast_acknowledgments FOR ALL USING (((EXISTS ( SELECT 1 FROM emergency_broadcasts b WHERE ((b.id = emergency_broadcast_acknowledgments.broadcast_id) AND (b.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- emergency_broadcasts
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS emergency_broadcasts_tenant_isolation ON emergency_broadcasts;
CREATE POLICY emergency_broadcasts_tenant_isolation ON emergency_broadcasts FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- emergency_contacts
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS emergency_contacts_tenant_isolation ON emergency_contacts;
CREATE POLICY emergency_contacts_tenant_isolation ON emergency_contacts FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- emergency_drills
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS emergency_drills_tenant_isolation ON emergency_drills;
CREATE POLICY emergency_drills_tenant_isolation ON emergency_drills FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- emergency_incident_attachments
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS emergency_incident_attachments_tenant_isolation ON emergency_incident_attachments;
CREATE POLICY emergency_incident_attachments_tenant_isolation ON emergency_incident_attachments FOR ALL USING (((EXISTS ( SELECT 1 FROM emergency_incidents i WHERE ((i.id = emergency_incident_attachments.incident_id) AND (i.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- emergency_incident_updates
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS emergency_incident_updates_tenant_isolation ON emergency_incident_updates;
CREATE POLICY emergency_incident_updates_tenant_isolation ON emergency_incident_updates FOR ALL USING (((EXISTS ( SELECT 1 FROM emergency_incidents i WHERE ((i.id = emergency_incident_updates.incident_id) AND (i.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- emergency_incidents
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS emergency_incidents_tenant_isolation ON emergency_incidents;
CREATE POLICY emergency_incidents_tenant_isolation ON emergency_incidents FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- emergency_protocols
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS emergency_protocols_tenant_isolation ON emergency_protocols;
CREATE POLICY emergency_protocols_tenant_isolation ON emergency_protocols FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- enforcement_actions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS enforcement_actions_tenant_isolation ON enforcement_actions;
CREATE POLICY enforcement_actions_tenant_isolation ON enforcement_actions FOR ALL USING (((organization_id = (current_setting('app.current_tenant'::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- equipment
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS equipment_tenant_isolation ON equipment;
CREATE POLICY equipment_tenant_isolation ON equipment FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- equipment_documents
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS equipment_documents_tenant_isolation ON equipment_documents;
CREATE POLICY equipment_documents_tenant_isolation ON equipment_documents FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted()) WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- equipment_health_thresholds
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS health_thresholds_tenant_isolation ON equipment_health_thresholds;
CREATE POLICY health_thresholds_tenant_isolation ON equipment_health_thresholds FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted()) WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- equipment_maintenance
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS equipment_maintenance_tenant_isolation ON equipment_maintenance;
CREATE POLICY equipment_maintenance_tenant_isolation ON equipment_maintenance FOR ALL USING (((equipment_id IN ( SELECT equipment.id FROM equipment WHERE (equipment.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- equipment_predictions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS equipment_predictions_tenant_isolation ON equipment_predictions;
CREATE POLICY equipment_predictions_tenant_isolation ON equipment_predictions FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted()) WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- equipment_registry
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS equipment_registry_delete_policy ON equipment_registry;
CREATE POLICY equipment_registry_delete_policy ON equipment_registry FOR DELETE USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS equipment_registry_insert_policy ON equipment_registry;
CREATE POLICY equipment_registry_insert_policy ON equipment_registry FOR INSERT WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS equipment_registry_select_policy ON equipment_registry;
CREATE POLICY equipment_registry_select_policy ON equipment_registry FOR SELECT USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS equipment_registry_tenant_isolation ON equipment_registry;
CREATE POLICY equipment_registry_tenant_isolation ON equipment_registry FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted()) WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS equipment_registry_update_policy ON equipment_registry;
CREATE POLICY equipment_registry_update_policy ON equipment_registry FOR UPDATE USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- escalations
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS escalations_tenant_isolation ON escalations;
CREATE POLICY escalations_tenant_isolation ON escalations FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM disputes p WHERE ((p.id = escalations.dispute_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM disputes p WHERE ((p.id = escalations.dispute_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- esg_benchmarks
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS esg_benchmarks_insert ON esg_benchmarks;
CREATE POLICY esg_benchmarks_insert ON esg_benchmarks FOR INSERT WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_benchmarks_select ON esg_benchmarks;
CREATE POLICY esg_benchmarks_select ON esg_benchmarks FOR SELECT USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_benchmarks_update ON esg_benchmarks;
CREATE POLICY esg_benchmarks_update ON esg_benchmarks FOR UPDATE USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- esg_configurations
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS esg_configs_insert ON esg_configurations;
CREATE POLICY esg_configs_insert ON esg_configurations FOR INSERT WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_configs_select ON esg_configurations;
CREATE POLICY esg_configs_select ON esg_configurations FOR SELECT USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_configs_update ON esg_configurations;
CREATE POLICY esg_configs_update ON esg_configurations FOR UPDATE USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- esg_dashboard_metrics
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS esg_dashboard_insert ON esg_dashboard_metrics;
CREATE POLICY esg_dashboard_insert ON esg_dashboard_metrics FOR INSERT WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_dashboard_select ON esg_dashboard_metrics;
CREATE POLICY esg_dashboard_select ON esg_dashboard_metrics FOR SELECT USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_dashboard_update ON esg_dashboard_metrics;
CREATE POLICY esg_dashboard_update ON esg_dashboard_metrics FOR UPDATE USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- esg_import_jobs
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS esg_import_insert ON esg_import_jobs;
CREATE POLICY esg_import_insert ON esg_import_jobs FOR INSERT WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_import_select ON esg_import_jobs;
CREATE POLICY esg_import_select ON esg_import_jobs FOR SELECT USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_import_update ON esg_import_jobs;
CREATE POLICY esg_import_update ON esg_import_jobs FOR UPDATE USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- esg_metrics
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS esg_metrics_delete ON esg_metrics;
CREATE POLICY esg_metrics_delete ON esg_metrics FOR DELETE USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_metrics_insert ON esg_metrics;
CREATE POLICY esg_metrics_insert ON esg_metrics FOR INSERT WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_metrics_select ON esg_metrics;
CREATE POLICY esg_metrics_select ON esg_metrics FOR SELECT USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_metrics_update ON esg_metrics;
CREATE POLICY esg_metrics_update ON esg_metrics FOR UPDATE USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- esg_reports
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS esg_reports_insert ON esg_reports;
CREATE POLICY esg_reports_insert ON esg_reports FOR INSERT WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_reports_select ON esg_reports;
CREATE POLICY esg_reports_select ON esg_reports FOR SELECT USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_reports_update ON esg_reports;
CREATE POLICY esg_reports_update ON esg_reports FOR UPDATE USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- esg_targets
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS esg_targets_delete ON esg_targets;
CREATE POLICY esg_targets_delete ON esg_targets FOR DELETE USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_targets_insert ON esg_targets;
CREATE POLICY esg_targets_insert ON esg_targets FOR INSERT WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_targets_select ON esg_targets;
CREATE POLICY esg_targets_select ON esg_targets FOR SELECT USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_targets_update ON esg_targets;
CREATE POLICY esg_targets_update ON esg_targets FOR UPDATE USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- eu_taxonomy_assessments
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS eu_taxonomy_insert ON eu_taxonomy_assessments;
CREATE POLICY eu_taxonomy_insert ON eu_taxonomy_assessments FOR INSERT WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS eu_taxonomy_select ON eu_taxonomy_assessments;
CREATE POLICY eu_taxonomy_select ON eu_taxonomy_assessments FOR SELECT USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS eu_taxonomy_update ON eu_taxonomy_assessments;
CREATE POLICY eu_taxonomy_update ON eu_taxonomy_assessments FOR UPDATE USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- exchange_rate_fetch_log
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS "Tenant isolation for exchange_rate_fetch_log" ON exchange_rate_fetch_log;
CREATE POLICY "Tenant isolation for exchange_rate_fetch_log" ON exchange_rate_fetch_log FOR ALL USING ((((organization_id IS NULL) OR (organization_id = (current_setting('app.current_tenant'::text))::uuid))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- expense_approval_requests
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS expense_approval_requests_tenant_isolation ON expense_approval_requests;
CREATE POLICY expense_approval_requests_tenant_isolation ON expense_approval_requests FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (units u JOIN buildings b ON ((b.id = u.building_id))) WHERE ((u.id = expense_approval_requests.unit_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (units u JOIN buildings b ON ((b.id = u.building_id))) WHERE ((u.id = expense_approval_requests.unit_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- expense_auto_approval_rules
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS expense_auto_approval_rules_tenant_isolation ON expense_auto_approval_rules;
CREATE POLICY expense_auto_approval_rules_tenant_isolation ON expense_auto_approval_rules FOR ALL USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- external_listing_ids
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS external_listing_ids_tenant_isolation ON external_listing_ids;
CREATE POLICY external_listing_ids_tenant_isolation ON external_listing_ids FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM listings p WHERE ((p.id = external_listing_ids.listing_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM listings p WHERE ((p.id = external_listing_ids.listing_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- extraction_corrections
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS extraction_corrections_org_policy ON extraction_corrections;
CREATE POLICY extraction_corrections_org_policy ON extraction_corrections FOR ALL USING (((extraction_id IN ( SELECT e.id FROM (lease_extractions e JOIN lease_documents d ON ((d.id = e.document_id))) WHERE (d.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- facilities
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS facilities_delete_manager ON facilities;
CREATE POLICY facilities_delete_manager ON facilities FOR DELETE USING (((EXISTS ( SELECT 1 FROM (buildings b JOIN organization_members om ON ((om.organization_id = b.organization_id))) WHERE ((b.id = facilities.building_id) AND (om.user_id = (current_setting('app.current_user_id'::text, true))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[])))))) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS facilities_insert_manager ON facilities;
CREATE POLICY facilities_insert_manager ON facilities FOR INSERT WITH CHECK (((EXISTS ( SELECT 1 FROM (buildings b JOIN organization_members om ON ((om.organization_id = b.organization_id))) WHERE ((b.id = facilities.building_id) AND (om.user_id = (current_setting('app.current_user_id'::text, true))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[])))))) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS facilities_select_org ON facilities;
CREATE POLICY facilities_select_org ON facilities FOR SELECT USING (((EXISTS ( SELECT 1 FROM buildings b WHERE ((b.id = facilities.building_id) AND (b.organization_id = (current_setting('app.current_org_id'::text, true))::uuid))))) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS facilities_super_admin ON facilities;
CREATE POLICY facilities_super_admin ON facilities FOR ALL USING ((((current_setting('app.is_super_admin'::text, true))::boolean = true)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS facilities_update_manager ON facilities;
CREATE POLICY facilities_update_manager ON facilities FOR UPDATE USING (((EXISTS ( SELECT 1 FROM (buildings b JOIN organization_members om ON ((om.organization_id = b.organization_id))) WHERE ((b.id = facilities.building_id) AND (om.user_id = (current_setting('app.current_user_id'::text, true))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[])))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- facility_bookings
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS facility_bookings_delete_own ON facility_bookings;
CREATE POLICY facility_bookings_delete_own ON facility_bookings FOR DELETE USING ((((user_id = (current_setting('app.current_user_id'::text, true))::uuid) AND (status = ANY (ARRAY['pending'::booking_status, 'approved'::booking_status])))) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS facility_bookings_insert_resident ON facility_bookings;
CREATE POLICY facility_bookings_insert_resident ON facility_bookings FOR INSERT WITH CHECK ((((user_id = (current_setting('app.current_user_id'::text, true))::uuid) AND (EXISTS ( SELECT 1 FROM (facilities f JOIN buildings b ON ((b.id = f.building_id))) WHERE ((f.id = facility_bookings.facility_id) AND (b.organization_id = (current_setting('app.current_org_id'::text, true))::uuid)))))) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS facility_bookings_select_org ON facility_bookings;
CREATE POLICY facility_bookings_select_org ON facility_bookings FOR SELECT USING ((((EXISTS ( SELECT 1 FROM (facilities f JOIN buildings b ON ((b.id = f.building_id))) WHERE ((f.id = facility_bookings.facility_id) AND (b.organization_id = (current_setting('app.current_org_id'::text, true))::uuid)))) OR (user_id = (current_setting('app.current_user_id'::text, true))::uuid))) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS facility_bookings_super_admin ON facility_bookings;
CREATE POLICY facility_bookings_super_admin ON facility_bookings FOR ALL USING ((((current_setting('app.is_super_admin'::text, true))::boolean = true)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS facility_bookings_update_own ON facility_bookings;
CREATE POLICY facility_bookings_update_own ON facility_bookings FOR UPDATE USING ((((user_id = (current_setting('app.current_user_id'::text, true))::uuid) OR (EXISTS ( SELECT 1 FROM ((facilities f JOIN buildings b ON ((b.id = f.building_id))) JOIN organization_members om ON ((om.organization_id = b.organization_id))) WHERE ((f.id = facility_bookings.facility_id) AND (om.user_id = (current_setting('app.current_user_id'::text, true))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- fault_attachments
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS fault_attachments_tenant_isolation ON fault_attachments;
CREATE POLICY fault_attachments_tenant_isolation ON fault_attachments FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM faults f WHERE ((f.id = fault_attachments.fault_id) AND (f.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM faults f WHERE ((f.id = fault_attachments.fault_id) AND (f.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- fault_timeline
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS fault_timeline_tenant_isolation ON fault_timeline;
CREATE POLICY fault_timeline_tenant_isolation ON fault_timeline FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM faults f WHERE ((f.id = fault_timeline.fault_id) AND (f.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM faults f WHERE ((f.id = fault_timeline.fault_id) AND (f.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- faults
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS faults_tenant_isolation ON faults;
CREATE POLICY faults_tenant_isolation ON faults FOR ALL USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- favorite_providers
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS favorites_org_access ON favorite_providers;
CREATE POLICY favorites_org_access ON favorite_providers FOR ALL USING (((organization_id IN ( SELECT organization_members.organization_id FROM organization_members WHERE (organization_members.user_id = (current_setting('app.user_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- feature_usage_events
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS feature_usage_events_tenant_isolation ON feature_usage_events;
CREATE POLICY feature_usage_events_tenant_isolation ON feature_usage_events FOR ALL USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- fee_schedules
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS fee_schedules_manage ON fee_schedules;
CREATE POLICY fee_schedules_manage ON fee_schedules FOR ALL USING ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = fee_schedules.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted()) WITH CHECK ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = fee_schedules.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS fee_schedules_select ON fee_schedules;
CREATE POLICY fee_schedules_select ON fee_schedules FOR SELECT USING ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = fee_schedules.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid)))) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- financial_accounts
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS financial_accounts_manage ON financial_accounts;
CREATE POLICY financial_accounts_manage ON financial_accounts FOR ALL USING ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = financial_accounts.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted()) WITH CHECK ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = financial_accounts.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS financial_accounts_select ON financial_accounts;
CREATE POLICY financial_accounts_select ON financial_accounts FOR SELECT USING ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = financial_accounts.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid)))) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- financial_forecasts
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS financial_forecasts_tenant_isolation ON financial_forecasts;
CREATE POLICY financial_forecasts_tenant_isolation ON financial_forecasts FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- financial_metrics
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS financial_metrics_tenant_isolation ON financial_metrics;
CREATE POLICY financial_metrics_tenant_isolation ON financial_metrics FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM performance_portfolios p WHERE ((p.id = financial_metrics.portfolio_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM performance_portfolios p WHERE ((p.id = financial_metrics.portfolio_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- fine_payments
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS fine_payments_tenant_isolation ON fine_payments;
CREATE POLICY fine_payments_tenant_isolation ON fine_payments FOR ALL USING (((organization_id = (current_setting('app.current_tenant'::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- form_downloads
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS form_downloads_org_isolation ON form_downloads;
CREATE POLICY form_downloads_org_isolation ON form_downloads FOR ALL USING (((form_id IN ( SELECT forms.id FROM forms WHERE (forms.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- form_fields
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS form_fields_org_isolation ON form_fields;
CREATE POLICY form_fields_org_isolation ON form_fields FOR ALL USING (((form_id IN ( SELECT forms.id FROM forms WHERE (forms.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- form_submissions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS form_submissions_org_isolation ON form_submissions;
CREATE POLICY form_submissions_org_isolation ON form_submissions FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- forms
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS forms_org_isolation ON forms;
CREATE POLICY forms_org_isolation ON forms FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- fund_alerts
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS fund_alerts_tenant_isolation ON fund_alerts;
CREATE POLICY fund_alerts_tenant_isolation ON fund_alerts FOR ALL USING (((fund_id IN ( SELECT reserve_funds.id FROM reserve_funds WHERE (reserve_funds.organization_id = (NULLIF(current_setting('app.current_organization_id'::text, true), ''::text))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- fund_components
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS fund_components_tenant_isolation ON fund_components;
CREATE POLICY fund_components_tenant_isolation ON fund_components FOR ALL USING (((fund_id IN ( SELECT reserve_funds.id FROM reserve_funds WHERE (reserve_funds.organization_id = (NULLIF(current_setting('app.current_organization_id'::text, true), ''::text))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- fund_contribution_schedules
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS contribution_schedules_tenant_isolation ON fund_contribution_schedules;
CREATE POLICY contribution_schedules_tenant_isolation ON fund_contribution_schedules FOR ALL USING (((fund_id IN ( SELECT reserve_funds.id FROM reserve_funds WHERE (reserve_funds.organization_id = (NULLIF(current_setting('app.current_organization_id'::text, true), ''::text))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- fund_investment_policies
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS investment_policies_tenant_isolation ON fund_investment_policies;
CREATE POLICY investment_policies_tenant_isolation ON fund_investment_policies FOR ALL USING (((fund_id IN ( SELECT reserve_funds.id FROM reserve_funds WHERE (reserve_funds.organization_id = (NULLIF(current_setting('app.current_organization_id'::text, true), ''::text))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- fund_projection_items
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS projection_items_tenant_isolation ON fund_projection_items;
CREATE POLICY projection_items_tenant_isolation ON fund_projection_items FOR ALL USING (((projection_id IN ( SELECT fp.id FROM (fund_projections fp JOIN reserve_funds rf ON ((rf.id = fp.fund_id))) WHERE (rf.organization_id = (NULLIF(current_setting('app.current_organization_id'::text, true), ''::text))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- fund_projections
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS fund_projections_tenant_isolation ON fund_projections;
CREATE POLICY fund_projections_tenant_isolation ON fund_projections FOR ALL USING (((fund_id IN ( SELECT reserve_funds.id FROM reserve_funds WHERE (reserve_funds.organization_id = (NULLIF(current_setting('app.current_organization_id'::text, true), ''::text))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- fund_transactions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS fund_transactions_tenant_isolation ON fund_transactions;
CREATE POLICY fund_transactions_tenant_isolation ON fund_transactions FOR ALL USING (((fund_id IN ( SELECT reserve_funds.id FROM reserve_funds WHERE (reserve_funds.organization_id = (NULLIF(current_setting('app.current_organization_id'::text, true), ''::text))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- government_portal_connections
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS gov_portal_connections_org_access ON government_portal_connections;
CREATE POLICY gov_portal_connections_org_access ON government_portal_connections FOR ALL USING ((((organization_id = (NULLIF(current_setting('app.current_tenant_id'::text, true), ''::text))::uuid) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- inquiry_messages
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS inquiry_messages_tenant_isolation ON inquiry_messages;
CREATE POLICY inquiry_messages_tenant_isolation ON inquiry_messages FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (listing_inquiries li JOIN listings p ON ((p.id = li.listing_id))) WHERE ((li.id = inquiry_messages.inquiry_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (listing_inquiries li JOIN listings p ON ((p.id = li.listing_id))) WHERE ((li.id = inquiry_messages.inquiry_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- insurance_claim_documents
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS insurance_claim_documents_org_isolation ON insurance_claim_documents;
CREATE POLICY insurance_claim_documents_org_isolation ON insurance_claim_documents FOR ALL USING (((claim_id IN ( SELECT insurance_claims.id FROM insurance_claims WHERE (insurance_claims.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- insurance_claim_history
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS insurance_claim_history_org_isolation ON insurance_claim_history;
CREATE POLICY insurance_claim_history_org_isolation ON insurance_claim_history FOR ALL USING (((claim_id IN ( SELECT insurance_claims.id FROM insurance_claims WHERE (insurance_claims.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- insurance_claims
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS insurance_claims_org_isolation ON insurance_claims;
CREATE POLICY insurance_claims_org_isolation ON insurance_claims FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- insurance_policies
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS insurance_policies_org_isolation ON insurance_policies;
CREATE POLICY insurance_policies_org_isolation ON insurance_policies FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- insurance_policy_documents
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS insurance_policy_documents_org_isolation ON insurance_policy_documents;
CREATE POLICY insurance_policy_documents_org_isolation ON insurance_policy_documents FOR ALL USING (((policy_id IN ( SELECT insurance_policies.id FROM insurance_policies WHERE (insurance_policies.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- insurance_renewal_reminders
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS insurance_renewal_reminders_org_isolation ON insurance_renewal_reminders;
CREATE POLICY insurance_renewal_reminders_org_isolation ON insurance_renewal_reminders FOR ALL USING (((policy_id IN ( SELECT insurance_policies.id FROM insurance_policies WHERE (insurance_policies.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- integration_ratings
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS integration_ratings_read ON integration_ratings;
CREATE POLICY integration_ratings_read ON integration_ratings FOR SELECT USING ((true) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS integration_ratings_write ON integration_ratings;
CREATE POLICY integration_ratings_write ON integration_ratings FOR ALL USING (((organization_id = (NULLIF(current_setting('app.current_organization_id'::text, true), ''::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- investment_portfolios
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS investment_portfolios_tenant_isolation ON investment_portfolios;
CREATE POLICY investment_portfolios_tenant_isolation ON investment_portfolios FOR ALL USING (((organization_id = (current_setting('app.current_tenant_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- investor_dashboard_metrics
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS investor_dashboard_metrics_tenant_isolation ON investor_dashboard_metrics;
CREATE POLICY investor_dashboard_metrics_tenant_isolation ON investor_dashboard_metrics FOR ALL USING (((organization_id = (current_setting('app.current_tenant_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- investor_distributions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS investor_distributions_tenant_isolation ON investor_distributions;
CREATE POLICY investor_distributions_tenant_isolation ON investor_distributions FOR ALL USING (((organization_id = (current_setting('app.current_tenant_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- investor_profiles
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS investor_profiles_tenant_isolation ON investor_profiles;
CREATE POLICY investor_profiles_tenant_isolation ON investor_profiles FOR ALL USING (((organization_id = (current_setting('app.current_tenant_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- investor_reports
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS investor_reports_tenant_isolation ON investor_reports;
CREATE POLICY investor_reports_tenant_isolation ON investor_reports FOR ALL USING (((organization_id = (current_setting('app.current_tenant_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- invoice_items
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS invoice_items_manage ON invoice_items;
CREATE POLICY invoice_items_manage ON invoice_items FOR ALL USING ((((EXISTS ( SELECT 1 FROM (invoices i JOIN organization_members om ON ((om.organization_id = i.organization_id))) WHERE ((i.id = invoice_items.invoice_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted()) WITH CHECK ((((EXISTS ( SELECT 1 FROM (invoices i JOIN organization_members om ON ((om.organization_id = i.organization_id))) WHERE ((i.id = invoice_items.invoice_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS invoice_items_select ON invoice_items;
CREATE POLICY invoice_items_select ON invoice_items FOR SELECT USING ((((EXISTS ( SELECT 1 FROM (invoices i JOIN organization_members om ON ((om.organization_id = i.organization_id))) WHERE ((i.id = invoice_items.invoice_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid)))) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- invoice_line_items
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS invoice_lines_tenant_isolation ON invoice_line_items;
CREATE POLICY invoice_lines_tenant_isolation ON invoice_line_items FOR ALL USING (((invoice_id IN ( SELECT subscription_invoices.id FROM subscription_invoices WHERE (subscription_invoices.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- invoices
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS invoices_manage ON invoices;
CREATE POLICY invoices_manage ON invoices FOR ALL USING ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = invoices.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted()) WITH CHECK ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = invoices.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS invoices_select ON invoices;
CREATE POLICY invoices_select ON invoices FOR SELECT USING ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = invoices.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid)))) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- late_fee_configs
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS late_fee_configs_manage ON late_fee_configs;
CREATE POLICY late_fee_configs_manage ON late_fee_configs FOR ALL USING ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = late_fee_configs.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = 'org_admin'::text)))) OR is_super_admin())) AND get_current_org_not_deleted()) WITH CHECK ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = late_fee_configs.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = 'org_admin'::text)))) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS late_fee_configs_select ON late_fee_configs;
CREATE POLICY late_fee_configs_select ON late_fee_configs FOR SELECT USING ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = late_fee_configs.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = 'org_admin'::text)))) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- lease_amendments
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS lease_amendments_org_isolation ON lease_amendments;
CREATE POLICY lease_amendments_org_isolation ON lease_amendments FOR ALL USING (((lease_id IN ( SELECT leases.id FROM leases WHERE (leases.organization_id = (current_setting('app.current_org_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- lease_documents
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS lease_documents_org_policy ON lease_documents;
CREATE POLICY lease_documents_org_policy ON lease_documents FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- lease_extractions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS lease_extractions_org_policy ON lease_extractions;
CREATE POLICY lease_extractions_org_policy ON lease_extractions FOR ALL USING (((document_id IN ( SELECT lease_documents.id FROM lease_documents WHERE (lease_documents.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- lease_imports
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS lease_imports_org_policy ON lease_imports;
CREATE POLICY lease_imports_org_policy ON lease_imports FOR ALL USING (((extraction_id IN ( SELECT e.id FROM (lease_extractions e JOIN lease_documents d ON ((d.id = e.document_id))) WHERE (d.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- lease_payments
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS lease_payments_org_isolation ON lease_payments;
CREATE POLICY lease_payments_org_isolation ON lease_payments FOR ALL USING (((organization_id = (current_setting('app.current_org_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- lease_reminders
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS lease_reminders_org_isolation ON lease_reminders;
CREATE POLICY lease_reminders_org_isolation ON lease_reminders FOR ALL USING (((lease_id IN ( SELECT leases.id FROM leases WHERE (leases.organization_id = (current_setting('app.current_org_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- lease_templates
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS lease_templates_org_isolation ON lease_templates;
CREATE POLICY lease_templates_org_isolation ON lease_templates FOR ALL USING (((organization_id = (current_setting('app.current_org_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- leases
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS leases_org_isolation ON leases;
CREATE POLICY leases_org_isolation ON leases FOR ALL USING (((organization_id = (current_setting('app.current_org_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- legal_document_versions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS legal_versions_tenant_isolation ON legal_document_versions;
CREATE POLICY legal_versions_tenant_isolation ON legal_document_versions FOR ALL USING (((document_id IN ( SELECT legal_documents.id FROM legal_documents WHERE (legal_documents.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- legal_documents
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS legal_documents_tenant_isolation ON legal_documents;
CREATE POLICY legal_documents_tenant_isolation ON legal_documents FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- legal_notice_recipients
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS notice_recipients_tenant_isolation ON legal_notice_recipients;
CREATE POLICY notice_recipients_tenant_isolation ON legal_notice_recipients FOR ALL USING (((notice_id IN ( SELECT legal_notices.id FROM legal_notices WHERE (legal_notices.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- legal_notices
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS legal_notices_tenant_isolation ON legal_notices;
CREATE POLICY legal_notices_tenant_isolation ON legal_notices FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- listing_analytics
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS listing_analytics_tenant_isolation ON listing_analytics;
CREATE POLICY listing_analytics_tenant_isolation ON listing_analytics FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM listings p WHERE ((p.id = listing_analytics.listing_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM listings p WHERE ((p.id = listing_analytics.listing_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- listing_edit_history
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS listing_edit_history_tenant_isolation ON listing_edit_history;
CREATE POLICY listing_edit_history_tenant_isolation ON listing_edit_history FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM listings p WHERE ((p.id = listing_edit_history.listing_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM listings p WHERE ((p.id = listing_edit_history.listing_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- listing_inquiries
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS listing_inquiries_tenant_isolation ON listing_inquiries;
CREATE POLICY listing_inquiries_tenant_isolation ON listing_inquiries FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM listings p WHERE ((p.id = listing_inquiries.listing_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM listings p WHERE ((p.id = listing_inquiries.listing_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- listing_photos
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS listing_photos_tenant_isolation ON listing_photos;
CREATE POLICY listing_photos_tenant_isolation ON listing_photos FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM listings p WHERE ((p.id = listing_photos.listing_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM listings p WHERE ((p.id = listing_photos.listing_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- listing_price_history
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS listing_price_history_tenant_isolation ON listing_price_history;
CREATE POLICY listing_price_history_tenant_isolation ON listing_price_history FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM listings p WHERE ((p.id = listing_price_history.listing_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM listings p WHERE ((p.id = listing_price_history.listing_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- listing_reports
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS listing_reports_tenant_isolation ON listing_reports;
CREATE POLICY listing_reports_tenant_isolation ON listing_reports FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM listings p WHERE ((p.id = listing_reports.listing_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM listings p WHERE ((p.id = listing_reports.listing_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- listing_syndications
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS listing_syndications_tenant_isolation ON listing_syndications;
CREATE POLICY listing_syndications_tenant_isolation ON listing_syndications FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM listings p WHERE ((p.id = listing_syndications.listing_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM listings p WHERE ((p.id = listing_syndications.listing_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- listings
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS listings_tenant_isolation ON listings;
CREATE POLICY listings_tenant_isolation ON listings FOR ALL USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- maintenance_alerts
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS maintenance_alerts_tenant_isolation ON maintenance_alerts;
CREATE POLICY maintenance_alerts_tenant_isolation ON maintenance_alerts FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted()) WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- maintenance_log_photos
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS maintenance_log_photos_tenant_isolation ON maintenance_log_photos;
CREATE POLICY maintenance_log_photos_tenant_isolation ON maintenance_log_photos FOR ALL USING (((maintenance_log_id IN ( SELECT maintenance_logs.id FROM maintenance_logs WHERE (maintenance_logs.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- maintenance_logs
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS maintenance_logs_tenant_isolation ON maintenance_logs;
CREATE POLICY maintenance_logs_tenant_isolation ON maintenance_logs FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted()) WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- maintenance_predictions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS maintenance_predictions_tenant_isolation ON maintenance_predictions;
CREATE POLICY maintenance_predictions_tenant_isolation ON maintenance_predictions FOR ALL USING (((equipment_id IN ( SELECT equipment.id FROM equipment WHERE (equipment.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- maintenance_schedules
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS maintenance_schedules_tenant_isolation ON maintenance_schedules;
CREATE POLICY maintenance_schedules_tenant_isolation ON maintenance_schedules FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- market_benchmarks
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS market_benchmarks_tenant_isolation ON market_benchmarks;
CREATE POLICY market_benchmarks_tenant_isolation ON market_benchmarks FOR ALL USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- market_data_points
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS market_data_points_tenant_isolation ON market_data_points;
CREATE POLICY market_data_points_tenant_isolation ON market_data_points FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM market_regions mr WHERE ((mr.id = market_data_points.region_id) AND (mr.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM market_regions mr WHERE ((mr.id = market_data_points.region_id) AND (mr.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- market_regions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS market_regions_tenant_isolation ON market_regions;
CREATE POLICY market_regions_tenant_isolation ON market_regions FOR ALL USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- market_statistics
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS market_statistics_tenant_isolation ON market_statistics;
CREATE POLICY market_statistics_tenant_isolation ON market_statistics FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM market_regions mr WHERE ((mr.id = market_statistics.region_id) AND (mr.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM market_regions mr WHERE ((mr.id = market_statistics.region_id) AND (mr.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- marketplace_favorites
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS marketplace_favorites_tenant_isolation ON marketplace_favorites;
CREATE POLICY marketplace_favorites_tenant_isolation ON marketplace_favorites FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (marketplace_items i JOIN buildings b ON ((b.id = i.building_id))) WHERE ((i.id = marketplace_favorites.item_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (marketplace_items i JOIN buildings b ON ((b.id = i.building_id))) WHERE ((i.id = marketplace_favorites.item_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- marketplace_inquiries
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS marketplace_inquiries_tenant_isolation ON marketplace_inquiries;
CREATE POLICY marketplace_inquiries_tenant_isolation ON marketplace_inquiries FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (marketplace_items i JOIN buildings b ON ((b.id = i.building_id))) WHERE ((i.id = marketplace_inquiries.item_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (marketplace_items i JOIN buildings b ON ((b.id = i.building_id))) WHERE ((i.id = marketplace_inquiries.item_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- marketplace_items
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS marketplace_items_tenant_isolation ON marketplace_items;
CREATE POLICY marketplace_items_tenant_isolation ON marketplace_items FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM buildings b WHERE ((b.id = marketplace_items.building_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM buildings b WHERE ((b.id = marketplace_items.building_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- mediation_sessions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS mediation_sessions_tenant_isolation ON mediation_sessions;
CREATE POLICY mediation_sessions_tenant_isolation ON mediation_sessions FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM disputes p WHERE ((p.id = mediation_sessions.dispute_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM disputes p WHERE ((p.id = mediation_sessions.dispute_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- meeting_action_items
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS action_items_tenant_isolation ON meeting_action_items;
CREATE POLICY action_items_tenant_isolation ON meeting_action_items FOR ALL USING (((EXISTS ( SELECT 1 FROM board_meetings bm WHERE ((bm.id = meeting_action_items.meeting_id) AND (bm.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- meeting_agenda_items
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS agenda_items_tenant_isolation ON meeting_agenda_items;
CREATE POLICY agenda_items_tenant_isolation ON meeting_agenda_items FOR ALL USING (((EXISTS ( SELECT 1 FROM board_meetings bm WHERE ((bm.id = meeting_agenda_items.meeting_id) AND (bm.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- meeting_attendance
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS attendance_tenant_isolation ON meeting_attendance;
CREATE POLICY attendance_tenant_isolation ON meeting_attendance FOR ALL USING (((EXISTS ( SELECT 1 FROM board_meetings bm WHERE ((bm.id = meeting_attendance.meeting_id) AND (bm.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- meeting_documents
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS meeting_docs_tenant_isolation ON meeting_documents;
CREATE POLICY meeting_docs_tenant_isolation ON meeting_documents FOR ALL USING (((EXISTS ( SELECT 1 FROM board_meetings bm WHERE ((bm.id = meeting_documents.meeting_id) AND (bm.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- meeting_minutes
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS minutes_tenant_isolation ON meeting_minutes;
CREATE POLICY minutes_tenant_isolation ON meeting_minutes FOR ALL USING (((EXISTS ( SELECT 1 FROM board_meetings bm WHERE ((bm.id = meeting_minutes.meeting_id) AND (bm.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- meeting_motions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS motions_tenant_isolation ON meeting_motions;
CREATE POLICY motions_tenant_isolation ON meeting_motions FOR ALL USING (((EXISTS ( SELECT 1 FROM board_meetings bm WHERE ((bm.id = meeting_motions.meeting_id) AND (bm.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- meeting_statistics
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS meeting_stats_tenant_isolation ON meeting_statistics;
CREATE POLICY meeting_stats_tenant_isolation ON meeting_statistics FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- message_threads
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS message_threads_tenant_isolation ON message_threads;
CREATE POLICY message_threads_tenant_isolation ON message_threads FOR ALL USING (((((current_setting('app.current_user_id'::text, true))::uuid = ANY (participant_ids)) AND (organization_id = COALESCE((current_setting('app.current_org_id'::text, true))::uuid, organization_id)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- messages
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS messages_tenant_isolation ON messages;
CREATE POLICY messages_tenant_isolation ON messages FOR ALL USING (((thread_id IN ( SELECT message_threads.id FROM message_threads WHERE (((current_setting('app.current_user_id'::text, true))::uuid = ANY (message_threads.participant_ids)) AND (message_threads.organization_id = COALESCE((current_setting('app.current_org_id'::text, true))::uuid, message_threads.organization_id)))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- meter_readings
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS meter_readings_insert ON meter_readings;
CREATE POLICY meter_readings_insert ON meter_readings FOR INSERT WITH CHECK ((((EXISTS ( SELECT 1 FROM (meters m JOIN organization_members om ON ((om.organization_id = m.organization_id))) WHERE ((m.id = meter_readings.meter_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid)))) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS meter_readings_manage ON meter_readings;
CREATE POLICY meter_readings_manage ON meter_readings FOR UPDATE USING ((((EXISTS ( SELECT 1 FROM (meters m JOIN organization_members om ON ((om.organization_id = m.organization_id))) WHERE ((m.id = meter_readings.meter_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS meter_readings_select ON meter_readings;
CREATE POLICY meter_readings_select ON meter_readings FOR SELECT USING ((((EXISTS ( SELECT 1 FROM (meters m JOIN organization_members om ON ((om.organization_id = m.organization_id))) WHERE ((m.id = meter_readings.meter_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid)))) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- meters
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS meters_manage ON meters;
CREATE POLICY meters_manage ON meters FOR ALL USING ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = meters.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted()) WITH CHECK ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = meters.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS meters_select ON meters;
CREATE POLICY meters_select ON meters FOR SELECT USING ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = meters.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid)))) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- missing_reading_alerts
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS missing_reading_alerts_manage ON missing_reading_alerts;
CREATE POLICY missing_reading_alerts_manage ON missing_reading_alerts FOR ALL USING ((((EXISTS ( SELECT 1 FROM (meters m JOIN organization_members om ON ((om.organization_id = m.organization_id))) WHERE ((m.id = missing_reading_alerts.meter_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted()) WITH CHECK ((((EXISTS ( SELECT 1 FROM (meters m JOIN organization_members om ON ((om.organization_id = m.organization_id))) WHERE ((m.id = missing_reading_alerts.meter_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS missing_reading_alerts_select ON missing_reading_alerts;
CREATE POLICY missing_reading_alerts_select ON missing_reading_alerts FOR SELECT USING ((((EXISTS ( SELECT 1 FROM (meters m JOIN organization_members om ON ((om.organization_id = m.organization_id))) WHERE ((m.id = missing_reading_alerts.meter_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- moderation_cases
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS moderation_cases_tenant_isolation ON moderation_cases;
CREATE POLICY moderation_cases_tenant_isolation ON moderation_cases FOR ALL USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- motion_votes
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS motion_votes_tenant_isolation ON motion_votes;
CREATE POLICY motion_votes_tenant_isolation ON motion_votes FOR ALL USING (((EXISTS ( SELECT 1 FROM (meeting_motions mm JOIN board_meetings bm ON ((bm.id = mm.meeting_id))) WHERE ((mm.id = motion_votes.motion_id) AND (bm.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- multi_currency_report_config
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS "Tenant isolation for multi_currency_report_config" ON multi_currency_report_config;
CREATE POLICY "Tenant isolation for multi_currency_report_config" ON multi_currency_report_config FOR ALL USING (((organization_id = (current_setting('app.current_tenant'::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- multi_currency_report_snapshots
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS "Tenant isolation for multi_currency_report_snapshots" ON multi_currency_report_snapshots;
CREATE POLICY "Tenant isolation for multi_currency_report_snapshots" ON multi_currency_report_snapshots FOR ALL USING (((organization_id = (current_setting('app.current_tenant'::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- multi_currency_transactions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS "Tenant isolation for multi_currency_transactions" ON multi_currency_transactions;
CREATE POLICY "Tenant isolation for multi_currency_transactions" ON multi_currency_transactions FOR ALL USING (((organization_id = (current_setting('app.current_tenant'::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- news_articles
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS news_articles_tenant_isolation ON news_articles;
CREATE POLICY news_articles_tenant_isolation ON news_articles FOR ALL USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- online_payment_sessions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS online_payment_sessions_manage ON online_payment_sessions;
CREATE POLICY online_payment_sessions_manage ON online_payment_sessions FOR ALL USING ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = online_payment_sessions.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid)))) OR is_super_admin())) AND get_current_org_not_deleted()) WITH CHECK ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = online_payment_sessions.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid)))) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS online_payment_sessions_select ON online_payment_sessions;
CREATE POLICY online_payment_sessions_select ON online_payment_sessions FOR SELECT USING ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = online_payment_sessions.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid)))) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- organization_connectors
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS org_connectors_policy ON organization_connectors;
CREATE POLICY org_connectors_policy ON organization_connectors FOR ALL USING (((organization_id = (NULLIF(current_setting('app.current_organization_id'::text, true), ''::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- organization_currency_config
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS "Tenant isolation for organization_currency_config" ON organization_currency_config;
CREATE POLICY "Tenant isolation for organization_currency_config" ON organization_currency_config FOR ALL USING (((organization_id = (current_setting('app.current_tenant'::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- organization_feature_preferences
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS org_feature_prefs_modify_policy ON organization_feature_preferences;
CREATE POLICY org_feature_prefs_modify_policy ON organization_feature_preferences FOR ALL USING (((EXISTS ( SELECT 1 FROM (organization_members om JOIN roles r ON ((om.role_id = r.id))) WHERE ((om.organization_id = organization_feature_preferences.organization_id) AND (om.user_id = (current_setting('app.user_id'::text, true))::uuid) AND (r.permissions ? 'organization:manage_features'::text))))) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS org_feature_prefs_select_policy ON organization_feature_preferences;
CREATE POLICY org_feature_prefs_select_policy ON organization_feature_preferences FOR SELECT USING (((EXISTS ( SELECT 1 FROM organization_members WHERE ((organization_members.organization_id = organization_feature_preferences.organization_id) AND (organization_members.user_id = (current_setting('app.user_id'::text, true))::uuid))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- organization_integrations
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS org_integrations_policy ON organization_integrations;
CREATE POLICY org_integrations_policy ON organization_integrations FOR ALL USING (((organization_id = (NULLIF(current_setting('app.current_organization_id'::text, true), ''::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- organization_packages
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS organization_packages_tenant_isolation ON organization_packages;
CREATE POLICY organization_packages_tenant_isolation ON organization_packages FOR ALL USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- organization_subscriptions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS org_subscriptions_tenant_isolation ON organization_subscriptions;
CREATE POLICY org_subscriptions_tenant_isolation ON organization_subscriptions FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- outage_notifications
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS outage_notifications_tenant_isolation ON outage_notifications;
CREATE POLICY outage_notifications_tenant_isolation ON outage_notifications FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM outages o WHERE ((o.id = outage_notifications.outage_id) AND (o.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM outages o WHERE ((o.id = outage_notifications.outage_id) AND (o.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- outages
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS outages_tenant_isolation ON outages;
CREATE POLICY outages_tenant_isolation ON outages FOR ALL USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- package_notifications
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS package_notifications_tenant_isolation ON package_notifications;
CREATE POLICY package_notifications_tenant_isolation ON package_notifications FOR ALL USING (((package_id IN ( SELECT packages.id FROM packages WHERE (packages.tenant_id = (current_setting('app.current_tenant_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- packages
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS packages_tenant_isolation ON packages;
CREATE POLICY packages_tenant_isolation ON packages FOR ALL USING (((tenant_id = (current_setting('app.current_tenant_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- parking_spots
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS parking_spots_insert ON parking_spots;
CREATE POLICY parking_spots_insert ON parking_spots FOR INSERT WITH CHECK (((tenant_id = (current_setting('app.current_tenant_id'::text))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS parking_spots_tenant_isolation ON parking_spots;
CREATE POLICY parking_spots_tenant_isolation ON parking_spots FOR ALL USING (((tenant_id = (current_setting('app.current_tenant_id'::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- party_submissions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS party_submissions_tenant_isolation ON party_submissions;
CREATE POLICY party_submissions_tenant_isolation ON party_submissions FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM disputes p WHERE ((p.id = party_submissions.dispute_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM disputes p WHERE ((p.id = party_submissions.dispute_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- payment_allocations
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS payment_allocations_manage ON payment_allocations;
CREATE POLICY payment_allocations_manage ON payment_allocations FOR ALL USING ((((EXISTS ( SELECT 1 FROM (payments p JOIN organization_members om ON ((om.organization_id = p.organization_id))) WHERE ((p.id = payment_allocations.payment_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted()) WITH CHECK ((((EXISTS ( SELECT 1 FROM (payments p JOIN organization_members om ON ((om.organization_id = p.organization_id))) WHERE ((p.id = payment_allocations.payment_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS payment_allocations_select ON payment_allocations;
CREATE POLICY payment_allocations_select ON payment_allocations FOR SELECT USING ((((EXISTS ( SELECT 1 FROM (payments p JOIN organization_members om ON ((om.organization_id = p.organization_id))) WHERE ((p.id = payment_allocations.payment_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid)))) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- payment_methods
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS payment_methods_tenant_isolation ON payment_methods;
CREATE POLICY payment_methods_tenant_isolation ON payment_methods FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- payments
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS payments_manage ON payments;
CREATE POLICY payments_manage ON payments FOR ALL USING ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = payments.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted()) WITH CHECK ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = payments.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS payments_select ON payments;
CREATE POLICY payments_select ON payments FOR SELECT USING ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = payments.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid)))) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- performance_alerts
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS performance_alerts_tenant_isolation ON performance_alerts;
CREATE POLICY performance_alerts_tenant_isolation ON performance_alerts FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM performance_portfolios p WHERE ((p.id = performance_alerts.portfolio_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM performance_portfolios p WHERE ((p.id = performance_alerts.portfolio_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- performance_portfolios
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS perf_portfolios_delete ON performance_portfolios;
CREATE POLICY perf_portfolios_delete ON performance_portfolios FOR DELETE USING (((organization_id IN ( SELECT organization_members.organization_id FROM organization_members WHERE (organization_members.user_id = (current_setting('app.current_user_id'::text, true))::uuid)))) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS perf_portfolios_insert ON performance_portfolios;
CREATE POLICY perf_portfolios_insert ON performance_portfolios FOR INSERT WITH CHECK (((organization_id IN ( SELECT organization_members.organization_id FROM organization_members WHERE (organization_members.user_id = (current_setting('app.current_user_id'::text, true))::uuid)))) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS perf_portfolios_select ON performance_portfolios;
CREATE POLICY perf_portfolios_select ON performance_portfolios FOR SELECT USING (((organization_id IN ( SELECT organization_members.organization_id FROM organization_members WHERE (organization_members.user_id = (current_setting('app.current_user_id'::text, true))::uuid)))) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS perf_portfolios_update ON performance_portfolios;
CREATE POLICY perf_portfolios_update ON performance_portfolios FOR UPDATE USING (((organization_id IN ( SELECT organization_members.organization_id FROM organization_members WHERE (organization_members.user_id = (current_setting('app.current_user_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- person_months
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS person_months_delete_manager ON person_months;
CREATE POLICY person_months_delete_manager ON person_months FOR DELETE USING (((EXISTS ( SELECT 1 FROM ((units u JOIN buildings b ON ((u.building_id = b.id))) JOIN organization_members om ON ((om.organization_id = b.organization_id))) WHERE ((u.id = person_months.unit_id) AND (om.user_id = (current_setting('app.current_user_id'::text, true))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[])))))) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS person_months_insert_manager ON person_months;
CREATE POLICY person_months_insert_manager ON person_months FOR INSERT WITH CHECK (((EXISTS ( SELECT 1 FROM ((units u JOIN buildings b ON ((u.building_id = b.id))) JOIN organization_members om ON ((om.organization_id = b.organization_id))) WHERE ((u.id = person_months.unit_id) AND (om.user_id = (current_setting('app.current_user_id'::text, true))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[])))))) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS person_months_select_org ON person_months;
CREATE POLICY person_months_select_org ON person_months FOR SELECT USING (((EXISTS ( SELECT 1 FROM (units u JOIN buildings b ON ((u.building_id = b.id))) WHERE ((u.id = person_months.unit_id) AND (b.organization_id = (current_setting('app.current_org_id'::text, true))::uuid))))) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS person_months_super_admin ON person_months;
CREATE POLICY person_months_super_admin ON person_months FOR ALL USING ((((current_setting('app.is_super_admin'::text, true))::boolean = true)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS person_months_update_manager ON person_months;
CREATE POLICY person_months_update_manager ON person_months FOR UPDATE USING (((EXISTS ( SELECT 1 FROM ((units u JOIN buildings b ON ((u.building_id = b.id))) JOIN organization_members om ON ((om.organization_id = b.organization_id))) WHERE ((u.id = person_months.unit_id) AND (om.user_id = (current_setting('app.current_user_id'::text, true))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[])))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- pet_registrations
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS pet_registrations_insert ON pet_registrations;
CREATE POLICY pet_registrations_insert ON pet_registrations FOR INSERT WITH CHECK (((tenant_id = (current_setting('app.current_tenant_id'::text))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS pet_registrations_tenant_isolation ON pet_registrations;
CREATE POLICY pet_registrations_tenant_isolation ON pet_registrations FOR ALL USING (((tenant_id = (current_setting('app.current_tenant_id'::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- portal_favorites
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS portal_favorites_tenant_isolation ON portal_favorites;
CREATE POLICY portal_favorites_tenant_isolation ON portal_favorites FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM listings p WHERE ((p.id = portal_favorites.listing_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM listings p WHERE ((p.id = portal_favorites.listing_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- portfolio_aggregated_metrics
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS "Tenant isolation for portfolio_aggregated_metrics" ON portfolio_aggregated_metrics;
CREATE POLICY "Tenant isolation for portfolio_aggregated_metrics" ON portfolio_aggregated_metrics FOR ALL USING (((organization_id = (current_setting('app.current_tenant'::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- portfolio_alert_rules
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS "Tenant isolation for portfolio_alert_rules" ON portfolio_alert_rules;
CREATE POLICY "Tenant isolation for portfolio_alert_rules" ON portfolio_alert_rules FOR ALL USING (((organization_id = (current_setting('app.current_tenant'::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- portfolio_alerts
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS "Tenant isolation for portfolio_alerts" ON portfolio_alerts;
CREATE POLICY "Tenant isolation for portfolio_alerts" ON portfolio_alerts FOR ALL USING (((organization_id = (current_setting('app.current_tenant'::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- portfolio_benchmarks
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS "Tenant isolation for portfolio_benchmarks" ON portfolio_benchmarks;
CREATE POLICY "Tenant isolation for portfolio_benchmarks" ON portfolio_benchmarks FOR ALL USING (((organization_id = (current_setting('app.current_tenant'::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- portfolio_properties
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS portfolio_properties_tenant_isolation ON portfolio_properties;
CREATE POLICY portfolio_properties_tenant_isolation ON portfolio_properties FOR ALL USING (((portfolio_id IN ( SELECT investment_portfolios.id FROM investment_portfolios WHERE (investment_portfolios.organization_id = (current_setting('app.current_tenant_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- portfolio_properties_perf
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS portfolio_properties_perf_tenant_isolation ON portfolio_properties_perf;
CREATE POLICY portfolio_properties_perf_tenant_isolation ON portfolio_properties_perf FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM performance_portfolios p WHERE ((p.id = portfolio_properties_perf.portfolio_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM performance_portfolios p WHERE ((p.id = portfolio_properties_perf.portfolio_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- portfolio_trends
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS "Tenant isolation for portfolio_trends" ON portfolio_trends;
CREATE POLICY "Tenant isolation for portfolio_trends" ON portfolio_trends FOR ALL USING (((organization_id = (current_setting('app.current_tenant'::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- pricing_recommendations
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS pricing_recommendations_tenant_isolation ON pricing_recommendations;
CREATE POLICY pricing_recommendations_tenant_isolation ON pricing_recommendations FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (units u JOIN buildings b ON ((b.id = u.building_id))) WHERE ((u.id = pricing_recommendations.unit_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (units u JOIN buildings b ON ((b.id = u.building_id))) WHERE ((u.id = pricing_recommendations.unit_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- property_cash_flows
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS property_cash_flows_tenant_isolation ON property_cash_flows;
CREATE POLICY property_cash_flows_tenant_isolation ON property_cash_flows FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM performance_portfolios p WHERE ((p.id = property_cash_flows.portfolio_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM performance_portfolios p WHERE ((p.id = property_cash_flows.portfolio_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- property_comparisons
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS "Tenant isolation for property_comparisons" ON property_comparisons;
CREATE POLICY "Tenant isolation for property_comparisons" ON property_comparisons FOR ALL USING (((organization_id = (current_setting('app.current_tenant'::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- property_currency_config
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS "Tenant isolation for property_currency_config" ON property_currency_config;
CREATE POLICY "Tenant isolation for property_currency_config" ON property_currency_config FOR ALL USING (((organization_id = (current_setting('app.current_tenant'::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- property_performance_metrics
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS "Tenant isolation for property_performance_metrics" ON property_performance_metrics;
CREATE POLICY "Tenant isolation for property_performance_metrics" ON property_performance_metrics FOR ALL USING (((organization_id = (current_setting('app.current_tenant'::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- property_transactions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS property_transactions_tenant_isolation ON property_transactions;
CREATE POLICY property_transactions_tenant_isolation ON property_transactions FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM performance_portfolios p WHERE ((p.id = property_transactions.portfolio_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM performance_portfolios p WHERE ((p.id = property_transactions.portfolio_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- property_valuation_features
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS valuation_features_tenant_isolation ON property_valuation_features;
CREATE POLICY valuation_features_tenant_isolation ON property_valuation_features FOR ALL USING (((organization_id = (current_setting('app.current_tenant'::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- property_valuation_models
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS valuation_models_tenant_isolation ON property_valuation_models;
CREATE POLICY valuation_models_tenant_isolation ON property_valuation_models FOR ALL USING (((organization_id = (current_setting('app.current_tenant'::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- property_valuations
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS property_valuations_tenant_isolation ON property_valuations;
CREATE POLICY property_valuations_tenant_isolation ON property_valuations FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (units u JOIN buildings b ON ((b.id = u.building_id))) WHERE ((u.id = property_valuations.unit_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (units u JOIN buildings b ON ((b.id = u.building_id))) WHERE ((u.id = property_valuations.unit_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- property_value_history
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS property_value_history_tenant_isolation ON property_value_history;
CREATE POLICY property_value_history_tenant_isolation ON property_value_history FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (units u JOIN buildings b ON ((b.id = u.building_id))) WHERE ((u.id = property_value_history.unit_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (units u JOIN buildings b ON ((b.id = u.building_id))) WHERE ((u.id = property_value_history.unit_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- provider_quotes
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS provider_quotes_tenant_isolation ON provider_quotes;
CREATE POLICY provider_quotes_tenant_isolation ON provider_quotes FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM rfqs p WHERE ((p.id = provider_quotes.rfq_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM rfqs p WHERE ((p.id = provider_quotes.rfq_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- provider_reviews
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS reviews_delete_own ON provider_reviews;
CREATE POLICY reviews_delete_own ON provider_reviews FOR DELETE USING (((reviewer_id = (current_setting('app.user_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS reviews_insert ON provider_reviews;
CREATE POLICY reviews_insert ON provider_reviews FOR INSERT WITH CHECK (((organization_id IN ( SELECT organization_members.organization_id FROM organization_members WHERE (organization_members.user_id = (current_setting('app.user_id'::text, true))::uuid)))) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS reviews_read_all ON provider_reviews;
CREATE POLICY reviews_read_all ON provider_reviews FOR SELECT USING (((((status)::text = 'approved'::text) OR (reviewer_id = (current_setting('app.user_id'::text, true))::uuid))) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS reviews_update_own ON provider_reviews;
CREATE POLICY reviews_update_own ON provider_reviews FOR UPDATE USING (((reviewer_id = (current_setting('app.user_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- reading_submission_windows
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS submission_windows_manage ON reading_submission_windows;
CREATE POLICY submission_windows_manage ON reading_submission_windows FOR ALL USING ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = reading_submission_windows.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted()) WITH CHECK ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = reading_submission_windows.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS submission_windows_select ON reading_submission_windows;
CREATE POLICY submission_windows_select ON reading_submission_windows FOR SELECT USING ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = reading_submission_windows.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid)))) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- reading_validation_rules
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS validation_rules_manage ON reading_validation_rules;
CREATE POLICY validation_rules_manage ON reading_validation_rules FOR ALL USING ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = reading_validation_rules.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted()) WITH CHECK ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = reading_validation_rules.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS validation_rules_select ON reading_validation_rules;
CREATE POLICY validation_rules_select ON reading_validation_rules FOR SELECT USING ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = reading_validation_rules.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- realtor_listings
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS realtor_listings_tenant_isolation ON realtor_listings;
CREATE POLICY realtor_listings_tenant_isolation ON realtor_listings FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM listings p WHERE ((p.id = realtor_listings.listing_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM listings p WHERE ((p.id = realtor_listings.listing_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- regulatory_submission_attachments
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS reg_submission_attachments_access ON regulatory_submission_attachments;
CREATE POLICY reg_submission_attachments_access ON regulatory_submission_attachments FOR ALL USING (((EXISTS ( SELECT 1 FROM regulatory_submissions s WHERE ((s.id = regulatory_submission_attachments.submission_id) AND ((s.organization_id = (NULLIF(current_setting('app.current_tenant_id'::text, true), ''::text))::uuid) OR is_super_admin()))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- regulatory_submission_audit
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS reg_submission_audit_access ON regulatory_submission_audit;
CREATE POLICY reg_submission_audit_access ON regulatory_submission_audit FOR SELECT USING (((EXISTS ( SELECT 1 FROM regulatory_submissions s WHERE ((s.id = regulatory_submission_audit.submission_id) AND ((s.organization_id = (NULLIF(current_setting('app.current_tenant_id'::text, true), ''::text))::uuid) OR is_super_admin()))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- regulatory_submission_schedules
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS reg_submission_schedules_org_access ON regulatory_submission_schedules;
CREATE POLICY reg_submission_schedules_org_access ON regulatory_submission_schedules FOR ALL USING ((((organization_id = (NULLIF(current_setting('app.current_tenant_id'::text, true), ''::text))::uuid) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- regulatory_submissions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS reg_submissions_org_access ON regulatory_submissions;
CREATE POLICY reg_submissions_org_access ON regulatory_submissions FOR ALL USING ((((organization_id = (NULLIF(current_setting('app.current_tenant_id'::text, true), ''::text))::uuid) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- reminder_logs
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS reminder_logs_manage ON reminder_logs;
CREATE POLICY reminder_logs_manage ON reminder_logs FOR ALL USING ((((EXISTS ( SELECT 1 FROM (invoices i JOIN organization_members om ON ((om.organization_id = i.organization_id))) WHERE ((i.id = reminder_logs.invoice_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted()) WITH CHECK ((((EXISTS ( SELECT 1 FROM (invoices i JOIN organization_members om ON ((om.organization_id = i.organization_id))) WHERE ((i.id = reminder_logs.invoice_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS reminder_logs_select ON reminder_logs;
CREATE POLICY reminder_logs_select ON reminder_logs FOR SELECT USING ((((EXISTS ( SELECT 1 FROM (invoices i JOIN organization_members om ON ((om.organization_id = i.organization_id))) WHERE ((i.id = reminder_logs.invoice_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- reminder_schedules
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS reminder_schedules_manage ON reminder_schedules;
CREATE POLICY reminder_schedules_manage ON reminder_schedules FOR ALL USING ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = reminder_schedules.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted()) WITH CHECK ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = reminder_schedules.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS reminder_schedules_select ON reminder_schedules;
CREATE POLICY reminder_schedules_select ON reminder_schedules FOR SELECT USING ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = reminder_schedules.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- rental_bookings
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS rental_bookings_tenant_isolation ON rental_bookings;
CREATE POLICY rental_bookings_tenant_isolation ON rental_bookings FOR ALL USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- rental_calendar_blocks
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS rental_calendar_blocks_tenant_isolation ON rental_calendar_blocks;
CREATE POLICY rental_calendar_blocks_tenant_isolation ON rental_calendar_blocks FOR ALL USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- rental_guest_reports
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS rental_guest_reports_tenant_isolation ON rental_guest_reports;
CREATE POLICY rental_guest_reports_tenant_isolation ON rental_guest_reports FOR ALL USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- rental_guests
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS rental_guests_tenant_isolation ON rental_guests;
CREATE POLICY rental_guests_tenant_isolation ON rental_guests FOR ALL USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- rental_ical_feeds
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS rental_ical_feeds_tenant_isolation ON rental_ical_feeds;
CREATE POLICY rental_ical_feeds_tenant_isolation ON rental_ical_feeds FOR ALL USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- rental_platform_connections
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS rental_platform_connections_tenant_isolation ON rental_platform_connections;
CREATE POLICY rental_platform_connections_tenant_isolation ON rental_platform_connections FOR ALL USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- reserve_funds
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS reserve_funds_tenant_isolation ON reserve_funds;
CREATE POLICY reserve_funds_tenant_isolation ON reserve_funds FOR ALL USING (((organization_id = (NULLIF(current_setting('app.current_organization_id'::text, true), ''::text))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- resolution_votes
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS resolution_votes_tenant_isolation ON resolution_votes;
CREATE POLICY resolution_votes_tenant_isolation ON resolution_votes FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (dispute_resolutions dr JOIN disputes p ON ((p.id = dr.dispute_id))) WHERE ((dr.id = resolution_votes.resolution_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (dispute_resolutions dr JOIN disputes p ON ((p.id = dr.dispute_id))) WHERE ((dr.id = resolution_votes.resolution_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- review_helpful_votes
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS review_helpful_votes_tenant_isolation ON review_helpful_votes;
CREATE POLICY review_helpful_votes_tenant_isolation ON review_helpful_votes FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM provider_reviews p WHERE ((p.id = review_helpful_votes.review_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM provider_reviews p WHERE ((p.id = review_helpful_votes.review_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- rfq_invitations
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS rfq_invitations_tenant_isolation ON rfq_invitations;
CREATE POLICY rfq_invitations_tenant_isolation ON rfq_invitations FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM rfqs p WHERE ((p.id = rfq_invitations.rfq_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM rfqs p WHERE ((p.id = rfq_invitations.rfq_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- rfqs
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS rfqs_org_access ON rfqs;
CREATE POLICY rfqs_org_access ON rfqs FOR ALL USING (((organization_id IN ( SELECT organization_members.organization_id FROM organization_members WHERE (organization_members.user_id = (current_setting('app.user_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- roi_calculations
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS roi_calculations_tenant_isolation ON roi_calculations;
CREATE POLICY roi_calculations_tenant_isolation ON roi_calculations FOR ALL USING (((organization_id = (current_setting('app.current_tenant_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- role_notification_defaults
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS role_notification_defaults_admin_access ON role_notification_defaults;
CREATE POLICY role_notification_defaults_admin_access ON role_notification_defaults FOR ALL USING ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = role_notification_defaults.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = 'org_admin'::text)))) OR is_super_admin())) AND get_current_org_not_deleted()) WITH CHECK ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = role_notification_defaults.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = 'org_admin'::text)))) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS role_notification_defaults_read_access ON role_notification_defaults;
CREATE POLICY role_notification_defaults_read_access ON role_notification_defaults FOR SELECT USING ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = role_notification_defaults.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid)))) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- schedule_executions
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS schedule_executions_tenant_isolation ON schedule_executions;
CREATE POLICY schedule_executions_tenant_isolation ON schedule_executions FOR ALL USING (((schedule_id IN ( SELECT maintenance_schedules.id FROM maintenance_schedules WHERE (maintenance_schedules.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- screening_ai_results
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS ai_results_insert ON screening_ai_results;
CREATE POLICY ai_results_insert ON screening_ai_results FOR INSERT WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS ai_results_select ON screening_ai_results;
CREATE POLICY ai_results_select ON screening_ai_results FOR SELECT USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- screening_background_results
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS background_results_insert ON screening_background_results;
CREATE POLICY background_results_insert ON screening_background_results FOR INSERT WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS background_results_select ON screening_background_results;
CREATE POLICY background_results_select ON screening_background_results FOR SELECT USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- screening_consent_audit_log
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS screening_audit_log_org_isolation ON screening_consent_audit_log;
CREATE POLICY screening_audit_log_org_isolation ON screening_consent_audit_log FOR ALL USING (((organization_id = (current_setting('app.current_org_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- screening_consent_requests
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS screening_consent_requests_org_isolation ON screening_consent_requests;
CREATE POLICY screening_consent_requests_org_isolation ON screening_consent_requests FOR ALL USING (((organization_id = (current_setting('app.current_org_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- screening_consents
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS screening_consents_org_isolation ON screening_consents;
CREATE POLICY screening_consents_org_isolation ON screening_consents FOR ALL USING (((organization_id = (current_setting('app.current_org_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- screening_credit_results
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS credit_results_insert ON screening_credit_results;
CREATE POLICY credit_results_insert ON screening_credit_results FOR INSERT WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS credit_results_select ON screening_credit_results;
CREATE POLICY credit_results_select ON screening_credit_results FOR SELECT USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- screening_data_access_requests
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS screening_access_requests_org_isolation ON screening_data_access_requests;
CREATE POLICY screening_access_requests_org_isolation ON screening_data_access_requests FOR ALL USING (((organization_id = (current_setting('app.current_org_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- screening_data_processing_records
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS screening_processing_org_isolation ON screening_data_processing_records;
CREATE POLICY screening_processing_org_isolation ON screening_data_processing_records FOR ALL USING (((organization_id = (current_setting('app.current_org_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- screening_eviction_results
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS eviction_results_insert ON screening_eviction_results;
CREATE POLICY eviction_results_insert ON screening_eviction_results FOR INSERT WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS eviction_results_select ON screening_eviction_results;
CREATE POLICY eviction_results_select ON screening_eviction_results FOR SELECT USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- screening_provider_configs
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS provider_configs_delete ON screening_provider_configs;
CREATE POLICY provider_configs_delete ON screening_provider_configs FOR DELETE USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS provider_configs_insert ON screening_provider_configs;
CREATE POLICY provider_configs_insert ON screening_provider_configs FOR INSERT WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS provider_configs_select ON screening_provider_configs;
CREATE POLICY provider_configs_select ON screening_provider_configs FOR SELECT USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS provider_configs_update ON screening_provider_configs;
CREATE POLICY provider_configs_update ON screening_provider_configs FOR UPDATE USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- screening_reports
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS reports_insert ON screening_reports;
CREATE POLICY reports_insert ON screening_reports FOR INSERT WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS reports_select ON screening_reports;
CREATE POLICY reports_select ON screening_reports FOR SELECT USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS reports_update ON screening_reports;
CREATE POLICY reports_update ON screening_reports FOR UPDATE USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- screening_request_queue
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS request_queue_insert ON screening_request_queue;
CREATE POLICY request_queue_insert ON screening_request_queue FOR INSERT WITH CHECK (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS request_queue_select ON screening_request_queue;
CREATE POLICY request_queue_select ON screening_request_queue FOR SELECT USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS request_queue_update ON screening_request_queue;
CREATE POLICY request_queue_update ON screening_request_queue FOR UPDATE USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- screening_risk_factors
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS risk_factors_insert ON screening_risk_factors;
CREATE POLICY risk_factors_insert ON screening_risk_factors FOR INSERT WITH CHECK (((EXISTS ( SELECT 1 FROM screening_ai_results WHERE ((screening_ai_results.id = screening_risk_factors.ai_result_id) AND (screening_ai_results.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid))))) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS risk_factors_select ON screening_risk_factors;
CREATE POLICY risk_factors_select ON screening_risk_factors FOR SELECT USING (((EXISTS ( SELECT 1 FROM screening_ai_results WHERE ((screening_ai_results.id = screening_risk_factors.ai_result_id) AND (screening_ai_results.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- search_embeddings
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS search_embeddings_tenant_isolation ON search_embeddings;
CREATE POLICY search_embeddings_tenant_isolation ON search_embeddings FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- search_history
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS search_history_tenant_isolation ON search_history;
CREATE POLICY search_history_tenant_isolation ON search_history FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- sensor_alerts
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS sensor_alerts_tenant_isolation ON sensor_alerts;
CREATE POLICY sensor_alerts_tenant_isolation ON sensor_alerts FOR ALL USING (((sensor_id IN ( SELECT sensors.id FROM sensors WHERE (sensors.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- sensor_fault_correlations
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS sensor_correlations_tenant_isolation ON sensor_fault_correlations;
CREATE POLICY sensor_correlations_tenant_isolation ON sensor_fault_correlations FOR ALL USING (((sensor_id IN ( SELECT sensors.id FROM sensors WHERE (sensors.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- sensor_readings
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS sensor_readings_tenant_isolation ON sensor_readings;
CREATE POLICY sensor_readings_tenant_isolation ON sensor_readings FOR ALL USING (((sensor_id IN ( SELECT sensors.id FROM sensors WHERE (sensors.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- sensor_threshold_templates
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS sensor_templates_tenant_isolation ON sensor_threshold_templates;
CREATE POLICY sensor_templates_tenant_isolation ON sensor_threshold_templates FOR ALL USING ((((organization_id IS NULL) OR (organization_id = (current_setting('app.current_organization_id'::text, true))::uuid))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- sensor_thresholds
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS sensor_thresholds_tenant_isolation ON sensor_thresholds;
CREATE POLICY sensor_thresholds_tenant_isolation ON sensor_thresholds FOR ALL USING (((sensor_id IN ( SELECT sensors.id FROM sensors WHERE (sensors.organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- sensors
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS sensors_tenant_isolation ON sensors;
CREATE POLICY sensors_tenant_isolation ON sensors FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- sentiment_alerts
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS sentiment_alerts_tenant_isolation ON sentiment_alerts;
CREATE POLICY sentiment_alerts_tenant_isolation ON sentiment_alerts FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- sentiment_thresholds
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS sentiment_thresholds_tenant_isolation ON sentiment_thresholds;
CREATE POLICY sentiment_thresholds_tenant_isolation ON sentiment_thresholds FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- sentiment_trends
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS sentiment_trends_tenant_isolation ON sentiment_trends;
CREATE POLICY sentiment_trends_tenant_isolation ON sentiment_trends FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- session_attendances
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS session_attendances_tenant_isolation ON session_attendances;
CREATE POLICY session_attendances_tenant_isolation ON session_attendances FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (mediation_sessions ms JOIN disputes p ON ((p.id = ms.dispute_id))) WHERE ((ms.id = session_attendances.session_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (mediation_sessions ms JOIN disputes p ON ((p.id = ms.dispute_id))) WHERE ((ms.id = session_attendances.session_id) AND (p.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- signature_requests
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS signature_requests_delete_policy ON signature_requests;
CREATE POLICY signature_requests_delete_policy ON signature_requests FOR DELETE USING ((((created_by = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) OR (EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = signature_requests.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = 'org_admin'::text)))) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS signature_requests_insert_policy ON signature_requests;
CREATE POLICY signature_requests_insert_policy ON signature_requests FOR INSERT WITH CHECK ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = signature_requests.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS signature_requests_select_policy ON signature_requests;
CREATE POLICY signature_requests_select_policy ON signature_requests FOR SELECT USING ((((created_by = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) OR (EXISTS ( SELECT 1 FROM jsonb_array_elements(signature_requests.signers) signer(value) WHERE ((signer.value ->> 'email'::text) = (( SELECT users.email FROM users WHERE (users.id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid)))::text))) OR (EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = signature_requests.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS signature_requests_update_policy ON signature_requests;
CREATE POLICY signature_requests_update_policy ON signature_requests FOR UPDATE USING ((((created_by = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) OR (EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = signature_requests.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = 'org_admin'::text)))) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- smart_meter_providers
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS smart_meter_providers_manage ON smart_meter_providers;
CREATE POLICY smart_meter_providers_manage ON smart_meter_providers FOR ALL USING ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = smart_meter_providers.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = 'org_admin'::text)))) OR is_super_admin())) AND get_current_org_not_deleted()) WITH CHECK ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = smart_meter_providers.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = 'org_admin'::text)))) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS smart_meter_providers_select ON smart_meter_providers;
CREATE POLICY smart_meter_providers_select ON smart_meter_providers FOR SELECT USING ((((EXISTS ( SELECT 1 FROM organization_members om WHERE ((om.organization_id = smart_meter_providers.organization_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = 'org_admin'::text)))) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- smart_meter_reading_logs
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS smart_reading_logs_manage ON smart_meter_reading_logs;
CREATE POLICY smart_reading_logs_manage ON smart_meter_reading_logs FOR ALL USING ((is_super_admin()) AND get_current_org_not_deleted()) WITH CHECK ((is_super_admin()) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS smart_reading_logs_select ON smart_meter_reading_logs;
CREATE POLICY smart_reading_logs_select ON smart_meter_reading_logs FOR SELECT USING ((((EXISTS ( SELECT 1 FROM (meters m JOIN organization_members om ON ((om.organization_id = m.organization_id))) WHERE ((m.id = smart_meter_reading_logs.meter_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- subscription_events
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS sub_events_tenant_isolation ON subscription_events;
CREATE POLICY sub_events_tenant_isolation ON subscription_events FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- subscription_invoices
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS sub_invoices_tenant_isolation ON subscription_invoices;
CREATE POLICY sub_invoices_tenant_isolation ON subscription_invoices FOR ALL USING (((organization_id = (current_setting('app.current_organization_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- suspicious_activities
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS suspicious_activities_tenant_isolation ON suspicious_activities;
CREATE POLICY suspicious_activities_tenant_isolation ON suspicious_activities FOR ALL USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- tenant_applications
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS tenant_applications_org_isolation ON tenant_applications;
CREATE POLICY tenant_applications_org_isolation ON tenant_applications FOR ALL USING (((organization_id = (current_setting('app.current_org_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- tenant_screenings
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS tenant_screenings_org_isolation ON tenant_screenings;
CREATE POLICY tenant_screenings_org_isolation ON tenant_screenings FOR ALL USING (((organization_id = (current_setting('app.current_org_id'::text, true))::uuid)) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- unit_credit_balances
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS unit_credit_balances_manage ON unit_credit_balances;
CREATE POLICY unit_credit_balances_manage ON unit_credit_balances FOR ALL USING ((((EXISTS ( SELECT 1 FROM ((units u JOIN buildings b ON ((b.id = u.building_id))) JOIN organization_members om ON ((om.organization_id = b.organization_id))) WHERE ((u.id = unit_credit_balances.unit_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted()) WITH CHECK ((((EXISTS ( SELECT 1 FROM ((units u JOIN buildings b ON ((b.id = u.building_id))) JOIN organization_members om ON ((om.organization_id = b.organization_id))) WHERE ((u.id = unit_credit_balances.unit_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS unit_credit_balances_select ON unit_credit_balances;
CREATE POLICY unit_credit_balances_select ON unit_credit_balances FOR SELECT USING ((((EXISTS ( SELECT 1 FROM ((units u JOIN buildings b ON ((b.id = u.building_id))) JOIN organization_members om ON ((om.organization_id = b.organization_id))) WHERE ((u.id = unit_credit_balances.unit_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid)))) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- unit_fees
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS unit_fees_manage ON unit_fees;
CREATE POLICY unit_fees_manage ON unit_fees FOR ALL USING ((((EXISTS ( SELECT 1 FROM (fee_schedules fs JOIN organization_members om ON ((om.organization_id = fs.organization_id))) WHERE ((fs.id = unit_fees.fee_schedule_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted()) WITH CHECK ((((EXISTS ( SELECT 1 FROM (fee_schedules fs JOIN organization_members om ON ((om.organization_id = fs.organization_id))) WHERE ((fs.id = unit_fees.fee_schedule_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid) AND ((om.role_type)::text = ANY ((ARRAY['org_admin'::character varying, 'manager'::character varying])::text[]))))) OR is_super_admin())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS unit_fees_select ON unit_fees;
CREATE POLICY unit_fees_select ON unit_fees FOR SELECT USING ((((EXISTS ( SELECT 1 FROM (fee_schedules fs JOIN organization_members om ON ((om.organization_id = fs.organization_id))) WHERE ((fs.id = unit_fees.fee_schedule_id) AND (om.user_id = (NULLIF(current_setting('app.current_user_id'::text, true), ''::text))::uuid)))) OR is_super_admin())) AND get_current_org_not_deleted());

-- ----------------------------------------------------------------------------
-- unit_owners
-- ----------------------------------------------------------------------------
DROP POLICY IF EXISTS unit_owners_tenant_isolation ON unit_owners;
CREATE POLICY unit_owners_tenant_isolation ON unit_owners FOR ALL USING (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (units u JOIN buildings b ON ((b.id = u.building_id))) WHERE ((u.id = unit_owners.unit_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted()) WITH CHECK (((is_super_admin() OR (EXISTS ( SELECT 1 FROM (units u JOIN buildings b ON ((b.id = u.building_id))) WHERE ((u.id = unit_owners.unit_id) AND (b.organization_id = get_current_org_id())))))) AND get_current_org_not_deleted());
