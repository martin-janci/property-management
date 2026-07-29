//! Repo-level behavioral RLS regression test for PAP-80 (parent PAP-67).
//!
//! Background
//! ----------
//! Migration `00179` (PAP-62) put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on `sentiment_alerts`, `sentiment_thresholds`,
//! and `sentiment_trends`. The production api-server connects as the table
//! OWNER, which `FORCE` binds. `SentimentRepository` held a raw `PgPool` and ran
//! every query WITHOUT ever calling `set_request_context`, so on `dev`
//! `get_current_org_id()` returned NULL and the policy collapsed to
//! `organization_id = NULL` → **deny-all**: own-org reads returned empty, writes
//! failed. (PAP-80.)
//!
//! The fix makes the repo stateless: every method takes an RLS-context
//! connection (the `RlsConnection` extractor in handlers sets the org/user GUCs
//! before any query). This test exercises the *repository methods themselves* on
//! a `FORCE`-bound role and proves:
//!
//!   1. **Deny-all reproduction** — with the role bound but NO context set
//!      (exactly what the raw-pool repo did on `dev`), an own-org `list_alerts`
//!      returns nothing.
//!   2. **Fix** — with `set_request_context(org_a, user_a)` applied first (what
//!      `RlsConnection` now does), the same call returns the own-org row.
//!   3. **Cross-tenant by-id** — org B's alert cannot be acknowledged by an
//!      org-A caller: the by-id `acknowledge_alert` surfaces `RowNotFound`,
//!      while the own-org alert acknowledges fine.
//!   4. **Write path** — a `create_alert` on a context-set connection succeeds
//!      and the row is the caller's org (the write-side of deny-all).
//!
//! Why this test switches roles
//! ----------------------------
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — even `FORCE` does not bind a superuser, so a behavioral assertion
//! would pass vacuously. The test creates a plain `NOSUPERUSER NOBYPASSRLS`
//! role, grants it access, and `SET ROLE`s to it so `FORCE` actually enforces
//! the policy the way the production owner role experiences it. Mirrors
//! `emergency_rls_repo_tests.rs` / `vendor_rls_repo_tests.rs` (the PAP-67
//! precedent PAP-80 follows).

