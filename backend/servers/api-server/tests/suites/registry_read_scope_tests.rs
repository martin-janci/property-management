//! Registry read scope: pet/vehicle registrations hold personal data (owner
//! name, unit, microchip number, licence plate). Managers (DB role) see every
//! registration in the organization; any other member only sees registrations
//! on units they are linked to (active `unit_owners` row or a `unit_residents`
//! row with `end_date IS NULL`). Someone else's registration reads as 404.

#![allow(dead_code)]

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{
    create_authenticated_user, create_authenticated_user_with_org, seed_membership,
    AuthenticatedSession, TestApp, TestUser,
};

struct Fixture {
    org: Uuid,
    manager_token: String,
    tenant_token: String,
    neighbor_token: String,
    /// Unit 2B: the tenant is a current resident.
    unit_2b: Uuid,
    /// Unit 3C: the neighbor is an active owner; the tenant lived there before.
    unit_3c: Uuid,
    tenant_pet: Uuid,
    tenant_vehicle: Uuid,
    neighbor_pet: Uuid,
    neighbor_vehicle: Uuid,
}

async fn user_id(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("user id")
}

async fn seed_unit(pool: &PgPool, building: Uuid, designation: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO units (building_id, designation, floor) VALUES ($1, $2, 1) RETURNING id",
    )
    .bind(building)
    .bind(designation)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

async fn create(
    session: &AuthenticatedSession<'_>,
    app: &TestApp,
    path: &str,
    body: Value,
) -> Uuid {
    let resp = app.execute(session.post(path).json(body).build()).await;
    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
    Uuid::parse_str(resp.json_value()["registration"]["id"].as_str().unwrap()).unwrap()
}

async fn create_pet(s: &AuthenticatedSession<'_>, app: &TestApp, unit: Uuid, chip: &str) -> Uuid {
    create(
        s,
        app,
        "/api/v1/registry/pets",
        json!({ "unit_id": unit, "pet_name": "Rex", "pet_type": "dog",
                "pet_size": "medium", "microchip_id": chip }),
    )
    .await
}

async fn create_vehicle(
    s: &AuthenticatedSession<'_>,
    app: &TestApp,
    unit: Uuid,
    plate: &str,
) -> Uuid {
    create(
        s,
        app,
        "/api/v1/registry/vehicles",
        json!({ "unit_id": unit, "vehicle_type": "car", "make": "Skoda",
                "model": "Octavia", "license_plate": plate }),
    )
    .await
}

async fn setup(app: &TestApp, pool: &PgPool) -> Fixture {
    let manager = TestUser::new();
    let (manager_token, org) = create_authenticated_user_with_org(app, &manager, "reg-scope").await;

    let building = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO buildings (organization_id, name, street, city, postal_code, country, status) \
         VALUES ($1, 'Scope Bldg', 'Street 1', 'City', '12345', 'SK', 'active') RETURNING id",
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .expect("seed building");
    let unit_2b = seed_unit(pool, building, "2B").await;
    let unit_3c = seed_unit(pool, building, "3C").await;

    let tenant = TestUser::new();
    let (tenant_token, _) = create_authenticated_user(app, &tenant).await;
    let tenant_id = user_id(pool, &tenant.email).await;
    seed_membership(pool, org, tenant_id, "tenant").await;
    // Current residency in 2B, and a residency in 3C that has ended.
    sqlx::query(
        "INSERT INTO unit_residents (unit_id, user_id, resident_type, is_primary) \
         VALUES ($1, $2, 'tenant', true)",
    )
    .bind(unit_2b)
    .bind(tenant_id)
    .execute(pool)
    .await
    .expect("seed current residency");
    sqlx::query(
        "INSERT INTO unit_residents (unit_id, user_id, resident_type, is_primary, start_date, end_date) \
         VALUES ($1, $2, 'tenant', false, CURRENT_DATE - 400, CURRENT_DATE - 30)",
    )
    .bind(unit_3c)
    .bind(tenant_id)
    .execute(pool)
    .await
    .expect("seed ended residency");

    let neighbor = TestUser::new();
    let (neighbor_token, _) = create_authenticated_user(app, &neighbor).await;
    let neighbor_id = user_id(pool, &neighbor.email).await;
    seed_membership(pool, org, neighbor_id, "owner").await;
    sqlx::query("INSERT INTO unit_owners (unit_id, user_id, status) VALUES ($1, $2, 'active')")
        .bind(unit_3c)
        .bind(neighbor_id)
        .execute(pool)
        .await
        .expect("seed unit owner");

    let ts = app.session(tenant_token.clone(), org);
    let ns = app.session(neighbor_token.clone(), org);
    let tenant_pet = create_pet(&ts, app, unit_2b, "CHIP-TENANT").await;
    let tenant_vehicle = create_vehicle(&ts, app, unit_2b, "BA111AA").await;
    let neighbor_pet = create_pet(&ns, app, unit_3c, "CHIP-NEIGHBOR").await;
    let neighbor_vehicle = create_vehicle(&ns, app, unit_3c, "BA222BB").await;

    Fixture {
        org,
        manager_token,
        tenant_token,
        neighbor_token,
        unit_2b,
        unit_3c,
        tenant_pet,
        tenant_vehicle,
        neighbor_pet,
        neighbor_vehicle,
    }
}

