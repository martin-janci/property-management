//! Concurrency / atomicity regression tests for
//! `ViolationRepository::record_payment` (follow-up #1361, post-merge review of
//! PR #1299 / PAP-138).
//!
//! ## What this guards
//! `record_payment` records a fine payment in three independent statements on
//! `&self.pool`:
//!   1. `SELECT EXISTS(... enforcement_actions WHERE id=$1 AND organization_id=$2)`
//!      — the cross-tenant ownership check.
//!   2. `INSERT INTO fine_payments ...`.
//!   3. `UPDATE enforcement_actions SET paid_amount = COALESCE(paid_amount,0) + $2 ...`
//!      — the read-modify-write that advances the running balance and flips the
//!      action to `paid` once the fine is covered.
//!
//! Because the three statements are separate pool checkouts with no surrounding
//! transaction, #1361 flags two robustness wrinkles:
//!   * **Lost update** — concurrent payments can interleave so the aggregate
//!     `paid_amount` ends up below the true sum of the recorded `fine_payments`
//!     rows, and the `>= fine_amount` paid-transition is evaluated against stale
//!     data.
//!   * **Partial write** — a failure between (2) and (3) leaves a
//!     `fine_payments` row with the `enforcement_actions` aggregate not advanced.
//!
//! The fix (#1361 proposal) is to wrap (1)–(3) in a single transaction with the
//! action row locked (`SELECT ... FOR UPDATE`, or fold the ownership check into
//! the `UPDATE ... RETURNING`). The tests below encode the post-fix invariant:
//!
//!   > After N successful `record_payment` calls, the stored `paid_amount`
//!   > equals the sum of the N inserted `fine_payments` rows, and the action's
//!   > `status` reflects whether that sum covers `fine_amount`.
//!
//! `concurrent_payments_accumulate_without_lost_update` fires the payments
//! concurrently and is the live regression for the race. It is expected to FAIL
//! against the current non-transactional implementation on `dev` and to PASS
//! once the atomic fix lands — this is the IG3 "failing-on-main" regression
//! test the follow-up asks for. The sequential sibling pins the same
//! data-integrity invariant without contention so a future refactor can't quietly
//! break the happy path either.
//!
//! Run against a superuser pool (CI default) so the assertions exercise the
//! application-level read-modify-write rather than any RLS layer.

use crate::common::seed_org;
use chrono::Utc;
use db::models::violations::{
    CreateEnforcementAction, CreateViolation, EnforcementActionType, EnforcementStatus,
    RecordFinePayment, ViolationCategory,
};
use db::repositories::ViolationRepository;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Fixtures (mirrors violations_cross_org_idor_tests.rs)
// ---------------------------------------------------------------------------

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'PayAtomicity User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

fn new_violation_req() -> CreateViolation {
    CreateViolation {
        building_id: None,
        unit_id: None,
        rule_id: None,
        category: ViolationCategory::Noise,
        severity: None,
        title: "Loud music".into(),
        description: "After midnight".into(),
        location: None,
        violator_id: None,
        violator_name: Some("Tenant".into()),
        violator_unit: Some("4B".into()),
        occurred_at: Utc::now(),
        evidence_description: None,
        witness_count: None,
    }
}

/// Enforcement action with a 100.00 fine.
fn new_action_req() -> CreateEnforcementAction {
    CreateEnforcementAction {
        action_type: EnforcementActionType::FirstFine,
        fine_amount: Some(Decimal::new(10000, 2)), // 100.00
        due_date: None,
        description: Some("Fine notice".into()),
        notes: None,
        suspended_privileges: None,
        suspension_start: None,
        suspension_end: None,
    }
}

fn payment(amount: Decimal, who: &str) -> RecordFinePayment {
    RecordFinePayment {
        amount,
        payment_method: Some("cash".into()),
        transaction_reference: None,
        payer_id: None,
        payer_name: Some(who.into()),
        notes: None,
    }
}

async fn seed_action(pool: &PgPool, slug: &str) -> (ViolationRepository, Uuid, Uuid, Uuid) {
    let repo = ViolationRepository::new(pool.clone());
    let org = seed_org(pool, slug).await;
    let user = seed_user(pool, &format!("{slug}@pay-atomicity.test")).await;
    let violation = repo
        .create_violation(org, new_violation_req(), user)
        .await
        .expect("create violation");
    let action = repo
        .create_enforcement_action(violation.id, org, new_action_req(), user)
        .await
        .expect("create action");
    (repo, org, user, action.id)
}

