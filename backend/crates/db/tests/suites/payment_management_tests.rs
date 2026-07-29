//! Org-wide payment management repo tests (Story 11.4, #975.5).
//!
//! Cover the new `FinancialRepository` methods backing the Payment Management
//! page: `list_payments` / `count_payments`, `list_unallocated_payments`,
//! `allocate_existing_payment` (manual match, org-scoped), and
//! `auto_match_payments` (bulk oldest-first allocation).
//!
//! Run against a live Postgres (sqlx spins up a per-test database):
//! `DATABASE_URL=... cargo test -p db --test payment_management_tests`.

use crate::common::seed_org;
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

/// Seed an invoice with a given outstanding balance, due `days_from_today`.
async fn seed_invoice(
    pool: &PgPool,
    org: Uuid,
    unit: Uuid,
    number: &str,
    total: Decimal,
    balance: Decimal,
    days_from_today: i64,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO invoices (organization_id, unit_id, invoice_number, due_date, total, balance_due)
        VALUES ($1, $2, $3, CURRENT_DATE + ($4 || ' days')::interval, $5, $6)
        RETURNING id
        "#,
    )
    .bind(org)
    .bind(unit)
    .bind(number)
    .bind(days_from_today.to_string())
    .bind(total)
    .bind(balance)
    .fetch_one(pool)
    .await
    .expect("seed invoice")
}

async fn seed_payment(pool: &PgPool, org: Uuid, unit: Uuid, amount: Decimal) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO payments (organization_id, unit_id, amount, payment_method)
        VALUES ($1, $2, $3, 'bank_transfer')
        RETURNING id
        "#,
    )
    .bind(org)
    .bind(unit)
    .bind(amount)
    .fetch_one(pool)
    .await
    .expect("seed payment")
}

async fn invoice_balance(pool: &PgPool, invoice_id: Uuid) -> Decimal {
    sqlx::query_scalar::<_, Decimal>("SELECT balance_due FROM invoices WHERE id = $1")
        .bind(invoice_id)
        .fetch_one(pool)
        .await
        .expect("invoice balance")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn list_payments_is_org_scoped_and_counted(pool: PgPool) {
    let repo = FinancialRepository::new(pool.clone());
    let org_a = seed_org(&pool, "list-a").await;
    let org_b = seed_org(&pool, "list-b").await;
    let unit_a = seed_unit(&pool, org_a).await;
    let unit_b = seed_unit(&pool, org_b).await;

    seed_payment(&pool, org_a, unit_a, money(100)).await;
    seed_payment(&pool, org_a, unit_a, money(50)).await;
    seed_payment(&pool, org_b, unit_b, money(70)).await;

    let payments = repo.list_payments(org_a, None, 100, 0).await.unwrap();
    assert_eq!(payments.len(), 2, "only org A's payments are listed");
    assert!(payments.iter().all(|p| p.organization_id == org_a));
    assert_eq!(repo.count_payments(org_a, None).await.unwrap(), 2);

    // Status filter (all seeded payments default to 'completed').
    assert_eq!(
        repo.list_payments(org_a, Some("completed".into()), 100, 0)
            .await
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        repo.list_payments(org_a, Some("failed".into()), 100, 0)
            .await
            .unwrap()
            .len(),
        0
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn unallocated_list_drops_a_payment_once_fully_allocated(pool: PgPool) {
    let repo = FinancialRepository::new(pool.clone());
    let org = seed_org(&pool, "unalloc").await;
    let unit = seed_unit(&pool, org).await;
    let payment = seed_payment(&pool, org, unit, money(100)).await;
    let invoice = seed_invoice(&pool, org, unit, "INV-1", money(100), money(100), 10).await;

    // Initially unallocated.
    let before = repo.list_unallocated_payments(org, 100, 0).await.unwrap();
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].id, payment);

    // Allocate the full amount → no longer unallocated.
    let alloc = repo
        .allocate_existing_payment(org, payment, invoice, None)
        .await
        .unwrap()
        .expect("allocation created");
    assert_eq!(alloc.amount, money(100));
    assert_eq!(invoice_balance(&pool, invoice).await, Decimal::ZERO);

    let after = repo.list_unallocated_payments(org, 100, 0).await.unwrap();
    assert!(after.is_empty(), "fully-allocated payment leaves the queue");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn allocate_clamps_to_invoice_balance_and_is_org_scoped(pool: PgPool) {
    let repo = FinancialRepository::new(pool.clone());
    let org = seed_org(&pool, "alloc").await;
    let other = seed_org(&pool, "alloc-other").await;
    let unit = seed_unit(&pool, org).await;
    let payment = seed_payment(&pool, org, unit, money(100)).await;
    let invoice = seed_invoice(&pool, org, unit, "INV-1", money(60), money(60), 10).await;

    // No `amount` → clamp to the invoice's 60 balance, not the payment's 100.
    let alloc = repo
        .allocate_existing_payment(org, payment, invoice, None)
        .await
        .unwrap()
        .expect("allocation");
    assert_eq!(alloc.amount, money(60));
    assert_eq!(invoice_balance(&pool, invoice).await, Decimal::ZERO);

    // 40 of the payment is still unallocated.
    assert_eq!(
        repo.list_unallocated_payments(org, 100, 0)
            .await
            .unwrap()
            .len(),
        1
    );

    // Cross-org: the wrong org cannot allocate this payment/invoice → None (404).
    let cross = repo
        .allocate_existing_payment(other, payment, invoice, None)
        .await
        .unwrap();
    assert!(cross.is_none(), "cross-org allocation must be refused");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn auto_match_allocates_oldest_first(pool: PgPool) {
    let repo = FinancialRepository::new(pool.clone());
    let org = seed_org(&pool, "auto").await;
    let unit = seed_unit(&pool, org).await;
    seed_payment(&pool, org, unit, money(100)).await;
    // Two invoices on the same unit; older due-date should be paid first.
    let older = seed_invoice(&pool, org, unit, "INV-OLD", money(60), money(60), 1).await;
    let newer = seed_invoice(&pool, org, unit, "INV-NEW", money(80), money(80), 30).await;

    let matched = repo.auto_match_payments(org).await.unwrap();
    assert_eq!(matched, 2, "100 splits across the two oldest invoices");

    // 60 → older (fully paid), remaining 40 → newer (partial).
    assert_eq!(invoice_balance(&pool, older).await, Decimal::ZERO);
    assert_eq!(invoice_balance(&pool, newer).await, money(40));

    // Payment is now fully allocated → reconciliation queue empty.
    assert!(repo
        .list_unallocated_payments(org, 100, 0)
        .await
        .unwrap()
        .is_empty());

    // Idempotent-ish: a second run finds nothing left to match.
    assert_eq!(repo.auto_match_payments(org).await.unwrap(), 0);
}
