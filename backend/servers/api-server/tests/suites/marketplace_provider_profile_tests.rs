//! Integration tests for marketplace provider-profile and search endpoints
//! (BIT-268 Wave 4 — integrations group, batch 1).
//!
//! # Coverage
//!
//! - POST /api/v1/marketplace/providers          — create_profile
//! - GET  /api/v1/marketplace/providers          — search_providers
//! - GET  /api/v1/marketplace/providers/me       — get_my_profile
//! - PATCH /api/v1/marketplace/providers/me      — update_my_profile
//! - GET  /api/v1/marketplace/providers/me/dashboard — get_provider_dashboard
//! - GET  /api/v1/marketplace/providers/statistics   — get_marketplace_statistics
//! - GET  /api/v1/marketplace/providers/{id}         — get_provider
//! - GET  /api/v1/marketplace/providers/{id}/complete — get_provider_complete
//! - POST /api/v1/marketplace/rfqs               — create_rfq
//! - GET  /api/v1/marketplace/rfqs               — list_rfqs
//! - PATCH /api/v1/marketplace/rfqs/{id}         — update_rfq
//! - DELETE /api/v1/marketplace/rfqs/{id}        — delete_rfq
//! - GET  /api/v1/marketplace/rfqs/{id}/quotes   — list_rfq_quotes
//! - GET  /api/v1/marketplace/rfqs/{id}/compare  — compare_quotes
//! - POST /api/v1/marketplace/rfqs/{id}/cancel   — cancel_rfq
//! - POST /api/v1/marketplace/quotes             — submit_quote
//! - GET  /api/v1/marketplace/quotes/my          — list_my_quotes
//! - GET  /api/v1/marketplace/quotes/{id}        — get_quote
//! - PATCH /api/v1/marketplace/quotes/{id}       — update_quote
//! - DELETE /api/v1/marketplace/quotes/{id}      — withdraw_quote
//! - GET  /api/v1/marketplace/invitations        — list_my_invitations
//! - POST /api/v1/marketplace/verifications      — submit_verification
//! - GET  /api/v1/marketplace/verifications      — list_verifications
//! - GET  /api/v1/marketplace/verifications/queue — get_verification_queue
//! - GET  /api/v1/marketplace/verifications/expiring — get_expiring_verifications
//! - GET  /api/v1/marketplace/dashboard          — get_manager_dashboard

#![allow(dead_code)]

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user, seed_membership, seed_org, TestApp, TestUser};

const JWT_SECRET: &str = "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

#[derive(Serialize)]
struct Claims {
    sub: Uuid,
    exp: i64,
    iat: i64,
    token_type: String,
    tenant_id: Option<Uuid>,
    role: Option<String>,
    email: String,
    name: String,
}

fn mint_token(user_id: Uuid, org_id: Uuid) -> String {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id,
        iat: now.timestamp(),
        exp: (now + Duration::hours(1)).timestamp(),
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some("manager".to_string()),
        email: "mp-test@test.local".to_string(),
        name: "MP Test".to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("mint token")
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'hash', 'MP Test User', 'active', NOW()) RETURNING id"#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_provider_profile(pool: &PgPool, user_id: Uuid, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO service_provider_profiles
               (user_id, company_name, contact_name, contact_email, status, service_categories, pricing_type)
           VALUES ($1, $2, 'Contact', $3, 'active', ARRAY['plumbing'], 'hourly')
           RETURNING id"#,
    )
    .bind(user_id)
    .bind(format!("Test Provider {tag}"))
    .bind(format!("{tag}@mp-test.local"))
    .fetch_one(pool)
    .await
    .expect("seed provider profile")
}

async fn seed_rfq(pool: &PgPool, org_id: Uuid, created_by: Uuid, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO rfqs (organization_id, created_by, title, description, service_category, status)
           VALUES ($1, $2, $3, 'Test description', 'plumbing', 'draft') RETURNING id"#,
    )
    .bind(org_id)
    .bind(created_by)
    .bind(format!("Test RFQ {tag}"))
    .fetch_one(pool)
    .await
    .expect("seed rfq")
}

