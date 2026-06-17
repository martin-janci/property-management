//! Real-JWT RBAC regression for building mutations (#799).
//!
//! `create_building`, `archive_building` and `bulk_import_buildings` relied
//! only on RLS tenant isolation and never checked the caller's *role*, so any
//! org member (e.g. a resident) could create, archive or bulk-import
//! buildings. The fix gates each behind manager role.
//!
//! Uses a real JWT + `X-Tenant-ID` with `organization_members` seeded so
//! `RlsConnection` resolves the role from the DB (see
//! report_schedule_org_scope_jwt_tests.rs). A resident must get 403 before the
//! mutation; the role gate runs after the org-membership check.

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
    .bind(format!("BldgRBAC {slug}"))
    .bind(format!("bldg-rbac-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@bldg-rbac.test", Uuid::new_v4()))
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

/// Authenticate a resident member of a fresh org. Returns (token, org_id).
async fn resident(pool: &PgPool, app: &TestApp, slug: &str) -> (String, Uuid) {
    let org = seed_org(pool, slug).await;
    let user = TestUser::new();
    let (token, _r) = create_authenticated_user(app, &user).await;
    let uid = user_id_for(pool, &user.email).await;
    seed_membership(pool, org, uid, "resident").await;
    (token, org)
}

/// Authenticate a manager member of a fresh org. Returns (token, org_id).
async fn manager(pool: &PgPool, app: &TestApp, slug: &str) -> (String, Uuid) {
    let org = seed_org(pool, slug).await;
    let user = TestUser::new();
    let (token, _r) = create_authenticated_user(app, &user).await;
    let uid = user_id_for(pool, &user.email).await;
    seed_membership(pool, org, uid, "manager").await;
    (token, org)
}

fn authed(
    method: Method,
    uri: &str,
    org: Uuid,
    token: &str,
    body: serde_json::Value,
) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org.to_string())
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_cannot_create_building(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, org) = resident(&pool, &app, "create").await;

    let body = json!({
        "organization_id": org,
        "street": "Main 1", "city": "Bratislava", "postal_code": "81101",
        "country": "SK", "total_floors": 1, "total_entrances": 1, "amenities": []
    });
    let resp = app
        .execute(authed(Method::POST, "/api/v1/buildings", org, &token, body))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "create as resident must be 403, got {}",
        resp.status
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_cannot_archive_building(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, org) = resident(&pool, &app, "archive").await;

    let resp = app
        .execute(authed(
            Method::DELETE,
            &format!("/api/v1/buildings/{}", Uuid::new_v4()),
            org,
            &token,
            json!({}),
        ))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "archive as resident must be 403, got {}",
        resp.status
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_cannot_bulk_import_buildings(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, org) = resident(&pool, &app, "bulk").await;

    // Empty list deserializes fine and the role gate runs before the
    // size/empty validation, so this is enough to reach (and assert) the gate.
    let body = json!({ "organization_id": org, "buildings": [] });
    let resp = app
        .execute(authed(
            Method::POST,
            "/api/v1/buildings/bulk",
            org,
            &token,
            body,
        ))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "bulk import as resident must be 403, got {}",
        resp.status
    );
}

/// Round-10 finding (#789): `list_buildings` trusted the client-supplied
/// `organization_id` query param. A member authenticated against org A who
/// asks for `?organization_id=<orgB>` must be rejected with 403 (matching
/// `create_building`) rather than receiving a silently-empty 200 page.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_rejects_org_mismatch(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, org_a) = manager(&pool, &app, "list-self").await;
    // A second org the caller is NOT scoped to for this request.
    let org_b = seed_org(&pool, "list-other").await;

    let resp = app
        .execute(authed(
            Method::GET,
            &format!("/api/v1/buildings?organization_id={org_b}"),
            org_a,
            &token,
            json!({}),
        ))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "list with mismatched organization_id must be 403, got {}",
        resp.status
    );
}

/// Positive control: the same manager listing their OWN org succeeds, so the
/// new guard does not over-reject the happy path.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_allows_own_org(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, org) = manager(&pool, &app, "list-ok").await;

    let resp = app
        .execute(authed(
            Method::GET,
            &format!("/api/v1/buildings?organization_id={org}"),
            org,
            &token,
            json!({}),
        ))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list with matching organization_id must be 200, got {}",
        resp.status
    );
}
