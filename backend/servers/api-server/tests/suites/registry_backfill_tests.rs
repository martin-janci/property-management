//! BIT-268 wave-4 (AI/automation) batch 1: registry success-path tests.
//!
//! Behaviour-asserting (2xx success path) coverage for the previously
//! `partial` (zero-test) endpoints in
//! docs/endpoint-checklist/groups/ai-automation.md — registry.rs group
//! (mount /api/v1/registry):
//!
//! Pets:
//! - POST   /api/v1/registry/pets
//! - GET    /api/v1/registry/pets
//! - GET    /api/v1/registry/pets/{id}
//! - PUT    /api/v1/registry/pets/{id}
//! - DELETE /api/v1/registry/pets/{id}
//! - POST   /api/v1/registry/pets/{id}/review
//!
//! Vehicles:
//! - POST   /api/v1/registry/vehicles
//! - GET    /api/v1/registry/vehicles
//! - GET    /api/v1/registry/vehicles/{id}
//! - PUT    /api/v1/registry/vehicles/{id}
//! - DELETE /api/v1/registry/vehicles/{id}
//! - POST   /api/v1/registry/vehicles/{id}/review
//!
//! Parking spots:
//! - POST   /api/v1/registry/parking-spots
//! - GET    /api/v1/registry/parking-spots
//! - GET    /api/v1/registry/parking-spots/{id}
//! - PUT    /api/v1/registry/parking-spots/{id}
//! - DELETE /api/v1/registry/parking-spots/{id}
//!
//! Rules + statistics:
//! - GET    /api/v1/registry/buildings/{building_id}/rules
//! - PUT    /api/v1/registry/buildings/{building_id}/rules
//! - GET    /api/v1/registry/buildings/{building_id}/statistics

#![allow(dead_code)]

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user_with_org, AuthenticatedSession, TestApp, TestUser};

// ---------------------------------------------------------------------------
// Seed helpers
// ---------------------------------------------------------------------------

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, name, street, city, postal_code, country, status)
        VALUES ($1, 'Registry Building', 'Test Street 1', 'Test City', '12345', 'SK', 'active')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_unit(pool: &PgPool, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO units (building_id, designation, floor) VALUES ($1, $2, 1) RETURNING id",
    )
    .bind(building_id)
    .bind(format!("U-{}", &Uuid::new_v4().to_string()[..8]))
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

