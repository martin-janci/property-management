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
pub mod document_folder_tests;
pub mod health_tests;
pub mod integration_sync_tests;
pub mod mfa_e2e_tests;
pub mod mfa_recovery_codes_tests;
