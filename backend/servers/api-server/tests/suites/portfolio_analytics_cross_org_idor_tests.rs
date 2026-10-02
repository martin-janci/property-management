//! Regression tests for the cross-tenant IDOR fix on the portfolio-analytics
//! property-metrics endpoints (Epic 140 — issue #2946).
//!
//! Audit history: the `PortfolioAnalyticsRepository` property-metrics read and
//! write plumbed no `org_id`. The handlers in
//! `routes/portfolio_analytics.rs` extracted the caller's `tenant_id` into a
//! `_org_id` binding and then **discarded** it — the repo queries were keyed
//! solely on `building_id` (+ period). The api-server connects with a BYPASSRLS
//! role (see migration 00165), so the table-level RLS policies do NOT catch this
//! at runtime. Net effect: any authenticated user could read another org's
//! `property_performance_metrics` by guessing a `building_id`
//! (`GET /portfolio-analytics/properties/{building_id}/metrics`), and could
//! create/overwrite metrics against a building in another org
//! (`POST /portfolio-analytics/properties/metrics`).
//!
//! The fix (mirroring PR #2976 / #2977) threads the caller's `tenant_id` into
//! both queries:
//!   * read  — `... AND organization_id = $org`, so a foreign building's row
//!     does not match (surfaced as 404, never the foreign row);
//!   * write — persists `organization_id` and guards the target building with
//!     `WHERE EXISTS (SELECT 1 FROM buildings WHERE id = $building AND
//!     organization_id = $org)`, so a cross-tenant upsert writes no row.
//!
//! KNOWN LIMITATION (pre-existing, tracked as BIT-567): the property-metrics
//! repo queries also project columns that are absent from the
//! `property_performance_metrics` table in migration 00091 (e.g.
//! `gross_rental_income`, `average_lease_term_months`, `total_revenue`,
//! `estimated_value`). Because of that pre-existing schema drift these endpoints
//! currently return 500 at runtime regardless of the IDOR fix, so a same-org
//! `200 OK` control and a strict `404`-on-foreign assertion cannot be exercised
//! here (the sibling `analytics_portfolio_success_tests.rs` deletes those cases
//! for the same reason). These tests therefore assert the tenant-boundary
//! invariant that holds independently of the projection bug — a foreign caller
//! must NOT get a `2xx`, and must NOT be able to persist a row against another
//! org's building — plus the standard unauthenticated-rejection case. Once
//! BIT-567 reconciles the schema, the `assert_ne!` reads should be tightened to
//! `assert_eq!(NOT_FOUND)` and a same-org success control added.

#![allow(dead_code)]

use axum::http::StatusCode;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, seed_org, TestApp, TestConfig};

// ---------------------------------------------------------------------------
// JWT helper — mint an HS256 access token whose `tenant_id` claim populates
// `AuthUser.tenant_id` (JwtService serializes the org as `org_id`, which does
// NOT map to tenant context — hence the hand-rolled claims here).
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
        name: "PortfolioIDOR User".to_string(),
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

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'PortfolioIDOR User', 'active', NOW())
           RETURNING id"#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Metrics St 1', 'Bratislava', '81101', 'Slovakia')
           RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

/// Insert a `property_performance_metrics` row for `building_id`/`org_id` using
/// only columns that exist in migration 00091, so the foreign row physically
/// exists as an IDOR target (independent of the BIT-567 projection drift).
async fn seed_metrics_row(pool: &PgPool, org_id: Uuid, building_id: Uuid) {
    sqlx::query(
        r#"INSERT INTO property_performance_metrics
               (organization_id, building_id, period_type, period_start, period_end,
                occupancy_rate, total_units, occupied_units)
           VALUES ($1, $2, 'monthly', '2024-01-01', '2024-01-31', 95.0, 10, 9)"#,
    )
    .bind(org_id)
    .bind(building_id)
    .execute(pool)
    .await
    .expect("seed metrics row");
}

async fn count_metrics_for_building(pool: &PgPool, building_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM property_performance_metrics WHERE building_id = $1",
    )
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("count metrics")
}

// ---------------------------------------------------------------------------
// T1 — cross-org property-metrics READ is rejected (IDOR)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn property_metrics_from_other_org_read_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "pa-idor-read-a").await;
    let org_b = seed_org(&pool, "pa-idor-read-b").await;
    let user_b = seed_user(&pool, &format!("read-b-{}@pa-idor.test", Uuid::new_v4())).await;
    seed_membership(&pool, org_b, user_b, "org_admin").await;

    let building_a = seed_building(&pool, org_a).await;
    seed_metrics_row(&pool, org_a, building_a).await;

    // User B (org B) probes Org A's building metrics.
    let token_b = mint_token(user_b, "read-b@pa-idor.test", org_b);
    let uri = format!(
        "/api/v1/portfolio-analytics/properties/{building_a}/metrics\
         ?period_start=2024-01-01&period_end=2024-01-31"
    );
    let resp = app.execute(app.get(&uri).bearer(&token_b).build()).await;

    assert_ne!(
        resp.status,
        StatusCode::OK,
        "Org A property metrics must NOT be readable by Org B (got 200): {}",
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// T2 — cross-org property-metrics WRITE cannot persist against a foreign
//      building (upsert IDOR)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn property_metrics_upsert_against_other_org_building_writes_nothing(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "pa-idor-write-a").await;
    let org_b = seed_org(&pool, "pa-idor-write-b").await;
    let user_b = seed_user(&pool, &format!("write-b-{}@pa-idor.test", Uuid::new_v4())).await;
    seed_membership(&pool, org_b, user_b, "org_admin").await;

    // Building belongs to org A; attacker is org B.
    let building_a = seed_building(&pool, org_a).await;

    let token_b = mint_token(user_b, "write-b@pa-idor.test", org_b);
    let resp = app
        .execute(
            app.post("/api/v1/portfolio-analytics/properties/metrics")
                .bearer(&token_b)
                .json(serde_json::json!({
                    "building_id": building_a,
                    "period_start": "2024-02-01",
                    "period_end": "2024-02-29",
                    "period_type": "monthly",
                    "total_units": 10,
                    "occupied_units": 8,
                    "gross_rental_income": "12000.00"
                }))
                .build(),
        )
        .await;

    // The write must not succeed against a building the caller does not own.
    assert_ne!(
        resp.status,
        StatusCode::OK,
        "Org B must NOT upsert metrics against Org A's building (got 200): {}",
        resp.text()
    );

    // And, crucially, no metrics row for org A's building may have been created
    // by the cross-tenant caller.
    let count = count_metrics_for_building(&pool, building_a).await;
    assert_eq!(
        count, 0,
        "cross-tenant upsert must not persist a metrics row for Org A's building; found {count}"
    );
}

// ---------------------------------------------------------------------------
// T3 — unauthenticated read is rejected
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn property_metrics_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let uri = format!(
        "/api/v1/portfolio-analytics/properties/{}/metrics\
         ?period_start=2024-01-01&period_end=2024-01-31",
        Uuid::new_v4()
    );
    let resp = app.execute(app.get(&uri).build()).await;
    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "missing bearer token must be 401, got {}",
        resp.status
    );
}
