//! Regression tests for the unauthenticated voice OAuth exchange fix (#890).
//!
//! Audit history: `POST /api/v1/webhooks/voice/oauth/exchange`
//! (`oauth_token_exchange`) accepted an `auth_code` + `platform` from any
//! caller with **no authentication**, then minted a voice device bound to
//! `Uuid::new_v4()` for both `user_id` and `org_id` — creating orphan,
//! cross-tenant-unscoped voice devices.
//!
//! The fix adds the `AuthUser` extractor (PM access-token bearer auth) and
//! derives `user_id` + `org_id` from the verified token claims, rejecting:
//!   - requests with no/invalid bearer token  -> 401
//!   - authenticated requests with no active org (no `tenant_id` claim) -> 403
//!
//! TestApp wiring caveat: `AuthUser` reads `tenant_id` directly from the JWT
//! claims (no `host_tenant_middleware` needed), so we mint tokens carrying a
//! custom `tenant_id` claim to exercise the authenticated paths.
//!
//! Success-path note: the `voice_assistant_devices` table exists as of
//! migration 00226, so a fully-authenticated request runs end to end. Test 4
//! asserts the security contract (a valid token is never rejected at the auth
//! layer, while an unauthenticated or org-less request IS rejected before any
//! work); Test 5 asserts the device-dedup contract (re-linking rotates one row
//! rather than accumulating independent rows).

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{TestApp, TestConfig};

const EXCHANGE_URI: &str = "/api/v1/webhooks/voice/oauth/exchange";

/// Mirror of `api_core::extractors::auth::Claims` — the shape `AuthUser`
/// decodes. `tenant_id`/`role` are `Option` so we can mint a token with or
/// without an active organization context.
#[derive(Serialize)]
struct AuthClaims {
    sub: Uuid,
    exp: i64,
    iat: i64,
    token_type: String,
    tenant_id: Option<Uuid>,
    role: Option<String>,
    email: String,
    name: String,
}