async fn list_ids(
    s: &AuthenticatedSession<'_>,
    app: &TestApp,
    path: &str,
) -> (Vec<Uuid>, i64, String) {
    let resp = app.execute(s.get(path).build()).await;
    assert_eq!(resp.status, StatusCode::OK, "{path}: {}", resp.text());
    let body = resp.json_value();
    let mut ids: Vec<Uuid> = body["registrations"]
        .as_array()
        .expect("registrations array")
        .iter()
        .map(|r| Uuid::parse_str(r["id"].as_str().unwrap()).unwrap())
        .collect();
    ids.sort();
    (ids, body["total"].as_i64().expect("total"), resp.text())
}

fn sorted(mut v: Vec<Uuid>) -> Vec<Uuid> {
    v.sort();
    v
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn tenant_lists_only_own_unit_registrations(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let fx = setup(&app, &pool).await;
    let ts = app.session(fx.tenant_token.clone(), fx.org);

    let (ids, total, raw) = list_ids(&ts, &app, "/api/v1/registry/pets").await;
    assert_eq!(ids, vec![fx.tenant_pet]);
    assert_eq!(total, 1);
    assert!(
        !raw.contains("CHIP-NEIGHBOR") && !raw.contains(&fx.unit_3c.to_string()),
        "{raw}"
    );

    let (ids, total, raw) = list_ids(&ts, &app, "/api/v1/registry/vehicles").await;
    assert_eq!(ids, vec![fx.tenant_vehicle]);
    assert_eq!(total, 1);
    assert!(!raw.contains("BA222BB"), "{raw}");

    // Explicit filters for the neighbor's unit cannot widen the scope; the
    // tenant's ended residency in 3C does not count as a link.
    for path in [
        format!("/api/v1/registry/pets?unit_id={}", fx.unit_3c),
        format!("/api/v1/registry/vehicles?unit_id={}", fx.unit_3c),
    ] {
        let (ids, total, _) = list_ids(&ts, &app, &path).await;
        assert!(ids.is_empty(), "{path} leaked {ids:?}");
        assert_eq!(total, 0, "{path}");
    }

    // The neighbor (linked via unit_owners) likewise sees only unit 3C.
    let ns = app.session(fx.neighbor_token.clone(), fx.org);
    let (ids, total, _) = list_ids(&ns, &app, "/api/v1/registry/pets").await;
    assert_eq!(ids, vec![fx.neighbor_pet]);
    assert_eq!(total, 1);
    let (ids, _, _) = list_ids(
        &ns,
        &app,
        &format!("/api/v1/registry/vehicles?unit_id={}", fx.unit_2b),
    )
    .await;
    assert!(ids.is_empty());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn tenant_gets_404_for_other_units_registration(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let fx = setup(&app, &pool).await;
    let ts = app.session(fx.tenant_token.clone(), fx.org);

    for (path, expected) in [
        (
            format!("/api/v1/registry/pets/{}", fx.neighbor_pet),
            StatusCode::NOT_FOUND,
        ),
        (
            format!("/api/v1/registry/vehicles/{}", fx.neighbor_vehicle),
            StatusCode::NOT_FOUND,
        ),
        (
            format!("/api/v1/registry/pets/{}", fx.tenant_pet),
            StatusCode::OK,
        ),
        (
            format!("/api/v1/registry/vehicles/{}", fx.tenant_vehicle),
            StatusCode::OK,
        ),
    ] {
        let resp = app.execute(ts.get(&path).build()).await;
        assert_eq!(resp.status, expected, "{path}: {}", resp.text());
        if expected == StatusCode::NOT_FOUND {
            let raw = resp.text();
            assert!(
                !raw.contains("CHIP-NEIGHBOR") && !raw.contains("BA222BB"),
                "{raw}"
            );
        }
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_sees_all_registrations(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let fx = setup(&app, &pool).await;
    let ms = app.session(fx.manager_token.clone(), fx.org);

    let (ids, total, _) = list_ids(&ms, &app, "/api/v1/registry/pets").await;
    assert_eq!(ids, sorted(vec![fx.tenant_pet, fx.neighbor_pet]));
    assert_eq!(total, 2);
    let (ids, total, _) = list_ids(&ms, &app, "/api/v1/registry/vehicles").await;
    assert_eq!(ids, sorted(vec![fx.tenant_vehicle, fx.neighbor_vehicle]));
    assert_eq!(total, 2);

    for path in [
        format!("/api/v1/registry/pets/{}", fx.neighbor_pet),
        format!("/api/v1/registry/pets/{}", fx.tenant_pet),
        format!("/api/v1/registry/vehicles/{}", fx.neighbor_vehicle),
        format!("/api/v1/registry/vehicles/{}", fx.tenant_vehicle),
    ] {
        let resp = app.execute(ms.get(&path).build()).await;
        assert_eq!(resp.status, StatusCode::OK, "{path}: {}", resp.text());
    }
}
