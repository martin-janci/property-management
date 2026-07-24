//! Regression tests for the reports CRUD cluster IDOR/scope fixes.
//!
//! Closes:
//!   - #614  RequireCapability / role check on PUT /api/v1/reports/schedules/{id}
//!   - #624  Cross-tenant mutation via missing org_id in WHERE (update_schedule)
//!   - #646  pause_schedule and resume_schedule IDOR (no org scope in WHERE)
//!   - #647  list_executions / get_execution / get_execution_download_url /
//!     retry_execution IDOR (no org scope)
//!   - #693  The original cross-tenant tests below sent only `X-Tenant-ID`
//!     (no Bearer JWT), so `AuthUser`/`RlsConnection` returned 401 at the
//!     outer gate and the `AND organization_id = $caller_org_id` clause in
//!     `pause`, `resume`, `get_by_id_scoped`, `get_execution_scoped`, and
//!     `retry_execution_scoped` was never exercised. The companion
//!     `*_authenticated` tests below send a real Bearer JWT from a
//!     manager/resident in `org_b` targeting an `org_a` resource, asserting
//!     the org-scoped WHERE clause produces a 404 — proving the DB-layer
//!     isolation fires, not the auth gate.
//!
//! # Audit-matrix note
//!
//! The PR #660 audit matrix listed six sibling handlers: `update_schedule`,
//! `pause`, `resume`, `list_executions`, `get_execution`,
//! `get_execution_download_url`, `retry_execution`. There is **no**
//! `delete_schedule` handler in `routes/reports.rs` (the only `delete_schedule`
//! handlers in the codebase live in `routes/work_orders.rs` and
//! `routes/government_portal.rs`, which are unrelated clusters). Finding 2 of
//! #693 is therefore resolved as "no audit-matrix comment to update" — no
//! phantom entry exists in this file or in `routes/reports.rs`.
//!
//! # What these tests verify
//!
//! Each mutating or data-fetching handler in the schedule/execution cluster
//! must:
//! 1. Require the caller to be authenticated (401 for missing auth) — the
//!    `*_without_auth_is_rejected` tests prove the outer JWT gate works.
//! 2. Require at least manager-tier role for mutating operations (403) —
//!    covered by `report_schedule_rbac_tests.rs`.
//! 3. Prevent cross-tenant IDOR even with a valid Bearer JWT — the
//!    `*_cross_tenant_authenticated_*` tests prove the DB-layer org scope
//!    fires by asserting a strict 404 from a real authenticated request.
//!
//! # Why both unauth and authenticated tests
//!
//! The unauth tests cover the auth gate (legitimate outer-gate coverage). The
//! authenticated companions cover the org-scoped WHERE clause (the real fix
//! in #624/#646/#647). Both must keep passing for the security contract to
//! hold end-to-end.

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user, seed_membership, TestApp, TestUser};

// ---------------------------------------------------------------------------
// Seed helpers (shared with report_schedule_rbac_tests if/when merged)
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("Sched Sibling Org {slug}"))
    .bind(format!("sched-sibling-org-{slug}"))
    .bind(format!("{slug}@sched-sibling.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Sibling Test User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_schedule(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO report_schedules
            (report_id, organization_id, name, frequency, time, timezone, format, recipients, is_active, status)
        VALUES
            (gen_random_uuid(), $1, 'Sibling Test Schedule', 'weekly', '09:00', 'UTC', 'pdf',
             '["owner@example.com"]', true, 'active')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed report schedule")
}

async fn seed_execution(pool: &PgPool, schedule_id: Uuid, status: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO report_executions
            (schedule_id, status, started_at, created_at)
        VALUES
            ($1, $2, NOW(), NOW())
        RETURNING id
        "#,
    )
    .bind(schedule_id)
    .bind(status)
    .fetch_one(pool)
    .await
    .expect("seed report execution")
}

/// Build a request with only `X-Tenant-ID` (no JWT — triggers 401 at auth gate).
fn no_auth_req(method: Method, uri: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::empty())
        .unwrap()
}

/// Build a request with `X-Tenant-ID` from org_id, but no JWT.
fn tenant_req(method: Method, uri: &str, org_id: Uuid) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("X-Tenant-ID", org_id.to_string())
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::empty())
        .unwrap()
}

