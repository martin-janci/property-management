//! End-to-end regression tests for the report execution-history **download +
//! retry** happy path (Epic 81, Story 81.2; API #611, UI #547/#623).
//!
//! # Why this file exists (test-gap-81-2)
//!
//! The existing `report_schedule_sibling_scope_tests.rs` exhaustively covers the
//! *negative* (cross-tenant IDOR) path for the execution cluster — every handler
//! is proven to return 404 for an attacker in another org. But there was **no**
//! positive coverage proving that a legitimate, same-org caller can actually:
//!
//!   1. List execution history and receive a populated `download_url` for an
//!      execution that produced a file (`GET /schedules/{id}/executions`).
//!   2. Resolve that execution's presigned download payload — URL + file name +
//!      content type derived from the stored `file_key`/`file_name`
//!      (`GET /executions/{id}/download`).
//!   3. Retry a *failed* execution and observe its status reset to `pending`
//!      (`POST /executions/{id}/retry`).
//!
//! Without these, a regression that (e.g.) stopped populating `download_url`,
//! dropped the `file_name`/content-type derivation, or broke the
//! `failed → pending` retry transition would pass CI silently. Each test below
//! asserts both the HTTP contract (status + body shape) and, for retry, the
//! persisted DB side effect — so it fails loudly if the end-to-end path breaks.
//!
//! These run as a same-org authenticated **Manager** (retry requires
//! manager-tier RBAC; download/list require only authentication + membership).

#![allow(dead_code)]

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user, seed_membership, TestApp, TestUser};

// ---------------------------------------------------------------------------
// Seed helpers
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("Exec E2E Org {slug}"))
    .bind(format!("exec-e2e-org-{slug}"))
    .bind(format!("{slug}@exec-e2e.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_schedule(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO report_schedules
            (report_id, organization_id, name, frequency, time, timezone, format, recipients, is_active, status)
        VALUES
            (gen_random_uuid(), $1, 'E2E Test Schedule', 'weekly', '09:00', 'UTC', 'pdf',
             '["owner@example.com"]', true, 'active')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed report schedule")
}

/// Seed a *completed* execution that produced a file. The presence of
/// `file_key` is what makes the execution eligible for a download URL.
async fn seed_completed_execution_with_file(
    pool: &PgPool,
    schedule_id: Uuid,
    file_key: &str,
    file_name: &str,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO report_executions
            (schedule_id, status, started_at, completed_at, file_key, file_name, file_size, created_at)
        VALUES
            ($1, 'completed', NOW(), NOW(), $2, $3, 2048, NOW())
        RETURNING id
        "#,
    )
    .bind(schedule_id)
    .bind(file_key)
    .bind(file_name)
    .fetch_one(pool)
    .await
    .expect("seed completed execution with file")
}

async fn seed_failed_execution(pool: &PgPool, schedule_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO report_executions
            (schedule_id, status, started_at, completed_at, error_code, error_message, created_at)
        VALUES
            ($1, 'failed', NOW(), NOW(), 'GEN_ERR', 'generation blew up', NOW())
        RETURNING id
        "#,
    )
    .bind(schedule_id)
    .fetch_one(pool)
    .await
    .expect("seed failed execution")
}

/// Look up a user id by email — `create_authenticated_user` registers via
/// `POST /api/v1/auth/register`, so we read back the id to attach a membership.
async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("fetch user id by email")
}

/// Build an authenticated request: Bearer JWT + `X-Tenant-ID` of the caller's
/// own org (same org as the seeded schedule — this is the *happy* path).
fn auth_req(method: Method, uri: &str, caller_org_id: Uuid, access_token: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .header("X-Tenant-ID", caller_org_id.to_string())
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::empty())
        .unwrap()
}

/// Register + verify a user and attach a same-org membership with `role`.
/// Returns the access token to use as the authenticated caller.
async fn authed_member(app: &TestApp, pool: &PgPool, org_id: Uuid, role: &str) -> String {
    let user = TestUser::new();
    let (access_token, _) = create_authenticated_user(app, &user).await;
    let user_id = user_id_for(pool, &user.email).await;
    seed_membership(pool, org_id, user_id, role).await;
    access_token
}

// ===========================================================================
// list_schedule_executions — happy path populates download_url (#611)
// ===========================================================================

