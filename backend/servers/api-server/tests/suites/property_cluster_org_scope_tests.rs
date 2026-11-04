//! Round-4 property-cluster authorization regressions.
//!
//! Covers the org-scope and role gates added to the building, unit, owner,
//! facility, booking, resident, person-month, meter, energy, registry,
//! certification, predictive-maintenance, AI-equipment and IoT handlers.
//!
//! RLS on most of these tables is not FORCE'd and the tests connect as a
//! superuser, so every assertion here exercises the explicit handler checks:
//!   * a by-id lookup of another org's resource returns 404 (nested ids must
//!     belong to their parent),
//!   * a non-manager member (resident/tenant) gets 403 on manager mutations,
//!   * the owning org's manager still succeeds.

#![allow(dead_code)]

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, TestApp, TestConfig, TestResponse};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO organizations (name, slug, contact_email, status)
           VALUES ($1, $2, $3, 'active') RETURNING id"#,
    )
    .bind(format!("R4 Org {slug}"))
    .bind(format!("r4-org-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@r4.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, slug: &str) -> (Uuid, String) {
    let email = format!("{slug}-{}@r4.test", Uuid::new_v4());
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'R4 User', 'active', NOW()) RETURNING id"#,
    )
    .bind(&email)
    .fetch_one(pool)
    .await
    .expect("seed user");
    (id, email)
}

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, name, street, city, postal_code, country)
           VALUES ($1, 'R4 Building', 'Main 1', 'Bratislava', '81101', 'SK') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_unit(pool: &PgPool, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation, floor, size_sqm)
           VALUES ($1, $2, 1, 50.0) RETURNING id"#,
    )
    .bind(building_id)
    .bind(format!("U-{}", &Uuid::new_v4().to_string()[..8]))
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

fn mint_token(user_id: Uuid, email: &str) -> String {
    use api_server::services::JwtService;
    let config = TestConfig::default();
    let jwt = JwtService::new(&config.jwt_secret).expect("jwt service");
    jwt.generate_access_token(user_id, email, "R4 User", None, None)
        .expect("mint access token")
}

/// A caller: token plus the org they act in.
struct Actor {
    user_id: Uuid,
    token: String,
    org: Uuid,
}

async fn actor(pool: &PgPool, org: Uuid, role: &str, slug: &str) -> Actor {
    let (user_id, email) = seed_user(pool, slug).await;
    seed_membership(pool, org, user_id, role).await;
    Actor {
        user_id,
        token: mint_token(user_id, &email),
        org,
    }
}

/// Two orgs, each with a building and unit; a manager in both and a resident
/// in org A.
struct World {
    org_a: Uuid,
    org_b: Uuid,
    bld_a: Uuid,
    bld_b: Uuid,
    unit_a: Uuid,
    unit_b: Uuid,
    mgr_a: Actor,
    res_a: Actor,
    mgr_b: Actor,
}

async fn world(pool: &PgPool) -> World {
    let org_a = seed_org(pool, "a").await;
    let org_b = seed_org(pool, "b").await;
    let bld_a = seed_building(pool, org_a).await;
    let bld_b = seed_building(pool, org_b).await;
    let unit_a = seed_unit(pool, bld_a).await;
    let unit_b = seed_unit(pool, bld_b).await;
    World {
        org_a,
        org_b,
        bld_a,
        bld_b,
        unit_a,
        unit_b,
        mgr_a: actor(pool, org_a, "org_admin", "mgr-a").await,
        res_a: actor(pool, org_a, "resident", "res-a").await,
        mgr_b: actor(pool, org_b, "org_admin", "mgr-b").await,
    }
}

async fn call(
    app: &TestApp,
    method: &str,
    uri: &str,
    who: &Actor,
    body: Option<serde_json::Value>,
) -> TestResponse {
    let rb = match method {
        "GET" => app.get(uri),
        "POST" => app.post(uri),
        "PUT" => app.put(uri),
        "DELETE" => app.delete(uri),
        other => panic!("unsupported method {other}"),
    }
    .bearer(&who.token)
    .tenant(who.org);
    let rb = match body {
        Some(b) => rb.json(b),
        None => rb,
    };
    app.execute(rb.build()).await
}

fn expect(resp: &TestResponse, status: StatusCode, ctx: &str) {
    assert_eq!(resp.status, status, "{ctx}: {}", resp.text());
}