/// Look up a user id by email — `create_authenticated_user` registers via
/// `POST /api/v1/auth/register` so we read back the id to attach a membership.
async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("fetch user id by email")
}

/// Assert the response is a 4xx rejection (not a success).
fn assert_rejected(status: StatusCode, label: &str) {
    let code = status.as_u16();
    assert!(
        (400..500).contains(&code),
        "{label}: expected 4xx rejection, got {status}"
    );
}

// ===========================================================================
// pause_schedule (#646)
// ===========================================================================

/// Unauthenticated request to pause a schedule must be rejected.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pause_schedule_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "pause-noauth").await;
    let schedule = seed_schedule(&pool, org).await;

    let req = no_auth_req(
        Method::PUT,
        &format!("/api/v1/reports/schedules/{}/pause", schedule),
    );
    let response = app.execute(req).await;
    assert_rejected(response.status, "pause: no-auth must be rejected (#646)");
}

/// A request with wrong-org tenant header cannot pause another org's schedule.
///
/// In production (valid JWT for org_b), the repo UPDATE WHERE includes
/// `AND organization_id = org_b` which finds no row → 404.
/// In TestApp (no JWT) the auth gate fires first → 401.
/// Either way the cross-tenant mutation is never applied.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pause_schedule_cross_tenant_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "pause-ctor-a").await;
    let schedule_in_a = seed_schedule(&pool, org_a).await;

    let org_b = seed_org(&pool, "pause-ctor-b").await;
    let user_b = seed_user(&pool, "attacker-pause@sched-sibling.test").await;
    seed_membership(&pool, org_b, user_b, "Manager").await;

    // Attacker from org_b targets schedule_in_a.
    let req = tenant_req(
        Method::PUT,
        &format!("/api/v1/reports/schedules/{}/pause", schedule_in_a),
        org_b,
    );
    let response = app.execute(req).await;
    assert_rejected(
        response.status,
        "pause: cross-tenant request must be rejected (#646)",
    );

    // Verify the schedule is still active in org_a.
    let is_active: bool =
        sqlx::query_scalar("SELECT is_active FROM report_schedules WHERE id = $1")
            .bind(schedule_in_a)
            .fetch_one(&pool)
            .await
            .expect("fetch schedule is_active");
    assert!(
        is_active,
        "#646 regression: cross-tenant pause must not deactivate org A's schedule"
    );
}

// ===========================================================================
// resume_schedule (#646)
// ===========================================================================

/// Unauthenticated request to resume a schedule must be rejected.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resume_schedule_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "resume-noauth").await;
    // Seed a paused schedule.
    let schedule = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO report_schedules
            (report_id, organization_id, name, frequency, time, timezone, format, recipients, is_active, status)
        VALUES
            (gen_random_uuid(), $1, 'Paused Schedule', 'weekly', '09:00', 'UTC', 'pdf',
             '[]', false, 'paused')
        RETURNING id
        "#,
    )
    .bind(org)
    .fetch_one(&pool)
    .await
    .expect("seed paused schedule");

    let req = no_auth_req(
        Method::PUT,
        &format!("/api/v1/reports/schedules/{}/resume", schedule),
    );
    let response = app.execute(req).await;
    assert_rejected(response.status, "resume: no-auth must be rejected (#646)");
}

/// A cross-tenant request to resume another org's schedule must be rejected.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resume_schedule_cross_tenant_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "resume-ctor-a").await;
    // Start the schedule as paused.
    let schedule_in_a = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO report_schedules
            (report_id, organization_id, name, frequency, time, timezone, format, recipients, is_active, status)
        VALUES
            (gen_random_uuid(), $1, 'Paused Org A Schedule', 'weekly', '09:00', 'UTC', 'pdf',
             '[]', false, 'paused')
        RETURNING id
        "#,
    )
    .bind(org_a)
    .fetch_one(&pool)
    .await
    .expect("seed paused schedule for org_a");

    let org_b = seed_org(&pool, "resume-ctor-b").await;
    let user_b = seed_user(&pool, "attacker-resume@sched-sibling.test").await;
    seed_membership(&pool, org_b, user_b, "Manager").await;

    let req = tenant_req(
        Method::PUT,
        &format!("/api/v1/reports/schedules/{}/resume", schedule_in_a),
        org_b,
    );
    let response = app.execute(req).await;
    assert_rejected(
        response.status,
        "resume: cross-tenant request must be rejected (#646)",
    );

    // Verify schedule_in_a remains paused.
    let is_active: bool =
        sqlx::query_scalar("SELECT is_active FROM report_schedules WHERE id = $1")
            .bind(schedule_in_a)
            .fetch_one(&pool)
            .await
            .expect("fetch schedule is_active");
    assert!(
        !is_active,
        "#646 regression: cross-tenant resume must not reactivate org A's paused schedule"
    );
}