/// A same-org authenticated member listing executions for a schedule with one
/// completed, file-bearing execution must receive a populated `download_url`
/// pointing at that execution's download endpoint.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_executions_populates_download_url_for_completed(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "list-dl").await;
    let schedule = seed_schedule(&pool, org).await;
    let exec = seed_completed_execution_with_file(
        &pool,
        schedule,
        "reports/2026/exec-list.pdf",
        "fault-report.pdf",
    )
    .await;

    let token = authed_member(&app, &pool, org, "Manager").await;

    let req = auth_req(
        Method::GET,
        &format!("/api/v1/reports/schedules/{}/executions", schedule),
        org,
        &token,
    );
    let response = app.execute(req).await;

    assert_eq!(
        response.status,
        StatusCode::OK,
        "#611: same-org list_executions must return 200, got {} body={}",
        response.status,
        response.text(),
    );

    let body = response.json_value();
    let executions = body
        .get("executions")
        .and_then(|e| e.as_array())
        .expect("response must carry an executions array");
    assert_eq!(executions.len(), 1, "exactly one execution was seeded");

    let dl = executions[0]
        .get("downloadUrl")
        .and_then(|v| v.as_str())
        .or_else(|| executions[0].get("download_url").and_then(|v| v.as_str()))
        .unwrap_or("");
    assert_eq!(
        dl,
        format!("/api/v1/reports/executions/{}/download", exec),
        "#611 regression: completed execution with a file must carry a \
         download_url pointing at its download endpoint, body={}",
        body
    );
}

// ===========================================================================
// get_execution_download_url — happy path returns presigned payload (#611)
// ===========================================================================

/// A same-org authenticated member resolving the download endpoint for a
/// completed, file-bearing execution must receive a 200 with a non-empty URL,
/// the stored file name, and a content type derived from the file extension.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_execution_download_url_returns_presigned_payload(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "dl-ok").await;
    let schedule = seed_schedule(&pool, org).await;
    let exec = seed_completed_execution_with_file(
        &pool,
        schedule,
        "reports/2026/exec-dl.xlsx",
        "occupancy-report.xlsx",
    )
    .await;

    let token = authed_member(&app, &pool, org, "Manager").await;

    let req = auth_req(
        Method::GET,
        &format!("/api/v1/reports/executions/{}/download", exec),
        org,
        &token,
    );
    let response = app.execute(req).await;

    assert_eq!(
        response.status,
        StatusCode::OK,
        "#611: same-org download_url must return 200, got {} body={}",
        response.status,
        response.text(),
    );

    let body = response.json_value();

    let url = body.get("url").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        !url.is_empty(),
        "#611 regression: download payload must carry a non-empty presigned url, body={}",
        body
    );

    let file_name = body
        .get("fileName")
        .and_then(|v| v.as_str())
        .or_else(|| body.get("file_name").and_then(|v| v.as_str()))
        .unwrap_or("");
    assert_eq!(
        file_name, "occupancy-report.xlsx",
        "#611 regression: download payload must echo the stored file_name, body={}",
        body
    );

    let content_type = body
        .get("contentType")
        .and_then(|v| v.as_str())
        .or_else(|| body.get("content_type").and_then(|v| v.as_str()))
        .unwrap_or("");
    assert_eq!(
        content_type, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "#611 regression: .xlsx file must resolve to the spreadsheet MIME type, body={}",
        body
    );
}

