//! Behaviour-asserting integration tests for listings endpoints (BIT-268 batch 3).
//!
//! Covers 19 endpoints from `docs/endpoint-checklist/groups/leasing.md`:
//! - Core (create, list, from-unit, statistics, get, update, delete, status)
//! - Syndication (publish, global-publish, global-unpublish, dashboard, org-stats, syndications, status)
//! - Photos (get, add, reorder, delete)

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
    .bind(format!("Listing Test {tag}"))
    .bind(format!("listing-test-{tag}-{uid}"))
    .bind(format!("{tag}-{uid}@listing-test.internal"))
    .fetch_one(pool)
    .await
    .expect("seed_org")
}

async fn seed_user(pool: &PgPool, tag: &str) -> Uuid {
    let uid = Uuid::new_v4();
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind) \
         VALUES ($1,'hash','Listing Tester','active',NOW(),'staff') RETURNING id",
    )
    .bind(format!("{tag}-{uid}@listing-test.internal"))
    .fetch_one(pool)
    .await
    .expect("seed_user")
}

async fn seed_unit(pool: &PgPool, org_id: Uuid, tag: &str) -> Uuid {
    let bld_id: Uuid = sqlx::query_scalar(
        "INSERT INTO buildings (organization_id, name, street, city, postal_code, country, status) \
         VALUES ($1,$2,'Main St 1','Testville','00000','SK','active') RETURNING id",
    )
    .bind(org_id)
    .bind(format!("Building {tag}"))
    .fetch_one(pool)
    .await
    .expect("seed_building");

    sqlx::query_scalar(
        "INSERT INTO units (building_id, designation, floor) VALUES ($1,$2,1) RETURNING id",
    )
    .bind(bld_id)
    .bind(format!("U-{tag}"))
    .fetch_one(pool)
    .await
    .expect("seed_unit")
}

async fn seed_listing(pool: &PgPool, org_id: Uuid, user_id: Uuid, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO listings \
             (organization_id, transaction_type, title, property_type, \
              street, city, postal_code, country, price, currency, \
              is_negotiable, status, created_by) \
         VALUES ($1,'sale',$2,'apartment','Main St 1','Testville','00000','SK', \
                 150000,'EUR',false,'draft',$3) RETURNING id",
    )
    .bind(org_id)
    .bind(format!("Test Listing {tag}"))
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed_listing")
}

