//! Integration tests for the realtor "my listings" LIST + per-listing analytics
//! endpoints added to `reality-server/src/routes/portal_listings.rs`.
//!
//! # Background
//!
//! Before this change the portal-listings router only exposed
//! `POST /api/v1/my/listings`, `GET|PATCH /api/v1/my/listings/{id}`. There was
//! no LIST endpoint over a portal user's own listings and no HTTP surface for
//! the per-listing `get_listing_analytics` repo method — so the mobile
//! `MyListings` / `ListingAnalytics` screens and the web realtor dashboard
//! rendered permanent empty stubs (`GET /api/v1/my/listings` and
//! `GET /api/v1/my/listings/{id}/analytics` 404'd / 405'd on `main`).
//!
//! # IG3 (failing-on-main proof)
//!
//! `list_returns_only_owner_listings` and `analytics_owner_returns_200` both
//! drive routes that **did not exist on `main`** — the request would have
//! matched no route and returned 404. They pass only with the new handlers.
//! `list_excludes_other_users_listings` additionally locks the tenant/RLS
//! scoping: the LIST must never surface another portal user's rows.
//!
//! The harness mirrors `portal_listings_idor_tests.rs`: the real production
//! router (no mocking), real HS256 JWTs, and `#[sqlx::test(migrator =
//! "db::MIGRATOR")]` for an isolated migrated database.

#![allow(dead_code)]

use api_core::PrincipalClaims;
use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use jsonwebtoken::{encode, EncodingKey, Header};
use reality_server::{routes, state::AppState};
use sqlx::PgPool;
use std::sync::{Arc, Once};
use tower::ServiceExt;
use uuid::Uuid;

const TEST_JWT_SECRET: &str =
    "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

static TEST_ENV: Once = Once::new();

fn ensure_test_env() {
    TEST_ENV.call_once(|| {
        if std::env::var("RUST_ENV").is_err() {
            std::env::set_var("RUST_ENV", "development");
        }
        if std::env::var("JWT_SECRET").is_err() {
            std::env::set_var("JWT_SECRET", TEST_JWT_SECRET);
        }
    });
}

/// Real production portal-listings sub-router backed by `pool`.
fn listings_router(pool: PgPool) -> Router {
    ensure_test_env();

    let tenant_cache = Arc::new(api_core::TenantResolutionCache::new(60, 60, 1000));
    let rate_limiters = Arc::new(api_core::TenantRateLimiterSet::new());
    let state = AppState::new(pool, tenant_cache, rate_limiters);
    Router::new()
        .nest("/api/v1/my/listings", routes::portal_listings::router())
        .with_state(state)
}

/// Mint a real HS256 access token for `user_id` (portal `public` principal).
fn mint_token(user_id: Uuid) -> String {
    let claims = PrincipalClaims {
        sub: user_id,
        iat: 0,
        exp: i64::MAX,
        kind: Some("public".to_string()),
        token_type: Some("access".to_string()),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(TEST_JWT_SECRET.as_bytes()),
    )
    .expect("mint test token")
}

/// Seed a `public`-kind portal user (the kind `PortalPrincipal` admits).
async fn seed_portal_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', $2, 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(format!("{tag}@portal-mylistings.test"))
    .bind(format!("PortalMyListings {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_portal_user({tag}): {e}"))
}

/// Create a listing owned by `user_id` via the SECURITY DEFINER repo path.
async fn seed_listing(pool: &PgPool, user_id: Uuid, title: &str) -> Uuid {
    let repo = db::repositories::RealityPortalRepository::new(pool.clone());
    repo.create_portal_listing(
        user_id,
        title,
        Some("desc"),
        "apartment",
        "sale",
        rust_decimal::Decimal::new(100_000, 0),
        "EUR",
        "Test Street 1",
        "Bratislava",
        "81101",
        "SK",
        None,
        None,
        None,
        None,
    )
    .await
    .expect("seed_listing")
    .id
}

async fn body_json(app: &Router, req: Request<Body>) -> (StatusCode, serde_json::Value) {
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    };
    (status, json)
}

fn list_req(token: &str, query: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/my/listings{query}"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

fn analytics_req(id: Uuid, token: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/my/listings/{id}/analytics"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

// ============================================================================
// LIST — my listings
// ============================================================================

/// The owner's LIST returns exactly their own listings with a correct `total`.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_returns_only_owner_listings(pool: PgPool) {
    let user_a = seed_portal_user(&pool, "list-a").await;
    seed_listing(&pool, user_a, "A-one").await;
    seed_listing(&pool, user_a, "A-two").await;

    let app = listings_router(pool);
    let (status, json) = body_json(&app, list_req(&mint_token(user_a), "")).await;

    assert_eq!(status, StatusCode::OK, "owner LIST must return 200");
    assert_eq!(
        json["total"].as_i64(),
        Some(2),
        "total must count owned rows"
    );
    assert_eq!(
        json["listings"].as_array().map(|a| a.len()),
        Some(2),
        "owner must see both their listings"
    );
}

/// The LIST must never surface another portal user's listings (tenant/RLS
/// scoping): user B's listing must not appear in user A's LIST.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_excludes_other_users_listings(pool: PgPool) {
    let user_a = seed_portal_user(&pool, "excl-a").await;
    let user_b = seed_portal_user(&pool, "excl-b").await;
    seed_listing(&pool, user_a, "A-owned").await;
    seed_listing(&pool, user_b, "B-owned").await;
    seed_listing(&pool, user_b, "B-owned-2").await;

    let app = listings_router(pool);

    let (status_a, json_a) = body_json(&app, list_req(&mint_token(user_a), "")).await;
    assert_eq!(status_a, StatusCode::OK);
    assert_eq!(
        json_a["total"].as_i64(),
        Some(1),
        "A must only see their single listing, not B's"
    );

    let (status_b, json_b) = body_json(&app, list_req(&mint_token(user_b), "")).await;
    assert_eq!(status_b, StatusCode::OK);
    assert_eq!(
        json_b["total"].as_i64(),
        Some(2),
        "B must see exactly their two listings"
    );
}

