// Consolidated integration-test harness 3/4 for db
// (build-experience roadmap item 8 follow-up; recipe from api-server,
// PR #2461). Former top-level tests/*.rs live in tests/suites/ as
// #[path]-included modules — one link instead of one binary per file.

mod common;

#[path = "suites/notification_rls_guc_tests.rs"]
mod notification_rls_guc_tests;
#[path = "suites/nparty_unread_watermark_tests.rs"]
mod nparty_unread_watermark_tests;
#[path = "suites/oauth_refresh_token_tests.rs"]
mod oauth_refresh_token_tests;
#[path = "suites/payment_management_tests.rs"]
mod payment_management_tests;
#[path = "suites/payment_reminder_dedup_tests.rs"]
mod payment_reminder_dedup_tests;
#[path = "suites/portal_imports_cross_org_idor_tests.rs"]
mod portal_imports_cross_org_idor_tests;
#[path = "suites/portal_listing_analytics_force_rls_tests.rs"]
mod portal_listing_analytics_force_rls_tests;
#[path = "suites/portal_listing_view_count_force_rls_tests.rs"]
mod portal_listing_view_count_force_rls_tests;
#[path = "suites/portal_track_listing_view_force_rls_tests.rs"]
mod portal_track_listing_view_force_rls_tests;
#[path = "suites/portal_user_merge_tests.rs"]
mod portal_user_merge_tests;
#[path = "suites/portfolio_performance_test.rs"]
mod portfolio_performance_test;
#[path = "suites/predictive_maintenance_rls_repo_tests.rs"]
mod predictive_maintenance_rls_repo_tests;
#[path = "suites/predictive_maintenance_sort_injection_tests.rs"]
mod predictive_maintenance_sort_injection_tests;
#[path = "suites/principal_kind_actor_check_tests.rs"]
mod principal_kind_actor_check_tests;
#[path = "suites/principal_kind_guard_tests.rs"]
mod principal_kind_guard_tests;
#[path = "suites/public_plans_rls_tests.rs"]
mod public_plans_rls_tests;
#[path = "suites/rag_similarity_search_tests.rs"]
mod rag_similarity_search_tests;
#[path = "suites/record_payment_atomicity_tests.rs"]
mod record_payment_atomicity_tests;
#[path = "suites/regional_compliance_quorum_tests.rs"]
mod regional_compliance_quorum_tests;
#[path = "suites/registry_enum_decode_tests.rs"]
mod registry_enum_decode_tests;
#[path = "suites/registry_parking_spot_cross_tenant_tests.rs"]
mod registry_parking_spot_cross_tenant_tests;
#[path = "suites/registry_rules_enum_decode_tests.rs"]
mod registry_rules_enum_decode_tests;
#[path = "suites/rental_booking_nullable_decimal_tests.rs"]
mod rental_booking_nullable_decimal_tests;
#[path = "suites/rental_cross_tenant_upsert_tests.rs"]
mod rental_cross_tenant_upsert_tests;
#[path = "suites/rental_enum_decode_tests.rs"]
mod rental_enum_decode_tests;
#[path = "suites/rental_guest_ical_enum_decode_tests.rs"]
mod rental_guest_ical_enum_decode_tests;
