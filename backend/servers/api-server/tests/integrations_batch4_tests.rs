//! Behaviour-asserting integration tests for integrations/API ecosystem endpoints (BIT-268 batch 4).
//!
//! Covers 25 endpoints across three groups:
//! - Marketplace providers (create, search, get_my, update_my, get, get_complete, statistics, dashboard)
//! - Feature packages admin (list, create, get, update, delete, add_features, remove_feature,
//!   get_org_packages, assign_package, deactivate_org_package)
//! - Feature packages public (list_public, get_public, compare)
//! - Features (resolved, check, upgrade_options, log_event)

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// JWT helpers — two different claim shapes used by different route families
// ---------------------------------------------------------------------------

/// Mint a token for routes that use `AuthUser` (api-core extractor).
/// Fields: sub (UUID), email, name, exp, iat, token_type, tenant_id.
fn mint_auth_user_jwt(user_id: Uuid, tenant_id: Option<Uuid>) -> String {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::Serialize;
    #[derive(Serialize)]
    struct Claims {
        sub: Uuid,
        email: String,
        name: String,
        exp: i64,
        iat: i64,
        token_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        tenant_id: Option<Uuid>,
    }
    let now = chrono::Utc::now().timestamp();
    encode(
        &Header::default(),
        &Claims {
            sub: user_id,
            email: format!("{user_id}@test.internal"),
            name: "Test User".into(),
            exp: now + 900,
            iat: now,
            token_type: "access".into(),
            tenant_id,
        },
        &EncodingKey::from_secret(
            b"test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes",
        ),
    )
    .expect("mint_auth_user_jwt")
}

/// Mint a token for routes that use `JwtService::validate_access_token` (services/jwt Claims).
/// Fields: sub (String), email, name, exp, iat, jti, token_type, org_id, roles.
fn mint_service_jwt(user_id: Uuid, org_id: Option<Uuid>, roles: Option<Vec<&str>>) -> String {
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
        #[serde(skip_serializing_if = "Option::is_none")]
        org_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        roles: Option<Vec<String>>,
    }
    let now = chrono::Utc::now().timestamp();
    encode(
        &Header::default(),
        &Claims {
            sub: user_id.to_string(),
            email: format!("{user_id}@test.internal"),
            name: "Test User".into(),
            exp: now + 900,
            iat: now,
            jti: Uuid::new_v4().to_string(),
            token_type: "access".into(),
            org_id: org_id.map(|id| id.to_string()),
            roles: roles.map(|rs| rs.iter().map(|s| s.to_string()).collect()),
        },
        &EncodingKey::from_secret(
            b"test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes",
        ),
    )
    .expect("mint_service_jwt")
}

// ---------------------------------------------------------------------------
// Seed helpers
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, tag: &str) -> Uuid {
    let uid = Uuid::new_v4();
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO organizations (name, slug, contact_email, status) \
         VALUES ($1,$2,$3,'active') RETURNING id",
    )
    .bind(format!("Batch4 Org {tag}"))
    .bind(format!("batch4-{tag}-{uid}"))
    .bind(format!("{tag}-{uid}@batch4.internal"))
    .fetch_one(pool)
    .await
    .expect("seed_org")
}

async fn seed_user(pool: &PgPool, tag: &str) -> Uuid {
    let uid = Uuid::new_v4();
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind) \
         VALUES ($1,'hash','Batch4 User','active',NOW(),'staff') RETURNING id",
    )
    .bind(format!("{tag}-{uid}@batch4.internal"))
    .fetch_one(pool)
    .await
    .expect("seed_user")
}

async fn seed_provider_profile(pool: &PgPool, user_id: Uuid) -> Uuid {
    let uid = Uuid::new_v4();
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO service_provider_profiles \
             (user_id, company_name, contact_name, contact_email, service_categories, status) \
         VALUES ($1,$2,$3,$4,ARRAY['cleaning'],'active') RETURNING id",
    )
    .bind(user_id)
    .bind(format!("Provider Co {uid}"))
    .bind("Contact Person")
    .bind(format!("{uid}@provider.internal"))
    .fetch_one(pool)
    .await
    .expect("seed_provider_profile")
}

async fn seed_feature_package(pool: &PgPool, key_suffix: &str) -> Uuid {
    let uid = Uuid::new_v4();
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO feature_packages \
             (key, name, display_name, description, package_type, is_active, is_public) \
         VALUES ($1,$2,$3,'Test package','base',true,true) RETURNING id",
    )
    .bind(format!("pkg-{key_suffix}-{uid}"))
    .bind(format!("Package {key_suffix}"))
    .bind(format!("Package {key_suffix}"))
    .fetch_one(pool)
    .await
    .expect("seed_feature_package")
}

