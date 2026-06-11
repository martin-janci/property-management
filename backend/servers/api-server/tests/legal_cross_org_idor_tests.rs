//! Regression tests for the cross-org IDOR fix on legal-document endpoints (#829).
//!
//! Audit history: the by-id handlers in
//! `servers/api-server/src/routes/legal.rs` (`get_document`, `update_document`,
//! `delete_document`, and the sibling requirement/notice/template handlers)
//! extracted nothing from the caller and called the repository with only the
//! resource UUID — e.g. `find_document_by_id(id)`. Any authenticated user who
//! knew (or guessed) another org's document id could read, update, or delete
//! that org's legal documents (P0/P1 IDOR — confidential contracts, GDPR
//! notices, compliance PII).
//!
//! The fix derives the verified `org_id` from the JWT `tenant_id` claim (the
//! same idiom as `violations.rs` / `reserve_funds.rs`) and threads it into the
//! repository so the SQL is org-scoped:
//!   `SELECT * FROM legal_documents WHERE id = $1 AND organization_id = $2`.
//! A foreign-org row therefore surfaces as `None` → HTTP 404, never a
//! cross-tenant read or write.
//!
//! Since the PAP-80 RLS conversion the handlers acquire an `RlsConnection`:
//! tenancy comes from the validated `X-Tenant-ID` header backed by an active
//! `organization_members` row (the JWT `tenant_id` claim alone is not a tenant
//! source). These tests mint a real access token per org, seed the membership
//! row, send the header, and drive the handlers end-to-end:
//!   - Org A's context reading Org A's document -> 200 (same-org succeeds).
//!   - Org B's own valid context reading Org A's document -> 404 (org-scoped
//!     query finds no row; cross-org is blocked).

#[allow(dead_code)]
mod common;

use axum::http::{Method, StatusCode};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use common::{RequestBuilder, TestApp, TestConfig};

// ---------------------------------------------------------------------------
// JWT minting (matches api_core::extractors::auth::Claims)
// ---------------------------------------------------------------------------
//
// NOTE: `JwtService` emits the org claim as `org_id`, which does NOT map to
// `Claims.tenant_id`. We therefore mint a custom claims struct whose field is
// literally `tenant_id`, matching what `AuthUser` reads.

#[derive(Serialize)]
struct TestClaims {
    sub: Uuid,
    exp: i64,
    iat: i64,
    token_type: String,
    tenant_id: Option<Uuid>,
    role: Option<String>,
    email: String,
    name: String,
}

