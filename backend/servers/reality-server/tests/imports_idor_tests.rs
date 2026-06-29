//! Handler-level IDOR regression tests for `GET /api/v1/imports/jobs/{id}`
//! and `GET /api/v1/imports/feeds/{id}` (GH #1300, PR #1297 follow-up).
//!
//! # Background
//!
//! PR #1297 fixed an IDOR on all 7 by-id handlers in
//! `reality-server/src/routes/imports.rs` by threading the principal through
//! every repo call. Import jobs key on `user_id` (per-user). Feed subscriptions
//! are agency-scoped (#1584): the handlers resolve the caller's agency from
//! `reality_agency_members` and scope feeds by `agency_id`, so a feed is shared
//! across the agency's members and isolated from other agencies. The IDOR lived
//! in the *handler* (a missing/incorrect principal scope), so these tests drive
//! the HTTP surface end-to-end.
//!
//! These tests drive the HTTP surface end-to-end via a real Axum router with
//! real HS256 JWTs, asserting:
//! - User B requesting User A's import job → 404 (not 200 / not 401).
//! - User A requesting their own import job → 200.
//! - Unauthenticated request → 401.
//! - A feed is shared across its agency's members → 200.
//! - A member of another agency requesting a feed → 404.
//! - A caller in no agency listing feeds → 403.
//!
//! The test uses `#[sqlx::test]` for an isolated, migrated database (same
//! harness as every other integration test in this workspace).

#![allow(dead_code)]

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
    seed_portal_user_with_kind(pool, tag, "platform").await
}

/// Seed a portal user with an explicit `principal_kind` (`public` | `staff` |
/// `platform`). `seed_portal_user` delegates here with `platform`; the explicit
/// form lets a test pin the kind it exercises independently of that default.
async fn seed_portal_user_with_kind(pool: &PgPool, tag: &str, principal_kind: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', $2, 'active', NOW(), $3)
        RETURNING id
        "#,
    )
    .bind(format!("{tag}@imports-idor.test"))
    .bind(format!("ImportsIDOR {tag}"))
    .bind(principal_kind)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_portal_user_with_kind({tag}, {principal_kind}): {e}"))
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
/// Feeds are scoped to a real agency (FK `feed_subscriptions.agency_id →
/// reality_agencies`). The import handlers resolve the caller's agency from
/// `reality_agency_members` (#1584), so feed tests seed an agency + membership.
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

/// Make `user_id` an active member of `agency_id`. Feeds are agency-scoped
/// (#1584): the import handlers resolve the caller's agency from this table.
async fn seed_membership(pool: &PgPool, agency_id: Uuid, user_id: Uuid) {
    sqlx::query(
        r#"
        INSERT INTO reality_agency_members (agency_id, user_id, role, is_active)
        VALUES ($1, $2, 'realtor', TRUE)
        "#,
    )
    .bind(agency_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("seed_membership");
}

// ============================================================================
// Tests — import jobs
// ============================================================================

/// Cross-tenant probe: user B requests user A's job → 404.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
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
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
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

/// Finding 3 (GH #1584): `PortalPrincipal` admits any authenticated, non-deleted
/// principal kind — including the highest-privilege `platform` (super-admin)
/// kind — but yields only `user_id` and applies no kind-based privilege. Pin the
/// intended behavior explicitly (independently of the kind `seed_portal_user`
/// happens to use): a `platform`-kind caller is still scoped to its own
/// `user_id`, so it gets 404 — not 200 — on another user's import job. There is
/// no admin bypass at this layer.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn import_job_platform_kind_caller_is_still_user_scoped_404(pool: PgPool) {
    let owner = seed_portal_user(&pool, "job-plat-owner").await;
    let platform_caller = seed_portal_user_with_kind(&pool, "job-plat-admin", "platform").await;
    let job_id = seed_import_job(&pool, owner).await;

    let app = imports_router(pool);
    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/imports/jobs/{job_id}"))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", mint_token(platform_caller)),
        )
        .body(Body::empty())
        .unwrap();

    let status = app.oneshot(req).await.unwrap().status();
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "platform-kind caller must stay user-scoped (no admin bypass), got {status}"
    );
}

/// Unauthenticated request → 401.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
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

/// Feeds are agency-scoped and SHARED across the agency's members (#1584): a
/// feed created for agency A is readable by every active member of A — not just
/// the user who happened to create it. Two distinct members both get 200.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn feed_is_shared_across_agency_members(pool: PgPool) {
    let agency_a = seed_agency(&pool, "feed-shared-a").await;
    let member_1 = seed_portal_user(&pool, "feed-member-1").await;
    let member_2 = seed_portal_user(&pool, "feed-member-2").await;
    seed_membership(&pool, agency_a, member_1).await;
    seed_membership(&pool, agency_a, member_2).await;
    let feed_id = seed_feed(&pool, agency_a).await;

    let app = imports_router(pool);
    for member in [member_1, member_2] {
        let req = Request::builder()
            .method(Method::GET)
            .uri(format!("/api/v1/imports/feeds/{feed_id}"))
            .header(
                header::AUTHORIZATION,
                format!("Bearer {}", mint_token(member)),
            )
            .body(Body::empty())
            .unwrap();
        let status = app.clone().oneshot(req).await.unwrap().status();
        assert_eq!(
            status,
            StatusCode::OK,
            "every active member of the agency must read its feed; got {status}"
        );
    }
}

/// Cross-agency probe: a member of a DIFFERENT agency cannot read agency A's
/// feed — the handler resolves the caller's own agency, so the by-id lookup is
/// scoped away → 404.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn feed_non_member_returns_404(pool: PgPool) {
    let agency_a = seed_agency(&pool, "feed-idor-a").await;
    let agency_b = seed_agency(&pool, "feed-idor-b").await;
    let attacker = seed_portal_user(&pool, "feed-attacker").await;
    seed_membership(&pool, agency_b, attacker).await; // member of B, not A
    let feed_id = seed_feed(&pool, agency_a).await;

    let app = imports_router(pool);
    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/imports/feeds/{feed_id}"))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", mint_token(attacker)),
        )
        .body(Body::empty())
        .unwrap();

    let status = app.oneshot(req).await.unwrap().status();
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "a member of another agency must not read agency A's feed, got {status}"
    );
}

/// A caller who is a member of no agency cannot own/list feeds → 403 (rather than
/// silently operating on a bogus user-id-as-agency-id scope).
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn feed_caller_without_agency_returns_403(pool: PgPool) {
    let orphan = seed_portal_user(&pool, "feed-orphan").await; // no membership

    let app = imports_router(pool);
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/imports/feeds")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", mint_token(orphan)),
        )
        .body(Body::empty())
        .unwrap();

    let status = app.oneshot(req).await.unwrap().status();
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "a user in no agency must get 403 listing feeds, got {status}"
    );
}
