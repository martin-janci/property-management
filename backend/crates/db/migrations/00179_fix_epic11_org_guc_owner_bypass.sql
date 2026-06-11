-- Migration: 00179_fix_epic11_org_guc_owner_bypass
-- Security Fix (PAP-62): close the `app.current_organization_id` RLS
-- owner-bypass + GUC-mismatch cluster across the Epic-11+ feature tables.
--
-- Background
-- ----------
-- A large cluster of Epic-11+ feature tables (work orders, budgets, vendors,
-- equipment, ESG, screening, insurance, emergency, certifications, meetings,
-- forms, sensors, workflows, document-intelligence, etc.) shipped their
-- tenant-isolation RLS policies against `current_setting('app.current_organization_id')`
-- and only `ENABLE`d RLS -- never `FORCE`.
--
-- Two compounding defects made the isolation non-functional:
--
--   1. GUC mismatch. The codebase NEVER sets `app.current_organization_id`.
--      `set_request_context` (00004) sets `app.current_org_id`, read by the
--      `get_current_org_id()` helper. So for any non-owner role these policies
--      evaluated `organization_id = NULL` -> deny-all, and the feature only
--      worked because of defect (2).
--   2. Owner-bypass. `ENABLE` (without `FORCE`) is bypassed for the table owner
--      and for superuser/`BYPASSRLS` roles. The production api-server connects
--      as the table OWNER, so RLS was bypassed entirely -> every tenant context
--      saw EVERY org's rows: a cross-tenant IDOR / data-leak path (PAP-51 exit
--      gate: "no known cross-tenant data-leak paths").
--
-- This is the same class of bug already fixed for documents (#754 / 00172),
-- messaging (#898 / 00171) and the Epic 8A notification tables (#755 / 00177).
-- This migration brings the remaining `app.current_organization_id` cluster in
-- line with the project convention:
--
--   * rewrite every policy expression
--       (current_setting('app.current_organization_id', ...))::uuid
--     to the canonical `get_current_org_id()` helper, and
--   * `ALTER TABLE ... FORCE ROW LEVEL SECURITY` so the owner role is bound by
--     the policy instead of bypassing it.
--
-- Policy *logic* is otherwise preserved verbatim (same subquery joins, same
-- soft-delete `get_current_org_not_deleted()` guard where 00140 added it). No
-- super-admin OR-leg is added where one did not already exist -- this is the
-- minimal change that closes the leak without broadening access.
--
-- Scope note: tables whose policies use a DIFFERENT unset GUC
-- (`app.current_tenant`, `app.current_tenant_id`, `app.tenant_id`) are NOT in
-- this cluster and are handled by their own sibling tickets. Generated and
-- verified by /tmp analysis over the full migration history -- 135 tables,
-- 174 policies.

-- ------------------------------------------------------------------------
-- ai_chat_messages
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS ai_chat_messages_tenant_isolation ON ai_chat_messages;
CREATE POLICY ai_chat_messages_tenant_isolation ON ai_chat_messages FOR ALL USING (((session_id IN ( SELECT ai_chat_sessions.id FROM ai_chat_sessions WHERE (ai_chat_sessions.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE ai_chat_messages FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- ai_chat_sessions
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS ai_chat_sessions_tenant_isolation ON ai_chat_sessions;
CREATE POLICY ai_chat_sessions_tenant_isolation ON ai_chat_sessions FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE ai_chat_sessions FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- ai_risk_scoring_models
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS ai_risk_models_select ON ai_risk_scoring_models;
CREATE POLICY ai_risk_models_select ON ai_risk_scoring_models FOR SELECT USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS ai_risk_models_insert ON ai_risk_scoring_models;
CREATE POLICY ai_risk_models_insert ON ai_risk_scoring_models FOR INSERT WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS ai_risk_models_update ON ai_risk_scoring_models;
CREATE POLICY ai_risk_models_update ON ai_risk_scoring_models FOR UPDATE USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS ai_risk_models_delete ON ai_risk_scoring_models;
CREATE POLICY ai_risk_models_delete ON ai_risk_scoring_models FOR DELETE USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE ai_risk_scoring_models FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- ai_training_feedback
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS ai_training_feedback_tenant_isolation ON ai_training_feedback;
CREATE POLICY ai_training_feedback_tenant_isolation ON ai_training_feedback FOR ALL USING (((message_id IN ( SELECT m.id FROM (ai_chat_messages m JOIN ai_chat_sessions s ON ((s.id = m.session_id))) WHERE (s.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE ai_training_feedback FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- board_meetings
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS board_meetings_tenant_isolation ON board_meetings;
CREATE POLICY board_meetings_tenant_isolation ON board_meetings FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE board_meetings FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- board_members
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS board_members_tenant_isolation ON board_members;
CREATE POLICY board_members_tenant_isolation ON board_members FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE board_members FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- budget_actuals
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS budget_actuals_tenant_isolation ON budget_actuals;
CREATE POLICY budget_actuals_tenant_isolation ON budget_actuals FOR ALL USING (((budget_item_id IN ( SELECT bi.id FROM (budget_items bi JOIN budgets b ON ((b.id = bi.budget_id))) WHERE (b.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE budget_actuals FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- budget_categories
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS budget_categories_tenant_isolation ON budget_categories;
CREATE POLICY budget_categories_tenant_isolation ON budget_categories FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE budget_categories FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- budget_items
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS budget_items_tenant_isolation ON budget_items;
CREATE POLICY budget_items_tenant_isolation ON budget_items FOR ALL USING (((budget_id IN ( SELECT budgets.id FROM budgets WHERE (budgets.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE budget_items FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- budget_variance_alerts
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS budget_alerts_tenant_isolation ON budget_variance_alerts;
CREATE POLICY budget_alerts_tenant_isolation ON budget_variance_alerts FOR ALL USING (((budget_item_id IN ( SELECT bi.id FROM (budget_items bi JOIN budgets b ON ((b.id = bi.budget_id))) WHERE (b.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE budget_variance_alerts FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- budgets
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS budgets_tenant_isolation ON budgets;
CREATE POLICY budgets_tenant_isolation ON budgets FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE budgets FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- building_certifications
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS building_certifications_org_policy ON building_certifications;
CREATE POLICY building_certifications_org_policy ON building_certifications FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE building_certifications FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- capital_plans
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS capital_plans_tenant_isolation ON capital_plans;
CREATE POLICY capital_plans_tenant_isolation ON capital_plans FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE capital_plans FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- carbon_footprints
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS carbon_footprints_select ON carbon_footprints;
CREATE POLICY carbon_footprints_select ON carbon_footprints FOR SELECT USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS carbon_footprints_insert ON carbon_footprints;
CREATE POLICY carbon_footprints_insert ON carbon_footprints FOR INSERT WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS carbon_footprints_update ON carbon_footprints;
CREATE POLICY carbon_footprints_update ON carbon_footprints FOR UPDATE USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE carbon_footprints FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- certification_audit_logs
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS certification_audit_logs_org_policy ON certification_audit_logs;
CREATE POLICY certification_audit_logs_org_policy ON certification_audit_logs FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE certification_audit_logs FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- certification_benchmarks
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS certification_benchmarks_org_policy ON certification_benchmarks;
CREATE POLICY certification_benchmarks_org_policy ON certification_benchmarks FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE certification_benchmarks FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- certification_costs
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS certification_costs_org_policy ON certification_costs;
CREATE POLICY certification_costs_org_policy ON certification_costs FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE certification_costs FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- certification_credits
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS certification_credits_org_policy ON certification_credits;
CREATE POLICY certification_credits_org_policy ON certification_credits FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE certification_credits FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- certification_documents
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS certification_documents_org_policy ON certification_documents;
CREATE POLICY certification_documents_org_policy ON certification_documents FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE certification_documents FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- certification_milestones
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS certification_milestones_org_policy ON certification_milestones;
CREATE POLICY certification_milestones_org_policy ON certification_milestones FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE certification_milestones FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- certification_reminders
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS certification_reminders_org_policy ON certification_reminders;
CREATE POLICY certification_reminders_org_policy ON certification_reminders FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE certification_reminders FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- compliance_audit_trail
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS compliance_audit_tenant_isolation ON compliance_audit_trail;
CREATE POLICY compliance_audit_tenant_isolation ON compliance_audit_trail FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE compliance_audit_trail FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- compliance_requirements
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS compliance_req_tenant_isolation ON compliance_requirements;
CREATE POLICY compliance_req_tenant_isolation ON compliance_requirements FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE compliance_requirements FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- compliance_templates
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS compliance_templates_tenant_isolation ON compliance_templates;
CREATE POLICY compliance_templates_tenant_isolation ON compliance_templates FOR ALL USING ((((organization_id IS NULL) OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());
ALTER TABLE compliance_templates FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- compliance_verifications
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS compliance_verifications_tenant_isolation ON compliance_verifications;
CREATE POLICY compliance_verifications_tenant_isolation ON compliance_verifications FOR ALL USING (((requirement_id IN ( SELECT compliance_requirements.id FROM compliance_requirements WHERE (compliance_requirements.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE compliance_verifications FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- connector_execution_logs
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS connector_logs_policy ON connector_execution_logs;
CREATE POLICY connector_logs_policy ON connector_execution_logs FOR ALL USING (((org_connector_id IN ( SELECT organization_connectors.id FROM organization_connectors WHERE (organization_connectors.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE connector_execution_logs FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- coupon_redemptions
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS coupon_redemptions_tenant_isolation ON coupon_redemptions;
CREATE POLICY coupon_redemptions_tenant_isolation ON coupon_redemptions FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE coupon_redemptions FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- document_classification_history
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS document_classification_history_tenant_isolation ON document_classification_history;
CREATE POLICY document_classification_history_tenant_isolation ON document_classification_history FOR ALL USING (((EXISTS ( SELECT 1 FROM documents d WHERE ((d.id = document_classification_history.document_id) AND (d.organization_id = get_current_org_id()))))) AND get_current_org_not_deleted());
ALTER TABLE document_classification_history FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- document_embeddings
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS document_embeddings_org_isolation ON document_embeddings;
CREATE POLICY document_embeddings_org_isolation ON document_embeddings FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted()) WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE document_embeddings FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- document_intelligence_stats
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS document_intelligence_stats_tenant_isolation ON document_intelligence_stats;
CREATE POLICY document_intelligence_stats_tenant_isolation ON document_intelligence_stats FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE document_intelligence_stats FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- document_ocr_queue
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS document_ocr_queue_tenant_isolation ON document_ocr_queue;
CREATE POLICY document_ocr_queue_tenant_isolation ON document_ocr_queue FOR ALL USING (((EXISTS ( SELECT 1 FROM documents d WHERE ((d.id = document_ocr_queue.document_id) AND (d.organization_id = get_current_org_id()))))) AND get_current_org_not_deleted());
ALTER TABLE document_ocr_queue FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- document_summarization_queue
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS document_summarization_queue_tenant_isolation ON document_summarization_queue;
CREATE POLICY document_summarization_queue_tenant_isolation ON document_summarization_queue FOR ALL USING (((EXISTS ( SELECT 1 FROM documents d WHERE ((d.id = document_summarization_queue.document_id) AND (d.organization_id = get_current_org_id()))))) AND get_current_org_not_deleted());
ALTER TABLE document_summarization_queue FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- emergency_broadcast_acknowledgments
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS emergency_broadcast_acks_tenant_isolation ON emergency_broadcast_acknowledgments;
CREATE POLICY emergency_broadcast_acks_tenant_isolation ON emergency_broadcast_acknowledgments FOR ALL USING (((EXISTS ( SELECT 1 FROM emergency_broadcasts b WHERE ((b.id = emergency_broadcast_acknowledgments.broadcast_id) AND (b.organization_id = get_current_org_id()))))) AND get_current_org_not_deleted());
ALTER TABLE emergency_broadcast_acknowledgments FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- emergency_broadcasts
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS emergency_broadcasts_tenant_isolation ON emergency_broadcasts;
CREATE POLICY emergency_broadcasts_tenant_isolation ON emergency_broadcasts FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE emergency_broadcasts FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- emergency_contacts
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS emergency_contacts_tenant_isolation ON emergency_contacts;
CREATE POLICY emergency_contacts_tenant_isolation ON emergency_contacts FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE emergency_contacts FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- emergency_drills
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS emergency_drills_tenant_isolation ON emergency_drills;
CREATE POLICY emergency_drills_tenant_isolation ON emergency_drills FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE emergency_drills FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- emergency_incident_attachments
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS emergency_incident_attachments_tenant_isolation ON emergency_incident_attachments;
CREATE POLICY emergency_incident_attachments_tenant_isolation ON emergency_incident_attachments FOR ALL USING (((EXISTS ( SELECT 1 FROM emergency_incidents i WHERE ((i.id = emergency_incident_attachments.incident_id) AND (i.organization_id = get_current_org_id()))))) AND get_current_org_not_deleted());
ALTER TABLE emergency_incident_attachments FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- emergency_incident_updates
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS emergency_incident_updates_tenant_isolation ON emergency_incident_updates;
CREATE POLICY emergency_incident_updates_tenant_isolation ON emergency_incident_updates FOR ALL USING (((EXISTS ( SELECT 1 FROM emergency_incidents i WHERE ((i.id = emergency_incident_updates.incident_id) AND (i.organization_id = get_current_org_id()))))) AND get_current_org_not_deleted());
ALTER TABLE emergency_incident_updates FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- emergency_incidents
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS emergency_incidents_tenant_isolation ON emergency_incidents;
CREATE POLICY emergency_incidents_tenant_isolation ON emergency_incidents FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE emergency_incidents FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- emergency_protocols
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS emergency_protocols_tenant_isolation ON emergency_protocols;
CREATE POLICY emergency_protocols_tenant_isolation ON emergency_protocols FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE emergency_protocols FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- equipment
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS equipment_tenant_isolation ON equipment;
CREATE POLICY equipment_tenant_isolation ON equipment FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE equipment FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- equipment_documents
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS equipment_documents_tenant_isolation ON equipment_documents;
CREATE POLICY equipment_documents_tenant_isolation ON equipment_documents FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted()) WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE equipment_documents FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- equipment_health_thresholds
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS health_thresholds_tenant_isolation ON equipment_health_thresholds;
CREATE POLICY health_thresholds_tenant_isolation ON equipment_health_thresholds FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted()) WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE equipment_health_thresholds FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- equipment_maintenance
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS equipment_maintenance_tenant_isolation ON equipment_maintenance;
CREATE POLICY equipment_maintenance_tenant_isolation ON equipment_maintenance FOR ALL USING (((equipment_id IN ( SELECT equipment.id FROM equipment WHERE (equipment.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE equipment_maintenance FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- equipment_predictions
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS equipment_predictions_tenant_isolation ON equipment_predictions;
CREATE POLICY equipment_predictions_tenant_isolation ON equipment_predictions FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted()) WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE equipment_predictions FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- equipment_registry
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS equipment_registry_tenant_isolation ON equipment_registry;
CREATE POLICY equipment_registry_tenant_isolation ON equipment_registry FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted()) WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS equipment_registry_select_policy ON equipment_registry;
CREATE POLICY equipment_registry_select_policy ON equipment_registry FOR SELECT USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS equipment_registry_insert_policy ON equipment_registry;
CREATE POLICY equipment_registry_insert_policy ON equipment_registry FOR INSERT WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS equipment_registry_update_policy ON equipment_registry;
CREATE POLICY equipment_registry_update_policy ON equipment_registry FOR UPDATE USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS equipment_registry_delete_policy ON equipment_registry;
CREATE POLICY equipment_registry_delete_policy ON equipment_registry FOR DELETE USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE equipment_registry FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- esg_benchmarks
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS esg_benchmarks_select ON esg_benchmarks;
CREATE POLICY esg_benchmarks_select ON esg_benchmarks FOR SELECT USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_benchmarks_insert ON esg_benchmarks;
CREATE POLICY esg_benchmarks_insert ON esg_benchmarks FOR INSERT WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_benchmarks_update ON esg_benchmarks;
CREATE POLICY esg_benchmarks_update ON esg_benchmarks FOR UPDATE USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE esg_benchmarks FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- esg_configurations
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS esg_configs_select ON esg_configurations;
CREATE POLICY esg_configs_select ON esg_configurations FOR SELECT USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_configs_insert ON esg_configurations;
CREATE POLICY esg_configs_insert ON esg_configurations FOR INSERT WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_configs_update ON esg_configurations;
CREATE POLICY esg_configs_update ON esg_configurations FOR UPDATE USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE esg_configurations FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- esg_dashboard_metrics
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS esg_dashboard_select ON esg_dashboard_metrics;
CREATE POLICY esg_dashboard_select ON esg_dashboard_metrics FOR SELECT USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_dashboard_insert ON esg_dashboard_metrics;
CREATE POLICY esg_dashboard_insert ON esg_dashboard_metrics FOR INSERT WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_dashboard_update ON esg_dashboard_metrics;
CREATE POLICY esg_dashboard_update ON esg_dashboard_metrics FOR UPDATE USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE esg_dashboard_metrics FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- esg_import_jobs
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS esg_import_select ON esg_import_jobs;
CREATE POLICY esg_import_select ON esg_import_jobs FOR SELECT USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_import_insert ON esg_import_jobs;
CREATE POLICY esg_import_insert ON esg_import_jobs FOR INSERT WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_import_update ON esg_import_jobs;
CREATE POLICY esg_import_update ON esg_import_jobs FOR UPDATE USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE esg_import_jobs FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- esg_metrics
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS esg_metrics_select ON esg_metrics;
CREATE POLICY esg_metrics_select ON esg_metrics FOR SELECT USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_metrics_insert ON esg_metrics;
CREATE POLICY esg_metrics_insert ON esg_metrics FOR INSERT WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_metrics_update ON esg_metrics;
CREATE POLICY esg_metrics_update ON esg_metrics FOR UPDATE USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_metrics_delete ON esg_metrics;
CREATE POLICY esg_metrics_delete ON esg_metrics FOR DELETE USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE esg_metrics FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- esg_reports
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS esg_reports_select ON esg_reports;
CREATE POLICY esg_reports_select ON esg_reports FOR SELECT USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_reports_insert ON esg_reports;
CREATE POLICY esg_reports_insert ON esg_reports FOR INSERT WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_reports_update ON esg_reports;
CREATE POLICY esg_reports_update ON esg_reports FOR UPDATE USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE esg_reports FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- esg_targets
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS esg_targets_select ON esg_targets;
CREATE POLICY esg_targets_select ON esg_targets FOR SELECT USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_targets_insert ON esg_targets;
CREATE POLICY esg_targets_insert ON esg_targets FOR INSERT WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_targets_update ON esg_targets;
CREATE POLICY esg_targets_update ON esg_targets FOR UPDATE USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS esg_targets_delete ON esg_targets;
CREATE POLICY esg_targets_delete ON esg_targets FOR DELETE USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE esg_targets FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- eu_taxonomy_assessments
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS eu_taxonomy_select ON eu_taxonomy_assessments;
CREATE POLICY eu_taxonomy_select ON eu_taxonomy_assessments FOR SELECT USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS eu_taxonomy_insert ON eu_taxonomy_assessments;
CREATE POLICY eu_taxonomy_insert ON eu_taxonomy_assessments FOR INSERT WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS eu_taxonomy_update ON eu_taxonomy_assessments;
CREATE POLICY eu_taxonomy_update ON eu_taxonomy_assessments FOR UPDATE USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE eu_taxonomy_assessments FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- financial_forecasts
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS financial_forecasts_tenant_isolation ON financial_forecasts;
CREATE POLICY financial_forecasts_tenant_isolation ON financial_forecasts FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE financial_forecasts FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- form_downloads
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS form_downloads_org_isolation ON form_downloads;
CREATE POLICY form_downloads_org_isolation ON form_downloads FOR ALL USING (((form_id IN ( SELECT forms.id FROM forms WHERE (forms.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE form_downloads FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- form_fields
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS form_fields_org_isolation ON form_fields;
CREATE POLICY form_fields_org_isolation ON form_fields FOR ALL USING (((form_id IN ( SELECT forms.id FROM forms WHERE (forms.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE form_fields FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- form_submissions
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS form_submissions_org_isolation ON form_submissions;
CREATE POLICY form_submissions_org_isolation ON form_submissions FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE form_submissions FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- forms
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS forms_org_isolation ON forms;
CREATE POLICY forms_org_isolation ON forms FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE forms FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- fund_alerts
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS fund_alerts_tenant_isolation ON fund_alerts;
CREATE POLICY fund_alerts_tenant_isolation ON fund_alerts FOR ALL USING (((fund_id IN ( SELECT reserve_funds.id FROM reserve_funds WHERE (reserve_funds.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE fund_alerts FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- fund_components
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS fund_components_tenant_isolation ON fund_components;
CREATE POLICY fund_components_tenant_isolation ON fund_components FOR ALL USING (((fund_id IN ( SELECT reserve_funds.id FROM reserve_funds WHERE (reserve_funds.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE fund_components FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- fund_contribution_schedules
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS contribution_schedules_tenant_isolation ON fund_contribution_schedules;
CREATE POLICY contribution_schedules_tenant_isolation ON fund_contribution_schedules FOR ALL USING (((fund_id IN ( SELECT reserve_funds.id FROM reserve_funds WHERE (reserve_funds.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE fund_contribution_schedules FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- fund_investment_policies
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS investment_policies_tenant_isolation ON fund_investment_policies;
CREATE POLICY investment_policies_tenant_isolation ON fund_investment_policies FOR ALL USING (((fund_id IN ( SELECT reserve_funds.id FROM reserve_funds WHERE (reserve_funds.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE fund_investment_policies FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- fund_projection_items
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS projection_items_tenant_isolation ON fund_projection_items;
CREATE POLICY projection_items_tenant_isolation ON fund_projection_items FOR ALL USING (((projection_id IN ( SELECT fp.id FROM (fund_projections fp JOIN reserve_funds rf ON ((rf.id = fp.fund_id))) WHERE (rf.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE fund_projection_items FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- fund_projections
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS fund_projections_tenant_isolation ON fund_projections;
CREATE POLICY fund_projections_tenant_isolation ON fund_projections FOR ALL USING (((fund_id IN ( SELECT reserve_funds.id FROM reserve_funds WHERE (reserve_funds.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE fund_projections FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- fund_transactions
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS fund_transactions_tenant_isolation ON fund_transactions;
CREATE POLICY fund_transactions_tenant_isolation ON fund_transactions FOR ALL USING (((fund_id IN ( SELECT reserve_funds.id FROM reserve_funds WHERE (reserve_funds.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE fund_transactions FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- insurance_claim_documents
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS insurance_claim_documents_org_isolation ON insurance_claim_documents;
CREATE POLICY insurance_claim_documents_org_isolation ON insurance_claim_documents FOR ALL USING (((claim_id IN ( SELECT insurance_claims.id FROM insurance_claims WHERE (insurance_claims.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE insurance_claim_documents FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- insurance_claim_history
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS insurance_claim_history_org_isolation ON insurance_claim_history;
CREATE POLICY insurance_claim_history_org_isolation ON insurance_claim_history FOR ALL USING (((claim_id IN ( SELECT insurance_claims.id FROM insurance_claims WHERE (insurance_claims.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE insurance_claim_history FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- insurance_claims
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS insurance_claims_org_isolation ON insurance_claims;
CREATE POLICY insurance_claims_org_isolation ON insurance_claims FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE insurance_claims FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- insurance_policies
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS insurance_policies_org_isolation ON insurance_policies;
CREATE POLICY insurance_policies_org_isolation ON insurance_policies FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE insurance_policies FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- insurance_policy_documents
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS insurance_policy_documents_org_isolation ON insurance_policy_documents;
CREATE POLICY insurance_policy_documents_org_isolation ON insurance_policy_documents FOR ALL USING (((policy_id IN ( SELECT insurance_policies.id FROM insurance_policies WHERE (insurance_policies.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE insurance_policy_documents FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- insurance_renewal_reminders
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS insurance_renewal_reminders_org_isolation ON insurance_renewal_reminders;
CREATE POLICY insurance_renewal_reminders_org_isolation ON insurance_renewal_reminders FOR ALL USING (((policy_id IN ( SELECT insurance_policies.id FROM insurance_policies WHERE (insurance_policies.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE insurance_renewal_reminders FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- integration_ratings
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS integration_ratings_write ON integration_ratings;
CREATE POLICY integration_ratings_write ON integration_ratings FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE integration_ratings FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- invoice_line_items
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS invoice_lines_tenant_isolation ON invoice_line_items;
CREATE POLICY invoice_lines_tenant_isolation ON invoice_line_items FOR ALL USING (((invoice_id IN ( SELECT subscription_invoices.id FROM subscription_invoices WHERE (subscription_invoices.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE invoice_line_items FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- legal_document_versions
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS legal_versions_tenant_isolation ON legal_document_versions;
CREATE POLICY legal_versions_tenant_isolation ON legal_document_versions FOR ALL USING (((document_id IN ( SELECT legal_documents.id FROM legal_documents WHERE (legal_documents.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE legal_document_versions FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- legal_documents
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS legal_documents_tenant_isolation ON legal_documents;
CREATE POLICY legal_documents_tenant_isolation ON legal_documents FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE legal_documents FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- legal_notice_recipients
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS notice_recipients_tenant_isolation ON legal_notice_recipients;
CREATE POLICY notice_recipients_tenant_isolation ON legal_notice_recipients FOR ALL USING (((notice_id IN ( SELECT legal_notices.id FROM legal_notices WHERE (legal_notices.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE legal_notice_recipients FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- legal_notices
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS legal_notices_tenant_isolation ON legal_notices;
CREATE POLICY legal_notices_tenant_isolation ON legal_notices FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE legal_notices FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- maintenance_alerts
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS maintenance_alerts_tenant_isolation ON maintenance_alerts;
CREATE POLICY maintenance_alerts_tenant_isolation ON maintenance_alerts FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted()) WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE maintenance_alerts FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- maintenance_log_photos
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS maintenance_log_photos_tenant_isolation ON maintenance_log_photos;
CREATE POLICY maintenance_log_photos_tenant_isolation ON maintenance_log_photos FOR ALL USING (((maintenance_log_id IN ( SELECT maintenance_logs.id FROM maintenance_logs WHERE (maintenance_logs.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE maintenance_log_photos FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- maintenance_logs
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS maintenance_logs_tenant_isolation ON maintenance_logs;
CREATE POLICY maintenance_logs_tenant_isolation ON maintenance_logs FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted()) WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE maintenance_logs FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- maintenance_predictions
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS maintenance_predictions_tenant_isolation ON maintenance_predictions;
CREATE POLICY maintenance_predictions_tenant_isolation ON maintenance_predictions FOR ALL USING (((equipment_id IN ( SELECT equipment.id FROM equipment WHERE (equipment.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE maintenance_predictions FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- maintenance_schedules
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS maintenance_schedules_tenant_isolation ON maintenance_schedules;
CREATE POLICY maintenance_schedules_tenant_isolation ON maintenance_schedules FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE maintenance_schedules FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- meeting_action_items
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS action_items_tenant_isolation ON meeting_action_items;
CREATE POLICY action_items_tenant_isolation ON meeting_action_items FOR ALL USING (((EXISTS ( SELECT 1 FROM board_meetings bm WHERE ((bm.id = meeting_action_items.meeting_id) AND (bm.organization_id = get_current_org_id()))))) AND get_current_org_not_deleted());
ALTER TABLE meeting_action_items FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- meeting_agenda_items
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS agenda_items_tenant_isolation ON meeting_agenda_items;
CREATE POLICY agenda_items_tenant_isolation ON meeting_agenda_items FOR ALL USING (((EXISTS ( SELECT 1 FROM board_meetings bm WHERE ((bm.id = meeting_agenda_items.meeting_id) AND (bm.organization_id = get_current_org_id()))))) AND get_current_org_not_deleted());
ALTER TABLE meeting_agenda_items FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- meeting_attendance
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS attendance_tenant_isolation ON meeting_attendance;
CREATE POLICY attendance_tenant_isolation ON meeting_attendance FOR ALL USING (((EXISTS ( SELECT 1 FROM board_meetings bm WHERE ((bm.id = meeting_attendance.meeting_id) AND (bm.organization_id = get_current_org_id()))))) AND get_current_org_not_deleted());
ALTER TABLE meeting_attendance FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- meeting_documents
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS meeting_docs_tenant_isolation ON meeting_documents;
CREATE POLICY meeting_docs_tenant_isolation ON meeting_documents FOR ALL USING (((EXISTS ( SELECT 1 FROM board_meetings bm WHERE ((bm.id = meeting_documents.meeting_id) AND (bm.organization_id = get_current_org_id()))))) AND get_current_org_not_deleted());
ALTER TABLE meeting_documents FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- meeting_minutes
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS minutes_tenant_isolation ON meeting_minutes;
CREATE POLICY minutes_tenant_isolation ON meeting_minutes FOR ALL USING (((EXISTS ( SELECT 1 FROM board_meetings bm WHERE ((bm.id = meeting_minutes.meeting_id) AND (bm.organization_id = get_current_org_id()))))) AND get_current_org_not_deleted());
ALTER TABLE meeting_minutes FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- meeting_motions
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS motions_tenant_isolation ON meeting_motions;
CREATE POLICY motions_tenant_isolation ON meeting_motions FOR ALL USING (((EXISTS ( SELECT 1 FROM board_meetings bm WHERE ((bm.id = meeting_motions.meeting_id) AND (bm.organization_id = get_current_org_id()))))) AND get_current_org_not_deleted());
ALTER TABLE meeting_motions FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- meeting_statistics
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS meeting_stats_tenant_isolation ON meeting_statistics;
CREATE POLICY meeting_stats_tenant_isolation ON meeting_statistics FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE meeting_statistics FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- motion_votes
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS motion_votes_tenant_isolation ON motion_votes;
CREATE POLICY motion_votes_tenant_isolation ON motion_votes FOR ALL USING (((EXISTS ( SELECT 1 FROM (meeting_motions mm JOIN board_meetings bm ON ((bm.id = mm.meeting_id))) WHERE ((mm.id = motion_votes.motion_id) AND (bm.organization_id = get_current_org_id()))))) AND get_current_org_not_deleted());
ALTER TABLE motion_votes FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- organization_connectors
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS org_connectors_policy ON organization_connectors;
CREATE POLICY org_connectors_policy ON organization_connectors FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE organization_connectors FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- organization_integrations
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS org_integrations_policy ON organization_integrations;
CREATE POLICY org_integrations_policy ON organization_integrations FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE organization_integrations FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- organization_subscriptions
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS org_subscriptions_tenant_isolation ON organization_subscriptions;
CREATE POLICY org_subscriptions_tenant_isolation ON organization_subscriptions FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE organization_subscriptions FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- payment_methods
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS payment_methods_tenant_isolation ON payment_methods;
CREATE POLICY payment_methods_tenant_isolation ON payment_methods FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE payment_methods FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- reserve_funds
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS reserve_funds_tenant_isolation ON reserve_funds;
CREATE POLICY reserve_funds_tenant_isolation ON reserve_funds FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE reserve_funds FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- schedule_executions
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS schedule_executions_tenant_isolation ON schedule_executions;
CREATE POLICY schedule_executions_tenant_isolation ON schedule_executions FOR ALL USING (((schedule_id IN ( SELECT maintenance_schedules.id FROM maintenance_schedules WHERE (maintenance_schedules.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE schedule_executions FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- screening_ai_results
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS ai_results_select ON screening_ai_results;
CREATE POLICY ai_results_select ON screening_ai_results FOR SELECT USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS ai_results_insert ON screening_ai_results;
CREATE POLICY ai_results_insert ON screening_ai_results FOR INSERT WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE screening_ai_results FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- screening_background_results
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS background_results_select ON screening_background_results;
CREATE POLICY background_results_select ON screening_background_results FOR SELECT USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS background_results_insert ON screening_background_results;
CREATE POLICY background_results_insert ON screening_background_results FOR INSERT WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE screening_background_results FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- screening_credit_results
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS credit_results_select ON screening_credit_results;
CREATE POLICY credit_results_select ON screening_credit_results FOR SELECT USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS credit_results_insert ON screening_credit_results;
CREATE POLICY credit_results_insert ON screening_credit_results FOR INSERT WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE screening_credit_results FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- screening_eviction_results
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS eviction_results_select ON screening_eviction_results;
CREATE POLICY eviction_results_select ON screening_eviction_results FOR SELECT USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS eviction_results_insert ON screening_eviction_results;
CREATE POLICY eviction_results_insert ON screening_eviction_results FOR INSERT WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE screening_eviction_results FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- screening_provider_configs
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS provider_configs_select ON screening_provider_configs;
CREATE POLICY provider_configs_select ON screening_provider_configs FOR SELECT USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS provider_configs_insert ON screening_provider_configs;
CREATE POLICY provider_configs_insert ON screening_provider_configs FOR INSERT WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS provider_configs_update ON screening_provider_configs;
CREATE POLICY provider_configs_update ON screening_provider_configs FOR UPDATE USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS provider_configs_delete ON screening_provider_configs;
CREATE POLICY provider_configs_delete ON screening_provider_configs FOR DELETE USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE screening_provider_configs FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- screening_reports
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS reports_select ON screening_reports;
CREATE POLICY reports_select ON screening_reports FOR SELECT USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS reports_insert ON screening_reports;
CREATE POLICY reports_insert ON screening_reports FOR INSERT WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS reports_update ON screening_reports;
CREATE POLICY reports_update ON screening_reports FOR UPDATE USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE screening_reports FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- screening_request_queue
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS request_queue_select ON screening_request_queue;
CREATE POLICY request_queue_select ON screening_request_queue FOR SELECT USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS request_queue_insert ON screening_request_queue;
CREATE POLICY request_queue_insert ON screening_request_queue FOR INSERT WITH CHECK (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS request_queue_update ON screening_request_queue;
CREATE POLICY request_queue_update ON screening_request_queue FOR UPDATE USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE screening_request_queue FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- screening_risk_factors
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS risk_factors_select ON screening_risk_factors;
CREATE POLICY risk_factors_select ON screening_risk_factors FOR SELECT USING (((EXISTS ( SELECT 1 FROM screening_ai_results WHERE ((screening_ai_results.id = screening_risk_factors.ai_result_id) AND (screening_ai_results.organization_id = get_current_org_id()))))) AND get_current_org_not_deleted());
DROP POLICY IF EXISTS risk_factors_insert ON screening_risk_factors;
CREATE POLICY risk_factors_insert ON screening_risk_factors FOR INSERT WITH CHECK (((EXISTS ( SELECT 1 FROM screening_ai_results WHERE ((screening_ai_results.id = screening_risk_factors.ai_result_id) AND (screening_ai_results.organization_id = get_current_org_id()))))) AND get_current_org_not_deleted());
ALTER TABLE screening_risk_factors FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- search_embeddings
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS search_embeddings_tenant_isolation ON search_embeddings;
CREATE POLICY search_embeddings_tenant_isolation ON search_embeddings FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE search_embeddings FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- search_history
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS search_history_tenant_isolation ON search_history;
CREATE POLICY search_history_tenant_isolation ON search_history FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE search_history FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- sensor_alerts
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS sensor_alerts_tenant_isolation ON sensor_alerts;
CREATE POLICY sensor_alerts_tenant_isolation ON sensor_alerts FOR ALL USING (((sensor_id IN ( SELECT sensors.id FROM sensors WHERE (sensors.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE sensor_alerts FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- sensor_fault_correlations
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS sensor_correlations_tenant_isolation ON sensor_fault_correlations;
CREATE POLICY sensor_correlations_tenant_isolation ON sensor_fault_correlations FOR ALL USING (((sensor_id IN ( SELECT sensors.id FROM sensors WHERE (sensors.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE sensor_fault_correlations FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- sensor_readings
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS sensor_readings_tenant_isolation ON sensor_readings;
CREATE POLICY sensor_readings_tenant_isolation ON sensor_readings FOR ALL USING (((sensor_id IN ( SELECT sensors.id FROM sensors WHERE (sensors.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE sensor_readings FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- sensor_threshold_templates
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS sensor_templates_tenant_isolation ON sensor_threshold_templates;
CREATE POLICY sensor_templates_tenant_isolation ON sensor_threshold_templates FOR ALL USING ((((organization_id IS NULL) OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());
ALTER TABLE sensor_threshold_templates FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- sensor_thresholds
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS sensor_thresholds_tenant_isolation ON sensor_thresholds;
CREATE POLICY sensor_thresholds_tenant_isolation ON sensor_thresholds FOR ALL USING (((sensor_id IN ( SELECT sensors.id FROM sensors WHERE (sensors.organization_id = get_current_org_id())))) AND get_current_org_not_deleted());
ALTER TABLE sensor_thresholds FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- sensors
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS sensors_tenant_isolation ON sensors;
CREATE POLICY sensors_tenant_isolation ON sensors FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE sensors FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- sentiment_alerts
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS sentiment_alerts_tenant_isolation ON sentiment_alerts;
CREATE POLICY sentiment_alerts_tenant_isolation ON sentiment_alerts FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE sentiment_alerts FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- sentiment_thresholds
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS sentiment_thresholds_tenant_isolation ON sentiment_thresholds;
CREATE POLICY sentiment_thresholds_tenant_isolation ON sentiment_thresholds FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE sentiment_thresholds FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- sentiment_trends
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS sentiment_trends_tenant_isolation ON sentiment_trends;
CREATE POLICY sentiment_trends_tenant_isolation ON sentiment_trends FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE sentiment_trends FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- subscription_events
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS sub_events_tenant_isolation ON subscription_events;
CREATE POLICY sub_events_tenant_isolation ON subscription_events FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE subscription_events FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- subscription_invoices
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS sub_invoices_tenant_isolation ON subscription_invoices;
CREATE POLICY sub_invoices_tenant_isolation ON subscription_invoices FOR ALL USING (((organization_id = get_current_org_id())) AND get_current_org_not_deleted());
ALTER TABLE subscription_invoices FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- usage_records
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS usage_records_tenant_isolation ON usage_records;
CREATE POLICY usage_records_tenant_isolation ON usage_records USING (organization_id = get_current_org_id());
ALTER TABLE usage_records FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- vendor_contacts
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS vendor_contacts_org_isolation ON vendor_contacts;
CREATE POLICY vendor_contacts_org_isolation ON vendor_contacts FOR ALL USING ( vendor_id IN ( SELECT id FROM vendors WHERE organization_id = get_current_org_id() ) );
ALTER TABLE vendor_contacts FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- vendor_contract_documents
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS vendor_contract_documents_org_isolation ON vendor_contract_documents;
CREATE POLICY vendor_contract_documents_org_isolation ON vendor_contract_documents FOR ALL USING ( contract_id IN ( SELECT id FROM vendor_contracts WHERE organization_id = get_current_org_id() ) );
ALTER TABLE vendor_contract_documents FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- vendor_contracts
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS vendor_contracts_org_isolation ON vendor_contracts;
CREATE POLICY vendor_contracts_org_isolation ON vendor_contracts FOR ALL USING (organization_id = get_current_org_id());
ALTER TABLE vendor_contracts FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- vendor_invoices
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS vendor_invoices_org_isolation ON vendor_invoices;
CREATE POLICY vendor_invoices_org_isolation ON vendor_invoices FOR ALL USING (organization_id = get_current_org_id());
ALTER TABLE vendor_invoices FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- vendor_ratings
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS vendor_ratings_org_isolation ON vendor_ratings;
CREATE POLICY vendor_ratings_org_isolation ON vendor_ratings FOR ALL USING ( vendor_id IN ( SELECT id FROM vendors WHERE organization_id = get_current_org_id() ) );
ALTER TABLE vendor_ratings FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- vendor_service_areas
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS vendor_service_areas_org_isolation ON vendor_service_areas;
CREATE POLICY vendor_service_areas_org_isolation ON vendor_service_areas FOR ALL USING ( vendor_id IN ( SELECT id FROM vendors WHERE organization_id = get_current_org_id() ) );
ALTER TABLE vendor_service_areas FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- vendors
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS vendors_org_isolation ON vendors;
CREATE POLICY vendors_org_isolation ON vendors FOR ALL USING (organization_id = get_current_org_id());
ALTER TABLE vendors FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- webhook_deliveries
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS webhook_deliveries_policy ON webhook_deliveries;
CREATE POLICY webhook_deliveries_policy ON webhook_deliveries FOR ALL USING ( subscription_id IN ( SELECT id FROM webhook_subscriptions WHERE organization_id = get_current_org_id() ) );
ALTER TABLE webhook_deliveries FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- webhook_subscriptions
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS webhook_subscriptions_policy ON webhook_subscriptions;
CREATE POLICY webhook_subscriptions_policy ON webhook_subscriptions FOR ALL USING ( organization_id = get_current_org_id() );
ALTER TABLE webhook_subscriptions FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- work_order_updates
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS work_order_updates_tenant_isolation ON work_order_updates;
CREATE POLICY work_order_updates_tenant_isolation ON work_order_updates FOR ALL USING (work_order_id IN ( SELECT id FROM work_orders WHERE organization_id = get_current_org_id() ));
ALTER TABLE work_order_updates FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- work_orders
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS work_orders_tenant_isolation ON work_orders;
CREATE POLICY work_orders_tenant_isolation ON work_orders FOR ALL USING (organization_id = get_current_org_id());
ALTER TABLE work_orders FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- workflow_actions
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS workflow_actions_tenant_isolation ON workflow_actions;
CREATE POLICY workflow_actions_tenant_isolation ON workflow_actions FOR ALL USING (workflow_id IN ( SELECT id FROM workflows WHERE organization_id = get_current_org_id() ));
ALTER TABLE workflow_actions FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- workflow_execution_steps
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS workflow_execution_steps_tenant_isolation ON workflow_execution_steps;
CREATE POLICY workflow_execution_steps_tenant_isolation ON workflow_execution_steps FOR ALL USING (execution_id IN ( SELECT e.id FROM workflow_executions e JOIN workflows w ON w.id = e.workflow_id WHERE w.organization_id = get_current_org_id() ));
ALTER TABLE workflow_execution_steps FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- workflow_executions
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS workflow_executions_tenant_isolation ON workflow_executions;
CREATE POLICY workflow_executions_tenant_isolation ON workflow_executions FOR ALL USING (workflow_id IN ( SELECT id FROM workflows WHERE organization_id = get_current_org_id() ));
ALTER TABLE workflow_executions FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- workflow_schedules
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS workflow_schedules_tenant_isolation ON workflow_schedules;
CREATE POLICY workflow_schedules_tenant_isolation ON workflow_schedules FOR ALL USING (workflow_id IN ( SELECT id FROM workflows WHERE organization_id = get_current_org_id() ));
ALTER TABLE workflow_schedules FORCE ROW LEVEL SECURITY;

-- ------------------------------------------------------------------------
-- workflows
-- ------------------------------------------------------------------------
DROP POLICY IF EXISTS workflows_tenant_isolation ON workflows;
CREATE POLICY workflows_tenant_isolation ON workflows FOR ALL USING (organization_id = get_current_org_id());
ALTER TABLE workflows FORCE ROW LEVEL SECURITY;
