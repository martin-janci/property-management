// Consolidated integration-test harness 1/4 for db
// (build-experience roadmap item 8 follow-up; recipe from api-server,
// PR #2461). Former top-level tests/*.rs live in tests/suites/ as
// #[path]-included modules — one link instead of one binary per file.

mod common;

#[path = "suites/acc_mvp_rls_tests.rs"]
mod acc_mvp_rls_tests;
#[path = "suites/accounting_provider_rls_repo_tests.rs"]
mod accounting_provider_rls_repo_tests;
#[path = "suites/accounting_rls_repo_tests.rs"]
mod accounting_rls_repo_tests;
#[path = "suites/agency_domain_cache_tests.rs"]
mod agency_domain_cache_tests;
#[path = "suites/ai_chat_rls_repo_tests.rs"]
mod ai_chat_rls_repo_tests;
#[path = "suites/announcement_comments_rls_tests.rs"]
mod announcement_comments_rls_tests;
#[path = "suites/announcement_fanout_metrics_tests.rs"]
mod announcement_fanout_metrics_tests;
#[path = "suites/announcement_targeting_visibility_tests.rs"]
mod announcement_targeting_visibility_tests;
#[path = "suites/announcement_tests.rs"]
mod announcement_tests;
#[path = "suites/api_ecosystem_developer_force_rls_tests.rs"]
mod api_ecosystem_developer_force_rls_tests;
#[path = "suites/api_ecosystem_public_conn_routing_tests.rs"]
mod api_ecosystem_public_conn_routing_tests;
#[path = "suites/api_ecosystem_rls_repo_tests.rs"]
mod api_ecosystem_rls_repo_tests;
#[path = "suites/auth_policy_tests.rs"]
mod auth_policy_tests;
#[path = "suites/background_jobs_stuck_retry_budget_tests.rs"]
mod background_jobs_stuck_retry_budget_tests;
#[path = "suites/board_meetings_list_join_tests.rs"]
mod board_meetings_list_join_tests;
#[path = "suites/budget_rls_repo_tests.rs"]
mod budget_rls_repo_tests;
#[path = "suites/building_certification_rls_repo_tests.rs"]
mod building_certification_rls_repo_tests;
#[path = "suites/capability_revoke_mfa_constraint_tests.rs"]
mod capability_revoke_mfa_constraint_tests;
#[path = "suites/delegation_scopes_enum_decode_tests.rs"]
mod delegation_scopes_enum_decode_tests;
#[path = "suites/dispute_cross_tenant_tests.rs"]
mod dispute_cross_tenant_tests;
#[path = "suites/dispute_lifecycle_tests.rs"]
mod dispute_lifecycle_tests;
#[path = "suites/document_shares_access_log_scoping_tests.rs"]
mod document_shares_access_log_scoping_tests;
#[path = "suites/document_shares_enum_decode_tests.rs"]
mod document_shares_enum_decode_tests;
#[path = "suites/document_shares_revoke_scoping_tests.rs"]
mod document_shares_revoke_scoping_tests;
#[path = "suites/document_shares_rls_cross_tenant_tests.rs"]
mod document_shares_rls_cross_tenant_tests;
#[path = "suites/document_template_cross_tenant_tests.rs"]
mod document_template_cross_tenant_tests;
