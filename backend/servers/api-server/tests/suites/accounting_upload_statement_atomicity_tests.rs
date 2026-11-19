//! Regression test for `AccountingService::process_statement_upload` atomicity
//! (code-review-api-core-statement-upload-not-atomic).
//!
//! The upload flow performs multiple DB writes — the `bank_statement` header,
//! one `bank_statement_line` per parsed CSV row, and the payment-matcher side
//! effects. These used to run in the connection's autocommit mode with no
//! enclosing transaction, so a mid-file failure (e.g. an unparseable date on
//! row N) returned an error to the caller while the already-inserted statement
//! header and its first N-1 lines stayed committed — orphaned, partial state a
//! retry would then duplicate.
//!
//! Correct behaviour: the whole upload is one transaction. A parse failure
//! partway through rolls back the statement header AND every line inserted so
//! far, leaving no partial state behind.
//!
//! Before the fix this test fails (1 statement + 1 line persist); after it
//! passes (0 rows).

#![allow(dead_code)]

use crate::common::seed_org;
use api_server::services::accounting::AccountingService;
use db::repositories::{AccConfigRepository, AccountingRepository};
use sqlx::PgPool;
use uuid::Uuid;

/// Bind the request context for `conn` so RLS predicates resolve to `org`
/// (super-admin, so the post-rollback row counts see every tenant row).
async fn set_ctx(conn: &mut sqlx::PgConnection, org: Uuid) {
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(org)
        .bind(Option::<Uuid>::None)
        .bind(true)
        .execute(conn)
        .await
        .expect("set request context");
}

async fn count_statements(conn: &mut sqlx::PgConnection, org: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM bank_statement WHERE tenant_id = $1")
        .bind(org)
        .fetch_one(conn)
        .await
        .expect("count bank_statement")
}

async fn count_lines(conn: &mut sqlx::PgConnection, org: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM bank_statement_line WHERE tenant_id = $1")
        .bind(org)
        .fetch_one(conn)
        .await
        .expect("count bank_statement_line")
}

/// A CSV whose first row parses cleanly and whose second row carries an
/// unparseable `booking_date`. The service inserts the statement header and the
/// first line, then fails on the second row's date — the classic mid-operation
/// failure that used to leave partial state.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn partial_upload_failure_rolls_back_all_writes(pool: PgPool) {
    let org = seed_org(&pool, "stmt-atomicity").await;

    let csv = "date,amount,currency,counterparty_iban,variable_symbol,reference\n\
               2026-06-01,100.00,CZK,SK0000000000000000000000,VS1,First line (valid)\n\
               not-a-date,200.00,CZK,SK0000000000000000000000,VS2,Second line (bad date)\n";

    let svc = AccountingService::new(
        AccountingRepository::new(pool.clone()),
        AccConfigRepository::new(pool.clone()),
    );
    let mut conn = pool.acquire().await.unwrap();
    set_ctx(&mut conn, org).await;

    let result = svc
        .process_statement_upload(&mut conn, org, "statement.csv".to_string(), csv.as_bytes())
        .await;

    // The bad date on row 2 must surface as an error.
    assert!(
        result.is_err(),
        "upload with an unparseable date must fail, not partially succeed"
    );

    // ...and NOTHING may remain: not the statement header, not the one line
    // that was inserted before the failure. Before the atomicity fix the
    // statement + first line stayed committed and these counts were 1.
    let statements = count_statements(&mut conn, org).await;
    let lines = count_lines(&mut conn, org).await;
    assert_eq!(
        statements, 0,
        "a mid-file failure must roll back the statement header (found {statements})"
    );
    assert_eq!(
        lines, 0,
        "a mid-file failure must roll back every inserted line (found {lines})"
    );
}
