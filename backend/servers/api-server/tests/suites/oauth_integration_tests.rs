//! OAuth 2.0 Authorization Server integration tests (Epic 10A, gap-10a-1).
//!
//! Covers:
//!  - Full PKCE S256 authorization code flow (public client)
//!  - Consent / revoke user grant
//!  - Token refresh rotation with family_id reuse detection
//!  - Authorization audit trail (OAuthAuthorize, OAuthRevoke, OAuthTokenDeniedPrincipalKind)
//!  - Security: revoked tokens rejected, PKCE plain method rejected, introspect auth enforced
//!  - Epic 10A provider security: PKCE S256 enforcement, revoked-token introspection,
//!    refresh-rotation replay, principal_kind access control, redirect URI binding,
//!    deactivated client rejection, Basic-auth introspection (pm-security-oauth-10a-security-tests)
//!
//! Every test uses `#[sqlx::test(migrator = "db::MIGRATOR")]` so the schema
//! is fully up-to-date, and uses `TestApp` so the real Axum router (with all
//! middleware) is exercised end-to-end.
//!
//! This module is split by OAuth flow into submodules; shared
//! helpers live in `fixtures`. See the individual files.

#![allow(dead_code)]

// NOTE: this file is itself pulled in via `#[path = "suites/oauth_integration_tests.rs"]`
// from `tests/suite_6.rs`, so its child modules resolve relative to `suites/`
// rather than an `oauth_integration_tests/` subdir — each needs an explicit
// `#[path]` pointing into the subdirectory.
#[path = "oauth_integration_tests/fixtures.rs"]
mod fixtures;

#[path = "oauth_integration_tests/admin_analytics.rs"]
mod admin_analytics;
#[path = "oauth_integration_tests/authorize.rs"]
mod authorize;
#[path = "oauth_integration_tests/grants.rs"]
mod grants;
#[path = "oauth_integration_tests/introspect_revoke.rs"]
mod introspect_revoke;

// Token-endpoint flow — split by sub-flow (refresh rotation / grant validation /
// Epic 10A provider security) so no single file grows back into a churn hotspot.
#[path = "oauth_integration_tests/token_grant_validation.rs"]
mod token_grant_validation;
#[path = "oauth_integration_tests/token_provider_security.rs"]
mod token_provider_security;
#[path = "oauth_integration_tests/token_refresh_rotation.rs"]
mod token_refresh_rotation;
