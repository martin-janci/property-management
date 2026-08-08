// Consolidated integration-test harness 7/8 (build-experience roadmap
// item 8, .research/build-experience-report-2026-07-22.md): each former
// top-level tests/*.rs was one statically-linked binary; 206 binaries made
// the workspace link-dominated. Former files live in tests/suites/ as
// modules. ws_integration_tests.rs stays standalone (CI targets its
// binary_id for the Redis-gated ignored test).

mod common;

#[path = "suites/payment_webhook_settlement_tests.rs"]
mod payment_webhook_settlement_tests;
#[path = "suites/person_months_happy_path_tests.rs"]
mod person_months_happy_path_tests;
#[path = "suites/platform_admin_authz_batch2_tests.rs"]
mod platform_admin_authz_batch2_tests;
#[path = "suites/platform_admin_authz_batch3_tests.rs"]
mod platform_admin_authz_batch3_tests;
#[path = "suites/platform_admin_authz_tests.rs"]
mod platform_admin_authz_tests;
#[path = "suites/platform_settings_patch_tests.rs"]
mod platform_settings_patch_tests;
#[path = "suites/portal_webhook_signature_tests.rs"]
mod portal_webhook_signature_tests;
#[path = "suites/predictive_maintenance_happy_path_tests.rs"]
mod predictive_maintenance_happy_path_tests;
#[path = "suites/predictive_maintenance_photos_idor_tests.rs"]
mod predictive_maintenance_photos_idor_tests;
#[path = "suites/principal_platform_host_tests.rs"]
mod principal_platform_host_tests;
#[path = "suites/property_valuation_happy_path_tests.rs"]
mod property_valuation_happy_path_tests;
#[path = "suites/push_delivery_receipt_tests.rs"]
mod push_delivery_receipt_tests;
#[path = "suites/push_fanout_stale_token_isolation_tests.rs"]
mod push_fanout_stale_token_isolation_tests;
#[path = "suites/push_token_tests.rs"]
mod push_token_tests;
#[path = "suites/rag_index_embedding_write_tests.rs"]
mod rag_index_embedding_write_tests;
#[path = "suites/rag_migrate_pgvector_tests.rs"]
mod rag_migrate_pgvector_tests;
#[path = "suites/regional_compliance_tests.rs"]
mod regional_compliance_tests;
#[path = "suites/registry_backfill_tests.rs"]
mod registry_backfill_tests;
#[path = "suites/rental_connection_token_leak_tests.rs"]
mod rental_connection_token_leak_tests;
#[path = "suites/rental_guest_id_document_tests.rs"]
mod rental_guest_id_document_tests;
#[path = "suites/rentals_core_backfill_batch2_tests.rs"]
mod rentals_core_backfill_batch2_tests;
#[path = "suites/report_execution_download_retry_e2e_tests.rs"]
mod report_execution_download_retry_e2e_tests;
#[path = "suites/report_schedule_create_tests.rs"]
mod report_schedule_create_tests;
#[path = "suites/report_schedule_cron_roundtrip_tests.rs"]
mod report_schedule_cron_roundtrip_tests;
#[path = "suites/report_schedule_org_scope_jwt_tests.rs"]
mod report_schedule_org_scope_jwt_tests;
#[path = "suites/report_schedule_rbac_tests.rs"]
mod report_schedule_rbac_tests;
#[path = "suites/report_schedule_sibling_scope_tests.rs"]
mod report_schedule_sibling_scope_tests;
