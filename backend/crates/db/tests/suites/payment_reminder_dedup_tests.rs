//! DB-backed regression tests for the payment-reminder scheduler helpers
//! (`FinancialRepository::find_invoices_due_for_reminder`,
//! `mark_payment_reminder_sent`, `transition_invoices_to_overdue`).
//!
//! These are the IG3 evidence for #1790 / #1769: they seed real invoices around
//! the reminder/overdue boundaries and call the actual repo methods (unlike the
//! pre-existing closure tests in `financial.rs`, which re-implemented the
//! predicate locally and would pass even if the SQL were wrong).
//!
//! Coverage:
//! - persistent dedup: an invoice already stamped via `mark_payment_reminder_sent`
//!   is excluded from the next reminder sweep within the cooldown (the
//!   high-severity #1790 f1 / #1769 f1 fix). FAILS on pre-#1804 code, where the
//!   `last_payment_reminder_at` column and `mark_payment_reminder_sent` don't
//!   exist.
//! - `partial` invoices receive an upcoming-due reminder (#1790 f2).
//! - the due/overdue boundary uses SQL `CURRENT_DATE` (matching the
//!   `update_invoice_payment_status` trigger from migration 00040), not
//!   `Utc::now()` (#1769 f2).
//!
//! Run against a live Postgres (sqlx spins up a per-test database):
//! `DATABASE_URL=... cargo test -p db --test payment_reminder_dedup_tests`.

use crate::common::seed_org;
use db::models::financial::InvoiceStatus;
use db::repositories::FinancialRepository;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

fn money(units: i64) -> Decimal {
    Decimal::new(units * 100, 2) // whole-euro helper: money(100) == 100.00
}

async fn seed_unit(pool: &PgPool, org: Uuid) -> Uuid {
    let building = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, name, street, city, postal_code, country, status)
        VALUES ($1, 'B', 'Street 1', 'City', '00000', 'SK', 'active')
        RETURNING id
        "#,
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .expect("seed building");
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO units (building_id, designation) VALUES ($1, '1A') RETURNING id",
    )
    .bind(building)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

/// Seed an invoice with an explicit `status` and an outstanding `balance`, due
/// `days_from_today` relative to `CURRENT_DATE`. `status` is set directly: the
/// `update_invoice_payment_status` trigger only fires on `payment_allocations`
/// changes, so a freshly-inserted invoice with no allocation keeps the status
/// we give it here.
async fn seed_invoice(
    pool: &PgPool,
    org: Uuid,
    unit: Uuid,
    number: &str,
    status: &str,
    balance: Decimal,
    days_from_today: i64,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO invoices
            (organization_id, unit_id, invoice_number, status, due_date, total, balance_due)
        VALUES
            ($1, $2, $3, $4::invoice_status,
             CURRENT_DATE + make_interval(days => $5), $6, $6)
        RETURNING id
        "#,
    )
    .bind(org)
    .bind(unit)
    .bind(number)
    .bind(status)
    .bind(days_from_today as i32)
    .bind(balance)
    .fetch_one(pool)
    .await
    .expect("seed invoice")
}

fn ids(invoices: &[db::models::financial::Invoice]) -> Vec<Uuid> {
    invoices.iter().map(|i| i.id).collect()
}

/// IG3 (high): once an in-window `sent` invoice is stamped via
/// `mark_payment_reminder_sent`, it must drop out of the reminder sweep for the
/// 24h cooldown — proving the persistent dedup that stops the ~60s tick from
/// re-spamming the unit for the whole window. On pre-#1804 code this test can't
/// even compile (no `last_payment_reminder_at` column / `mark_payment_reminder_sent`),
/// and behaviourally the invoice would be returned on every call.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reminder_is_not_resent_after_stamp(pool: PgPool) {
    let repo = FinancialRepository::new(pool.clone());
    let org = seed_org(&pool, "dedup").await;
    let unit = seed_unit(&pool, org).await;
    // Due in 3 days → inside the 7-day reminder window, with an outstanding balance.
    let invoice = seed_invoice(&pool, org, unit, "INV-DEDUP", "sent", money(100), 3).await;

    // First sweep: the invoice is due for a reminder.
    let first = repo.find_invoices_due_for_reminder(7).await.unwrap();
    assert!(
        ids(&first).contains(&invoice),
        "an in-window sent invoice must be returned before any reminder is sent"
    );

    // Reminder dispatched → stamp it.
    repo.mark_payment_reminder_sent(invoice).await.unwrap();

    // Next sweep within the cooldown: it must NOT be returned again.
    let second = repo.find_invoices_due_for_reminder(7).await.unwrap();
    assert!(
        !ids(&second).contains(&invoice),
        "a just-reminded invoice must be skipped within the 24h dedup window"
    );
}

