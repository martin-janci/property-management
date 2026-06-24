//! Regression tests for the OCR meter-reading endpoints' auth gate (#1772).
//!
//! `POST /api/v1/ai/ocr/meter-reading` and `POST /api/v1/ai/ocr/correction`
//! originally took NO auth extractor, so any anonymous caller could drive the
//! image-upload / feedback sink — out of step with the deny-all AI-router
//! invariant every sibling handler honors. The fix wires a `TenantExtractor`
//! (validates the JWT + tenant context) onto both handlers.
//!
//! These tests pin that contract:
//!   * no Authorization header        -> 401 (the auth layer fires BEFORE the
//!                                       multipart/body is read);
//!   * authenticated meter-reading    -> 501 OCR_NOT_CONFIGURED (honest stub);
//!   * authenticated correction       -> 200 (accepted-and-discarded stub).
//!
//! Auth mirrors `rental_guest_id_document_tests.rs`: a minted access JWT +
//! `X-Tenant-ID` header. The OCR stubs touch no DB and check only that a tenant
//! is present, so no `organization_members` seeding is required for the
//! authenticated cases — but we seed an org so the `X-Tenant-ID` is a real id.

#[allow(dead_code)]
mod common;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_org, TestApp};

// Must match `TestConfig::default().jwt_secret` in tests/common/mod.rs.
const JWT_SECRET: &str = "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

const MULTIPART_BOUNDARY: &str = "----ppt1772ocrboundary";

#[derive(Serialize)]
struct AccessClaims {
    sub: Uuid,
    exp: i64,
    iat: i64,
    token_type: String,
    tenant_id: Option<Uuid>,
    role: Option<String>,
    email: String,
    name: String,
}

fn mint_access_token(user_id: Uuid, tenant_id: Uuid) -> String {
    let now = Utc::now();
    let claims = AccessClaims {
        sub: user_id,
        iat: now.timestamp(),
        exp: (now + Duration::hours(1)).timestamp(),
        token_type: "access".to_string(),
        tenant_id: Some(tenant_id),
        role: Some("manager".to_string()),
        email: "ocr-auth@test.local".to_string(),
        name: "OCR Auth Test".to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("mint access token")
}

/// Build a `multipart/form-data` body with a single `image` part (the field the
/// handler drains). Content is a tiny PNG-typed payload.
fn multipart_image() -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{MULTIPART_BOUNDARY}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"image\"; filename=\"meter.png\"\r\n",
    );
    body.extend_from_slice(b"Content-Type: image/png\r\n\r\n");
    body.extend_from_slice(&[0x89, b'P', b'N', b'G', 1, 2, 3]);
    body.extend_from_slice(format!("\r\n--{MULTIPART_BOUNDARY}--\r\n").as_bytes());
    body
}

const CORRECTION_BODY: &str = r#"{
    "original_value": 1234.0,
    "corrected_value": 1235.0,
    "image_url": "https://example.test/m.png",
    "bounding_box": null,
    "timestamp": "2026-01-01T00:00:00Z"
}"#;

// ---------------------------------------------------------------------------
// (1) No auth header -> 401, BEFORE the multipart/body is read.
//
// This is the failing-on-dev IG3: pre-fix the handler had no extractor, so an
// anonymous POST reached the handler and returned 501/200. With `TenantExtractor`
// in front, `AuthUser` rejects the missing bearer with 401 first.
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn ocr_meter_reading_without_auth_is_401(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/ai/ocr/meter-reading")
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={MULTIPART_BOUNDARY}"),
        )
        .body(Body::from(multipart_image()))
        .unwrap();
    let resp = app.execute(req).await;
    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "anonymous meter-reading must be 401 (auth fires before the body), got {}: {}",
        resp.status,
        resp.text()
    );
}

#[sqlx::test]
async fn ocr_correction_without_auth_is_401(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/ai/ocr/correction")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(CORRECTION_BODY))
        .unwrap();
    let resp = app.execute(req).await;
    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "anonymous correction must be 401, got {}: {}",
        resp.status,
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// (2) Authenticated -> the honest stub responses are preserved.
//
// Proves the new auth gate does not break the existing 501/200 contract for a
// legitimately authenticated tenant.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ocr_meter_reading_authenticated_is_501_not_configured(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "ocr-501").await;
    let token = mint_access_token(Uuid::new_v4(), org);

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/ai/ocr/meter-reading")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org.to_string())
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={MULTIPART_BOUNDARY}"),
        )
        .body(Body::from(multipart_image()))
        .unwrap();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_IMPLEMENTED,
        "authenticated meter-reading must be 501, got {}: {}",
        resp.status,
        resp.text()
    );
    assert!(
        resp.text().contains("OCR_NOT_CONFIGURED"),
        "501 body should carry the OCR_NOT_CONFIGURED code: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ocr_correction_authenticated_is_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "ocr-200").await;
    let token = mint_access_token(Uuid::new_v4(), org);

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/ai/ocr/correction")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org.to_string())
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(CORRECTION_BODY))
        .unwrap();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "authenticated correction must be 200, got {}: {}",
        resp.status,
        resp.text()
    );
}
