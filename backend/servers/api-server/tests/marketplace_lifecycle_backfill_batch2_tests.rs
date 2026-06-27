//! Behaviour-asserting success-path tests for the marketplace endpoints
//! (BIT-268 / BIT-300, integrations & ecosystem group, batch 2).
//!
//! The sibling `marketplace_voting_investor_cross_org_idor_tests.rs` covers the
//! cross-tenant *rejection* paths (404/403) for a handful of marketplace
//! handlers. This file covers the positive **success** path (200/201) for the
//! provider-profile, RFQ, quote, invitation, and verification surfaces that were
//! still marked `partial` ("real handler, no test") in the endpoint checklist.
//!
//! Auth model (mirrors the sibling file):
//!   * Marketplace handlers read `AuthUser` (JWT `sub`). Provider-owned actions
//!     (profile, quote, invitation, verification) resolve the provider via
//!     `service_provider_profiles.user_id` — no tenant header needed.
//!   * Org-scoped actions (RFQ create/list/update/quotes) call
//!     `verify_org_access(user_id, org_id)`, which checks an active
//!     `organization_members` row — so the caller is seeded as an `org_admin`.
//!   * `list_verifications` is platform-admin-gated (`role = super_admin`).
//!
//! We mint HS256 tokens with the `Claims` shape directly (rather than
//! `JwtService`, which emits the org as `org_id`, not `tenant_id`) so the
//! `tenant_id` claim lands where the extractors expect it.

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
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
    .bind(format!("MP Org {slug}"))
    .bind(format!("mp-org-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@mp-batch2.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'MP User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed a service-provider profile owned by `user_id`; return its id.
/// Provider profiles are owned by `user_id` (not org); status 'active'.
async fn seed_provider(pool: &PgPool, user_id: Uuid, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO service_provider_profiles
            (user_id, company_name, contact_name, contact_email, status)
        VALUES ($1, $2, 'Contact', $3, 'active')
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(format!("Provider {slug}"))
    .bind(format!("{slug}-{}@mp-provider.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed provider")
}

/// Seed an RFQ owned by `org_id` and return its id.
async fn seed_rfq(pool: &PgPool, org_id: Uuid, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rfqs (organization_id, created_by, title, description, service_category, status)
        VALUES ($1, $2, 'Roof repair', 'Fix the roof', 'roofing', 'sent')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed rfq")
}

/// Seed an invitation addressed to `provider_id` for `rfq_id`; return its id.
async fn seed_invitation(pool: &PgPool, rfq_id: Uuid, provider_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rfq_invitations (rfq_id, provider_id)
        VALUES ($1, $2)
        RETURNING id
        "#,
    )
    .bind(rfq_id)
    .bind(provider_id)
    .fetch_one(pool)
    .await
    .expect("seed invitation")
}

/// Claims shape matching `api_core::extractors::auth::Claims`.
#[derive(Serialize)]
struct TestClaims {
    sub: Uuid,
    exp: i64,
    iat: i64,
    token_type: String,
    tenant_id: Option<Uuid>,
    role: Option<String>,
    email: String,
    name: String,
}

fn mint_token(user_id: Uuid, email: &str, org_id: Option<Uuid>) -> String {
    mint_token_with_role(user_id, email, org_id, "manager")
}

fn mint_token_with_role(user_id: Uuid, email: &str, org_id: Option<Uuid>, role: &str) -> String {
    let now = Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: org_id,
        role: Some(role.to_string()),
        email: email.to_string(),
        name: "MP User".to_string(),
    };
    let secret = TestConfig::default().jwt_secret;
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("encode test JWT")
}

// ---------------------------------------------------------------------------
// Provider profile — create / read / update / dashboard
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_profile_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = seed_user(&pool, "create-profile@mp-batch2.test").await;
    let token = mint_token(user, "create-profile@mp-batch2.test", None);

    let resp = app
        .execute(
            app.post("/api/v1/marketplace/providers")
                .bearer(&token)
                .json(json!({
                    "company_name": "Acme Plumbing",
                    "contact_name": "Jane Doe",
                    "contact_email": "jane@acme-plumbing.test",
                    "service_categories": ["plumbing"]
                }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create_profile must return 201: {}",
        resp.text()
    );
    let body = resp.json_value();
    assert!(!body["id"].is_null(), "response must carry the new profile id");
    assert_eq!(body["company_name"], json!("Acme Plumbing"));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_my_profile_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = seed_user(&pool, "my-profile@mp-batch2.test").await;
    let provider = seed_provider(&pool, user, "my-prof").await;
    let token = mint_token(user, "my-profile@mp-batch2.test", None);

    let resp = app
        .execute(
            app.get("/api/v1/marketplace/providers/me")
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_my_profile must return 200: {}",
        resp.text()
    );
    assert_eq!(resp.json_value()["id"], json!(provider));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_my_profile_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = seed_user(&pool, "upd-profile@mp-batch2.test").await;
    seed_provider(&pool, user, "upd-prof").await;
    let token = mint_token(user, "upd-profile@mp-batch2.test", None);

    let resp = app
        .execute(
            app.patch("/api/v1/marketplace/providers/me")
                .bearer(&token)
                .json(json!({ "description": "Now with 24/7 emergency service" }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update_my_profile must return 200: {}",
        resp.text()
    );
    assert_eq!(
        resp.json_value()["description"],
        json!("Now with 24/7 emergency service")
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_provider_dashboard_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = seed_user(&pool, "dash@mp-batch2.test").await;
    seed_provider(&pool, user, "dash").await;
    let token = mint_token(user, "dash@mp-batch2.test", None);

    let resp = app
        .execute(
            app.get("/api/v1/marketplace/providers/me/dashboard")
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_provider_dashboard must return 200: {}",
        resp.text()
    );
    assert!(
        !resp.json_value()["profile"].is_null(),
        "dashboard must embed the provider profile"
    );
}

// ---------------------------------------------------------------------------
// Provider discovery — search / statistics / public reads
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn search_providers_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = seed_user(&pool, "search@mp-batch2.test").await;
    seed_provider(&pool, user, "search").await;
    let token = mint_token(user, "search@mp-batch2.test", None);

    let resp = app
        .execute(
            app.get("/api/v1/marketplace/providers?limit=10")
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "search_providers must return 200: {}",
        resp.text()
    );
    assert!(
        resp.json_value().is_array(),
        "search_providers must return an array"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_marketplace_statistics_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = seed_user(&pool, "stats@mp-batch2.test").await;
    let token = mint_token(user, "stats@mp-batch2.test", None);

    let resp = app
        .execute(
            app.get("/api/v1/marketplace/providers/statistics")
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_marketplace_statistics must return 200: {}",
        resp.text()
    );
    assert!(
        !resp.json_value()["total_providers"].is_null(),
        "statistics must report total_providers"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_provider_by_id_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let owner = seed_user(&pool, "prov-owner@mp-batch2.test").await;
    let provider = seed_provider(&pool, owner, "by-id").await;
    let reader = seed_user(&pool, "prov-reader@mp-batch2.test").await;
    let token = mint_token(reader, "prov-reader@mp-batch2.test", None);

    let resp = app
        .execute(
            app.get(&format!("/api/v1/marketplace/providers/{provider}"))
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_provider must return 200: {}",
        resp.text()
    );
    assert_eq!(resp.json_value()["id"], json!(provider));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_provider_complete_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let owner = seed_user(&pool, "complete-owner@mp-batch2.test").await;
    let provider = seed_provider(&pool, owner, "complete").await;
    let reader = seed_user(&pool, "complete-reader@mp-batch2.test").await;
    let token = mint_token(reader, "complete-reader@mp-batch2.test", None);

    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/marketplace/providers/{provider}/complete"
            ))
            .bearer(&token)
            .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_provider_complete must return 200: {}",
        resp.text()
    );
    assert!(
        !resp.json_value()["profile"].is_null(),
        "complete view must embed the profile"
    );
}

// ---------------------------------------------------------------------------
// RFQ — create / list / update / quotes-for-rfq
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_rfq_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "rfq-create").await;
    let user = seed_user(&pool, "rfq-create@mp-batch2.test").await;
    seed_membership(&pool, org, user, "org_admin").await;
    let token = mint_token(user, "rfq-create@mp-batch2.test", Some(org));

    let resp = app
        .execute(
            app.post("/api/v1/marketplace/rfqs")
                .bearer(&token)
                .json(json!({
                    "organization_id": org,
                    "title": "Lobby repaint",
                    "description": "Repaint the main lobby",
                    "service_category": "painting",
                    "provider_ids": []
                }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create_rfq must return 201: {}",
        resp.text()
    );
    let body = resp.json_value();
    assert!(!body["id"].is_null(), "response must carry the new RFQ id");
    assert_eq!(body["title"], json!("Lobby repaint"));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_rfqs_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "rfq-list").await;
    let user = seed_user(&pool, "rfq-list@mp-batch2.test").await;
    seed_membership(&pool, org, user, "org_admin").await;
    let rfq = seed_rfq(&pool, org, user).await;
    let token = mint_token(user, "rfq-list@mp-batch2.test", Some(org));

    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/marketplace/rfqs?organization_id={org}"
            ))
            .bearer(&token)
            .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_rfqs must return 200: {}",
        resp.text()
    );
    let body = resp.json_value();
    assert!(body.is_array(), "list_rfqs must return an array");
    assert!(
        body.as_array().unwrap().iter().any(|r| r["id"] == json!(rfq)),
        "seeded RFQ must appear in its org's list"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_rfq_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "rfq-update").await;
    let user = seed_user(&pool, "rfq-update@mp-batch2.test").await;
    seed_membership(&pool, org, user, "org_admin").await;
    let rfq = seed_rfq(&pool, org, user).await;
    let token = mint_token(user, "rfq-update@mp-batch2.test", Some(org));

    let resp = app
        .execute(
            app.patch(&format!("/api/v1/marketplace/rfqs/{rfq}"))
                .bearer(&token)
                .json(json!({ "title": "Roof repair (revised scope)" }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update_rfq must return 200: {}",
        resp.text()
    );
    assert_eq!(
        resp.json_value()["title"],
        json!("Roof repair (revised scope)")
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_rfq_quotes_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "rfq-quotes").await;
    let user = seed_user(&pool, "rfq-quotes@mp-batch2.test").await;
    seed_membership(&pool, org, user, "org_admin").await;
    let rfq = seed_rfq(&pool, org, user).await;
    let token = mint_token(user, "rfq-quotes@mp-batch2.test", Some(org));

    let resp = app
        .execute(
            app.get(&format!("/api/v1/marketplace/rfqs/{rfq}/quotes"))
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_rfq_quotes must return 200: {}",
        resp.text()
    );
    assert!(
        resp.json_value().is_array(),
        "list_rfq_quotes must return an array"
    );
}

// ---------------------------------------------------------------------------
// Quotes — submit / my-quotes / get
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn submit_quote_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "quote-submit").await;
    let buyer = seed_user(&pool, "quote-buyer@mp-batch2.test").await;
    seed_membership(&pool, org, buyer, "org_admin").await;
    let rfq = seed_rfq(&pool, org, buyer).await;

    let prov_user = seed_user(&pool, "quote-prov@mp-batch2.test").await;
    seed_provider(&pool, prov_user, "quote").await;
    let token = mint_token(prov_user, "quote-prov@mp-batch2.test", None);

    let resp = app
        .execute(
            app.post("/api/v1/marketplace/quotes")
                .bearer(&token)
                .json(json!({ "rfq_id": rfq, "price": "1250.00" }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "submit_quote must return 201: {}",
        resp.text()
    );
    assert_eq!(resp.json_value()["rfq_id"], json!(rfq));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_my_quotes_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let prov_user = seed_user(&pool, "myq-prov@mp-batch2.test").await;
    seed_provider(&pool, prov_user, "myq").await;
    let token = mint_token(prov_user, "myq-prov@mp-batch2.test", None);

    let resp = app
        .execute(
            app.get("/api/v1/marketplace/quotes/my?limit=10")
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_my_quotes must return 200: {}",
        resp.text()
    );
    assert!(
        resp.json_value().is_array(),
        "list_my_quotes must return an array"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_quote_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "get-quote").await;
    let buyer = seed_user(&pool, "gq-buyer@mp-batch2.test").await;
    seed_membership(&pool, org, buyer, "org_admin").await;
    let rfq = seed_rfq(&pool, org, buyer).await;

    let prov_user = seed_user(&pool, "gq-prov@mp-batch2.test").await;
    let provider = seed_provider(&pool, prov_user, "gq").await;

    // Insert a quote directly so the org member can read it back.
    let quote = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO provider_quotes (rfq_id, provider_id, price, currency, status, submitted_at)
        VALUES ($1, $2, 999.00, 'EUR', 'submitted', NOW())
        RETURNING id
        "#,
    )
    .bind(rfq)
    .bind(provider)
    .fetch_one(&pool)
    .await
    .expect("seed quote");

    let token = mint_token(buyer, "gq-buyer@mp-batch2.test", Some(org));
    let resp = app
        .execute(
            app.get(&format!("/api/v1/marketplace/quotes/{quote}"))
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_quote must return 200 for the RFQ's org member: {}",
        resp.text()
    );
    assert_eq!(resp.json_value()["id"], json!(quote));
}

