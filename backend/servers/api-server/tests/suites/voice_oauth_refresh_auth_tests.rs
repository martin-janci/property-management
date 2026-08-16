//! Regression tests for the unauthenticated voice OAuth *refresh* fix (#2769).
//!
//! Audit history: `POST /api/v1/webhooks/voice/oauth/refresh`
//! (`oauth_token_refresh`) accepted only `State` + `Json<{device_id}>` with **no
//! authentication extractor**. Any caller that learned a `device_id` could POST
//! it and force a rotation of that device owner's upstream Amazon/Google OAuth
//! token — an integration-DoS + amplification vector against the provider's
//! token endpoint, executed entirely anonymously.
//!
//! The fix adds the `AuthUser` extractor (PM access-token bearer auth), a
//! per-user rate limit, and an ownership check that only lets the device's
//! owner (or a platform admin) rotate its tokens. This suite pins the contract:
//! - no / invalid bearer token -> 401 (before any DB work)
//! - authenticated caller who does NOT own device -> 403
//! - authenticated owner -> 200 + `access_token_hash` rotates on the row
//! - authenticated **platform admin** who does NOT own the device -> 200 +
//!   rotation (the `is_platform_admin()` arm of `auth.user_id ==
//!   device.user_id || auth.is_platform_admin()` — the only authorization path
//!   that lets a non-owner through, so it is pinned here alongside the 403 arm)
//!
//! TestApp wiring caveat (mirrors `voice_oauth_exchange_auth_tests`): `AuthUser`
//! reads its claims straight from the JWT, so we mint tokens directly. The
//! `voice_assistant_devices` table exists as of migration 00226, so unlike the
//! exchange suite this one can seed a real device row and assert the full
//! owner-rotation path end to end.

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

const REFRESH_URI: &str = "/api/v1/webhooks/voice/oauth/refresh";

/// A valid 32-byte (64 hex char) AES-256 key for the test process, so the
/// simulated-refresh branch's mandatory `encrypt_required` succeeds.
const TEST_KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// Mirror of `api_core::extractors::auth::Claims`.
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

/// Mint an access token carrying an explicit `role` claim. `AuthUser` reads the
/// role straight from the JWT (no DB membership lookup), and
/// `is_platform_admin()` is driven purely by that claim, so a `"platform_admin"`
/// / `"super_admin"` token is sufficient to exercise the admin-bypass arm.
/// The wire values are the `snake_case` serde renames of `TenantRole`.
fn mint_access_token_with_role(
    secret: &str,
    user_id: Uuid,
    tenant_id: Option<Uuid>,
    role: &str,
) -> String {
    let now = Utc::now();
    let claims = AuthClaims {
        sub: user_id,
        iat: now.timestamp(),
        exp: (now + Duration::hours(1)).timestamp(),
        token_type: "access".to_string(),
        tenant_id,
        role: Some(role.to_string()),
        email: "voice-refresh@example.test".to_string(),
        name: "Voice Refresher".to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("mint access token")
}

/// A non-admin (`manager`) access token — the common case for owner /
/// non-owner callers.
fn mint_access_token(secret: &str, user_id: Uuid, tenant_id: Option<Uuid>) -> String {
    mint_access_token_with_role(secret, user_id, tenant_id, "manager")
}

/// Insert a linked voice device owned by `owner` with a refresh token present
/// (so the handler proceeds past the "no refresh token" guard) and a known
/// initial `access_token_hash`. Returns the device id.
async fn seed_device(pool: &PgPool, owner: Uuid, initial_hash: &[u8]) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO voice_assistant_devices (
            organization_id, user_id, platform, device_id,
            access_token_encrypted, refresh_token_encrypted,
            token_expires_at, capabilities, access_token_hash, is_active, linked_at
        )
        VALUES ($1, $2, 'alexa', $3, $4, $5,
                NOW() + interval '1 hour', '["check_balance"]'::jsonb, $6, TRUE, NOW())
        RETURNING id
        "#,
    )
    .bind(Uuid::new_v4()) // organization_id (logical FK, unconstrained per migration 00226)
    .bind(owner)
    .bind(format!("device-{}", Uuid::new_v4()))
    .bind("enc-access-placeholder")
    .bind("enc-refresh-placeholder")
    .bind(initial_hash)
    .fetch_one(pool)
    .await
    .expect("seed voice device")
}

async fn read_token_hash(pool: &PgPool, device_id: Uuid) -> Option<Vec<u8>> {
    sqlx::query_scalar::<_, Option<Vec<u8>>>(
        "SELECT access_token_hash FROM voice_assistant_devices WHERE id = $1",
    )
    .bind(device_id)
    .fetch_one(pool)
    .await
    .expect("read device token hash")
}

fn refresh_body(device_id: Uuid) -> serde_json::Value {
    json!({ "device_id": device_id })
}

// ---------------------------------------------------------------------------
// Test 1: no Authorization header -> 401, before any device lookup or rotation.
// ---------------------------------------------------------------------------
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oauth_refresh_without_auth_is_rejected(pool: PgPool) {
    let owner = Uuid::new_v4();
    let device_id = seed_device(&pool, owner, b"initial-hash").await;
    let app = TestApp::new(pool.clone()).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri(REFRESH_URI)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(refresh_body(device_id).to_string()))
        .unwrap();

    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "unauthenticated refresh must be 401, got {}: {}",
        resp.status,
        resp.text()
    );

    // The device's token hash must be untouched by the rejected request.
    assert_eq!(
        read_token_hash(&pool, device_id).await.as_deref(),
        Some(&b"initial-hash"[..]),
        "rejected request must not rotate the token"
    );
}

