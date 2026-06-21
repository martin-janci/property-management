//! Integration tests for api-server routes.
//!
//! These tests validate end-to-end HTTP flows including:
//! - Authentication (register, login, logout, token refresh)
//! - Authorization (role-based access)
//! - Error handling
//! - Document access control
//! - Integration sync flows
//! - Health checks

pub mod auth_tests;
// document_folder_tests (and the former document_access_tests) moved to the
// root-level `tests/document_folder_tests.rs` binary (this `integration/` subtree
// is never declared as `mod integration;` by any test bin, so it does not compile
// — see that file's header note).
pub mod health_tests;
pub mod integration_sync_tests;
pub mod mfa_e2e_tests;
pub mod mfa_recovery_codes_tests;
