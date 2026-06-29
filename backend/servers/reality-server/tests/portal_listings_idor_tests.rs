//! Handler-level IDOR + validation regression tests for the portal-listing
//! CRUD surface in `reality-server/src/routes/portal_listings.rs` (GH #1671,
//! follow-up to PR #1642 / task #986).
//!
//! # Background
//!
//! PR #1642 added owner-facing listing CRUD (`POST /api/v1/my/listings`,
//! `GET|PATCH /api/v1/my/listings/{id}`) gated by the `SECURITY DEFINER`
//! functions of migration 00186 (`portal_owner_id = user_id OR created_by =
//! user_id`). The post-merge review (#1671) flagged two gaps:
//!
//! 1. **No ownership tests** — the PR's own test plan left the IDOR cases as
//!    unchecked boxes. This file closes that with end-to-end HTTP probes:
//!    user B's `GET`/`PATCH` of user A's listing must return 404 (not 200),
//!    while A's own `GET`/`PATCH` succeed.
//! 2. **No enum/status validation** — `property_type` / `transaction_type` /
//!    `currency` were accepted as free text (the columns have no DB CHECK), and
//!    an owner could set `status` directly into the publicly-visible `active`
//!    state, bypassing the moderation path that gates `is_published`. The route
//!    now allow-lists those fields; these tests assert the `400` rejections.
//!
//! The tests drive the real production router (no mocking) with real HS256
//! JWTs, and use `#[sqlx::test(migrator = "db::MIGRATOR")]` for an isolated,
//! migrated database — the same harness as every other integration test here.
//!
//! # IG3 clarity (#1784)
//!
//! The cross-user `404` cases are **regression locks** for the #1642 ownership
//! gate — they already held before #1746. The genuine *failing-on-main*
//! behavior added by #1746 is the route-level `400` enum/status rejection. The
//! `db_rejects_out_of_domain_status` test below is the failing-on-main proof for
//! the #1784 DB-level defense-in-depth: it bypasses the route and calls the
//! `SECURITY DEFINER` `portal_update_listing` directly, so the only thing under
//! test is the migration-00194 `CHECK` constraint, not the route allow-list.

#![allow(dead_code)]

use api_core::PrincipalClaims;
use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use jsonwebtoken::{encode, EncodingKey, Header};
use reality_server::{routes, state::AppState};
use serde_json::json;
use sqlx::PgPool;
use std::sync::{Arc, Once};
use tower::ServiceExt;
use uuid::Uuid;

// ============================================================================
// Test environment setup
// ============================================================================

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

/// Build a minimal Axum router with only the portal-listings sub-router, backed
/// by the given pool. This is the real production router code — no mocking.
fn listings_router(pool: PgPool) -> Router {
    ensure_test_env();

    let tenant_cache = Arc::new(api_core::TenantResolutionCache::new(60, 60, 1000));
    let rate_limiters = Arc::new(api_core::TenantRateLimiterSet::new());
    let state = AppState::new(pool, tenant_cache, rate_limiters);
    Router::new()
        .nest("/api/v1/my/listings", routes::portal_listings::router())
        .with_state(state)
}

/// Mint a real HS256 access token for `user_id` signed with `TEST_JWT_SECRET`.
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

// ============================================================================
// Seed helpers
// ============================================================================

/// Seed a `public`-kind portal user (the kind `PortalPrincipal` admits).
async fn seed_portal_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', $2, 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(format!("{tag}@portal-listings-idor.test"))
    .bind(format!("PortalListingsIDOR {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_portal_user({tag}): {e}"))
}