/// IG3 (#1790 f2): partially-paid invoices must get an upcoming-due reminder
/// before the overdue escalation (which already covers `partial`). On pre-fix
/// code the reminder predicate was `status = 'sent'` only, so a `partial`
/// invoice was silently excluded from reminders.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn partial_invoice_is_reminded(pool: PgPool) {
    let repo = FinancialRepository::new(pool.clone());
    let org = seed_org(&pool, "partial").await;
    let unit = seed_unit(&pool, org).await;
    let sent = seed_invoice(&pool, org, unit, "INV-SENT", "sent", money(100), 3).await;
    let partial = seed_invoice(&pool, org, unit, "INV-PARTIAL", "partial", money(40), 3).await;
    // A fully-paid invoice (zero balance) must never be reminded.
    let _paid = seed_invoice(&pool, org, unit, "INV-PAID", "partial", Decimal::ZERO, 3).await;

    let due = repo.find_invoices_due_for_reminder(7).await.unwrap();
    let due_ids = ids(&due);
    assert!(due_ids.contains(&sent), "sent invoice is reminded");
    assert!(
        due_ids.contains(&partial),
        "partial invoice must also receive the upcoming-due reminder (#1790 f2)"
    );
    assert_eq!(
        due.len(),
        2,
        "only the two outstanding in-window invoices qualify (zero-balance excluded)"
    );
}

/// IG3 (#1769 f2): the reminder window boundary uses SQL `CURRENT_DATE`. An
/// invoice due exactly today is NOT "upcoming" and must be excluded; one due
/// inside the window is included; one due past the window is excluded.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reminder_window_boundary_uses_current_date(pool: PgPool) {
    let repo = FinancialRepository::new(pool.clone());
    let org = seed_org(&pool, "window").await;
    let unit = seed_unit(&pool, org).await;
    let due_today = seed_invoice(&pool, org, unit, "INV-TODAY", "sent", money(100), 0).await;
    let in_window = seed_invoice(&pool, org, unit, "INV-IN", "sent", money(100), 5).await;
    let past_window = seed_invoice(&pool, org, unit, "INV-FAR", "sent", money(100), 8).await;

    let due_ids = ids(&repo.find_invoices_due_for_reminder(7).await.unwrap());
    assert!(
        !due_ids.contains(&due_today),
        "an invoice due exactly today is not upcoming and must be excluded"
    );
    assert!(
        due_ids.contains(&in_window),
        "due in 5 days is inside the window"
    );
    assert!(
        !due_ids.contains(&past_window),
        "due in 8 days is past the 7-day window"
    );
}

/// IG3 (#1769 f2): `transition_invoices_to_overdue` keys off SQL `CURRENT_DATE`
/// minus the grace period (the same definition the `update_invoice_payment_status`
/// trigger uses), so the scheduler and the trigger agree on "overdue". With a
/// 3-day grace, an invoice 4 days past due flips; one 2 days past due stays.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn overdue_transition_boundary_uses_current_date(pool: PgPool) {
    let repo = FinancialRepository::new(pool.clone());
    let org = seed_org(&pool, "overdue").await;
    let unit = seed_unit(&pool, org).await;
    // grace = 3 days; cutoff = CURRENT_DATE - 3.
    let past_cutoff = seed_invoice(&pool, org, unit, "INV-OLD", "sent", money(100), -4).await;
    let within_grace = seed_invoice(&pool, org, unit, "INV-GRACE", "partial", money(50), -2).await;
    // Inside grace AND a future-dated invoice must never be touched.
    let future = seed_invoice(&pool, org, unit, "INV-FUTURE", "sent", money(100), 5).await;

    let transitioned = ids(&repo.transition_invoices_to_overdue(3).await.unwrap());
    assert!(
        transitioned.contains(&past_cutoff),
        "an invoice 4 days past due with grace 3 must transition to overdue"
    );
    assert!(
        !transitioned.contains(&within_grace),
        "an invoice only 2 days past due is still inside the 3-day grace"
    );
    assert!(
        !transitioned.contains(&future),
        "a future-dated invoice is never overdue"
    );

    // The transition is idempotent: a second run finds nothing new.
    assert!(
        repo.transition_invoices_to_overdue(3)
            .await
            .unwrap()
            .is_empty(),
        "already-overdue rows leave the predicate, so a repeat run is a no-op"
    );

    // The transitioned invoice's status is now `overdue`.
    let status =
        sqlx::query_scalar::<_, InvoiceStatus>("SELECT status FROM invoices WHERE id = $1")
            .bind(past_cutoff)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, InvoiceStatus::Overdue);
}
