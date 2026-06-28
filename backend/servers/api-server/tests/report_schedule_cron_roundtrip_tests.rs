//! Round-trip regression tests for the dedicated `cron_expression` column
//! on `report_schedules` (issue #616).
//!
//! # Background
//!
//! Migration `00162_create_report_schedules_executions.sql` created
//! `time TEXT NOT NULL DEFAULT '08:00'` for an HH:MM schedule time. PR #531
//! (gap-81-1) then *overloaded* that same column to carry 5-field UNIX cron
//! expressions, so anything reading `time` as HH:MM silently mis-interpreted
//! cron data and vice-versa.
//!
//! Migration `00166_report_schedules_add_cron_expression.sql` fixed this by
//! adding a dedicated `cron_expression TEXT` column; `update_schedule()` was
//! re-pointed at it and the `ReportScheduleRow` projection was extended to
//! select it back (the in-module tests in `report_schedule.rs` guard the
//! projection without a DB).
//!
//! These tests close the loop end-to-end against a real Postgres: a same-org
//! Manager editing a schedule's cron expression via
//! `PUT /api/v1/reports/schedules/{id}` must:
//!
//!   1. get `200 OK`,
//!   2. see the new cron value reflected in the response body's
//!      `cron_expression` field,
//!   3. have the value persisted to the dedicated `cron_expression` **column**
//!      (not the legacy `time` column), and
//!   4. leave the legacy `time` column untouched (back-compat).
//!
//! Without the #616 fix, step 3/4 fail: the edit either lands in `time`
//! (clobbering the HH:MM value) or never round-trips back through the SELECT.
//!
//! # TestApp wiring note
//!
//! `TestApp` does not layer `host_tenant_middleware`, so the tenant is
//! resolved purely from the `X-Tenant-ID` header. `ValidatedTenantExtractor`
//! still validates membership against `organization_members`, which the
//! fixtures seed below.

#![allow(dead_code)]

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user, seed_membership, seed_org, TestApp, TestUser};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Seed a `report_schedules` row owned by `org_id` with a legacy HH:MM `time`
/// and **no** `cron_expression`. Returns the schedule UUID.
async fn seed_schedule_hhmm(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO report_schedules
            (report_id, organization_id, name, frequency, time, timezone,
             format, recipients, is_active, status)
        VALUES
            (gen_random_uuid(), $1, 'Cron round-trip target', 'weekly', '08:00',
             'UTC', 'pdf', '["original@cron-roundtrip.test"]', true, 'active')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed schedule")
}

/// Read back `(time, cron_expression)` for a schedule straight from the DB.
async fn read_time_and_cron(pool: &PgPool, schedule_id: Uuid) -> (String, Option<String>) {
    sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT time, cron_expression FROM report_schedules WHERE id = $1",
    )
    .bind(schedule_id)
    .fetch_one(pool)
    .await
    .expect("read time + cron_expression")
}

/// Look up the user id created by `create_authenticated_user`.
async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("fetch user id")
}

/// Build an authenticated `PUT /api/v1/reports/schedules/{id}` request for the
/// caller's own org.
fn put_schedule_req(
    schedule_id: Uuid,
    access_token: &str,
    org_id: Uuid,
    body: serde_json::Value,
) -> Request<Body> {
    Request::builder()
        .method(Method::PUT)
        .uri(format!("/api/v1/reports/schedules/{}", schedule_id))
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .header("X-Tenant-ID", org_id.to_string())
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request")
}

// ---------------------------------------------------------------------------
// Test 1 — editing cron_expression round-trips through the dedicated column
// ---------------------------------------------------------------------------

/// A same-org Manager edits a schedule's cron expression. The new value must
/// land in the dedicated `cron_expression` column, be returned in the response
/// body, and leave the legacy `time` HH:MM value untouched.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_cron_expression_roundtrips_through_dedicated_column(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "cron-rt").await;
    let schedule = seed_schedule_hhmm(&pool, org).await;

    // Pre-condition: time is the seeded HH:MM, cron_expression is NULL.
    let (time_before, cron_before) = read_time_and_cron(&pool, schedule).await;
    assert_eq!(
        time_before, "08:00",
        "pre: legacy time must be the HH:MM seed"
    );
    assert!(
        cron_before.is_none(),
        "pre: cron_expression must start NULL, got {:?}",
        cron_before
    );

    // Same-org Manager mints a real JWT.
    let manager = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &manager).await;
    let manager_user_id = user_id_for(&pool, &manager.email).await;
    seed_membership(&pool, org, manager_user_id, "manager").await;

    let new_cron = "*/15 9-17 * * 1-5";
    let response = app
        .execute(put_schedule_req(
            schedule,
            &access_token,
            org,
            serde_json::json!({ "cron_expression": new_cron }),
        ))
        .await;

    assert_eq!(
        response.status,
        StatusCode::OK,
        "#616: same-org Manager cron edit must succeed, got {} body={}",
        response.status,
        response.text()
    );

    // Response body reflects the new cron expression.
    let body: serde_json::Value = response.json_value();
    assert_eq!(
        body.get("cron_expression").and_then(|c| c.as_str()),
        Some(new_cron),
        "#616: response body must echo the updated cron_expression, body={}",
        body
    );

    // Persisted to the dedicated column; legacy `time` left untouched.
    let (time_after, cron_after) = read_time_and_cron(&pool, schedule).await;
    assert_eq!(
        cron_after.as_deref(),
        Some(new_cron),
        "#616: cron edit must persist to the dedicated cron_expression column"
    );
    assert_eq!(
        time_after, "08:00",
        "#616: legacy `time` HH:MM column must remain untouched (back-compat)"
    );
}