// ===========================================================================
// list_schedule_executions (#647)
// ===========================================================================

/// Unauthenticated request to list executions must be rejected.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_executions_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "list-exec-noauth").await;
    let schedule = seed_schedule(&pool, org).await;

    let req = no_auth_req(
        Method::GET,
        &format!("/api/v1/reports/schedules/{}/executions", schedule),
    );
    let response = app.execute(req).await;
    assert_rejected(
        response.status,
        "list_executions: no-auth must be rejected (#647)",
    );
}

/// A cross-tenant request to list another org's execution history must be rejected.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_executions_cross_tenant_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "list-exec-ctor-a").await;
    let schedule_in_a = seed_schedule(&pool, org_a).await;
    // Seed some executions so there would be data if the check were bypassed.
    seed_execution(&pool, schedule_in_a, "completed").await;

    let org_b = seed_org(&pool, "list-exec-ctor-b").await;
    let user_b = seed_user(&pool, "attacker-list@sched-sibling.test").await;
    seed_membership(&pool, org_b, user_b, "Manager").await;

    let req = tenant_req(
        Method::GET,
        &format!("/api/v1/reports/schedules/{}/executions", schedule_in_a),
        org_b,
    );
    let response = app.execute(req).await;
    assert_rejected(
        response.status,
        "list_executions: cross-tenant request must be rejected (#647)",
    );
}

// ===========================================================================
// get_execution (#647)
// ===========================================================================

/// Unauthenticated request to get an execution must be rejected.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_execution_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "get-exec-noauth").await;
    let schedule = seed_schedule(&pool, org).await;
    let execution = seed_execution(&pool, schedule, "completed").await;

    let req = no_auth_req(
        Method::GET,
        &format!("/api/v1/reports/executions/{}", execution),
    );
    let response = app.execute(req).await;
    assert_rejected(
        response.status,
        "get_execution: no-auth must be rejected (#647)",
    );
}

/// A cross-tenant request to read another org's execution must be rejected.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_execution_cross_tenant_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "get-exec-ctor-a").await;
    let schedule_in_a = seed_schedule(&pool, org_a).await;
    let execution_in_a = seed_execution(&pool, schedule_in_a, "completed").await;

    let org_b = seed_org(&pool, "get-exec-ctor-b").await;
    let user_b = seed_user(&pool, "attacker-getexec@sched-sibling.test").await;
    seed_membership(&pool, org_b, user_b, "Resident").await;

    let req = tenant_req(
        Method::GET,
        &format!("/api/v1/reports/executions/{}", execution_in_a),
        org_b,
    );
    let response = app.execute(req).await;
    assert_rejected(
        response.status,
        "get_execution: cross-tenant read must be rejected (#647)",
    );
}

// ===========================================================================
// get_execution_download_url (#647)
// ===========================================================================

/// Unauthenticated request to get a download URL must be rejected.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_execution_download_url_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "dl-url-noauth").await;
    let schedule = seed_schedule(&pool, org).await;
    let execution = seed_execution(&pool, schedule, "completed").await;

    let req = no_auth_req(
        Method::GET,
        &format!("/api/v1/reports/executions/{}/download", execution),
    );
    let response = app.execute(req).await;
    assert_rejected(
        response.status,
        "get_execution_download_url: no-auth must be rejected (#647)",
    );
}

/// A cross-tenant request to get a download URL for another org's execution
/// must be rejected.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_execution_download_url_cross_tenant_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "dl-url-ctor-a").await;
    let schedule_in_a = seed_schedule(&pool, org_a).await;
    let execution_in_a = seed_execution(&pool, schedule_in_a, "completed").await;

    let org_b = seed_org(&pool, "dl-url-ctor-b").await;
    let user_b = seed_user(&pool, "attacker-dlurl@sched-sibling.test").await;
    seed_membership(&pool, org_b, user_b, "Resident").await;

    let req = tenant_req(
        Method::GET,
        &format!("/api/v1/reports/executions/{}/download", execution_in_a),
        org_b,
    );
    let response = app.execute(req).await;
    assert_rejected(
        response.status,
        "get_execution_download_url: cross-tenant read must be rejected (#647)",
    );
}