fn id_of(resp: &TestResponse) -> Uuid {
    resp.json_value()["id"]
        .as_str()
        .expect("response id")
        .parse()
        .expect("uuid id")
}

// ---------------------------------------------------------------------------
// Buildings / units / owners
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn buildings_cross_org_ids_return_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let w = world(&pool).await;
    let (a, ua, mb) = (w.bld_a, w.unit_a, &w.mgr_b);

    for (m, uri, body) in [
        ("GET", format!("/api/v1/buildings/{a}"), None),
        (
            "PUT",
            format!("/api/v1/buildings/{a}"),
            Some(json!({ "name": "Hijacked" })),
        ),
        ("GET", format!("/api/v1/buildings/{a}/statistics"), None),
        ("DELETE", format!("/api/v1/buildings/{a}"), None),
        ("POST", format!("/api/v1/buildings/{a}/restore"), None),
        ("GET", format!("/api/v1/buildings/{a}/units"), None),
        (
            "POST",
            format!("/api/v1/buildings/{a}/units"),
            Some(json!({ "designation": "X1", "floor": 1 })),
        ),
        ("GET", format!("/api/v1/buildings/{a}/units/{ua}"), None),
        (
            "PUT",
            format!("/api/v1/buildings/{a}/units/{ua}"),
            Some(json!({ "description": "x" })),
        ),
        ("DELETE", format!("/api/v1/buildings/{a}/units/{ua}"), None),
        (
            "GET",
            format!("/api/v1/buildings/{a}/units/{ua}/owners"),
            None,
        ),
        (
            "POST",
            format!("/api/v1/buildings/{a}/units/{ua}/owners"),
            Some(json!({ "user_id": mb.user_id, "ownership_percentage": "10.00" })),
        ),
        (
            "DELETE",
            format!("/api/v1/buildings/{a}/units/{ua}/owners/{}", mb.user_id),
            None,
        ),
    ] {
        let resp = call(&app, m, &uri, mb, body).await;
        expect(&resp, StatusCode::NOT_FOUND, &format!("{m} {uri}"));
    }

    let name: Option<String> = sqlx::query_scalar("SELECT name FROM buildings WHERE id = $1")
        .bind(a)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(name.as_deref(), Some("R4 Building"));
    let owners: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM unit_owners WHERE unit_id = $1")
        .bind(ua)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(owners, 0, "no cross-org owner assignment");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn buildings_resident_cannot_mutate_manager_can(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let w = world(&pool).await;
    let (a, ua) = (w.bld_a, w.unit_a);

    for (m, uri, body) in [
        (
            "PUT",
            format!("/api/v1/buildings/{a}"),
            Some(json!({ "name": "Resident edit" })),
        ),
        ("POST", format!("/api/v1/buildings/{a}/restore"), None),
        (
            "POST",
            format!("/api/v1/buildings/{a}/units"),
            Some(json!({ "designation": "R1", "floor": 1 })),
        ),
        (
            "PUT",
            format!("/api/v1/buildings/{a}/units/{ua}"),
            Some(json!({ "description": "x" })),
        ),
        ("DELETE", format!("/api/v1/buildings/{a}/units/{ua}"), None),
        (
            "POST",
            format!("/api/v1/buildings/{a}/units/{ua}/owners"),
            Some(json!({ "user_id": w.res_a.user_id, "ownership_percentage": "10.00" })),
        ),
    ] {
        let resp = call(&app, m, &uri, &w.res_a, body).await;
        expect(&resp, StatusCode::FORBIDDEN, &format!("resident {m} {uri}"));
    }

    // Reads stay open to members.
    let resp = call(
        &app,
        "GET",
        &format!("/api/v1/buildings/{a}"),
        &w.res_a,
        None,
    )
    .await;
    expect(&resp, StatusCode::OK, "resident GET building");

    // The owning org's manager succeeds.
    let resp = call(
        &app,
        "PUT",
        &format!("/api/v1/buildings/{a}"),
        &w.mgr_a,
        Some(json!({ "name": "Manager edit" })),
    )
    .await;
    expect(&resp, StatusCode::OK, "manager PUT building");
    let resp = call(
        &app,
        "POST",
        &format!("/api/v1/buildings/{a}/units/{ua}/owners"),
        &w.mgr_a,
        Some(json!({ "user_id": w.res_a.user_id, "ownership_percentage": "10.00" })),
    )
    .await;
    expect(&resp, StatusCode::CREATED, "manager assign owner");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn bulk_import_into_other_org_is_forbidden(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let w = world(&pool).await;
    let resp = call(
        &app,
        "POST",
        "/api/v1/buildings/bulk",
        &w.mgr_b,
        Some(json!({
            "organization_id": w.org_a,
            "buildings": [{ "street": "Evil 1", "city": "Kosice", "postal_code": "04001" }]
        })),
    )
    .await;
    expect(&resp, StatusCode::FORBIDDEN, "bulk import into org A");
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM buildings WHERE organization_id = $1")
        .bind(w.org_a)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 1, "no building injected into org A");

    let resp = call(
        &app,
        "POST",
        "/api/v1/buildings/bulk",
        &w.mgr_b,
        Some(json!({
            "organization_id": w.org_b,
            "buildings": [{ "street": "Own 1", "city": "Kosice", "postal_code": "04001" }]
        })),
    )
    .await;
    expect(&resp, StatusCode::OK, "bulk import into own org");
}

