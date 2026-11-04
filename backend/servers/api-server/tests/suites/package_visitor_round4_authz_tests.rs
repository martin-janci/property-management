//! Round-4 regressions (audit4-operations) for packages and visitors.
//!
//! * delete package / building package & visitor settings are staff-only
//!   (role from the `organization_members` row, not the JWT);
//! * update / pickup package: the package's resident or staff only;
//! * update / cancel / delete visitor: the host or staff only;
//! * ids from another org are 404.

#![allow(dead_code)]

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, seed_org, TestApp, TestConfig};

async fn member(pool: &PgPool, org: Uuid, role: &str) -> (String, Uuid) {
    use api_server::services::JwtService;
    let email = format!("pv-r4-{}@test.internal", Uuid::new_v4());
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'PV R4', 'active', NOW()) RETURNING id"#,
    )
    .bind(&email)
    .fetch_one(pool)
    .await
    .unwrap();
    seed_membership(pool, org, id, role).await;
    let config = TestConfig::default();
    let token = JwtService::new(&config.jwt_secret)
        .unwrap()
        .generate_access_token(id, &email, "PV R4", None, None)
        .unwrap();
    (token, id)
}

async fn seed_building_unit(pool: &PgPool, org_id: Uuid) -> (Uuid, Uuid) {
    let building = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'PV St 1', 'Bratislava', '81101', 'Slovakia') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let unit = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation) VALUES ($1, 'R-1') RETURNING id"#,
    )
    .bind(building)
    .fetch_one(pool)
    .await
    .unwrap();
    (building, unit)
}

async fn seed_package(
    pool: &PgPool,
    org: Uuid,
    building: Uuid,
    unit: Uuid,
    resident: Uuid,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO packages (tenant_id, building_id, unit_id, resident_id, status)
           VALUES ($1, $2, $3, $4, 'received') RETURNING id"#,
    )
    .bind(org)
    .bind(building)
    .bind(unit)
    .bind(resident)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn seed_visitor(pool: &PgPool, org: Uuid, building: Uuid, unit: Uuid, host: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO visitors (tenant_id, building_id, unit_id, host_id, visitor_name,
                                 access_code, access_code_expires_at, expected_arrival)
           VALUES ($1, $2, $3, $4, 'Guest', 'ABC123', NOW() + INTERVAL '1 day', NOW() + INTERVAL '2 hours')
           RETURNING id"#,
    )
    .bind(org)
    .bind(building)
    .bind(unit)
    .bind(host)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn package_status(pool: &PgPool, id: Uuid) -> String {
    sqlx::query_scalar("SELECT status::text FROM packages WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_cannot_touch_others_packages_visitors_or_settings(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "pvr4r").await;
    let (_t1, victim) = member(&pool, org, "resident").await;
    let (token, _attacker) = member(&pool, org, "resident").await;
    let (building, unit) = seed_building_unit(&pool, org).await;
    let pkg = seed_package(&pool, org, building, unit, victim).await;
    let vis = seed_visitor(&pool, org, building, unit, victim).await;
    let tenant = org.to_string();

    let cases = vec![
        app.post(&format!("/api/v1/packages/{pkg}/pickup"))
            .json(json!({})),
        app.put(&format!("/api/v1/packages/{pkg}"))
            .json(json!({ "description": "mine now" })),
        app.delete(&format!("/api/v1/packages/{pkg}")),
        app.put(&format!("/api/v1/packages/buildings/{building}/settings"))
            .json(json!({})),
        app.post(&format!("/api/v1/visitors/{vis}/cancel")),
        app.delete(&format!("/api/v1/visitors/{vis}")),
        app.put(&format!("/api/v1/visitors/buildings/{building}/settings"))
            .json(json!({})),
    ];
    for rb in cases {
        let req = rb.bearer(&token).header("X-Tenant-ID", &tenant).build();
        let uri = req.uri().to_string();
        let resp = app.execute(req).await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "{uri} must be resident-owner or staff only: {}",
            resp.text()
        );
    }
    assert_eq!(package_status(&pool, pkg).await, "received");
    let vis_status: String = sqlx::query_scalar("SELECT status::text FROM visitors WHERE id = $1")
        .bind(vis)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(vis_status, "pending");
}

// Positive owner path. (`pickup_package` / `cancel_visitor` themselves hit a
// pre-existing enum-bind 500 on dev — BIT-440 quarantine — so the owner path
// is exercised through `delete_visitor`, which goes through the same
// host-or-staff gate.)
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn host_can_delete_own_visitor_and_staff_can_delete_package(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "pvr4o").await;
    let (token_r, resident) = member(&pool, org, "resident").await;
    let (token_m, _) = member(&pool, org, "manager").await;
    let (building, unit) = seed_building_unit(&pool, org).await;
    let vis = seed_visitor(&pool, org, building, unit, resident).await;
    let pkg = seed_package(&pool, org, building, unit, resident).await;
    let tenant = org.to_string();

    let resp = app
        .execute(
            app.delete(&format!("/api/v1/visitors/{vis}"))
                .bearer(&token_r)
                .header("X-Tenant-ID", &tenant)
                .build(),
        )
        .await;
    assert!(resp.status.is_success(), "{} {}", resp.status, resp.text());

    let resp = app
        .execute(
            app.delete(&format!("/api/v1/packages/{pkg}"))
                .bearer(&token_m)
                .header("X-Tenant-ID", &tenant)
                .build(),
        )
        .await;
    assert!(resp.status.is_success(), "{} {}", resp.status, resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn foreign_org_package_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "pvr4a").await;
    let org_b = seed_org(&pool, "pvr4b").await;
    let (_ta, resident_a) = member(&pool, org_a, "resident").await;
    let (token_b, _) = member(&pool, org_b, "manager").await;
    let (building, unit) = seed_building_unit(&pool, org_a).await;
    let pkg = seed_package(&pool, org_a, building, unit, resident_a).await;

    let resp = app
        .execute(
            app.post(&format!("/api/v1/packages/{pkg}/pickup"))
                .json(json!({}))
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_b.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND, "{}", resp.text());
    assert_eq!(package_status(&pool, pkg).await, "received");
}
