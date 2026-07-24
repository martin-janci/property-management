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
//! Success-path note: there is no `voice_assistant_devices` migration in the
//! PPT schema, so a fully-authenticated request gets *past* the auth gate and
//! then fails at the DB insert. The security contract is what we assert: a
//! valid token is NOT rejected at the auth layer (status is never 401/403),
//! while an unauthenticated or org-less request IS rejected before any work.

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
// status is NOT 401/403. (It does not reach a 200 because the PPT schema has
// no `voice_assistant_devices` table, so the downstream DB insert fails — but
// that is well past the security boundary this fix establishes.)
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
