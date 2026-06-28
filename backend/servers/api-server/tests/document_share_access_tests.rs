//! Public document-share access integration tests (Epic 7B, issue #807).
//!
//! The public share-access routes are mounted at the router root (merged, not
//! nested under `/api/v1/documents`):
//!
//! - `GET  /shared/{token}`         — open a share (no password)
//! - `POST /shared/{token}/access`  — open a password-protected share
//!
//! These tests pin the security contract for those public, unauthenticated
//! routes:
//!
//! - An **expired** share token (`expires_at < now()`) is rejected with 404,
//!   not served. Expiry is enforced in the SQL predicate of
//!   `find_share_by_token` (`AND (expires_at IS NULL OR expires_at > NOW())`);
//!   a regression that drops that predicate would surface here.
//! - A **valid, non-expired** link share is served (200) with download/preview
//!   URLs.
//! - A **password-protected** share returns 401 on the no-password GET route,
//!   rejects a wrong password (401) on the POST route, and serves the document
//!   (200) for the correct password.
//! - A **revoked** share is rejected with 404.
//!
//! Shares are seeded with raw SQL; the password hash is produced with the same
//! `argon2` default the create-share handler uses, so `verify_share_password`
//! accepts it.

#![allow(dead_code)]

#[allow(dead_code)]
mod common;

use argon2::{password_hash::PasswordHasher, Argon2};
use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{cleanup_test_user, create_authenticated_user, seed_membership, TestApp, TestUser};

// ============================================================================
// Seed helpers
// ============================================================================

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO organizations (name, slug, contact_email, status) \
         VALUES ($1, $2, $3, 'active') RETURNING id",
    )
    .bind(format!("ShareTest {slug}"))
    .bind(format!("share-test-{slug}"))
    .bind(format!("{slug}@share-test.example"))
    .fetch_one(pool)
    .await
    .expect("seed_org")
}

async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("user_id_for")
}

/// Insert a `documents` row directly and return its id.
async fn seed_document(pool: &PgPool, org_id: Uuid, created_by: Uuid, title: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO documents \
             (organization_id, title, category, file_key, file_name, mime_type, \
              size_bytes, access_scope, created_by) \
         VALUES ($1, $2, 'reports', $3, 'report.pdf', 'application/pdf', 1024, \
                 'organization', $4) \
         RETURNING id",
    )
    .bind(org_id)
    .bind(title)
    .bind(format!("{org_id}/test/{}.pdf", Uuid::new_v4()))
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed_document")
}

/// Hash a password exactly as the share-create handler does (`argon2`
/// default, PHC string), so `verify_share_password` accepts it.
fn hash_password(password: &str) -> String {
    Argon2::default()
        .hash_password(password.as_bytes())
        .expect("hash_password")
        .to_string()
}

/// Insert a `link`-type share row directly and return its `share_token`.
///
/// Done with raw SQL (rather than `DocumentRepository::create_share`) so the
/// test does not depend on the deprecated repo method's `RETURNING *` decode
/// path. The expiry / revocation / password columns are written verbatim,
/// which is exactly what the public read path filters on.
async fn seed_link_share(
    pool: &PgPool,
    document_id: Uuid,
    shared_by: Uuid,
    password: Option<&str>,
    expires_at: Option<DateTime<Utc>>,
) -> String {
    let token: String = format!("tok{}", Uuid::new_v4().simple());
    let password_hash = password.map(hash_password);
    sqlx::query(
        "INSERT INTO document_shares \
             (document_id, share_type, shared_by, share_token, password_hash, expires_at) \
         VALUES ($1, 'link'::document_share_type, $2, $3, $4, $5)",
    )
    .bind(document_id)
    .bind(shared_by)
    .bind(&token)
    .bind(password_hash)
    .bind(expires_at)
    .execute(pool)
    .await
    .expect("seed_link_share");
    token
}

/// Full setup: org + authenticated user (as creator) + document. Returns
/// `(pool-ready org_id, user_id, document_id)`.
async fn setup(app: &TestApp, pool: &PgPool, user: &TestUser, title: &str) -> (Uuid, Uuid, Uuid) {
    cleanup_test_user(pool, &user.email).await;
    let (_token, _) = create_authenticated_user(app, user).await;
    let user_id = user_id_for(pool, &user.email).await;
    let org_id = seed_org(pool, &Uuid::new_v4().to_string()[..8]).await;
    seed_membership(pool, org_id, user_id, "admin").await;
    let doc_id = seed_document(pool, org_id, user_id, title).await;
    (org_id, user_id, doc_id)
}

// ============================================================================
// Expiry enforcement (issue #807)
// ============================================================================

/// An expired share token must NOT serve the document. The SQL predicate
/// `expires_at > NOW()` filters it out, so the handler sees `None` → 404.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_expired_share_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (_org, user_id, doc_id) = setup(&app, &pool, &user, "Expired Share Doc").await;

    // expires_at one hour in the PAST
    let token = seed_link_share(
        &pool,
        doc_id,
        user_id,
        None,
        Some(Utc::now() - Duration::hours(1)),
    )
    .await;

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/shared/{token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.execute(request).await;

    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "expired share must be rejected with 404; got {}. Body: {}",
        response.status,
        response.text()
    );

    cleanup_test_user(&pool, &user.email).await;
}

