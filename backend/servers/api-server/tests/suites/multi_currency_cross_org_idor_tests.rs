//! Regression tests for the cross-org IDOR fix on multi-currency reads (#850).
//!
//! Audit history: `get_property_currency_config` in
//! `servers/api-server/src/routes/multi_currency.rs` authenticated the caller
//! but loaded the property currency config by `building_id` only — the repo
//! query was `... WHERE building_id = $1`, with no organization predicate. Any
//! authenticated user who knew (or guessed) another org's `building_id` could
//! read that org's currency/tax configuration. The UPDATE path already scoped
//! by `organization_id`; the read path did not.
//!
//! The fix threads the verified `org_id` (JWT `tenant_id`) into the repository
//! so the SQL is org-scoped:
//!   `... WHERE building_id = $1 AND organization_id = $2`
//! A foreign-org row therefore surfaces as `None` → HTTP 404, never a
//! cross-tenant read.
//!
//! Tenancy is resolved by `ValidatedTenantExtractor`: the `X-Tenant-ID` header
//! plus an active `organization_members` row (role read from the DB). The
//! tests seed a membership per org and drive the handler end-to-end:
//!   - Org A's token reading Org A's property config -> 200 (same-org succeeds).
//!   - Org B's token reading Org A's property config -> 404 (cross-org blocked).

#![allow(dead_code)]

use axum::http::StatusCode;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use serde_json::json;

use crate::common::{seed_membership, TestApp, TestConfig};

// ---------------------------------------------------------------------------
// JWT minting (matches api_core::extractors::auth::Claims)
// ---------------------------------------------------------------------------

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

/// Mint an access token bound to `org_id` (the JWT `tenant_id` claim), signed
/// with the same secret `TestApp` configures.
fn access_token(user_id: Uuid, org_id: Uuid) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some("manager".to_string()),
        email: "idor-test@multi-currency.test".to_string(),
        name: "MultiCurrency IDOR User".to_string(),
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
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("MultiCurrencyIDOR Org {slug}"))
    .bind(format!("mc-idor-{slug}"))
    .bind(format!("{slug}@mc-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'MultiCurrencyIDOR User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code, country)
        VALUES ($1, $2, 'Bratislava', '81101', 'Slovakia') RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(format!("{slug} Street 1"))
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_property_currency_config(pool: &PgPool, org_id: Uuid, building_id: Uuid) {
    sqlx::query(
        r#"
        INSERT INTO property_currency_config (building_id, organization_id, default_currency, country)
        VALUES ($1, $2, 'EUR', 'SK')
        "#,
    )
    .bind(building_id)
    .bind(org_id)
    .execute(pool)
    .await
    .expect("seed property currency config");
}

// ---------------------------------------------------------------------------
// Test 1: same-org read succeeds
// ---------------------------------------------------------------------------

/// A user whose JWT `tenant_id` matches the building's organization can read
/// the property currency config.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_property_currency_config_same_org_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "same-org@mc-idor.test").await;
    let org_a = seed_org(&pool, "same-a").await;
    let building_id = seed_building(&pool, org_a, "same-a").await;
    seed_property_currency_config(&pool, org_a, building_id).await;
    seed_membership(&pool, org_a, user, "manager").await;

    let token = access_token(user, org_a);
    let req = app
        .get(&format!("/api/v1/multi-currency/properties/{building_id}"))
        .bearer(&token)
        .tenant(org_a)
        .build();
    let resp = app.execute(req).await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(
        body["building_id"].as_str(),
        Some(building_id.to_string().as_str()),
        "same-org read must return the requested property config"
    );
}

// ---------------------------------------------------------------------------
// Test 2: cross-org read is blocked (404)
// ---------------------------------------------------------------------------

/// A user from Org B cannot read Org A's property currency config by
/// building id — the org-scoped query finds no row and the handler returns
/// 404 (the core #850 IDOR).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_property_currency_config_cross_org_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user_b = seed_user(&pool, "attacker@mc-idor.test").await;
    let org_a = seed_org(&pool, "x-a").await;
    let org_b = seed_org(&pool, "x-b").await;
    let building_id = seed_building(&pool, org_a, "x-a").await;
    seed_property_currency_config(&pool, org_a, building_id).await;

    seed_membership(&pool, org_b, user_b, "manager").await;

    // Org B token targeting Org A's building.
    let token = access_token(user_b, org_b);
    let req = app
        .get(&format!("/api/v1/multi-currency/properties/{building_id}"))
        .bearer(&token)
        .tenant(org_b)
        .build();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "cross-org property config read must be 404, got {} (body: {})",
        resp.status,
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// Round-4 authz: role gates, body-id scoping, global exchange rates
// ---------------------------------------------------------------------------

fn property_config_body(building_id: Uuid) -> serde_json::Value {
    json!({
        "building_id": building_id,
        "default_currency": "EUR",
        "country": "SK"
    })
}

/// A manager cannot create a property currency config for another org's
/// building: the body `building_id` is checked against the caller's org.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_property_config_for_foreign_building_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user_b = seed_user(&pool, "r4-attacker@mc-idor.test").await;
    let org_a = seed_org(&pool, "r4-a").await;
    let org_b = seed_org(&pool, "r4-b").await;
    let foreign_building = seed_building(&pool, org_a, "r4-a").await;
    seed_membership(&pool, org_b, user_b, "manager").await;

    let resp = app
        .execute(
            app.post("/api/v1/multi-currency/properties")
                .bearer(&access_token(user_b, org_b))
                .tenant(org_b)
                .json(property_config_body(foreign_building))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND, "{}", resp.text());
    let rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM property_currency_config WHERE building_id = $1")
            .bind(foreign_building)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rows, 0, "no config may be written for a foreign building");
}

