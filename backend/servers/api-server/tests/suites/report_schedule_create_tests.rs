//! End-to-end tests for `POST /api/v1/reports/schedules` (gap-81-1).
//!
//! The create-schedule endpoint was previously stubbed and never persisted a
//! row. These tests pin the real behaviour against a live Postgres:
//!
//!   1. a same-org Manager creating a valid schedule gets `201 Created`, a body
//!      that echoes the submitted fields, and a persisted `report_schedules`
//!      row scoped to *their* org (never the request body's choosing);
//!   2. a non-manager (resident) is rejected with `403 Forbidden` and no row is
//!      written;
//!   3. an invalid `frequency` is rejected with `400 Bad Request`.
//!
//! # TestApp wiring note
//!
//! `TestApp` does not layer `host_tenant_middleware`, so the tenant is resolved
//! purely from the `X-Tenant-ID` header. `ValidatedTenantExtractor` still
//! validates membership against `organization_members`, which the fixtures seed.

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user, seed_membership, seed_org, TestApp, TestUser};

/// Look up the user id created by `create_authenticated_user`.
async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("fetch user id")
}

/// Count schedules owned by an org.
async fn count_schedules(pool: &PgPool, org_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM report_schedules WHERE organization_id = $1")
        .bind(org_id)
        .fetch_one(pool)
        .await
        .expect("count schedules")
}

/// Count schedules for an org whose `next_run_at` is set (non-NULL).
async fn count_schedules_with_next_run(pool: &PgPool, org_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM report_schedules \
         WHERE organization_id = $1 AND next_run_at IS NOT NULL",
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("count schedules with next_run_at")
}

/// Build an authenticated `POST /api/v1/reports/schedules` request.
fn post_schedule_req(access_token: &str, org_id: Uuid, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri("/api/v1/reports/schedules")
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .header("X-Tenant-ID", org_id.to_string())
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request")
}

// ---------------------------------------------------------------------------
// Test 1 — a same-org Manager creates a schedule that is persisted
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_creates_schedule_persists(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "sched-create").await;

    let manager = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &manager).await;
    let manager_user_id = user_id_for(&pool, &manager.email).await;
    seed_membership(&pool, org, manager_user_id, "manager").await;

    assert_eq!(
        count_schedules(&pool, org).await,
        0,
        "pre: no schedules yet"
    );

    let report_id = Uuid::new_v4();
    let response = app
        .execute(post_schedule_req(
            &access_token,
            org,
            serde_json::json!({
                "report_id": report_id,
                "name": "Weekly occupancy",
                "frequency": "weekly",
                "day_of_week": 1,
                "time": "08:30",
                "format": "pdf",
                "recipients": ["ops@example.test"]
            }),
        ))
        .await;

    assert_eq!(
        response.status,
        StatusCode::CREATED,
        "manager create must return 201, got {} body={}",
        response.status,
        response.text()
    );

    let body: serde_json::Value = response.json_value();
    assert_eq!(
        body.get("organization_id").and_then(|v| v.as_str()),
        Some(org.to_string().as_str()),
        "org must be derived from the authenticated tenant, body={body}"
    );
    assert_eq!(
        body.get("name").and_then(|v| v.as_str()),
        Some("Weekly occupancy")
    );
    assert_eq!(
        body.get("frequency").and_then(|v| v.as_str()),
        Some("weekly")
    );
    assert_eq!(body.get("is_active").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(body.get("status").and_then(|v| v.as_str()), Some("active"));

    // Persisted exactly once for the caller's org.
    assert_eq!(
        count_schedules(&pool, org).await,
        1,
        "schedule must be persisted"
    );
}

// ---------------------------------------------------------------------------
// Test 2 — a non-manager is forbidden and nothing is written
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn non_manager_forbidden(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "sched-create-403").await;

    let resident = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &resident).await;
    let resident_user_id = user_id_for(&pool, &resident.email).await;
    seed_membership(&pool, org, resident_user_id, "resident").await;

    let response = app
        .execute(post_schedule_req(
            &access_token,
            org,
            serde_json::json!({
                "report_id": Uuid::new_v4(),
                "name": "Should be blocked",
                "frequency": "daily",
                "recipients": []
            }),
        ))
        .await;

    assert_eq!(
        response.status,
        StatusCode::FORBIDDEN,
        "resident must be forbidden, got {} body={}",
        response.status,
        response.text()
    );
    assert_eq!(
        count_schedules(&pool, org).await,
        0,
        "no schedule may be written on a forbidden request"
    );
}

