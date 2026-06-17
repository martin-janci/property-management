//! Handler-level IDOR regression tests for `GET /api/v1/imports/jobs/{id}`
//! and `GET /api/v1/imports/feeds/{id}` (GH #1300, PR #1297 follow-up).
//!
//! # Background
//!
//! PR #1297 fixed an IDOR on all 7 by-id handlers in
//! `reality-server/src/routes/imports.rs` by threading `principal.user_id`
//! through every repo call (jobs key on `user_id`; feeds key on `agency_id`
//! which actually stores a user id — pre-existing column-name quirk). The
//! existing `backend/crates/db/tests/portal_imports_cross_org_idor_tests.rs`
//! exercises the repo SQL layer, but the IDOR lived in the *handler* (missing
//! `RequestPrincipal` extractor). A future regression that removes `principal`
//! from a handler signature would compile, ship, and bypass the repo tests.
//!
//! These tests drive the HTTP surface end-to-end via a real Axum router with
//! real HS256 JWTs, asserting:
//! - User B requesting User A's import job → 404 (not 200 / not 401).
//! - User A requesting their own import job → 200.
//! - Unauthenticated request → 401.
//! - User B requesting User A's feed → 404.
//! - User A requesting their own feed → 200.
//!
//! The test uses `#[sqlx::test]` for an isolated, migrated database (same
//! harness as every other integration test in this workspace).

use api_core::PrincipalClaims;
use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use db::models::{CreateFeedSubscription, CreatePortalImportJob};
use db::repositories::RealityPortalRepository;
use jsonwebtoken::{encode, EncodingKey, Header};
use reality_server::{routes, state::AppState};
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

/// Build a minimal Axum router with only the imports sub-router, backed by
/// the given pool. This is the real production router code — no mocking.
fn imports_router(pool: PgPool) -> Router {
    ensure_test_env();

    let tenant_cache = Arc::new(api_core::TenantResolutionCache::new(60, 60, 1000));
    let rate_limiters = Arc::new(api_core::TenantRateLimiterSet::new());
    let state = AppState::new(pool, tenant_cache, rate_limiters);
    Router::new()
        .nest("/api/v1/imports", routes::imports::router())
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

async fn seed_portal_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', $2, 'active', NOW(), 'platform')
        RETURNING id
        "#,
    )
    .bind(format!("{tag}@imports-idor.test"))
    .bind(format!("ImportsIDOR {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_portal_user({tag}): {e}"))
}

async fn seed_import_job(pool: &PgPool, user_id: Uuid) -> Uuid {
    let repo = RealityPortalRepository::new(pool.clone());
    let job = repo
        .create_import_job(
            user_id,
            CreatePortalImportJob {
                agency_id: None,
                source_type: "url".to_string(),
                source_url: Some("https://example.com/feed.xml".to_string()),
                source_filename: None,
            },
        )
        .await
        .expect("seed_import_job");
    job.id
}

/// Seed a `reality_agencies` row and return its id.
///
/// `feed_subscriptions.agency_id` carries a `REFERENCES reality_agencies(id)` FK
/// (migration 00063). In production the handler passes `principal.user_id` into
/// that column — a pre-existing data-model mismatch flagged in GH #1300 finding 2.
/// Until that column is reconciled, feed tests must seed a real agency row and
/// use its id as the "owner key" to satisfy the FK.
async fn seed_agency(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO reality_agencies (name, slug, email, status)
        VALUES ($1, $2, $3, 'verified')
        RETURNING id
        "#,
    )
    .bind(format!("ImportsIDOR Agency {tag}"))
    .bind(format!("imports-idor-agency-{tag}"))
    .bind(format!("{tag}@agency-imports-idor.test"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_agency({tag}): {e}"))
}

async fn seed_feed(pool: &PgPool, agency_id: Uuid) -> Uuid {
    let repo = RealityPortalRepository::new(pool.clone());
    let feed = repo
        .create_feed_subscription(
            agency_id,
            CreateFeedSubscription {
                name: "IDOR Test Feed Sub".to_string(),
                feed_url: "https://example.com/feed.xml".to_string(),
                feed_type: None,
                sync_interval: None,
            },
        )
        .await
        .expect("seed_feed");
    feed.id
}

// ============================================================================
// Tests — import jobs
// ============================================================================

/// Cross-tenant probe: user B requests user A's job → 404.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn import_job_cross_user_returns_404(pool: PgPool) {
    let user_a = seed_portal_user(&pool, "job-user-a").await;
    let user_b = seed_portal_user(&pool, "job-user-b").await;
    let job_id = seed_import_job(&pool, user_a).await;

    let app = imports_router(pool);
    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/imports/jobs/{job_id}"))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", mint_token(user_b)),
        )
        .body(Body::empty())
        .unwrap();

    let status = app.oneshot(req).await.unwrap().status();
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "cross-user IDOR probe must return 404, got {status}"
    );
}

