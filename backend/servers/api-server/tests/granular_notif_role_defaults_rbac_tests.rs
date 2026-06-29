//! Real-JWT RBAC regression for org-wide role notification defaults (#808).
//!
//! `update_role_defaults` (PUT) and `delete_role_defaults` (DELETE) on
//! `/api/v1/users/me/notification-preferences/granular/roles/{role}` change
//! ORG-WIDE notification defaults but had no role check, so any authenticated
//! member could rewrite them. The fix gates both behind manager role
//! (`tenant.role.is_manager()`). `apply_role_defaults` is intentionally left
//! open — it only applies defaults to the *calling* user's own prefs.
//!
//! These endpoints use `TenantExtractor`, whose role derives from the bearer
//! token, so a freshly-registered/logged-in user is a non-manager and must be
//! rejected with 403.

#![allow(dead_code)]

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user, TestApp, TestUser};

const BASE: &str = "/api/v1/users/me/notification-preferences/granular/roles";

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO organizations (name, slug, contact_email, status)
           VALUES ($1, $2, $3, 'active') RETURNING id"#,
    )
    .bind(format!("NotifRBAC {slug}"))
    .bind(format!("notif-rbac-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@notif-rbac.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn non_manager_cannot_update_role_defaults(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "update").await;
    let (token, _r) = create_authenticated_user(&app, &TestUser::new()).await;

    let req = Request::builder()
        .method(Method::PUT)
        .uri(format!("{BASE}/manager"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org.to_string())
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "eventPreferences": {} }).to_string()))
        .unwrap();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "non-manager updating org role defaults must be 403, got {}",
        resp.status
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn non_manager_cannot_delete_role_defaults(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "delete").await;
    let (token, _r) = create_authenticated_user(&app, &TestUser::new()).await;

    let req = Request::builder()
        .method(Method::DELETE)
        .uri(format!("{BASE}/manager"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org.to_string())
        .body(Body::empty())
        .unwrap();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "non-manager deleting org role defaults must be 403, got {}",
        resp.status
    );
}
