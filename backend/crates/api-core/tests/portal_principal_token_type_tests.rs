//! Regression: `PortalPrincipal` must reject non-access and expired tokens
//! through the shared `verify_access_token` gate (#1675).
//!
//! `PortalPrincipal` (the reality-portal extractor) shares the exact same
//! bearer-decode + `token_type` gate as `RequestPrincipal` via
//! `decode_bearer_subject()` → `api_core::extractors::auth::verify_access_token`.
//! Before #1675 that gate was only directly unit-tested through
//! `RequestPrincipal` (`principal_token_type_tests.rs`); `PortalPrincipal` was
//! exercised only indirectly via reality-server integration *happy-path* tests
//! that always mint `token_type: "access"`. These tests pin the rejection
//! contract on the second consumer of the shared helper so a future divergence
//! is caught:
//!
//!   * `token_type: "refresh"` (or any non-access value) → 401 BEFORE the DB
//!     lookup runs.
//!   * an expired token → 401 (the `Validation::default()` `exp` check) BEFORE
//!     the DB lookup runs.
//!   * `token_type: "access"` (or the claim omitted, legacy-tolerant) → passes
//!     the discriminator and proceeds to the DB lookup.
//!
//! As in `principal_token_type_tests.rs`, the stub pool points at an
//! unreachable address: every rejection branch returns before any query, so the
//! pool is never touched. The access-token case DOES reach the DB and fails
//! there (unreachable pool → non-401), which is how we prove it got *past* the
//! token_type gate without standing up Postgres.

use api_core::extractors::tenant::TenantMembershipProvider;
use api_core::extractors::PortalPrincipal;
use axum::extract::FromRequestParts;
use axum::http::{header::AUTHORIZATION, Request, StatusCode};
use db::DbPool;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const TEST_SECRET: &str = "test-secret-key-at-least-32-chars-long!!";

#[derive(Clone)]
struct StubState {
    pool: DbPool,
}

impl StubState {
    fn new() -> Self {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://never-used:never@127.0.0.1:1/never")
            .expect("lazy pool init");
        Self { pool }
    }
}

impl TenantMembershipProvider for StubState {
    fn db_pool(&self) -> &DbPool {
        &self.pool
    }
}

/// Mirrors `PrincipalClaims` field-for-field; the extractor decodes by claim
/// name, not by struct identity. `token_type` is optional here so we can mint
/// tokens that omit it (legacy shape) as well as explicit access/refresh.
#[derive(Debug, Serialize, Deserialize)]
struct TestClaims {
    sub: Uuid,
    iat: i64,
    exp: i64,
    kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token_type: Option<String>,
}

/// Mint a token with a far-future expiry (so only the `token_type` branch is
/// under test) and the given discriminator.
fn mint(token_type: Option<&str>) -> String {
    mint_with_exp(token_type, 4_102_444_800) // 2100-01-01
}

/// Mint a token with an explicit `exp` so the expiry path can be tested.
fn mint_with_exp(token_type: Option<&str>, exp: i64) -> String {
    let claims = TestClaims {
        sub: Uuid::new_v4(),
        iat: 1_700_000_000,
        exp,
        kind: Some("Public".to_string()),
        token_type: token_type.map(str::to_string),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(TEST_SECRET.as_bytes()),
    )
    .expect("encode test token")
}

async fn extract(token: &str) -> Result<PortalPrincipal, (StatusCode, &'static str)> {
    // `JWT_SECRET` is process-global; this test binary owns it.
    std::env::set_var("JWT_SECRET", TEST_SECRET);
    let state = StubState::new();
    let req = Request::builder()
        .uri("/anything")
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .body(())
        .expect("build req");
    let (mut parts, _) = req.into_parts();
    PortalPrincipal::from_request_parts(&mut parts, &state).await
}

#[tokio::test]
async fn refresh_token_is_rejected_with_401_before_db() {
    let result = extract(&mint(Some("refresh"))).await;
    let (status, msg) = result.expect_err("refresh token must be rejected");
    assert_eq!(status, StatusCode::UNAUTHORIZED, "refresh → 401");
    assert_eq!(msg, "Invalid token type for this endpoint");
}

#[tokio::test]
async fn arbitrary_token_type_is_rejected() {
    let result = extract(&mint(Some("magic-link"))).await;
    let (status, _) = result.expect_err("non-access token_type must be rejected");
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn expired_access_token_is_rejected_with_401_before_db() {
    // An access token whose `exp` is in the past must be rejected by the
    // `Validation::default()` exp check inside `verify_access_token`, BEFORE
    // the unreachable DB pool is ever touched.
    let result = extract(&mint_with_exp(Some("access"), 1_000_000_000)).await; // 2001
    let (status, msg) = result.expect_err("expired token must be rejected");
    assert_eq!(status, StatusCode::UNAUTHORIZED, "expired → 401");
    assert_eq!(msg, "Invalid or expired token");
}

#[tokio::test]
async fn access_token_passes_discriminator_then_hits_db() {
    // An access token clears the token_type gate and proceeds to the DB
    // lookup, which fails against the unreachable stub pool. The key
    // assertion is that the failure is NOT the 401 token-type rejection —
    // i.e. the discriminator let it through.
    let result = extract(&mint(Some("access"))).await;
    let (status, msg) = result.expect_err("unreachable pool → lookup error");
    assert_ne!(
        (status, msg),
        (
            StatusCode::UNAUTHORIZED,
            "Invalid token type for this endpoint"
        ),
        "access token must NOT be rejected by the token_type gate"
    );
}

#[tokio::test]
async fn legacy_token_without_token_type_passes_discriminator() {
    // Backward-compat: a token minted before token_type existed omits the
    // claim entirely. PortalPrincipal is `LegacyTolerant`, so it must still
    // clear the gate (treated as access) and proceed to the DB.
    let result = extract(&mint(None)).await;
    let (status, msg) = result.expect_err("unreachable pool → lookup error");
    assert_ne!(
        (status, msg),
        (
            StatusCode::UNAUTHORIZED,
            "Invalid token type for this endpoint"
        ),
        "legacy token (no token_type) must NOT be rejected by the gate"
    );
}