// ---------------------------------------------------------------------------
// Facilities / bookings
// ---------------------------------------------------------------------------

async fn seed_facility(pool: &PgPool, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO facilities (building_id, name, facility_type, is_bookable)
           VALUES ($1, 'R4 Gym', 'gym', TRUE) RETURNING id"#,
    )
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed facility")
}

async fn seed_pending_booking(pool: &PgPool, facility_id: Uuid, user_id: Uuid) -> Uuid {
    let start = Utc::now() + Duration::days(2);
    let end = start + Duration::hours(1);
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO facility_bookings (facility_id, user_id, start_time, end_time, status)
           VALUES ($1, $2, $3, $4, 'pending') RETURNING id"#,
    )
    .bind(facility_id)
    .bind(user_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await
    .expect("seed booking")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn facilities_cross_org_ids_return_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let w = world(&pool).await;
    let fac_a = seed_facility(&pool, w.bld_a).await;
    let booking_a = seed_pending_booking(&pool, fac_a, w.res_a.user_id).await;
    let (bb, ba) = (w.bld_b, w.bld_a);
    let start = Utc::now() + Duration::days(3);
    let end = start + Duration::hours(1);

    for (m, uri, body) in [
        // Own building in the path, foreign facility id.
        (
            "PUT",
            format!("/api/v1/buildings/{bb}/facilities/{fac_a}"),
            Some(json!({ "name": "Hijacked" })),
        ),
        (
            "DELETE",
            format!("/api/v1/buildings/{bb}/facilities/{fac_a}"),
            None,
        ),
        (
            "GET",
            format!("/api/v1/buildings/{bb}/facilities/{fac_a}/bookings"),
            None,
        ),
        (
            "POST",
            format!("/api/v1/buildings/{bb}/facilities/{fac_a}/bookings"),
            Some(json!({ "start_time": start, "end_time": end })),
        ),
        // Foreign building in the path.
        (
            "PUT",
            format!("/api/v1/buildings/{ba}/facilities/{fac_a}"),
            Some(json!({ "name": "Hijacked" })),
        ),
        (
            "POST",
            format!("/api/v1/bookings/{booking_a}/approve"),
            None,
        ),
        (
            "POST",
            format!("/api/v1/bookings/{booking_a}/reject"),
            Some(json!({ "reason": "no" })),
        ),
    ] {
        let resp = call(&app, m, &uri, &w.mgr_b, body).await;
        expect(&resp, StatusCode::NOT_FOUND, &format!("{m} {uri}"));
    }

    let (name, status): (String, String) = sqlx::query_as(
        "SELECT f.name, b.status::text FROM facilities f JOIN facility_bookings b ON b.facility_id = f.id WHERE f.id = $1",
    )
    .bind(fac_a)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(name, "R4 Gym");
    assert_eq!(status, "pending");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn facilities_resident_cannot_manage_manager_can(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let w = world(&pool).await;
    let fac_a = seed_facility(&pool, w.bld_a).await;
    let booking_a = seed_pending_booking(&pool, fac_a, w.res_a.user_id).await;
    let a = w.bld_a;

    let create = json!({ "name": "Sauna", "facility_type": "sauna" });
    let resp = call(
        &app,
        "POST",
        &format!("/api/v1/buildings/{a}/facilities"),
        &w.res_a,
        Some(create.clone()),
    )
    .await;
    expect(&resp, StatusCode::FORBIDDEN, "resident create facility");
    let resp = call(
        &app,
        "DELETE",
        &format!("/api/v1/buildings/{a}/facilities/{fac_a}"),
        &w.res_a,
        None,
    )
    .await;
    expect(&resp, StatusCode::FORBIDDEN, "resident delete facility");
    let resp = call(
        &app,
        "POST",
        &format!("/api/v1/bookings/{booking_a}/approve"),
        &w.res_a,
        None,
    )
    .await;
    expect(&resp, StatusCode::FORBIDDEN, "resident approve booking");

    let resp = call(
        &app,
        "POST",
        &format!("/api/v1/buildings/{a}/facilities"),
        &w.mgr_a,
        Some(create),
    )
    .await;
    expect(&resp, StatusCode::CREATED, "manager create facility");
    let resp = call(
        &app,
        "POST",
        &format!("/api/v1/bookings/{booking_a}/approve"),
        &w.mgr_a,
        None,
    )
    .await;
    expect(&resp, StatusCode::OK, "manager approve booking");
}

// ---------------------------------------------------------------------------
// Unit residents
// ---------------------------------------------------------------------------

async fn seed_resident(pool: &PgPool, unit_id: Uuid, user_id: Uuid, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO unit_residents (unit_id, user_id, resident_type, start_date, created_by)
           VALUES ($1, $2, 'tenant'::resident_type, CURRENT_DATE, $3) RETURNING id"#,
    )
    .bind(unit_id)
    .bind(user_id)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed resident")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn residents_cross_org_ids_return_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let w = world(&pool).await;
    let rid = seed_resident(&pool, w.unit_a, w.res_a.user_id, w.mgr_a.user_id).await;
    let (bb, ua) = (w.bld_b, w.unit_a);
    let base = format!("/api/v1/buildings/{bb}/units/{ua}/residents");

    for (m, uri, body) in [
        ("GET", format!("{base}/{rid}"), None),
        (
            "PUT",
            format!("{base}/{rid}"),
            Some(json!({ "is_primary": true })),
        ),
        (
            "POST",
            format!("{base}/{rid}/end"),
            Some(json!({ "end_date": "2030-01-01" })),
        ),
        ("DELETE", format!("{base}/{rid}"), None),
    ] {
        let resp = call(&app, m, &uri, &w.mgr_b, body).await;
        expect(&resp, StatusCode::NOT_FOUND, &format!("{m} {uri}"));
    }
    let still: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM unit_residents WHERE id = $1 AND end_date IS NULL",
    )
    .bind(rid)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(still, 1, "resident untouched by cross-org caller");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn residents_resident_cannot_manage_manager_can(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let w = world(&pool).await;
    let base = format!("/api/v1/buildings/{}/units/{}/residents", w.bld_a, w.unit_a);
    let body = json!({
        "user_id": w.res_a.user_id,
        "resident_type": "tenant",
        "start_date": "2020-01-01"
    });
    let resp = call(&app, "POST", &base, &w.res_a, Some(body.clone())).await;
    expect(&resp, StatusCode::FORBIDDEN, "resident add resident");

    let resp = call(&app, "GET", &base, &w.res_a, None).await;
    expect(&resp, StatusCode::OK, "resident list residents");

    let resp = call(&app, "POST", &base, &w.mgr_a, Some(body)).await;
    expect(&resp, StatusCode::CREATED, "manager add resident");
    let rid = id_of(&resp);
    let resp = call(&app, "DELETE", &format!("{base}/{rid}"), &w.res_a, None).await;
    expect(&resp, StatusCode::FORBIDDEN, "resident remove resident");
}