// ---------------------------------------------------------------------------
// Invitations & verifications
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_my_invitations_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "inv-list").await;
    let owner = seed_user(&pool, "inv-owner@mp-batch2.test").await;
    seed_membership(&pool, org, owner, "org_admin").await;
    let rfq = seed_rfq(&pool, org, owner).await;

    let prov_user = seed_user(&pool, "inv-prov@mp-batch2.test").await;
    let provider = seed_provider(&pool, prov_user, "inv").await;
    let invitation = seed_invitation(&pool, rfq, provider).await;
    let token = mint_token(prov_user, "inv-prov@mp-batch2.test", None);

    let resp = app
        .execute(
            app.get("/api/v1/marketplace/invitations?limit=10")
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_my_invitations must return 200: {}",
        resp.text()
    );
    let body = resp.json_value();
    assert!(body.is_array(), "list_my_invitations must return an array");
    assert!(
        body.as_array()
            .unwrap()
            .iter()
            .any(|i| i["id"] == json!(invitation)),
        "the provider's own invitation must appear in the list"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn submit_verification_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let prov_user = seed_user(&pool, "ver-submit@mp-batch2.test").await;
    seed_provider(&pool, prov_user, "ver-submit").await;
    let token = mint_token(prov_user, "ver-submit@mp-batch2.test", None);

    let resp = app
        .execute(
            app.post("/api/v1/marketplace/verifications")
                .bearer(&token)
                .json(json!({
                    "verification_type": "license",
                    "document_name": "Trade License 2026.pdf"
                }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "submit_verification must return 201: {}",
        resp.text()
    );
    assert_eq!(resp.json_value()["verification_type"], json!("license"));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_verifications_as_admin_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let prov_user = seed_user(&pool, "ver-list-prov@mp-batch2.test").await;
    let provider = seed_provider(&pool, prov_user, "ver-list").await;

    // A pending verification for the admin to see in the moderation queue.
    let _ = sqlx::query(
        r#"
        INSERT INTO provider_verifications
            (provider_id, verification_type, document_name, status)
        VALUES ($1, 'license', 'Pending.pdf', 'pending')
        "#,
    )
    .bind(provider)
    .execute(&pool)
    .await
    .expect("seed verification");

    let admin = seed_user(&pool, "ver-admin@mp-batch2.test").await;
    let token = mint_token_with_role(admin, "ver-admin@mp-batch2.test", None, "super_admin");

    let resp = app
        .execute(
            app.get("/api/v1/marketplace/verifications?limit=50")
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_verifications must return 200 for a platform admin: {}",
        resp.text()
    );
    assert!(
        resp.json_value().is_array(),
        "list_verifications must return an array"
    );
}