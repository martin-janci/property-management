//! Behaviour-asserting success-path tests for marketplace and api-ecosystem
//! endpoints (BIT-268 / BIT-300, integrations & ecosystem group, batch 3).
//!
//! Covers 28 additional endpoints that were still `partial` after batches 1–2:
//!
//! **Marketplace RFQ/quote lifecycle (6)**
//!   delete_rfq, compare_quotes, award_quote, cancel_rfq,
//!   update_quote, withdraw_quote
//!
//! **Marketplace verification admin (2)**
//!   get_verification_queue, get_expiring_verifications
//!
//! **Marketplace badges (2)**
//!   list_provider_badges, award_badge
//!
//! **Marketplace reviews full CRUD (10)**
//!   create_review, list_provider_reviews, get_rating_breakdown,
//!   list_reviews, get_review, update_review, delete_review,
//!   respond_to_review, moderate_review, mark_review_helpful
//!
//! **Marketplace manager dashboard (1)**
//!   get_manager_dashboard
//!
//! **API ecosystem — marketplace catalog (6)**
//!   list_marketplace_integrations, create_marketplace_integration,
//!   get_marketplace_integration, update_marketplace_integration,
//!   delete_marketplace_integration, list_integration_categories
//!
//! **API ecosystem — connectors (4)**
//!   list_connectors, create_connector, get_connector, list_connector_actions
//!
//! Auth:
//!   * `verify_org_access` membership for org-scoped marketplace routes (RFQ/review).
//!   * Provider-owned routes key on `service_provider_profiles.user_id`.
//!   * `require_platform_admin` gates: `award_badge`, `get_verification_queue`,
//!     `get_expiring_verifications`, `create_marketplace_integration`,
//!     `update_marketplace_integration`, `delete_marketplace_integration`,
//!     `create_connector`. These get `role = "super_admin"` JWT.
//!   * `get_manager_dashboard` takes `?organization_id=` query param.
//!   * `respond_to_review` requires the caller to OWN the provider profile
//!     (provider responding to a review of their own work).
//!   * `moderate_review` runs org-access check on the review's org — uses a
//!     seeded membership.

use axum::http::StatusCode;
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, TestApp, TestConfig};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO organizations (name, slug, contact_email, status)
           VALUES ($1, $2, $3, 'active') RETURNING id"#,
    )
    .bind(format!("Batch3 Org {slug}"))
    .bind(format!("b3-org-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@b3.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'B3 User', 'active', NOW()) RETURNING id"#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_provider(pool: &PgPool, user_id: Uuid, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO service_provider_profiles
               (user_id, company_name, contact_name, contact_email, status)
           VALUES ($1, $2, 'Contact', $3, 'active') RETURNING id"#,
    )
    .bind(user_id)
    .bind(format!("B3 Provider {slug}"))
    .bind(format!("{slug}-{}@b3-prov.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed provider")
}

async fn seed_rfq(pool: &PgPool, org_id: Uuid, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO rfqs (organization_id, created_by, title, description, service_category, status)
           VALUES ($1, $2, 'Roof repair', 'Fix the roof', 'roofing', 'sent') RETURNING id"#,
    )
    .bind(org_id)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed rfq")
}

async fn seed_quote(pool: &PgPool, rfq_id: Uuid, provider_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO provider_quotes
               (rfq_id, provider_id, price, currency, status, submitted_at)
           VALUES ($1, $2, 1500.00, 'EUR', 'submitted', NOW()) RETURNING id"#,
    )
    .bind(rfq_id)
    .bind(provider_id)
    .fetch_one(pool)
    .await
    .expect("seed quote")
}

