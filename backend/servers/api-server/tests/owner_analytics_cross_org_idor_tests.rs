//! Regression tests for the cross-tenant IDOR fix on the owner-analytics
//! endpoints (`/api/v1/owner-analytics/*`, Epic 74 — issue #901).
//!
//! Audit history: the `OwnerAnalyticsRepository` queries plumbed an `org_id`
//! parameter through but never used it — every read/mutate was keyed solely on
//! `unit_id` / `valuation_id` (`WHERE unit_id = $1`). The api-server connects
//! with a BYPASSRLS role (see migration 00165), so the table-level RLS policies
//! added in 00116 do NOT catch this at runtime. The net effect: any
//! authenticated user could read another org's property valuations, value
//! history, ROI / cash-flow, and expense data by guessing a `unit_id`.
//!
//! The fix threads the caller's `tenant_id` (from the JWT) into every query and
//! constrains it via `units -> buildings.organization_id`, so a unit in another
//! org simply does not match (surfaced as 404 / empty, never the foreign row).
//!
//! These tests exercise the HTTP surface end-to-end with real HS256 JWTs:
//!   1. Seed two orgs (A, B), each with a building + unit; a valuation in A's
//!      unit; and a member user in each org (tenant_id carried in the token).
//!   2. Org B's member probes Org A's unit/valuation → must NOT 200.
//!   3. Org A's member reads its own unit's valuation → 200 with the row.

#![allow(dead_code)]

mod common;

use axum::http::StatusCode;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
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
    .bind(format!("OwnerIDOR Org {slug}"))
    .bind(format!("owner-idor-org-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@owner-idor.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'OwnerIDOR User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code, country)
        VALUES ($1, 'Test St 1', 'Bratislava', '81101', 'Slovakia')
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
        r#"
        INSERT INTO units (building_id, designation, floor, unit_type)
        VALUES ($1, '3B', 3, 'apartment')
        RETURNING id
        "#,
    )
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

async fn seed_valuation(pool: &PgPool, unit_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO property_valuations
            (unit_id, value_low, value_high, estimated_value, valuation_method, confidence_score)
        VALUES ($1, 95000, 105000, 100000, 'avm', 85)
        RETURNING id
        "#,
    )
    .bind(unit_id)
    .fetch_one(pool)
    .await
    .expect("seed valuation")
}

/// Claims shape matching `api_core::extractors::auth::Claims`. We mint with this
/// rather than `JwtService` — which serializes the org as the `org_id` claim —
/// so the `tenant_id` claim populates `AuthUser.tenant_id`. Without it the
/// owner-analytics handlers see no tenant context (400 "Tenant context
/// required") and the same-org success case wrongly fails.
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

/// Mint an HS256 access token for `user_id`, scoped to `org_id` via the JWT
/// `tenant_id` claim, signed with the same secret `TestApp` configures.
fn mint_token(user_id: Uuid, email: &str, org_id: Uuid) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some("manager".to_string()),
        email: email.to_string(),
        name: "OwnerIDOR User".to_string(),
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
// T1 — cross-org unit valuation read is rejected (IDOR)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn unit_valuation_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "valuation-a").await;
    let org_b = seed_org(&pool, "valuation-b").await;
    let user_b = seed_user(&pool, &format!("val-b-{}@owner-idor.test", Uuid::new_v4())).await;
    seed_membership(&pool, org_b, user_b, "org_admin").await;

    let building_a = seed_building(&pool, org_a).await;
    let unit_a = seed_unit(&pool, building_a).await;
    let _val_a = seed_valuation(&pool, unit_a).await;

    // User B (org B) probes Org A's unit valuation.
    let token_b = mint_token(user_b, "val-b@owner-idor.test", org_b);
    let uri = format!("/api/v1/owner-analytics/units/{unit_a}/valuation");
    let resp = app.execute(app.get(&uri).bearer(&token_b).build()).await;

    assert_ne!(
        resp.status,
        StatusCode::OK,
        "Org A unit valuation must NOT be readable by Org B (got 200): {}",
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// T2 — cross-org value-history read is rejected and returns no rows
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn value_history_from_other_org_is_empty(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "hist-a").await;
    let org_b = seed_org(&pool, "hist-b").await;
    let user_b = seed_user(&pool, &format!("hist-b-{}@owner-idor.test", Uuid::new_v4())).await;
    seed_membership(&pool, org_b, user_b, "org_admin").await;

    let building_a = seed_building(&pool, org_a).await;
    let unit_a = seed_unit(&pool, building_a).await;
    // create_valuation also writes a value-history row.
    let _val_a = seed_valuation(&pool, unit_a).await;
    sqlx::query("INSERT INTO property_value_history (unit_id, value) VALUES ($1, 100000)")
        .bind(unit_a)
        .execute(&pool)
        .await
        .expect("seed history");

    let token_b = mint_token(user_b, "hist-b@owner-idor.test", org_b);
    let uri = format!("/api/v1/owner-analytics/units/{unit_a}/value-history");
    let resp = app.execute(app.get(&uri).bearer(&token_b).build()).await;

    // The list endpoint returns 200 with a JSON array; the org-scoping must make
    // that array empty for a foreign caller (no rows leak across the tenant
    // boundary). Either a non-200 OR an empty array is acceptable; a populated
    // array is the bug.
    if resp.status == StatusCode::OK {
        let body = resp.json_value();
        let len = body.as_array().map(|a| a.len()).unwrap_or(0);
        assert_eq!(
            len, 0,
            "Org A value history must not leak to Org B; got {len} rows: {body}"
        );
    }
}

// ---------------------------------------------------------------------------
// T3 — legitimate same-org valuation read succeeds (fix does not over-block)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn unit_valuation_same_org_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "legit-a").await;
    let user_a = seed_user(
        &pool,
        &format!("legit-a-{}@owner-idor.test", Uuid::new_v4()),
    )
    .await;
    seed_membership(&pool, org_a, user_a, "org_admin").await;

    let building_a = seed_building(&pool, org_a).await;
    let unit_a = seed_unit(&pool, building_a).await;
    let _val_a = seed_valuation(&pool, unit_a).await;

    let token_a = mint_token(user_a, "legit-a@owner-idor.test", org_a);
    let uri = format!("/api/v1/owner-analytics/units/{unit_a}/valuation");
    let resp = app.execute(app.get(&uri).bearer(&token_a).build()).await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "Org A member must read its own unit valuation (got {}): {}",
        resp.status,
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// T4 — unauthenticated read is rejected
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn unit_valuation_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let uri = format!("/api/v1/owner-analytics/units/{}/valuation", Uuid::new_v4());
    let resp = app.execute(app.get(&uri).build()).await;
    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "missing bearer token must be 401, got {}",
        resp.status
    );
}