fn mint_access_token(secret: &str, user_id: Uuid, tenant_id: Option<Uuid>) -> String {
    let now = Utc::now();
    let claims = AuthClaims {
        sub: user_id,
        iat: now.timestamp(),
        exp: (now + Duration::hours(1)).timestamp(),
        token_type: "access".to_string(),
        tenant_id,
        role: Some("manager".to_string()),
        email: "voice-link@example.test".to_string(),
        name: "Voice Linker".to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("mint access token")
}

fn exchange_body() -> serde_json::Value {
    json!({
        "auth_code": "test-authorization-code",
        "platform": "alexa",
        "redirect_uri": "https://ppt.three-two-bit.com/api/v1/webhooks/voice/oauth/callback",
        "state": null
    })
}

// ---------------------------------------------------------------------------
// Test 1: no Authorization header -> 401, no device created.
// ---------------------------------------------------------------------------
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oauth_exchange_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri(EXCHANGE_URI)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(exchange_body().to_string()))
        .unwrap();

    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "unauthenticated exchange must be 401, got {}: {}",
        resp.status,
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// Test 2: garbage bearer token -> 401.
// ---------------------------------------------------------------------------
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oauth_exchange_invalid_token_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri(EXCHANGE_URI)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer not-a-real-jwt")
        .body(Body::from(exchange_body().to_string()))
        .unwrap();

    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "invalid bearer token must be 401, got {}: {}",
        resp.status,
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// Test 3: authenticated but no org context (no tenant_id claim) -> 403.
// ---------------------------------------------------------------------------
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oauth_exchange_without_org_context_is_forbidden(pool: PgPool) {
    let config = TestConfig::default();
    let secret = config.jwt_secret.clone();
    let app = TestApp::with_config(pool.clone(), config).await;

    let token = mint_access_token(&secret, Uuid::new_v4(), None);

    let req = Request::builder()
        .method(Method::POST)
        .uri(EXCHANGE_URI)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(exchange_body().to_string()))
        .unwrap();

    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "org-less exchange must be 403, got {}: {}",
        resp.status,
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// Test 4: valid token with an org context passes the auth gate.
//
// Proves the extractor admits a real authenticated principal: the response
// status is NOT 401/403. (This test sets no `INTEGRATION_ENCRYPTION_KEY`, so it
// stops at the mandatory-encryption gate rather than reaching a 200 — but that
// is well past the security boundary this fix establishes. Test 5 exercises the
// full 200 success path with the key set.)
// ---------------------------------------------------------------------------
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oauth_exchange_with_valid_auth_passes_auth_gate(pool: PgPool) {
    let config = TestConfig::default();
    let secret = config.jwt_secret.clone();
    let app = TestApp::with_config(pool.clone(), config).await;

    let token = mint_access_token(&secret, Uuid::new_v4(), Some(Uuid::new_v4()));

    let req = Request::builder()
        .method(Method::POST)
        .uri(EXCHANGE_URI)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(exchange_body().to_string()))
        .unwrap();

    let resp = app.execute(req).await;

    assert_ne!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "valid token must clear the auth gate, got 401: {}",
        resp.text()
    );
    assert_ne!(
        resp.status,
        StatusCode::FORBIDDEN,
        "valid token with org context must clear the org gate, got 403: {}",
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// Test 5: re-linking the same (org, user, platform) reuses ONE device row
// instead of accumulating independent rows with independently-usable tokens.
//
// Regression for the device-dedup fix: `oauth_token_exchange` used to mint a
// fresh random `device_id` and INSERT a new `voice_assistant_devices` row on
// every call, so re-linking left the previous row active with its own still
// usable stored token (stale-token accumulation). The handler now upserts on
// `(organization_id, user_id, platform)`: the second link rotates the tokens
// on the existing row in place, so exactly one active row exists and the
// returned `device_id` is stable across re-links.
//
// No upstream OAuth is configured in tests, so the handler takes its
// simulated-tokens branch (debug build) which still runs the mandatory
// `encrypt_required` — hence `INTEGRATION_ENCRYPTION_KEY` is set below.
// ---------------------------------------------------------------------------

/// A valid 32-byte (64 hex char) AES-256 key so the simulated-tokens branch's
/// mandatory `encrypt_required` succeeds.
const TEST_KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oauth_exchange_relink_reuses_single_device_row(pool: PgPool) {
    std::env::set_var("INTEGRATION_ENCRYPTION_KEY", TEST_KEY);

    let config = TestConfig::default();
    let secret = config.jwt_secret.clone();
    let app = TestApp::with_config(pool.clone(), config).await;

    let user_id = Uuid::new_v4();
    let org_id = Uuid::new_v4();
    let token = mint_access_token(&secret, user_id, Some(org_id));

    let build_req = || {
        Request::builder()
            .method(Method::POST)
            .uri(EXCHANGE_URI)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::from(exchange_body().to_string()))
            .unwrap()
    };

    // First link.
    let resp1 = app.execute(build_req()).await;
    assert_eq!(
        resp1.status,
        StatusCode::OK,
        "first link must succeed, got {}: {}",
        resp1.status,
        resp1.text()
    );
    let body1: serde_json::Value = serde_json::from_str(&resp1.text()).unwrap();
    let device_id_1 = body1["device_id"].as_str().unwrap().to_string();

    // Re-link (same user + org + platform).
    let resp2 = app.execute(build_req()).await;
    assert_eq!(
        resp2.status,
        StatusCode::OK,
        "re-link must succeed, got {}: {}",
        resp2.status,
        resp2.text()
    );
    let body2: serde_json::Value = serde_json::from_str(&resp2.text()).unwrap();
    let device_id_2 = body2["device_id"].as_str().unwrap().to_string();

    assert_eq!(
        device_id_1, device_id_2,
        "re-link must reuse the same device row, not mint a new device_id"
    );

    // Exactly one active device row exists for the tuple.
    let active_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM voice_assistant_devices \
         WHERE organization_id = $1 AND user_id = $2 AND platform = $3 AND is_active = TRUE",
    )
    .bind(org_id)
    .bind(user_id)
    .bind("alexa")
    .fetch_one(&pool)
    .await
    .expect("count active voice devices");

    assert_eq!(
        active_count, 1,
        "re-link must not accumulate active device rows (found {active_count})"
    );
}