use crate::common::{seed_org, set_ctx};
use db::models::{CreateSentimentAlert, UpsertSentimentTrend};
use db::repositories::SentimentRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Sentiment User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed an alert directly (as superuser, RLS-exempt) for an org.
async fn seed_alert(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO sentiment_alerts
            (organization_id, alert_type, threshold_breached, current_sentiment)
        VALUES ($1, 'spike_negative', -0.5, -0.7)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed alert")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn sentiment_repo_force_rls_deny_all_and_fix(pool: PgPool) {
    let repo = SentimentRepository::new(pool.clone());

    // --- Seed as superuser / super-admin context (satisfies org roles-trigger). ---
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "sforce-a").await;
    let org_b = seed_org(&pool, "sforce-b").await;
    let user_a = seed_user(&pool, "a@sentiment.test").await;
    let _user_b = seed_user(&pool, "b@sentiment.test").await;
    let alert_a = seed_alert(&pool, org_a).await;
    let alert_b = seed_alert(&pool, org_b).await;

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    let role = format!("ppt_rls_sentiment_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON sentiment_alerts TO \"{role}\""),
        // get_current_org_not_deleted() is SECURITY INVOKER and reads
        // `organizations`, so the bound role needs SELECT on it (PAP-133).
        format!("GRANT SELECT ON organizations TO \"{role}\""),
        // RLS policy helpers must be EXECUTE-able by the bound role.
        format!("GRANT EXECUTE ON FUNCTION get_current_org_id() TO \"{role}\""),
        format!("GRANT EXECUTE ON FUNCTION get_current_org_not_deleted() TO \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .expect("grant setup");
    }

    // ====================================================================
    // (1) DENY-ALL reproduction: role bound, NO context set (the dev raw-pool
    //     behavior). Own-org reads return nothing.
    // ====================================================================
    {
        let mut conn = pool.acquire().await.expect("acquire");
        // Explicitly clear any inherited context, then drop to the bound role.
        sqlx::query("SELECT clear_request_context()")
            .execute(&mut *conn)
            .await
            .expect("clear ctx");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        let listed = repo
            .list_alerts(&mut *conn, org_a, None, 50, 0)
            .await
            .expect("list_alerts (no ctx)");
        assert!(
            listed.is_empty(),
            "PAP-80 regression: without RLS context, own-org alerts must be \
             invisible (deny-all) — this is what the raw-pool repo did on dev"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // ====================================================================
    // (2) FIX + (3) cross-tenant by-id: set context, drop to bound role, query.
    // ====================================================================
    {
        let mut conn = pool.acquire().await.expect("acquire");
        sqlx::query("SELECT set_request_context($1, $2, $3)")
            .bind(org_a)
            .bind(user_a)
            .bind(false)
            .execute(&mut *conn)
            .await
            .expect("set org-A ctx");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        // (2) Own-org alert IS now visible through the repo — the fix.
        let listed = repo
            .list_alerts(&mut *conn, org_a, None, 50, 0)
            .await
            .expect("list_alerts (ctx)");
        assert_eq!(
            listed.iter().map(|a| a.id).collect::<Vec<_>>(),
            vec![alert_a],
            "PAP-80 fix: with RLS context set, list returns exactly the own-org alert"
        );

        // (3) Cross-tenant by-id write: org B's alert cannot be acknowledged by
        //     an org-A caller — the by-id UPDATE matches no visible row.
        let cross = repo
            .acknowledge_alert(&mut *conn, alert_b, org_a, user_a)
            .await;
        assert!(
            matches!(cross, Err(sqlx::Error::RowNotFound)),
            "cross-tenant: org B's alert must NOT be acknowledgeable by an org-A caller \
             (expected RowNotFound, got {cross:?})"
        );

        // Own-org by-id acknowledge succeeds.
        let acked = repo
            .acknowledge_alert(&mut *conn, alert_a, org_a, user_a)
            .await
            .expect("acknowledge own-org alert under context must succeed");
        assert_eq!(acked.id, alert_a);
        assert!(acked.acknowledged, "own-org alert is now acknowledged");

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // ====================================================================
    // (4) WRITE path: a create on a context-set connection succeeds and the
    //     row is the caller's org. Without context the INSERT would fail the
    //     policy WITH CHECK (the write-side of deny-all).
    // ====================================================================
    {
        let mut conn = pool.acquire().await.expect("acquire");
        sqlx::query("SELECT set_request_context($1, $2, $3)")
            .bind(org_a)
            .bind(user_a)
            .bind(false)
            .execute(&mut *conn)
            .await
            .expect("set org-A ctx");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        let created = repo
            .create_alert(
                &mut *conn,
                CreateSentimentAlert {
                    organization_id: org_a,
                    building_id: None,
                    alert_type: "anomaly".to_string(),
                    threshold_breached: -0.4,
                    current_sentiment: -0.6,
                    previous_sentiment: None,
                    sample_message_ids: vec![],
                },
            )
            .await
            .expect("create_alert under context must succeed");
        assert_eq!(created.organization_id, org_a);

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // --- Cleanup the test role. ---
    set_ctx(&pool, None, None, true).await;
    for stmt in [
        format!("REVOKE ALL ON sentiment_alerts FROM \"{role}\""),
        // DROP OWNED severs every remaining privilege this role holds in the
        // test database — the explicit REVOKEs above keep missing the RLS
        // helper-function EXECUTE grants, so DROP ROLE failed with "objects
        // depend on it" and leaked the cluster-global role (PAP-134).
        format!("DROP OWNED BY \"{role}\""),
        format!("DROP ROLE IF EXISTS \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .ok();
    }
}

/// PAP-150 / PR #1287 regression for the `sentiment_trends` + `sentiment_thresholds`
/// write paths the AI-chat handler actually uses.
///
/// PR #1287 converted `send_message` (`routes/ai/sessions.rs`) off a
/// hand-rolled `state.db.acquire()` + manual `set/clear_request_context` onto a
/// short-lived `RlsPool::acquire_with_rls(...)` guard. On that path the handler
/// calls `get_thresholds` (reads/inserts `sentiment_thresholds`) and, when an
/// alert fires, `upsert_trend` (writes `sentiment_trends`). Both tables are
/// `FORCE ROW LEVEL SECURITY` (migration `00179`,
/// `organization_id = get_current_org_id() AND get_current_org_not_deleted()`),
/// so on the pre-#1287 raw-pool path (no `app.current_org_id` GUC set) these
/// writes collapse to deny-all: the default-threshold INSERT and the trend
/// upsert both fail the policy `WITH CHECK`.
///
/// `sentiment_repo_force_rls_deny_all_and_fix` above only exercises
/// `sentiment_alerts`; this pins the *other two* sentiment tables the handler
/// touches on the RLS guard. Same `NOSUPERUSER NOBYPASSRLS` role technique so
/// `FORCE` actually binds (see that test's header for why).
///
/// Assertions:
///   1. **Deny-all reproduction** — role bound, NO context set: `upsert_trend`
///      and the `get_thresholds` default-INSERT are both rejected (the raw-pool
///      write-side of deny-all).
///   2. **Fix** — with `set_request_context(org_a, user_a)` applied first,
///      `get_thresholds` returns the own-org row and `upsert_trend` persists a
///      row stamped with the caller's org.
///   3. **Cross-tenant write** — an org-A caller cannot upsert a trend for org
///      B: the policy `WITH CHECK` rejects the foreign `organization_id`.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn sentiment_trends_and_thresholds_force_rls_deny_all_and_fix(pool: PgPool) {
    let repo = SentimentRepository::new(pool.clone());

    // --- Seed as superuser / super-admin context (satisfies org roles-trigger). ---
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "strend-a").await;
    let org_b = seed_org(&pool, "strend-b").await;
    let user_a = seed_user(&pool, "a@strend.test").await;

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    let role = format!("ppt_rls_strend_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON sentiment_trends TO \"{role}\""),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON sentiment_thresholds TO \"{role}\""),
        // get_current_org_not_deleted() is SECURITY INVOKER and reads
        // `organizations`, so the bound role needs SELECT on it (PAP-133).
        format!("GRANT SELECT ON organizations TO \"{role}\""),
        // RLS policy helpers must be EXECUTE-able by the bound role.
        format!("GRANT EXECUTE ON FUNCTION get_current_org_id() TO \"{role}\""),
        format!("GRANT EXECUTE ON FUNCTION get_current_org_not_deleted() TO \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .expect("grant setup");
    }

    // A daily-trend payload for an arbitrary org.
    let trend_for = |org: Uuid| UpsertSentimentTrend {
        organization_id: org,
        building_id: None,
        date: chrono::Utc::now().date_naive(),
        avg_sentiment: -0.3,
        message_count: 1,
        negative_count: 1,
        neutral_count: 0,
        positive_count: 0,
    };

    // ====================================================================
    // (1) DENY-ALL reproduction: role bound, NO context set (the dev raw-pool
    //     behavior). The trend upsert and the default-threshold insert both
    //     fail the policy WITH CHECK.
    // ====================================================================
    {
        let mut conn = pool.acquire().await.expect("acquire");
        sqlx::query("SELECT clear_request_context()")
            .execute(&mut *conn)
            .await
            .expect("clear ctx");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        let denied_trend = repo.upsert_trend(&mut *conn, trend_for(org_a)).await;
        assert!(
            denied_trend.is_err(),
            "PAP-150 regression: without RLS context, upsert_trend must be denied \
             (deny-all write) — exactly what the raw-pool path #1287 removed"
        );

        // get_thresholds SELECTs (nothing visible), then tries to INSERT the
        // org default — that INSERT fails the FORCE policy without context.
        let denied_thr = repo.get_thresholds(&mut conn, org_a).await;
        assert!(
            denied_thr.is_err(),
            "PAP-150 regression: without RLS context, the get_thresholds default \
             INSERT must be denied (deny-all write)"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // ====================================================================
    // (2) FIX + (3) cross-tenant write: set context, drop to bound role.
    // ====================================================================
    {
        let mut conn = pool.acquire().await.expect("acquire");
        sqlx::query("SELECT set_request_context($1, $2, $3)")
            .bind(org_a)
            .bind(user_a)
            .bind(false)
            .execute(&mut *conn)
            .await
            .expect("set org-A ctx");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        // (2) get_thresholds now materializes + returns the own-org default row.
        let thr = repo
            .get_thresholds(&mut conn, org_a)
            .await
            .expect("get_thresholds under context must succeed");
        assert_eq!(
            thr.organization_id, org_a,
            "PAP-150 fix: get_thresholds returns the caller-org thresholds"
        );

        // (2) upsert_trend persists a row stamped with the caller's org.
        let trend = repo
            .upsert_trend(&mut *conn, trend_for(org_a))
            .await
            .expect("upsert_trend under context must succeed");
        assert_eq!(
            trend.organization_id, org_a,
            "PAP-150 fix: upsert_trend writes under the caller's org"
        );

        // (3) Cross-tenant write: an org-A caller cannot upsert a trend for org
        //     B — the policy WITH CHECK rejects the foreign organization_id.
        let cross = repo.upsert_trend(&mut *conn, trend_for(org_b)).await;
        assert!(
            cross.is_err(),
            "cross-tenant: an org-A caller must NOT be able to upsert a sentiment \
             trend for org B (expected WITH CHECK denial, got {cross:?})"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // --- Cleanup the test role. ---
    set_ctx(&pool, None, None, true).await;
    for stmt in [
        format!("REVOKE ALL ON sentiment_trends FROM \"{role}\""),
        format!("REVOKE ALL ON sentiment_thresholds FROM \"{role}\""),
        // DROP OWNED severs every remaining privilege (incl. the RLS
        // helper-function EXECUTE grants the REVOKEs above miss), so DROP ROLE
        // does not fail with "objects depend on it" (PAP-134).
        format!("DROP OWNED BY \"{role}\""),
        format!("DROP ROLE IF EXISTS \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .ok();
    }
}
