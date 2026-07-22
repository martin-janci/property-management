//! Auth-guard tests for the dispute evidence-upload endpoint (gap-80-1,
//! Epic 77 / Story 77.1).
//!
//! The `POST /api/v1/disputes/{id}/evidence` handler takes an `AuthUser`
//! extractor, so an unauthenticated request must be rejected *before* any row
//! is written to `dispute_evidence`. These tests pin that contract at the HTTP
//! surface and assert the side-effect-free outcome (no evidence row created).
//!
//! TestApp wiring caveat (same as `dispute_cross_org_idor_tests.rs`): `TestApp`
//! mounts the router without `host_tenant_middleware`, so any request lacking a
//! fully-provisioned Bearer JWT is rejected with 4xx before the handler body
//! runs. The security contract — no evidence persisted without auth — still
//! holds and is what we assert.

#![allow(dead_code)]

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::TestApp;

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("DisputeEvidence Org {slug}"))
    .bind(format!("dispute-evidence-{slug}"))
    .bind(format!("{slug}@dispute-evidence.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'DisputeEvidence User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_dispute(pool: &PgPool, org_id: Uuid, filed_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO disputes (
            organization_id, reference_number, category,
            title, description, status, priority, filed_by
        )
        VALUES ($1, $2, 'noise', 'Evidence dispute', 'Has evidence',
                'under_review', 'medium', $3)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(format!("DISP-EV-{}", Uuid::new_v4()))
    .bind(filed_by)
    .fetch_one(pool)
    .await
    .expect("seed dispute")
}

async fn evidence_count(pool: &PgPool, dispute_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM dispute_evidence WHERE dispute_id = $1")
        .bind(dispute_id)
        .fetch_one(pool)
        .await
        .expect("count evidence")
}

fn assert_rejected(status: StatusCode, ctx: &str) {
    let code = status.as_u16();
    assert!(
        (400..500).contains(&code),
        "{ctx}: expected 4xx rejection, got {status}"
    );
}

fn evidence_body() -> serde_json::Value {
    json!({
        "data": {
            "dispute_id": Uuid::nil(),
            "uploaded_by": Uuid::nil(),
            "filename": "evil.pdf",
            "original_filename": "evil.pdf",
            "content_type": "application/pdf",
            "size_bytes": 10,
            "storage_url": "s3://evidence/evil.pdf",
            "description": "unauthenticated upload attempt"
        }
    })
}

/// An unauthenticated POST to the evidence-upload endpoint is rejected and no
/// evidence row is persisted.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_evidence_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user_id = seed_user(&pool, "no-auth@dispute-evidence.test").await;
    let org_id = seed_org(&pool, "no-auth").await;
    let dispute_id = seed_dispute(&pool, org_id, user_id).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/disputes/{dispute_id}/evidence"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(evidence_body().to_string()))
        .unwrap();
    let resp = app.execute(req).await;

    assert_rejected(resp.status, "POST /evidence without auth");
    assert_eq!(
        evidence_count(&pool, dispute_id).await,
        0,
        "no evidence row may be created without authentication"
    );
}

/// A POST carrying a malformed/forged bearer token is rejected and no evidence
/// row is persisted (the `AuthUser` extractor validates the JWT signature).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_evidence_with_bogus_bearer_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user_id = seed_user(&pool, "bogus@dispute-evidence.test").await;
    let org_id = seed_org(&pool, "bogus").await;
    let dispute_id = seed_dispute(&pool, org_id, user_id).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/disputes/{dispute_id}/evidence"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer not-a-real-jwt")
        .body(Body::from(evidence_body().to_string()))
        .unwrap();
    let resp = app.execute(req).await;

    assert_rejected(resp.status, "POST /evidence with bogus bearer");
    assert_eq!(
        evidence_count(&pool, dispute_id).await,
        0,
        "a forged bearer must not create an evidence row"
    );
}
