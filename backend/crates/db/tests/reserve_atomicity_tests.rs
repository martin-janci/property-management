use db::models::reserve_funds::{
    CreateReserveFund, FundTransactionType, FundType, RecordFundTransaction,
};
use db::models::RecordReserveTransaction;
use db::repositories::{BudgetRepository, ReserveFundRepository};
use rust_decimal_macros::dec;
use sqlx::PgPool;
use uuid::Uuid;

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

// PoolConnection<Postgres> requires explicit deref to satisfy Executor trait bounds;
// auto-deref does not work here due to lifetime constraints in sqlx's impl.
#[allow(clippy::explicit_auto_deref)]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_budget_reserve_transaction_atomicity(pool: PgPool) {
    let repo = BudgetRepository::new(pool.clone());

    // --- Seed data ---
    set_ctx(&pool, None, None, true).await;
    let org_id = seed_org(&pool, "budget-atom").await;
    let user_id = seed_user(&pool, "budget-atom@test.com").await;

    // Create a reserve fund
    let fund = repo
        .create_reserve_fund_rls(
            &pool,
            org_id,
            db::models::CreateReserveFund {
                building_id: None,
                name: Some("Atomicity Fund".to_string()),
                target_balance: Some(dec!(10000.0)),
                annual_contribution: dec!(1200.0),
                notes: None,
            },
        )
        .await
        .expect("create reserve fund");

    // --- Test transaction ---
    let mut conn = pool.acquire().await.expect("acquire");
    set_ctx(&pool, Some(org_id), Some(user_id), false).await;

    let data = RecordReserveTransaction {
        transaction_type: "contribution".to_string(),
        amount: dec!(500.0),
        description: Some("Test contribution".to_string()),
        reference_type: None,
        reference_id: None,
        transaction_date: chrono::Utc::now().date_naive(),
    };

    let txn = repo
        .record_reserve_transaction(&mut *conn, fund.id, user_id, data)
        .await
        .expect("record reserve transaction");

    assert_eq!(txn.amount, dec!(500.0));
    assert_eq!(txn.balance_after, dec!(500.0)); // Initial balance was 0

    // Verify fund balance was updated (via trigger or manual update)
    let updated_fund = repo
        .find_reserve_fund_by_id_rls(&mut *conn, org_id, fund.id)
        .await
        .expect("find fund")
        .expect("fund exists");

    assert_eq!(updated_fund.current_balance, dec!(500.0));
}

// PoolConnection<Postgres> requires explicit deref to satisfy Executor trait bounds;
// auto-deref does not work here due to lifetime constraints in sqlx's impl.
#[allow(clippy::explicit_auto_deref)]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_reserve_fund_management_transaction_atomicity(pool: PgPool) {
    let repo = ReserveFundRepository::new();

    // --- Seed data ---
    set_ctx(&pool, None, None, true).await;
    let org_id = seed_org(&pool, "rf-atom").await;
    let user_id = seed_user(&pool, "rf-atom@test.com").await;

    // Create a reserve fund
    let mut conn = pool.acquire().await.expect("acquire");
    set_ctx(&pool, Some(org_id), Some(user_id), false).await;

    let fund = repo
        .create_fund(
            &mut *conn,
            org_id,
            CreateReserveFund {
                building_id: None,
                name: "RF Atomicity Fund".to_string(),
                description: None,
                fund_type: FundType::Reserve,
                target_balance: Some(dec!(10000.0)),
                minimum_balance: Some(dec!(1000.0)),
                currency: Some("EUR".to_string()),
            },
            user_id,
        )
        .await
        .expect("create reserve fund");

    // --- Test transaction ---
    let data = RecordFundTransaction {
        transaction_type: FundTransactionType::Contribution,
        amount: dec!(500.0),
        description: Some("Test contribution".to_string()),
        reference_number: None,
        contribution_schedule_id: None,
        transfer_to_fund_id: None,
        requires_approval: None,
    };

    let txn = repo
        .record_transaction(&mut *conn, org_id, fund.id, data, user_id)
        .await
        .expect("record transaction");

    assert_eq!(txn.amount, dec!(500.0));
    assert_eq!(txn.balance_after, dec!(500.0));

    // Verify fund balance was updated
    let updated_fund = repo
        .get_fund(&mut *conn, org_id, fund.id)
        .await
        .expect("get fund")
        .expect("fund exists");

    assert_eq!(updated_fund.current_balance, dec!(500.0));
}