/// Create a pet registration through the real endpoint and return its id.
async fn create_pet(session: &AuthenticatedSession<'_>, app: &TestApp, unit_id: Uuid) -> Uuid {
    let resp = app
        .execute(
            session
                .post("/api/v1/registry/pets")
                .json(json!({
                    "unit_id": unit_id,
                    "pet_name": "Rex",
                    "pet_type": "dog",
                    "pet_size": "medium"
                }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
    let body = resp.json_value();
    Uuid::parse_str(
        body["registration"]["id"]
            .as_str()
            .expect("registration.id"),
    )
    .expect("uuid")
}

/// Create a vehicle registration through the real endpoint and return its id.
async fn create_vehicle(session: &AuthenticatedSession<'_>, app: &TestApp, unit_id: Uuid) -> Uuid {
    let resp = app
        .execute(
            session
                .post("/api/v1/registry/vehicles")
                .json(json!({
                    "unit_id": unit_id,
                    "vehicle_type": "car",
                    "make": "Skoda",
                    "model": "Octavia",
                    "license_plate": "BA123AB"
                }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
    let body = resp.json_value();
    Uuid::parse_str(
        body["registration"]["id"]
            .as_str()
            .expect("registration.id"),
    )
    .expect("uuid")
}

/// Create a parking spot through the real endpoint and return its id.
async fn create_spot(session: &AuthenticatedSession<'_>, app: &TestApp, building_id: Uuid) -> Uuid {
    let resp = app
        .execute(
            session
                .post("/api/v1/registry/parking-spots")
                .json(json!({
                    "building_id": building_id,
                    "spot_number": format!("P-{}", &Uuid::new_v4().to_string()[..6]),
                    "spot_type": "standard"
                }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
    let body = resp.json_value();
    Uuid::parse_str(body["spot"]["id"].as_str().expect("spot.id")).expect("uuid")
}

// ---------------------------------------------------------------------------
// Pets
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_pet_registration_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-pet-c").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(
            session
                .post("/api/v1/registry/pets")
                .json(json!({
                    "unit_id": unit_id,
                    "pet_name": "Buddy",
                    "pet_type": "cat",
                    "pet_size": "small"
                }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
    assert!(resp.json_value()["registration"]["id"].is_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_pet_registrations_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-pet-l").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let session = app.session(token, org_id);
    let _ = create_pet(&session, &app, unit_id).await;

    let resp = app
        .execute(session.get("/api/v1/registry/pets").build())
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["registrations"].is_array());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_pet_registration_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-pet-g").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let session = app.session(token, org_id);
    let pet_id = create_pet(&session, &app, unit_id).await;

    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/registry/pets/{pet_id}"))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["registration"].is_object());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_pet_registration_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-pet-u").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let session = app.session(token, org_id);
    let pet_id = create_pet(&session, &app, unit_id).await;

    let resp = app
        .execute(
            session
                .put(&format!("/api/v1/registry/pets/{pet_id}"))
                .json(json!({ "pet_name": "Rex II", "color": "brown" }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn review_pet_registration_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-pet-r").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let session = app.session(token, org_id);
    let pet_id = create_pet(&session, &app, unit_id).await;

    let resp = app
        .execute(
            session
                .post(&format!("/api/v1/registry/pets/{pet_id}/review"))
                .json(json!({ "approve": true }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_pet_registration_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-pet-d").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let session = app.session(token, org_id);
    let pet_id = create_pet(&session, &app, unit_id).await;

    let resp = app
        .execute(
            session
                .delete(&format!("/api/v1/registry/pets/{pet_id}"))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::NO_CONTENT, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// Vehicles
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_vehicle_registration_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-veh-c").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(
            session
                .post("/api/v1/registry/vehicles")
                .json(json!({
                    "unit_id": unit_id,
                    "vehicle_type": "car",
                    "make": "Toyota",
                    "model": "Corolla",
                    "license_plate": "BA999XY"
                }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
    assert!(resp.json_value()["registration"]["id"].is_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_vehicle_registrations_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-veh-l").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let session = app.session(token, org_id);
    let _ = create_vehicle(&session, &app, unit_id).await;

    let resp = app
        .execute(session.get("/api/v1/registry/vehicles").build())
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["registrations"].is_array());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_vehicle_registration_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-veh-g").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let session = app.session(token, org_id);
    let vehicle_id = create_vehicle(&session, &app, unit_id).await;

    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/registry/vehicles/{vehicle_id}"))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["registration"].is_object());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_vehicle_registration_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-veh-u").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let session = app.session(token, org_id);
    let vehicle_id = create_vehicle(&session, &app, unit_id).await;

    let resp = app
        .execute(
            session
                .put(&format!("/api/v1/registry/vehicles/{vehicle_id}"))
                .json(json!({ "color": "blue", "year": 2022 }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn review_vehicle_registration_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-veh-r").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let session = app.session(token, org_id);
    let vehicle_id = create_vehicle(&session, &app, unit_id).await;

    let resp = app
        .execute(
            session
                .post(&format!("/api/v1/registry/vehicles/{vehicle_id}/review"))
                .json(json!({ "approve": true }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_vehicle_registration_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-veh-d").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let session = app.session(token, org_id);
    let vehicle_id = create_vehicle(&session, &app, unit_id).await;

    let resp = app
        .execute(
            session
                .delete(&format!("/api/v1/registry/vehicles/{vehicle_id}"))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::NO_CONTENT, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// Parking spots
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_parking_spot_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-spot-c").await;
    let building_id = seed_building(&pool, org_id).await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(
            session
                .post("/api/v1/registry/parking-spots")
                .json(json!({
                    "building_id": building_id,
                    "spot_number": "A-101",
                    "spot_type": "standard",
                    "is_covered": true
                }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
    assert!(resp.json_value()["spot"]["id"].is_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_parking_spots_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-spot-l").await;
    let building_id = seed_building(&pool, org_id).await;
    let session = app.session(token, org_id);
    let _ = create_spot(&session, &app, building_id).await;

    let resp = app
        .execute(session.get("/api/v1/registry/parking-spots").build())
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["spots"].is_array());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_parking_spot_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-spot-g").await;
    let building_id = seed_building(&pool, org_id).await;
    let session = app.session(token, org_id);
    let spot_id = create_spot(&session, &app, building_id).await;

    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/registry/parking-spots/{spot_id}"))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["spot"].is_object());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_parking_spot_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-spot-u").await;
    let building_id = seed_building(&pool, org_id).await;
    let session = app.session(token, org_id);
    let spot_id = create_spot(&session, &app, building_id).await;

    let resp = app
        .execute(
            session
                .put(&format!("/api/v1/registry/parking-spots/{spot_id}"))
                .json(json!({ "is_reserved": true, "floor_level": "-1" }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_parking_spot_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-spot-d").await;
    let building_id = seed_building(&pool, org_id).await;
    let session = app.session(token, org_id);
    let spot_id = create_spot(&session, &app, building_id).await;

    let resp = app
        .execute(
            session
                .delete(&format!("/api/v1/registry/parking-spots/{spot_id}"))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::NO_CONTENT, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// Rules + statistics
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_registry_rules_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-rules-g").await;
    let building_id = seed_building(&pool, org_id).await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/registry/buildings/{building_id}/rules"))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["rules"].is_object());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_registry_rules_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-rules-u").await;
    let building_id = seed_building(&pool, org_id).await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(
            session
                .put(&format!("/api/v1/registry/buildings/{building_id}/rules"))
                .json(json!({
                    "pets_allowed": true,
                    "pets_require_approval": false,
                    "max_pets_per_unit": 2
                }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["rules"].is_object());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_registry_statistics_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-stats-g").await;
    let building_id = seed_building(&pool, org_id).await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/registry/buildings/{building_id}/statistics"
                ))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["statistics"].is_object());
}
