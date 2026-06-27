//! Happy-path integration tests for the financial endpoints (`/api/v1/financial/*`).
//!
//! Covers 30 endpoints across accounts, transactions, fee-schedules, invoices,
//! payments, reminder-schedules, late-fee-config, overdue-invoices, and reports.
//! Part of BIT-271 Wave 1A financial test-backfill.

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user_with_org, TestApp, TestUser};

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn financial_happy_path_accounts_transactions_ledger(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "finacc").await;
    let session = app.session(token, org_id);

    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id");

    // Seed building and unit
    let building_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Fin Street 1', 'Bratislava', '81101', 'Slovakia') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed building");

    let unit_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation) VALUES ($1, 'U-FIN-1') RETURNING id"#,
    )
    .bind(building_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed unit");

    // ========================================================================
    // 1. ACCOUNTS
    // ========================================================================

    // 1.1 POST /api/v1/financial/accounts -> create_account
    let create_acct = json!({
        "organization_id": org_id,
        "building_id": building_id,
        "name": "Main Operating Account",
        "account_type": "operating",
        "currency": "EUR",
        "opening_balance": "0.00"
    });
    let resp = app
        .execute(
            session
                .post("/api/v1/financial/accounts")
                .json(&create_acct)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create_account: {}",
        resp.text()
    );
    let acct = resp.json_value();
    let account_id = Uuid::parse_str(acct["id"].as_str().expect("id")).expect("uuid");

    // 1.2 GET /api/v1/financial/accounts -> list_accounts
    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/financial/accounts?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_accounts: {}",
        resp.text()
    );
    let list = resp.json_value();
    assert!(list.as_array().map(|a| !a.is_empty()).unwrap_or(false));

    // 1.3 GET /api/v1/financial/accounts/{id} -> get_account
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/financial/accounts/{account_id}"))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "get_account: {}", resp.text());
    assert_eq!(resp.json_value()["id"], acct["id"]);

    // ========================================================================
    // 2. TRANSACTIONS
    // ========================================================================

    // 2.1 POST /api/v1/financial/accounts/{id}/transactions -> create_transaction
    let create_tx = json!({
        "account_id": account_id,
        "amount": "500.00",
        "transaction_type": "credit",
        "category": "income",
        "description": "Rent payment received",
        "transaction_date": "2025-01-15"
    });
    let resp = app
        .execute(
            session
                .post(&format!(
                    "/api/v1/financial/accounts/{account_id}/transactions"
                ))
                .json(&create_tx)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create_transaction: {}",
        resp.text()
    );
    let tx = resp.json_value();
    let _transaction_id = Uuid::parse_str(tx["id"].as_str().expect("id")).expect("uuid");

    // 2.2 GET /api/v1/financial/accounts/{id}/transactions -> list_transactions
    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/financial/accounts/{account_id}/transactions"
                ))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_transactions: {}",
        resp.text()
    );
    let txs = resp.json_value();
    assert!(txs.as_array().map(|a| !a.is_empty()).unwrap_or(false));

    // ========================================================================
    // 3. UNIT LEDGER
    // ========================================================================

    // 3.1 GET /api/v1/financial/units/{unit_id}/ledger -> get_unit_ledger
    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/financial/units/{unit_id}/ledger?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_unit_ledger: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn financial_happy_path_fee_schedules_and_unit_fees(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "finfee").await;
    let session = app.session(token, org_id);

    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id");

    let building_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Fee Street 1', 'Bratislava', '81101', 'Slovakia') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed building");

    let unit_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation) VALUES ($1, 'U-FEE-1') RETURNING id"#,
    )
    .bind(building_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed unit");

    // ========================================================================
    // 1. FEE SCHEDULES
    // ========================================================================

    // 1.1 POST /api/v1/financial/fee-schedules -> create_fee_schedule
    let create_fs = json!({
        "organization_id": org_id,
        "user_id": user_id,
        "building_id": building_id,
        "name": "Monthly Service Charge",
        "amount": "150.00",
        "currency": "EUR",
        "frequency": "monthly",
        "billing_day": 1
    });
    let resp = app
        .execute(
            session
                .post("/api/v1/financial/fee-schedules")
                .json(&create_fs)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create_fee_schedule: {}",
        resp.text()
    );
    let fs = resp.json_value();
    let fs_id = Uuid::parse_str(fs["id"].as_str().expect("id")).expect("uuid");

    // 1.2 GET /api/v1/financial/fee-schedules -> list_fee_schedules
    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/financial/fee-schedules?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_fee_schedules: {}",
        resp.text()
    );
    let list = resp.json_value();
    assert!(list.as_array().map(|a| !a.is_empty()).unwrap_or(false));

    // 1.3 GET /api/v1/financial/fee-schedules/{id} -> get_fee_schedule
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/financial/fee-schedules/{fs_id}"))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_fee_schedule: {}",
        resp.text()
    );
    assert_eq!(resp.json_value()["id"], fs["id"]);

    // ========================================================================
    // 2. UNIT FEES
    // ========================================================================

    // 2.1 POST /api/v1/financial/units/{unit_id}/fees -> assign_unit_fee
    let assign_fee = json!({
        "organization_id": org_id,
        "fee_schedule_id": fs_id,
        "effective_from": "2025-01-01"
    });
    let resp = app
        .execute(
            session
                .post(&format!("/api/v1/financial/units/{unit_id}/fees"))
                .json(&assign_fee)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "assign_unit_fee: {}",
        resp.text()
    );

    // 2.2 GET /api/v1/financial/units/{unit_id}/fees -> get_unit_fees
    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/financial/units/{unit_id}/fees?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_unit_fees: {}",
        resp.text()
    );
    let fees = resp.json_value();
    assert!(fees.as_array().map(|a| !a.is_empty()).unwrap_or(false));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn financial_happy_path_invoices_lifecycle(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "fininv").await;
    let session = app.session(token, org_id);

    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id");

    let building_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Invoice Street 1', 'Bratislava', '81101', 'Slovakia') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed building");

    let unit_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation) VALUES ($1, 'U-INV-1') RETURNING id"#,
    )
    .bind(building_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed unit");

    // ========================================================================
    // 1. INVOICE LIFECYCLE
    // ========================================================================

    // 1.1 POST /api/v1/financial/invoices -> create_invoice
    let create_inv = json!({
        "organization_id": org_id,
        "user_id": user_id,
        "unit_id": unit_id,
        "due_date": "2025-02-01",
        "billing_period_start": "2025-01-01",
        "billing_period_end": "2025-01-31",
        "currency": "EUR",
        "items": [
            {
                "description": "January Rent",
                "quantity": 1,
                "unit_price": "800.00",
                "tax_rate": "0.20"
            }
        ]
    });
    let resp = app
        .execute(
            session
                .post("/api/v1/financial/invoices")
                .json(&create_inv)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create_invoice: {}",
        resp.text()
    );
    let inv = resp.json_value();
    let invoice_id = Uuid::parse_str(inv["id"].as_str().expect("id")).expect("uuid");

    // 1.2 GET /api/v1/financial/invoices -> list_invoices
    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/financial/invoices?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_invoices: {}",
        resp.text()
    );
    let list = resp.json_value();
    assert!(list.as_array().map(|a| !a.is_empty()).unwrap_or(false));

    // 1.3 GET /api/v1/financial/invoices/{id} -> get_invoice
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/financial/invoices/{invoice_id}"))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "get_invoice: {}", resp.text());
    assert_eq!(resp.json_value()["id"], inv["id"]);

    // 1.4 POST /api/v1/financial/invoices/{id}/send -> send_invoice
    let resp = app
        .execute(
            session
                .post(&format!("/api/v1/financial/invoices/{invoice_id}/send"))
                .json(&json!({}))
                .build(),
        )
        .await;
    // Accept 200 or 204; email may be disabled in test env
    assert!(
        resp.status == StatusCode::OK || resp.status == StatusCode::NO_CONTENT,
        "send_invoice: {}",
        resp.text()
    );

    // 1.5 GET /api/v1/financial/invoices/{id}/pdf -> get_invoice_pdf
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/financial/invoices/{invoice_id}/pdf"))
                .build(),
        )
        .await;
    // Accept 200 or 202 (async generation) or redirect (302) for PDF
    assert!(
        resp.status == StatusCode::OK
            || resp.status == StatusCode::ACCEPTED
            || resp.status == StatusCode::FOUND,
        "get_invoice_pdf: {}",
        resp.text()
    );

    // 1.6 GET /api/v1/financial/units/{unit_id}/invoices -> list_unit_invoices
    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/financial/units/{unit_id}/invoices?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_unit_invoices: {}",
        resp.text()
    );
    let unit_invs = resp.json_value();
    assert!(unit_invs.as_array().map(|a| !a.is_empty()).unwrap_or(false));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn financial_happy_path_payments_lifecycle(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "finpay").await;
    let session = app.session(token, org_id);

    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id");

    let building_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Payment Street 1', 'Bratislava', '81101', 'Slovakia') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed building");

    let unit_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation) VALUES ($1, 'U-PAY-1') RETURNING id"#,
    )
    .bind(building_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed unit");

    // Seed an invoice to allocate against
    let invoice_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO invoices (organization_id, user_id, unit_id, due_date, currency, status, total_amount, subtotal, tax_amount)
           VALUES ($1, $2, $3, '2025-02-01', 'EUR', 'sent', 800.00, 666.67, 133.33)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(user_id)
    .bind(unit_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed invoice");

    // ========================================================================
    // 1. PAYMENT LIFECYCLE
    // ========================================================================

    // 1.1 POST /api/v1/financial/payments -> record_payment
    let record_pay = json!({
        "organization_id": org_id,
        "user_id": user_id,
        "unit_id": unit_id,
        "amount": "800.00",
        "currency": "EUR",
        "payment_method": "bank_transfer",
        "payment_date": "2025-01-28",
        "reference": "REF-001"
    });
    let resp = app
        .execute(
            session
                .post("/api/v1/financial/payments")
                .json(&record_pay)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "record_payment: {}",
        resp.text()
    );
    let payment = resp.json_value();
    let payment_id = Uuid::parse_str(payment["id"].as_str().expect("id")).expect("uuid");

    // 1.2 GET /api/v1/financial/payments -> list_payments
    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/financial/payments?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_payments: {}",
        resp.text()
    );
    let list = resp.json_value();
    assert!(list.as_array().map(|a| !a.is_empty()).unwrap_or(false));

    // 1.3 GET /api/v1/financial/payments/unallocated -> list_unallocated_payments
    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/financial/payments/unallocated?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_unallocated_payments: {}",
        resp.text()
    );

    // 1.4 GET /api/v1/financial/payments/{id} -> get_payment
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/financial/payments/{payment_id}"))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "get_payment: {}", resp.text());
    assert_eq!(resp.json_value()["id"], payment["id"]);

    // 1.5 POST /api/v1/financial/payments/{id}/allocate -> allocate_payment
    let allocate = json!({
        "organization_id": org_id,
        "invoice_id": invoice_id,
        "amount": "800.00"
    });
    let resp = app
        .execute(
            session
                .post(&format!("/api/v1/financial/payments/{payment_id}/allocate"))
                .json(&allocate)
                .build(),
        )
        .await;
    assert!(
        resp.status == StatusCode::OK || resp.status == StatusCode::CREATED,
        "allocate_payment: {}",
        resp.text()
    );

    // 1.6 GET /api/v1/financial/units/{unit_id}/payments -> list_unit_payments
    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/financial/units/{unit_id}/payments?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_unit_payments: {}",
        resp.text()
    );
    let unit_pays = resp.json_value();
    assert!(unit_pays.as_array().map(|a| !a.is_empty()).unwrap_or(false));

    // 1.7 POST /api/v1/financial/payments/auto-match -> auto_match_payments
    let resp = app
        .execute(
            session
                .post("/api/v1/financial/payments/auto-match")
                .json(&json!({ "organization_id": org_id }))
                .build(),
        )
        .await;
    assert!(
        resp.status == StatusCode::OK || resp.status == StatusCode::NO_CONTENT,
        "auto_match_payments: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn financial_happy_path_config_and_reports(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "finrpt").await;
    let session = app.session(token, org_id);

    // ========================================================================
    // 1. REMINDER SCHEDULES / LATE FEE CONFIG / OVERDUE INVOICES
    // ========================================================================

    // 1.1 GET /api/v1/financial/reminder-schedules -> get_reminder_schedules
    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/financial/reminder-schedules?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_reminder_schedules: {}",
        resp.text()
    );

    // 1.2 GET /api/v1/financial/late-fee-config -> get_late_fee_config
    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/financial/late-fee-config?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_late_fee_config: {}",
        resp.text()
    );

    // 1.3 GET /api/v1/financial/overdue-invoices -> get_overdue_invoices
    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/financial/overdue-invoices?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_overdue_invoices: {}",
        resp.text()
    );

    // ========================================================================
    // 2. FINANCIAL REPORTS
    // ========================================================================

    let report_base = format!("/api/v1/financial/reports");

    // 2.1 GET /api/v1/financial/reports/ar-aging -> get_ar_aging_report
    let resp = app
        .execute(
            session
                .get(&format!("{report_base}/ar-aging?organization_id={org_id}"))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_ar_aging_report: {}",
        resp.text()
    );

    // 2.2 GET /api/v1/financial/reports/income-statement -> get_income_statement
    let resp = app
        .execute(
            session
                .get(&format!(
                    "{report_base}/income-statement?organization_id={org_id}&from=2025-01-01&to=2025-12-31"
                ))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_income_statement: {}",
        resp.text()
    );

    // 2.3 GET /api/v1/financial/reports/balance-sheet -> get_balance_sheet
    let resp = app
        .execute(
            session
                .get(&format!(
                    "{report_base}/balance-sheet?organization_id={org_id}&as_of=2025-12-31"
                ))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_balance_sheet: {}",
        resp.text()
    );

    // 2.4 GET /api/v1/financial/reports/cash-flow -> get_cash_flow
    let resp = app
        .execute(
            session
                .get(&format!(
                    "{report_base}/cash-flow?organization_id={org_id}&from=2025-01-01&to=2025-12-31"
                ))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_cash_flow: {}",
        resp.text()
    );

    // 2.5 GET /api/v1/financial/reports/{report}/export -> export_report
    for report_type in &["ar-aging", "income-statement", "balance-sheet", "cash-flow"] {
        let resp = app
            .execute(
                session
                    .get(&format!(
                        "{report_base}/{report_type}/export?organization_id={org_id}&format=csv"
                    ))
                    .build(),
            )
            .await;
        assert!(
            resp.status == StatusCode::OK
                || resp.status == StatusCode::ACCEPTED
                || resp.status == StatusCode::FOUND,
            "export_report {report_type}: {}",
            resp.text()
        );
    }
}
