//! Invoice status-lifecycle guard tests (UC-ACC-05.17).
//!
//! Verifies the repo-level state machine added for the `sent`/`cancelled`
//! lifecycle wiring, against a live migrated DB:
//!   1. `mark_sent_rls` advances ONLY `issued` → `sent`;
//!   2. `cancel_invoice_rls` cancels ONLY unpaid `issued`/`sent`/`overdue`
//!      documents (drafts are deleted instead; paid documents need a credit
//!      note) and is not re-appliable to an already-cancelled document;
//!   3. `update_invoice_payment_status_rls` refuses to touch a cancelled
//!      document — a late payment confirmation cannot resurrect it as paid.
//!
//! Guards, not RLS, are under test — the superuser test pool (which bypasses
//! RLS) is intentionally fine here; RLS coverage lives in
//! `accounting_rls_repo_tests.rs` / `acc_mvp_rls_tests.rs`.

use db::repositories::acc_invoicing::AccInvoicingRepository;
use db::repositories::accounting::AccountingRepository;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO organizations (name, slug, contact_email, status)
           VALUES ($1, $2, $3, 'active') RETURNING id"#,
    )
    .bind(format!("Acc {slug}"))
    .bind(slug)
    .bind(format!("{slug}@acc.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_contact(pool: &PgPool, org: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO contact (tenant_id, name) VALUES ($1, 'Acme s.r.o.') RETURNING id",
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .expect("seed contact")
}

/// Seed an invoice in the given lifecycle state with the given paid amount.
async fn seed_invoice(
    pool: &PgPool,
    org: Uuid,
    contact: Uuid,
    status: &str,
    paid_amount: Decimal,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO invoice
             (tenant_id, contact_id, number, issue_date, due_date,
              total_amount, base_amount, vat_amount, status, paid_amount)
           VALUES ($1, $2, $3, CURRENT_DATE, CURRENT_DATE, 120, 100, 20, $4, $5)
           RETURNING id"#,
    )
    .bind(org)
    .bind(contact)
    .bind(format!("F-{}", Uuid::new_v4().simple()))
    .bind(status)
    .bind(paid_amount)
    .fetch_one(pool)
    .await
    .expect("seed invoice")
}

async fn status_of(pool: &PgPool, id: Uuid) -> String {
    sqlx::query_scalar("SELECT status FROM invoice WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .expect("read status")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mark_sent_advances_only_issued(pool: PgPool) {
    let org = seed_org(&pool, "lifecycle-sent").await;
    let contact = seed_contact(&pool, org).await;
    let repo = AccInvoicingRepository::new(pool.clone());

    // issued → sent
    let issued = seed_invoice(&pool, org, contact, "issued", dec!(0)).await;
    let sent = repo
        .mark_sent_rls(&pool, issued)
        .await
        .expect("mark_sent query")
        .expect("issued must advance to sent");
    assert_eq!(sent.status, "sent");
    assert_eq!(status_of(&pool, issued).await, "sent");

    // Every other state is refused (None), unchanged in the DB.
    for state in ["draft", "sent", "paid", "partially_paid", "cancelled"] {
        let id = seed_invoice(&pool, org, contact, state, dec!(0)).await;
        let refused = repo.mark_sent_rls(&pool, id).await.expect("query ok");
        assert!(refused.is_none(), "{state} must not advance to sent");
        assert_eq!(status_of(&pool, id).await, state);
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cancel_allows_only_unpaid_issued_sent_overdue(pool: PgPool) {
    let org = seed_org(&pool, "lifecycle-cancel").await;
    let contact = seed_contact(&pool, org).await;
    let repo = AccInvoicingRepository::new(pool.clone());

    // Unpaid issued/sent/overdue → cancelled.
    for state in ["issued", "sent", "overdue"] {
        let id = seed_invoice(&pool, org, contact, state, dec!(0)).await;
        let cancelled = repo
            .cancel_invoice_rls(&pool, id)
            .await
            .expect("cancel query")
            .unwrap_or_else(|| panic!("unpaid {state} must be cancellable"));
        assert_eq!(cancelled.status, "cancelled");
        assert_eq!(status_of(&pool, id).await, "cancelled");

        // Cancel is not re-appliable (already cancelled → None).
        assert!(repo.cancel_invoice_rls(&pool, id).await.unwrap().is_none());
    }

    // Drafts and paid lifecycles are refused.
    for state in ["draft", "paid", "partially_paid", "cancelled"] {
        let id = seed_invoice(&pool, org, contact, state, dec!(0)).await;
        let refused = repo.cancel_invoice_rls(&pool, id).await.expect("query ok");
        assert!(refused.is_none(), "{state} must not be cancellable");
        assert_eq!(status_of(&pool, id).await, state);
    }

    // A partial payment on an otherwise-cancellable state blocks cancellation
    // (paid_amount guard, independent of status).
    let part_paid = seed_invoice(&pool, org, contact, "issued", dec!(50)).await;
    assert!(
        repo.cancel_invoice_rls(&pool, part_paid)
            .await
            .unwrap()
            .is_none(),
        "any received payment must block cancellation"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn payment_update_cannot_resurrect_cancelled(pool: PgPool) {
    let org = seed_org(&pool, "lifecycle-pay").await;
    let contact = seed_contact(&pool, org).await;
    let repo = AccountingRepository::new(pool.clone());

    let cancelled = seed_invoice(&pool, org, contact, "cancelled", dec!(0)).await;
    let result = repo
        .update_invoice_payment_status_rls(&pool, cancelled, dec!(120))
        .await
        .expect("payment update query");
    assert!(
        result.is_none(),
        "a payment applied to a cancelled invoice must be a no-op"
    );
    assert_eq!(status_of(&pool, cancelled).await, "cancelled");

    // Sanity: the same delta on an issued invoice still settles it in full.
    let issued = seed_invoice(&pool, org, contact, "issued", dec!(0)).await;
    let paid = repo
        .update_invoice_payment_status_rls(&pool, issued, dec!(120))
        .await
        .expect("payment update query")
        .expect("issued invoice accepts payment");
    assert_eq!(paid.paid_amount, dec!(120));
    assert_eq!(status_of(&pool, issued).await, "paid");
}
