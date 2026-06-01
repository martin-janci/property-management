//! Regression tests for the cross-tenant IDOR fix on the competitive-intelligence
//! endpoints (issue #895).
//!
//! Audit history: every handler in `routes/competitive.rs` accepted a
//! `listing_id` path param and a `TenantExtractor` but DISCARDED both (params
//! named `_tenant` / `_listing`, unused `State`). There was no
//! `listing.organization_id == tenant` ownership check, so any authenticated
//! caller could target any listing across organizations — and every handler
//! returned hard-coded mock data regardless.
//!
//! The fix threads the authenticated org through each handler: every endpoint
//! now resolves the listing via `ListingRepository::find_by_id_and_org(listing_id,
//! tenant_id)` and returns `404` for a missing or foreign-org listing BEFORE
//! doing anything. After the org gate, the (still-unimplemented) features return
//! `501 Not Implemented` rather than fabricated data.
//!
//! Test contract:
//!   * Cross-org access to a listing → `404` (foreign listing is invisible).
//!   * Same-org access → reaches the handler: `501 Not Implemented` (NOT
//!     `404`/`403`), proving the org gate does not over-block legitimate access.
//!
//! `competitive` handlers use `TenantExtractor`, which resolves the tenant from
//! the `X-Tenant-ID` header (no `host_tenant_middleware` is mounted under
//! `TestApp`, so the header is the only source). We set `X-Tenant-ID: <org>` so
//! the extractor resolves the caller's org. JWTs are minted with the `tenant_id`
//! claim, matching `marketplace_voting_investor_cross_org_idor_tests`.

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use common::{TestApp, TestConfig};

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
    .bind(format!("Competitive Org {slug}"))
    .bind(format!("competitive-org-{slug}"))
    .bind(format!("{slug}@competitive-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Competitive User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Make `user_id` an active member of `org_id`.
async fn seed_membership(pool: &PgPool, org_id: Uuid, user_id: Uuid) {
    sqlx::query(
        r#"
        INSERT INTO organization_members (organization_id, user_id, role_type, status, joined_at)
        VALUES ($1, $2, 'org_admin', 'active', NOW())
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("seed membership");
}

/// Seed a listing owned by `org_id` and return its id.
///
/// NOTE: `size_sqm`, `latitude`, and `longitude` MUST be seeded non-null. The
/// `db::Listing` model decodes them with `#[sqlx(try_from = "rust_decimal::Decimal")]`,
/// which decodes the column as a non-nullable `Decimal` BEFORE the `Option`
/// wrapper applies — so a NULL column makes `find_by_id_and_org` fail with
/// "unexpected null" (a 500), not the expected 501. The cross-org test never
/// hits this decode path (its `WHERE organization_id = $2` filters the row out
/// entirely → clean `None` → 404), which is why only the same-org case tripped.
async fn seed_listing(pool: &PgPool, org_id: Uuid, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO listings
            (organization_id, created_by, transaction_type, title,
             property_type, street, city, postal_code, price,
             size_sqm, latitude, longitude)
        VALUES ($1, $2, 'sale', 'Competitive listing',
                'apartment', 'Test Street 1', 'Bratislava', '81101', 195000.00,
                72.50, 48.148598, 17.107748)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed listing")
}

/// Claims shape that matches `api_core::extractors::auth::Claims`.
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
        name: "Competitive User".to_string(),
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
// Cross-org: a foreign-org listing must be invisible (404), never 200/501.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn competitive_endpoints_reject_cross_org_listing(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "x-a").await;
    let org_b = seed_org(&pool, "x-b").await;
    let user_a = seed_user(&pool, "x-a@competitive-idor.test").await;
    let user_b = seed_user(&pool, "x-b@competitive-idor.test").await;
    seed_membership(&pool, org_a, user_a).await;
    seed_membership(&pool, org_b, user_b).await;
    // Listing belongs to org A.
    let listing_a = seed_listing(&pool, org_a, user_a).await;

    // Org B's caller authenticates as org B (member) but targets org A's listing.
    let token_b = mint_token(user_b, "x-b@competitive-idor.test", org_b);

    // Representative endpoints across all four stories + the combined views.
    let endpoints = [
        format!("/api/v1/competitive/listings/{listing_a}/tours"),
        format!("/api/v1/competitive/listings/{listing_a}/pricing"),
        format!("/api/v1/competitive/listings/{listing_a}/pricing/history"),
        format!("/api/v1/competitive/listings/{listing_a}/neighborhood"),
        format!("/api/v1/competitive/listings/{listing_a}/neighborhood/amenities"),
        format!("/api/v1/competitive/listings/{listing_a}/comparables"),
        format!("/api/v1/competitive/listings/{listing_a}/competitive-analysis"),
        format!("/api/v1/competitive/listings/{listing_a}/competitive-status"),
    ];

    for uri in endpoints {
        let resp = app
            .execute(
                app.get(&uri)
                    .bearer(&token_b)
                    .header("X-Tenant-ID", &org_b.to_string())
                    .build(),
            )
            .await;

        assert_eq!(
            resp.status,
            StatusCode::NOT_FOUND,
            "cross-org access to {uri} must be 404 (got {}): {}",
            resp.status,
            resp.text()
        );
    }
}

// ---------------------------------------------------------------------------
// Same-org: the org gate must NOT over-block — a legitimate caller reaches the
// handler. The handler currently returns 501 (no persistence), proving the
// listing was resolved and ownership confirmed (not 404/403).
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn competitive_endpoints_allow_same_org_listing(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "own-a").await;
    let user_a = seed_user(&pool, "own-a@competitive-idor.test").await;
    seed_membership(&pool, org_a, user_a).await;
    let listing_a = seed_listing(&pool, org_a, user_a).await;

    let token_a = mint_token(user_a, "own-a@competitive-idor.test", org_a);

    let endpoints = [
        format!("/api/v1/competitive/listings/{listing_a}/tours"),
        format!("/api/v1/competitive/listings/{listing_a}/pricing"),
        format!("/api/v1/competitive/listings/{listing_a}/neighborhood"),
        format!("/api/v1/competitive/listings/{listing_a}/comparables"),
        format!("/api/v1/competitive/listings/{listing_a}/competitive-analysis"),
        format!("/api/v1/competitive/listings/{listing_a}/competitive-status"),
    ];

    for uri in endpoints {
        let resp = app
            .execute(
                app.get(&uri)
                    .bearer(&token_a)
                    .header("X-Tenant-ID", &org_a.to_string())
                    .build(),
            )
            .await;

        // Org gate passed: the handler was reached. It must NOT be 404 (would
        // mean the org gate wrongly rejected the owner) and must NOT be 403.
        assert_ne!(
            resp.status,
            StatusCode::NOT_FOUND,
            "same-org access to {uri} must NOT be 404 (org gate over-blocked owner): {}",
            resp.text()
        );
        assert_ne!(
            resp.status,
            StatusCode::FORBIDDEN,
            "same-org access to {uri} must NOT be 403: {}",
            resp.text()
        );
        // Current honest-stub behavior: feature has no persistence yet → 501.
        assert_eq!(
            resp.status,
            StatusCode::NOT_IMPLEMENTED,
            "same-org access to {uri} should reach the 501 stub (got {}): {}",
            resp.status,
            resp.text()
        );
    }
}
