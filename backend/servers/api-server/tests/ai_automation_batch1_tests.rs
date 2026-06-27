//! BIT-297 Batch 1 — Registry success-path integration tests.
//!
//! Covers: pet registrations (CRUD + review), vehicle registrations (CRUD + review),
//! parking spots (CRUD), building registry rules (get + update) and statistics.
//! All tests assert 200/201 success paths for real handlers.

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use common::{create_authenticated_user_with_org, TestApp, TestUser};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Seed helpers
// ---------------------------------------------------------------------------

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO buildings (organization_id, street, city, postal_code, country) \
         VALUES ($1, 'Registry St 1', 'Bratislava', '81101', 'Slovakia') RETURNING id",
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_unit(pool: &PgPool, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO units (building_id, designation, floor) VALUES ($1, 'Reg-A1', 1) RETURNING id",
    )
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

// ---------------------------------------------------------------------------
// Pet registration — success paths
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_pet_registration_returns_201(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-pet-create").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;

    let resp = app
        .execute(
            app.post("/api/v1/registry/pets")
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({
                    "unit_id": unit_id,
                    "pet_name": "Fluffy",
                    "pet_type": "cat",
                    "pet_size": "small"
                }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::CREATED);
    assert!(resp.json_value()["registration"]["id"].is_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_pet_registrations_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-pet-list").await;

    let resp = app
        .execute(
            app.get("/api/v1/registry/pets")
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.json_value()["registrations"].is_array());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_pet_registration_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-pet-get").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;

    let create_resp = app
        .execute(
            app.post("/api/v1/registry/pets")
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({
                    "unit_id": unit_id,
                    "pet_name": "Rex",
                    "pet_type": "dog",
                    "pet_size": "large"
                }))
                .build(),
        )
        .await;
    assert_eq!(create_resp.status, StatusCode::CREATED);
    let pet_id = create_resp.json_value()["registration"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .execute(
            app.get(&format!("/api/v1/registry/pets/{pet_id}"))
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(
        resp.json_value()["registration"]["id"].as_str().unwrap(),
        pet_id
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_pet_registration_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-pet-update").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;

    let create_resp = app
        .execute(
            app.post("/api/v1/registry/pets")
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({
                    "unit_id": unit_id,
                    "pet_name": "Bella",
                    "pet_type": "dog",
                    "pet_size": "medium"
                }))
                .build(),
        )
        .await;
    assert_eq!(create_resp.status, StatusCode::CREATED);
    let pet_id = create_resp.json_value()["registration"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .execute(
            app.put(&format!("/api/v1/registry/pets/{pet_id}"))
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({"pet_name": "Bella Updated"}))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_pet_registration_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-pet-delete").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;

    let create_resp = app
        .execute(
            app.post("/api/v1/registry/pets")
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({
                    "unit_id": unit_id,
                    "pet_name": "ToDelete",
                    "pet_type": "cat",
                    "pet_size": "small"
                }))
                .build(),
        )
        .await;
    assert_eq!(create_resp.status, StatusCode::CREATED);
    let pet_id = create_resp.json_value()["registration"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .execute(
            app.delete(&format!("/api/v1/registry/pets/{pet_id}"))
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn review_pet_registration_approve_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-pet-review").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;

    let create_resp = app
        .execute(
            app.post("/api/v1/registry/pets")
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({
                    "unit_id": unit_id,
                    "pet_name": "ForReview",
                    "pet_type": "dog",
                    "pet_size": "large"
                }))
                .build(),
        )
        .await;
    assert_eq!(create_resp.status, StatusCode::CREATED);
    let pet_id = create_resp.json_value()["registration"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .execute(
            app.post(&format!("/api/v1/registry/pets/{pet_id}/review"))
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({"approve": true}))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

// ---------------------------------------------------------------------------
// Vehicle registration — success paths
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_vehicle_registration_returns_201(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-veh-create").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;

    let resp = app
        .execute(
            app.post("/api/v1/registry/vehicles")
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({
                    "unit_id": unit_id,
                    "vehicle_type": "car",
                    "make": "Skoda",
                    "model": "Octavia",
                    "license_plate": "BA123AA"
                }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::CREATED);
    assert!(resp.json_value()["registration"]["id"].is_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_vehicle_registrations_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-veh-list").await;

    let resp = app
        .execute(
            app.get("/api/v1/registry/vehicles")
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.json_value()["registrations"].is_array());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_vehicle_registration_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-veh-get").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;

    let create_resp = app
        .execute(
            app.post("/api/v1/registry/vehicles")
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({
                    "unit_id": unit_id,
                    "vehicle_type": "car",
                    "make": "VW",
                    "model": "Golf",
                    "license_plate": "TT001BB"
                }))
                .build(),
        )
        .await;
    assert_eq!(create_resp.status, StatusCode::CREATED);
    let vehicle_id = create_resp.json_value()["registration"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .execute(
            app.get(&format!("/api/v1/registry/vehicles/{vehicle_id}"))
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(
        resp.json_value()["registration"]["id"].as_str().unwrap(),
        vehicle_id
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_vehicle_registration_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-veh-update").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;

    let create_resp = app
        .execute(
            app.post("/api/v1/registry/vehicles")
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({
                    "unit_id": unit_id,
                    "vehicle_type": "car",
                    "make": "BMW",
                    "model": "3 Series",
                    "license_plate": "NZ555CC"
                }))
                .build(),
        )
        .await;
    assert_eq!(create_resp.status, StatusCode::CREATED);
    let vehicle_id = create_resp.json_value()["registration"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .execute(
            app.put(&format!("/api/v1/registry/vehicles/{vehicle_id}"))
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({"color": "red"}))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_vehicle_registration_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-veh-delete").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;

    let create_resp = app
        .execute(
            app.post("/api/v1/registry/vehicles")
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({
                    "unit_id": unit_id,
                    "vehicle_type": "motorcycle",
                    "make": "Honda",
                    "model": "CB500",
                    "license_plate": "PO777DD"
                }))
                .build(),
        )
        .await;
    assert_eq!(create_resp.status, StatusCode::CREATED);
    let vehicle_id = create_resp.json_value()["registration"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .execute(
            app.delete(&format!("/api/v1/registry/vehicles/{vehicle_id}"))
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn review_vehicle_registration_approve_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-veh-review").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;

    let create_resp = app
        .execute(
            app.post("/api/v1/registry/vehicles")
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({
                    "unit_id": unit_id,
                    "vehicle_type": "car",
                    "make": "Audi",
                    "model": "A4",
                    "license_plate": "KE888EE"
                }))
                .build(),
        )
        .await;
    assert_eq!(create_resp.status, StatusCode::CREATED);
    let vehicle_id = create_resp.json_value()["registration"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .execute(
            app.post(&format!("/api/v1/registry/vehicles/{vehicle_id}/review"))
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({"approve": true}))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

// ---------------------------------------------------------------------------
// Parking spots — success paths
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_parking_spot_returns_201(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) =
        create_authenticated_user_with_org(&app, &user, "reg-parking-create").await;
    let building_id = seed_building(&pool, org_id).await;

    let resp = app
        .execute(
            app.post("/api/v1/registry/parking-spots")
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({
                    "building_id": building_id,
                    "spot_number": "P-001"
                }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::CREATED);
    assert!(resp.json_value()["spot"]["id"].is_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_parking_spots_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-parking-list").await;

    let resp = app
        .execute(
            app.get("/api/v1/registry/parking-spots")
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.json_value()["spots"].is_array());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_parking_spot_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-parking-get").await;
    let building_id = seed_building(&pool, org_id).await;

    let create_resp = app
        .execute(
            app.post("/api/v1/registry/parking-spots")
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({
                    "building_id": building_id,
                    "spot_number": "P-002"
                }))
                .build(),
        )
        .await;
    assert_eq!(create_resp.status, StatusCode::CREATED);
    let spot_id = create_resp.json_value()["spot"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .execute(
            app.get(&format!("/api/v1/registry/parking-spots/{spot_id}"))
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.json_value()["spot"]["id"].as_str().unwrap(), spot_id);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_parking_spot_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) =
        create_authenticated_user_with_org(&app, &user, "reg-parking-update").await;
    let building_id = seed_building(&pool, org_id).await;

    let create_resp = app
        .execute(
            app.post("/api/v1/registry/parking-spots")
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({
                    "building_id": building_id,
                    "spot_number": "P-003"
                }))
                .build(),
        )
        .await;
    assert_eq!(create_resp.status, StatusCode::CREATED);
    let spot_id = create_resp.json_value()["spot"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .execute(
            app.put(&format!("/api/v1/registry/parking-spots/{spot_id}"))
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({"is_covered": true}))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_parking_spot_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) =
        create_authenticated_user_with_org(&app, &user, "reg-parking-delete").await;
    let building_id = seed_building(&pool, org_id).await;

    let create_resp = app
        .execute(
            app.post("/api/v1/registry/parking-spots")
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({
                    "building_id": building_id,
                    "spot_number": "P-DEL"
                }))
                .build(),
        )
        .await;
    assert_eq!(create_resp.status, StatusCode::CREATED);
    let spot_id = create_resp.json_value()["spot"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .execute(
            app.delete(&format!("/api/v1/registry/parking-spots/{spot_id}"))
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

// ---------------------------------------------------------------------------
// Building registry rules and statistics
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_registry_rules_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-rules-get").await;
    let building_id = seed_building(&pool, org_id).await;

    let resp = app
        .execute(
            app.get(&format!("/api/v1/registry/buildings/{building_id}/rules"))
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.json_value()["rules"].is_object());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_registry_rules_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-rules-update").await;
    let building_id = seed_building(&pool, org_id).await;

    let resp = app
        .execute(
            app.put(&format!("/api/v1/registry/buildings/{building_id}/rules"))
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({"pets_allowed": false}))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_registry_statistics_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-stats-get").await;
    let building_id = seed_building(&pool, org_id).await;

    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/registry/buildings/{building_id}/statistics"
            ))
            .bearer(&token)
            .header("X-Tenant-ID", &org_id.to_string())
            .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.json_value()["statistics"].is_object());
}
