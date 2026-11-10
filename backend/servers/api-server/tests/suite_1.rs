// Consolidated integration-test harness 1/8 (build-experience roadmap
// item 8, .research/build-experience-report-2026-07-22.md): each former
// top-level tests/*.rs was one statically-linked binary; 206 binaries made
// the workspace link-dominated. Former files live in tests/suites/ as
// modules. ws_integration_tests.rs stays standalone (CI targets its
// binary_id for the Redis-gated ignored test).

mod common;

#[path = "suites/accounting_contacts_authz_tests.rs"]
mod accounting_contacts_authz_tests;
#[path = "suites/accounting_happy_path_tests.rs"]
mod accounting_happy_path_tests;
#[path = "suites/accounting_invoices_tests.rs"]
mod accounting_invoices_tests;
#[path = "suites/accounting_payment_match_state_machine_tests.rs"]
mod accounting_payment_match_state_machine_tests;
#[path = "suites/accounting_payment_matcher_suggest_tests.rs"]
mod accounting_payment_matcher_suggest_tests;
#[path = "suites/accounting_upload_statement_happy_path_tests.rs"]
mod accounting_upload_statement_happy_path_tests;
#[path = "suites/admin_mfa_disable_tests.rs"]
mod admin_mfa_disable_tests;
#[path = "suites/admin_mfa_recovery_tests.rs"]
mod admin_mfa_recovery_tests;
#[path = "suites/admin_mfa_step_up_tests.rs"]
mod admin_mfa_step_up_tests;
#[path = "suites/admin_platform_happy_path_batch2_tests.rs"]
mod admin_platform_happy_path_batch2_tests;
#[path = "suites/admin_platform_happy_path_batch3_tests.rs"]
mod admin_platform_happy_path_batch3_tests;
#[path = "suites/admin_platform_happy_path_tests.rs"]
mod admin_platform_happy_path_tests;
#[path = "suites/agency_authz_idor_tests.rs"]
mod agency_authz_idor_tests;
#[path = "suites/ai_auth_tests.rs"]
mod ai_auth_tests;
#[path = "suites/ai_automation_batch1_tests.rs"]
mod ai_automation_batch1_tests;
#[path = "suites/ai_automation_batch2_tests.rs"]
mod ai_automation_batch2_tests;
#[path = "suites/ai_automation_batch3_tests.rs"]
mod ai_automation_batch3_tests;
#[path = "suites/ai_chat_sentiment_equipment_batch4_tests.rs"]
mod ai_chat_sentiment_equipment_batch4_tests;
#[path = "suites/ai_llm_cross_tenant_idor_tests.rs"]
mod ai_llm_cross_tenant_idor_tests;
#[path = "suites/ai_llm_workflow_batch5_tests.rs"]
mod ai_llm_workflow_batch5_tests;
#[path = "suites/airbnb_connections_routes_tests.rs"]
mod airbnb_connections_routes_tests;
#[path = "suites/airbnb_direct_connect_and_availability_sync_tests.rs"]
mod airbnb_direct_connect_and_availability_sync_tests;
#[path = "suites/airbnb_install_routes_tests.rs"]
mod airbnb_install_routes_tests;
#[path = "suites/airbnb_oauth_routes_tests.rs"]
mod airbnb_oauth_routes_tests;
#[path = "suites/airbnb_sync_reconciliation_tests.rs"]
mod airbnb_sync_reconciliation_tests;
#[path = "suites/airbnb_webhook_dedup_tests.rs"]
mod airbnb_webhook_dedup_tests;
#[path = "suites/airbnb_webhook_routes_tests.rs"]
mod airbnb_webhook_routes_tests;
