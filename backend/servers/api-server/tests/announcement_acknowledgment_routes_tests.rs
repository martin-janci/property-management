//! Integration tests for the announcement read/acknowledge HTTP surface
//! (Story 6.2 — Coverage 6-2).
//!
//! # What is tested
//!
//! 1. **Mark-read endpoint** — `POST /api/v1/announcements/{id}/read`
//!    - Rejects unauthenticated requests (401).
//!    - Returns 200 for an authenticated member; persists a row in
//!      `announcement_reads` without an `acknowledged_at`.
//!    - Second call is idempotent (ON CONFLICT DO NOTHING semantics).
//!
//! 2. **Acknowledge endpoint** — `POST /api/v1/announcements/{id}/acknowledge`
//!    - Rejects unauthenticated requests (401).
//!    - Returns 200 for an authenticated member; persists `acknowledged_at`
//!      in `announcement_reads`.
//!    - Idempotent: calling again does NOT overwrite the original
//!      `acknowledged_at` (COALESCE guard in the repo).
//!
//! 3. **Acknowledgments ledger endpoint** — `GET /api/v1/announcements/{id}/acknowledgments`
//!    - Rejects unauthenticated requests (401).
//!    - Returns 403 for a non-manager role (resident).
//!    - Returns 404 when the announcement UUID does not exist.
//!    - Returns 200 for a manager; body contains the `stats` field.
//!    - After mark-read + acknowledge: stats show `read_count ≥ 1`,
//!      `acknowledged_count ≥ 1`.
//!
//! # Scope
//!
//! These tests run end-to-end against a real Postgres database (provisioned by
//! `#[sqlx::test(migrator = "db::MIGRATOR")]`) with the full Axum router wired
//! up through `TestApp`. They do NOT make external HTTP calls and have no
//! dependency on S3, Redis, or the email service.

#[allow(dead_code)]
mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use common::TestApp;

// ──────────────────────────────────────────────────────────────────────────────
// Constants
// ──────────────────────────────────────────────────────────────────────────────

/// Must match `TestConfig::default().jwt_secret`.
const JWT_SECRET: &str =
    "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

// ──────────────────────────────────────────────────────────────────────────────
// JWT helpers
// ──────────────────────────────────────────────────────────────────────────────

/// Mirror of `api_core::extractors::auth::Claims`.
#[derive(Serialize)]
struct AccessClaims {
    sub: Uuid,
    exp: i64,
    iat: i64,
    token_type: String,
    tenant_id: Option<Uuid>,
    role: Option<String>,
    email: String,
    name: String,
}

/// Mint a short-lived access token for `user_id` scoped to `tenant_id` with the
/// given `role` string (e.g. `"manager"`, `"resident"`).
fn mint_token(user_id: Uuid, tenant_id: Uuid, role: &str) -> String {
    let now = Utc::now();
    let claims = AccessClaims {
        sub: user_id,
        iat: now.timestamp(),
        exp: (now + Duration::hours(1)).timestamp(),
        token_type: "access".to_string(),
        tenant_id: Some(tenant_id),
        role: Some(role.to_string()),
        email: "ack-routes-test@test.local".to_string(),
        name: "Ack Routes Test".to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("mint access token")
}

// ──────────────────────────────────────────────────────────────────────────────
// Database seed helpers
// ──────────────────────────────────────────────────────────────────────────────

/// Insert an active organization and return its id.
async fn seed_org(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("Ack Routes Test Org {tag}"))
    .bind(format!("ack-routes-{tag}-{}", Uuid::new_v4().simple()))
    .bind(format!("{tag}@ack-routes.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

/// Insert an active, email-verified user and return its id.
async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'hash', 'Ack Test User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Make `user_id` an active `role_type` member of `org_id` in
/// `organization_members` (used by `ValidatedTenantExtractor`/`is_member`).
/// Also inserts the matching `user_memberships` row (used by the acknowledgment
/// stats query that counts targeted users).
async fn seed_membership(pool: &PgPool, org_id: Uuid, user_id: Uuid, role: &str) {
    // Legacy table – checked by ValidatedTenantExtractor.
    sqlx::query(
        r#"
        INSERT INTO organization_members (organization_id, user_id, role_type, status)
        VALUES ($1, $2, $3, 'active')
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .bind(role)
    .execute(pool)
    .await
    .expect("seed organization_members");

    // Unified membership spine – used by `get_acknowledgment_stats_rls` to
    // count `total_targeted` (migration 00128).
    sqlx::query(
        r#"
        INSERT INTO user_memberships (user_id, organization_id, role)
        VALUES ($1, $2, $3)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(user_id)
    .bind(org_id)
    .bind(role)
    .execute(pool)
    .await
    .expect("seed user_memberships");
}

/// Insert a published announcement owned by `org_id` and authored by
/// `author_id`. Returns the new announcement's UUID.
async fn seed_published_announcement(pool: &PgPool, org_id: Uuid, author_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO announcements
            (organization_id, author_id, title, content,
             target_type, target_ids, status, published_at)
        VALUES ($1, $2, 'Test announcement', 'Body text',
                'all', '[]'::jsonb, 'published', NOW())
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(author_id)
    .fetch_one(pool)
    .await
    .expect("seed published announcement")
}

// ──────────────────────────────────────────────────────────────────────────────
// Request helpers
// ──────────────────────────────────────────────────────────────────────────────

fn anon_post(uri: &str) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::empty())
        .unwrap()
}

fn anon_get(uri: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

fn authed_post(uri: &str, token: &str, org_id: Uuid) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org_id.to_string())
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::empty())
        .unwrap()
}

fn authed_get(uri: &str, token: &str, org_id: Uuid) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org_id.to_string())
        .body(Body::empty())
        .unwrap()
}

