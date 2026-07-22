//! Behaviour-asserting integration tests for rentals endpoints (BIT-268 batch 2).
//!
//! Covers 32 endpoints from `docs/endpoint-checklist/groups/leasing.md`:
//! - Connections (statistics, sync-status, list, create, update, delete, unit-connections)
//! - Bookings (list, create, get, update, update-status, guests)
//! - Calendar (get, availability, create-block, delete-block)
//! - Guests (create, get, update, delete, register)
//! - Check-in reminders
//! - Reports (list, preview, create, get, submit)
//! - iCal feeds (create, update, delete, list-unit)

#![allow(dead_code)]

use crate::common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Seed helpers
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, tag: &str) -> Uuid {
    let uid = Uuid::new_v4();
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO organizations (name, slug, contact_email, status) \
         VALUES ($1,$2,$3,'active') RETURNING id",
    )
    .bind(format!("Rental Test {tag}"))
    .bind(format!("rental-test-{tag}-{uid}"))
    .bind(format!("{tag}-{uid}@rental-test.internal"))
    .fetch_one(pool)
    .await
    .expect("seed_org")
}

async fn seed_user(pool: &PgPool, tag: &str) -> Uuid {
    let uid = Uuid::new_v4();
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind) \
         VALUES ($1,'hash','Rental Tester','active',NOW(),'staff') RETURNING id",
    )
    .bind(format!("{tag}-{uid}@rental-test.internal"))
    .fetch_one(pool)
    .await
    .expect("seed_user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO buildings (organization_id, name, street, city, postal_code, country, status) \
         VALUES ($1,$2,'Test St 1','Testville','00000','SK','active') RETURNING id",
    )
    .bind(org_id)
    .bind(format!("Building {tag}"))
    .fetch_one(pool)
    .await
    .expect("seed_building")
}

async fn seed_unit(pool: &PgPool, building_id: Uuid, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO units (building_id, designation, floor) VALUES ($1,$2,1) RETURNING id",
    )
    .bind(building_id)
    .bind(format!("U-{tag}"))
    .fetch_one(pool)
    .await
    .expect("seed_unit")
}

async fn seed_connection(pool: &PgPool, org_id: Uuid, unit_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO rental_platform_connections \
             (organization_id, unit_id, platform, is_active) \
         VALUES ($1,$2,'airbnb',true) RETURNING id",
    )
    .bind(org_id)
    .bind(unit_id)
    .fetch_one(pool)
    .await
    .expect("seed_connection")
}

async fn seed_booking(pool: &PgPool, org_id: Uuid, unit_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO rental_bookings \
             (organization_id, unit_id, platform, guest_name, guest_count, \
              check_in, check_out, status) \
         VALUES ($1,$2,'direct','Test Guest',2, \
                 CURRENT_DATE + 10, CURRENT_DATE + 15,'confirmed') RETURNING id",
    )
    .bind(org_id)
    .bind(unit_id)
    .fetch_one(pool)
    .await
    .expect("seed_booking")
}

async fn seed_guest(pool: &PgPool, org_id: Uuid, booking_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO rental_guests \
             (organization_id, booking_id, first_name, last_name) \
         VALUES ($1,$2,'John','Doe') RETURNING id",
    )
    .bind(org_id)
    .bind(booking_id)
    .fetch_one(pool)
    .await
    .expect("seed_guest")
}

async fn seed_calendar_block(pool: &PgPool, org_id: Uuid, unit_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO rental_calendar_blocks \
             (organization_id, unit_id, block_start, block_end, reason) \
         VALUES ($1,$2, CURRENT_DATE + 30, CURRENT_DATE + 35,'maintenance') RETURNING id",
    )
    .bind(org_id)
    .bind(unit_id)
    .fetch_one(pool)
    .await
    .expect("seed_calendar_block")
}

async fn seed_report(pool: &PgPool, org_id: Uuid, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO rental_guest_reports \
             (organization_id, building_id, report_type, period_start, period_end, \
              authority_code, status) \
         VALUES ($1,$2,'ubyport', CURRENT_DATE - 30, CURRENT_DATE - 1, \
                 'SK001','generated') RETURNING id",
    )
    .bind(org_id)
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed_report")
}

