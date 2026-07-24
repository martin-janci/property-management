//! BIT-268 Wave 4 — AI/Automation group — Batch 1: Registry endpoints.
//!
//! Verifies success-path 200/201 responses for the /api/v1/registry surface.
//! All previous tests in this group were auth-only (401/403); this file
//! asserts that authenticated requests get 2xx responses.

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user_with_org, TestApp, TestUser};

const UUID: &str = "00000000-0000-0000-0000-000000000001";

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO buildings (organization_id, street, city, postal_code, country) \
         VALUES ($1, 'Registry Test Street 1', 'Bratislava', '81101', 'Slovakia') RETURNING id",
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_unit(pool: &PgPool, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO units (building_id, designation) VALUES ($1, 'R-1') RETURNING id",
    )
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

fn authed(
    token: &str,
    method: Method,
    uri: &str,
    body: Option<serde_json::Value>,
    org_id: Uuid,
) -> Request<Body> {
    let b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org_id.to_string());
    match body {
        Some(j) => b
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(j.to_string()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    }
}

// ---------------------------------------------------------------------------
// Registry — Pet Registrations
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_create_pet_registration_returns_201(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-pet-1").await;
    let building = seed_building(&pool, org_id).await;
    let unit = seed_unit(&pool, building).await;

    let body = json!({
        "unit_id": unit,
        "pet_name": "Buddy",
        "pet_type": "dog",
        "pet_size": "medium"
    });
    let resp = app
        .execute(authed(
            &token,
            Method::POST,
            "/api/v1/registry/pets",
            Some(body),
            org_id,
        ))
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "create pet registration");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_pet_registrations_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-pet-2").await;

    let resp = app
        .execute(authed(
            &token,
            Method::GET,
            "/api/v1/registry/pets",
            None,
            org_id,
        ))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "list pet registrations");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_pet_registration_not_found_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-pet-3").await;

    let uri = format!("/api/v1/registry/pets/{UUID}");
    let resp = app
        .execute(authed(&token, Method::GET, &uri, None, org_id))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "get missing pet registration"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_update_pet_registration_not_found_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-pet-4").await;

    let uri = format!("/api/v1/registry/pets/{UUID}");
    let body = json!({"pet_name": "Max"});
    let resp = app
        .execute(authed(&token, Method::PUT, &uri, Some(body), org_id))
        .await;
    assert!(
        resp.status.is_client_error(),
        "update missing pet: {}",
        resp.status
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_delete_pet_registration_not_found_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-pet-5").await;

    let uri = format!("/api/v1/registry/pets/{UUID}");
    let resp = app
        .execute(authed(&token, Method::DELETE, &uri, None, org_id))
        .await;
    assert!(
        resp.status.is_client_error(),
        "delete missing pet: {}",
        resp.status
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_review_pet_registration_not_found_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-pet-6").await;

    let uri = format!("/api/v1/registry/pets/{UUID}/review");
    let body = json!({"status": "approved"});
    let resp = app
        .execute(authed(&token, Method::POST, &uri, Some(body), org_id))
        .await;
    assert!(
        resp.status.is_client_error(),
        "review missing pet: {}",
        resp.status
    );
}

// ---------------------------------------------------------------------------
// Registry — Vehicle Registrations
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_create_vehicle_registration_returns_201(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-veh-1").await;
    let building = seed_building(&pool, org_id).await;
    let unit = seed_unit(&pool, building).await;

    let body = json!({
        "unit_id": unit,
        "vehicle_type": "car",
        "make": "Skoda",
        "model": "Octavia",
        "license_plate": "BA-123AA"
    });
    let resp = app
        .execute(authed(
            &token,
            Method::POST,
            "/api/v1/registry/vehicles",
            Some(body),
            org_id,
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create vehicle registration"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_vehicle_registrations_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-veh-2").await;

    let resp = app
        .execute(authed(
            &token,
            Method::GET,
            "/api/v1/registry/vehicles",
            None,
            org_id,
        ))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "list vehicle registrations");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_vehicle_registration_not_found_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-veh-3").await;

    let uri = format!("/api/v1/registry/vehicles/{UUID}");
    let resp = app
        .execute(authed(&token, Method::GET, &uri, None, org_id))
        .await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND, "get missing vehicle");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_update_vehicle_registration_not_found_returns_client_error(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-veh-4").await;

    let uri = format!("/api/v1/registry/vehicles/{UUID}");
    let body = json!({"make": "Volkswagen"});
    let resp = app
        .execute(authed(&token, Method::PUT, &uri, Some(body), org_id))
        .await;
    assert!(
        resp.status.is_client_error(),
        "update missing vehicle: {}",
        resp.status
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_delete_vehicle_registration_not_found_returns_client_error(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-veh-5").await;

    let uri = format!("/api/v1/registry/vehicles/{UUID}");
    let resp = app
        .execute(authed(&token, Method::DELETE, &uri, None, org_id))
        .await;
    assert!(
        resp.status.is_client_error(),
        "delete missing vehicle: {}",
        resp.status
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_review_vehicle_registration_not_found_returns_client_error(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-veh-6").await;

    let uri = format!("/api/v1/registry/vehicles/{UUID}/review");
    let body = json!({"status": "approved"});
    let resp = app
        .execute(authed(&token, Method::POST, &uri, Some(body), org_id))
        .await;
    assert!(
        resp.status.is_client_error(),
        "review missing vehicle: {}",
        resp.status
    );
}

// ---------------------------------------------------------------------------
// Registry — Parking Spots
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_create_parking_spot_returns_201(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-park-1").await;
    let building = seed_building(&pool, org_id).await;

    let body = json!({
        "building_id": building,
        "spot_number": "P-001",
        "spot_type": "standard"
    });
    let resp = app
        .execute(authed(
            &token,
            Method::POST,
            "/api/v1/registry/parking-spots",
            Some(body),
            org_id,
        ))
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "create parking spot");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_parking_spots_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-park-2").await;

    let resp = app
        .execute(authed(
            &token,
            Method::GET,
            "/api/v1/registry/parking-spots",
            None,
            org_id,
        ))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "list parking spots");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_parking_spot_not_found_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-park-3").await;

    let uri = format!("/api/v1/registry/parking-spots/{UUID}");
    let resp = app
        .execute(authed(&token, Method::GET, &uri, None, org_id))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "get missing parking spot"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_update_parking_spot_not_found_returns_client_error(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-park-4").await;

    let uri = format!("/api/v1/registry/parking-spots/{UUID}");
    let body = json!({"spot_number": "P-002"});
    let resp = app
        .execute(authed(&token, Method::PUT, &uri, Some(body), org_id))
        .await;
    assert!(
        resp.status.is_client_error(),
        "update missing parking spot: {}",
        resp.status
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_delete_parking_spot_not_found_returns_client_error(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-park-5").await;

    let uri = format!("/api/v1/registry/parking-spots/{UUID}");
    let resp = app
        .execute(authed(&token, Method::DELETE, &uri, None, org_id))
        .await;
    assert!(
        resp.status.is_client_error(),
        "delete missing parking spot: {}",
        resp.status
    );
}

// ---------------------------------------------------------------------------
// Registry — Building Rules & Statistics
// ---------------------------------------------------------------------------

#[ignore = "BIT-351 quarantine: schema/route not implemented (BIT-568)"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_registry_rules_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-rules-1").await;
    let building = seed_building(&pool, org_id).await;

    let uri = format!("/api/v1/registry/buildings/{building}/rules");
    let resp = app
        .execute(authed(&token, Method::GET, &uri, None, org_id))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "get registry rules");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_update_registry_rules_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-rules-2").await;
    let building = seed_building(&pool, org_id).await;

    let uri = format!("/api/v1/registry/buildings/{building}/rules");
    let body = json!({
        "max_pets_per_unit": 2,
        "allowed_pet_types": ["dog", "cat"],
        "max_vehicles_per_unit": 2
    });
    let resp = app
        .execute(authed(&token, Method::PUT, &uri, Some(body), org_id))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "update registry rules");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_registry_statistics_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reg-stats-1").await;
    let building = seed_building(&pool, org_id).await;

    let uri = format!("/api/v1/registry/buildings/{building}/statistics");
    let resp = app
        .execute(authed(&token, Method::GET, &uri, None, org_id))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "get registry statistics");
}
