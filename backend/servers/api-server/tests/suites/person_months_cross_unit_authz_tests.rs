//! Cross-unit authorization for the person-month READ endpoints (pm-security).
//!
//! Person counts are residency data: a non-manager may read them only for a
//! unit they currently live in, and building-wide views stay manager-only.
//! This suite locks that in against regression for every read vector:
//!
//! - `GET  /units/{uid}/person-months`            (unit list)
//! - `GET  /units/{uid}/person-months/{id}`       (by id)
//! - `GET  /units/{uid}/person-months/yearly`     (yearly summary)
//! - `POST /units/{uid}/person-months/calculate`  (calculate from residents)
//! - `GET  /person-months`                        (building list)
//! - `GET  /person-months/summary`                (building summary)
//!
//! A resident of unit 1A gets 403 reading unit 1B, 403 on either building-wide
//! read, and succeeds only on their own unit. A manager reads everything. The
//! gate lives in `routes::person_months::authorize_building_inner`.

use axum::http::StatusCode;
use common::tenant::TenantRole;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{mint_tenant_token, seed_membership, seed_org, TestApp};

async fn seed_user(pool: &PgPool, label: &str) -> (Uuid, String) {
    let email = format!("{label}-{}@pm-authz.test", Uuid::new_v4());
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name) VALUES ($1, 'x', $2) RETURNING id",
    )
    .bind(&email)
    .bind(label)
    .fetch_one(pool)
    .await
    .expect("seed user");
    (id, email)
}

struct Fixture {
    org: Uuid,
    building: Uuid,
    /// Unit the resident lives in.
    own_unit: Uuid,
    /// A sibling unit in the same building the resident does NOT live in.
    other_unit: Uuid,
    resident: (Uuid, String),
    manager: (Uuid, String),
}

async fn fixture(pool: &PgPool) -> Fixture {
    let org = seed_org(pool, "pm-authz").await;
    let building = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Authz St 1', 'Bratislava', '81101', 'SK') RETURNING id"#,
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .expect("building");
    let mut units = vec![];
    for d in ["1A", "1B"] {
        units.push(
            sqlx::query_scalar::<_, Uuid>(
                "INSERT INTO units (building_id, designation) VALUES ($1, $2) RETURNING id",
            )
            .bind(building)
            .bind(d)
            .fetch_one(pool)
            .await
            .expect("unit"),
        );
    }
    let resident = seed_user(pool, "resident").await;
    let manager = seed_user(pool, "manager").await;
    // Both are org members (passes the membership gate); role differs.
    seed_membership(pool, org, resident.0, "resident").await;
    seed_membership(pool, org, manager.0, "manager").await;
    // The resident lives in 1A only.
    sqlx::query(
        "INSERT INTO unit_residents (unit_id, user_id, resident_type) VALUES ($1, $2, 'tenant')",
    )
    .bind(units[0])
    .bind(resident.0)
    .execute(pool)
    .await
    .expect("residency");
    Fixture {
        org,
        building,
        own_unit: units[0],
        other_unit: units[1],
        resident,
        manager,
    }
}

/// A non-manager resident of unit 1A is refused (403) on every read vector for
/// a sibling unit 1B, and on both building-wide reads.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn non_manager_resident_cannot_read_other_unit_or_building(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let f = fixture(&pool).await;
    let token = mint_tenant_token(f.resident.0, &f.resident.1, f.org, TenantRole::Resident);

    let other = format!(
        "/api/v1/buildings/{}/units/{}/person-months",
        f.building, f.other_unit
    );
    // GET unit list / yearly / by-id — all 403 for a unit the caller does not
    // reside in. Authorization runs before any lookup, so a random id is fine.
    let random_id = Uuid::new_v4();
    let cross_unit_reads = [
        format!("{other}?year=2026"),
        format!("{other}/yearly?year=2026"),
        format!("{other}/{random_id}"),
    ];
    for uri in &cross_unit_reads {
        let resp = app
            .execute(app.get(uri).bearer(&token).tenant(f.org).build())
            .await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "cross-unit read {uri} must be 403: {}",
            resp.text()
        );
        assert_eq!(resp.json_value()["code"], "NOT_RESIDENT", "{}", resp.text());
    }

    // POST /calculate on the other unit — 403.
    let resp = app
        .execute(
            app.post(&format!("{other}/calculate"))
                .bearer(&token)
                .tenant(f.org)
                .json(json!({ "year": 2026, "month": 2 }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "cross-unit calculate must be 403: {}",
        resp.text()
    );

    // Building-wide reads are manager-only; a non-manager gets 403 even for a
    // building that contains their own unit.
    let bldg = format!("/api/v1/buildings/{}/person-months", f.building);
    for uri in [
        format!("{bldg}?year=2026&month=3"),
        format!("{bldg}/summary?year=2026&month=3"),
    ] {
        let resp = app
            .execute(app.get(&uri).bearer(&token).tenant(f.org).build())
            .await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "building-wide read {uri} must be 403 for non-manager: {}",
            resp.text()
        );
    }
}

/// The same resident CAN read their own unit — the gate restricts, it does not
/// block residents outright.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_can_read_their_own_unit(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let f = fixture(&pool).await;
    let token = mint_tenant_token(f.resident.0, &f.resident.1, f.org, TenantRole::Resident);

    let own = format!(
        "/api/v1/buildings/{}/units/{}/person-months",
        f.building, f.own_unit
    );
    // Unit list and yearly summary succeed for the caller's own unit.
    for uri in [
        format!("{own}?year=2026"),
        format!("{own}/yearly?year=2026"),
    ] {
        let resp = app
            .execute(app.get(&uri).bearer(&token).tenant(f.org).build())
            .await;
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "own-unit read {uri} must be 200: {}",
            resp.text()
        );
    }
}

/// A manager reads any unit and the building-wide views without restriction.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_reads_any_unit_and_building(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let f = fixture(&pool).await;
    let token = mint_tenant_token(f.manager.0, &f.manager.1, f.org, TenantRole::Manager);

    // The manager is not a resident of 1B, yet reads it fine.
    let other = format!(
        "/api/v1/buildings/{}/units/{}/person-months",
        f.building, f.other_unit
    );
    for uri in [
        format!("{other}?year=2026"),
        format!("{other}/yearly?year=2026"),
    ] {
        let resp = app
            .execute(app.get(&uri).bearer(&token).tenant(f.org).build())
            .await;
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "manager read {uri} must be 200: {}",
            resp.text()
        );
    }

    // Building-wide reads succeed for a manager.
    let bldg = format!("/api/v1/buildings/{}/person-months", f.building);
    let resp = app
        .execute(
            app.get(&format!("{bldg}?year=2026&month=3"))
                .bearer(&token)
                .tenant(f.org)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
    // Building summary is 200 with data or 404 when the period is empty; either
    // proves the manager passed the authorization gate (a non-manager is 403).
    let resp = app
        .execute(
            app.get(&format!("{bldg}/summary?year=2026&month=3"))
                .bearer(&token)
                .tenant(f.org)
                .build(),
        )
        .await;
    assert!(
        resp.status == StatusCode::OK || resp.status == StatusCode::NOT_FOUND,
        "manager building summary must not be 403, got {}: {}",
        resp.status,
        resp.text()
    );
}