// ===========================================================================
// retry_execution (#647)
// ===========================================================================

/// Unauthenticated request to retry an execution must be rejected.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn retry_execution_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "retry-noauth").await;
    let schedule = seed_schedule(&pool, org).await;
    let execution = seed_execution(&pool, schedule, "failed").await;

    let req = no_auth_req(
        Method::POST,
        &format!("/api/v1/reports/executions/{}/retry", execution),
    );
    let response = app.execute(req).await;
    assert_rejected(
        response.status,
        "retry_execution: no-auth must be rejected (#647)",
    );
}

/// A cross-tenant request to retry another org's failed execution must be rejected,
/// and the execution must remain in its original 'failed' state.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn retry_execution_cross_tenant_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "retry-ctor-a").await;
    let schedule_in_a = seed_schedule(&pool, org_a).await;
    let execution_in_a = seed_execution(&pool, schedule_in_a, "failed").await;

    let org_b = seed_org(&pool, "retry-ctor-b").await;
    let user_b = seed_user(&pool, "attacker-retry@sched-sibling.test").await;
    seed_membership(&pool, org_b, user_b, "Manager").await;

    let req = tenant_req(
        Method::POST,
        &format!("/api/v1/reports/executions/{}/retry", execution_in_a),
        org_b,
    );
    let response = app.execute(req).await;
    assert_rejected(
        response.status,
        "retry_execution: cross-tenant retry must be rejected (#647)",
    );

    // Verify the execution is still 'failed' in org_a (not reset to 'pending').
    let status: String = sqlx::query_scalar("SELECT status FROM report_executions WHERE id = $1")
        .bind(execution_in_a)
        .fetch_one(&pool)
        .await
        .expect("fetch execution status");
    assert_eq!(
        status, "failed",
        "#647 regression: cross-tenant retry must not reset org A's execution to 'pending'"
    );
}

// ===========================================================================
// Authenticated cross-tenant companions (#693)
// ===========================================================================
//
// These tests send a REAL Bearer JWT from a user authenticated in `org_b` and
// target a resource in `org_a`. The auth gate passes, the tenant extractor
// passes, the RBAC check (when present) passes — the request reaches the
// repository and the org-scoped WHERE clause must filter the row out, yielding
// a strict `404 NOT_FOUND` rather than the `4xx`-anywhere outcome of the
// unauth tests above.
//
// Issue #693 noted that the original tests only sent `X-Tenant-ID` (no JWT),
// so `AuthUser`/`RlsConnection` returned 401 at the outer gate and the
// `AND organization_id = $caller_org_id` clause in the repository was never
// actually exercised. The strict `assert_eq!(..., StatusCode::NOT_FOUND)`
// below would fail if a future regression dropped that clause and the
// handler started reading/mutating cross-tenant rows.

// ---------------------------------------------------------------------------
// pause_schedule — authenticated cross-tenant (#693 / #646)
// ---------------------------------------------------------------------------

/// Authenticated Manager in `org_b` calls `PUT .../pause` on a schedule
/// owned by `org_a`. The handler reaches `report_schedule_repo.pause`, the
/// `UPDATE ... WHERE id = $1 AND organization_id = $caller_org_id` finds no
/// row, and the handler maps `AppError::NotFound` to `404 SCHEDULE_NOT_FOUND`.
/// The Org A schedule must remain active.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pause_schedule_cross_tenant_authenticated_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "pause-auth-a").await;
    let schedule_in_a = seed_schedule(&pool, org_a).await;

    let org_b = seed_org(&pool, "pause-auth-b").await;
    let attacker = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &attacker).await;
    let attacker_id = user_id_for(&pool, &attacker.email).await;
    seed_membership(&pool, org_b, attacker_id, "manager").await;

    let session = app.session(access_token.to_string(), org_b);
    let req = session
        .put(&format!(
            "/api/v1/reports/schedules/{}/pause",
            schedule_in_a
        ))
        .build();
    let response = app.execute(req).await;

    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "#693/#646: authenticated cross-tenant pause must hit the org-scoped \
         WHERE and return 404, got {} body={}",
        response.status,
        response.text(),
    );

    let body = response.json_value();
    let code = body
        .get("code")
        .and_then(|c| c.as_str())
        .unwrap_or_default();
    assert_eq!(
        code, "SCHEDULE_NOT_FOUND",
        "#693/#646: 404 must carry SCHEDULE_NOT_FOUND, body={}",
        body
    );

    // Org A's schedule must still be active (the UPDATE found no row).
    let is_active: bool =
        sqlx::query_scalar("SELECT is_active FROM report_schedules WHERE id = $1")
            .bind(schedule_in_a)
            .fetch_one(&pool)
            .await
            .expect("fetch schedule is_active");
    assert!(
        is_active,
        "#693/#646 regression: authenticated cross-tenant pause must not \
         deactivate org A's schedule"
    );
}