async fn seed_quote(pool: &PgPool, rfq_id: Uuid, provider_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO provider_quotes (rfq_id, provider_id, price, currency, status)
           VALUES ($1, $2, 1000.00, 'EUR', 'submitted') RETURNING id"#,
    )
    .bind(rfq_id)
    .bind(provider_id)
    .fetch_one(pool)
    .await
    .expect("seed quote")
}

#[allow(dead_code)]
async fn seed_verification(pool: &PgPool, provider_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO provider_verifications
               (provider_id, verification_type, document_name, status)
           VALUES ($1, 'insurance', 'Test Insurance Doc', 'pending')
           RETURNING id"#,
    )
    .bind(provider_id)
    .fetch_one(pool)
    .await
    .expect("seed verification")
}

// ===========================================================================
// POST /api/v1/marketplace/providers — create_profile
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_profile_rejects_unauthenticated(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let resp = app
        .post("/api/v1/marketplace/providers")
        .json(json!({
            "company_name": "Test Co",
            "contact_name": "Owner",
            "contact_email": "owner@test.local",
            "service_categories": ["plumbing"]
        }))
        .build();
    let resp = app.execute(resp).await;
    assert!(
        matches!(
            resp.status,
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
        ),
        "unauthenticated create must be rejected; got {}",
        resp.status
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_profile_returns_201(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, _) = create_authenticated_user(&app, &user).await;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&pool)
        .await
        .expect("resolve user");

    let resp = app
        .post("/api/v1/marketplace/providers")
        .bearer(&token)
        .json(json!({
            "company_name": "Test Plumbing Co",
            "contact_name": "Owner",
            "contact_email": format!("owner-{}@test.local", user_id),
            "service_categories": ["plumbing"],
            "pricing_type": "hourly"
        }))
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create_profile must return 201; got {}: {}",
        resp.status,
        resp.text()
    );
    let body = resp.json_value();
    assert_eq!(body["company_name"], "Test Plumbing Co");
}

// ===========================================================================
// GET /api/v1/marketplace/providers — search_providers
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn search_providers_rejects_unauthenticated(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let resp = app.get("/api/v1/marketplace/providers").build();
    let resp = app.execute(resp).await;
    assert!(
        matches!(
            resp.status,
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
        ),
        "unauthenticated search must be rejected; got {}",
        resp.status
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn search_providers_returns_200_empty(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "sp-search").await;
    let user_id = seed_user(&pool, "sp-search@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .get("/api/v1/marketplace/providers")
        .bearer(&token)
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "search must return 200; got {}: {}",
        resp.status,
        resp.text()
    );
    let body = resp.json_value();
    assert!(body.is_array(), "search result must be an array");
}

// ===========================================================================
// GET /api/v1/marketplace/providers/me — get_my_profile
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_my_profile_returns_404_when_no_profile(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "gmp-none").await;
    let user_id = seed_user(&pool, "gmp-none@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .get("/api/v1/marketplace/providers/me")
        .bearer(&token)
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "no profile must return 404; got {}: {}",
        resp.status,
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_my_profile_returns_200_when_profile_exists(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "gmp-exists").await;
    let user_id = seed_user(&pool, "gmp-exists@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    seed_provider_profile(&pool, user_id, "gmp-exists").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .get("/api/v1/marketplace/providers/me")
        .bearer(&token)
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "existing profile must return 200; got {}: {}",
        resp.status,
        resp.text()
    );
    let body = resp.json_value();
    assert_eq!(body["company_name"], "Test Provider gmp-exists");
}

// ===========================================================================
// PATCH /api/v1/marketplace/providers/me — update_my_profile
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_my_profile_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "ump").await;
    let user_id = seed_user(&pool, "ump@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    seed_provider_profile(&pool, user_id, "ump").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .patch("/api/v1/marketplace/providers/me")
        .bearer(&token)
        .json(json!({"company_name": "Updated Co"}))
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update_my_profile must return 200; got {}: {}",
        resp.status,
        resp.text()
    );
    let body = resp.json_value();
    assert_eq!(body["company_name"], "Updated Co");
}

// ===========================================================================
// GET /api/v1/marketplace/providers/me/dashboard — get_provider_dashboard
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_provider_dashboard_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "gpd").await;
    let user_id = seed_user(&pool, "gpd@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    seed_provider_profile(&pool, user_id, "gpd").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .get("/api/v1/marketplace/providers/me/dashboard")
        .bearer(&token)
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_provider_dashboard must return 200; got {}: {}",
        resp.status,
        resp.text()
    );
}