// ---------------------------------------------------------------------------
// Person months
// ---------------------------------------------------------------------------

async fn seed_person_month(pool: &PgPool, unit_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO person_months (unit_id, year, month, count)
           VALUES ($1, 2026, 1, 2) RETURNING id"#,
    )
    .bind(unit_id)
    .fetch_one(pool)
    .await
    .expect("seed person month")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn person_months_cross_org_ids_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let w = world(&pool).await;
    let pm = seed_person_month(&pool, w.unit_a).await;
    let base = format!(
        "/api/v1/buildings/{}/units/{}/person-months",
        w.bld_b, w.unit_a
    );

    for (m, uri, body) in [
        ("GET", format!("{base}/{pm}"), None),
        ("PUT", format!("{base}/{pm}"), Some(json!({ "count": 9 }))),
        ("DELETE", format!("{base}/{pm}"), None),
    ] {
        let resp = call(&app, m, &uri, &w.mgr_b, body).await;
        expect(&resp, StatusCode::NOT_FOUND, &format!("{m} {uri}"));
    }

    // Bulk upsert on the caller's own building may not name a foreign unit.
    let resp = call(
        &app,
        "POST",
        &format!("/api/v1/buildings/{}/person-months/bulk", w.bld_b),
        &w.mgr_b,
        Some(json!({
            "year": 2026,
            "month": 1,
            "entries": [{ "unit_id": w.unit_a, "count": 7 }]
        })),
    )
    .await;
    expect(&resp, StatusCode::BAD_REQUEST, "bulk upsert foreign unit");

    let count: i32 = sqlx::query_scalar("SELECT count FROM person_months WHERE id = $1")
        .bind(pm)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 2, "person month untouched");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn person_months_resident_cannot_write_manager_can(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let w = world(&pool).await;
    let base = format!(
        "/api/v1/buildings/{}/units/{}/person-months",
        w.bld_a, w.unit_a
    );
    let body = json!({ "year": 2026, "month": 2, "count": 3, "source": "manual" });
    let resp = call(&app, "POST", &base, &w.res_a, Some(body.clone())).await;
    expect(&resp, StatusCode::FORBIDDEN, "resident upsert person month");
    let resp = call(&app, "GET", &format!("{base}?year=2026"), &w.res_a, None).await;
    expect(&resp, StatusCode::OK, "resident list person months");
    let resp = call(&app, "POST", &base, &w.mgr_a, Some(body)).await;
    expect(&resp, StatusCode::OK, "manager upsert person month");
}

