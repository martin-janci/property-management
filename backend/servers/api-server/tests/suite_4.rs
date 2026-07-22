// Consolidated integration-test harness 4/8 (build-experience roadmap
// item 8, .research/build-experience-report-2026-07-22.md): each former
// top-level tests/*.rs was one statically-linked binary; 206 binaries made
// the workspace link-dominated. Former files live in tests/suites/ as
// modules. ws_integration_tests.rs stays standalone (CI targets its
// binary_id for the Redis-gated ignored test).

mod common;

#[path = "suites/esignature_email_status_tracking_tests.rs"]
mod esignature_email_status_tracking_tests;
#[path = "suites/esignature_nonce_replay_tests.rs"]
mod esignature_nonce_replay_tests;
#[path = "suites/esignature_sign_consumer_tests.rs"]
mod esignature_sign_consumer_tests;
#[path = "suites/esignature_webhook_idempotency_tests.rs"]
mod esignature_webhook_idempotency_tests;
#[path = "suites/fault_notification_recipient_tests.rs"]
mod fault_notification_recipient_tests;
#[path = "suites/faults_authz_gap_tests.rs"]
mod faults_authz_gap_tests;
#[path = "suites/faults_behaviour_tests.rs"]
mod faults_behaviour_tests;
#[path = "suites/faults_cross_org_idor_tests.rs"]
mod faults_cross_org_idor_tests;
#[path = "suites/faults_tests.rs"]
mod faults_tests;
#[path = "suites/faults_workflow_happy_path_tests.rs"]
mod faults_workflow_happy_path_tests;
#[path = "suites/feature_flag_kill_switch_tests.rs"]
mod feature_flag_kill_switch_tests;
#[path = "suites/financial_cross_org_idor_tests.rs"]
mod financial_cross_org_idor_tests;
#[path = "suites/financial_happy_path_tests.rs"]
mod financial_happy_path_tests;
#[path = "suites/financial_reports_happy_path_tests.rs"]
mod financial_reports_happy_path_tests;
#[path = "suites/form_cross_org_idor_tests.rs"]
mod form_cross_org_idor_tests;
#[path = "suites/governance_delegations_neighbors_happy_path_tests.rs"]
mod governance_delegations_neighbors_happy_path_tests;
#[path = "suites/granular_notif_role_defaults_rbac_tests.rs"]
mod granular_notif_role_defaults_rbac_tests;
#[path = "suites/health_tests.rs"]
mod health_tests;
#[path = "suites/help_tests.rs"]
mod help_tests;
#[path = "suites/infra_migration_platform_admin_tests.rs"]
mod infra_migration_platform_admin_tests;
#[path = "suites/infra_ops_admin_authz_backfill_bit331_tests.rs"]
mod infra_ops_admin_authz_backfill_bit331_tests;
#[path = "suites/infra_ops_authz_backfill_tests.rs"]
mod infra_ops_authz_backfill_tests;
#[path = "suites/insurance_cross_tenant_idor_tests.rs"]
mod insurance_cross_tenant_idor_tests;
#[path = "suites/integration_sync_tests.rs"]
mod integration_sync_tests;
#[path = "suites/integrations_batch4_tests.rs"]
mod integrations_batch4_tests;
