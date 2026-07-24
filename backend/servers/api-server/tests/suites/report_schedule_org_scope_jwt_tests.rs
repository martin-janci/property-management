//! Real-JWT regression tests for report-schedule org-scope (closes #710).
//!
//! # Background
//!
//! PR #643 closed cross-tenant IDOR on `PUT /api/v1/reports/schedules/{id}`
//! (`update_schedule`) by adding `AND organization_id = $caller_org_id` to
//! the SQL WHERE clause and threading `caller_org_id` from `RlsConnection`
//! into the repository call.
//!
//! The same pattern was later applied to `pause`, `resume`, and
//! `get_by_id_scoped` in `report_schedule.rs`. Issue #710 flagged the
//! follow-up gap that the regression tests added in PR #643 use
//! `assert_rejected(status, label)` which accepts any 4xx — including 401
//! from the auth gate firing before the org-scoped WHERE is reached. The
//! tests therefore "passed" without ever exercising the IDOR fix end-to-end.
//!
//! # What these tests verify
//!
//! Using a *real* JWT (minted by hitting `/api/v1/auth/register` +
//! `/api/v1/auth/login` via `create_authenticated_user`) for a Manager in
//! Org B, plus `X-Tenant-ID: org_b`, a cross-tenant request must:
//!
//!   - reach the handler (auth gate passes — proves it's not a 401),
//!   - reach the repository (RBAC `is_manager` passes — proves it's not a 403),
//!   - return `404 NOT_FOUND` because the org-scoped WHERE clause finds no row.
//!
//! This is the exact path the production code takes when an attacker holds a
//! valid JWT and guesses a foreign schedule UUID.
//!
//! Covers handlers:
//!   - `PUT /api/v1/reports/schedules/{id}/pause`
//!   - `PUT /api/v1/reports/schedules/{id}/resume`
//!   - `GET /api/v1/reports/schedules/{id}/executions`
//!     (uses `get_by_id_scoped` for the cross-tenant guard)
//!
//! # TestApp wiring note
//!
//! `TestApp` does not layer `host_tenant_middleware`, so the tenant is
//! resolved purely from the `X-Tenant-ID` header. `ValidatedTenantExtractor`
//! still validates membership against `organization_members`, which the
//! fixtures seed below.

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user, seed_membership, TestApp, TestUser};

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
    .bind(format!("OrgScope JWT {slug}"))
    .bind(format!("orgscope-jwt-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@orgscope-jwt.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

/// Seed a `report_schedules` row owned by `org_id`. Returns the schedule UUID.
async fn seed_schedule(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO report_schedules
            (report_id, organization_id, name, frequency, time, timezone,
             format, recipients, is_active, status)
        VALUES
            (gen_random_uuid(), $1, 'OrgScope JWT target', 'weekly', '08:00',
             'UTC', 'pdf', '["original@orgscope-jwt.test"]', true, 'active')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed schedule")
}

/// Look up the user id created by `create_authenticated_user`.
async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("fetch user id")
}

/// Build an authenticated cross-tenant request hitting `uri` with method
/// `method`. The JWT identifies user_b; the `X-Tenant-ID` claims org_b; the
/// `uri` targets a resource that belongs to org_a.
fn cross_tenant_req(
    method: Method,
    uri: &str,
    access_token: &str,
    attacker_org: Uuid,
    body: Option<serde_json::Value>,
) -> Request<Body> {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .header("X-Tenant-ID", attacker_org.to_string());
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    let payload = body
        .map(|v| Body::from(v.to_string()))
        .unwrap_or_else(Body::empty);
    b.body(payload).expect("build request")
}

// ---------------------------------------------------------------------------
// Test 1 — PUT /schedules/{id}/pause is org-scoped → 404
// ---------------------------------------------------------------------------

/// A Manager in Org B holds a real JWT but attempts to pause a schedule that
/// lives in Org A. The repository UPDATE has `AND organization_id = org_b`,
/// so no row is returned and the handler maps the repository `NotFound` to
/// `404 NOT_FOUND`.
///
/// This explicitly asserts `404`, not just any 4xx, so a future regression
/// that drops the org clause and starts mutating the Org A row would be
/// caught (the response would become `200` or a different 4xx).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pause_schedule_from_other_org_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "a-pause").await;
    let org_b = seed_org(&pool, "b-pause").await;
    let schedule_in_a = seed_schedule(&pool, org_a).await;

    // Attacker: a real authenticated Manager in Org B.
    let attacker = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &attacker).await;
    let attacker_user_id = user_id_for(&pool, &attacker.email).await;
    seed_membership(&pool, org_b, attacker_user_id, "manager").await;

    // Sanity: schedule starts `is_active = true, status = 'active'`.
    let (initial_active, initial_status): (bool, String) =
        sqlx::query_as("SELECT is_active, status FROM report_schedules WHERE id = $1")
            .bind(schedule_in_a)
            .fetch_one(&pool)
            .await
            .expect("fetch initial state");
    assert!(initial_active, "pre: schedule must start active");
    assert_eq!(initial_status, "active", "pre: status must start 'active'");

    let response = app
        .execute(cross_tenant_req(
            Method::PUT,
            &format!("/api/v1/reports/schedules/{}/pause", schedule_in_a),
            &access_token,
            org_b,
            None,
        ))
        .await;

    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "#710: cross-tenant pause must return 404 (org-scoped WHERE found no row), got {} body={}",
        response.status,
        response.text()
    );

    // The schedule in Org A must be untouched — same is_active + status.
    let (after_active, after_status): (bool, String) =
        sqlx::query_as("SELECT is_active, status FROM report_schedules WHERE id = $1")
            .bind(schedule_in_a)
            .fetch_one(&pool)
            .await
            .expect("fetch post-call state");
    assert!(
        after_active,
        "#710: Org A schedule must remain active after cross-tenant pause attempt"
    );
    assert_eq!(
        after_status, "active",
        "#710: Org A schedule status must remain 'active' after cross-tenant pause attempt"
    );
}

