// Consolidated integration-test harness 5/8 (build-experience roadmap
// item 8, .research/build-experience-report-2026-07-22.md): each former
// top-level tests/*.rs was one statically-linked binary; 206 binaries made
// the workspace link-dominated. Former files live in tests/suites/ as
// modules. ws_integration_tests.rs stays standalone (CI targets its
// binary_id for the Redis-gated ignored test).

mod common;

#[path = "suites/documents_forms_batch2_happy_path_tests.rs"]
mod documents_forms_batch2_happy_path_tests;
#[path = "suites/integrations_cross_org_idor_tests.rs"]
mod integrations_cross_org_idor_tests;
#[path = "suites/integrations_marketplace_reviews_tests.rs"]
mod integrations_marketplace_reviews_tests;
#[path = "suites/iot_auth_tests.rs"]
mod iot_auth_tests;
#[path = "suites/iot_sensor_ws_authz_tests.rs"]
mod iot_sensor_ws_authz_tests;
#[path = "suites/layout_tenant_override_updated_by_tests.rs"]
mod layout_tenant_override_updated_by_tests;
#[path = "suites/lease_abstraction_forms_tests.rs"]
mod lease_abstraction_forms_tests;
#[path = "suites/lease_review_rbac_tests.rs"]
mod lease_review_rbac_tests;
#[path = "suites/lease_send_for_signature_rbac_tests.rs"]
mod lease_send_for_signature_rbac_tests;
#[path = "suites/leases_core_backfill_batch1_tests.rs"]
mod leases_core_backfill_batch1_tests;
#[path = "suites/legal_cross_org_idor_tests.rs"]
mod legal_cross_org_idor_tests;
#[path = "suites/legal_insurance_wave1b_tests.rs"]
mod legal_insurance_wave1b_tests;
#[path = "suites/legal_notices_audit_tests.rs"]
mod legal_notices_audit_tests;
#[path = "suites/listings_core_backfill_batch3_tests.rs"]
mod listings_core_backfill_batch3_tests;
#[path = "suites/listings_global_publish_authz_tests.rs"]
mod listings_global_publish_authz_tests;
#[path = "suites/market_pricing_happy_path_tests.rs"]
mod market_pricing_happy_path_tests;
#[path = "suites/marketplace_ecosystem_backfill_batch3_tests.rs"]
mod marketplace_ecosystem_backfill_batch3_tests;
#[path = "suites/marketplace_lifecycle_backfill_batch2_tests.rs"]
mod marketplace_lifecycle_backfill_batch2_tests;
#[path = "suites/marketplace_provider_profile_tests.rs"]
mod marketplace_provider_profile_tests;
#[path = "suites/marketplace_voting_investor_cross_org_idor_tests.rs"]
mod marketplace_voting_investor_cross_org_idor_tests;
#[path = "suites/messaging_attachments_authz_tests.rs"]
mod messaging_attachments_authz_tests;
#[path = "suites/messaging_behaviour_backfill_tests.rs"]
mod messaging_behaviour_backfill_tests;
#[path = "suites/messaging_group_conversations_tests.rs"]
mod messaging_group_conversations_tests;
#[path = "suites/messaging_thread_state_authz_tests.rs"]
mod messaging_thread_state_authz_tests;
#[path = "suites/meters_energy_cross_org_idor_tests.rs"]
mod meters_energy_cross_org_idor_tests;
#[path = "suites/mfa_brute_force_rate_limit_tests.rs"]
mod mfa_brute_force_rate_limit_tests;
#[path = "suites/mfa_disable_rls_scope_tests.rs"]
mod mfa_disable_rls_scope_tests;
