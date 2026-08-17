//! Regression tests for `AccountingService::run_payment_matcher` suggestion
//! generation (code-review-api-core-payment-matcher-multi-suggest).
//!
//! The matcher used to upsert a `suggested` PaymentMatch for EVERY open invoice
//! that cleared the 0.5 auto-suggest threshold, with no `break`. So a single
//! bank-statement line sharing a variable symbol with N open invoices was
//! auto-suggested against all N — burying the genuinely-best candidate under
//! strictly-worse ones and over-generating rows a human then has to reject.
//!
//! Correct behaviour: surface only the best-scoring tier. When one candidate
//! outscores the rest it is the sole suggestion; when several candidates are
//! genuinely tied at the top they are all surfaced for manual disambiguation.

#![allow(dead_code)]

use crate::common::seed_org;
use api_server::services::accounting::AccountingService;
use db::repositories::AccountingRepository;
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

async fn seed_contact(pool: &PgPool, org: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO contact (tenant_id, name) VALUES ($1, 'C') RETURNING id",
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Seed an open (issued) invoice with the given number, total, and variable
/// symbol. `paid_amount` stays 0, so `remaining = total`.
async fn seed_invoice(
    pool: &PgPool,
    org: Uuid,
    contact: Uuid,
    number: &str,
    total: i64,
    vs: &str,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO invoice (tenant_id, contact_id, number, issue_date, due_date, currency, total_amount, variable_symbol, status) \
         VALUES ($1, $2, $3, NOW(), NOW(), 'CZK', $4, $5, 'issued') RETURNING id",
    )
    .bind(org)
    .bind(contact)
    .bind(number)
    .bind(total)
    .bind(vs)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Seed a bank statement plus a single unmatched line (given amount + VS).
/// Returns (statement_id, line_id).
async fn seed_statement_line(pool: &PgPool, org: Uuid, amount: i64, vs: &str) -> (Uuid, Uuid) {
    let stmt = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO bank_statement (tenant_id, source_filename, account_iban) \
         VALUES ($1, 'S', 'IBAN') RETURNING id",
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .unwrap();
    let line = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO bank_statement_line (statement_id, tenant_id, booking_date, amount, currency, variable_symbol, match_state) \
         VALUES ($1, $2, NOW(), $3, 'CZK', $4, 'unmatched') RETURNING id",
    )
    .bind(stmt)
    .bind(org)
    .bind(amount)
    .bind(vs)
    .fetch_one(pool)
    .await
    .unwrap();
    (stmt, line)
}

/// The invoice IDs suggested for a given line, ascending, for stable asserts.
async fn suggested_invoices(pool: &PgPool, line: Uuid) -> Vec<Uuid> {
    sqlx::query_scalar::<_, Uuid>(
        "SELECT invoice_id FROM payment_match WHERE statement_line_id = $1 AND state = 'suggested' ORDER BY invoice_id",
    )
    .bind(line)
    .fetch_all(pool)
    .await
    .unwrap()
}

/// A line sharing a variable symbol with two open invoices must be suggested
/// against ONLY the best-scoring one (exact-amount 0.95), not also the
/// strictly-worse partial-amount candidate (0.85).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn best_scoring_invoice_wins_over_strictly_worse_candidate(pool: PgPool) {
    let org = seed_org(&pool, "matcher-best-wins").await;
    let contact = seed_contact(&pool, org).await;

    // Both share VS "VS100"; the line pays 100.
    // exact:   total 100 -> remaining 100 == amount -> 0.8 + 0.15 = 0.95 (best)
    // partial: total 500 -> remaining 500 >= amount -> 0.8 + 0.05 = 0.85 (worse)
    let exact = seed_invoice(&pool, org, contact, "INV-EXACT", 100, "VS100").await;
    let _partial = seed_invoice(&pool, org, contact, "INV-PARTIAL", 500, "VS100").await;
    let (stmt, line) = seed_statement_line(&pool, org, 100, "VS100").await;

    let svc = AccountingService::new(AccountingRepository::new(pool.clone()));
    let mut conn = pool.acquire().await.unwrap();
    set_ctx(&mut conn, org).await;
    svc.run_payment_matcher(&mut conn, org, stmt)
        .await
        .expect("run matcher");

    let suggested = suggested_invoices(&pool, line).await;
    assert_eq!(
        suggested,
        vec![exact],
        "only the best-scoring invoice must be suggested, not every VS-sharing invoice"
    );
}

/// When two open invoices tie at the top confidence (both VS-only 0.8, neither
/// amount-corroborated), BOTH are surfaced so a human can disambiguate — the
/// best-tier rule must keep ties, not arbitrarily hide an equally-good one.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn genuinely_tied_candidates_are_all_surfaced(pool: PgPool) {
    let org = seed_org(&pool, "matcher-ties").await;
    let contact = seed_contact(&pool, org).await;

    // Both share VS "VSTIE"; the line pays 100 but each invoice only owes 50,
    // so amount adds nothing (100 > 50, not equal) -> both score exactly 0.8.
    let a = seed_invoice(&pool, org, contact, "INV-A", 50, "VSTIE").await;
    let b = seed_invoice(&pool, org, contact, "INV-B", 50, "VSTIE").await;
    let (stmt, line) = seed_statement_line(&pool, org, 100, "VSTIE").await;

    let svc = AccountingService::new(AccountingRepository::new(pool.clone()));
    let mut conn = pool.acquire().await.unwrap();
    set_ctx(&mut conn, org).await;
    svc.run_payment_matcher(&mut conn, org, stmt)
        .await
        .expect("run matcher");

    let mut suggested = suggested_invoices(&pool, line).await;
    suggested.sort();
    let mut expected = vec![a, b];
    expected.sort();
    assert_eq!(
        suggested, expected,
        "both genuinely-tied top candidates must be surfaced for disambiguation"
    );
}
