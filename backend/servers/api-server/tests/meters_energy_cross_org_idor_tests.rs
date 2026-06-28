//! Regression tests for the cross-tenant IDOR / forged-org fixes on the meter
//! (`/api/v1/meters/*`, Epic 12 — issue #803) and energy/ESG
//! (`/api/v1/energy/*`, Epic 65 — issue #846) endpoints.
//!
//! Audit history: every meter and energy handler either discarded its
//! `AuthUser` (the `_auth` binding) or accepted a client-supplied
//! `organization_id` (JSON body / query param) without checking that the
//! caller is actually a member of that org, and read resources by UUID/path
//! with no ownership check. The combination let a foreign authenticated
//! caller:
//!   * register a meter against an attacker-chosen org,
//!   * read any meter / reading / consumption by UUID,
//!   * list another org's building/unit meters,
//!   * read another org's EPC / carbon / benchmark data by building/unit path.
//!
//! The fix derives or validates the org against the authenticated principal
//! via a `verify_org_access` membership gate (mirrors `routes/financial.rs`)
//! plus a `verify_building_access` building->org gate (mirrors
//! `routes/community.rs`). Cross-tenant requests are rejected (403 for
//! org-keyed reads/writes, 404 for by-ID/path probes); same-org access works.
//!
//! NOTE: Epic 65 (energy) has repository code but no schema migration for its
//! tables, so only the *rejection* path (which short-circuits in the tenant
//! gate before any energy query) is asserted for energy; the same-org success
//! path is proven on the meter endpoints whose tables do exist.

#![allow(dead_code)]

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_membership, TestApp, TestConfig};

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
    .bind(format!("MeterIDOR Org {slug}"))
    .bind(format!("meter-idor-org-{slug}"))
    .bind(format!("{slug}@meter-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'MeterIDOR User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code, country)
        VALUES ($1, $2, 'Bratislava', '81101', 'Slovakia') RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(format!("{slug} Street 1"))
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_meter(pool: &PgPool, org_id: Uuid, building_id: Uuid, number: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO meters (
            organization_id, building_id, meter_number, meter_type,
            initial_reading, current_reading, unit_of_measure, is_active
        )
        VALUES ($1, $2, $3, 'electricity', 0, 0, 'kWh', true)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .bind(number)
    .fetch_one(pool)
    .await
    .expect("seed meter")
}

/// Mint a real HS256 access token for `user_id`, signed with the same secret
/// the TestApp configures into `JWT_SECRET`.
fn mint_token(user_id: Uuid, email: &str) -> String {
    use api_server::services::JwtService;
    let config = TestConfig::default();
    let jwt = JwtService::new(&config.jwt_secret).expect("jwt service");
    jwt.generate_access_token(user_id, email, "MeterIDOR User", None, None)
        .expect("mint access token")
}

fn assert_rejected(status: StatusCode, ctx: &str) {
    let code = status.as_u16();
    assert!(
        (400..500).contains(&code),
        "{ctx}: cross-tenant/unauthenticated request must be rejected with 4xx, got {status}"
    );
}

// ---------------------------------------------------------------------------
// #803 — meters: unauthenticated get_meter is rejected
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_meter_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "noauth-a").await;
    let building_a = seed_building(&pool, org_a, "noauth-a").await;
    let meter_a = seed_meter(&pool, org_a, building_a, "NOAUTH-1").await;

    let uri = format!("/api/v1/meters/{meter_a}");
    let resp = app.execute(app.get(&uri).build()).await;

    assert_rejected(resp.status, "get_meter without bearer token");
}

// ---------------------------------------------------------------------------
// #803 — meters: cross-org get_meter by UUID is rejected (IDOR)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_meter_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "get-a").await;
    let org_b = seed_org(&pool, "get-b").await;
    let user_b = seed_user(&pool, "get-b@meter-idor.test").await;
    seed_membership(&pool, org_b, user_b, "org_admin").await;
    let building_a = seed_building(&pool, org_a, "get-a").await;
    let meter_a = seed_meter(&pool, org_a, building_a, "GET-1").await;

    let token_b = mint_token(user_b, "get-b@meter-idor.test");
    let uri = format!("/api/v1/meters/{meter_a}");
    let resp = app.execute(app.get(&uri).bearer(&token_b).build()).await;

    assert_rejected(resp.status, "get_meter cross-tenant");
    assert_ne!(
        resp.status,
        StatusCode::OK,
        "Org A meter must not be readable by Org B"
    );
}

// ---------------------------------------------------------------------------
// #803 — meters: cross-org list_meters (building path) is rejected
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_meters_for_other_orgs_building_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "lst-a").await;
    let org_b = seed_org(&pool, "lst-b").await;
    let user_b = seed_user(&pool, "lst-b@meter-idor.test").await;
    seed_membership(&pool, org_b, user_b, "org_admin").await;
    let building_a = seed_building(&pool, org_a, "lst-a").await;
    let _meter_a = seed_meter(&pool, org_a, building_a, "LST-1").await;

    let token_b = mint_token(user_b, "lst-b@meter-idor.test");
    let uri = format!("/api/v1/meters/buildings/{building_a}");
    let resp = app.execute(app.get(&uri).bearer(&token_b).build()).await;

    assert_rejected(resp.status, "list_meters cross-tenant building");
    assert_ne!(resp.status, StatusCode::OK);
}

