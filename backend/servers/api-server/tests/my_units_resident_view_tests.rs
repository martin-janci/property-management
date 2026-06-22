//! Resident "My Unit" view tests (Epic 3, Story 3.6).
//!
//! `GET /api/v1/users/me/units` is the resident-facing, read-only view of the
//! unit(s) the caller is a resident of. These tests pin the two properties the
//! story requires:
//!
//!   1. A resident sees their own active units + association status.
//!   2. Privacy: other residents'/owners' PII is filtered server-side — the
//!      response must not leak a co-resident's name or email.
//!
//! The endpoint is resolved strictly by the authenticated `user_id`, so it
//! needs only a valid JWT (no `X-Tenant-ID`); buildings/units/residencies are
//! seeded directly.

#[allow(dead_code)]
mod common;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user, seed_org, TestApp, TestUser};

async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("user id")
}

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, name, street, city, postal_code, country, status)
        VALUES ($1, 'My Unit Bldg', 'Rezidentská 7', 'Bratislava', '81101', 'SK', 'active')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_unit(pool: &PgPool, building_id: Uuid, designation: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO units (building_id, designation, floor, unit_type)
        VALUES ($1, $2, 3, 'apartment')
        RETURNING id
        "#,
    )
    .bind(building_id)
    .bind(designation)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

async fn seed_resident(pool: &PgPool, unit_id: Uuid, user_id: Uuid, rtype: &str, primary: bool) {
    sqlx::query(
        "INSERT INTO unit_residents (unit_id, user_id, resident_type, is_primary) \
         VALUES ($1, $2, $3::resident_type, $4)",
    )
    .bind(unit_id)
    .bind(user_id)
    .bind(rtype)
    .bind(primary)
    .execute(pool)
    .await
    .expect("seed unit_resident");
}

fn authed_get(uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

/// A resident sees their own unit + association, and a co-resident's PII is
/// NOT leaked in the payload.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_sees_own_unit_without_coresident_pii(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "myunit").await;

    // The caller.
    let me = TestUser::new();
    let (token, _r) = create_authenticated_user(&app, &me).await;
    let my_id = user_id_for(&pool, &me.email).await;

    // A co-resident of the SAME unit, with a distinctive name/email we can
    // search the response for.
    let other = TestUser::with_email("nosy-neighbor-pii@my-unit.test");
    let (_t2, _r2) = create_authenticated_user(&app, &other).await;
    let other_id = user_id_for(&pool, &other.email).await;

    let building = seed_building(&pool, org).await;
    let unit = seed_unit(&pool, building, "3B").await;
    seed_resident(&pool, unit, my_id, "tenant", true).await;
    seed_resident(&pool, unit, other_id, "owner", false).await;

    let resp = app
        .execute(authed_get("/api/v1/users/me/units", &token))
        .await;
    resp.assert_status(StatusCode::OK);

    let body = resp.json_value();
    let arr = body.as_array().expect("array body");
    assert_eq!(arr.len(), 1, "caller should see exactly their one unit");

    let entry = &arr[0];
    assert_eq!(entry["unit"]["id"], serde_json::json!(unit.to_string()));
    assert_eq!(entry["unit"]["designation"], serde_json::json!("3B"));
    assert_eq!(
        entry["association"]["residentType"],
        serde_json::json!("tenant")
    );
    assert_eq!(entry["association"]["isActive"], serde_json::json!(true));
    assert_eq!(entry["building"]["city"], serde_json::json!("Bratislava"));

    // Privacy: the co-resident's email/name must never appear anywhere in the
    // serialized response.
    let raw = resp.text();
    assert!(
        !raw.contains("nosy-neighbor-pii@my-unit.test"),
        "co-resident email leaked into My Unit response: {raw}"
    );
    assert!(
        !raw.contains(&other_id.to_string()),
        "co-resident user id leaked into My Unit response: {raw}"
    );
}

/// The unit switcher needs every active association: a resident of two units
/// gets both back.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_sees_all_their_units(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "multi").await;

    let me = TestUser::new();
    let (token, _r) = create_authenticated_user(&app, &me).await;
    let my_id = user_id_for(&pool, &me.email).await;

    let building = seed_building(&pool, org).await;
    let unit_a = seed_unit(&pool, building, "1A").await;
    let unit_b = seed_unit(&pool, building, "2C").await;
    seed_resident(&pool, unit_a, my_id, "owner", true).await;
    seed_resident(&pool, unit_b, my_id, "tenant", false).await;

    let resp = app
        .execute(authed_get("/api/v1/users/me/units", &token))
        .await;
    resp.assert_status(StatusCode::OK);
    let arr = resp.json_value();
    assert_eq!(arr.as_array().expect("array").len(), 2);
}

/// A user with no residencies gets an empty list (not an error), and never
/// sees units they aren't a resident of.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn non_resident_sees_no_units(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "none").await;

    // Seed a unit with a *different* resident.
    let resident = TestUser::new();
    let (_t, _r) = create_authenticated_user(&app, &resident).await;
    let resident_id = user_id_for(&pool, &resident.email).await;
    let building = seed_building(&pool, org).await;
    let unit = seed_unit(&pool, building, "9Z").await;
    seed_resident(&pool, unit, resident_id, "owner", true).await;

    // The caller is a member of nothing.
    let outsider = TestUser::new();
    let (token, _r) = create_authenticated_user(&app, &outsider).await;

    let resp = app
        .execute(authed_get("/api/v1/users/me/units", &token))
        .await;
    resp.assert_status(StatusCode::OK);
    assert_eq!(resp.json_value().as_array().expect("array").len(), 0);
}

/// No JWT → 401.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn unauthenticated_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/users/me/units")
        .body(Body::empty())
        .unwrap();
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
}
