//! Regression tests for `update_schedule` RBAC and cross-tenant isolation fixes.
//!
//! Closes:
//!   - #614  RequireCapability / role check on `PUT /api/v1/reports/schedules/{id}`
//!   - #624  Cross-tenant mutation: caller in Org B could update Org A's schedule
//!   - #696  Original test suite only proved the outer JWT gate worked — the
//!     RBAC predicate (`rls.role().is_manager()`) and the cross-tenant
//!     WHERE clause (`AND organization_id = $caller_org_id`) were never
//!     exercised because every request was sent without a Bearer JWT.
//!
//! # What these tests verify
//!
//! 1. **RBAC denial (#614)** — `update_schedule_without_manager_role_is_rejected`:
//!    A user authenticated with a *real* JWT whose DB-backed role is
//!    `Resident` (below the manager threshold) gets past `AuthUser`, gets past
//!    `ValidatedTenantExtractor`, and is rejected by the `rls.role().is_manager()`
//!    check inside the handler with `403 FORBIDDEN`. The schedule row is
//!    unchanged.
//!
//! 2. **Cross-tenant isolation (#624)** — `update_schedule_from_other_org_is_rejected`:
//!    A *real* authenticated Manager in Org B sends `X-Tenant-ID: org_b` but
//!    targets a schedule UUID owned by Org A. The handler reaches the
//!    repository, the UPDATE WHERE clause includes `AND organization_id = org_b`,
//!    finds no row, and the handler maps the `NotFound` error to `404 NOT_FOUND`.
//!    The Org A schedule row is unchanged.
//!
//! 3. **JWT gate (kept as-is)** — `update_schedule_without_any_auth_is_rejected`:
//!    A request with no Authorization header at all is rejected with 4xx by the
//!    `AuthUser` extractor before any handler logic runs. This is the legitimate
//!    outer-gate test.
//!
//! # Why specific status codes matter
//!
//! Tests 1 and 2 assert *exact* status codes (`403`, `404`) rather than "any 4xx".
//! A future regression that removes the `is_manager()` check or drops the
//! `AND organization_id = $caller_org_id` clause would silently start mutating
//! the row and the response would change shape — the strict assertions catch
//! that. The earlier "any 4xx is fine" assertion would have passed even if the
//! security mechanism stopped firing, because the outer JWT gate was still
//! active (issue #696).

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::json;
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
    .bind(format!("Sched RBAC Org {slug}"))
    .bind(format!("sched-rbac-org-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@sched-rbac.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

/// Insert a `report_schedules` row in the given org and return its id.
async fn seed_schedule(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO report_schedules
            (report_id, organization_id, name, frequency, time, timezone, format, recipients)
        VALUES
            (gen_random_uuid(), $1, 'Original Schedule Name', 'weekly', '08:00', 'UTC', 'pdf',
             '["original@example.com"]')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed report schedule")
}

/// Look up a user id by email (the user is created by
/// `create_authenticated_user` via `POST /api/v1/auth/register`).
async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("fetch user id")
}

/// Build an authenticated `PUT /api/v1/reports/schedules/{id}` request.
fn put_schedule_req_auth(
    schedule_id: Uuid,
    tenant_id: Uuid,
    access_token: &str,
    body: serde_json::Value,
) -> Request<Body> {
    Request::builder()
        .method(Method::PUT)
        .uri(format!("/api/v1/reports/schedules/{}", schedule_id))
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .header("X-Tenant-ID", tenant_id.to_string())
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request")
}

// ---------------------------------------------------------------------------
// Test 1 — #614/#696: an authenticated Resident gets 403 from the RBAC check
// ---------------------------------------------------------------------------

/// A user with a real Bearer JWT whose DB-backed membership role is
/// `Resident` attempts to update a schedule in their own org. The request
/// passes `AuthUser` and `ValidatedTenantExtractor` and reaches the handler;
/// the handler's `if !rls.role().is_manager()` branch fires and returns
/// `403 FORBIDDEN` with the `FORBIDDEN` error code. The schedule row is
/// unchanged.
///
/// Issue #696 noted the previous version of this test sent NO JWT, so
/// `AuthUser` returned 401 before the RBAC check ever ran — the test only
/// proved the outer auth gate worked. This rewrite exercises the actual
/// RBAC predicate.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_schedule_without_manager_role_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "role").await;
    let schedule = seed_schedule(&pool, org).await;

    // Register + login a real user, then attach them to `org` as a Resident.
    let resident = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &resident).await;
    let resident_user_id = user_id_for(&pool, &resident.email).await;
    seed_membership(&pool, org, resident_user_id, "Resident").await;

    let response = app
        .execute(put_schedule_req_auth(
            schedule,
            org,
            &access_token,
            json!({ "recipients": ["attacker@evil.test"] }),
        ))
        .await;

    // The RBAC check inside the handler must fire — not the outer JWT gate.
    assert_eq!(
        response.status,
        StatusCode::FORBIDDEN,
        "#614/#696: authenticated Resident must be rejected by the RBAC check \
         (`rls.role().is_manager()`) with 403, got {} body={}",
        response.status,
        response.text(),
    );

    // The handler returns the `FORBIDDEN` error code for this branch.
    let body: serde_json::Value = response.json_value();
    let code = body
        .get("code")
        .and_then(|c| c.as_str())
        .unwrap_or_default();
    assert_eq!(
        code, "FORBIDDEN",
        "#614/#696: 403 response must carry the FORBIDDEN code, body={}",
        body
    );

    // Verify the DB row was NOT mutated.
    let recipients_json: serde_json::Value =
        sqlx::query_scalar("SELECT recipients FROM report_schedules WHERE id = $1")
            .bind(schedule)
            .fetch_one(&pool)
            .await
            .expect("fetch schedule recipients");
    assert_eq!(
        recipients_json,
        json!(["original@example.com"]),
        "#614 regression: recipients must not be changed by a Resident-role request"
    );
}

