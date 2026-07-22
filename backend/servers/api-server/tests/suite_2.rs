// Consolidated integration-test harness 2/8 (build-experience roadmap
// item 8, .research/build-experience-report-2026-07-22.md): each former
// top-level tests/*.rs was one statically-linked binary; 206 binaries made
// the workspace link-dominated. Former files live in tests/suites/ as
// modules. ws_integration_tests.rs stays standalone (CI targets its
// binary_id for the Redis-gated ignored test).

mod common;

#[path = "suites/aml_dsa_audit_logging_tests.rs"]
mod aml_dsa_audit_logging_tests;
#[path = "suites/aml_dsa_authz_pap60_tests.rs"]
mod aml_dsa_authz_pap60_tests;
#[path = "suites/aml_dsa_cross_org_idor_tests.rs"]
mod aml_dsa_cross_org_idor_tests;
#[path = "suites/analytics_esg_reporting_success_tests.rs"]
mod analytics_esg_reporting_success_tests;
#[path = "suites/analytics_government_portal_success_tests.rs"]
mod analytics_government_portal_success_tests;
#[path = "suites/analytics_investor_portal_success_tests.rs"]
mod analytics_investor_portal_success_tests;
#[path = "suites/analytics_owner_success_tests.rs"]
mod analytics_owner_success_tests;
#[path = "suites/analytics_portfolio_success_tests.rs"]
mod analytics_portfolio_success_tests;
#[path = "suites/announcement_comments_threading_tests.rs"]
mod announcement_comments_threading_tests;
#[path = "suites/announcement_residency_revocation_tests.rs"]
mod announcement_residency_revocation_tests;
#[path = "suites/announcements_happy_path_tests.rs"]
mod announcements_happy_path_tests;
#[path = "suites/auth_enumeration_tests.rs"]
mod auth_enumeration_tests;
#[path = "suites/auth_partial_backfill_batch1_tests.rs"]
mod auth_partial_backfill_batch1_tests;
#[path = "suites/auth_partial_backfill_batch2_tests.rs"]
mod auth_partial_backfill_batch2_tests;
#[path = "suites/auth_policy_enforcement_tests.rs"]
mod auth_policy_enforcement_tests;
#[path = "suites/auth_tests.rs"]
mod auth_tests;
#[path = "suites/automation_auth_tests.rs"]
mod automation_auth_tests;
#[path = "suites/automation_workflow_batch3_tests.rs"]
mod automation_workflow_batch3_tests;
#[path = "suites/board_meetings_auth_tests.rs"]
mod board_meetings_auth_tests;
#[path = "suites/board_meetings_cross_org_idor_tests.rs"]
mod board_meetings_cross_org_idor_tests;
#[path = "suites/board_meetings_happy_path_tests.rs"]
mod board_meetings_happy_path_tests;
#[path = "suites/booking_connect_encryption_tests.rs"]
mod booking_connect_encryption_tests;
#[path = "suites/booking_credential_encryption_tests.rs"]
mod booking_credential_encryption_tests;
#[path = "suites/booking_cross_user_idor_tests.rs"]
mod booking_cross_user_idor_tests;
#[path = "suites/booking_handler_routes_tests.rs"]
mod booking_handler_routes_tests;
#[path = "suites/booking_oauth_csrf_tests.rs"]
mod booking_oauth_csrf_tests;