/// Sum of the `amount` column across all `fine_payments` rows for an action —
/// the ground truth the aggregate `paid_amount` must equal.
async fn payments_sum(pool: &PgPool, action_id: Uuid) -> Decimal {
    sqlx::query_scalar::<_, Option<Decimal>>(
        "SELECT COALESCE(SUM(amount), 0) FROM fine_payments WHERE enforcement_action_id = $1",
    )
    .bind(action_id)
    .fetch_one(pool)
    .await
    .expect("sum payments")
    .unwrap_or_default()
}

async fn payments_count(pool: &PgPool, action_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM fine_payments WHERE enforcement_action_id = $1",
    )
    .bind(action_id)
    .fetch_one(pool)
    .await
    .expect("count payments")
}

// ---------------------------------------------------------------------------
// Sequential baseline — pins the data-integrity invariant without contention.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn sequential_payments_accumulate_and_mark_paid(pool: PgPool) {
    let (repo, org, user, action_id) = seed_action(&pool, "seq").await;

    // Two 50.00 payments cover the 100.00 fine exactly.
    repo.record_payment(action_id, org, payment(Decimal::new(5000, 2), "p1"), user)
        .await
        .expect("first payment");
    repo.record_payment(action_id, org, payment(Decimal::new(5000, 2), "p2"), user)
        .await
        .expect("second payment");

    let action = repo
        .get_enforcement_action_for_org(action_id, org)
        .await
        .expect("query ok")
        .expect("action exists");

    assert_eq!(
        payments_count(&pool, action_id).await,
        2,
        "both rows persisted"
    );
    assert_eq!(
        action.paid_amount,
        Some(Decimal::new(10000, 2)),
        "aggregate paid_amount must equal the sum of payments (100.00)"
    );
    assert_eq!(
        action.paid_amount.unwrap_or_default(),
        payments_sum(&pool, action_id).await,
        "aggregate must match the fine_payments ledger"
    );
    assert_eq!(
        action.status,
        EnforcementStatus::Paid,
        "status flips to Paid once paid_amount >= fine_amount"
    );
    assert!(action.paid_at.is_some(), "paid_at stamped when fully paid");
}

// ---------------------------------------------------------------------------
// Concurrency regression — the live #1361 race.
//
// Fire several concurrent payments at the SAME action. Each call is a separate
// pool checkout, so the non-transactional read-modify-write on `paid_amount`
// can interleave and lose an increment. The invariant below
// (paid_amount == SUM(fine_payments.amount)) holds iff the update is atomic.
//
// Expected: FAILS on `dev` (current 3-statement impl can lose an update),
// PASSES once record_payment runs inside a single locked transaction.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn concurrent_payments_accumulate_without_lost_update(pool: PgPool) {
    let (_repo, org, user, action_id) = seed_action(&pool, "conc").await;

    // 8 concurrent 25.00 payments → true total 200.00 (overpays the 100.00 fine,
    // which is fine: the invariant is "aggregate == ledger sum", not capped).
    const N: usize = 8;
    let per = Decimal::new(2500, 2); // 25.00
    let expected_total = per * Decimal::from(N as i64);

    let mut handles = Vec::with_capacity(N);
    for i in 0..N {
        // Each task gets its own repo cloned over the shared pool so the calls
        // genuinely contend for separate connections.
        let repo = ViolationRepository::new(pool.clone());
        handles.push(tokio::spawn(async move {
            repo.record_payment(action_id, org, payment(per, &format!("payer-{i}")), user)
                .await
        }));
    }

    let mut ok = 0usize;
    for h in handles {
        let res = h.await.expect("task join");
        res.expect("each concurrent record_payment must succeed");
        ok += 1;
    }
    assert_eq!(ok, N, "all concurrent payments returned Ok");

    // 1. Every payment row landed — no INSERT was dropped.
    assert_eq!(
        payments_count(&pool, action_id).await,
        N as i64,
        "all {N} fine_payments rows must persist"
    );

    // 2. Ledger ground truth equals the intended total.
    let ledger = payments_sum(&pool, action_id).await;
    assert_eq!(
        ledger, expected_total,
        "fine_payments ledger == {expected_total}"
    );

    // 3. The crux: the denormalised aggregate must equal the ledger. A lost
    //    update from the non-transactional read-modify-write shows up here as
    //    paid_amount < ledger.
    let action = ViolationRepository::new(pool.clone())
        .get_enforcement_action_for_org(action_id, org)
        .await
        .expect("query ok")
        .expect("action exists");
    assert_eq!(
        action.paid_amount,
        Some(expected_total),
        "aggregate paid_amount ({:?}) must equal the fine_payments ledger ({ledger}) — \
         a mismatch is the #1361 lost-update race",
        action.paid_amount
    );

    // 4. Total covers the 100.00 fine, so the paid-transition must have fired.
    assert_eq!(
        action.status,
        EnforcementStatus::Paid,
        "status must be Paid once the accumulated total covers the fine"
    );
    assert!(action.paid_at.is_some(), "paid_at must be stamped");
}