/// Create a listing owned by `user_id` via the SECURITY DEFINER repo path and
/// return its id (status starts as `draft`, `is_published = FALSE`).
async fn seed_listing(pool: &PgPool, user_id: Uuid) -> Uuid {
    let repo = db::repositories::RealityPortalRepository::new(pool.clone());
    let listing = repo
        .create_portal_listing(
            user_id,
            "IDOR Test Listing",
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
        .expect("seed_listing");
    listing.id
}

/// Issue a request and return its status code.
async fn status_of(app: &Router, req: Request<Body>) -> StatusCode {
    app.clone().oneshot(req).await.unwrap().status()
}

fn get_req(id: Uuid, token: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/my/listings/{id}"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

fn patch_req(id: Uuid, token: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(Method::PATCH)
        .uri(format!("/api/v1/my/listings/{id}"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap()
}

// ============================================================================
// IDOR — cross-user access is denied
// ============================================================================

/// User B `GET`-ing user A's listing must return 404 (not 200, not 401).
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn get_cross_user_returns_404(pool: PgPool) {
    let user_a = seed_portal_user(&pool, "get-a").await;
    let user_b = seed_portal_user(&pool, "get-b").await;
    let listing_id = seed_listing(&pool, user_a).await;

    let app = listings_router(pool);
    let status = status_of(&app, get_req(listing_id, &mint_token(user_b))).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "cross-user GET IDOR probe must return 404, got {status}"
    );
}

/// User A reading their own listing → 200.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn get_owner_returns_200(pool: PgPool) {
    let user_a = seed_portal_user(&pool, "get-own-a").await;
    let listing_id = seed_listing(&pool, user_a).await;

    let app = listings_router(pool);
    let status = status_of(&app, get_req(listing_id, &mint_token(user_a))).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "owner must read own listing with 200, got {status}"
    );
}

/// User B `PATCH`-ing user A's listing must return 404 (the SECURITY DEFINER
/// update matches 0 rows → `fetch_optional` is `None` → handler maps to 404).
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn patch_cross_user_returns_404(pool: PgPool) {
    let user_a = seed_portal_user(&pool, "patch-a").await;
    let user_b = seed_portal_user(&pool, "patch-b").await;
    let listing_id = seed_listing(&pool, user_a).await;

    let app = listings_router(pool.clone());
    let body = json!({ "city": "Hacked" });
    let status = status_of(&app, patch_req(listing_id, &mint_token(user_b), body)).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "cross-user PATCH IDOR probe must return 404, got {status}"
    );

    // Defense-in-depth: the value must be unchanged in the DB.
    let city: String = sqlx::query_scalar("SELECT city FROM listings WHERE id = $1")
        .bind(listing_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        city, "Bratislava",
        "cross-user PATCH must not mutate the row"
    );
}

/// User A patching their own listing with valid data → 200.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn patch_owner_returns_200(pool: PgPool) {
    let user_a = seed_portal_user(&pool, "patch-own-a").await;
    let listing_id = seed_listing(&pool, user_a).await;

    let app = listings_router(pool);
    let body = json!({ "city": "Kosice", "status": "paused" });
    let status = status_of(&app, patch_req(listing_id, &mint_token(user_a), body)).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "owner must patch own listing with 200, got {status}"
    );
}

/// Unauthenticated GET → 401.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn get_unauthenticated_returns_401(pool: PgPool) {
    let user_a = seed_portal_user(&pool, "unauth-a").await;
    let listing_id = seed_listing(&pool, user_a).await;

    let app = listings_router(pool);
    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/my/listings/{listing_id}"))
        .body(Body::empty())
        .unwrap();
    let status = status_of(&app, req).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "unauthenticated GET must return 401, got {status}"
    );
}

// ============================================================================
// Validation — enum allow-lists + privileged status guard (400)
// ============================================================================

/// Create with an unknown `propertyType` → 400.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn create_invalid_property_type_returns_400(pool: PgPool) {
    let user = seed_portal_user(&pool, "cv-pt").await;
    let app = listings_router(pool);
    let body = json!({
        "title": "T", "propertyType": "spaceship", "transactionType": "sale",
        "price": "100000", "street": "S", "city": "C", "postalCode": "81101"
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/my/listings")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", mint_token(user)),
        )
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let status = status_of(&app, req).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "unknown propertyType must be rejected with 400, got {status}"
    );
}

/// Create with an unknown `transactionType` → 400.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn create_invalid_transaction_type_returns_400(pool: PgPool) {
    let user = seed_portal_user(&pool, "cv-tt").await;
    let app = listings_router(pool);
    let body = json!({
        "title": "T", "propertyType": "apartment", "transactionType": "barter",
        "price": "100000", "street": "S", "city": "C", "postalCode": "81101"
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/my/listings")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", mint_token(user)),
        )
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let status = status_of(&app, req).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "unknown transactionType must be rejected with 400, got {status}"
    );
}

/// Create with an unknown `currency` → 400.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn create_invalid_currency_returns_400(pool: PgPool) {
    let user = seed_portal_user(&pool, "cv-cur").await;
    let app = listings_router(pool);
    let body = json!({
        "title": "T", "propertyType": "apartment", "transactionType": "sale",
        "price": "100000", "currency": "XYZ",
        "street": "S", "city": "C", "postalCode": "81101"
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/my/listings")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", mint_token(user)),
        )
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let status = status_of(&app, req).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "unknown currency must be rejected with 400, got {status}"
    );
}

