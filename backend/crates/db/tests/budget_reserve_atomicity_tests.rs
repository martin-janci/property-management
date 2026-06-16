//! Concurrency regression test for `BudgetRepository::record_reserve_transaction`
//! atomicity (#1371, follow-up to PR #1327 / PAP-180).
//!
//! Background
//! ----------
//! `record_reserve_transaction` is a read-modify-write on a reserve fund:
//!
//!   1. `SELECT * FROM reserve_funds WHERE id = $1` → `current_balance`
//!   2. compute `balance_after = current_balance ± amount` in application code
//!   3. `INSERT INTO reserve_fund_transactions (... balance_after ...)`, whose
//!      `AFTER INSERT` trigger writes `reserve_funds.current_balance = balance_after`
//!
//! PR #1327 moved both queries onto a single RLS-bound connection (fixing RLS
//! enforcement) but left them **autocommitting independently** — no transaction,
//! no row lock. Two concurrent callers against the same fund both read the same
//! `current_balance`, compute the same `balance_after`, and the trigger writes
//! that same value twice: a classic lost-update race that silently drops one
//! contribution from the running balance.
//!
//! The fix (#1371) wraps the read-modify-write in an explicit transaction and
//! locks the fund row with `SELECT ... FOR UPDATE`, so concurrent writers
//! serialize: the second caller blocks until the first commits and then reads
//! the *updated* balance.
//!
//! This test fires N concurrent `record_reserve_transaction` contributions of a
//! fixed amount against one fund and asserts:
//!   * the final `reserve_funds.current_balance` == start + N * amount (no lost
//!     update), and
//!   * the set of ledger `balance_after` values is exactly the monotonic ladder
//!     `{start+amount, start+2*amount, …, start+N*amount}` with no duplicates
//!     (each writer observed a distinct predecessor).
//!
//! Each writer runs on its own pool connection under the same org's RLS context
//! and a `FORCE`-bound NOSUPERUSER role, mirroring the production owner-role path
//! and the sibling `budget_rls_repo_tests.rs` / `reserve_funds_rls_repo_tests.rs`.
//!
//! ## STATUS: `#[ignore]`d — blocked on a missing-table migration
//!
//! Migration `00097_create_reserve_funds.sql` (Epic 141) `DROP TABLE IF EXISTS
//! reserve_fund_transactions` (and its `trg_update_reserve_fund_balance`
//! trigger) to "replace the simpler reserve_funds tables from migration 57", but
//! **never recreates `reserve_fund_transactions` or the balance trigger**. At the
//! migrated HEAD schema the table the repository writes to does not exist, so
//! `record_reserve_transaction` would fail at runtime (the route at
//! `POST /reserve-funds/{id}/transactions` is still wired). This is a pre-existing
//! latent bug outside the QA test scope; restoring the table + trigger is a
//! `db-migration` follow-up. The test below is written and ready: drop the
//! `#[ignore]` once the table is restored.

use db::models::budget::RecordReserveTransaction;
use db::repositories::BudgetRepository;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

const WRITERS: i64 = 8;
const AMOUNT: i64 = 100; // each contribution adds 100.00
const START_BALANCE: i64 = 1_000;

