//! Happy-path integration tests for the reports endpoints (`/api/v1/reports/*`).
//!
//! Exercises fault statistics, voting participation, occupancy, consumption,
//! export, and schedule/execution management routes.
//!
//! Auth/tenant model (verified against current handlers):
//! - GET report handlers use `AuthUser` + `RlsConnection` (`ValidatedTenantExtractor`),
//!   so every request needs a Bearer token AND a matching `X-Tenant-ID` header
//!   plus a seeded `organization_members` row — provided by
//!   `create_authenticated_user_with_org` + `app.session(token, org_id)`.
//! - The report query/body structs additionally carry an explicit
//!   `organization_id` field which the handler cross-checks against the
//!   authenticated tenant (IDOR guard) — so it must equal `org_id` or the
//!   handler returns 403. The date params are `from_date` / `to_date`
//!   (NOT `from` / `to`), and occupancy uses `year` (+ optional `month`).
//! - Schedule/execution mutation handlers use `RlsConnection` only and require a
//!   manager-tier role; `create_authenticated_user_with_org` seeds `org_admin`
//!   which satisfies `is_manager()`.

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user_with_org, TestApp, TestUser};

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reports_endpoints_happy_path(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "rphappy").await;
    let session = app.session(token, org_id);

    // Seed building (org-scoped; report handlers may look it up for the name).
    let building_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Report Lane 1', 'Bratislava', '81101', 'SK') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed building");

    let base = "/api/v1/reports";

    // ========================================================================
    // 1. STATISTICAL REPORTS (read-only)
    //
    // Each handler returns `Ok(Json(..))` => 200, falling back to empty data on
    // repository errors via `unwrap_or_default()`. The required query params are
    // `organization_id` (cross-checked against the tenant), `from_date`/`to_date`
    // (occupancy uses `year`), and an optional `building_id`.
    // ========================================================================

    // 1.1 GET /faults -> get_fault_statistics_report (200)
    let resp = app
        .execute(
            session
                .get(&format!(
                    "{base}/faults?organization_id={org_id}&building_id={building_id}\
                     &from_date=2026-01-01&to_date=2026-06-30"
                ))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    // Response is the bare `FaultStatisticsReportResponse` (no wrapper).
    resp.assert_json_field("statistics");
    resp.assert_json_field("trends");

    // 1.2 GET /voting -> get_voting_participation_report (200)
    let resp = app
        .execute(
            session
                .get(&format!(
                    "{base}/voting?organization_id={org_id}&building_id={building_id}\
                     &from_date=2026-01-01&to_date=2026-06-30"
                ))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    resp.assert_json_field("summary");
    resp.assert_json_field("votes");

    // 1.3 GET /occupancy -> get_occupancy_report (200).
    // Uses `year` (+ optional `month`), NOT from_date/to_date.
    let resp = app
        .execute(
            session
                .get(&format!(
                    "{base}/occupancy?organization_id={org_id}&building_id={building_id}&year=2026"
                ))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    resp.assert_json_field("summary");
    resp.assert_json_field("by_unit");

    // 1.4 GET /consumption -> get_consumption_report (200).
    // `from_date`/`to_date` are REQUIRED (non-Option) here.
    let resp = app
        .execute(
            session
                .get(&format!(
                    "{base}/consumption?organization_id={org_id}&building_id={building_id}\
                     &from_date=2026-01-01&to_date=2026-06-30"
                ))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    resp.assert_json_field("summary");
    resp.assert_json_field("by_utility_type");

    // ========================================================================
    // 2. EXPORT
    //
    // `export_report` requires `organization_id` (cross-checked), `report_type`,
    // `format`, and `from_date`/`to_date`. For a small report (< 1000 estimated
    // rows) with `format=csv`, the handler generates synchronously and returns
    // `200 OK` with an inline `SyncExportReportResponse` ({data, filename,
    // content_type, format}) — NOT a job id. With no seeded faults/votes/units/
    // meters the estimate is 0, so we deterministically hit the sync path.
    // ========================================================================

    // 2.1 POST /export -> export_report (200, sync inline CSV)
    let export_payload = json!({
        "organization_id": org_id,
        "report_type": "occupancy",
        "building_id": building_id,
        "from_date": "2026-01-01",
        "to_date": "2026-06-30",
        "format": "csv"
    });
    let resp = app
        .execute(
            session
                .post(&format!("{base}/export"))
                .json(&export_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    // Untagged union => sync variant fields are inlined on the object.
    resp.assert_json_field("data");
    resp.assert_json_field("filename");
    resp.assert_json_field("format");

    // 2.2 GET /export/{job_id}/status -> get_export_job_status.
    // The sync export path above returns no job id, so we only verify the
    // endpoint is reachable: an unknown job id yields 404 (Ok(None) from the
    // background-job lookup).
    let unknown_job_id = Uuid::new_v4();
    let resp = app
        .execute(
            session
                .get(&format!("{base}/export/{unknown_job_id}/status"))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);

    // ========================================================================
    // 3. REPORT SCHEDULES
    //
    // Seed a schedule directly (the pool bypasses RLS) so we can exercise the
    // schedule-management routes. Columns match migration 00162 / 00166.
    // `report_id` is a synthetic UUID — there is no report-definition table FK.
    // ========================================================================
    let fake_report_id = Uuid::new_v4();
    let schedule_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO report_schedules
               (organization_id, report_id, name, frequency, format, cron_expression)
           VALUES ($1, $2, 'Occupancy Monthly', 'monthly', 'csv', '0 8 1 * *')
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(fake_report_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed report schedule");

    // 3.1 PUT /schedules/{id} -> update_schedule (200, returns ReportSchedule).
    // Body fields are `cron_expression` / `recipients` / `enabled` (all optional,
    // at least one required). The cron must be a valid 5-field UNIX expression.
    let update_schedule_payload = json!({
        "cron_expression": "0 9 1 * *",
        "enabled": true
    });
    let resp = app
        .execute(
            session
                .put(&format!("{base}/schedules/{schedule_id}"))
                .json(&update_schedule_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    resp.assert_json_field("id");

    // 3.2 PUT /schedules/{id}/pause -> pause_schedule (200, ReportSchedule)
    let resp = app
        .execute(
            session
                .put(&format!("{base}/schedules/{schedule_id}/pause"))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    resp.assert_json_field("id");

    // 3.3 PUT /schedules/{id}/resume -> resume_schedule (200, ReportSchedule)
    let resp = app
        .execute(
            session
                .put(&format!("{base}/schedules/{schedule_id}/resume"))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    resp.assert_json_field("id");

    // 3.4 GET /schedules/{id}/executions -> list_schedule_executions (200).
    // Returns a paginated `ExecutionHistoryResponse`; with no executions seeded
    // the list is empty but the schedule exists (org-scoped lookup succeeds).
    let resp = app
        .execute(
            session
                .get(&format!("{base}/schedules/{schedule_id}/executions"))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    resp.assert_json_field("executions");

    // ========================================================================
    // 4. EXECUTIONS
    //
    // Verify the single-execution routes are reachable. With no executions
    // seeded for this org, the org-scoped lookups return None => 404.
    // ========================================================================
    let fake_exec_id = Uuid::new_v4();

    // 4.1 GET /executions/{id} -> get_execution (404 for unknown id)
    let resp = app
        .execute(
            session
                .get(&format!("{base}/executions/{fake_exec_id}"))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);

    // 4.2 GET /executions/{id}/download -> get_execution_download_url (404)
    let resp = app
        .execute(
            session
                .get(&format!("{base}/executions/{fake_exec_id}/download"))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);
}