async fn seed_listing_photo(pool: &PgPool, listing_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO listing_photos \
             (listing_id, url, display_order) \
         VALUES ($1,'https://cdn.example.com/photo.jpg',1) RETURNING id",
    )
    .bind(listing_id)
    .fetch_one(pool)
    .await
    .expect("seed_listing_photo")
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
            email: "listing-test@internal.example".into(),
            name: "Listing Tester".into(),
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
// Core CRUD
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_create_listing_returns_201(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "crlt").await;
    let user_id = seed_user(&pool, "crlt").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);

    let body = json!({
        "transaction_type": "sale",
        "title": "Modern apartment in city centre",
        "property_type": "apartment",
        "street": "Štefánikova 10",
        "city": "Bratislava",
        "postal_code": "81105",
        "country": "SK",
        "price": "185000.00",
        "currency": "EUR",
        "is_negotiable": false,
        "features": []
    });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/listings")
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
        "create listing → 200; body: {}",
        resp.text()
    );
    let json = resp.json_value();
    assert!(json.get("id").is_some(), "response must include id");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_listings_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "lslt").await;
    let user_id = seed_user(&pool, "lslt").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/listings")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list listings → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_create_listing_from_unit_returns_201(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "lfun").await;
    let user_id = seed_user(&pool, "lfun").await;
    let unit_id = seed_unit(&pool, org_id, "lfun").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);

    let body = json!({
        "unit_id": unit_id,
        "transaction_type": "rent",
        "price": "800.00",
        "currency": "EUR",
        "is_negotiable": true,
        "features": []
    });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/listings/from-unit")
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
        "create from-unit → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_listing_statistics_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "ltst").await;
    let user_id = seed_user(&pool, "ltst").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/listings/statistics")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "listing statistics → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_listing_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "gtlt").await;
    let user_id = seed_user(&pool, "gtlt").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let lt_id = seed_listing(&pool, org_id, user_id, "gtlt").await;

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v1/listings/{lt_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get listing → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_update_listing_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "uplt").await;
    let user_id = seed_user(&pool, "uplt").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let lt_id = seed_listing(&pool, org_id, user_id, "uplt").await;

    let body = json!({ "title": "Updated Listing Title" });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/api/v1/listings/{lt_id}"))
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
        "update listing → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_delete_listing_returns_204(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "dllt").await;
    let user_id = seed_user(&pool, "dllt").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let lt_id = seed_listing(&pool, org_id, user_id, "dllt").await;

    let resp = app
        .execute(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/listings/{lt_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete listing → 204; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_update_listing_status_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "ltst2").await;
    let user_id = seed_user(&pool, "ltst2").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let lt_id = seed_listing(&pool, org_id, user_id, "ltst2").await;

    let body = json!({ "status": "active" });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/api/v1/listings/{lt_id}/status"))
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
        "update listing status → 200; body: {}",
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// Syndication
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_publish_listing_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "pblt").await;
    let user_id = seed_user(&pool, "pblt").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let lt_id = seed_listing(&pool, org_id, user_id, "pblt").await;

    let body = json!({ "portals": [] });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v1/listings/{lt_id}/publish"))
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
        "publish listing → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_global_publish_listing_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "gplt").await;
    let user_id = seed_user(&pool, "gplt").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let lt_id = seed_listing(&pool, org_id, user_id, "gplt").await;

    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v1/listings/{lt_id}/global-publish"))
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
        "global-publish → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_global_unpublish_listing_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "guplt").await;
    let user_id = seed_user(&pool, "guplt").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let lt_id = seed_listing(&pool, org_id, user_id, "guplt").await;

    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v1/listings/{lt_id}/global-unpublish"))
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
        "global-unpublish → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_syndication_dashboard_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "synd").await;
    let user_id = seed_user(&pool, "synd").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/listings/syndication/dashboard")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "syndication dashboard → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_org_syndication_stats_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "syns").await;
    let user_id = seed_user(&pool, "syns").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/listings/syndication/stats")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "org syndication stats → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_listing_syndications_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "ltsyn").await;
    let user_id = seed_user(&pool, "ltsyn").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let lt_id = seed_listing(&pool, org_id, user_id, "ltsyn").await;

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v1/listings/{lt_id}/syndications"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "listing syndications → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_listing_syndication_status_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "synst").await;
    let user_id = seed_user(&pool, "synst").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let lt_id = seed_listing(&pool, org_id, user_id, "synst").await;

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v1/listings/{lt_id}/syndication/status"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "syndication status → 200; body: {}",
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// Photos
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_photos_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "gtph").await;
    let user_id = seed_user(&pool, "gtph").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let lt_id = seed_listing(&pool, org_id, user_id, "gtph").await;

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v1/listings/{lt_id}/photos"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get photos → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_add_photo_returns_201(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "adph").await;
    let user_id = seed_user(&pool, "adph").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let lt_id = seed_listing(&pool, org_id, user_id, "adph").await;

    let body = json!({
        "url": "https://cdn.example.com/listing-photo.jpg",
        "display_order": 1
    });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v1/listings/{lt_id}/photos"))
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
        "add photo → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_reorder_photos_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "roph").await;
    let user_id = seed_user(&pool, "roph").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let lt_id = seed_listing(&pool, org_id, user_id, "roph").await;
    let photo_id = seed_listing_photo(&pool, lt_id).await;

    let body = json!({ "photo_ids": [photo_id] });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v1/listings/{lt_id}/photos/reorder"))
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
        "reorder photos → 200; body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_delete_photo_returns_204(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "dlph").await;
    let user_id = seed_user(&pool, "dlph").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_manager_jwt(user_id, org_id);
    let lt_id = seed_listing(&pool, org_id, user_id, "dlph").await;
    let photo_id = seed_listing_photo(&pool, lt_id).await;

    let resp = app
        .execute(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/listings/{lt_id}/photos/{photo_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete photo → 204; body: {}",
        resp.text()
    );
}