// ---------------------------------------------------------------------------
// Test 3 — an invalid frequency is rejected with 400
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn invalid_frequency_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "sched-create-400").await;

    let manager = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &manager).await;
    let manager_user_id = user_id_for(&pool, &manager.email).await;
    seed_membership(&pool, org, manager_user_id, "manager").await;

    let response = app
        .execute(post_schedule_req(
            &access_token,
            org,
            serde_json::json!({
                "report_id": Uuid::new_v4(),
                "name": "Bad frequency",
                "frequency": "fortnightly",
                "recipients": []
            }),
        ))
        .await;

    assert_eq!(
        response.status,
        StatusCode::BAD_REQUEST,
        "invalid frequency must be 400, got {} body={}",
        response.status,
        response.text()
    );
    assert_eq!(
        count_schedules(&pool, org).await,
        0,
        "no row on validation failure"
    );
}

// ---------------------------------------------------------------------------
// Test 4 — a created schedule gets a non-NULL next_run_at (issue #2198)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_sets_next_run_at(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "sched-next-run").await;

    let manager = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &manager).await;
    let manager_user_id = user_id_for(&pool, &manager.email).await;
    seed_membership(&pool, org, manager_user_id, "manager").await;

    let response = app
        .execute(post_schedule_req(
            &access_token,
            org,
            serde_json::json!({
                "report_id": Uuid::new_v4(),
                "name": "Daily occupancy",
                "frequency": "daily",
                "time": "08:30",
                "timezone": "Europe/Bratislava",
                "format": "pdf",
                "recipients": []
            }),
        ))
        .await;

    assert_eq!(
        response.status,
        StatusCode::CREATED,
        "create must return 201, got {} body={}",
        response.status,
        response.text()
    );

    // Response body must carry a next_run_at.
    let body: serde_json::Value = response.json_value();
    let next_run = body.get("next_run_at").and_then(|v| v.as_str());
    assert!(
        next_run.is_some(),
        "next_run_at must be set in the response body, body={body}"
    );

    // And it must be persisted (not left NULL) — otherwise a
    // `WHERE next_run_at <= NOW()` scheduler tick would never fire it.
    assert_eq!(
        count_schedules_with_next_run(&pool, org).await,
        1,
        "persisted schedule must have a non-NULL next_run_at"
    );
}

// ---------------------------------------------------------------------------
// Test 5 — an invalid IANA timezone is rejected with 400 (issue #2198)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn invalid_timezone_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "sched-bad-tz").await;

    let manager = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &manager).await;
    let manager_user_id = user_id_for(&pool, &manager.email).await;
    seed_membership(&pool, org, manager_user_id, "manager").await;

    let response = app
        .execute(post_schedule_req(
            &access_token,
            org,
            serde_json::json!({
                "report_id": Uuid::new_v4(),
                "name": "Bad timezone",
                "frequency": "daily",
                "timezone": "Mars/Phobos",
                "recipients": []
            }),
        ))
        .await;

    assert_eq!(
        response.status,
        StatusCode::BAD_REQUEST,
        "garbage timezone must be 400, got {} body={}",
        response.status,
        response.text()
    );
    assert_eq!(
        count_schedules(&pool, org).await,
        0,
        "no row on invalid timezone"
    );
}

// ---------------------------------------------------------------------------
// Test 6 — the nil report_id is rejected with 400 (issue #2198)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn nil_report_id_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "sched-nil-report").await;

    let manager = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &manager).await;
    let manager_user_id = user_id_for(&pool, &manager.email).await;
    seed_membership(&pool, org, manager_user_id, "manager").await;

    let response = app
        .execute(post_schedule_req(
            &access_token,
            org,
            serde_json::json!({
                "report_id": Uuid::nil(),
                "name": "Nil report",
                "frequency": "daily",
                "recipients": []
            }),
        ))
        .await;

    assert_eq!(
        response.status,
        StatusCode::BAD_REQUEST,
        "nil report_id must be 400, got {} body={}",
        response.status,
        response.text()
    );
    assert_eq!(
        count_schedules(&pool, org).await,
        0,
        "no row on nil report_id"
    );
}