// ──────────────────────────────────────────────────────────────────────────────
// Helper: verify the announcement_reads ledger directly in the DB
// ──────────────────────────────────────────────────────────────────────────────

/// Return `(read_at IS NOT NULL, acknowledged_at IS NOT NULL)` for the given
/// (announcement, user) pair, or `None` if no row exists.
async fn read_record(
    pool: &PgPool,
    announcement_id: Uuid,
    user_id: Uuid,
) -> Option<(bool, bool)> {
    sqlx::query_as::<_, (bool, bool)>(
        r#"
        SELECT
            read_at IS NOT NULL,
            acknowledged_at IS NOT NULL
        FROM announcement_reads
        WHERE announcement_id = $1 AND user_id = $2
        "#,
    )
    .bind(announcement_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .expect("query announcement_reads")
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests — mark-read endpoint
// ══════════════════════════════════════════════════════════════════════════════

/// Unauthenticated POST must be rejected with 401 (no bearer token).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mark_read_rejects_unauthenticated(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "mr-unauth").await;
    let announcement_id = Uuid::new_v4(); // need not exist — auth check fires first
    let uri = format!("/api/v1/announcements/{announcement_id}/read");
    let resp = app.execute(anon_post(&uri)).await;
    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "unauthenticated mark-read must be 401; got {}: {}",
        resp.status,
        resp.text()
    );
    // Suppress unused variable warning from seed_org (org_id used for clarity)
    let _ = org_id;
}

/// Authenticated member can mark an announcement as read; the ledger row is
/// persisted with `read_at IS NOT NULL` and `acknowledged_at IS NULL`.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mark_read_persists_read_record(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "mr-happy").await;
    let user_id = seed_user(&pool, "mr-happy@ack-routes.test").await;
    seed_membership(&pool, org_id, user_id, "resident").await;
    let announcement_id = seed_published_announcement(&pool, org_id, user_id).await;

    let token = mint_token(user_id, org_id, "resident");
    let uri = format!("/api/v1/announcements/{announcement_id}/read");
    let resp = app.execute(authed_post(&uri, &token, org_id)).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "mark-read must return 200; got {}: {}",
        resp.status,
        resp.text()
    );

    // Verify ledger persistence.
    let row = read_record(&pool, announcement_id, user_id).await;
    assert!(
        row.is_some(),
        "announcement_reads row must exist after mark-read"
    );
    let (has_read_at, has_acked_at) = row.unwrap();
    assert!(has_read_at, "read_at must be set after mark-read");
    assert!(!has_acked_at, "acknowledged_at must NOT be set by mark-read");
}

/// Calling mark-read a second time is idempotent — the endpoint returns 200
/// and the `acknowledged_at` column remains NULL (ON CONFLICT preserves the
/// original `read_at`).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mark_read_is_idempotent(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "mr-idem").await;
    let user_id = seed_user(&pool, "mr-idem@ack-routes.test").await;
    seed_membership(&pool, org_id, user_id, "resident").await;
    let announcement_id = seed_published_announcement(&pool, org_id, user_id).await;
    let token = mint_token(user_id, org_id, "resident");
    let uri = format!("/api/v1/announcements/{announcement_id}/read");

    // First call.
    let resp1 = app.execute(authed_post(&uri, &token, org_id)).await;
    assert_eq!(resp1.status, StatusCode::OK, "first mark-read must be 200");

    // Second call — must also succeed without error.
    let resp2 = app.execute(authed_post(&uri, &token, org_id)).await;
    assert_eq!(
        resp2.status,
        StatusCode::OK,
        "second mark-read must be idempotent 200; got {}: {}",
        resp2.status,
        resp2.text()
    );

    // Exactly one row in the ledger.
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM announcement_reads WHERE announcement_id = $1 AND user_id = $2",
    )
    .bind(announcement_id)
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("count announcement_reads");
    assert_eq!(count, 1, "idempotent mark-read must not create duplicate rows");
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests — acknowledge endpoint
// ══════════════════════════════════════════════════════════════════════════════