// ===========================================================================
// GET /api/v1/marketplace/providers/statistics — get_marketplace_statistics
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_marketplace_statistics_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "gms").await;
    let user_id = seed_user(&pool, "gms@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .get("/api/v1/marketplace/providers/statistics")
        .bearer(&token)
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_marketplace_statistics must return 200; got {}: {}",
        resp.status,
        resp.text()
    );
    let body = resp.json_value();
    assert!(body["total_providers"].is_number());
}

// ===========================================================================
// GET /api/v1/marketplace/providers/{id} — get_provider
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_provider_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "gp").await;
    let user_id = seed_user(&pool, "gp@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let provider_id = seed_provider_profile(&pool, user_id, "gp").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .get(&format!("/api/v1/marketplace/providers/{provider_id}"))
        .bearer(&token)
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_provider must return 200; got {}: {}",
        resp.status,
        resp.text()
    );
    let body = resp.json_value();
    assert_eq!(body["id"], provider_id.to_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_provider_returns_404_for_missing(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "gp-404").await;
    let user_id = seed_user(&pool, "gp-404@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .get(&format!("/api/v1/marketplace/providers/{}", Uuid::new_v4()))
        .bearer(&token)
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND);
}

// ===========================================================================
// GET /api/v1/marketplace/providers/{id}/complete — get_provider_complete
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_provider_complete_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "gpc").await;
    let user_id = seed_user(&pool, "gpc@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let provider_id = seed_provider_profile(&pool, user_id, "gpc").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .get(&format!(
            "/api/v1/marketplace/providers/{provider_id}/complete"
        ))
        .bearer(&token)
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_provider_complete must return 200; got {}: {}",
        resp.status,
        resp.text()
    );
}

// ===========================================================================
// POST /api/v1/marketplace/rfqs — create_rfq
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_rfq_returns_201(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "crfq").await;
    let user_id = seed_user(&pool, "crfq@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .post("/api/v1/marketplace/rfqs")
        .bearer(&token)
        .json(json!({
            "organization_id": org_id,
            "title": "Plumbing repair",
            "description": "Fix the pipes",
            "service_category": "plumbing",
            "provider_ids": []
        }))
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create_rfq must return 201; got {}: {}",
        resp.status,
        resp.text()
    );
    let body = resp.json_value();
    assert_eq!(body["title"], "Plumbing repair");
    assert_eq!(body["organization_id"], org_id.to_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_rfq_idor_rejects_non_member(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "crfq-idor-a").await;
    let org_b = seed_org(&pool, "crfq-idor-b").await;
    let user_b = seed_user(&pool, "crfq-idor-b@test.local").await;
    seed_membership(&pool, org_b, user_b, "manager").await;
    let token = mint_token(user_b, org_b);

    let resp = app
        .post("/api/v1/marketplace/rfqs")
        .bearer(&token)
        .json(json!({
            "organization_id": org_a,
            "title": "IDOR RFQ",
            "description": "Should fail",
            "service_category": "plumbing",
            "provider_ids": []
        }))
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "non-member create_rfq must be rejected; got {}: {}",
        resp.status,
        resp.text()
    );
}

// ===========================================================================
// GET /api/v1/marketplace/rfqs — list_rfqs
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_rfqs_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "lrfq").await;
    let user_id = seed_user(&pool, "lrfq@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    seed_rfq(&pool, org_id, user_id, "lrfq").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .get(&format!(
            "/api/v1/marketplace/rfqs?organization_id={org_id}"
        ))
        .bearer(&token)
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_rfqs must return 200; got {}: {}",
        resp.status,
        resp.text()
    );
    let body = resp.json_value();
    assert!(body.is_array());
    assert_eq!(body.as_array().unwrap().len(), 1);
}

// ===========================================================================
// PATCH /api/v1/marketplace/rfqs/{id} — update_rfq
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_rfq_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "urfq").await;
    let user_id = seed_user(&pool, "urfq@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let rfq_id = seed_rfq(&pool, org_id, user_id, "urfq").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .patch(&format!("/api/v1/marketplace/rfqs/{rfq_id}"))
        .bearer(&token)
        .json(json!({"title": "Updated RFQ Title"}))
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update_rfq must return 200; got {}: {}",
        resp.status,
        resp.text()
    );
    let body = resp.json_value();
    assert_eq!(body["title"], "Updated RFQ Title");
}

