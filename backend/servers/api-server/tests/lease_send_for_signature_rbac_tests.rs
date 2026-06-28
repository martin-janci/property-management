//! Real-JWT RBAC regression for lease e-signing (#977).
//!
//! `send_lease_for_signature`
//! (`POST /api/v1/leases/{id}/send-for-signature`) wires leases into the
//! hardened signature-request subsystem (Option A). Sending a lease out for
//! signature is a manager action, gated with the same role check as
//! `review_application`: a **resident** must be rejected with 403 before
//! reaching the repository; a **manager** must pass the role gate (proven by
//! NOT getting a 403 — a random lease/document id then fails downstream, but
//! crucially not with 403).
//!
//! Mirrors `lease_review_rbac_tests`.

#![allow(dead_code)]

#[allow(dead_code)]
mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user, seed_membership, TestApp, TestUser};

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO organizations (name, slug, contact_email, status)
           VALUES ($1, $2, $3, 'active') RETURNING id"#,
    )
    .bind(format!("LeaseSign {slug}"))
    .bind(format!("lease-sign-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@lease-sign.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("user id")
}

/// Authenticate a user and make them a member of `org` with `role`.
/// Returns the bearer token.
async fn member_token(pool: &PgPool, app: &TestApp, org: Uuid, role: &str) -> String {
    let user = TestUser::new();
    let (token, _refresh) = create_authenticated_user(app, &user).await;
    let uid = user_id_for(pool, &user.email).await;
    seed_membership(pool, org, uid, role).await;
    token
}

fn send_req(token: &str, org: Uuid, lease_id: Uuid) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/leases/{lease_id}/send-for-signature"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org.to_string())
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "document_id": Uuid::new_v4() }).to_string(),
        ))
        .unwrap()
}

/// A resident must NOT be able to send a lease for signature.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_cannot_send_lease_for_signature(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "res").await;
    let token = member_token(&pool, &app, org, "resident").await;

    let resp = app.execute(send_req(&token, org, Uuid::new_v4())).await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "a resident sending a lease for signature must get 403, got {}",
        resp.status
    );
}

/// A manager passes the role gate (does not get 403). The random lease id won't
/// be found, so the handler returns 404 downstream — but crucially NOT 403,
/// proving managers are allowed through the gate.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_passes_send_for_signature_role_gate(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "mgr").await;
    let token = member_token(&pool, &app, org, "manager").await;

    let resp = app.execute(send_req(&token, org, Uuid::new_v4())).await;

    assert_ne!(
        resp.status,
        StatusCode::FORBIDDEN,
        "a manager must pass the send-for-signature role gate, got 403"
    );
}