// ---------------------------------------------------------------------------
// Test 2: garbage bearer token -> 401.
// ---------------------------------------------------------------------------
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oauth_refresh_invalid_token_is_rejected(pool: PgPool) {
    let device_id = seed_device(&pool, Uuid::new_v4(), b"initial-hash").await;
    let app = TestApp::new(pool.clone()).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri(REFRESH_URI)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer not-a-real-jwt")
        .body(Body::from(refresh_body(device_id).to_string()))
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
// Test 3: authenticated caller who does NOT own the device -> 403, no rotation.
// ---------------------------------------------------------------------------
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oauth_refresh_non_owner_is_forbidden(pool: PgPool) {
    let owner = Uuid::new_v4();
    let device_id = seed_device(&pool, owner, b"initial-hash").await;

    let config = TestConfig::default();
    let secret = config.jwt_secret.clone();
    let app = TestApp::with_config(pool.clone(), config).await;

    // Token for a DIFFERENT, non-admin user.
    let attacker = Uuid::new_v4();
    let token = mint_access_token(&secret, attacker, Some(Uuid::new_v4()));

    let req = Request::builder()
        .method(Method::POST)
        .uri(REFRESH_URI)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(refresh_body(device_id).to_string()))
        .unwrap();

    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "non-owner refresh must be 403, got {}: {}",
        resp.status,
        resp.text()
    );

    assert_eq!(
        read_token_hash(&pool, device_id).await.as_deref(),
        Some(&b"initial-hash"[..]),
        "forbidden request must not rotate the token"
    );
}

// ---------------------------------------------------------------------------
// Test 4: the device owner -> 200 and the stored access_token_hash rotates.
//
// No upstream OAuth is configured in tests, so the handler takes its
// simulated-refresh branch: it mints a fresh access token, encrypts it (needs
// INTEGRATION_ENCRYPTION_KEY, set below) and writes a new keyed HMAC into
// `access_token_hash`. We assert the row's hash changed from the seeded value.
// ---------------------------------------------------------------------------
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oauth_refresh_owner_rotates_token(pool: PgPool) {
    std::env::set_var("INTEGRATION_ENCRYPTION_KEY", TEST_KEY);

    let owner = Uuid::new_v4();
    let initial_hash = b"initial-hash-value";
    let device_id = seed_device(&pool, owner, initial_hash).await;

    let config = TestConfig::default();
    let secret = config.jwt_secret.clone();
    let app = TestApp::with_config(pool.clone(), config).await;

    let token = mint_access_token(&secret, owner, Some(Uuid::new_v4()));

    let req = Request::builder()
        .method(Method::POST)
        .uri(REFRESH_URI)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(refresh_body(device_id).to_string()))
        .unwrap();

    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "owner refresh must be 200, got {}: {}",
        resp.status,
        resp.text()
    );

    let new_hash = read_token_hash(&pool, device_id).await;
    assert!(
        new_hash.is_some(),
        "owner refresh must persist a new access_token_hash"
    );
    assert_ne!(
        new_hash.as_deref(),
        Some(&initial_hash[..]),
        "owner refresh must rotate access_token_hash away from the seeded value"
    );
}

// ---------------------------------------------------------------------------
// Test 5: a platform admin who does NOT own the device -> 200 and rotation.
//
// The authorization gate is `auth.user_id == device.user_id ||
// auth.is_platform_admin()`. Test 3 pins the 403 arm (non-owner, non-admin);
// this pins the *other* way a caller who is not the owner is allowed through —
// the `is_platform_admin()` bypass. Without this test a regression that dropped
// the admin arm (locking admins out) or, worse, widened it to the wrong roles
// would slip past the suite. The caller here is a different user than the
// owner and carries a `platform_admin` role claim; the seeded device is owned
// by someone else. Same simulated-refresh branch as Test 4, so
// `INTEGRATION_ENCRYPTION_KEY` is set and we assert the hash rotates.
// ---------------------------------------------------------------------------
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oauth_refresh_platform_admin_non_owner_rotates_token(pool: PgPool) {
    std::env::set_var("INTEGRATION_ENCRYPTION_KEY", TEST_KEY);

    let owner = Uuid::new_v4();
    let initial_hash = b"initial-hash-admin";
    let device_id = seed_device(&pool, owner, initial_hash).await;

    let config = TestConfig::default();
    let secret = config.jwt_secret.clone();
    let app = TestApp::with_config(pool.clone(), config).await;

    // A DIFFERENT user who is a platform admin (not the device owner).
    let admin = Uuid::new_v4();
    assert_ne!(admin, owner, "admin must not coincidentally be the owner");
    let token = mint_access_token_with_role(&secret, admin, Some(Uuid::new_v4()), "platform_admin");

    let req = Request::builder()
        .method(Method::POST)
        .uri(REFRESH_URI)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(refresh_body(device_id).to_string()))
        .unwrap();

    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "platform-admin non-owner refresh must be 200, got {}: {}",
        resp.status,
        resp.text()
    );

    let new_hash = read_token_hash(&pool, device_id).await;
    assert!(
        new_hash.is_some(),
        "platform-admin refresh must persist a new access_token_hash"
    );
    assert_ne!(
        new_hash.as_deref(),
        Some(&initial_hash[..]),
        "platform-admin refresh must rotate access_token_hash away from the seeded value"
    );
}