// ---------------------------------------------------------------------------
// Test 2 — PUT /schedules/{id}/resume is org-scoped → 404
// ---------------------------------------------------------------------------

/// Mirror of Test 1 for the resume path. The Org A schedule starts paused;
/// the cross-tenant resume from Org B must return `404` and leave the row
/// paused.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resume_schedule_from_other_org_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "a-resume").await;
    let org_b = seed_org(&pool, "b-resume").await;
    let schedule_in_a = seed_schedule(&pool, org_a).await;

    // Flip Org A's schedule to paused so a successful (buggy) resume would be
    // visibly different in the post-call assertion.
    sqlx::query("UPDATE report_schedules SET is_active = false, status = 'paused' WHERE id = $1")
        .bind(schedule_in_a)
        .execute(&pool)
        .await
        .expect("seed paused state");

    let attacker = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &attacker).await;
    let attacker_user_id = user_id_for(&pool, &attacker.email).await;
    seed_membership(&pool, org_b, attacker_user_id, "manager").await;

    let response = app
        .execute(cross_tenant_req(
            Method::PUT,
            &format!("/api/v1/reports/schedules/{}/resume", schedule_in_a),
            &access_token,
            org_b,
            None,
        ))
        .await;

    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "#710: cross-tenant resume must return 404 (org-scoped WHERE found no row), got {} body={}",
        response.status,
        response.text()
    );

    // Schedule must remain paused.
    let (after_active, after_status): (bool, String) =
        sqlx::query_as("SELECT is_active, status FROM report_schedules WHERE id = $1")
            .bind(schedule_in_a)
            .fetch_one(&pool)
            .await
            .expect("fetch post-call state");
    assert!(
        !after_active,
        "#710: Org A schedule must remain paused after cross-tenant resume attempt"
    );
    assert_eq!(
        after_status, "paused",
        "#710: Org A schedule status must remain 'paused' after cross-tenant resume attempt"
    );
}

// ---------------------------------------------------------------------------
// Test 3 — GET /schedules/{id}/executions uses get_by_id_scoped → 404
// ---------------------------------------------------------------------------

/// The list-executions handler runs `get_by_id_scoped(id, caller_org_id)` as
/// its existence check before querying executions. A cross-tenant call from
/// Org B for an Org A schedule must therefore return `404` (schedule not
/// found in caller's org), not `200` with an empty list and not `401`.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_executions_for_other_org_schedule_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "a-list").await;
    let org_b = seed_org(&pool, "b-list").await;
    let schedule_in_a = seed_schedule(&pool, org_a).await;

    let attacker = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &attacker).await;
    let attacker_user_id = user_id_for(&pool, &attacker.email).await;
    // Manager role is not strictly required for the GET path, but seeding it
    // here proves the 404 is from the org scope and not from RBAC denial.
    seed_membership(&pool, org_b, attacker_user_id, "manager").await;

    let response = app
        .execute(cross_tenant_req(
            Method::GET,
            &format!("/api/v1/reports/schedules/{}/executions", schedule_in_a),
            &access_token,
            org_b,
            None,
        ))
        .await;

    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "#710: cross-tenant list executions must return 404 (get_by_id_scoped found no row), got {} body={}",
        response.status,
        response.text()
    );
}

// ---------------------------------------------------------------------------
// Test 4 — A non-existent schedule UUID also returns 404 (uniform response)
// ---------------------------------------------------------------------------

/// Verifies the "uniform 404 for both not-found and wrong-org" contract:
/// hitting pause on a UUID that does not exist at all returns the same 404
/// the cross-tenant case returns. This protects the existence-leak property:
/// an attacker cannot distinguish "exists in another org" from "does not
/// exist" by status code or response body.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pause_unknown_schedule_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_b = seed_org(&pool, "b-unknown").await;

    let attacker = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &attacker).await;
    let attacker_user_id = user_id_for(&pool, &attacker.email).await;
    seed_membership(&pool, org_b, attacker_user_id, "manager").await;

    let nonexistent = Uuid::new_v4();

    let response = app
        .execute(cross_tenant_req(
            Method::PUT,
            &format!("/api/v1/reports/schedules/{}/pause", nonexistent),
            &access_token,
            org_b,
            None,
        ))
        .await;

    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "#710: pause on unknown UUID must return 404 (same response shape as cross-tenant), got {} body={}",
        response.status,
        response.text(),
    );

    // The body must be the same SCHEDULE_NOT_FOUND code as cross-tenant —
    // otherwise an attacker could distinguish "exists in another org" from
    // "does not exist" by inspecting the error code.
    let body: serde_json::Value = response.json_value();
    let code = body
        .get("code")
        .and_then(|c| c.as_str())
        .unwrap_or_default();
    assert_eq!(
        code, "SCHEDULE_NOT_FOUND",
        "#710: error code for unknown UUID must match cross-tenant for opacity, body={}",
        body
    );
}