// ---------------------------------------------------------------------------
// #803 — meters: register_meter with attacker-supplied org_id is rejected
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn register_meter_for_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "reg-a").await;
    let org_b = seed_org(&pool, "reg-b").await;
    let user_b = seed_user(&pool, "reg-b@meter-idor.test").await;
    seed_membership(&pool, org_b, user_b, "org_admin").await;
    let building_a = seed_building(&pool, org_a, "reg-a").await;

    let token_b = mint_token(user_b, "reg-b@meter-idor.test");
    let body = json!({
        "organization_id": org_a,
        "building_id": building_a,
        "meter_number": "ATTACKER-1",
        "meter_type": "electricity",
        "initial_reading": "0",
        "unit_of_measure": "kWh",
    });
    let resp = app
        .execute(
            app.post("/api/v1/meters")
                .bearer(&token_b)
                .json(body)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "Org B member must not register a meter against Org A: {}",
        resp.text()
    );

    // No meter row should have been created for Org A via the forged request.
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM meters WHERE meter_number = 'ATTACKER-1'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0, "no meter may be registered via cross-tenant POST");
}

// ---------------------------------------------------------------------------
// #803 — meters: legitimate same-org get_meter succeeds
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_meter_for_own_org_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "own-a").await;
    let user_a = seed_user(&pool, "own-a@meter-idor.test").await;
    seed_membership(&pool, org_a, user_a, "org_admin").await;
    let building_a = seed_building(&pool, org_a, "own-a").await;
    let meter_a = seed_meter(&pool, org_a, building_a, "OWN-1").await;

    let token_a = mint_token(user_a, "own-a@meter-idor.test");
    let uri = format!("/api/v1/meters/{meter_a}");
    let resp = app.execute(app.get(&uri).bearer(&token_a).build()).await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "Org A member must be able to read its own meter: {}",
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// #846 — energy: cross-org get_unit_epc (unit path) is rejected (IDOR)
//
// The tenant gate (verify_unit_access) runs before any energy-table query, so
// a foreign caller is rejected regardless of whether the EPC row exists.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_unit_epc_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "epc-a").await;
    let org_b = seed_org(&pool, "epc-b").await;
    let user_b = seed_user(&pool, "epc-b@meter-idor.test").await;
    seed_membership(&pool, org_b, user_b, "org_admin").await;
    let building_a = seed_building(&pool, org_a, "epc-a").await;
    let unit_a = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO units (
            building_id, designation, floor, unit_type,
            size_sqm, rooms, ownership_share, occupancy_status, status, settings
        )
        VALUES ($1, 'EPC-1A', 1, 'apartment', 50.0, 2, 100.00, 'unknown', 'active', '{}'::jsonb)
        RETURNING id
        "#,
    )
    .bind(building_a)
    .fetch_one(&pool)
    .await
    .expect("seed unit");

    let token_b = mint_token(user_b, "epc-b@meter-idor.test");
    let uri = format!("/api/v1/energy/units/{unit_a}/epc");
    let resp = app.execute(app.get(&uri).bearer(&token_b).build()).await;

    assert_rejected(resp.status, "get_unit_epc cross-tenant");
    assert_ne!(
        resp.status,
        StatusCode::OK,
        "Org A unit EPC must not be readable by Org B"
    );
}

// ---------------------------------------------------------------------------
// #846 — energy: cross-org get_benchmark_dashboard (building path) is rejected
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_benchmark_dashboard_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "bm-a").await;
    let org_b = seed_org(&pool, "bm-b").await;
    let user_b = seed_user(&pool, "bm-b@meter-idor.test").await;
    seed_membership(&pool, org_b, user_b, "org_admin").await;
    let building_a = seed_building(&pool, org_a, "bm-a").await;

    let token_b = mint_token(user_b, "bm-b@meter-idor.test");
    let uri = format!("/api/v1/energy/buildings/{building_a}/benchmark");
    let resp = app.execute(app.get(&uri).bearer(&token_b).build()).await;

    assert_rejected(resp.status, "get_benchmark_dashboard cross-tenant");
    assert_ne!(
        resp.status,
        StatusCode::OK,
        "Org A benchmark dashboard must not be readable by Org B"
    );
}

// ---------------------------------------------------------------------------
// #846 — energy: unauthenticated benchmark dashboard is rejected
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_benchmark_dashboard_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "bm-noauth-a").await;
    let building_a = seed_building(&pool, org_a, "bm-noauth-a").await;

    let uri = format!("/api/v1/energy/buildings/{building_a}/benchmark");
    let resp = app.execute(app.get(&uri).build()).await;

    assert_rejected(resp.status, "get_benchmark_dashboard without bearer token");
}