// ---------------------------------------------------------------------------
// Meters (AuthUser routes: org comes from the body / resource)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn meters_foreign_building_and_roles(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let w = world(&pool).await;

    // Org B manager may not attach a bill / window to org A's building
    // (previously only the body org was checked, not the building's org).
    let bill = |org: Uuid, building: Uuid| {
        json!({
            "organization_id": org,
            "building_id": building,
            "meter_type": "electricity",
            "period_start": "2026-01-01",
            "period_end": "2026-01-31",
            "total_amount": "100.00",
            "currency": "EUR",
            "distribution_method": "equal"
        })
    };
    let resp = call(
        &app,
        "POST",
        "/api/v1/meters/utility-bills",
        &w.mgr_b,
        Some(bill(w.org_b, w.bld_a)),
    )
    .await;
    // Not a member of the building's org: rejected by the membership gate.
    expect(&resp, StatusCode::FORBIDDEN, "bill on foreign building");
    let resp = call(
        &app,
        "POST",
        "/api/v1/meters/submission-windows",
        &w.mgr_b,
        Some(json!({
            "organization_id": w.org_b,
            "building_id": w.bld_a,
            "name": "Injected",
            "billing_period_start": "2026-01-01",
            "billing_period_end": "2026-01-31",
            "submission_start": "2026-02-01",
            "submission_end": "2026-02-10"
        })),
    )
    .await;
    expect(&resp, StatusCode::FORBIDDEN, "window on foreign building");
    let n: i64 = sqlx::query_scalar(
        "SELECT (SELECT COUNT(*) FROM utility_bills WHERE building_id = $1) + (SELECT COUNT(*) FROM reading_submission_windows WHERE building_id = $1)",
    )
    .bind(w.bld_a)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(n, 0, "nothing attached to org A's building");

    // Residents may not register meters or create bills; managers may.
    let meter = json!({
        "organization_id": w.org_a,
        "building_id": w.bld_a,
        "meter_number": "R4-1",
        "meter_type": "electricity",
        "initial_reading": "0",
        "unit_of_measure": "kWh",
    });
    let resp = call(
        &app,
        "POST",
        "/api/v1/meters",
        &w.res_a,
        Some(meter.clone()),
    )
    .await;
    expect(&resp, StatusCode::FORBIDDEN, "resident register meter");
    let resp = call(
        &app,
        "POST",
        "/api/v1/meters/utility-bills",
        &w.res_a,
        Some(bill(w.org_a, w.bld_a)),
    )
    .await;
    expect(&resp, StatusCode::FORBIDDEN, "resident create bill");
    let resp = call(&app, "POST", "/api/v1/meters", &w.mgr_a, Some(meter)).await;
    expect(&resp, StatusCode::CREATED, "manager register meter");
}

