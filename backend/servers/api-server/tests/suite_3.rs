// Consolidated integration-test harness 3/8 (build-experience roadmap
// item 8, .research/build-experience-report-2026-07-22.md): each former
// top-level tests/*.rs was one statically-linked binary; 206 binaries made
// the workspace link-dominated. Former files live in tests/suites/ as
// modules. ws_integration_tests.rs stays standalone (CI targets its
// binary_id for the Redis-gated ignored test).

mod common;

#[path = "suites/booking_oauth_routes_tests.rs"]
mod booking_oauth_routes_tests;
#[path = "suites/budget_capital_forecast_tests.rs"]
mod budget_capital_forecast_tests;
#[path = "suites/budgets_tests.rs"]
mod budgets_tests;
#[path = "suites/building_manager_rbac_tests.rs"]
mod building_manager_rbac_tests;
#[path = "suites/caddy_ask_tests.rs"]
mod caddy_ask_tests;
#[path = "suites/community_cross_tenant_idor_tests.rs"]
mod community_cross_tenant_idor_tests;
#[path = "suites/community_happy_path_tests.rs"]
mod community_happy_path_tests;
#[path = "suites/community_unauthenticated_reads_tests.rs"]
mod community_unauthenticated_reads_tests;
#[path = "suites/compliance_wave1b_tests.rs"]
mod compliance_wave1b_tests;
#[path = "suites/data_residency_tests.rs"]
mod data_residency_tests;
#[path = "suites/dev_mode_tenant_tests.rs"]
mod dev_mode_tenant_tests;
#[path = "suites/dispute_cross_org_idor_tests.rs"]
mod dispute_cross_org_idor_tests;
#[path = "suites/dispute_evidence_auth_tests.rs"]
mod dispute_evidence_auth_tests;
#[path = "suites/disputes_happy_path_tests.rs"]
mod disputes_happy_path_tests;
#[path = "suites/document_access_rls_tests.rs"]
mod document_access_rls_tests;
#[path = "suites/document_download_preview_tests.rs"]
mod document_download_preview_tests;
#[path = "suites/document_folder_tests.rs"]
mod document_folder_tests;
#[path = "suites/document_share_access_tests.rs"]
mod document_share_access_tests;
#[path = "suites/document_upload_tests.rs"]
mod document_upload_tests;
#[path = "suites/documents_core_crud_tests.rs"]
mod documents_core_crud_tests;
#[path = "suites/documents_forms_happy_path_tests.rs"]
mod documents_forms_happy_path_tests;
#[path = "suites/documents_intelligence_templates_tests.rs"]
mod documents_intelligence_templates_tests;
#[path = "suites/dsa_report_download_tests.rs"]
mod dsa_report_download_tests;
#[path = "suites/emergency_cross_org_idor_tests.rs"]
mod emergency_cross_org_idor_tests;
#[path = "suites/endpoints_smoke_tests.rs"]
mod endpoints_smoke_tests;
#[path = "suites/enhanced_screening_cross_org_idor_tests.rs"]
mod enhanced_screening_cross_org_idor_tests;
#[path = "suites/equipment_cross_tenant_idor_tests.rs"]
mod equipment_cross_tenant_idor_tests;
#[path = "suites/esg_reporting_cross_org_idor_tests.rs"]
mod esg_reporting_cross_org_idor_tests;
