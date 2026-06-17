//! Repo-level regression test for `SubscriptionRepository::list_public_plans`
//! on an RLS-cleared connection (GH #1305, PR #1287 follow-up).
//!
//! Background
//! ----------
//! PR #1287 converted `list_public_plans` from `state.db` (raw pool) to
//! `acquire_public()` (context-cleared pool connection). This is correct today
//! because the `subscription_plans_read` policy:
//!
//! ```sql
//! CREATE POLICY subscription_plans_read ON subscription_plans
//!     FOR SELECT USING (is_public = TRUE OR is_active = TRUE);
//! ```
//!
//! is context-independent — it does not reference `get_current_org_id()` — so
//! clearing RLS context does not lock out public reads. However, if a future
//! migration adds a tenant predicate or `FORCE ROW LEVEL SECURITY` with a
//! context-dependent policy, the acquire_public() path would silently break
//! unauthenticated reads. This test pins the contract.
//!
//! Assertions
//! ----------
//! 1. Plans that are `is_active = TRUE AND is_public = TRUE` are returned.
//! 2. Plans that are private (`is_public = FALSE`) or inactive (`is_active = FALSE`)
//!    are filtered out.
//! 3. The read works on an executor with **no RLS context set** (simulates
//!    the `acquire_public()` cleared-context path).

use db::repositories::SubscriptionRepository;
use sqlx::PgPool;
use uuid::Uuid;

/// Seed a plan directly as superuser with explicit public/active flags.
async fn seed_plan(pool: &PgPool, name: &str, is_active: bool, is_public: bool) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO subscription_plans
            (name, display_name, monthly_price, currency, is_active, is_public)
        VALUES ($1, $2, 10.00, 'EUR', $3, $4)
        RETURNING id
        "#,
    )
    .bind(name)
    .bind(format!("{name} (test)"))
    .bind(is_active)
    .bind(is_public)
    .fetch_one(pool)
    .await
    .expect("seed plan")
}

/// `list_public_plans` returns only plans where `is_active AND is_public`,
/// even on a connection with no RLS context set.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_public_plans_filters_correctly_without_rls_context(pool: PgPool) {
    // Seed four variants.
    let public_active = seed_plan(&pool, "plan-pub-act", true, true).await;
    let public_inactive = seed_plan(&pool, "plan-pub-inact", false, true).await;
    let private_active = seed_plan(&pool, "plan-priv-act", true, false).await;
    let private_inactive = seed_plan(&pool, "plan-priv-inact", false, false).await;

    let repo = SubscriptionRepository::new(pool.clone());

    // Acquire a connection and deliberately clear RLS context before the query,
    // mirroring the `acquire_public()` path used by the handler.
    let mut conn = pool.acquire().await.expect("acquire connection");
    sqlx::query("SELECT set_request_context(NULL, NULL, false)")
        .execute(&mut *conn)
        .await
        .expect("clear rls context");

    let plans = repo
        .list_public_plans(&mut *conn)
        .await
        .expect("list_public_plans");

    let returned_ids: Vec<Uuid> = plans.iter().map(|p| p.id).collect();

    assert!(
        returned_ids.contains(&public_active),
        "public+active plan must be returned"
    );
    assert!(
        !returned_ids.contains(&public_inactive),
        "public but inactive plan must NOT be returned"
    );
    assert!(
        !returned_ids.contains(&private_active),
        "active but private plan must NOT be returned"
    );
    assert!(
        !returned_ids.contains(&private_inactive),
        "inactive+private plan must NOT be returned"
    );
}

/// `list_public_plans` still works when RLS context IS set (i.e., the
/// cleared path is not the only path — authenticated callers can also see plans).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_public_plans_works_with_rls_context_set(pool: PgPool) {
    let public_active = seed_plan(&pool, "plan-ctx-pub-act", true, true).await;

    // Seed an org so set_request_context has a valid org to reference.
    let org_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO organizations (name, slug, contact_email, status) \
         VALUES ('Plans Ctx Org', 'plans-ctx-org', 'ctx@plans.test', 'active') \
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("seed org");

    let repo = SubscriptionRepository::new(pool.clone());

    let mut conn = pool.acquire().await.expect("acquire connection");
    // Set a real org context (authenticated caller scenario).
    sqlx::query("SELECT set_request_context($1, NULL, false)")
        .bind(org_id)
        .execute(&mut *conn)
        .await
        .expect("set rls context");

    let plans = repo
        .list_public_plans(&mut *conn)
        .await
        .expect("list_public_plans with context");

    let ids: Vec<Uuid> = plans.iter().map(|p| p.id).collect();
    assert!(
        ids.contains(&public_active),
        "public+active plan must be visible even with an org context set"
    );
}