// ===========================================================================
// DELETE /api/v1/marketplace/rfqs/{id} — delete_rfq
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_rfq_returns_204(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "drfq").await;
    let user_id = seed_user(&pool, "drfq@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let rfq_id = seed_rfq(&pool, org_id, user_id, "drfq").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .delete(&format!("/api/v1/marketplace/rfqs/{rfq_id}"))
        .bearer(&token)
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete_rfq must return 204; got {}: {}",
        resp.status,
        resp.text()
    );
}

// ===========================================================================
// GET /api/v1/marketplace/rfqs/{id}/quotes — list_rfq_quotes
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_rfq_quotes_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "lrq").await;
    let user_id = seed_user(&pool, "lrq@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let rfq_id = seed_rfq(&pool, org_id, user_id, "lrq").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .get(&format!("/api/v1/marketplace/rfqs/{rfq_id}/quotes"))
        .bearer(&token)
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_rfq_quotes must return 200; got {}: {}",
        resp.status,
        resp.text()
    );
    let body = resp.json_value();
    assert!(body.is_array());
}

// ===========================================================================
// GET /api/v1/marketplace/rfqs/{id}/compare — compare_quotes
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn compare_quotes_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "cq").await;
    let user_id = seed_user(&pool, "cq@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let rfq_id = seed_rfq(&pool, org_id, user_id, "cq").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .get(&format!("/api/v1/marketplace/rfqs/{rfq_id}/compare"))
        .bearer(&token)
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "compare_quotes must return 200; got {}: {}",
        resp.status,
        resp.text()
    );
}

// ===========================================================================
// POST /api/v1/marketplace/rfqs/{id}/cancel — cancel_rfq
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cancel_rfq_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "carfq").await;
    let user_id = seed_user(&pool, "carfq@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let rfq_id = seed_rfq(&pool, org_id, user_id, "carfq").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .post(&format!("/api/v1/marketplace/rfqs/{rfq_id}/cancel"))
        .bearer(&token)
        .json(json!({}))
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "cancel_rfq must return 200; got {}: {}",
        resp.status,
        resp.text()
    );
}

// ===========================================================================
// POST /api/v1/marketplace/quotes — submit_quote
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn submit_quote_returns_201(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "sq").await;
    let user_id = seed_user(&pool, "sq@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let rfq_id = seed_rfq(&pool, org_id, user_id, "sq").await;
    // Provider is a separate user
    let provider_user = seed_user(&pool, "sq-provider@test.local").await;
    seed_provider_profile(&pool, provider_user, "sq").await;
    let token = mint_token(provider_user, org_id);

    let resp = app
        .post("/api/v1/marketplace/quotes")
        .bearer(&token)
        .json(json!({
            "rfq_id": rfq_id,
            "price": "1500.00",
            "currency": "EUR"
        }))
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "submit_quote must return 201; got {}: {}",
        resp.status,
        resp.text()
    );
}

// ===========================================================================
// GET /api/v1/marketplace/quotes/my — list_my_quotes
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_my_quotes_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "lmq").await;
    let user_id = seed_user(&pool, "lmq@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    seed_provider_profile(&pool, user_id, "lmq").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .get("/api/v1/marketplace/quotes/my")
        .bearer(&token)
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_my_quotes must return 200; got {}: {}",
        resp.status,
        resp.text()
    );
    let body = resp.json_value();
    assert!(body.is_array());
}

// ===========================================================================
// GET /api/v1/marketplace/quotes/{id} — get_quote
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_quote_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "gq").await;
    let user_id = seed_user(&pool, "gq@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let rfq_id = seed_rfq(&pool, org_id, user_id, "gq").await;
    let provider_id = seed_provider_profile(&pool, user_id, "gq").await;
    let quote_id = seed_quote(&pool, rfq_id, provider_id).await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .get(&format!("/api/v1/marketplace/quotes/{quote_id}"))
        .bearer(&token)
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_quote must return 200; got {}: {}",
        resp.status,
        resp.text()
    );
    let body = resp.json_value();
    assert_eq!(body["id"], quote_id.to_string());
}