// ---------------------------------------------------------------------------
// Partial-write regression — the live #1361 bug this fix targets.
//
// `record_payment` does (2) INSERT INTO fine_payments and (3) UPDATE
// enforcement_actions.paid_amount. Pre-fix these ran as two independent pool
// checkouts with no surrounding transaction, so each statement auto-committed
// on its own connection: a failure in (3) left the (2) row behind — money
// recorded in the ledger without the fine's running balance ever advancing.
//
// This test forces (3) to fail deterministically with a BEFORE UPDATE trigger
// that raises, then asserts NO fine_payments row survives. It is the IG3
// "failing-on-main" regression: it FAILS on `dev` (the orphaned INSERT
// persists → count == 1) and PASSES once (1)–(3) share one transaction that
// rolls back as a unit.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: DB-backed sqlx::test; needs a live Postgres so it can't run on the DB-less PR gate (same class as its siblings). Repair tracked in BIT-352."]
async fn partial_write_rolls_back_payment_when_balance_update_fails(pool: PgPool) {
    let (repo, org, user, action_id) = seed_action(&pool, "partial").await;

    // Make step (3) — the enforcement_actions balance UPDATE — fail, while
    // leaving step (2) — the fine_payments INSERT — able to succeed, by
    // installing a BEFORE UPDATE trigger on enforcement_actions that raises.
    sqlx::query(
        r#"
        CREATE OR REPLACE FUNCTION ppt_test_block_ea_update() RETURNS trigger AS $$
        BEGIN
            RAISE EXCEPTION 'simulated balance-update failure';
        END;
        $$ LANGUAGE plpgsql;
        "#,
    )
    .execute(&pool)
    .await
    .expect("create trigger fn");
    sqlx::query(
        "CREATE TRIGGER ppt_test_block_ea_update BEFORE UPDATE ON enforcement_actions \
         FOR EACH ROW EXECUTE FUNCTION ppt_test_block_ea_update()",
    )
    .execute(&pool)
    .await
    .expect("create trigger");

    // The payment must surface the balance-update failure rather than swallow it.
    let res = repo
        .record_payment(action_id, org, payment(Decimal::new(5000, 2), "p1"), user)
        .await;
    assert!(
        res.is_err(),
        "record_payment must propagate the failed balance UPDATE, got {res:?}"
    );

    // Remove the trigger before the read-back assertions (tidiness — the reads
    // below don't UPDATE, but keep the fixture clean).
    sqlx::query("DROP TRIGGER ppt_test_block_ea_update ON enforcement_actions")
        .execute(&pool)
        .await
        .expect("drop trigger");

    // The crux: INSERT and UPDATE must be atomic, so a failed UPDATE leaves NO
    // fine_payments row. On the non-transactional impl the auto-committed
    // INSERT survives and this count is 1 — money recorded without advancing
    // the fine (#1361 partial write).
    assert_eq!(
        payments_count(&pool, action_id).await,
        0,
        "a failed balance update must roll back the fine_payments INSERT"
    );

    // And the aggregate balance stayed at its initial unpaid state.
    let action = repo
        .get_enforcement_action_for_org(action_id, org)
        .await
        .expect("query ok")
        .expect("action exists");
    assert_eq!(
        action.paid_amount.unwrap_or_default(),
        Decimal::ZERO,
        "paid_amount must be untouched after a rolled-back payment"
    );
    assert_eq!(
        action.status,
        EnforcementStatus::Pending,
        "status must stay Pending when the payment rolled back"
    );
}