/// Unauthenticated POST must be rejected with 401.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn acknowledge_rejects_unauthenticated(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let announcement_id = Uuid::new_v4();
    let uri = format!("/api/v1/announcements/{announcement_id}/acknowledge");
    let resp = app.execute(anon_post(&uri)).await;
    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "unauthenticated acknowledge must be 401; got {}: {}",
        resp.status,
        resp.text()
    );
}

/// Authenticated member can acknowledge an announcement; the ledger row is
/// persisted with both `read_at IS NOT NULL` and `acknowledged_at IS NOT NULL`.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn acknowledge_persists_acknowledgment(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "ack-happy").await;
    let user_id = seed_user(&pool, "ack-happy@ack-routes.test").await;
    seed_membership(&pool, org_id, user_id, "resident").await;
    let announcement_id = seed_published_announcement(&pool, org_id, user_id).await;

    let token = mint_token(user_id, org_id, "resident");
    let uri = format!("/api/v1/announcements/{announcement_id}/acknowledge");
    let resp = app.execute(authed_post(&uri, &token, org_id)).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "acknowledge must return 200; got {}: {}",
        resp.status,
        resp.text()
    );

    // Verify ledger persistence.
    let row = read_record(&pool, announcement_id, user_id).await;
    assert!(
        row.is_some(),
        "announcement_reads row must exist after acknowledge"
    );
    let (has_read_at, has_acked_at) = row.unwrap();
    assert!(has_read_at, "read_at must be set after acknowledge");
    assert!(has_acked_at, "acknowledged_at must be set after acknowledge");
}

/// Acknowledge is idempotent: a second call MUST NOT overwrite the original
/// `acknowledged_at` (the `COALESCE(acknowledged_at, NOW())` guard in
/// `AnnouncementRepository::acknowledge_rls`).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn acknowledge_is_idempotent_and_preserves_original_timestamp(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "ack-idem").await;
    let user_id = seed_user(&pool, "ack-idem@ack-routes.test").await;
    seed_membership(&pool, org_id, user_id, "resident").await;
    let announcement_id = seed_published_announcement(&pool, org_id, user_id).await;
    let token = mint_token(user_id, org_id, "resident");
    let uri = format!("/api/v1/announcements/{announcement_id}/acknowledge");

    // First acknowledgment.
    let resp1 = app.execute(authed_post(&uri, &token, org_id)).await;
    assert_eq!(resp1.status, StatusCode::OK, "first acknowledge must be 200");

    // Capture the original timestamp from the ledger.
    let first_ts: chrono::DateTime<Utc> = sqlx::query_scalar(
        "SELECT acknowledged_at FROM announcement_reads \
         WHERE announcement_id = $1 AND user_id = $2",
    )
    .bind(announcement_id)
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("fetch acknowledged_at after first call");

    // Second acknowledgment — must succeed without overwriting the timestamp.
    let resp2 = app.execute(authed_post(&uri, &token, org_id)).await;
    assert_eq!(
        resp2.status,
        StatusCode::OK,
        "second acknowledge must be idempotent 200; got {}: {}",
        resp2.status,
        resp2.text()
    );

    let second_ts: chrono::DateTime<Utc> = sqlx::query_scalar(
        "SELECT acknowledged_at FROM announcement_reads \
         WHERE announcement_id = $1 AND user_id = $2",
    )
    .bind(announcement_id)
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("fetch acknowledged_at after second call");

    assert_eq!(
        first_ts, second_ts,
        "idempotent acknowledge must preserve original acknowledged_at"
    );

    // Exactly one row in the ledger.
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM announcement_reads WHERE announcement_id = $1 AND user_id = $2",
    )
    .bind(announcement_id)
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("count announcement_reads");
    assert_eq!(
        count, 1,
        "idempotent acknowledge must not create duplicate rows"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests — acknowledgment ledger/stats endpoint
// ══════════════════════════════════════════════════════════════════════════════

/// Unauthenticated GET must be rejected with 401.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_acknowledgments_rejects_unauthenticated(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let announcement_id = Uuid::new_v4();
    let uri = format!("/api/v1/announcements/{announcement_id}/acknowledgments");
    let resp = app.execute(anon_get(&uri)).await;
    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "unauthenticated acknowledgments GET must be 401; got {}: {}",
        resp.status,
        resp.text()
    );
}

