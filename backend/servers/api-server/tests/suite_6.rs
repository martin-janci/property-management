// Consolidated integration-test harness 6/8 (build-experience roadmap
// item 8, .research/build-experience-report-2026-07-22.md): each former
// top-level tests/*.rs was one statically-linked binary; 206 binaries made
// the workspace link-dominated. Former files live in tests/suites/ as
// modules. ws_integration_tests.rs stays standalone (CI targets its
// binary_id for the Redis-gated ignored test).

mod common;

#[path = "suites/mfa_e2e_tests.rs"]
mod mfa_e2e_tests;
#[path = "suites/mfa_recovery_codes_tests.rs"]
mod mfa_recovery_codes_tests;
#[path = "suites/mfa_recovery_cross_user_idor_tests.rs"]
mod mfa_recovery_cross_user_idor_tests;
#[path = "suites/mfa_regenerate_backup_codes_rls_scope_tests.rs"]
mod mfa_regenerate_backup_codes_rls_scope_tests;
#[path = "suites/migration_db_tests.rs"]
mod migration_db_tests;
#[path = "suites/migration_pagination_tests.rs"]
mod migration_pagination_tests;
#[path = "suites/multi_currency_cross_org_idor_tests.rs"]
mod multi_currency_cross_org_idor_tests;
#[path = "suites/multi_currency_happy_path_tests.rs"]
mod multi_currency_happy_path_tests;
#[path = "suites/my_units_resident_view_tests.rs"]
mod my_units_resident_view_tests;
#[path = "suites/news_articles_tests.rs"]
mod news_articles_tests;
#[path = "suites/notification_pipeline_dispatch_tests.rs"]
mod notification_pipeline_dispatch_tests;
#[path = "suites/notification_preference_realtime_sync_tests.rs"]
mod notification_preference_realtime_sync_tests;
#[path = "suites/notification_prefs_granular_tests.rs"]
mod notification_prefs_granular_tests;
#[path = "suites/oauth_authorization_server_test.rs"]
mod oauth_authorization_server_test;
#[path = "suites/oauth_authz_server_edge_tests.rs"]
mod oauth_authz_server_edge_tests;
#[path = "suites/oauth_client_registration_test.rs"]
mod oauth_client_registration_test;
#[path = "suites/oauth_integration_tests.rs"]
mod oauth_integration_tests;
#[path = "suites/oauth_token_introspection_rotation_test.rs"]
mod oauth_token_introspection_rotation_test;
#[path = "suites/ocr_meter_reading_tests.rs"]
mod ocr_meter_reading_tests;
#[path = "suites/org_property_authz_backfill_bit331_tests.rs"]
mod org_property_authz_backfill_bit331_tests;
#[path = "suites/org_property_authz_backfill_tests.rs"]
mod org_property_authz_backfill_tests;
#[path = "suites/org_property_authz_tests.rs"]
mod org_property_authz_tests;
#[path = "suites/org_property_happy_path_tests.rs"]
mod org_property_happy_path_tests;
#[path = "suites/outages_happy_path_tests.rs"]
mod outages_happy_path_tests;
#[path = "suites/owner_analytics_cross_org_idor_tests.rs"]
mod owner_analytics_cross_org_idor_tests;
#[path = "suites/package_visitor_happy_path_tests.rs"]
mod package_visitor_happy_path_tests;
