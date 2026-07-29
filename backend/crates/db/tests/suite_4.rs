// Consolidated integration-test harness 4/4 for db
// (build-experience roadmap item 8 follow-up; recipe from api-server,
// PR #2461). Former top-level tests/*.rs live in tests/suites/ as
// #[path]-included modules — one link instead of one binary per file.
// (rls_smoke_tests.rs and rls_penetration_tests.rs stay STANDALONE:
// the rls-smoke-test and security-tests CI jobs invoke them by name via
// `cargo test --test <name>` — same exception as ws_integration_tests.)

mod common;

#[path = "suites/layout_change_events_retention_tests.rs"]
mod layout_change_events_retention_tests;
#[path = "suites/report_schedule_scheduler_rls_tests.rs"]
mod report_schedule_scheduler_rls_tests;
#[path = "suites/repository_tests.rs"]
mod repository_tests;
#[path = "suites/reserve_atomicity_tests.rs"]
mod reserve_atomicity_tests;
#[path = "suites/reserve_funds_rls_repo_tests.rs"]
mod reserve_funds_rls_repo_tests;
#[path = "suites/rls_context_bleed_tests.rs"]
mod rls_context_bleed_tests;
#[path = "suites/rls_listings_global_context_tests.rs"]
mod rls_listings_global_context_tests;
#[path = "suites/search_alert_delivery_tests.rs"]
mod search_alert_delivery_tests;
#[path = "suites/sensor_rls_repo_tests.rs"]
mod sensor_rls_repo_tests;
#[path = "suites/sentiment_rls_repo_tests.rs"]
mod sentiment_rls_repo_tests;
#[path = "suites/subscription_rls_repo_tests.rs"]
mod subscription_rls_repo_tests;
#[path = "suites/support_data_session_columns_tests.rs"]
mod support_data_session_columns_tests;
#[path = "suites/support_tooling_events_retention_tests.rs"]
mod support_tooling_events_retention_tests;
#[path = "suites/thread_participant_state_tests.rs"]
mod thread_participant_state_tests;
#[path = "suites/unified_portal_user_tests.rs"]
mod unified_portal_user_tests;
#[path = "suites/user_invite_tests.rs"]
mod user_invite_tests;
#[path = "suites/vendor_rls_repo_tests.rs"]
mod vendor_rls_repo_tests;
#[path = "suites/violations_cross_org_idor_tests.rs"]
mod violations_cross_org_idor_tests;
#[path = "suites/vote_cast_regression_tests.rs"]
mod vote_cast_regression_tests;
#[path = "suites/vote_delegation_eligibility_tests.rs"]
mod vote_delegation_eligibility_tests;
#[path = "suites/webhook_repo_dedup_tests.rs"]
mod webhook_repo_dedup_tests;
#[path = "suites/work_order_rls_repo_tests.rs"]
mod work_order_rls_repo_tests;
#[path = "suites/workflow_rls_repo_tests.rs"]
mod workflow_rls_repo_tests;
#[path = "suites/workflow_templates_rls_repo_tests.rs"]
mod workflow_templates_rls_repo_tests;
