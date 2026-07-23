// Consolidated integration-test harness 1/2 for reality-server
// (build-experience roadmap item 8 follow-up; recipe from api-server,
// PR #2461). Former top-level tests/*.rs live in tests/suites/ as
// #[path]-included modules — one link instead of one binary per file.

mod common;

#[path = "suites/agencies_authz_tests.rs"]
mod agencies_authz_tests;
#[path = "suites/agencies_happy_path_tests.rs"]
mod agencies_happy_path_tests;
#[path = "suites/agent_reviews_tests.rs"]
mod agent_reviews_tests;
#[path = "suites/articles_tests.rs"]
mod articles_tests;
#[path = "suites/buyer_inquiries_tests.rs"]
mod buyer_inquiries_tests;
#[path = "suites/compare_tests.rs"]
mod compare_tests;
#[path = "suites/favorite_alert_worker_tests.rs"]
mod favorite_alert_worker_tests;
#[path = "suites/favorites_authz_tests.rs"]
mod favorites_authz_tests;
#[path = "suites/favorites_happy_path_tests.rs"]
mod favorites_happy_path_tests;
#[path = "suites/health_tests.rs"]
mod health_tests;
#[path = "suites/imports_happy_path_tests.rs"]
mod imports_happy_path_tests;
#[path = "suites/imports_idor_tests.rs"]
mod imports_idor_tests;
#[path = "suites/inquiries_authz_tests.rs"]
mod inquiries_authz_tests;
#[path = "suites/inquiries_happy_path_tests.rs"]
mod inquiries_happy_path_tests;
#[path = "suites/inquiry_idor_tests.rs"]
mod inquiry_idor_tests;
#[path = "suites/inquiry_pagination_tests.rs"]
mod inquiry_pagination_tests;