/// Non-manager role (resident) must be rejected with 403.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_acknowledgments_rejects_non_manager(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "ackls-rbac").await;
    let user_id = seed_user(&pool, "ackls-rbac@ack-routes.test").await;
    seed_membership(&pool, org_id, user_id, "resident").await;
    let announcement_id = seed_published_announcement(&pool, org_id, user_id).await;

    let token = mint_token(user_id, org_id, "resident");
    let uri = format!("/api/v1/announcements/{announcement_id}/acknowledgments");
    let resp = app.execute(authed_get(&uri, &token, org_id)).await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "resident must be forbidden from acknowledgments list; got {}: {}",
        resp.status,
        resp.text()
    );
}

/// Manager calling with a nonexistent announcement UUID must get 404.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_acknowledgments_returns_404_for_missing_announcement(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "ackls-miss").await;
    let user_id = seed_user(&pool, "ackls-miss@ack-routes.test").await;
    seed_membership(&pool, org_id, user_id, "manager").await;

    let token = mint_token(user_id, org_id, "manager");
    let missing_id = Uuid::new_v4();
    let uri = format!("/api/v1/announcements/{missing_id}/acknowledgments");
    let resp = app.execute(authed_get(&uri, &token, org_id)).await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "missing announcement must return 404; got {}: {}",
        resp.status,
        resp.text()
    );
}

/// Manager can retrieve acknowledgment stats; body must contain a `stats` field.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_acknowledgments_returns_stats_for_manager(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "ackls-ok").await;
    let manager_id = seed_user(&pool, "ackls-ok-mgr@ack-routes.test").await;
    seed_membership(&pool, org_id, manager_id, "manager").await;
    let announcement_id = seed_published_announcement(&pool, org_id, manager_id).await;

    let token = mint_token(manager_id, org_id, "manager");
    let uri = format!("/api/v1/announcements/{announcement_id}/acknowledgments");
    let resp = app.execute(authed_get(&uri, &token, org_id)).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "manager must get 200 from acknowledgments; got {}: {}",
        resp.status,
        resp.text()
    );

    let body = resp.json_value();
    assert!(
        body.get("stats").is_some(),
        "response must contain a 'stats' field; body: {body}"
    );
}

/// After a read and then an acknowledgment, the stats must reflect at least one
/// read and one acknowledgment.
///
/// This is the key end-to-end persistence test: it drives the full
/// mark-read → acknowledge → stats flow and verifies the ledger counts.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn acknowledgment_stats_reflect_read_and_ack(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "ackls-e2e").await;
    let manager_id = seed_user(&pool, "ackls-e2e-mgr@ack-routes.test").await;
    let resident_id = seed_user(&pool, "ackls-e2e-res@ack-routes.test").await;
    seed_membership(&pool, org_id, manager_id, "manager").await;
    seed_membership(&pool, org_id, resident_id, "resident").await;
    let announcement_id = seed_published_announcement(&pool, org_id, manager_id).await;

    // Resident marks as read.
    let res_token = mint_token(resident_id, org_id, "resident");
    let read_uri = format!("/api/v1/announcements/{announcement_id}/read");
    let read_resp = app.execute(authed_post(&read_uri, &res_token, org_id)).await;
    assert_eq!(
        read_resp.status,
        StatusCode::OK,
        "mark-read must succeed; got {}: {}",
        read_resp.status,
        read_resp.text()
    );

    // Resident acknowledges.
    let ack_uri = format!("/api/v1/announcements/{announcement_id}/acknowledge");
    let ack_resp = app.execute(authed_post(&ack_uri, &res_token, org_id)).await;
    assert_eq!(
        ack_resp.status,
        StatusCode::OK,
        "acknowledge must succeed; got {}: {}",
        ack_resp.status,
        ack_resp.text()
    );

    // Manager queries the acknowledgment stats.
    let mgr_token = mint_token(manager_id, org_id, "manager");
    let stats_uri = format!("/api/v1/announcements/{announcement_id}/acknowledgments");
    let stats_resp = app.execute(authed_get(&stats_uri, &mgr_token, org_id)).await;
    assert_eq!(
        stats_resp.status,
        StatusCode::OK,
        "stats must return 200; got {}: {}",
        stats_resp.status,
        stats_resp.text()
    );

    let body = stats_resp.json_value();
    let stats = body.get("stats").expect("stats field must be present");

    let read_count = stats
        .get("read_count")
        .and_then(|v| v.as_i64())
        .expect("stats.read_count must be an integer");
    let ack_count = stats
        .get("acknowledged_count")
        .and_then(|v| v.as_i64())
        .expect("stats.acknowledged_count must be an integer");

    assert!(
        read_count >= 1,
        "read_count must be ≥ 1 after mark-read; got {read_count}"
    );
    assert!(
        ack_count >= 1,
        "acknowledged_count must be ≥ 1 after acknowledge; got {ack_count}"
    );
}
