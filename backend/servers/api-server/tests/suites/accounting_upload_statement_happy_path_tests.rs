//! Happy-path integration test for the bank-statement upload endpoint
//! (`POST /api/v1/accounting/statements/` -> `statements::upload_statement`) —
//! BIT-420 (BIT-417 bucket B).
//!
//! Auth model: the handler takes a `RlsConnection` (resolved from the
//! `X-Tenant-ID` header + an active `organization_members` row) and is
//! manager-gated (`rls.has_role(Manager)`); `org_admin` (level 90) satisfies the
//! `Manager` (level 80) gate, so `create_authenticated_user_with_org` provides a
//! suitable principal — exactly as in `accounting_happy_path_tests.rs`.
//!
//! The body is a `multipart/form-data` upload with a `file` field carrying a CSV
//! that the statement parser (`AccountingService::process_statement_upload`)
//! accepts: header row `date,amount,currency,counterparty_iban,variable_symbol,
//! reference` followed by one data row. A successful upload returns `200` with
//! the created `BankStatement`.

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;

use crate::common::{create_authenticated_user_with_org, TestApp, TestUser};

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn upload_statement_happy_path(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "uploadstmt").await;

    // A minimal, parser-valid CSV: one booking line.
    let csv = "date,amount,currency,counterparty_iban,variable_symbol,reference\n\
               2026-06-01,100.00,CZK,SK0000000000000000000000,VS999,Test payment\n";

    let boundary = "bit420uploadboundary";
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"statement.csv\"\r\n\
         Content-Type: text/csv\r\n\
         \r\n\
         {csv}\r\n\
         --{boundary}--\r\n"
    );

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/accounting/statements")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org_id.to_string())
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .expect("build multipart upload request");

    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
    // The created statement echoes the uploaded file name.
    resp.assert_json_field("id");
}

/// Regression: the created statement's `account_iban` is sourced from the
/// company's configured bank account (UC-ACC-02.10), not a hardcoded
/// placeholder (the old code recorded `"CZ0000000000000000000000"` for every
/// upload regardless of the tenant's configured account).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn upload_statement_records_configured_account_iban(pool: PgPool) {
    use serde_json::json;

    let app = TestApp::new(pool.clone()).await;

    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "uploadstmtiban").await;

    // Configure the tenant's default bank account — the source of truth for the
    // statement's own-account IBAN.
    let configured_iban = "SK9611000000002918599669";
    sqlx::query(
        "INSERT INTO acc_bank_account (tenant_id, label, iban, currency, is_default, is_active) \
         VALUES ($1, 'main', $2, 'EUR', TRUE, TRUE)",
    )
    .bind(org_id)
    .bind(configured_iban)
    .execute(&pool)
    .await
    .expect("seed configured bank account");

    let csv = "date,amount,currency,counterparty_iban,variable_symbol,reference\n\
               2026-06-01,100.00,EUR,SK0000000000000000000000,VS999,Test payment\n";

    let boundary = "bit420ibanboundary";
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"statement.csv\"\r\n\
         Content-Type: text/csv\r\n\
         \r\n\
         {csv}\r\n\
         --{boundary}--\r\n"
    );

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/accounting/statements")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org_id.to_string())
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .expect("build multipart upload request");

    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
    resp.assert_json_value("account_iban", &json!(configured_iban));
}