// ---------------------------------------------------------------------------
// resume_schedule — authenticated cross-tenant (#693 / #646)
// ---------------------------------------------------------------------------

/// Authenticated Manager in `org_b` calls `PUT .../resume` on a paused
/// schedule owned by `org_a`. Must hit the org-scoped WHERE and return 404.
/// Org A's schedule must remain paused.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resume_schedule_cross_tenant_authenticated_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "resume-auth-a").await;
    let schedule_in_a = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO report_schedules
            (report_id, organization_id, name, frequency, time, timezone, format, recipients, is_active, status)
        VALUES
            (gen_random_uuid(), $1, 'Paused Org A Schedule (auth)', 'weekly', '09:00', 'UTC', 'pdf',
             '[]', false, 'paused')
        RETURNING id
        "#,
    )
    .bind(org_a)
    .fetch_one(&pool)
    .await
    .expect("seed paused schedule for org_a (auth)");

    let org_b = seed_org(&pool, "resume-auth-b").await;
    let attacker = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &attacker).await;
    let attacker_id = user_id_for(&pool, &attacker.email).await;
    seed_membership(&pool, org_b, attacker_id, "manager").await;

    let session = app.session(access_token.to_string(), org_b);
    let req = session
        .put(&format!(
            "/api/v1/reports/schedules/{}/resume",
            schedule_in_a
        ))
        .build();
    let response = app.execute(req).await;

    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "#693/#646: authenticated cross-tenant resume must hit org-scoped \
         WHERE and return 404, got {} body={}",
        response.status,
        response.text(),
    );

    let body = response.json_value();
    let code = body
        .get("code")
        .and_then(|c| c.as_str())
        .unwrap_or_default();
    assert_eq!(
        code, "SCHEDULE_NOT_FOUND",
        "#693/#646: 404 must carry SCHEDULE_NOT_FOUND, body={}",
        body
    );

    let is_active: bool =
        sqlx::query_scalar("SELECT is_active FROM report_schedules WHERE id = $1")
            .bind(schedule_in_a)
            .fetch_one(&pool)
            .await
            .expect("fetch schedule is_active");
    assert!(
        !is_active,
        "#693/#646 regression: authenticated cross-tenant resume must not \
         reactivate org A's paused schedule"
    );
}

// ---------------------------------------------------------------------------
// list_schedule_executions — authenticated cross-tenant (#693 / #647)
// ---------------------------------------------------------------------------

/// Authenticated Manager in `org_b` calls `GET .../schedules/{id_in_a}/executions`.
/// The handler calls `get_by_id_scoped(id, org_b)`, finds no row (the schedule
/// lives in `org_a`), and returns 404 `SCHEDULE_NOT_FOUND` before any
/// executions are listed.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_executions_cross_tenant_authenticated_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "list-exec-auth-a").await;
    let schedule_in_a = seed_schedule(&pool, org_a).await;
    seed_execution(&pool, schedule_in_a, "completed").await;

    let org_b = seed_org(&pool, "list-exec-auth-b").await;
    let attacker = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &attacker).await;
    let attacker_id = user_id_for(&pool, &attacker.email).await;
    seed_membership(&pool, org_b, attacker_id, "manager").await;

    let session = app.session(access_token.to_string(), org_b);
    let req = session
        .get(&format!(
            "/api/v1/reports/schedules/{}/executions",
            schedule_in_a
        ))
        .build();
    let response = app.execute(req).await;

    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "#693/#647: authenticated cross-tenant list_executions must return \
         404 from get_by_id_scoped, got {} body={}",
        response.status,
        response.text(),
    );

    let body = response.json_value();
    let code = body
        .get("code")
        .and_then(|c| c.as_str())
        .unwrap_or_default();
    assert_eq!(
        code, "SCHEDULE_NOT_FOUND",
        "#693/#647: 404 must carry SCHEDULE_NOT_FOUND, body={}",
        body
    );
}

