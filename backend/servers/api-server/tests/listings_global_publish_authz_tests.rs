//! Authz + tenant-isolation regression for the listings global-publish
//! endpoints (#851, #809).
//!
//! Before the fix:
//! - `global_publish` / `global_unpublish` used a bare `AuthUser` + a "Phase 5
//!   replaces this" stub gate that accepted ANY authenticated org member
//!   (e.g. a resident) as long as their token carried a tenant. The fix
//!   standardizes them onto the same `TenantExtractor` the rest of
//!   `listings.rs` uses and gates the action behind manager role.
//! - The repository UPDATE is org-scoped, so a caller from org B publishing
//!   org A's listing must NOT succeed.
//!
//! These tests mint real JWTs (the same `Claims` shape the `AuthUser`
//! extractor decodes) so the role + tenant come from the token, exactly as in
//! production. `JwtService` emits an `org_id` claim that does NOT map onto the
//! `tenant_id` field the extractor reads, so we encode our own claims struct
//! carrying `tenant_id` + `role` directly (see the note in
//! reserve_funds_cross_org_idor_tests.rs).

#![allow(dead_code)]

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use common::TestApp;

/// Same secret `TestConfig::default()` stamps into `JWT_SECRET`, so tokens we
/// mint here validate against the `AuthUser` extractor's cached verifier.
const JWT_SECRET: &str = "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

/// Minimal mirror of `api_core::extractors::auth::Claims` (the access-token
/// wire shape the extractor decodes). `role` is serialized as snake_case to
/// match `TenantRole`'s serde representation.
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

/// Mint a real access JWT for `user_id` in `org_id` with `role`
/// (snake_case, e.g. "manager" / "resident").
fn mint_token(user_id: Uuid, org_id: Uuid, role: &str) -> String {
    let now = Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        iat: now,
        exp: now + 3600,
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some(role.to_string()),
        email: format!("{user_id}@listings-authz.test"),
        name: "Listings Authz Test".to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("mint jwt")
}

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO organizations (name, slug, contact_email, status)
           VALUES ($1, $2, $3, 'active') RETURNING id"#,
    )
    .bind(format!("ListingsAuthz {slug}"))
    .bind(format!("listings-authz-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@listings-authz.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'x', 'Authz User', 'active', NOW()) RETURNING id"#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed a draft listing in `org_id` created by `user_id`. Returns its id.
async fn seed_listing(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        // `size_sqm` / `latitude` / `longitude` are populated because the
        // `Listing` model decodes them via `#[sqlx(try_from = "Decimal")]`,
        // which does not tolerate NULL (a separate, pre-existing repo quirk
        // unrelated to this auth fix). Setting them keeps the success-path
        // `find_by_id_and_org` decode clean so the test exercises the authz +
        // org-scoping behavior we actually care about.
        r#"INSERT INTO listings
               (organization_id, created_by, status, transaction_type, title,
                property_type, size_sqm, rooms, street, city, postal_code,
                country, latitude, longitude, price, currency)
           VALUES ($1, $2, 'active', 'sale', 'Test Listing', 'apartment',
                   75.5, 3, 'Main 1', 'Bratislava', '81101', 'SK',
                   48.1486, 17.1077, 100000, 'EUR')
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed listing")
}

fn authed(method: Method, uri: &str, org: Uuid, token: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org.to_string())
        .body(Body::empty())
        .unwrap()
}

async fn is_published(pool: &PgPool, listing_id: Uuid) -> bool {
    sqlx::query_scalar::<_, bool>("SELECT is_published FROM listings WHERE id = $1")
        .bind(listing_id)
        .fetch_one(pool)
        .await
        .expect("read is_published")
}

// ---------------------------------------------------------------------------
// #809 — role gate: a non-manager (resident) must be rejected with 403.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_cannot_global_publish(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "resident-pub").await;
    let user = seed_user(
        &pool,
        &format!("res-{}@listings-authz.test", Uuid::new_v4()),
    )
    .await;
    let listing = seed_listing(&pool, org, user).await;

    let token = mint_token(user, org, "resident");
    let resp = app
        .execute(authed(
            Method::POST,
            &format!("/api/v1/listings/{listing}/global-publish"),
            org,
            &token,
        ))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "resident global-publish must be 403, got {} (body: {})",
        resp.status,
        String::from_utf8_lossy(&resp.body)
    );
    assert!(
        !is_published(&pool, listing).await,
        "listing must remain unpublished after a rejected publish"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_cannot_global_unpublish(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "resident-unpub").await;
    let user = seed_user(
        &pool,
        &format!("res-{}@listings-authz.test", Uuid::new_v4()),
    )
    .await;
    let listing = seed_listing(&pool, org, user).await;

    let token = mint_token(user, org, "resident");
    let resp = app
        .execute(authed(
            Method::POST,
            &format!("/api/v1/listings/{listing}/global-unpublish"),
            org,
            &token,
        ))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "resident global-unpublish must be 403, got {}",
        resp.status
    );
}

// ---------------------------------------------------------------------------
// #851 — cross-tenant: a manager in org B must NOT publish org A's listing.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_cannot_global_publish_other_orgs_listing(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "xorg-a").await;
    let org_b = seed_org(&pool, "xorg-b").await;
    let user_a = seed_user(&pool, &format!("a-{}@listings-authz.test", Uuid::new_v4())).await;
    let user_b = seed_user(&pool, &format!("b-{}@listings-authz.test", Uuid::new_v4())).await;
    let listing_in_a = seed_listing(&pool, org_a, user_a).await;

    // Manager of org B targeting org A's listing, with B's tenant context.
    let token_b = mint_token(user_b, org_b, "manager");
    let resp = app
        .execute(authed(
            Method::POST,
            &format!("/api/v1/listings/{listing_in_a}/global-publish"),
            org_b,
            &token_b,
        ))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "cross-org global-publish must 404 (org-scoped lookup finds nothing), got {}",
        resp.status
    );
    assert!(
        !is_published(&pool, listing_in_a).await,
        "org A's listing must NOT be published by an org B manager"
    );
}

// ---------------------------------------------------------------------------
// Same-org success: a manager publishing their OWN org's listing succeeds and
// the row is actually flipped (proves real org context, not a stub).
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_can_global_publish_own_listing(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "own-pub").await;
    let user = seed_user(
        &pool,
        &format!("mgr-{}@listings-authz.test", Uuid::new_v4()),
    )
    .await;
    let listing = seed_listing(&pool, org, user).await;

    let token = mint_token(user, org, "manager");
    let resp = app
        .execute(authed(
            Method::POST,
            &format!("/api/v1/listings/{listing}/global-publish"),
            org,
            &token,
        ))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "manager publishing own listing must be 200, got {} (body: {})",
        resp.status,
        String::from_utf8_lossy(&resp.body)
    );
    assert!(
        is_published(&pool, listing).await,
        "listing must be marked published after a successful global-publish"
    );
}
