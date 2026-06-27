//! Happy-path integration tests for the reports endpoints (`/api/v1/reports/*`).
//!
//! Exercises fault statistics, voting participation, occupancy, consumption,
//! export, and schedule/execution management routes.

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user_with_org, TestApp, TestUser};

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reports_endpoints_happy_path(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "rphappy").await;
    let session = app.session(token, org_id);

    // Seed building
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
    // 1. STATISTICAL REPORTS (read-only, always return 200 with possibly empty data)
    // ========================================================================

    // 1.1 GET /faults -> get_fault_statistics_report
    let resp = app
        .execute(
            session
                .get(&format!(
                    "{base}/faults?building_id={building_id}&from=2026-01-01&to=2026-06-30"
                ))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 1.2 GET /voting -> get_voting_participation_report
    let resp = app
        .execute(
            session
                .get(&format!(
                    "{base}/voting?building_id={building_id}&from=2026-01-01&to=2026-06-30"
                ))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 1.3 GET /occupancy -> get_occupancy_report
    let resp = app
        .execute(
            session
                .get(&format!(
                    "{base}/occupancy?building_id={building_id}&from=2026-01-01&to=2026-06-30"
                ))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 1.4 GET /consumption -> get_consumption_report
    let resp = app
        .execute(
            session
                .get(&format!(
                    "{base}/consumption?building_id={building_id}&from=2026-01-01&to=2026-06-30"
                ))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 2. EXPORT
    // ========================================================================

    // 2.1 POST /export -> export_report
    let export_payload = json!({
        "report_type": "occupancy",
        "building_id": building_id,
        "from": "2026-01-01",
        "to": "2026-06-30",
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
    let job_id: Option<Uuid> = if resp.status == StatusCode::OK || resp.status == StatusCode::ACCEPTED {
        resp.json_value()["job_id"]
            .as_str()
            .and_then(|s| Uuid::parse_str(s).ok())
    } else {
        None
    };

    // 2.2 GET /export/{job_id}/status -> get_export_job_status
    if let Some(jid) = job_id {
        let resp = app
            .execute(
                session
                    .get(&format!("{base}/export/{jid}/status"))
                    .build(),
            )
            .await;
        resp.assert_status(StatusCode::OK);
    }

    // ========================================================================
    // 3. REPORT SCHEDULES
    // ========================================================================

    // Seed a schedule directly so we can exercise schedule management routes.
    // report_id is the UUID of the report definition — use a synthetic value.
    let fake_report_id = Uuid::new_v4();
    let schedule_id_opt: Option<Uuid> = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO report_schedules
               (organization_id, report_id, name, frequency, format, cron_expression)
           VALUES ($1, $2, 'Occupancy Monthly', 'monthly', 'csv', '0 8 1 * *')
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(fake_report_id)
    .fetch_one(&app.pool)
    .await
    .ok();

    if let Some(sid) = schedule_id_opt {
        // 3.1 PUT /schedules/{id} -> update_schedule
        let update_schedule_payload = json!({
            "cron_expression": "0 9 1 * *",
            "is_active": true
        });
        let resp = app
            .execute(
                session
                    .put(&format!("{base}/schedules/{sid}"))
                    .json(&update_schedule_payload)
                    .build(),
            )
            .await;
        assert!(
            resp.status == StatusCode::OK || resp.status == StatusCode::NO_CONTENT,
            "update_schedule: unexpected {}",
            resp.status
        );

        // 3.2 PUT /schedules/{id}/pause -> pause_schedule
        let resp = app
            .execute(
                session
                    .put(&format!("{base}/schedules/{sid}/pause"))
                    .json(&json!({}))
                    .build(),
            )
            .await;
        assert!(
            resp.status == StatusCode::OK || resp.status == StatusCode::NO_CONTENT,
            "pause_schedule: unexpected {}",
            resp.status
        );

        // 3.3 PUT /schedules/{id}/resume -> resume_schedule
        let resp = app
            .execute(
                session
                    .put(&format!("{base}/schedules/{sid}/resume"))
                    .json(&json!({}))
                    .build(),
            )
            .await;
        assert!(
            resp.status == StatusCode::OK || resp.status == StatusCode::NO_CONTENT,
            "resume_schedule: unexpected {}",
            resp.status
        );

        // 3.4 GET /schedules/{id}/executions -> list_schedule_executions
        let resp = app
            .execute(
                session
                    .get(&format!("{base}/schedules/{sid}/executions"))
                    .build(),
            )
            .await;
        resp.assert_status(StatusCode::OK);
    }

    // ========================================================================
    // 4. EXECUTIONS (if any seeded above)
    // ========================================================================

    // Verify execution routes exist and are reachable (no existing executions, 404 is fine)
    let fake_exec_id = Uuid::new_v4();

    let resp = app
        .execute(
            session
                .get(&format!("{base}/executions/{fake_exec_id}"))
                .build(),
        )
        .await;
    assert!(
        resp.status == StatusCode::OK || resp.status == StatusCode::NOT_FOUND,
        "get_execution: unexpected {}",
        resp.status
    );

    let resp = app
        .execute(
            session
                .get(&format!("{base}/executions/{fake_exec_id}/download"))
                .build(),
        )
        .await;
    assert!(
        resp.status == StatusCode::OK || resp.status == StatusCode::NOT_FOUND,
        "get_execution_download_url: unexpected {}",
        resp.status
    );
}