// ---------------------------------------------------------------------------
// get_execution — authenticated cross-tenant (#693 / #647)
// ---------------------------------------------------------------------------

/// Authenticated Resident in `org_b` calls `GET /reports/executions/{id_in_a}`.
/// (Read endpoints have no manager-tier RBAC.) The handler calls
/// `get_execution_scoped(id, org_b)`, which joins through
/// `report_schedules.organization_id`, finds no row, and returns 404
/// `EXECUTION_NOT_FOUND`.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_execution_cross_tenant_authenticated_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "get-exec-auth-a").await;
    let schedule_in_a = seed_schedule(&pool, org_a).await;
    let execution_in_a = seed_execution(&pool, schedule_in_a, "completed").await;

    let org_b = seed_org(&pool, "get-exec-auth-b").await;
    let attacker = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &attacker).await;
    let attacker_id = user_id_for(&pool, &attacker.email).await;
    seed_membership(&pool, org_b, attacker_id, "Resident").await;

    let session = app.session(access_token.to_string(), org_b);
    let req = session
        .get(&format!("/api/v1/reports/executions/{}", execution_in_a))
        .build();
    let response = app.execute(req).await;

    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "#693/#647: authenticated cross-tenant get_execution must return 404 \
         from get_execution_scoped, got {} body={}",
        response.status,
        response.text(),
    );

    let body = response.json_value();
    let code = body
        .get("code")
        .and_then(|c| c.as_str())
        .unwrap_or_default();
    assert_eq!(
        code, "EXECUTION_NOT_FOUND",
        "#693/#647: 404 must carry EXECUTION_NOT_FOUND, body={}",
        body
    );
}

// ---------------------------------------------------------------------------
// get_execution_download_url — authenticated cross-tenant (#693 / #647)
// ---------------------------------------------------------------------------

/// Authenticated Resident in `org_b` calls
/// `GET /reports/executions/{id_in_a}/download`. The handler calls
/// `get_execution_scoped(id, org_b)` before generating any URL, finds no row,
/// and returns 404 `EXECUTION_NOT_FOUND` — no download URL is leaked.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_execution_download_url_cross_tenant_authenticated_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "dl-url-auth-a").await;
    let schedule_in_a = seed_schedule(&pool, org_a).await;
    let execution_in_a = seed_execution(&pool, schedule_in_a, "completed").await;

    let org_b = seed_org(&pool, "dl-url-auth-b").await;
    let attacker = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &attacker).await;
    let attacker_id = user_id_for(&pool, &attacker.email).await;
    seed_membership(&pool, org_b, attacker_id, "Resident").await;

    let session = app.session(access_token.to_string(), org_b);
    let req = session
        .get(&format!(
            "/api/v1/reports/executions/{}/download",
            execution_in_a
        ))
        .build();
    let response = app.execute(req).await;

    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "#693/#647: authenticated cross-tenant download_url must return 404 \
         from get_execution_scoped, got {} body={}",
        response.status,
        response.text(),
    );

    let body = response.json_value();
    let code = body
        .get("code")
        .and_then(|c| c.as_str())
        .unwrap_or_default();
    assert_eq!(
        code, "EXECUTION_NOT_FOUND",
        "#693/#647: 404 must carry EXECUTION_NOT_FOUND, body={}",
        body
    );
}

// ---------------------------------------------------------------------------
// retry_execution — authenticated cross-tenant (#693 / #647)
// ---------------------------------------------------------------------------