async fn seed_feature_flag(pool: &PgPool, key_suffix: &str) -> Uuid {
    let uid = Uuid::new_v4();
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO feature_flags (key, name, description, is_enabled) \
         VALUES ($1,$2,'test flag',true) RETURNING id",
    )
    .bind(format!("flag-{key_suffix}-{uid}"))
    .bind(format!("Flag {key_suffix}"))
    .fetch_one(pool)
    .await
    .expect("seed_feature_flag")
}

// ---------------------------------------------------------------------------
// Marketplace — Provider endpoints (8 endpoints)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_create_provider_profile_returns_201(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user_id = seed_user(&pool, "cpr").await;
    let token = mint_auth_user_jwt(user_id, None);

    let body = json!({
        "company_name": "Handy Services Ltd",
        "contact_name": "Jane Doe",
        "contact_email": "jane@handyservices.test",
        "service_categories": ["cleaning", "plumbing"],
        "pricing_type": "hourly"
    });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/marketplace/providers")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
    let body = resp.json_value();
    assert_eq!(body["company_name"], "Handy Services Ltd");
    assert!(body["id"].is_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_search_providers_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user_id = seed_user(&pool, "srch").await;
    seed_provider_profile(&pool, user_id).await;
    let token = mint_auth_user_jwt(user_id, None);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/marketplace/providers?limit=10")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert!(body.is_array());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_my_profile_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user_id = seed_user(&pool, "gmp").await;
    seed_provider_profile(&pool, user_id).await;
    let token = mint_auth_user_jwt(user_id, None);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/marketplace/providers/me")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert_eq!(body["user_id"], user_id.to_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_update_my_profile_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user_id = seed_user(&pool, "ump").await;
    seed_provider_profile(&pool, user_id).await;
    let token = mint_auth_user_jwt(user_id, None);

    let body = json!({ "description": "Updated description for provider." });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::PATCH)
                .uri("/api/v1/marketplace/providers/me")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert_eq!(body["description"], "Updated description for provider.");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_provider_by_id_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let owner = seed_user(&pool, "gpid").await;
    let profile_id = seed_provider_profile(&pool, owner).await;
    let requester = seed_user(&pool, "gpidr").await;
    let token = mint_auth_user_jwt(requester, None);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v1/marketplace/providers/{profile_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert_eq!(body["id"], profile_id.to_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_provider_complete_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let owner = seed_user(&pool, "gpc").await;
    let profile_id = seed_provider_profile(&pool, owner).await;
    let requester = seed_user(&pool, "gpcr").await;
    let token = mint_auth_user_jwt(requester, None);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/api/v1/marketplace/providers/{profile_id}/complete"
                ))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert_eq!(body["id"], profile_id.to_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_marketplace_statistics_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user_id = seed_user(&pool, "mstat").await;
    let token = mint_auth_user_jwt(user_id, None);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/marketplace/providers/statistics")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_provider_dashboard_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user_id = seed_user(&pool, "pdash").await;
    seed_provider_profile(&pool, user_id).await;
    let token = mint_auth_user_jwt(user_id, None);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/marketplace/providers/me/dashboard")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// Feature Packages — Admin endpoints (10 endpoints)
// ---------------------------------------------------------------------------

fn super_admin_token(user_id: Uuid) -> String {
    mint_service_jwt(user_id, None, Some(vec!["SuperAdministrator"]))
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_packages_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user_id = seed_user(&pool, "lpkg").await;
    seed_feature_package(&pool, "lp1").await;
    let token = super_admin_token(user_id);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/feature-packages/")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert!(body.is_array());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_create_package_returns_201(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user_id = seed_user(&pool, "cpkg").await;
    let token = super_admin_token(user_id);
    let uid = Uuid::new_v4();

    let body = json!({
        "key": format!("test-pkg-{uid}"),
        "name": "Test Package",
        "display_name": "Test Package Display",
        "description": "A test feature package",
        "package_type": "addon",
        "is_active": true,
        "is_public": true
    });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/feature-packages/")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
    let body = resp.json_value();
    assert_eq!(body["name"], "Test Package");
    assert!(body["id"].is_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_package_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user_id = seed_user(&pool, "gpkg").await;
    let pkg_id = seed_feature_package(&pool, "gp1").await;
    let token = super_admin_token(user_id);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v1/feature-packages/{pkg_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert_eq!(body["id"], pkg_id.to_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_update_package_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user_id = seed_user(&pool, "upkg").await;
    let pkg_id = seed_feature_package(&pool, "up1").await;
    let token = super_admin_token(user_id);

    let body = json!({ "description": "Updated package description." });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/api/v1/feature-packages/{pkg_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert_eq!(body["description"], "Updated package description.");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_delete_package_returns_204(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user_id = seed_user(&pool, "dpkg").await;
    let pkg_id = seed_feature_package(&pool, "dp1").await;
    let token = super_admin_token(user_id);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/feature-packages/{pkg_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::NO_CONTENT, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_add_features_to_package_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user_id = seed_user(&pool, "afpkg").await;
    let pkg_id = seed_feature_package(&pool, "af1").await;
    let flag_id = seed_feature_flag(&pool, "af1").await;
    let token = super_admin_token(user_id);

    let body = json!({ "features": [{ "feature_flag_id": flag_id }] });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v1/feature-packages/{pkg_id}/features"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_remove_feature_from_package_returns_204(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user_id = seed_user(&pool, "rfpkg").await;
    let pkg_id = seed_feature_package(&pool, "rf1").await;
    let flag_id = seed_feature_flag(&pool, "rf1").await;
    let token = super_admin_token(user_id);

    // Seed the item directly
    sqlx::query("INSERT INTO feature_package_items (package_id, feature_flag_id) VALUES ($1,$2)")
        .bind(pkg_id)
        .bind(flag_id)
        .execute(&pool)
        .await
        .expect("seed package item");

    let resp = app
        .execute(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!(
                    "/api/v1/feature-packages/{pkg_id}/features/{flag_id}"
                ))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::NO_CONTENT, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_org_packages_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user_id = seed_user(&pool, "gopkg").await;
    let org_id = seed_org(&pool, "gopkg").await;
    let token = super_admin_token(user_id);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v1/feature-packages/organizations/{org_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert!(body.is_array());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_assign_package_to_org_returns_201(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user_id = seed_user(&pool, "apkg").await;
    let org_id = seed_org(&pool, "apkg").await;
    let pkg_id = seed_feature_package(&pool, "ap1").await;
    let token = super_admin_token(user_id);

    let body = json!({ "package_id": pkg_id, "source": "manual" });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/feature-packages/organizations/{org_id}/assign"
                ))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
    let body = resp.json_value();
    assert_eq!(body["organization_id"], org_id.to_string());
    assert_eq!(body["package_id"], pkg_id.to_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_deactivate_org_package_returns_204(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user_id = seed_user(&pool, "dop").await;
    let org_id = seed_org(&pool, "dop").await;
    let pkg_id = seed_feature_package(&pool, "dop1").await;
    let token = super_admin_token(user_id);

    // Seed an organization_package
    let org_pkg_id: Uuid = sqlx::query_scalar(
        "INSERT INTO organization_packages (organization_id, package_id, source, is_active) \
         VALUES ($1,$2,'manual',true) RETURNING id",
    )
    .bind(org_id)
    .bind(pkg_id)
    .fetch_one(&pool)
    .await
    .expect("seed org package");

    let resp = app
        .execute(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!(
                    "/api/v1/feature-packages/organizations/{org_id}/packages/{org_pkg_id}"
                ))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::NO_CONTENT, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// Feature Packages — Public endpoints (3 endpoints)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_public_packages_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    seed_feature_package(&pool, "pub1").await;

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/feature-packages/public/")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert!(body.is_array());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_public_package_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let pkg_id = seed_feature_package(&pool, "pub2").await;

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v1/feature-packages/public/{pkg_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert_eq!(body["id"], pkg_id.to_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_compare_packages_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let pkg_a = seed_feature_package(&pool, "cmp1").await;
    let pkg_b = seed_feature_package(&pool, "cmp2").await;

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/api/v1/feature-packages/public/compare?ids={pkg_a},{pkg_b}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// Features endpoints (4 endpoints)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_resolved_features_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user_id = seed_user(&pool, "resf").await;
    let org_id = seed_org(&pool, "resf").await;
    let token = mint_service_jwt(user_id, Some(org_id), None);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/features/resolved")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert!(body["features"].is_array());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_check_feature_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user_id = seed_user(&pool, "chkf").await;
    let org_id = seed_org(&pool, "chkf").await;
    let token = mint_service_jwt(user_id, Some(org_id), None);

    // "new_dashboard" is seeded by migration 00030
    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/features/new_dashboard/check")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert_eq!(body["key"], "new_dashboard");
    assert!(body["is_enabled"].is_boolean());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_upgrade_options_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user_id = seed_user(&pool, "upgf").await;
    let org_id = seed_org(&pool, "upgf").await;
    let token = mint_service_jwt(user_id, Some(org_id), None);

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/features/new_dashboard/upgrade-options")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert_eq!(body["feature_key"], "new_dashboard");
    assert!(body["packages"].is_array());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_log_feature_event_returns_200(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user_id = seed_user(&pool, "levt").await;
    let org_id = seed_org(&pool, "levt").await;
    let token = mint_service_jwt(user_id, Some(org_id), None);

    let body = json!({
        "feature_key": "new_dashboard",
        "event_type": "access",
        "metadata": {}
    });

    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/features/analytics/event")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert_eq!(body["success"], true);
}