/// The status filter narrows the LIST to matching listings.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_status_filter_applies(pool: PgPool) {
    let user = seed_portal_user(&pool, "filter-u").await;
    let draft_id = seed_listing(&pool, user, "draft-one").await;
    let paused_id = seed_listing(&pool, user, "to-pause").await;

    // Move one listing to `paused` via the SECURITY DEFINER update.
    let repo = db::repositories::RealityPortalRepository::new(pool.clone());
    repo.update_portal_listing(
        paused_id,
        user,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some("paused"),
        None,
    )
    .await
    .expect("pause listing");

    let app = listings_router(pool);

    let (status, json) = body_json(&app, list_req(&mint_token(user), "?status=paused")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        json["total"].as_i64(),
        Some(1),
        "only the paused listing must match status=paused"
    );
    let listings = json["listings"].as_array().unwrap();
    assert_eq!(listings.len(), 1);
    assert_eq!(
        listings[0]["id"].as_str(),
        Some(paused_id.to_string().as_str())
    );
    assert_ne!(
        listings[0]["id"].as_str(),
        Some(draft_id.to_string().as_str())
    );
}

/// Unauthenticated LIST → 401.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_unauthenticated_returns_401(pool: PgPool) {
    let app = listings_router(pool);
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/my/listings")
        .body(Body::empty())
        .unwrap();
    let (status, _) = body_json(&app, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Per-listing analytics
// ============================================================================

/// Owner reads analytics for their own listing → 200 with a zeroed summary
/// (no analytics rows have been recorded yet).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn analytics_owner_returns_200(pool: PgPool) {
    let user = seed_portal_user(&pool, "an-owner").await;
    let listing_id = seed_listing(&pool, user, "analytics-me").await;

    let app = listings_router(pool);
    let (status, json) = body_json(&app, analytics_req(listing_id, &mint_token(user))).await;

    assert_eq!(status, StatusCode::OK, "owner analytics must return 200");
    assert_eq!(
        json["listingId"].as_str(),
        Some(listing_id.to_string().as_str())
    );
    assert_eq!(json["totalViews"].as_i64(), Some(0));
    assert!(json["dailyAnalytics"].is_array());
}

/// Owner reads analytics for a portal listing **with recorded views** → 200 and
/// a non-zero summary. Regression lock for #2199: before the SECURITY DEFINER
/// read path (migration 00213) the org-scoped `listing_analytics` RLS policy
/// filtered every row for a portal listing, so the endpoint returned a zeroed
/// body even with seeded views. (Note: `#[sqlx::test]` connects as the superuser
/// which bypasses RLS, so the *deny* half is proven separately in
/// `db::tests::portal_listing_analytics_force_rls_tests`; this test locks the
/// end-to-end route wiring — seeded rows must surface, not read back empty.)
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn analytics_owner_with_views_returns_nonzero(pool: PgPool) {
    let user = seed_portal_user(&pool, "an-views").await;
    let listing_id = seed_listing(&pool, user, "analytics-views").await;

    // Seed a day of recorded views for the portal listing.
    sqlx::query(
        "INSERT INTO listing_analytics (listing_id, date, views, source_website) \
         VALUES ($1, CURRENT_DATE, 5, 5)",
    )
    .bind(listing_id)
    .execute(&pool)
    .await
    .expect("seed analytics row");

    let app = listings_router(pool);
    let (status, json) = body_json(&app, analytics_req(listing_id, &mint_token(user))).await;

    assert_eq!(status, StatusCode::OK, "owner analytics must return 200");
    assert_eq!(
        json["totalViews"].as_i64(),
        Some(5),
        "recorded views must surface in the analytics summary (#2199)"
    );
    assert_eq!(
        json["dailyAnalytics"].as_array().map(|a| a.len()),
        Some(1),
        "the seeded daily analytics row must appear in the series"
    );
}

/// User B requesting analytics for user A's listing → 404 (ownership gate),
/// never leaking that the listing exists.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn analytics_cross_user_returns_404(pool: PgPool) {
    let user_a = seed_portal_user(&pool, "an-a").await;
    let user_b = seed_portal_user(&pool, "an-b").await;
    let listing_id = seed_listing(&pool, user_a, "analytics-a").await;

    let app = listings_router(pool);
    let (status, _) = body_json(&app, analytics_req(listing_id, &mint_token(user_b))).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "cross-user analytics must 404"
    );
}