/// A tenant member (DB role `tenant`) cannot mutate org currency settings,
/// even with a JWT that claims `manager`.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn tenant_member_cannot_create_property_config(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = seed_user(&pool, "r4-tenant@mc-idor.test").await;
    let org = seed_org(&pool, "r4-t").await;
    let building = seed_building(&pool, org, "r4-t").await;
    seed_membership(&pool, org, user, "tenant").await;

    let resp = app
        .execute(
            app.post("/api/v1/multi-currency/properties")
                .bearer(&access_token(user, org))
                .tenant(org)
                .json(property_config_body(building))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());
}

/// The owning org's manager can create the config for its own building.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_creates_property_config_for_own_building(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = seed_user(&pool, "r4-manager@mc-idor.test").await;
    let org = seed_org(&pool, "r4-m").await;
    let building = seed_building(&pool, org, "r4-m").await;
    seed_membership(&pool, org, user, "manager").await;

    let resp = app
        .execute(
            app.post("/api/v1/multi-currency/properties")
                .bearer(&access_token(user, org))
                .tenant(org)
                .json(property_config_body(building))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}

/// Exchange rates are global: an org admin (not a platform principal in
/// `users.principal_kind`) cannot write or override them.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn org_manager_cannot_write_global_exchange_rates(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = seed_user(&pool, "r4-rates@mc-idor.test").await;
    let org = seed_org(&pool, "r4-r").await;
    seed_membership(&pool, org, user, "org_admin").await;
    let token = access_token(user, org);

    let resp = app
        .execute(
            app.post("/api/v1/multi-currency/rates")
                .bearer(&token)
                .json(json!({
                    "from_currency": "EUR",
                    "to_currency": "USD",
                    "rate": "1.10",
                    "rate_date": "2026-01-15",
                    "source": "ecb"
                }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());

    let resp = app
        .execute(
            app.post("/api/v1/multi-currency/rates/override")
                .bearer(&token)
                .json(json!({
                    "from_currency": "EUR",
                    "to_currency": "USD",
                    "rate": "9.99",
                    "rate_date": "2026-01-15",
                    "reason": "attack"
                }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());

    let rates: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM exchange_rates WHERE rate_date = '2026-01-15'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rates, 0, "no global rate may be written by an org manager");
}