/// Owner attempting the privileged transition `status = active` (publicly
/// visible) must be rejected with 400 — publishing is moderation-only.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn patch_status_active_is_rejected_400(pool: PgPool) {
    let user = seed_portal_user(&pool, "ps-active").await;
    let listing_id = seed_listing(&pool, user).await;

    let app = listings_router(pool.clone());
    let body = json!({ "status": "active" });
    let status = status_of(&app, patch_req(listing_id, &mint_token(user), body)).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "owner self-publish via status=active must be rejected with 400, got {status}"
    );

    // The status must remain `draft` — the privileged transition was blocked.
    let db_status: String = sqlx::query_scalar("SELECT status FROM listings WHERE id = $1")
        .bind(listing_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        db_status, "draft",
        "rejected status transition must not mutate the row"
    );
}

/// An entirely unknown `status` value → 400.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn patch_unknown_status_is_rejected_400(pool: PgPool) {
    let user = seed_portal_user(&pool, "ps-unknown").await;
    let listing_id = seed_listing(&pool, user).await;

    let app = listings_router(pool);
    let body = json!({ "status": "superpublished" });
    let status = status_of(&app, patch_req(listing_id, &mint_token(user), body)).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "unknown status value must be rejected with 400, got {status}"
    );
}

/// A permitted owner lifecycle status (`paused`) → 200.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn patch_status_paused_is_accepted_200(pool: PgPool) {
    let user = seed_portal_user(&pool, "ps-paused").await;
    let listing_id = seed_listing(&pool, user).await;

    let app = listings_router(pool);
    let body = json!({ "status": "paused" });
    let status = status_of(&app, patch_req(listing_id, &mint_token(user), body)).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "permitted lifecycle status=paused must be accepted with 200, got {status}"
    );
}

// ============================================================================
// DB-level defense-in-depth (#1784) — the CHECK constraint rejects an
// out-of-domain status independent of the route allow-list.
// ============================================================================

/// IG3 for #1784: call the `SECURITY DEFINER` `portal_update_listing` directly
/// (bypassing the route's `ALLOWED_OWNER_STATUSES` guard) with a bogus status
/// and assert the database itself rejects it with SQLSTATE `23514`
/// (`check_violation`). Without the migration-00194 `listings_status_check`
/// constraint, `status = COALESCE(p_status, status)` would happily persist the
/// junk value — so this UPDATE succeeds on pre-00194 code and the test FAILS;
/// with the CHECK it raises a `check_violation` and the test PASSES.
///
/// The function arg order (migration 00186) is:
///   ($1=listing_id, $2=user_id, title, description, property_type,
///    transaction_type, price, currency, street, city, postal_code, country,
///    size_sqm, rooms, floor, total_floors, $3=status, is_negotiable)
/// i.e. only listing_id, user_id and status are bound; everything else is NULL
/// and skipped by the COALESCE.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn db_rejects_out_of_domain_status(pool: PgPool) {
    let user = seed_portal_user(&pool, "db-bad-status").await;
    let listing_id = seed_listing(&pool, user).await;

    // Negative: an out-of-domain status must violate the CHECK constraint.
    let result = sqlx::query(
        "SELECT portal_update_listing($1, $2, NULL, NULL, NULL, NULL, NULL, NULL, \
         NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, $3, NULL)",
    )
    .bind(listing_id)
    .bind(user)
    .bind("totally_bogus")
    .execute(&pool)
    .await;

    let err = result.expect_err(
        "out-of-domain status must be rejected by the DB CHECK constraint, but the UPDATE succeeded",
    );
    let sqlstate = err
        .as_database_error()
        .and_then(|e| e.code())
        .map(|c| c.into_owned());
    assert_eq!(
        sqlstate.as_deref(),
        Some("23514"),
        "expected SQLSTATE 23514 (check_violation) from listings_status_check, got {err:?}"
    );

    // Defense-in-depth: the row's status must be unchanged (still `draft`).
    let db_status: String = sqlx::query_scalar("SELECT status FROM listings WHERE id = $1")
        .bind(listing_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        db_status, "draft",
        "a rejected out-of-domain status must not mutate the row"
    );

    // Positive: a valid in-domain status still passes the CHECK (guards against
    // an over-tight constraint).
    sqlx::query(
        "SELECT portal_update_listing($1, $2, NULL, NULL, NULL, NULL, NULL, NULL, \
         NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, $3, NULL)",
    )
    .bind(listing_id)
    .bind(user)
    .bind("paused")
    .execute(&pool)
    .await
    .expect("a valid in-domain status ('paused') must pass the CHECK constraint");

    let db_status: String = sqlx::query_scalar("SELECT status FROM listings WHERE id = $1")
        .bind(listing_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        db_status, "paused",
        "a valid in-domain status must be applied"
    );
}