// ===========================================================================
// PATCH /api/v1/marketplace/quotes/{id} — update_quote
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_quote_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "uq").await;
    let user_id = seed_user(&pool, "uq@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let rfq_id = seed_rfq(&pool, org_id, user_id, "uq").await;
    let provider_id = seed_provider_profile(&pool, user_id, "uq").await;
    let quote_id = seed_quote(&pool, rfq_id, provider_id).await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .patch(&format!("/api/v1/marketplace/quotes/{quote_id}"))
        .bearer(&token)
        .json(json!({"notes": "Updated notes"}))
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update_quote must return 200; got {}: {}",
        resp.status,
        resp.text()
    );
}

// ===========================================================================
// DELETE /api/v1/marketplace/quotes/{id} — withdraw_quote
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn withdraw_quote_returns_204(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "wq").await;
    let user_id = seed_user(&pool, "wq@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let rfq_id = seed_rfq(&pool, org_id, user_id, "wq").await;
    let provider_id = seed_provider_profile(&pool, user_id, "wq").await;
    let quote_id = seed_quote(&pool, rfq_id, provider_id).await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .delete(&format!("/api/v1/marketplace/quotes/{quote_id}"))
        .bearer(&token)
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "withdraw_quote must return 204; got {}: {}",
        resp.status,
        resp.text()
    );
}

// ===========================================================================
// GET /api/v1/marketplace/invitations — list_my_invitations
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_my_invitations_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "lmi").await;
    let user_id = seed_user(&pool, "lmi@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    seed_provider_profile(&pool, user_id, "lmi").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .get("/api/v1/marketplace/invitations")
        .bearer(&token)
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_my_invitations must return 200; got {}: {}",
        resp.status,
        resp.text()
    );
    let body = resp.json_value();
    assert!(body.is_array());
}

// ===========================================================================
// POST /api/v1/marketplace/verifications — submit_verification
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn submit_verification_returns_201(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "sv").await;
    let user_id = seed_user(&pool, "sv@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    seed_provider_profile(&pool, user_id, "sv").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .post("/api/v1/marketplace/verifications")
        .bearer(&token)
        .json(json!({
            "verification_type": "insurance",
            "document_name": "Insurance Certificate 2024"
        }))
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "submit_verification must return 201; got {}: {}",
        resp.status,
        resp.text()
    );
}

// ===========================================================================
// GET /api/v1/marketplace/verifications — list_verifications
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_verifications_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "lv").await;
    let user_id = seed_user(&pool, "lv@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .get("/api/v1/marketplace/verifications")
        .bearer(&token)
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_verifications must return 200; got {}: {}",
        resp.status,
        resp.text()
    );
    let body = resp.json_value();
    assert!(body.is_array());
}

// ===========================================================================
// GET /api/v1/marketplace/verifications/queue — get_verification_queue
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_verification_queue_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "gvq").await;
    let user_id = seed_user(&pool, "gvq@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .get("/api/v1/marketplace/verifications/queue")
        .bearer(&token)
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_verification_queue must return 200; got {}: {}",
        resp.status,
        resp.text()
    );
    let body = resp.json_value();
    assert!(body.is_array());
}

// ===========================================================================
// GET /api/v1/marketplace/verifications/expiring — get_expiring_verifications
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_expiring_verifications_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "gev").await;
    let user_id = seed_user(&pool, "gev@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .get("/api/v1/marketplace/verifications/expiring")
        .bearer(&token)
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_expiring_verifications must return 200; got {}: {}",
        resp.status,
        resp.text()
    );
    let body = resp.json_value();
    assert!(body.is_array());
}

// ===========================================================================
// GET /api/v1/marketplace/dashboard — get_manager_dashboard
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_manager_dashboard_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "gmd").await;
    let user_id = seed_user(&pool, "gmd@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .get(&format!(
            "/api/v1/marketplace/dashboard?organization_id={org_id}"
        ))
        .bearer(&token)
        .build();
    let resp = app.execute(resp).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_manager_dashboard must return 200; got {}: {}",
        resp.status,
        resp.text()
    );
}