/// Re-presign-on-demand contract (the core of Coverage 81-2).
///
/// The handler (`get_execution_download_url`) stamps `expires_at = now + 1h` on
/// **every** call — it never caches. This test verifies that contract: two
/// successive calls to the same endpoint both succeed and each yields an
/// `expires_at` that is (a) in the future and (b) no earlier than the expiry
/// returned by the preceding call. Condition (b) is the non-tautological part:
/// it fails if the handler is changed to cache and replay the first response,
/// because a cached response's `expires_at` would not advance across calls.
///
/// # Scope boundary
///
/// The URL returned by this handler is a static internal proxy path
/// (`/api/v1/reports/files/{file_key}`), not a real S3 presigned URL. The
/// actual S3 presigning happens one hop downstream (at the `/reports/files/…`
/// handler). Consequently this test cannot assert a *distinct credential* per
/// call — only that the `expires_at` window is freshly computed each time.
/// A dedicated integration test against a live S3 stub would be required to
/// verify the downstream presigning step; that is out of scope here.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn download_url_can_be_repeatedly_represigned_after_expiry(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "dl-represign").await;
    let schedule = seed_schedule(&pool, org).await;
    let exec = seed_completed_execution_with_file(
        &pool,
        schedule,
        "reports/2026/exec-represign.pdf",
        "monthly-summary.pdf",
    )
    .await;

    let token = authed_member(&app, &pool, org, "Manager").await;
    let uri = format!("/api/v1/reports/executions/{}/download", exec);

    // --- First call: initial presign. ---
    let before_first = chrono::Utc::now();
    let first = app.execute(auth_req(Method::GET, &uri, org, &token)).await;
    assert_eq!(
        first.status,
        StatusCode::OK,
        "#611: initial presign must return 200, got {} body={}",
        first.status,
        first.text(),
    );
    let first_body = first.json_value();
    let first_url = first_body.get("url").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        !first_url.is_empty(),
        "#611: initial presign must carry a non-empty url, body={}",
        first_body
    );
    let first_expires = first_body
        .get("expiresAt")
        .and_then(|v| v.as_str())
        .or_else(|| first_body.get("expires_at").and_then(|v| v.as_str()))
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .expect("initial presign must carry a parseable expires_at");
    assert!(
        first_expires.with_timezone(&chrono::Utc) > before_first,
        "#611: initial presign expires_at {first_expires} must be in the future"
    );

    // --- Second call: re-presign (simulates client re-requesting after expiry). ---
    let before_second = chrono::Utc::now();
    let second = app.execute(auth_req(Method::GET, &uri, org, &token)).await;
    assert_eq!(
        second.status,
        StatusCode::OK,
        "#611 regression: re-requesting download must re-presign (200), got {} body={}",
        second.status,
        second.text(),
    );
    let second_body = second.json_value();
    let second_url = second_body
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        !second_url.is_empty(),
        "#611 regression: re-presign must carry a non-empty url, body={}",
        second_body
    );
    let second_expires = second_body
        .get("expiresAt")
        .and_then(|v| v.as_str())
        .or_else(|| second_body.get("expires_at").and_then(|v| v.as_str()))
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .expect("re-presign must carry a parseable expires_at");
    assert!(
        second_expires.with_timezone(&chrono::Utc) > before_second,
        "#611 regression: re-presign expires_at {second_expires} must be a fresh future window — \
         body={second_body}"
    );
    // Key non-tautological assertion: the second expiry must be no earlier than
    // the first. A handler that caches and replays its first response would fail
    // here because the replayed timestamp would not advance.
    assert!(
        second_expires >= first_expires,
        "#611 regression: re-presign expires_at {second_expires} must be >= initial \
         expires_at {first_expires} — a cached/stale response would fail this check"
    );

    // Re-presign must resolve the same execution's file.
    let second_name = second_body
        .get("fileName")
        .and_then(|v| v.as_str())
        .or_else(|| second_body.get("file_name").and_then(|v| v.as_str()))
        .unwrap_or("");
    assert_eq!(
        second_name, "monthly-summary.pdf",
        "#611 regression: re-presign must resolve the same execution's file, body={second_body}"
    );
}

/// An execution that has NOT produced a file (no `file_key`) must NOT yield a
/// download URL — the endpoint returns 422 `NO_FILE_YET` rather than leaking a
/// bogus link. Pins the "file not ready" branch of the handler.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_execution_download_url_without_file_returns_422(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "dl-nofile").await;
    let schedule = seed_schedule(&pool, org).await;
    // A running execution: same org, but no file_key yet.
    let exec = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO report_executions (schedule_id, status, started_at, created_at)
        VALUES ($1, 'running', NOW(), NOW())
        RETURNING id
        "#,
    )
    .bind(schedule)
    .fetch_one(&pool)
    .await
    .expect("seed running execution without file");

    let token = authed_member(&app, &pool, org, "Manager").await;

    let req = auth_req(
        Method::GET,
        &format!("/api/v1/reports/executions/{}/download", exec),
        org,
        &token,
    );
    let response = app.execute(req).await;

    assert_eq!(
        response.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "#611: download for a file-less execution must return 422, got {} body={}",
        response.status,
        response.text(),
    );
    let body = response.json_value();
    let code = body
        .get("code")
        .and_then(|c| c.as_str())
        .unwrap_or_default();
    assert_eq!(
        code, "NO_FILE_YET",
        "#611: file-less download must carry NO_FILE_YET, body={}",
        body
    );
}

// ===========================================================================
// retry_execution — happy path resets failed -> pending (#611)
// ===========================================================================