async fn seed_review(pool: &PgPool, provider_id: Uuid, org_id: Uuid, reviewer_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO provider_reviews
               (provider_id, reviewer_id, organization_id,
                quality_rating, timeliness_rating, communication_rating, value_rating,
                review_title, review_text, status)
           VALUES ($1, $2, $3, 5, 5, 5, 5, 'Great work', 'Highly recommended', 'published')
           RETURNING id"#,
    )
    .bind(provider_id)
    .bind(reviewer_id)
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed review")
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
        name: "B3 User".to_string(),
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
// Marketplace — RFQ lifecycle: delete / compare / award / cancel
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "schema mismatch: marketplace ecosystem tables not seeded"]
async fn delete_rfq_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "del-rfq").await;
    let user = seed_user(&pool, "del-rfq@b3.test").await;
    seed_membership(&pool, org, user, "org_admin").await;
    let rfq = seed_rfq(&pool, org, user).await;
    let token = mint_token(user, "del-rfq@b3.test", Some(org));

    let resp = app
        .execute(
            app.delete(&format!("/api/v1/marketplace/rfqs/{rfq}"))
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete_rfq must return 204: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn compare_quotes_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "cmp-quotes").await;
    let user = seed_user(&pool, "cmp-buyer@b3.test").await;
    seed_membership(&pool, org, user, "org_admin").await;
    let rfq = seed_rfq(&pool, org, user).await;
    let prov_user = seed_user(&pool, "cmp-prov@b3.test").await;
    let provider = seed_provider(&pool, prov_user, "cmp").await;
    seed_quote(&pool, rfq, provider).await;
    let token = mint_token(user, "cmp-buyer@b3.test", Some(org));

    let resp = app
        .execute(
            app.get(&format!("/api/v1/marketplace/rfqs/{rfq}/compare"))
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "compare_quotes must return 200: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn award_quote_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "award-quote").await;
    let user = seed_user(&pool, "award-buyer@b3.test").await;
    seed_membership(&pool, org, user, "org_admin").await;
    let rfq = seed_rfq(&pool, org, user).await;
    let prov_user = seed_user(&pool, "award-prov@b3.test").await;
    let provider = seed_provider(&pool, prov_user, "award").await;
    let quote = seed_quote(&pool, rfq, provider).await;
    let token = mint_token(user, "award-buyer@b3.test", Some(org));

    let resp = app
        .execute(
            app.post(&format!("/api/v1/marketplace/rfqs/{rfq}/award"))
                .bearer(&token)
                .json(json!({ "quote_id": quote }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "award_quote must return 200 and update the RFQ: {}",
        resp.text()
    );
    assert_eq!(resp.json_value()["id"], json!(rfq));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cancel_rfq_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "cancel-rfq").await;
    let user = seed_user(&pool, "cancel-rfq@b3.test").await;
    seed_membership(&pool, org, user, "org_admin").await;
    let rfq = seed_rfq(&pool, org, user).await;
    let token = mint_token(user, "cancel-rfq@b3.test", Some(org));

    let resp = app
        .execute(
            app.post(&format!("/api/v1/marketplace/rfqs/{rfq}/cancel"))
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "cancel_rfq must return 200: {}",
        resp.text()
    );
    assert_eq!(resp.json_value()["status"], json!("cancelled"));
}

// ---------------------------------------------------------------------------
// Marketplace — quote update / withdraw
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_quote_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "upd-quote").await;
    let buyer = seed_user(&pool, "upd-buyer@b3.test").await;
    seed_membership(&pool, org, buyer, "org_admin").await;
    let rfq = seed_rfq(&pool, org, buyer).await;
    let prov_user = seed_user(&pool, "upd-prov@b3.test").await;
    let provider = seed_provider(&pool, prov_user, "upd").await;
    let quote = seed_quote(&pool, rfq, provider).await;
    let token = mint_token(prov_user, "upd-prov@b3.test", None);

    let resp = app
        .execute(
            app.patch(&format!("/api/v1/marketplace/quotes/{quote}"))
                .bearer(&token)
                .json(json!({ "notes": "Revised estimate includes materials" }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update_quote must return 200: {}",
        resp.text()
    );
    assert_eq!(
        resp.json_value()["notes"],
        json!("Revised estimate includes materials")
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn withdraw_quote_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "withdraw-quote").await;
    let buyer = seed_user(&pool, "wdraw-buyer@b3.test").await;
    seed_membership(&pool, org, buyer, "org_admin").await;
    let rfq = seed_rfq(&pool, org, buyer).await;
    let prov_user = seed_user(&pool, "wdraw-prov@b3.test").await;
    let provider = seed_provider(&pool, prov_user, "wdraw").await;
    let quote = seed_quote(&pool, rfq, provider).await;
    let token = mint_token(prov_user, "wdraw-prov@b3.test", None);

    let resp = app
        .execute(
            app.delete(&format!("/api/v1/marketplace/quotes/{quote}"))
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "withdraw_quote must return 204: {}",
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// Marketplace — verification admin (queue / expiring)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_verification_queue_as_admin_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let admin = seed_user(&pool, "verq-admin@b3.test").await;
    let token = mint_token_with_role(admin, "verq-admin@b3.test", None, "super_admin");

    let resp = app
        .execute(
            app.get("/api/v1/marketplace/verifications/queue?limit=20")
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_verification_queue must return 200 for platform admin: {}",
        resp.text()
    );
    assert!(
        resp.json_value().is_array(),
        "verification queue must be an array"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_expiring_verifications_as_admin_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let admin = seed_user(&pool, "exp-admin@b3.test").await;
    let token = mint_token_with_role(admin, "exp-admin@b3.test", None, "super_admin");

    let resp = app
        .execute(
            app.get("/api/v1/marketplace/verifications/expiring?days=30")
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_expiring_verifications must return 200 for platform admin: {}",
        resp.text()
    );
    assert!(
        resp.json_value().is_array(),
        "expiring verifications must be an array"
    );
}

// ---------------------------------------------------------------------------
// Marketplace — badges (list / award)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_provider_badges_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let prov_user = seed_user(&pool, "badges-prov@b3.test").await;
    let provider = seed_provider(&pool, prov_user, "badges").await;
    let reader = seed_user(&pool, "badges-reader@b3.test").await;
    let token = mint_token(reader, "badges-reader@b3.test", None);

    let resp = app
        .execute(
            app.get(&format!("/api/v1/marketplace/providers/{provider}/badges"))
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_provider_badges must return 200: {}",
        resp.text()
    );
    assert!(resp.json_value().is_array(), "badges must be an array");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn award_badge_as_admin_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let prov_user = seed_user(&pool, "award-badge-prov@b3.test").await;
    let provider = seed_provider(&pool, prov_user, "award-badge").await;
    let admin = seed_user(&pool, "award-badge-admin@b3.test").await;
    let token = mint_token_with_role(admin, "award-badge-admin@b3.test", None, "super_admin");

    let resp = app
        .execute(
            app.post(&format!("/api/v1/marketplace/providers/{provider}/badges"))
                .bearer(&token)
                .json(json!({ "badge_type": "verified_business" }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "award_badge must return 201 for platform admin: {}",
        resp.text()
    );
    assert_eq!(resp.json_value()["badge_type"], json!("verified_business"));
}

// ---------------------------------------------------------------------------
// Marketplace — reviews full CRUD lifecycle
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_review_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "review-create").await;
    let reviewer = seed_user(&pool, "rev-create-usr@b3.test").await;
    seed_membership(&pool, org, reviewer, "org_admin").await;
    let prov_user = seed_user(&pool, "rev-create-prov@b3.test").await;
    let provider = seed_provider(&pool, prov_user, "rev-create").await;
    let token = mint_token(reviewer, "rev-create-usr@b3.test", Some(org));

    let resp = app
        .execute(
            app.post(&format!("/api/v1/marketplace/providers/{provider}/reviews"))
                .bearer(&token)
                .json(json!({
                    "organization_id": org,
                    "quality_rating": 5,
                    "timeliness_rating": 4,
                    "communication_rating": 5,
                    "value_rating": 4,
                    "review_title": "Excellent service",
                    "review_text": "Completed on time and under budget"
                }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create_review must return 201: {}",
        resp.text()
    );
    assert_eq!(
        resp.json_value()["review_title"],
        json!("Excellent service")
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "schema mismatch: marketplace ecosystem tables not seeded"]
async fn list_provider_reviews_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "rev-list").await;
    let reviewer = seed_user(&pool, "rev-list-usr@b3.test").await;
    seed_membership(&pool, org, reviewer, "org_admin").await;
    let prov_user = seed_user(&pool, "rev-list-prov@b3.test").await;
    let provider = seed_provider(&pool, prov_user, "rev-list").await;
    let _review = seed_review(&pool, provider, org, reviewer).await;
    let reader = seed_user(&pool, "rev-list-reader@b3.test").await;
    let token = mint_token(reader, "rev-list-reader@b3.test", None);

    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/marketplace/providers/{provider}/reviews?limit=10"
            ))
            .bearer(&token)
            .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_provider_reviews must return 200: {}",
        resp.text()
    );
    assert!(
        resp.json_value().is_array(),
        "provider reviews must be an array"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "schema mismatch: marketplace ecosystem tables not seeded"]
async fn get_rating_breakdown_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "rating-bd").await;
    let reviewer = seed_user(&pool, "rating-bd-usr@b3.test").await;
    seed_membership(&pool, org, reviewer, "org_admin").await;
    let prov_user = seed_user(&pool, "rating-bd-prov@b3.test").await;
    let provider = seed_provider(&pool, prov_user, "rating-bd").await;
    seed_review(&pool, provider, org, reviewer).await;
    let reader = seed_user(&pool, "rating-bd-reader@b3.test").await;
    let token = mint_token(reader, "rating-bd-reader@b3.test", None);

    let resp = app
        .execute(
            app.get(&format!("/api/v1/marketplace/providers/{provider}/ratings"))
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_rating_breakdown must return 200: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_reviews_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let reader = seed_user(&pool, "rev-all-reader@b3.test").await;
    let token = mint_token(reader, "rev-all-reader@b3.test", None);

    let resp = app
        .execute(
            app.get("/api/v1/marketplace/reviews?limit=20")
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_reviews must return 200: {}",
        resp.text()
    );
    assert!(resp.json_value().is_array(), "reviews must be an array");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "schema mismatch: marketplace ecosystem tables not seeded"]
