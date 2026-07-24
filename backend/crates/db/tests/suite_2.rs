// Consolidated integration-test harness 2/4 for db
// (build-experience roadmap item 8 follow-up; recipe from api-server,
// PR #2461). Former top-level tests/*.rs live in tests/suites/ as
// #[path]-included modules — one link instead of one binary per file.

mod common;

#[path = "suites/documents_rls_cross_tenant_tests.rs"]
mod documents_rls_cross_tenant_tests;
#[path = "suites/emergency_rls_repo_tests.rs"]
mod emergency_rls_repo_tests;
#[path = "suites/enhanced_tenant_screening_rls_repo_tests.rs"]
mod enhanced_tenant_screening_rls_repo_tests;
#[path = "suites/epic11_org_guc_rls_tests.rs"]
mod epic11_org_guc_rls_tests;
#[path = "suites/equipment_rls_repo_tests.rs"]
mod equipment_rls_repo_tests;
#[path = "suites/esg_force_rls_cross_tenant_tests.rs"]
mod esg_force_rls_cross_tenant_tests;
#[path = "suites/esignature_workflow_repo_tests.rs"]
mod esignature_workflow_repo_tests;
#[path = "suites/facility_enum_decode_tests.rs"]
mod facility_enum_decode_tests;
#[path = "suites/fault_kpi_unification_tests.rs"]
mod fault_kpi_unification_tests;
#[path = "suites/favorite_alert_read_idempotent_tests.rs"]
mod favorite_alert_read_idempotent_tests;
#[path = "suites/form_rls_repo_tests.rs"]
mod form_rls_repo_tests;
#[path = "suites/government_portal_connection_cross_org_idor_tests.rs"]
mod government_portal_connection_cross_org_idor_tests;
#[path = "suites/inquiry_mark_read_idor_tests.rs"]
mod inquiry_mark_read_idor_tests;
#[path = "suites/insurance_rls_repo_tests.rs"]
mod insurance_rls_repo_tests;
#[path = "suites/integration_rls_repo_tests.rs"]
mod integration_rls_repo_tests;
#[path = "suites/layout_repo_tests.rs"]
mod layout_repo_tests;
#[path = "suites/lease_units_join_tests.rs"]
mod lease_units_join_tests;
#[path = "suites/legal_rls_repo_tests.rs"]
mod legal_rls_repo_tests;
#[path = "suites/llm_document_rls_repo_tests.rs"]
mod llm_document_rls_repo_tests;
#[path = "suites/membership_revocation_tests.rs"]
mod membership_revocation_tests;
#[path = "suites/messaging_rls_cross_tenant_tests.rs"]
mod messaging_rls_cross_tenant_tests;
#[path = "suites/migration_templates_pagination_tests.rs"]
mod migration_templates_pagination_tests;
#[path = "suites/my_units_privacy_tests.rs"]
mod my_units_privacy_tests;
#[path = "suites/news_article_cross_tenant_tests.rs"]
mod news_article_cross_tenant_tests;
#[path = "suites/notification_events_repo_tests.rs"]
mod notification_events_repo_tests;