// ---------------------------------------------------------------------------
// Test 2 — a recipients-only edit must NOT disturb cron_expression / time
// ---------------------------------------------------------------------------

/// Partial-update semantics: an edit that omits `cron_expression` must leave
/// both the dedicated cron column and the legacy `time` column unchanged. This
/// guards the `COALESCE($cron, cron_expression)` partial-update in
/// `update_schedule()` from regressing into an unconditional overwrite.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn recipients_only_update_preserves_cron_and_time(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "cron-rt-2").await;
    let schedule = seed_schedule_hhmm(&pool, org).await;

    // Seed an existing cron_expression so we can prove it survives the edit.
    let existing_cron = "0 8 * * 1";
    sqlx::query("UPDATE report_schedules SET cron_expression = $1 WHERE id = $2")
        .bind(existing_cron)
        .bind(schedule)
        .execute(&pool)
        .await
        .expect("seed existing cron_expression");

    let manager = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &manager).await;
    let manager_user_id = user_id_for(&pool, &manager.email).await;
    seed_membership(&pool, org, manager_user_id, "manager").await;

    let response = app
        .execute(put_schedule_req(
            schedule,
            &access_token,
            org,
            serde_json::json!({ "recipients": ["new@cron-roundtrip.test"] }),
        ))
        .await;

    assert_eq!(
        response.status,
        StatusCode::OK,
        "#616: recipients-only edit must succeed, got {} body={}",
        response.status,
        response.text()
    );

    let (time_after, cron_after) = read_time_and_cron(&pool, schedule).await;
    assert_eq!(
        cron_after.as_deref(),
        Some(existing_cron),
        "#616: a recipients-only edit must not clobber cron_expression"
    );
    assert_eq!(
        time_after, "08:00",
        "#616: a recipients-only edit must not touch legacy `time`"
    );

    // And the response body still carries the untouched cron expression.
    let body: serde_json::Value = response.json_value();
    assert_eq!(
        body.get("cron_expression").and_then(|c| c.as_str()),
        Some(existing_cron),
        "#616: response body must still reflect the preserved cron_expression, body={}",
        body
    );
}

// ---------------------------------------------------------------------------
// Test 3 — validator drift guard: known-valid expressions round-trip unchanged
// ---------------------------------------------------------------------------

/// Guard against `validate_cron_expression` drift (GH #1368).
///
/// Seeds a schedule with each expression from the canonical valid set, then
/// uses a recipients-only PUT (which triggers the SELECT projection) to read
/// it back and confirms read == write.  The test fails if:
///
/// - the validator rejects an expression it previously accepted (PUT → 400), or
/// - the SELECT projection drops / transforms `cron_expression` (read ≠ write).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cron_validator_round_trip_stable(pool: PgPool) {
    // Canonical set taken from the validate_cron_expression unit tests in
    // reports.rs.  If the validator is tightened so that any of these become
    // invalid, the recipients-only PUT will 400 and this test fails — surfacing
    // the drift before it hits production.
    let expressions = [
        "* * * * *",
        "0 8 * * 1",
        "0,30 9-17 1-31 1-12 0-7",
        "*/5 * * * *",
        "59 23 31 12 7",
    ];

    let app = TestApp::new(pool.clone()).await;
    let manager = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &manager).await;

    for expr in expressions {
        let org = seed_org(&pool, &format!("cron-drift-{}", expr.replace(' ', "-"))).await;
        let schedule = seed_schedule_hhmm(&pool, org).await;

        sqlx::query("UPDATE report_schedules SET cron_expression = $1 WHERE id = $2")
            .bind(expr)
            .bind(schedule)
            .execute(&pool)
            .await
            .expect("seed cron_expression for drift guard");

        let manager_user_id = user_id_for(&pool, &manager.email).await;
        seed_membership(&pool, org, manager_user_id, "manager").await;

        let response = app
            .execute(put_schedule_req(
                schedule,
                &access_token,
                org,
                serde_json::json!({ "recipients": ["drift-guard@cron.test"] }),
            ))
            .await;

        assert_eq!(
            response.status,
            StatusCode::OK,
            "#1368 drift guard: PUT with cron_expression={expr:?} must succeed (validator may have drifted), body={}",
            response.text()
        );

        let body: serde_json::Value = response.json_value();
        assert_eq!(
            body.get("cron_expression").and_then(|c| c.as_str()),
            Some(expr),
            "#1368 drift guard: read != write for cron_expression={expr:?} — SELECT projection may have drifted, body={}",
            body
        );
    }
}
