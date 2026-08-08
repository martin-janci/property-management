// Consolidated integration-test harness 8/8 (build-experience roadmap
// item 8, .research/build-experience-report-2026-07-22.md): each former
// top-level tests/*.rs was one statically-linked binary; 206 binaries made
// the workspace link-dominated. Former files live in tests/suites/ as
// modules. ws_integration_tests.rs stays standalone (CI targets its
// binary_id for the Redis-gated ignored test).

mod common;

#[path = "suites/mobile_config_route_tests.rs"]
mod mobile_config_route_tests;
#[path = "suites/reports_export_org_scope_tests.rs"]
mod reports_export_org_scope_tests;
#[path = "suites/reports_get_execution_happy_path_tests.rs"]
mod reports_get_execution_happy_path_tests;
#[path = "suites/reports_happy_path_tests.rs"]
mod reports_happy_path_tests;
#[path = "suites/reserve_funds_cross_org_idor_tests.rs"]
mod reserve_funds_cross_org_idor_tests;
#[path = "suites/reserve_funds_happy_path_tests.rs"]
mod reserve_funds_happy_path_tests;
#[path = "suites/router_single_source_tests.rs"]
mod router_single_source_tests;
#[path = "suites/security_headers_tests.rs"]
mod security_headers_tests;
#[path = "suites/service_history_cross_org_idor_tests.rs"]
mod service_history_cross_org_idor_tests;
#[path = "suites/signatures_legal_docs_tests.rs"]
mod signatures_legal_docs_tests;
#[path = "suites/subscriptions_admin_happy_path_tests.rs"]
mod subscriptions_admin_happy_path_tests;
#[path = "suites/subscriptions_happy_path_tests.rs"]
mod subscriptions_happy_path_tests;
#[path = "suites/tenant_config_tests.rs"]
mod tenant_config_tests;
#[path = "suites/token_scope_tests.rs"]
mod token_scope_tests;
#[path = "suites/vendor_cross_org_idor_tests.rs"]
mod vendor_cross_org_idor_tests;
#[path = "suites/vendor_portal_stub_removal_tests.rs"]
mod vendor_portal_stub_removal_tests;
#[path = "suites/violations_cross_org_idor_tests.rs"]
mod violations_cross_org_idor_tests;
#[path = "suites/violations_happy_path_tests.rs"]
mod violations_happy_path_tests;
#[path = "suites/voice_oauth_exchange_auth_tests.rs"]
mod voice_oauth_exchange_auth_tests;
#[path = "suites/voting_behaviour_tests.rs"]
mod voting_behaviour_tests;
#[path = "suites/voting_happy_path_tests.rs"]
mod voting_happy_path_tests;
#[path = "suites/voting_report_tests.rs"]
mod voting_report_tests;
#[path = "suites/voting_tests.rs"]
mod voting_tests;
#[path = "suites/work_order_cross_org_idor_tests.rs"]
mod work_order_cross_org_idor_tests;
#[path = "suites/work_orders_happy_path_tests.rs"]
mod work_orders_happy_path_tests;
#[path = "suites/workflow_cross_tenant_idor_tests.rs"]
mod workflow_cross_tenant_idor_tests;