async fn seed_ical(pool: &PgPool, org_id: Uuid, unit_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO rental_ical_feeds \
             (organization_id, unit_id, feed_name, feed_token, is_active) \
         VALUES ($1,$2,'Test Feed', gen_random_uuid()::text, true) RETURNING id",
    )
    .bind(org_id)
    .bind(unit_id)
    .fetch_one(pool)
    .await
    .expect("seed_ical")
}

fn mint_manager_jwt(user_id: Uuid, org_id: Uuid) -> String {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::Serialize;
    #[derive(Serialize)]
    struct Claims {
        sub: String,
        email: String,
        name: String,
        exp: i64,
        iat: i64,
        jti: String,
        token_type: String,
        role: String,
        tenant_id: String,
    }
    let now = chrono::Utc::now().timestamp();
    encode(
        &Header::default(),
        &Claims {
            sub: user_id.to_string(),
            email: "rental-test@internal.example".into(),
            name: "Rental Tester".into(),
            exp: now + 900,
            iat: now,
            jti: Uuid::new_v4().to_string(),
            token_type: "access".into(),
            role: "manager".into(),
            tenant_id: org_id.to_string(),
        },
        &EncodingKey::from_secret(
            b"test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes",
        ),
    )
    .expect("mint_manager_jwt")
}

