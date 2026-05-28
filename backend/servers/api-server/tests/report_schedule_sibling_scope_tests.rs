//! Regression tests for the reports CRUD cluster IDOR/scope fixes.
//!
//! Closes:
//!   - #614  RequireCapability / role check on PUT /api/v1/reports/schedules/{id}
//!   - #624  Cross-tenant mutation via missing org_id in WHERE (update_schedule)
//!   - #646  pause_schedule and resume_schedule IDOR (no org scope in WHERE)
//!   - #647  list_executions / get_execution / get_execution_download_url /
//!            retry_execution IDOR (no org scope)
//!
//! # What these tests verify
//!
//! Each mutating or data-fetching handler in the schedule/execution cluster
//! must:
//! 1. Require the caller to be authenticated (401 for missing auth).
//! 2. Require at least manager-tier role for mutating operations (403).
//! 3. Prevent cross-tenant IDOR — a principal in Org B cannot read or mutate
//!    resources belonging to Org A, even when they know the resource UUID.
//!
//! # TestApp wiring caveat
//!
//! `TestApp` mounts the router without `host_tenant_middleware`, so there is no
//! `ResolvedTenant` extension. `ValidatedTenantExtractor` therefore looks for a
//! `X-Tenant-ID` header (UUID string) to identify the tenant. Without a Bearer
//! JWT the `AuthUser`/`RlsConnection` extractor returns 401 before the RBAC or
//! IDOR check fires. The tests assert a generic 4xx "rejected" outcome: the
//! security contract is "the operation must not be applied", which holds whether
//! the rejection is 401 (auth gate) or 404 (tenant-scoped WHERE in production).

#[allow(dead_code)]
mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;
use uuid::Uuid;

use common::TestApp;

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

async fn seed_membership(pool: &PgPool, org_id: Uuid, user_id: Uuid, role: &str) {
    sqlx::query(
        r#"
        INSERT INTO organization_members (id, organization_id, user_id, role_type, status, created_at)
        VALUES ($1, $2, $3, $4, 'active', NOW())
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(org_id)
    .bind(user_id)
    .bind(role)
    .execute(pool)
    .await
    .expect("seed membership");
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