// ---------------------------------------------------------------------------
// Test 2 — #624/#696: cross-tenant Manager gets 404 from the org-scoped WHERE
// ---------------------------------------------------------------------------

/// A *real* authenticated Manager in Org B targets a schedule UUID that lives
/// in Org A. The request passes the JWT gate, passes the tenant-membership
/// extractor (the user IS a member of Org B), passes the RBAC check (they
/// ARE a manager), and reaches the repository. The UPDATE WHERE clause
/// `... AND organization_id = $caller_org_id` finds no row for
/// `schedule_in_a` and the repo returns `AppError::NotFound`, which the
/// handler maps to `404 NOT_FOUND` with code `SCHEDULE_NOT_FOUND`.
///
/// Issue #696 noted the previous version of this test sent NO JWT, so the
/// repository UPDATE was never executed. This rewrite forces the org-scoped
/// WHERE clause to actually fire.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_schedule_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    // Org A owns the target schedule.
    let org_a = seed_org(&pool, "victim").await;
    let schedule_in_a = seed_schedule(&pool, org_a).await;

    // Org B is the attacker's org.
    let org_b = seed_org(&pool, "attacker").await;

    // Real authenticated Manager in Org B.
    let attacker = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &attacker).await;
    let attacker_user_id = user_id_for(&pool, &attacker.email).await;
    seed_membership(&pool, org_b, attacker_user_id, "manager").await;

    let response = app
        .execute(put_schedule_req_auth(
            schedule_in_a,
            org_b,
            &access_token,
            json!({ "recipients": ["attacker@evil.test"] }),
        ))
        .await;

    // The org-scoped WHERE must fire — not the JWT gate, not RBAC.
    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "#624/#696: cross-tenant Manager must hit the org-scoped WHERE and get \
         404 (schedule not found in caller's org), got {} body={}",
        response.status,
        response.text(),
    );

    let body: serde_json::Value = response.json_value();
    let code = body
        .get("code")
        .and_then(|c| c.as_str())
        .unwrap_or_default();
    assert_eq!(
        code, "SCHEDULE_NOT_FOUND",
        "#624/#696: 404 response must carry SCHEDULE_NOT_FOUND, body={}",
        body
    );

    // Verify the original row in Org A is untouched.
    let recipients_json: serde_json::Value =
        sqlx::query_scalar("SELECT recipients FROM report_schedules WHERE id = $1")
            .bind(schedule_in_a)
            .fetch_one(&pool)
            .await
            .expect("fetch schedule recipients");
    assert_eq!(
        recipients_json,
        json!(["original@example.com"]),
        "#624 regression: cross-tenant PUT must not mutate the target org's schedule"
    );
}

// ---------------------------------------------------------------------------
// Test 3 — No auth header at all → 4xx from `AuthUser` (legitimate gate test)
// ---------------------------------------------------------------------------

/// Unauthenticated request (no Authorization header, no tenant header) must
/// be rejected with 4xx by `AuthUser` before any DB access. This is the
/// legitimate outer-gate test and is intentionally kept as-is.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_schedule_without_any_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "noauth").await;
    let schedule = seed_schedule(&pool, org).await;

    let req = Request::builder()
        .method(Method::PUT)
        .uri(format!("/api/v1/reports/schedules/{}", schedule))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "enabled": false }).to_string()))
        .unwrap();

    let response = app.execute(req).await;
    let code = response.status.as_u16();
    assert!(
        (400..500).contains(&code),
        "no-auth request must be rejected with 4xx, got {}",
        response.status,
    );

    // Verify the row was not mutated either.
    let active: bool = sqlx::query_scalar("SELECT is_active FROM report_schedules WHERE id = $1")
        .bind(schedule)
        .fetch_one(&pool)
        .await
        .expect("fetch is_active");
    assert!(
        active,
        "no-auth request must not have flipped is_active to false"
    );
}
