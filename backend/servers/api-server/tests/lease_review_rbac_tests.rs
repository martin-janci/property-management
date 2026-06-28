//! Real-JWT RBAC regression for tenant-application review (#806).
//!
//! `review_application` (`POST /api/v1/leases/applications/{id}/review`)
//! accepted any authenticated org member and let them approve/reject tenant
//! applications — a privilege escalation (a resident could approve their own
//! or others' applications). The fix gates it behind manager role.
//!
//! These tests use a real JWT (via `create_authenticated_user`) plus
//! `X-Tenant-ID`, with `organization_members` seeded so `RlsConnection`
//! resolves the caller's role from the DB. A **resident** must be rejected
//! with 403 before reaching the repository; a **manager** must pass the role
//! gate (proven by NOT getting a 403). See report_schedule_org_scope_jwt_tests.

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
    .bind(format!("LeaseRBAC {slug}"))
    .bind(format!("lease-rbac-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@lease-rbac.test", Uuid::new_v4()))
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

fn review_req(token: &str, org: Uuid, application_id: Uuid) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(format!(
            "/api/v1/leases/applications/{application_id}/review"
        ))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org.to_string())
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "status": "approved", "decision_notes": "ok" }).to_string(),
        ))
        .unwrap()
}

/// A resident must NOT be able to review applications. On dev this reached the
/// repository and applied the decision; the fix returns 403 before that.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_cannot_review_application(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "res").await;
    let token = member_token(&pool, &app, org, "resident").await;

    let resp = app.execute(review_req(&token, org, Uuid::new_v4())).await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "a resident reviewing an application must get 403, got {}",
        resp.status
    );
}

/// A manager passes the role gate (does not get 403). The random application id
/// won't be found, so the handler errors downstream — but crucially NOT with a
/// 403, proving managers are allowed through.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_passes_review_role_gate(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "mgr").await;
    let token = member_token(&pool, &app, org, "manager").await;

    let resp = app.execute(review_req(&token, org, Uuid::new_v4())).await;

    assert_ne!(
        resp.status,
        StatusCode::FORBIDDEN,
        "a manager must pass the review role gate, got 403"
    );
}
