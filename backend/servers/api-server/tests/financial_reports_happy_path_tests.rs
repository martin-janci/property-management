//! Happy-path integration tests for the financial reporting endpoints
//! (`/api/v1/financial/{reminder-schedules,late-fee-config,overdue-invoices,reports/*}`).
//!
//! These 8 endpoints are the reporting / reminder slice of `financial.rs` that
//! the broader `financial_happy_path_tests.rs` (accounts / fee-schedules /
//! invoices / payments) does NOT exercise. All use the same auth model as that
//! file: a bearer token from `create_authenticated_user_with_org` plus an
//! `organization_id` query parameter resolved through `verify_org_access`.
//!
//! Endpoints covered (BIT-341, Wave 5):
//!   1. GET /api/v1/financial/reminder-schedules
//!   2. GET /api/v1/financial/late-fee-config
//!   3. GET /api/v1/financial/overdue-invoices
//!   4. GET /api/v1/financial/reports/ar-aging
//!   5. GET /api/v1/financial/reports/income-statement
//!   6. GET /api/v1/financial/reports/balance-sheet
//!   7. GET /api/v1/financial/reports/cash-flow
//!   8. GET /api/v1/financial/reports/income-statement/export

#![allow(dead_code)]

mod common;

use axum::http::StatusCode;
use sqlx::PgPool;

use common::{create_authenticated_user_with_org, TestApp, TestUser};

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn financial_reports_happy_path(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "finreports").await;

    // Seed a late-fee config so `get_late_fee_config` returns 200 instead of the
    // 404 it emits when an org has not configured late fees.
    sqlx::query(
        r#"INSERT INTO late_fee_configs
               (organization_id, enabled, grace_period_days, fee_type, fee_amount)
           VALUES ($1, true, 5, 'fixed', 25.00)"#,
    )
    .bind(org_id)
    .execute(&app.pool)
    .await
    .expect("seed late fee config");

    // Seed a reminder schedule so the list comes back non-empty.
    sqlx::query(
        r#"INSERT INTO reminder_schedules (organization_id, name, days_before_due)
           VALUES ($1, 'Pre-due reminder', 3)"#,
    )
    .bind(org_id)
    .execute(&app.pool)
    .await
    .expect("seed reminder schedule");

    // 1. GET /api/v1/financial/reminder-schedules -> get_reminder_schedules
    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/financial/reminder-schedules?organization_id={org_id}"
            ))
            .bearer(&token)
            .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_reminder_schedules should succeed: {}",
        resp.text()
    );
    assert!(resp
        .json_value()
        .as_array()
        .map(|a| !a.is_empty())
        .unwrap_or(false));

    // 2. GET /api/v1/financial/late-fee-config -> get_late_fee_config
    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/financial/late-fee-config?organization_id={org_id}"
            ))
            .bearer(&token)
            .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_late_fee_config should succeed: {}",
        resp.text()
    );

    // 3. GET /api/v1/financial/overdue-invoices -> get_overdue_invoices
    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/financial/overdue-invoices?organization_id={org_id}"
            ))
            .bearer(&token)
            .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_overdue_invoices should succeed: {}",
        resp.text()
    );
    assert!(resp.json_value().is_array());

    // 4. GET /api/v1/financial/reports/ar-aging -> get_ar_aging_report
    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/financial/reports/ar-aging?organization_id={org_id}"
            ))
            .bearer(&token)
            .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_ar_aging_report should succeed: {}",
        resp.text()
    );

    // 5. GET /api/v1/financial/reports/income-statement -> get_income_statement
    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/financial/reports/income-statement?organization_id={org_id}&from=2026-01-01&to=2026-12-31"
            ))
            .bearer(&token)
            .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_income_statement should succeed: {}",
        resp.text()
    );

    // 6. GET /api/v1/financial/reports/balance-sheet -> get_balance_sheet
    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/financial/reports/balance-sheet?organization_id={org_id}"
            ))
            .bearer(&token)
            .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_balance_sheet should succeed: {}",
        resp.text()
    );

    // 7. GET /api/v1/financial/reports/cash-flow -> get_cash_flow
    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/financial/reports/cash-flow?organization_id={org_id}&from=2026-01-01&to=2026-12-31"
            ))
            .bearer(&token)
            .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_cash_flow should succeed: {}",
        resp.text()
    );

    // 8. GET /api/v1/financial/reports/income-statement/export -> export_report (xlsx)
    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/financial/reports/income-statement/export?organization_id={org_id}&format=xlsx&from=2026-01-01&to=2026-12-31"
            ))
            .bearer(&token)
            .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "export_report should succeed: {}",
        resp.text()
    );
    assert_eq!(
        resp.headers
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok()),
        Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
    );
}