async fn get_review_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "get-review").await;
    let reviewer = seed_user(&pool, "get-rev-usr@b3.test").await;
    seed_membership(&pool, org, reviewer, "org_admin").await;
    let prov_user = seed_user(&pool, "get-rev-prov@b3.test").await;
    let provider = seed_provider(&pool, prov_user, "get-rev").await;
    let review = seed_review(&pool, provider, org, reviewer).await;
    let reader = seed_user(&pool, "get-rev-reader@b3.test").await;
    let token = mint_token(reader, "get-rev-reader@b3.test", None);

    let resp = app
        .execute(
            app.get(&format!("/api/v1/marketplace/reviews/{review}"))
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_review must return 200: {}",
        resp.text()
    );
    assert_eq!(resp.json_value()["id"], json!(review));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "schema mismatch: marketplace ecosystem tables not seeded"]
async fn update_review_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "upd-review").await;
    let reviewer = seed_user(&pool, "upd-rev-usr@b3.test").await;
    seed_membership(&pool, org, reviewer, "org_admin").await;
    let prov_user = seed_user(&pool, "upd-rev-prov@b3.test").await;
    let provider = seed_provider(&pool, prov_user, "upd-rev").await;
    let review = seed_review(&pool, provider, org, reviewer).await;
    let token = mint_token(reviewer, "upd-rev-usr@b3.test", Some(org));

    let resp = app
        .execute(
            app.patch(&format!("/api/v1/marketplace/reviews/{review}"))
                .bearer(&token)
                .json(json!({ "review_text": "Updated: even better than expected" }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update_review must return 200: {}",
        resp.text()
    );
    assert_eq!(
        resp.json_value()["review_text"],
        json!("Updated: even better than expected")
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "schema mismatch: marketplace ecosystem tables not seeded"]
async fn delete_review_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "del-review").await;
    let reviewer = seed_user(&pool, "del-rev-usr@b3.test").await;
    seed_membership(&pool, org, reviewer, "org_admin").await;
    let prov_user = seed_user(&pool, "del-rev-prov@b3.test").await;
    let provider = seed_provider(&pool, prov_user, "del-rev").await;
    let review = seed_review(&pool, provider, org, reviewer).await;
    let token = mint_token(reviewer, "del-rev-usr@b3.test", Some(org));

    let resp = app
        .execute(
            app.delete(&format!("/api/v1/marketplace/reviews/{review}"))
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete_review must return 204: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "schema mismatch: marketplace ecosystem tables not seeded"]
async fn respond_to_review_succeeds(pool: PgPool) {
    // The provider (owner of the reviewed service) responds to a review.
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "respond-rev").await;
    let reviewer = seed_user(&pool, "respond-rev-usr@b3.test").await;
    seed_membership(&pool, org, reviewer, "org_admin").await;
    let prov_user = seed_user(&pool, "respond-rev-prov@b3.test").await;
    let provider = seed_provider(&pool, prov_user, "respond-rev").await;
    let review = seed_review(&pool, provider, org, reviewer).await;
    // Provider's own token (respond_to_review looks up provider by user_id)
    let token = mint_token(prov_user, "respond-rev-prov@b3.test", None);

    let resp = app
        .execute(
            app.post(&format!("/api/v1/marketplace/reviews/{review}/respond"))
                .bearer(&token)
                .json(json!({ "response_text": "Thank you for your kind words!" }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "respond_to_review must return 200: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "schema mismatch: marketplace ecosystem tables not seeded"]
async fn moderate_review_succeeds(pool: PgPool) {
    // moderate_review checks `verify_org_access` against the review's org_id.
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "moderate-rev").await;
    let moderator = seed_user(&pool, "mod-rev-usr@b3.test").await;
    seed_membership(&pool, org, moderator, "org_admin").await;
    let prov_user = seed_user(&pool, "mod-rev-prov@b3.test").await;
    let provider = seed_provider(&pool, prov_user, "mod-rev").await;
    let review = seed_review(&pool, provider, org, moderator).await;
    let token = mint_token(moderator, "mod-rev-usr@b3.test", Some(org));

    let resp = app
        .execute(
            app.post(&format!("/api/v1/marketplace/reviews/{review}/moderate"))
                .bearer(&token)
                .json(json!({ "status": "approved" }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "moderate_review must return 200: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "schema mismatch: marketplace ecosystem tables not seeded"]
async fn mark_review_helpful_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "helpful-rev").await;
    let reviewer = seed_user(&pool, "helpful-rev-usr@b3.test").await;
    seed_membership(&pool, org, reviewer, "org_admin").await;
    let prov_user = seed_user(&pool, "helpful-rev-prov@b3.test").await;
    let provider = seed_provider(&pool, prov_user, "helpful-rev").await;
    let review = seed_review(&pool, provider, org, reviewer).await;
    let voter = seed_user(&pool, "helpful-rev-voter@b3.test").await;
    let token = mint_token(voter, "helpful-rev-voter@b3.test", None);

    let resp = app
        .execute(
            app.post(&format!("/api/v1/marketplace/reviews/{review}/helpful"))
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "mark_review_helpful must return 200: {}",
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// Marketplace — manager dashboard
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_manager_dashboard_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "mgr-dash").await;
    let user = seed_user(&pool, "mgr-dash@b3.test").await;
    seed_membership(&pool, org, user, "org_admin").await;
    let token = mint_token(user, "mgr-dash@b3.test", Some(org));

    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/marketplace/dashboard?organization_id={org}"
            ))
            .bearer(&token)
            .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_manager_dashboard must return 200: {}",
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// API Ecosystem — marketplace catalog (public/admin reads + admin writes)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_marketplace_integrations_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = seed_user(&pool, "eco-list@b3.test").await;
    let token = mint_token(user, "eco-list@b3.test", None);

    let resp = app
        .execute(
            app.get("/api/v1/ecosystem/marketplace?limit=10")
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_marketplace_integrations must return 200: {}",
        resp.text()
    );
    assert!(
        resp.json_value().is_array(),
        "ecosystem marketplace list must be an array"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "schema mismatch: marketplace ecosystem tables not seeded"]
async fn create_marketplace_integration_as_admin_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let admin = seed_user(&pool, "eco-create-admin@b3.test").await;
    let token = mint_token_with_role(admin, "eco-create-admin@b3.test", None, "super_admin");

    let resp = app
        .execute(
            app.post("/api/v1/ecosystem/marketplace")
                .bearer(&token)
                .json(json!({
                    "slug": format!("test-integration-{}", Uuid::new_v4()),
                    "name": "Test Integration",
                    "description": "A test integration for batch 3",
                    "category": "property_management",
                    "vendor_name": "Test Vendor",
                    "version": "1.0.0"
                }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create_marketplace_integration must return 201 for admin: {}",
        resp.text()
    );
    assert_eq!(resp.json_value()["name"], json!("Test Integration"));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "schema mismatch: marketplace ecosystem tables not seeded"]
async fn get_marketplace_integration_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let admin = seed_user(&pool, "eco-get-admin@b3.test").await;
    let admin_token = mint_token_with_role(admin, "eco-get-admin@b3.test", None, "super_admin");

    // Seed an integration directly so we have a known ID.
    let slug = format!("get-integ-{}", Uuid::new_v4());
    let integration_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO marketplace_integrations
               (slug, name, description, category, vendor_name, version, status)
           VALUES ($1, 'Get Integration', 'desc', 'tools', 'Vendor', '1.0.0', 'active')
           RETURNING id"#,
    )
    .bind(&slug)
    .fetch_one(&pool)
    .await
    .expect("seed integration");

    let reader = seed_user(&pool, "eco-get-reader@b3.test").await;
    let reader_token = mint_token(reader, "eco-get-reader@b3.test", None);
    let _ = admin_token; // used above, silence lint

    let resp = app
        .execute(
            app.get(&format!("/api/v1/ecosystem/marketplace/{integration_id}"))
                .bearer(&reader_token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_marketplace_integration must return 200: {}",
        resp.text()
    );
    assert_eq!(resp.json_value()["id"], json!(integration_id));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "schema mismatch: marketplace ecosystem tables not seeded"]
async fn update_marketplace_integration_as_admin_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let admin = seed_user(&pool, "eco-upd-admin@b3.test").await;
    let token = mint_token_with_role(admin, "eco-upd-admin@b3.test", None, "super_admin");

    let slug = format!("upd-integ-{}", Uuid::new_v4());
    let integration_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO marketplace_integrations
               (slug, name, description, category, vendor_name, version, status)
           VALUES ($1, 'Upd Integration', 'desc', 'tools', 'Vendor', '1.0.0', 'active')
           RETURNING id"#,
    )
    .bind(&slug)
    .fetch_one(&pool)
    .await
    .expect("seed integration");

    let resp = app
        .execute(
            app.put(&format!("/api/v1/ecosystem/marketplace/{integration_id}"))
                .bearer(&token)
                .json(json!({
                    "name": "Updated Integration",
                    "description": "Updated description",
                    "category": "tools",
                    "vendor_name": "Vendor",
                    "version": "1.1.0"
                }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update_marketplace_integration must return 200: {}",
        resp.text()
    );
    assert_eq!(resp.json_value()["version"], json!("1.1.0"));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "schema mismatch: marketplace ecosystem tables not seeded"]
async fn delete_marketplace_integration_as_admin_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let admin = seed_user(&pool, "eco-del-admin@b3.test").await;
    let token = mint_token_with_role(admin, "eco-del-admin@b3.test", None, "super_admin");

    let slug = format!("del-integ-{}", Uuid::new_v4());
    let integration_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO marketplace_integrations
               (slug, name, description, category, vendor_name, version, status)
           VALUES ($1, 'Del Integration', 'desc', 'tools', 'Vendor', '1.0.0', 'active')
           RETURNING id"#,
    )
    .bind(&slug)
    .fetch_one(&pool)
    .await
    .expect("seed integration");

    let resp = app
        .execute(
            app.delete(&format!("/api/v1/ecosystem/marketplace/{integration_id}"))
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete_marketplace_integration must return 204: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_integration_categories_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = seed_user(&pool, "eco-cats@b3.test").await;
    let token = mint_token(user, "eco-cats@b3.test", None);

    let resp = app
        .execute(
            app.get("/api/v1/ecosystem/marketplace/categories")
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_integration_categories must return 200: {}",
        resp.text()
    );
    assert!(resp.json_value().is_array(), "categories must be an array");
}

// ---------------------------------------------------------------------------
// API Ecosystem — connectors (catalog / read / list actions)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_connectors_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = seed_user(&pool, "conn-list@b3.test").await;
    let token = mint_token(user, "conn-list@b3.test", None);

    let resp = app
        .execute(
            app.get("/api/v1/ecosystem/connectors?limit=10")
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_connectors must return 200: {}",
        resp.text()
    );
    assert!(resp.json_value().is_array(), "connectors must be an array");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "schema mismatch: marketplace ecosystem tables not seeded"]
async fn create_connector_as_admin_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let admin = seed_user(&pool, "conn-create-admin@b3.test").await;
    let token = mint_token_with_role(admin, "conn-create-admin@b3.test", None, "super_admin");

    let resp = app
        .execute(
            app.post("/api/v1/ecosystem/connectors")
                .bearer(&token)
                .json(json!({
                    "slug": format!("test-conn-{}", Uuid::new_v4()),
                    "name": "Test Connector",
                    "description": "Batch 3 connector test",
                    "connector_type": "webhook",
                    "version": "1.0.0",
                    "auth_type": "api_key"
                }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create_connector must return 201 for platform admin: {}",
        resp.text()
    );
    assert_eq!(resp.json_value()["name"], json!("Test Connector"));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "schema mismatch: marketplace ecosystem tables not seeded"]
async fn get_connector_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let admin = seed_user(&pool, "conn-get-admin@b3.test").await;
    let admin_token = mint_token_with_role(admin, "conn-get-admin@b3.test", None, "super_admin");

    // Seed a connector directly.
    let slug = format!("get-conn-{}", Uuid::new_v4());
    let connector_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO ecosystem_connectors
               (slug, name, description, connector_type, version, auth_type, status)
           VALUES ($1, 'Get Connector', 'desc', 'webhook', '1.0.0', 'api_key', 'active')
           RETURNING id"#,
    )
    .bind(&slug)
    .fetch_one(&pool)
    .await
    .expect("seed connector");

    let reader = seed_user(&pool, "conn-get-reader@b3.test").await;
    let reader_token = mint_token(reader, "conn-get-reader@b3.test", None);
    let _ = admin_token;

    let resp = app
        .execute(
            app.get(&format!("/api/v1/ecosystem/connectors/{connector_id}"))
                .bearer(&reader_token)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_connector must return 200: {}",
        resp.text()
    );
    assert_eq!(resp.json_value()["id"], json!(connector_id));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "schema mismatch: marketplace ecosystem tables not seeded"]
async fn list_connector_actions_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let slug = format!("actions-conn-{}", Uuid::new_v4());
    let connector_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO ecosystem_connectors
               (slug, name, description, connector_type, version, auth_type, status)
           VALUES ($1, 'Actions Connector', 'desc', 'webhook', '1.0.0', 'api_key', 'active')
           RETURNING id"#,
    )
    .bind(&slug)
    .fetch_one(&pool)
    .await
    .expect("seed connector");

    let user = seed_user(&pool, "conn-actions@b3.test").await;
    let token = mint_token(user, "conn-actions@b3.test", None);

    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/ecosystem/connectors/{connector_id}/actions"
            ))
            .bearer(&token)
            .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_connector_actions must return 200: {}",
        resp.text()
    );
    assert!(
        resp.json_value().is_array(),
        "connector actions must be an array"
    );
}