async fn set_ctx(pool: &PgPool, org_id: Option<Uuid>, user_id: Option<Uuid>, is_super_admin: bool) {
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(org_id)
        .bind(user_id)
        .bind(is_super_admin)
        .execute(pool)
        .await
        .expect("set_request_context");
}

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("Atomicity {slug}"))
    .bind(slug)
    .bind(format!("{slug}@atomicity.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Atomicity User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed a reserve fund with a known starting balance (superuser, RLS-exempt).
async fn seed_fund(pool: &PgPool, org_id: Uuid, name: &str, start: Decimal) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO reserve_funds (organization_id, name, current_balance)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(name)
    .bind(start)
    .fetch_one(pool)
    .await
    .expect("seed reserve fund")
}

#[ignore = "blocked on #1371 schema follow-up: migration 00097 drops \
            reserve_fund_transactions + its balance trigger and never recreates \
            them; remove #[ignore] once a db-migration restores the table"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn record_reserve_transaction_is_atomic_under_concurrency(pool: PgPool) {
    let repo = BudgetRepository::new(pool.clone());

    // --- Seed as super-admin context (satisfies org roles-trigger). ---
    set_ctx(&pool, None, None, true).await;
    let org = seed_org(&pool, "rf-atomic").await;
    let user = seed_user(&pool, "writer@atomicity.test").await;
    let fund = seed_fund(&pool, org, "Concurrent Fund", Decimal::from(START_BALANCE)).await;

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE actually binds, like production. ---
    let role = format!("ppt_rls_rf_atomic_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!(
            "GRANT SELECT, INSERT, UPDATE, DELETE ON reserve_funds, \
             reserve_fund_transactions TO \"{role}\""
        ),
        format!(
            "GRANT EXECUTE ON FUNCTION get_current_org_id(), is_super_admin(), \
             get_current_org_not_deleted() TO \"{role}\""
        ),
        format!("GRANT SELECT ON organizations TO \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .expect("grant setup");
    }

    // --- Fire WRITERS concurrent contributions, each on its own connection. ---
    let mut handles = Vec::new();
    for i in 0..WRITERS {
        let pool = pool.clone();
        let repo = repo.clone();
        let role = role.clone();
        handles.push(tokio::spawn(async move {
            let mut conn = pool.acquire().await.expect("acquire");
            sqlx::query("SELECT set_request_context($1, $2, $3)")
                .bind(org)
                .bind(user)
                .bind(false)
                .execute(&mut *conn)
                .await
                .expect("set ctx");
            sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
                .execute(&mut *conn)
                .await
                .expect("set role");

            let txn = repo
                .record_reserve_transaction(
                    &mut conn,
                    fund,
                    user,
                    RecordReserveTransaction {
                        transaction_type: "contribution".to_string(),
                        amount: Decimal::from(AMOUNT),
                        description: Some(format!("concurrent #{i}")),
                        reference_type: None,
                        reference_id: None,
                        transaction_date: chrono::Utc::now().date_naive(),
                    },
                )
                .await
                .expect("record_reserve_transaction");

            sqlx::query("RESET ROLE").execute(&mut *conn).await.ok();
            txn.balance_after
        }));
    }

    let mut balances_after = Vec::new();
    for h in handles {
        balances_after.push(h.await.expect("writer task"));
    }

    // --- Assert: no lost update on the fund's running balance. ---
    set_ctx(&pool, Some(org), Some(user), true).await;
    let final_balance: Decimal =
        sqlx::query_scalar("SELECT current_balance FROM reserve_funds WHERE id = $1")
            .bind(fund)
            .fetch_one(&pool)
            .await
            .expect("read final balance");

    let expected_final = Decimal::from(START_BALANCE + WRITERS * AMOUNT);
    assert_eq!(
        final_balance, expected_final,
        "#1371 lost-update regression: after {WRITERS} concurrent contributions of \
         {AMOUNT} the fund balance must be {expected_final} (start {START_BALANCE} + \
         {WRITERS}*{AMOUNT}); a smaller value means a contribution was clobbered by a \
         stale read"
    );

    // --- Assert: balance_after values form the exact monotonic ladder. ---
    balances_after.sort();
    let expected_ladder: Vec<Decimal> = (1..=WRITERS)
        .map(|n| Decimal::from(START_BALANCE + n * AMOUNT))
        .collect();
    assert_eq!(
        balances_after, expected_ladder,
        "#1371: each writer must observe a distinct predecessor balance — the set of \
         balance_after values must be exactly {{start+amount … start+N*amount}} with no \
         duplicates; duplicates prove two writers read the same stale balance"
    );

    // --- Cleanup the test role. ---
    set_ctx(&pool, None, None, true).await;
    for stmt in [
        format!("REVOKE ALL ON reserve_funds, reserve_fund_transactions FROM \"{role}\""),
        format!("REVOKE ALL ON organizations FROM \"{role}\""),
        format!("DROP ROLE IF EXISTS \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .ok();
    }
}
