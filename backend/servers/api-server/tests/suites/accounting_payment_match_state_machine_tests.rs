//! PAP-325: payment-match state-machine integrity tests.
//!
//! `AccountingService::{confirm_match, reject_match}` must enforce the legal
//! match-state transitions so a manager cannot inflate (or corrupt) their own
//! invoice's `paid_amount` by replaying confirm/reject. These tests assert the
//! intra-tenant accounting correctness of the state machine — they are NOT
//! tenant-isolation tests (that surface is covered elsewhere).
//!
//! Legal transitions:
//!   - Suggested -> Confirmed  (apply +line.amount)
//!   - Suggested -> Rejected   (no financial effect)
//!   - Confirmed -> Rejected   (explicit unapply: subtract line.amount)
//!   - Confirmed -> Confirmed / Rejected -> Rejected : idempotent no-op
//!   - Rejected  -> Confirmed  : ILLEGAL -> 409 Conflict

#![allow(dead_code)]

use crate::common::seed_org;
use api_server::services::accounting::AccountingService;
use db::models::accounting::InvoiceStatus;
use db::repositories::AccountingRepository;
use rust_decimal_macros::dec;
use sqlx::PgPool;
use uuid::Uuid;

/// Bind the request context for `conn` so RLS predicates resolve to `org`.
async fn set_ctx(conn: &mut sqlx::PgConnection, org: Uuid) {
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(org)
        .bind(Option::<Uuid>::None)
        .bind(true)
        .execute(conn)
        .await
        .expect("set request context");
}

/// Seed a real user so FK on `payment_match.decided_by` is satisfied.
async fn seed_user(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at) \
         VALUES ($1, 'hash', 'Test', 'active', NOW()) RETURNING id",
    )
    .bind(format!("{slug}@test.test"))
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Seed an org + contact + invoice (total 100) + statement line (amount 100) +
/// a `suggested` payment match. Returns (org, invoice_id, match_id, user_id).
async fn seed_match_scenario(pool: &PgPool, slug: &str) -> (Uuid, Uuid, Uuid, Uuid) {
    let org = seed_org(pool, slug).await;
    let user = seed_user(pool, slug).await;
    let contact = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO contact (tenant_id, name) VALUES ($1, 'C') RETURNING id",
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .unwrap();
    let invoice = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO invoice (tenant_id, contact_id, number, issue_date, due_date, currency, total_amount, status) \
         VALUES ($1, $2, 'INV-1', NOW(), NOW(), 'CZK', 100, 'issued') RETURNING id",
    )
    .bind(org)
    .bind(contact)
    .fetch_one(pool)
    .await
    .unwrap();
    let stmt = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO bank_statement (tenant_id, source_filename, account_iban) \
         VALUES ($1, 'S', 'IBAN') RETURNING id",
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .unwrap();
    let line = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO bank_statement_line (statement_id, tenant_id, booking_date, amount, currency, match_state) \
         VALUES ($1, $2, NOW(), 100, 'CZK', 'suggested') RETURNING id",
    )
    .bind(stmt)
    .bind(org)
    .fetch_one(pool)
    .await
    .unwrap();
    let p_match = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO payment_match (tenant_id, statement_line_id, invoice_id, confidence, state) \
         VALUES ($1, $2, $3, 1.0, 'suggested') RETURNING id",
    )
    .bind(org)
    .bind(line)
    .bind(invoice)
    .fetch_one(pool)
    .await
    .unwrap();
    (org, invoice, p_match, user)
}

async fn invoice_state(pool: &PgPool, invoice: Uuid) -> (rust_decimal::Decimal, InvoiceStatus) {
    sqlx::query_as::<_, (rust_decimal::Decimal, InvoiceStatus)>(
        "SELECT paid_amount, status FROM invoice WHERE id = $1",
    )
    .bind(invoice)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// confirm -> reject -> confirm must NOT inflate paid_amount.
/// After confirm: paid=100/paid. After reject (unapply): paid=0/issued.
/// The final confirm of a now-rejected match is illegal (409) and leaves paid=0.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn confirm_reject_confirm_does_not_inflate_paid_amount(pool: PgPool) {
    let (org, invoice, match_id, user) = seed_match_scenario(&pool, "pap325-replay").await;
    let svc = AccountingService::new(AccountingRepository::new(pool.clone()));
    let mut conn = pool.acquire().await.unwrap();
    set_ctx(&mut conn, org).await;

    // Suggested -> Confirmed: applies the full amount.
    svc.confirm_match(&mut conn, org, match_id, user)
        .await
        .expect("confirm suggested match");
    let (paid, status) = invoice_state(&pool, invoice).await;
    assert_eq!(paid, dec!(100), "confirm must apply the line amount");
    assert_eq!(status, InvoiceStatus::Paid, "fully paid invoice is 'paid'");

    // Confirmed -> Rejected: unapplies the amount and reverts status.
    svc.reject_match(&mut conn, match_id, user)
        .await
        .expect("reject confirmed match");
    let (paid, status) = invoice_state(&pool, invoice).await;
    assert_eq!(
        paid,
        dec!(0),
        "rejecting a confirmed match must unapply the amount"
    );
    assert_eq!(
        status,
        InvoiceStatus::Issued,
        "unapplying back to zero reverts status to 'issued'"
    );

    // Rejected -> Confirmed: ILLEGAL. Must 409 and must NOT re-apply.
    let err = svc
        .confirm_match(&mut conn, org, match_id, user)
        .await
        .expect_err("confirming a rejected match must fail");
    assert!(
        matches!(err, ::common::errors::AppError::Conflict(_)),
        "rejected -> confirmed must be a 409 Conflict, got {err:?}"
    );
    let (paid, _) = invoice_state(&pool, invoice).await;
    assert_eq!(
        paid,
        dec!(0),
        "a rejected match must never re-apply paid_amount"
    );
}

/// Confirming an already-confirmed match is an idempotent no-op — it must not
/// re-apply paid_amount.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn double_confirm_is_idempotent(pool: PgPool) {
    let (org, invoice, match_id, user) = seed_match_scenario(&pool, "pap325-double-confirm").await;
    let svc = AccountingService::new(AccountingRepository::new(pool.clone()));
    let mut conn = pool.acquire().await.unwrap();
    set_ctx(&mut conn, org).await;

    svc.confirm_match(&mut conn, org, match_id, user)
        .await
        .expect("first confirm");
    svc.confirm_match(&mut conn, org, match_id, user)
        .await
        .expect("second confirm is a no-op");

    let (paid, _) = invoice_state(&pool, invoice).await;
    assert_eq!(
        paid,
        dec!(100),
        "double-confirm must apply the amount exactly once"
    );
}

/// Rejecting an already-rejected match is an idempotent no-op (no double
/// subtraction). Here the match was only ever Suggested -> Rejected, so paid
/// stays 0 and the second reject is harmless.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn double_reject_is_idempotent(pool: PgPool) {
    let (org, invoice, match_id, user) = seed_match_scenario(&pool, "pap325-double-reject").await;
    let svc = AccountingService::new(AccountingRepository::new(pool.clone()));
    let mut conn = pool.acquire().await.unwrap();
    set_ctx(&mut conn, org).await;

    svc.reject_match(&mut conn, match_id, user)
        .await
        .expect("first reject");
    svc.reject_match(&mut conn, match_id, user)
        .await
        .expect("second reject is a no-op");

    let (paid, _) = invoice_state(&pool, invoice).await;
    assert_eq!(
        paid,
        dec!(0),
        "rejecting a suggested match never touches paid_amount"
    );
}
