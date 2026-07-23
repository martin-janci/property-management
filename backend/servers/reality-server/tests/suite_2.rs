// Consolidated integration-test harness 2/2 for reality-server
// (build-experience roadmap item 8 follow-up; recipe from api-server,
// PR #2461). Former top-level tests/*.rs live in tests/suites/ as
// #[path]-included modules — one link instead of one binary per file.

mod common;

#[path = "suites/listings_happy_path_tests.rs"]
mod listings_happy_path_tests;
#[path = "suites/listings_pagination_clamp_tests.rs"]
mod listings_pagination_clamp_tests;
#[path = "suites/listings_tests.rs"]
mod listings_tests;
#[path = "suites/portal_listings_create_tests.rs"]
mod portal_listings_create_tests;
#[path = "suites/portal_listings_idor_tests.rs"]
mod portal_listings_idor_tests;
#[path = "suites/portal_listings_my_list_analytics_tests.rs"]
mod portal_listings_my_list_analytics_tests;
#[path = "suites/price_map_tests.rs"]
mod price_map_tests;
#[path = "suites/raw_pool_audit_tests.rs"]
mod raw_pool_audit_tests;
#[path = "suites/reports_tests.rs"]
mod reports_tests;
#[path = "suites/saved_searches_authz_tests.rs"]
mod saved_searches_authz_tests;
#[path = "suites/saved_searches_happy_path_tests.rs"]
mod saved_searches_happy_path_tests;
#[path = "suites/search_alert_drainer_tests.rs"]
mod search_alert_drainer_tests;
#[path = "suites/sso_authz_tests.rs"]
mod sso_authz_tests;
#[path = "suites/users_authz_tests.rs"]
mod users_authz_tests;
#[path = "suites/users_happy_path_tests.rs"]
mod users_happy_path_tests;