/// A valid, non-expired link share serves the document with 200 + URLs.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_valid_share_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (_org, user_id, doc_id) = setup(&app, &pool, &user, "Valid Share Doc").await;

    // expires_at one hour in the FUTURE
    let token = seed_link_share(
        &pool,
        doc_id,
        user_id,
        None,
        Some(Utc::now() + Duration::hours(1)),
    )
    .await;

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/shared/{token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.execute(request).await;

    assert_eq!(
        response.status,
        StatusCode::OK,
        "valid share must succeed with 200; got {}. Body: {}",
        response.status,
        response.text()
    );

    let body = response.json_value();
    assert_eq!(
        body["document"]["id"].as_str(),
        Some(doc_id.to_string().as_str()),
        "response must reference the shared document"
    );
    assert!(
        body["download_url"].as_str().is_some(),
        "response must contain a download_url"
    );

    cleanup_test_user(&pool, &user.email).await;
}

/// A link share with `expires_at = NULL` (never expires) succeeds.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_non_expiring_share_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (_org, user_id, doc_id) = setup(&app, &pool, &user, "No-Expiry Share Doc").await;

    let token = seed_link_share(&pool, doc_id, user_id, None, None).await;

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/shared/{token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.execute(request).await;

    assert_eq!(
        response.status,
        StatusCode::OK,
        "non-expiring share must succeed with 200; got {}. Body: {}",
        response.status,
        response.text()
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// Password enforcement (issue #807)
// ============================================================================

/// The no-password GET route must NOT serve a password-protected share — it
/// returns 401 PASSWORD_REQUIRED and never leaks the document URLs.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_password_protected_share_requires_password_on_get(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (_org, user_id, doc_id) = setup(&app, &pool, &user, "Pwd Share Doc").await;

    let token = seed_link_share(
        &pool,
        doc_id,
        user_id,
        Some("s3cret-pass"),
        Some(Utc::now() + Duration::hours(1)),
    )
    .await;

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/shared/{token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.execute(request).await;

    assert_eq!(
        response.status,
        StatusCode::UNAUTHORIZED,
        "password-protected share must return 401 on the GET route; got {}. Body: {}",
        response.status,
        response.text()
    );
    let body = response.json_value();
    assert_eq!(
        body["code"].as_str(),
        Some("PASSWORD_REQUIRED"),
        "error code must be PASSWORD_REQUIRED"
    );
    assert!(
        body.get("download_url").is_none(),
        "password-protected GET must not leak a download_url"
    );

    cleanup_test_user(&pool, &user.email).await;
}

/// The POST access route rejects a wrong password with 401.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_wrong_password_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (_org, user_id, doc_id) = setup(&app, &pool, &user, "Wrong Pwd Doc").await;

    let token = seed_link_share(
        &pool,
        doc_id,
        user_id,
        Some("correct-horse"),
        Some(Utc::now() + Duration::hours(1)),
    )
    .await;

    let request = Request::builder()
        .method(Method::POST)
        .uri(format!("/shared/{token}/access"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "password": "wrong-battery" }).to_string(),
        ))
        .unwrap();
    let response = app.execute(request).await;

    assert_eq!(
        response.status,
        StatusCode::UNAUTHORIZED,
        "wrong password must be rejected with 401; got {}. Body: {}",
        response.status,
        response.text()
    );

    cleanup_test_user(&pool, &user.email).await;
}

/// The POST access route serves the document for the correct password.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_correct_password_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (_org, user_id, doc_id) = setup(&app, &pool, &user, "Correct Pwd Doc").await;

    let token = seed_link_share(
        &pool,
        doc_id,
        user_id,
        Some("correct-horse"),
        Some(Utc::now() + Duration::hours(1)),
    )
    .await;

    let request = Request::builder()
        .method(Method::POST)
        .uri(format!("/shared/{token}/access"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "password": "correct-horse" }).to_string(),
        ))
        .unwrap();
    let response = app.execute(request).await;

    assert_eq!(
        response.status,
        StatusCode::OK,
        "correct password must serve the document with 200; got {}. Body: {}",
        response.status,
        response.text()
    );
    let body = response.json_value();
    assert_eq!(
        body["document"]["id"].as_str(),
        Some(doc_id.to_string().as_str()),
        "response must reference the shared document"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// Revocation (defense-in-depth — shares the same SQL predicate as expiry)
// ============================================================================

/// A revoked share is filtered out (`revoked_at IS NULL`) → 404.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_revoked_share_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (_org, user_id, doc_id) = setup(&app, &pool, &user, "Revoked Share Doc").await;

    let token = seed_link_share(
        &pool,
        doc_id,
        user_id,
        None,
        Some(Utc::now() + Duration::hours(1)),
    )
    .await;

    sqlx::query("UPDATE document_shares SET revoked_at = NOW() WHERE share_token = $1")
        .bind(&token)
        .execute(&pool)
        .await
        .expect("revoke share");

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/shared/{token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.execute(request).await;

    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "revoked share must be rejected with 404; got {}. Body: {}",
        response.status,
        response.text()
    );

    cleanup_test_user(&pool, &user.email).await;
}
