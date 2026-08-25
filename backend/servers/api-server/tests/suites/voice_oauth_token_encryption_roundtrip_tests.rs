//! End-to-end round-trip encryption tests for centralized voice OAuth token
//! persistence (#2838).
//!
//! `encrypt_voice_token_pair`
//! (`backend/servers/api-server/src/routes/voice_webhooks.rs`) is the single
//! choke point through which both `oauth_token_exchange` and
//! `oauth_token_refresh` persist upstream voice tokens. #2838 collapsed four
//! copy-pasted "encrypt access + optional refresh + derive indexed access-token
//! hash" blocks into it. This suite proves, end to end through the real HTTP
//! handler + Postgres, that for every provider the stored token material:
//!   - is encrypted at rest — carries the `enc:` prefix and never contains the
//!     plaintext token (no plaintext at rest),
//!   - round-trips — the stored ciphertext decrypts back to the minted token,
//!   - keeps the indexed `access_token_hash` in lock-step (#2662): it equals the
//!     keyed HMAC of the SAME plaintext that the stored ciphertext decrypts to,
//!     so the O(1) device lookup can never drift from the encrypted token.
//!
//! Providers covered: Alexa, Google, and the fallback (simulated) provider. In
//! a test/debug build neither upstream OAuth client is configured, so both the
//! `alexa` and `google_assistant` exchanges take the simulated-token fallback
//! branch (security #890 gates it behind `debug_assertions`), which still routes
//! through the mandatory `encrypt_voice_token_pair`.
//!
//! Note on `INTEGRATION_ENCRYPTION_KEY`: these tests set it to `TEST_KEY`, the
//! same value the sibling `voice_oauth_exchange_auth_tests` uses, so concurrent
//! `set_var` across the suite_8 binary is a race-free no-op.

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::{Duration, Utc};
use hmac::{Hmac, KeyInit, Mac};
use integrations::{decrypt_if_available, IntegrationCrypto, ENCRYPTION_KEY_ENV};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::json;
use sha2::Sha256;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{TestApp, TestConfig};

type HmacSha256 = Hmac<Sha256>;

const EXCHANGE_URI: &str = "/api/v1/webhooks/voice/oauth/exchange";

/// A valid 32-byte (64 hex char) AES-256 key. Shared with the other voice-oauth
/// suites; every suite_8 test sets this same value, so a concurrent `set_var`
/// to an identical value cannot race.
const TEST_KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// Mirror of `api_core::extractors::auth::Claims` — the shape `AuthUser`
/// decodes for PM access-token bearer auth.
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

fn exchange_body(platform: &str) -> serde_json::Value {
    json!({
        "auth_code": "test-authorization-code",
        "platform": platform,
        "redirect_uri": "https://ppt.three-two-bit.com/api/v1/webhooks/voice/oauth/callback",
        "state": null
    })
}

/// Drive one authenticated exchange for `platform` (which takes the simulated
/// fallback branch in a debug build), then assert the persisted token material
/// is encrypted at rest, round-trips, and keeps its access-token hash in
/// lock-step with the stored ciphertext.
async fn assert_encrypted_roundtrip_for(pool: &PgPool, platform: &str) {
    std::env::set_var(ENCRYPTION_KEY_ENV, TEST_KEY);

    let config = TestConfig::default();
    let secret = config.jwt_secret.clone();
    let app = TestApp::with_config(pool.clone(), config).await;

    let user_id = Uuid::new_v4();
    let org_id = Uuid::new_v4();
    let token = mint_access_token(&secret, user_id, Some(org_id));

    let req = Request::builder()
        .method(Method::POST)
        .uri(EXCHANGE_URI)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(exchange_body(platform).to_string()))
        .unwrap();

    let resp = app.execute(req).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "[{platform}] exchange (fallback branch) must succeed, got {}: {}",
        resp.status,
        resp.text()
    );

    // Read the stored token material straight from the persisted row.
    let (access_ct, refresh_ct, access_hash): (Option<String>, Option<String>, Option<Vec<u8>>) =
        sqlx::query_as(
            "SELECT access_token_encrypted, refresh_token_encrypted, access_token_hash \
             FROM voice_assistant_devices \
             WHERE organization_id = $1 AND user_id = $2 AND platform = $3 AND is_active = TRUE",
        )
        .bind(org_id)
        .bind(user_id)
        .bind(platform)
        .fetch_one(pool)
        .await
        .expect("stored voice device row");

    let access_ct = access_ct.expect("access token must be persisted");
    let refresh_ct = refresh_ct.expect("the simulated fallback always mints a refresh token");

    let crypto = IntegrationCrypto::new(TEST_KEY).expect("crypto");

    // The plaintext shapes the simulated fallback mints for this platform.
    let access_marker = format!("voice_access_{platform}_");
    let refresh_marker = format!("voice_refresh_{platform}_");

    // (1) Encrypted at rest — enc: prefix, and the plaintext token pattern never
    // appears anywhere in the stored ciphertext (no plaintext at rest).
    assert!(
        access_ct.starts_with("enc:"),
        "[{platform}] stored access token must be encrypted (enc: prefix), got {access_ct}"
    );
    assert!(
        refresh_ct.starts_with("enc:"),
        "[{platform}] stored refresh token must be encrypted (enc: prefix), got {refresh_ct}"
    );
    assert!(
        !access_ct.contains(&access_marker),
        "[{platform}] plaintext access token must never be stored: {access_ct}"
    );
    assert!(
        !refresh_ct.contains(&refresh_marker),
        "[{platform}] plaintext refresh token must never be stored: {refresh_ct}"
    );

    // (2) Round-trip — the stored ciphertext decrypts back to a well-formed
    // minted token for this platform.
    let access_plain = decrypt_if_available(Some(&crypto), &access_ct);
    let refresh_plain = decrypt_if_available(Some(&crypto), &refresh_ct);
    assert!(
        access_plain.starts_with(&access_marker),
        "[{platform}] decrypted access token has the wrong shape: {access_plain}"
    );
    assert!(
        refresh_plain.starts_with(&refresh_marker),
        "[{platform}] decrypted refresh token has the wrong shape: {refresh_plain}"
    );

    // (3) Hash lock-step (#2662) — the indexed lookup hash is the keyed HMAC of
    // the SAME plaintext that the stored ciphertext decrypts to.
    let access_hash =
        access_hash.expect("access_token_hash must be persisted when the key is configured");
    let mut mac = HmacSha256::new_from_slice(TEST_KEY.as_bytes()).expect("hmac key");
    mac.update(access_plain.as_bytes());
    let expected = mac.finalize().into_bytes().to_vec();
    assert_eq!(
        access_hash, expected,
        "[{platform}] stored access_token_hash must be the keyed HMAC of the decrypted access token"
    );
}

// ---------------------------------------------------------------------------
// Alexa provider — persisted token material is encrypted, round-trips, hashed.
// ---------------------------------------------------------------------------
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn voice_oauth_exchange_alexa_encrypts_and_roundtrips(pool: PgPool) {
    assert_encrypted_roundtrip_for(&pool, "alexa").await;
}

// ---------------------------------------------------------------------------
// Google provider — same guarantees on the google_assistant platform.
// ---------------------------------------------------------------------------
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn voice_oauth_exchange_google_encrypts_and_roundtrips(pool: PgPool) {
    assert_encrypted_roundtrip_for(&pool, "google_assistant").await;
}