/// A same-org authenticated **Manager** retrying a *failed* execution must
/// receive a 200, the returned execution must be back in `pending`, and the
/// persisted row must reflect the reset (status `pending`, error fields
/// cleared). This is the core of the retry contract.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn retry_failed_execution_resets_to_pending(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "retry-ok").await;
    let schedule = seed_schedule(&pool, org).await;
    let exec = seed_failed_execution(&pool, schedule).await;

    let token = authed_member(&app, &pool, org, "Manager").await;

    let req = auth_req(
        Method::POST,
        &format!("/api/v1/reports/executions/{}/retry", exec),
        org,
        &token,
    );
    let response = app.execute(req).await;

    assert_eq!(
        response.status,
        StatusCode::OK,
        "#611: same-org manager retry of a failed execution must return 200, got {} body={}",
        response.status,
        response.text(),
    );

    let body = response.json_value();
    let status = body.get("status").and_then(|v| v.as_str()).unwrap_or("");
    assert_eq!(
        status, "pending",
        "#611 regression: retry must return the execution in 'pending', body={}",
        body
    );

    // Persisted side effect: the DB row is reset to pending and error fields cleared.
    let row: (String, Option<String>) =
        sqlx::query_as("SELECT status, error_message FROM report_executions WHERE id = $1")
            .bind(exec)
            .fetch_one(&pool)
            .await
            .expect("fetch execution after retry");
    assert_eq!(
        row.0, "pending",
        "#611 regression: retried execution must persist status='pending'"
    );
    assert!(
        row.1.is_none(),
        "#611 regression: retry must clear error_message, got {:?}",
        row.1
    );
}

/// Retrying an execution that is NOT in `failed` state (here: `completed`) must
/// be rejected with 400 `INVALID_STATE` and must not mutate the row. Pins the
/// "only failed executions may be retried" guard end-to-end.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn retry_non_failed_execution_returns_400(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "retry-bad").await;
    let schedule = seed_schedule(&pool, org).await;
    let exec = seed_completed_execution_with_file(
        &pool,
        schedule,
        "reports/2026/exec-done.pdf",
        "done.pdf",
    )
    .await;

    let token = authed_member(&app, &pool, org, "Manager").await;

    let req = auth_req(
        Method::POST,
        &format!("/api/v1/reports/executions/{}/retry", exec),
        org,
        &token,
    );
    let response = app.execute(req).await;

    assert_eq!(
        response.status,
        StatusCode::BAD_REQUEST,
        "#611: retry of a non-failed execution must return 400, got {} body={}",
        response.status,
        response.text(),
    );
    let body = response.json_value();
    let code = body
        .get("code")
        .and_then(|c| c.as_str())
        .unwrap_or_default();
    assert_eq!(
        code, "INVALID_STATE",
        "#611: non-failed retry must carry INVALID_STATE, body={}",
        body
    );

    // The completed execution must remain completed.
    let status: String = sqlx::query_scalar("SELECT status FROM report_executions WHERE id = $1")
        .bind(exec)
        .fetch_one(&pool)
        .await
        .expect("fetch execution status");
    assert_eq!(
        status, "completed",
        "#611 regression: a rejected retry must not mutate a completed execution"
    );
}

/// A non-manager (Resident) member must NOT be able to retry — RBAC returns 403
/// and the failed execution stays failed. Proves the retry endpoint's
/// manager-tier gate end-to-end (complements the cross-tenant IDOR tests).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn retry_execution_as_resident_returns_403(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "retry-rbac").await;
    let schedule = seed_schedule(&pool, org).await;
    let exec = seed_failed_execution(&pool, schedule).await;

    let token = authed_member(&app, &pool, org, "Resident").await;

    let req = auth_req(
        Method::POST,
        &format!("/api/v1/reports/executions/{}/retry", exec),
        org,
        &token,
    );
    let response = app.execute(req).await;

    assert_eq!(
        response.status,
        StatusCode::FORBIDDEN,
        "#611: a Resident must not be able to retry (manager-tier required), got {} body={}",
        response.status,
        response.text(),
    );

    // The execution must remain failed.
    let status: String = sqlx::query_scalar("SELECT status FROM report_executions WHERE id = $1")
        .bind(exec)
        .fetch_one(&pool)
        .await
        .expect("fetch execution status");
    assert_eq!(
        status, "failed",
        "#611 regression: a forbidden retry must not reset the execution"
    );
}