// ---------------------------------------------------------------------------
// Connections
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_rental_statistics_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "rstat").await;
    let user_id = seed_user(&pool, "rstat").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/rentals/statistics")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "statistics → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_rental_sync_status_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "rsync").await;
    let user_id = seed_user(&pool, "rsync").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/rentals/sync-status")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "sync-status → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_list_connections_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "lscon").await;
    let user_id = seed_user(&pool, "lscon").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/rentals/connections")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list connections → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_create_connection_returns_201(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "crcon").await;
    let user_id = seed_user(&pool, "crcon").await;
    let bld_id = seed_building(&pool, org_id, "crcon").await;
    let unit_id = seed_unit(&pool, bld_id, "crcon").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);

    let body = json!({
        "unit_id": unit_id,
        "platform": "airbnb"
    });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/rentals/connections")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create connection → 201; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_update_connection_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "upcon").await;
    let user_id = seed_user(&pool, "upcon").await;
    let bld_id = seed_building(&pool, org_id, "upcon").await;
    let unit_id = seed_unit(&pool, bld_id, "upcon").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let conn_id = seed_connection(&pool, org_id, unit_id).await;

    let body = json!({ "is_active": false });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/api/v1/rentals/connections/{conn_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update connection → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_delete_connection_returns_204(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "dlcon").await;
    let user_id = seed_user(&pool, "dlcon").await;
    let bld_id = seed_building(&pool, org_id, "dlcon").await;
    let unit_id = seed_unit(&pool, bld_id, "dlcon").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let conn_id = seed_connection(&pool, org_id, unit_id).await;

    let resp = app
        .execute(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/rentals/connections/{conn_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete connection → 204; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_get_unit_connections_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "uccon").await;
    let user_id = seed_user(&pool, "uccon").await;
    let bld_id = seed_building(&pool, org_id, "uccon").await;
    let unit_id = seed_unit(&pool, bld_id, "uccon").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v1/rentals/units/{unit_id}/connections"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "unit connections → 200; body: {}",
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// Bookings
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_list_bookings_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "lsbk").await;
    let user_id = seed_user(&pool, "lsbk").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/rentals/bookings")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list bookings → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_create_booking_returns_201(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "crbk").await;
    let user_id = seed_user(&pool, "crbk").await;
    let bld_id = seed_building(&pool, org_id, "crbk").await;
    let unit_id = seed_unit(&pool, bld_id, "crbk").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);

    let body = json!({
        "unit_id": unit_id,
        "guest_name": "Alice Guest",
        "check_in": "2026-09-01",
        "check_out": "2026-09-07"
    });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/rentals/bookings")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create booking → 201; body: {}",
        resp.text()
    );
    let json = resp.json_value();
    assert!(json.get("id").is_some(), "response must include id");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_get_booking_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "gtbk").await;
    let user_id = seed_user(&pool, "gtbk").await;
    let bld_id = seed_building(&pool, org_id, "gtbk").await;
    let unit_id = seed_unit(&pool, bld_id, "gtbk").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let bk_id = seed_booking(&pool, org_id, unit_id).await;

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v1/rentals/bookings/{bk_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get booking → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_update_booking_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "upbk").await;
    let user_id = seed_user(&pool, "upbk").await;
    let bld_id = seed_building(&pool, org_id, "upbk").await;
    let unit_id = seed_unit(&pool, bld_id, "upbk").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let bk_id = seed_booking(&pool, org_id, unit_id).await;

    let body = json!({ "internal_notes": "Updated notes" });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/api/v1/rentals/bookings/{bk_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update booking → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_update_booking_status_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "bkst").await;
    let user_id = seed_user(&pool, "bkst").await;
    let bld_id = seed_building(&pool, org_id, "bkst").await;
    let unit_id = seed_unit(&pool, bld_id, "bkst").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let bk_id = seed_booking(&pool, org_id, unit_id).await;

    let body = json!({ "status": "checked_in" });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/api/v1/rentals/bookings/{bk_id}/status"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update booking status → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_get_booking_with_guests_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "bkgs").await;
    let user_id = seed_user(&pool, "bkgs").await;
    let bld_id = seed_building(&pool, org_id, "bkgs").await;
    let unit_id = seed_unit(&pool, bld_id, "bkgs").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let bk_id = seed_booking(&pool, org_id, unit_id).await;

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v1/rentals/bookings/{bk_id}/guests"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "booking guests → 200; body: {}",
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// Calendar
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_get_calendar_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "gcal").await;
    let user_id = seed_user(&pool, "gcal").await;
    let bld_id = seed_building(&pool, org_id, "gcal").await;
    let unit_id = seed_unit(&pool, bld_id, "gcal").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/api/v1/rentals/calendar/{unit_id}?start_date=2026-09-01&end_date=2026-09-30"
                ))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get calendar → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_check_availability_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "avail").await;
    let user_id = seed_user(&pool, "avail").await;
    let bld_id = seed_building(&pool, org_id, "avail").await;
    let unit_id = seed_unit(&pool, bld_id, "avail").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/api/v1/rentals/calendar/{unit_id}/availability?start_date=2026-10-01&end_date=2026-10-31"
                ))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "check availability → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_create_calendar_block_returns_201(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "crblk").await;
    let user_id = seed_user(&pool, "crblk").await;
    let bld_id = seed_building(&pool, org_id, "crblk").await;
    let unit_id = seed_unit(&pool, bld_id, "crblk").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);

    let body = json!({
        "unit_id": unit_id,
        "block_start": "2026-11-01",
        "block_end": "2026-11-05",
        "reason": "Maintenance"
    });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/rentals/calendar/blocks")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create calendar block → 201; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_delete_calendar_block_returns_204(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "dlblk").await;
    let user_id = seed_user(&pool, "dlblk").await;
    let bld_id = seed_building(&pool, org_id, "dlblk").await;
    let unit_id = seed_unit(&pool, bld_id, "dlblk").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let block_id = seed_calendar_block(&pool, org_id, unit_id).await;

    let resp = app
        .execute(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/rentals/calendar/blocks/{block_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete calendar block → 204; body: {}",
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// Guests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_create_guest_returns_201(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "crgs").await;
    let user_id = seed_user(&pool, "crgs").await;
    let bld_id = seed_building(&pool, org_id, "crgs").await;
    let unit_id = seed_unit(&pool, bld_id, "crgs").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let bk_id = seed_booking(&pool, org_id, unit_id).await;

    let body = json!({
        "booking_id": bk_id,
        "first_name": "Jane",
        "last_name": "Doe"
    });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/rentals/guests")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create guest → 201; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_get_guest_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "gtgs").await;
    let user_id = seed_user(&pool, "gtgs").await;
    let bld_id = seed_building(&pool, org_id, "gtgs").await;
    let unit_id = seed_unit(&pool, bld_id, "gtgs").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let bk_id = seed_booking(&pool, org_id, unit_id).await;
    let guest_id = seed_guest(&pool, org_id, bk_id).await;

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v1/rentals/guests/{guest_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get guest → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_update_guest_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "upgs").await;
    let user_id = seed_user(&pool, "upgs").await;
    let bld_id = seed_building(&pool, org_id, "upgs").await;
    let unit_id = seed_unit(&pool, bld_id, "upgs").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let bk_id = seed_booking(&pool, org_id, unit_id).await;
    let guest_id = seed_guest(&pool, org_id, bk_id).await;

    let body = json!({ "nationality": "SK" });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/api/v1/rentals/guests/{guest_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update guest → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_delete_guest_returns_204(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "dlgs").await;
    let user_id = seed_user(&pool, "dlgs").await;
    let bld_id = seed_building(&pool, org_id, "dlgs").await;
    let unit_id = seed_unit(&pool, bld_id, "dlgs").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let bk_id = seed_booking(&pool, org_id, unit_id).await;
    let guest_id = seed_guest(&pool, org_id, bk_id).await;

    let resp = app
        .execute(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/rentals/guests/{guest_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete guest → 204; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_register_guest_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "rgst").await;
    let user_id = seed_user(&pool, "rgst").await;
    let bld_id = seed_building(&pool, org_id, "rgst").await;
    let unit_id = seed_unit(&pool, bld_id, "rgst").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let bk_id = seed_booking(&pool, org_id, unit_id).await;
    let guest_id = seed_guest(&pool, org_id, bk_id).await;

    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v1/rentals/guests/{guest_id}/register"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "register guest → 200; body: {}",
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// Check-in reminders
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_get_checkin_reminders_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "cirem").await;
    let user_id = seed_user(&pool, "cirem").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/rentals/checkin-reminders")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "checkin-reminders → 200; body: {}",
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// Reports
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_list_reports_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "lsrep").await;
    let user_id = seed_user(&pool, "lsrep").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/rentals/reports")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list reports → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_generate_report_preview_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "rprev").await;
    let user_id = seed_user(&pool, "rprev").await;
    let bld_id = seed_building(&pool, org_id, "rprev").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);

    let body = json!({
        "building_id": bld_id,
        "period_start": "2026-06-01",
        "period_end": "2026-06-30"
    });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/rentals/reports/preview")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "report preview → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_create_report_returns_201(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "crrep").await;
    let user_id = seed_user(&pool, "crrep").await;
    let bld_id = seed_building(&pool, org_id, "crrep").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);

    let body = json!({
        "building_id": bld_id,
        "report_type": "ubyport",
        "period_start": "2026-05-01",
        "period_end": "2026-05-31",
        "authority_code": "SK001"
    });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/rentals/reports")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create report → 201; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_get_report_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "gtrep").await;
    let user_id = seed_user(&pool, "gtrep").await;
    let bld_id = seed_building(&pool, org_id, "gtrep").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let rep_id = seed_report(&pool, org_id, bld_id).await;

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v1/rentals/reports/{rep_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get report → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_submit_report_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "sbrep").await;
    let user_id = seed_user(&pool, "sbrep").await;
    let bld_id = seed_building(&pool, org_id, "sbrep").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let rep_id = seed_report(&pool, org_id, bld_id).await;

    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v1/rentals/reports/{rep_id}/submit"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "submit report → 200; body: {}",
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// iCal feeds
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_create_ical_feed_returns_201(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "crif").await;
    let user_id = seed_user(&pool, "crif").await;
    let bld_id = seed_building(&pool, org_id, "crif").await;
    let unit_id = seed_unit(&pool, bld_id, "crif").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);

    let body = json!({
        "unit_id": unit_id,
        "feed_name": "Airbnb Calendar"
    });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/rentals/ical")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create ical feed → 201; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_update_ical_feed_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "upif").await;
    let user_id = seed_user(&pool, "upif").await;
    let bld_id = seed_building(&pool, org_id, "upif").await;
    let unit_id = seed_unit(&pool, bld_id, "upif").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let ical_id = seed_ical(&pool, org_id, unit_id).await;

    let body = json!({ "feed_name": "Updated Feed Name" });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/api/v1/rentals/ical/{ical_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update ical feed → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_delete_ical_feed_returns_204(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "dlif").await;
    let user_id = seed_user(&pool, "dlif").await;
    let bld_id = seed_building(&pool, org_id, "dlif").await;
    let unit_id = seed_unit(&pool, bld_id, "dlif").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let ical_id = seed_ical(&pool, org_id, unit_id).await;

    let resp = app
        .execute(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/rentals/ical/{ical_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete ical feed → 204; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_get_unit_ical_feeds_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "uif").await;
    let user_id = seed_user(&pool, "uif").await;
    let bld_id = seed_building(&pool, org_id, "uif").await;
    let unit_id = seed_unit(&pool, bld_id, "uif").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v1/rentals/units/{unit_id}/ical"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "unit ical feeds → 200; body: {}",
        resp.text()
    );
}