/// Mint an access token bound to `org_id` (the JWT `tenant_id` claim), signed
/// with the same secret `TestApp` configures.
fn access_token(user_id: Uuid, org_id: Option<Uuid>) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: org_id,
        role: Some("manager".to_string()),
        email: "idor-test@legal.test".to_string(),
        name: "Legal IDOR User".to_string(),
    };
    let secret = TestConfig::default().jwt_secret;
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("encode test JWT")
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("LegalIDOR Org {slug}"))
    .bind(format!("legal-idor-{slug}"))
    .bind(format!("{slug}@legal-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'LegalIDOR User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_document(pool: &PgPool, org_id: Uuid, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO legal_documents (organization_id, document_type, title, created_by)
        VALUES ($1, 'contract', 'Cross-org Confidential Contract', $2)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed document")
}

/// Make `user_id` an active member of `org_id` — required by the
/// `RlsConnection` membership check since the PAP-80 conversion.
async fn seed_membership(pool: &PgPool, org_id: Uuid, user_id: Uuid) {
    sqlx::query(
        r#"
        INSERT INTO organization_members (organization_id, user_id, role_type, status, joined_at)
        VALUES ($1, $2, 'manager', 'active', NOW())
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("seed membership");
}

// ---------------------------------------------------------------------------
// Test 1: same-org read succeeds (proves real org context is wired)
// ---------------------------------------------------------------------------

/// A user whose JWT `tenant_id` matches the document's organization can read it.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_document_same_org_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "same-org@legal-idor.test").await;
    let org_a = seed_org(&pool, "same-a").await;
    let doc_id = seed_document(&pool, org_a, user).await;

    seed_membership(&pool, org_a, user).await;
    let token = access_token(user, Some(org_a));
    let req = app
        .get(&format!("/api/v1/legal/documents/{doc_id}"))
        .bearer(&token)
        .header("X-Tenant-ID", &org_a.to_string())
        .build();
    let resp = app.execute(req).await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(
        body["id"].as_str(),
        Some(doc_id.to_string().as_str()),
        "same-org read must return the requested document"
    );
}

// ---------------------------------------------------------------------------
// Test 2: cross-org read is blocked (404)
// ---------------------------------------------------------------------------

/// A user from Org B cannot read Org A's document by id — the org-scoped query
/// finds no row and the handler returns 404 (the core #829 IDOR).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_document_cross_org_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user_a = seed_user(&pool, "owner@legal-idor.test").await;
    let user_b = seed_user(&pool, "attacker@legal-idor.test").await;
    let org_a = seed_org(&pool, "x-a").await;
    let org_b = seed_org(&pool, "x-b").await;
    let doc_id = seed_document(&pool, org_a, user_a).await;

    // Org B's own valid tenant context targeting Org A's document — the
    // org-scoped query must come up empty (404), not leak.
    seed_membership(&pool, org_b, user_b).await;
    let token = access_token(user_b, Some(org_b));
    let req = app
        .get(&format!("/api/v1/legal/documents/{doc_id}"))
        .bearer(&token)
        .header("X-Tenant-ID", &org_b.to_string())
        .build();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "cross-org document read must be 404, got {} (body: {})",
        resp.status,
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// Test 3: cross-org update is blocked and does not mutate the row
// ---------------------------------------------------------------------------

/// A cross-org `PATCH` must not rename another org's document.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_document_cross_org_does_not_mutate(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user_a = seed_user(&pool, "upd-owner@legal-idor.test").await;
    let user_b = seed_user(&pool, "upd-attacker@legal-idor.test").await;
    let org_a = seed_org(&pool, "u-a").await;
    let org_b = seed_org(&pool, "u-b").await;
    let doc_id = seed_document(&pool, org_a, user_a).await;

    seed_membership(&pool, org_b, user_b).await;
    let token = access_token(user_b, Some(org_b));
    let req = RequestBuilder::new(Method::PATCH, &format!("/api/v1/legal/documents/{doc_id}"))
        .bearer(&token)
        .header("X-Tenant-ID", &org_b.to_string())
        .json(serde_json::json!({ "title": "Hijacked Document" }))
        .build();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "cross-org document update must be 404, got {} (body: {})",
        resp.status,
        resp.text()
    );

    // The document title must be unchanged.
    let title: String = sqlx::query_scalar("SELECT title FROM legal_documents WHERE id = $1")
        .bind(doc_id)
        .fetch_one(&pool)
        .await
        .expect("fetch document title");
    assert_eq!(
        title, "Cross-org Confidential Contract",
        "cross-org update must not mutate the document"
    );
}

// ---------------------------------------------------------------------------
// Test 4: cross-org delete is blocked and does not remove the row
// ---------------------------------------------------------------------------

/// A cross-org `DELETE` must not remove another org's document.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_document_cross_org_does_not_delete(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user_a = seed_user(&pool, "del-owner@legal-idor.test").await;
    let user_b = seed_user(&pool, "del-attacker@legal-idor.test").await;
    let org_a = seed_org(&pool, "d-a").await;
    let org_b = seed_org(&pool, "d-b").await;
    let doc_id = seed_document(&pool, org_a, user_a).await;

    seed_membership(&pool, org_b, user_b).await;
    let token = access_token(user_b, Some(org_b));
    let req = app
        .delete(&format!("/api/v1/legal/documents/{doc_id}"))
        .bearer(&token)
        .header("X-Tenant-ID", &org_b.to_string())
        .build();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "cross-org document delete must be 404, got {} (body: {})",
        resp.status,
        resp.text()
    );

    // The document must still exist.
    let still_there: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM legal_documents WHERE id = $1")
        .bind(doc_id)
        .fetch_one(&pool)
        .await
        .expect("count document");
    assert_eq!(
        still_there, 1,
        "cross-org delete must not remove the document"
    );
}

// ---------------------------------------------------------------------------
// Test 5: a token with no tenant_id claim is rejected (org-context guard)
// ---------------------------------------------------------------------------

/// Without an organization context the request cannot be tenant-scoped, so it
/// must be refused rather than fall back to an unscoped read. Since the
/// PAP-80 conversion that refusal comes from the `RlsConnection` extractor:
/// 400 "Missing or invalid X-Tenant-ID header".
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_document_without_tenant_claim_is_forbidden(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "no-org@legal-idor.test").await;
    let org_a = seed_org(&pool, "n-a").await;
    let doc_id = seed_document(&pool, org_a, user).await;

    // Token carries no tenant_id claim.
    let token = access_token(user, None);
    let req = app
        .get(&format!("/api/v1/legal/documents/{doc_id}"))
        .bearer(&token)
        .build();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::BAD_REQUEST,
        "request without org context must be rejected, got {} (body: {})",
        resp.status,
        resp.text()
    );
}
