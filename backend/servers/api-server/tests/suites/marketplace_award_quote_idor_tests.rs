//! Regression tests for the cross-RFQ / cross-tenant IDOR on the marketplace
//! "award quote" endpoint (`POST /api/v1/marketplace/rfqs/{id}/award`).
//!
//! Audit history: `award_quote` authorised ONLY the parent RFQ path param
//! `{id}` via `load_rfq_for_org(...)`, but the quote being awarded is taken
//! from the request body (`AwardQuoteRequest.quote_id`) and was passed straight
//! to `award_rfq(id, quote_id)` WITHOUT verifying the quote actually belongs to
//! that RFQ. A caller who legitimately owns RFQ A could therefore award RFQ A
//! to a quote submitted against RFQ B — including an RFQ owned by a different
//! organization — by guessing/knowing the foreign quote id. That leaks the
//! foreign provider onto the award and mutates the foreign quote's lifecycle.
//!
//! The fix threads a quote-ownership check into the handler (reject with 404
//! when `quote.rfq_id != {id}`) and hardens the `award_rfq` repo method as
//! defense-in-depth (returns `None` — surfaced as 404 — for a non-matching
//! quote). These tests pin the contract at the HTTP layer using the same
//! token-minting harness as the sibling marketplace success-path suite.

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
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("Award IDOR Org {slug}"))
    .bind(format!("award-idor-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@award-idor.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Award IDOR User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

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
    .bind(format!("{slug}-{}@award-provider.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed provider")
}

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

/// Seed a submitted quote for `rfq_id` from `provider_id`; return its id.
async fn seed_quote(pool: &PgPool, rfq_id: Uuid, provider_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO provider_quotes (rfq_id, provider_id, price, currency, status, submitted_at)
        VALUES ($1, $2, 999.00, 'EUR', 'submitted', NOW())
        RETURNING id
        "#,
    )
    .bind(rfq_id)
    .bind(provider_id)
    .fetch_one(pool)
    .await
    .expect("seed quote")
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
    let now = Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: org_id,
        role: Some("manager".to_string()),
        email: email.to_string(),
        name: "Award IDOR User".to_string(),
    };
    let secret = TestConfig::default().jwt_secret;
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("encode test JWT")
}

async fn rfq_awarded_quote_id(pool: &PgPool, rfq_id: Uuid) -> Option<Uuid> {
    sqlx::query_scalar::<_, Option<Uuid>>(r#"SELECT awarded_quote_id FROM rfqs WHERE id = $1"#)
        .bind(rfq_id)
        .fetch_one(pool)
        .await
        .expect("read rfq awarded_quote_id")
}

// ---------------------------------------------------------------------------
// T1 — a quote from a DIFFERENT org's RFQ cannot be awarded (the IDOR)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn award_rejects_quote_from_another_orgs_rfq(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    // Org A owns RFQ A; the caller is an org_admin of org A.
    let org_a = seed_org(&pool, "a").await;
    let buyer_a = seed_user(&pool, "buyer-a@award-idor.test").await;
    seed_membership(&pool, org_a, buyer_a, "org_admin").await;
    let rfq_a = seed_rfq(&pool, org_a, buyer_a).await;

    // Org B owns RFQ B with a submitted quote — foreign to the caller.
    let org_b = seed_org(&pool, "b").await;
    let buyer_b = seed_user(&pool, "buyer-b@award-idor.test").await;
    seed_membership(&pool, org_b, buyer_b, "org_admin").await;
    let rfq_b = seed_rfq(&pool, org_b, buyer_b).await;
    let prov_b_user = seed_user(&pool, "prov-b@award-idor.test").await;
    let prov_b = seed_provider(&pool, prov_b_user, "b").await;
    let foreign_quote = seed_quote(&pool, rfq_b, prov_b).await;

    let token = mint_token(buyer_a, "buyer-a@award-idor.test", Some(org_a));

    // Caller owns RFQ A (path param passes load_rfq_for_org) but tries to award
    // it to a quote that belongs to org B's RFQ B.
    let resp = app
        .execute(
            app.post(&format!("/api/v1/marketplace/rfqs/{rfq_a}/award"))
                .bearer(&token)
                .json(json!({ "quote_id": foreign_quote }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "awarding a quote from another org's RFQ must be rejected (404), got {}: {}",
        resp.status,
        resp.text()
    );

    // RFQ A must remain un-awarded, and the foreign quote must be untouched.
    assert_eq!(
        rfq_awarded_quote_id(&pool, rfq_a).await,
        None,
        "RFQ A must not have been awarded to the foreign quote"
    );
    let foreign_status: String =
        sqlx::query_scalar(r#"SELECT status FROM provider_quotes WHERE id = $1"#)
            .bind(foreign_quote)
            .fetch_one(&pool)
            .await
            .expect("read foreign quote status");
    assert_eq!(
        foreign_status, "submitted",
        "the foreign quote's lifecycle must not have been mutated"
    );
}

// ---------------------------------------------------------------------------
// T2 — a quote belonging to another RFQ in the SAME org still cannot be awarded
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn award_rejects_quote_from_another_rfq_same_org(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "same").await;
    let buyer = seed_user(&pool, "buyer-same@award-idor.test").await;
    seed_membership(&pool, org, buyer, "org_admin").await;

    let rfq_target = seed_rfq(&pool, org, buyer).await;
    let rfq_other = seed_rfq(&pool, org, buyer).await;

    let prov_user = seed_user(&pool, "prov-same@award-idor.test").await;
    let prov = seed_provider(&pool, prov_user, "same").await;
    let other_quote = seed_quote(&pool, rfq_other, prov).await;

    let token = mint_token(buyer, "buyer-same@award-idor.test", Some(org));

    let resp = app
        .execute(
            app.post(&format!("/api/v1/marketplace/rfqs/{rfq_target}/award"))
                .bearer(&token)
                .json(json!({ "quote_id": other_quote }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "awarding a quote that belongs to a different RFQ must be rejected (404), got {}: {}",
        resp.status,
        resp.text()
    );
    assert_eq!(
        rfq_awarded_quote_id(&pool, rfq_target).await,
        None,
        "the target RFQ must not have been awarded to a quote from another RFQ"
    );
}

// ---------------------------------------------------------------------------
// T3 — positive path: a quote that DOES belong to the RFQ is awarded
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn award_succeeds_for_own_quote(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "ok").await;
    let buyer = seed_user(&pool, "buyer-ok@award-idor.test").await;
    seed_membership(&pool, org, buyer, "org_admin").await;
    let rfq = seed_rfq(&pool, org, buyer).await;

    let prov_user = seed_user(&pool, "prov-ok@award-idor.test").await;
    let prov = seed_provider(&pool, prov_user, "ok").await;
    let quote = seed_quote(&pool, rfq, prov).await;

    let token = mint_token(buyer, "buyer-ok@award-idor.test", Some(org));

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
        "awarding a quote that belongs to the RFQ must succeed (200), got {}: {}",
        resp.status,
        resp.text()
    );
    assert_eq!(
        rfq_awarded_quote_id(&pool, rfq).await,
        Some(quote),
        "the RFQ must be awarded to its own quote"
    );
}
