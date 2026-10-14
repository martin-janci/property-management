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
//! SCHEMA DRIFT (reconciled by issue #3078): the property-metrics repo queries
//! and the `PropertyPerformanceMetrics` model used to project columns absent
//! from the `property_performance_metrics` table in migration 00091 (e.g.
//! `gross_rental_income`, `average_lease_term_months`, `total_revenue`,
//! `estimated_value`), and the upsert's `ON CONFLICT` keyed on a column set
//! (`building_id, period_start, period_end, period_type`) that no unique
//! constraint covered. Those endpoints therefore 500'd/400'd at runtime
//! regardless of the IDOR fix. Earlier this was labelled "BIT-567", which is
//! not a real issue in this repo; the tracked bug is #3078 and migration
//! 00236 adds the missing columns and aligns the unique constraint.
//!
//! With the drift reconciled, these tests now assert the tenant boundary
//! end-to-end: a foreign caller must get a strict `404` on read and must not be
//! able to persist a row against another org's building, while the owning org
//! must get a `200 OK` with a correct payload (read) and a persisted row
//! (write). The `200 OK` same-org controls prove the scoping is a boundary, not
//! a "deny everyone" false positive. The unauthenticated-rejection case is
//! retained.

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

/// Insert a `property_performance_metrics` row for `building_id`/`org_id`. Only
/// the always-present / defaulted columns are set explicitly; the columns added
/// by migration 00236 (#3078) fall back to their defaults (`gross_rental_income`
/// -> 0, `currency` -> 'EUR') or NULL, which is enough for the row to decode
/// into `PropertyPerformanceMetrics` on read.
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

    // A foreign building_id resolves to no org-scoped row -> the handler maps
    // `Ok(None)` to 404. Asserting the specific 404 (not merely `!= 200`) means
    // a regression that made the endpoint 500 for everyone, or returned "404 for
    // the owner too", would be caught here rather than passing vacuously.
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "Org A property metrics must be 404 (not found) for Org B, got {}: {}",
        resp.status,
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// T1b — same-org property-metrics READ succeeds (200 control)
//
// Proves the org scoping added by #2946 is a tenant *boundary*, not a blanket
// "deny everyone" — the owning org still reads its own row with a 200 and the
// correct payload. This control is only exercisable now that #3078 reconciled
// the `property_performance_metrics` schema drift (migration 00236).
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn property_metrics_same_org_read_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_b = seed_org(&pool, "pa-idor-read-ok-b").await;
    let user_b = seed_user(&pool, &format!("read-ok-b-{}@pa-idor.test", Uuid::new_v4())).await;
    seed_membership(&pool, org_b, user_b, "org_admin").await;

    // Building and metrics both belong to org B — the caller's own org.
    let building_b = seed_building(&pool, org_b).await;
    seed_metrics_row(&pool, org_b, building_b).await;

    let token_b = mint_token(user_b, "read-ok-b@pa-idor.test", org_b);
    let uri = format!(
        "/api/v1/portfolio-analytics/properties/{building_b}/metrics\
         ?period_start=2024-01-01&period_end=2024-01-31"
    );
    let resp = app.execute(app.get(&uri).bearer(&token_b).build()).await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "Org B must read its own property metrics (got {}): {}",
        resp.status,
        resp.text()
    );

    let body: serde_json::Value = resp.json();
    assert_eq!(
        body.get("building_id").and_then(|v| v.as_str()),
        Some(building_b.to_string().as_str()),
        "same-org read must return the requested building's metrics: {body}"
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
    // The `WHERE EXISTS (… org)` guard writes no row, so `fetch_one` surfaces
    // RowNotFound, which the handler maps to 400. Asserting the specific status
    // (not merely `!= 200`) catches a regression to a silent 2xx.
    assert_eq!(
        resp.status,
        StatusCode::BAD_REQUEST,
        "Org B must NOT upsert metrics against Org A's building (got {}): {}",
        resp.status,
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
// T2b — same-org property-metrics WRITE succeeds and persists (200 control)
//
// Proves the `WHERE EXISTS (… org)` guard admits the owning org: a caller
// upserting metrics for its *own* building gets a 200 and a persisted row.
// Only exercisable now that #3078 reconciled the schema drift (migration
// 00236) — the upsert's `ON CONFLICT` target now matches a real constraint.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn property_metrics_same_org_upsert_persists(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_b = seed_org(&pool, "pa-idor-write-ok-b").await;
    let user_b = seed_user(
        &pool,
        &format!("write-ok-b-{}@pa-idor.test", Uuid::new_v4()),
    )
    .await;
    seed_membership(&pool, org_b, user_b, "org_admin").await;

    // Building belongs to org B — the caller's own org.
    let building_b = seed_building(&pool, org_b).await;

    let token_b = mint_token(user_b, "write-ok-b@pa-idor.test", org_b);
    let resp = app
        .execute(
            app.post("/api/v1/portfolio-analytics/properties/metrics")
                .bearer(&token_b)
                .json(serde_json::json!({
                    "building_id": building_b,
                    "period_start": "2024-03-01",
                    "period_end": "2024-03-31",
                    "period_type": "monthly",
                    "total_units": 10,
                    "occupied_units": 8,
                    "gross_rental_income": "12000.00"
                }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "Org B must upsert metrics against its own building (got {}): {}",
        resp.status,
        resp.text()
    );

    let count = count_metrics_for_building(&pool, building_b).await;
    assert_eq!(
        count, 1,
        "same-org upsert must persist exactly one metrics row for Org B's building; found {count}"
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