/// Authenticated Manager in `org_b` calls `POST .../executions/{id_in_a}/retry`
/// where the execution is in `failed` state. The RBAC check passes
/// (`is_manager()` is true), the handler calls `retry_execution_scoped(id, org_b)`
/// which verifies the parent schedule is in `org_b`, finds no row, returns
/// `AppError::NotFound` → `404 EXECUTION_NOT_FOUND`. The execution must
/// remain in `failed` state (not reset to `pending`).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn retry_execution_cross_tenant_authenticated_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "retry-auth-a").await;
    let schedule_in_a = seed_schedule(&pool, org_a).await;
    let execution_in_a = seed_execution(&pool, schedule_in_a, "failed").await;

    let org_b = seed_org(&pool, "retry-auth-b").await;
    let attacker = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &attacker).await;
    let attacker_id = user_id_for(&pool, &attacker.email).await;
    seed_membership(&pool, org_b, attacker_id, "manager").await;

    let session = app.session(access_token.to_string(), org_b);
    let req = session
        .post(&format!(
            "/api/v1/reports/executions/{}/retry",
            execution_in_a
        ))
        .build();
    let response = app.execute(req).await;

    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "#693/#647: authenticated cross-tenant retry must return 404 from \
         retry_execution_scoped, got {} body={}",
        response.status,
        response.text(),
    );

    let body = response.json_value();
    let code = body
        .get("code")
        .and_then(|c| c.as_str())
        .unwrap_or_default();
    assert_eq!(
        code, "EXECUTION_NOT_FOUND",
        "#693/#647: 404 must carry EXECUTION_NOT_FOUND, body={}",
        body
    );

    let status: String = sqlx::query_scalar("SELECT status FROM report_executions WHERE id = $1")
        .bind(execution_in_a)
        .fetch_one(&pool)
        .await
        .expect("fetch execution status");
    assert_eq!(
        status, "failed",
        "#693/#647 regression: authenticated cross-tenant retry must not reset \
         org A's execution to 'pending'"
    );
}

// ---------------------------------------------------------------------------
// update_schedule — authenticated cross-tenant (#693 / #624)
// ---------------------------------------------------------------------------
//
// NOTE: An equivalent authenticated cross-tenant test for `update_schedule`
// also lives in `report_schedule_rbac_tests.rs` as
// `update_schedule_from_other_org_is_rejected` (added in PR #811 / closes #696).
// Issue #693 explicitly lists `update_schedule` in the set of seven sibling
// handlers to cover, so we keep a sibling-scope test here too — same shape,
// new fixtures — to document the cluster's full IDOR surface in one file.

/// Authenticated Manager in `org_b` calls `PUT /reports/schedules/{id_in_a}`.
/// The handler calls `report_schedule_repo.update_schedule(id, org_b, ...)`,
/// whose UPDATE WHERE includes `AND organization_id = $caller_org_id`, finds
/// no row, and returns `AppError::NotFound` → `404 SCHEDULE_NOT_FOUND`.
/// Org A's schedule recipients must be unchanged.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_schedule_cross_tenant_authenticated_returns_404(pool: PgPool) {
    use serde_json::json;

    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "upd-auth-a").await;
    let schedule_in_a = seed_schedule(&pool, org_a).await;

    let org_b = seed_org(&pool, "upd-auth-b").await;
    let attacker = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &attacker).await;
    let attacker_id = user_id_for(&pool, &attacker.email).await;
    seed_membership(&pool, org_b, attacker_id, "manager").await;

    let req = Request::builder()
        .method(Method::PUT)
        .uri(format!("/api/v1/reports/schedules/{}", schedule_in_a))
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .header("X-Tenant-ID", org_b.to_string())
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "recipients": ["attacker@evil.test"] }).to_string(),
        ))
        .unwrap();
    let response = app.execute(req).await;

    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "#693/#624: authenticated cross-tenant update_schedule must return 404 \
         from the org-scoped UPDATE WHERE, got {} body={}",
        response.status,
        response.text(),
    );

    let body = response.json_value();
    let code = body
        .get("code")
        .and_then(|c| c.as_str())
        .unwrap_or_default();
    assert_eq!(
        code, "SCHEDULE_NOT_FOUND",
        "#693/#624: 404 must carry SCHEDULE_NOT_FOUND, body={}",
        body
    );

    // Org A's schedule recipients must be unchanged.
    let recipients_json: serde_json::Value =
        sqlx::query_scalar("SELECT recipients FROM report_schedules WHERE id = $1")
            .bind(schedule_in_a)
            .fetch_one(&pool)
            .await
            .expect("fetch schedule recipients");
    assert_eq!(
        recipients_json,
        json!(["owner@example.com"]),
        "#693/#624 regression: cross-tenant update must not change org A's recipients"
    );
}