// ---------------------------------------------------------------------------
// Energy (rejection paths only: the energy tables have no migration yet, so
// the gate must reject before any energy query runs)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn energy_mutations_require_manager(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let w = world(&pool).await;
    let ua = w.unit_a;

    let resp = call(
        &app,
        "POST",
        &format!("/api/v1/energy/units/{ua}/epc"),
        &w.res_a,
        Some(json!({ "organization_id": w.org_a, "unit_id": ua, "rating": "B" })),
    )
    .await;
    expect(&resp, StatusCode::FORBIDDEN, "resident create EPC");

    let listing = Uuid::new_v4();
    let resp = call(
        &app,
        "POST",
        &format!("/api/v1/energy/listings/{listing}/sustainability"),
        &w.res_a,
        Some(json!({ "organization_id": w.org_a, "listing_id": listing })),
    )
    .await;
    expect(
        &resp,
        StatusCode::FORBIDDEN,
        "resident create sustainability score",
    );

    // Org B manager naming org A in the body is not a member there.
    let resp = call(
        &app,
        "POST",
        &format!("/api/v1/energy/buildings/{}/emissions", w.bld_a),
        &w.mgr_b,
        Some(json!({
            "organization_id": w.org_a,
            "building_id": w.bld_a,
            "year": 2026,
            "month": 1,
            "source_type": "electricity",
            "co2_kg": "1.0"
        })),
    )
    .await;
    expect(&resp, StatusCode::FORBIDDEN, "foreign org emission");
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