/// Happy path: user A requests their own job → 200.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn import_job_owner_returns_200(pool: PgPool) {
    let user_a = seed_portal_user(&pool, "job-owner-a").await;
    let job_id = seed_import_job(&pool, user_a).await;

    let app = imports_router(pool);
    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/imports/jobs/{job_id}"))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", mint_token(user_a)),
        )
        .body(Body::empty())
        .unwrap();

    let status = app.oneshot(req).await.unwrap().status();
    assert_eq!(
        status,
        StatusCode::OK,
        "owner must read own job with 200, got {status}"
    );
}

/// Unauthenticated request → 401.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn import_job_unauthenticated_returns_401(pool: PgPool) {
    let user_a = seed_portal_user(&pool, "job-unauth-a").await;
    let job_id = seed_import_job(&pool, user_a).await;

    let app = imports_router(pool);
    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/imports/jobs/{job_id}"))
        .body(Body::empty())
        .unwrap();

    let status = app.oneshot(req).await.unwrap().status();
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "missing auth must yield 401, got {status}"
    );
}

// ============================================================================
// Tests — feed subscriptions
// ============================================================================

/// Cross-user probe for feeds.
///
/// The handler passes `principal.user_id` as the `agency_id` scope key (GH #1300
/// finding 2 pre-existing mismatch). The FK constraint requires a real
/// `reality_agencies` row, so we seed two agencies whose UUIDs are also registered
/// as portal users — meaning only a user whose UUID matches the agency id that owns
/// the feed will get a 200. Any other user gets 404.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn feed_cross_user_returns_404(pool: PgPool) {
    // Agency A owns the feed; agency B is the attacker.
    let agency_a = seed_agency(&pool, "feed-idor-a").await;
    let agency_b = seed_agency(&pool, "feed-idor-b").await;

    // Register the agencies' UUIDs as portal users so RequestPrincipal can
    // resolve their principal_kind from the users table.
    sqlx::query(
        r#"INSERT INTO users (id, email, password_hash, name, status, email_verified_at, principal_kind)
           VALUES ($1, $2, 'test_hash', 'Feed Agency A', 'active', NOW(), 'public'),
                  ($3, $4, 'test_hash', 'Feed Agency B', 'active', NOW(), 'public')"#,
    )
    .bind(agency_a)
    .bind(format!("feed-agency-a-{agency_a}@test"))
    .bind(agency_b)
    .bind(format!("feed-agency-b-{agency_b}@test"))
    .execute(&pool)
    .await
    .expect("register agency UUIDs as portal users");

    let feed_id = seed_feed(&pool, agency_a).await;

    let app = imports_router(pool);
    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/imports/feeds/{feed_id}"))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", mint_token(agency_b)),
        )
        .body(Body::empty())
        .unwrap();

    let status = app.oneshot(req).await.unwrap().status();
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "cross-agency feed IDOR probe must return 404, got {status}"
    );
}

/// Happy path: feed owner reads their own feed → 200.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn feed_owner_returns_200(pool: PgPool) {
    let agency_a = seed_agency(&pool, "feed-owner-a").await;

    sqlx::query(
        r#"INSERT INTO users (id, email, password_hash, name, status, email_verified_at, principal_kind)
           VALUES ($1, $2, 'test_hash', 'Feed Owner A', 'active', NOW(), 'public')"#,
    )
    .bind(agency_a)
    .bind(format!("feed-owner-a-{agency_a}@test"))
    .execute(&pool)
    .await
    .expect("register agency UUID as portal user");

    let feed_id = seed_feed(&pool, agency_a).await;

    let app = imports_router(pool);
    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/imports/feeds/{feed_id}"))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", mint_token(agency_a)),
        )
        .body(Body::empty())
        .unwrap();

    let status = app.oneshot(req).await.unwrap().status();
    assert_eq!(
        status,
        StatusCode::OK,
        "feed owner must read own feed with 200, got {status}"
    );
}