async fn create_pet(app: &TestApp, who: &Actor, unit_id: Uuid) -> TestResponse {
    call(
        app,
        "POST",
        "/api/v1/registry/pets",
        who,
        Some(json!({
            "unit_id": unit_id,
            "pet_name": "Rex",
            "pet_type": "dog",
            "pet_size": "medium"
        })),
    )
    .await
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn registry_scope_and_roles(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let w = world(&pool).await;

    // Org B member may not register a pet against org A's unit.
    let resp = create_pet(&app, &w.mgr_b, w.unit_a).await;
    expect(&resp, StatusCode::NOT_FOUND, "pet on foreign unit");

    // A non-member cannot act in org A by sending its tenant header.
    let intruder = Actor {
        user_id: w.mgr_b.user_id,
        token: w.mgr_b.token.clone(),
        org: w.org_a,
    };
    let resp = call(&app, "GET", "/api/v1/registry/pets", &intruder, None).await;
    expect(&resp, StatusCode::FORBIDDEN, "non-member tenant header");

    // Manager-owned registration: resident can neither review nor modify it.
    let resp = create_pet(&app, &w.mgr_a, w.unit_a).await;
    expect(&resp, StatusCode::CREATED, "manager create pet");
    let pet: Uuid = resp.json_value()["registration"]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    for (m, uri, body) in [
        (
            "POST",
            format!("/api/v1/registry/pets/{pet}/review"),
            Some(json!({ "approve": true })),
        ),
        (
            "PUT",
            format!("/api/v1/registry/pets/{pet}"),
            Some(json!({ "pet_name": "Stolen" })),
        ),
        ("DELETE", format!("/api/v1/registry/pets/{pet}"), None),
        (
            "PUT",
            format!("/api/v1/registry/buildings/{}/rules", w.bld_a),
            Some(json!({ "pets_allowed": false })),
        ),
        (
            "POST",
            "/api/v1/registry/parking-spots".to_string(),
            Some(json!({ "building_id": w.bld_a, "spot_number": "P1" })),
        ),
    ] {
        let resp = call(&app, m, &uri, &w.res_a, body).await;
        expect(&resp, StatusCode::FORBIDDEN, &format!("resident {m} {uri}"));
    }

    // Residents still own their own registrations.
    let resp = create_pet(&app, &w.res_a, w.unit_a).await;
    expect(&resp, StatusCode::CREATED, "resident create own pet");
    let own: Uuid = resp.json_value()["registration"]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let resp = call(
        &app,
        "PUT",
        &format!("/api/v1/registry/pets/{own}"),
        &w.res_a,
        Some(json!({ "pet_name": "Rex II" })),
    )
    .await;
    expect(&resp, StatusCode::OK, "resident update own pet");

    // Manager reviews; parking spot on a foreign building is 404.
    let resp = call(
        &app,
        "POST",
        &format!("/api/v1/registry/pets/{own}/review"),
        &w.mgr_a,
        Some(json!({ "approve": true })),
    )
    .await;
    expect(&resp, StatusCode::OK, "manager review pet");
    let resp = call(
        &app,
        "POST",
        "/api/v1/registry/parking-spots",
        &w.mgr_b,
        Some(json!({ "building_id": w.bld_a, "spot_number": "P9" })),
    )
    .await;
    expect(
        &resp,
        StatusCode::NOT_FOUND,
        "parking spot on foreign building",
    );
}

// ---------------------------------------------------------------------------
// Building certifications
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn certifications_roles_and_parent_scope(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let w = world(&pool).await;
    let base = "/api/v1/building-certifications";
    let cert_body = |b: Uuid| json!({ "building_id": b, "program": "Leed", "level": "Gold" });

    let resp = call(&app, "POST", base, &w.res_a, Some(cert_body(w.bld_a))).await;
    expect(
        &resp,
        StatusCode::FORBIDDEN,
        "resident create certification",
    );

    let resp = call(&app, "POST", base, &w.mgr_a, Some(cert_body(w.bld_a))).await;
    expect(&resp, StatusCode::CREATED, "manager create certification");
    let cert_a = id_of(&resp);

    let credit = json!({
        "certification_id": cert_a,
        "category": "EnergyEfficiency",
        "credit_name": "Optimize energy",
        "points_possible": 10
    });
    // Org B manager may not hang a child row off org A's certification.
    let resp = call(
        &app,
        "POST",
        &format!("{base}/{cert_a}/credits"),
        &w.mgr_b,
        Some(credit.clone()),
    )
    .await;
    expect(
        &resp,
        StatusCode::NOT_FOUND,
        "credit on foreign certification",
    );
    let resp = call(
        &app,
        "POST",
        &format!("{base}/{cert_a}/credits"),
        &w.res_a,
        Some(credit.clone()),
    )
    .await;
    expect(&resp, StatusCode::FORBIDDEN, "resident create credit");
    let resp = call(
        &app,
        "POST",
        &format!("{base}/{cert_a}/credits"),
        &w.mgr_a,
        Some(credit),
    )
    .await;
    expect(&resp, StatusCode::CREATED, "manager create credit");
}

// ---------------------------------------------------------------------------
// Predictive maintenance + AI equipment
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn predictive_maintenance_roles_and_parent_scope(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let w = world(&pool).await;
    let uri = "/api/v1/predictive-maintenance/equipment";
    let body = |b: Uuid| {
        json!({
            "building_id": b,
            "name": "Boiler",
            "equipment_type": "hvac",
            "category": "heating"
        })
    };
    let resp = call(&app, "POST", uri, &w.res_a, Some(body(w.bld_a))).await;
    expect(&resp, StatusCode::FORBIDDEN, "resident create equipment");
    let resp = call(&app, "POST", uri, &w.mgr_a, Some(body(w.bld_b))).await;
    expect(
        &resp,
        StatusCode::NOT_FOUND,
        "equipment on foreign building",
    );
    let resp = call(&app, "POST", uri, &w.mgr_a, Some(body(w.bld_a))).await;
    expect(&resp, StatusCode::CREATED, "manager create equipment");
    let eq = id_of(&resp);
    let resp = call(&app, "DELETE", &format!("{uri}/{eq}"), &w.res_a, None).await;
    expect(&resp, StatusCode::FORBIDDEN, "resident delete equipment");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ai_equipment_mutations_require_manager(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let w = world(&pool).await;
    let body = json!({ "building_id": w.bld_a, "name": "Lift", "category": "elevator" });
    let resp = call(
        &app,
        "POST",
        "/api/v1/ai/equipment",
        &w.res_a,
        Some(body.clone()),
    )
    .await;
    expect(&resp, StatusCode::FORBIDDEN, "resident create AI equipment");
    let resp = call(&app, "POST", "/api/v1/ai/equipment", &w.mgr_a, Some(body)).await;
    expect(&resp, StatusCode::CREATED, "manager create AI equipment");
}

// ---------------------------------------------------------------------------
// IoT
// ---------------------------------------------------------------------------

async fn seed_sensor(pool: &PgPool, org: Uuid, building: Uuid, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO sensors (organization_id, building_id, name, sensor_type, created_by)
           VALUES ($1, $2, 'R4 Temp', 'temperature', $3) RETURNING id"#,
    )
    .bind(org)
    .bind(building)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed sensor")
}

async fn seed_threshold(pool: &PgPool, sensor: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO sensor_thresholds (sensor_id, comparison, warning_value)
           VALUES ($1, 'gt', 25) RETURNING id"#,
    )
    .bind(sensor)
    .fetch_one(pool)
    .await
    .expect("seed threshold")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn iot_cross_org_ids_return_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let w = world(&pool).await;
    let s = seed_sensor(&pool, w.org_a, w.bld_a, w.mgr_a.user_id).await;
    let t = seed_threshold(&pool, s).await;
    let base = "/api/v1/iot/sensors";

    for (m, uri, body) in [
        ("GET", format!("{base}/{s}"), None),
        (
            "PUT",
            format!("{base}/{s}"),
            Some(json!({ "name": "Hijacked" })),
        ),
        ("DELETE", format!("{base}/{s}"), None),
        ("GET", format!("{base}/{s}/thresholds"), None),
        (
            "POST",
            format!("{base}/{s}/thresholds"),
            Some(json!({ "sensor_id": s, "comparison": "lt", "metric": "x" })),
        ),
        (
            "PUT",
            format!("{base}/thresholds/{t}"),
            Some(json!({ "warning_value": 99.0 })),
        ),
        ("DELETE", format!("{base}/thresholds/{t}"), None),
        (
            "POST",
            format!("{base}/{s}/readings"),
            Some(json!({ "sensor_id": s, "value": 1.0, "unit": "C" })),
        ),
        ("GET", format!("{base}/{s}/correlations"), None),
    ] {
        let resp = call(&app, m, &uri, &w.mgr_b, body).await;
        expect(&resp, StatusCode::NOT_FOUND, &format!("{m} {uri}"));
    }

    let (name, warn): (String, Option<f64>) = sqlx::query_as(
        "SELECT s.name, t.warning_value FROM sensors s JOIN sensor_thresholds t ON t.sensor_id = s.id WHERE s.id = $1",
    )
    .bind(s)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(name, "R4 Temp");
    assert_eq!(warn, Some(25.0));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn iot_mutations_require_manager(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let w = world(&pool).await;
    let s = seed_sensor(&pool, w.org_a, w.bld_a, w.mgr_a.user_id).await;
    let base = "/api/v1/iot/sensors";

    let create = json!({
        "organization_id": w.org_a,
        "building_id": w.bld_a,
        "name": "Hall CO2",
        "sensor_type": "co2",
        "created_by": w.res_a.user_id
    });
    for (m, uri, body) in [
        ("POST", base.to_string(), Some(create.clone())),
        ("PUT", format!("{base}/{s}"), Some(json!({ "name": "x" }))),
        ("DELETE", format!("{base}/{s}"), None),
        (
            "POST",
            format!("{base}/{s}/readings"),
            Some(json!({ "sensor_id": s, "value": 1.0, "unit": "C" })),
        ),
        (
            "POST",
            format!("{base}/{s}/thresholds"),
            Some(json!({ "sensor_id": s, "comparison": "gt" })),
        ),
    ] {
        let resp = call(&app, m, &uri, &w.res_a, body).await;
        expect(&resp, StatusCode::FORBIDDEN, &format!("resident {m} {uri}"));
    }

    let resp = call(&app, "GET", &format!("{base}/{s}"), &w.res_a, None).await;
    expect(&resp, StatusCode::OK, "resident read sensor");
    let resp = call(
        &app,
        "PUT",
        &format!("{base}/{s}"),
        &w.mgr_a,
        Some(json!({ "name": "Renamed" })),
    )
    .await;
    expect(&resp, StatusCode::OK, "manager update sensor");
}
