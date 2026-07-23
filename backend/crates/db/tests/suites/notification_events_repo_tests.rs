//! Repository + RLS tests for `notification_events` (Epic 2B, Story 2B-C.3 /
//! gap #969-4).
//!
//! Covers:
//!   1. Append-batch insert + per-channel aggregation + totals roll-up.
//!   2. Failure-rate math and the >5% alert threshold (firing, below-threshold,
//!      and the min-attempts noise guard).
//!   3. Channel filter and empty-window behavior (no false alert).
//!   4. The service-role RLS policy: a `FORCE`-bound non-super role with NO user
//!      GUC (the pipeline / admin service pool) may write and read, while the
//!      same role WITH a user GUC set (an end-user RLS connection) is denied.
//!
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS — so
//! the RLS test (4) creates a `NOSUPERUSER NOBYPASSRLS` role and `SET ROLE`s to
//! it, mirroring `vendor_rls_repo_tests.rs`, so `FORCE` actually binds.

use chrono::{DateTime, Duration, Utc};
use db::models::NewNotificationEvent;
use db::repositories::{total_counts, NotificationEventRepository};
use sqlx::PgPool;
use uuid::Uuid;

fn ev(channel: &str, event: &str, at: DateTime<Utc>) -> NewNotificationEvent {
    NewNotificationEvent {
        notification_id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        channel: channel.to_string(),
        event: event.to_string(),
        error_message: (event == "failed").then(|| "transport error".to_string()),
        occurred_at: at,
    }
}

/// Build `n` events of one (channel, event) kind at time `at`.
fn many(channel: &str, event: &str, n: usize, at: DateTime<Utc>) -> Vec<NewNotificationEvent> {
    (0..n).map(|_| ev(channel, event, at)).collect()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn insert_aggregate_and_alert_fires(pool: PgPool) {
    let repo = NotificationEventRepository::new(pool.clone());
    let now = Utc::now();

    // email: 18 sent + 2 failed (attempted 20, 10% failure).
    // push:  5 sent + 3 skipped (skipped excluded from failure denominator).
    let mut batch = Vec::new();
    batch.extend(many("email", "sent", 18, now));
    batch.extend(many("email", "failed", 2, now));
    batch.extend(many("push", "sent", 5, now));
    batch.extend(many("push", "skipped", 3, now));

    let written = repo.insert_events(&batch).await.expect("insert");
    assert_eq!(written, 28, "all rows written in one batch");

    let from = now - Duration::hours(1);
    let to = now + Duration::hours(1);
    let rows = repo
        .aggregate_by_channel(from, to, None)
        .await
        .expect("aggregate");

    // Two channels present, ordered by channel name (email, push).
    assert_eq!(rows.len(), 2);
    let email = rows
        .iter()
        .find(|r| r.channel == "email")
        .expect("email row");
    let push = rows.iter().find(|r| r.channel == "push").expect("push row");

    assert_eq!(email.sent, 18);
    assert_eq!(email.failed, 2);
    assert_eq!(email.attempted(), 20);
    assert!((email.failure_rate() - 0.10).abs() < 1e-9);

    assert_eq!(push.sent, 5);
    assert_eq!(push.skipped, 3);
    assert_eq!(push.failed, 0);
    assert_eq!(push.attempted(), 5);
    assert_eq!(push.failure_rate(), 0.0);

    // Totals roll-up: 23 sent, 2 failed, 3 skipped → attempted 25, rate 8%.
    let totals = total_counts(&rows);
    assert_eq!(totals.channel, "all");
    assert_eq!(totals.sent, 23);
    assert_eq!(totals.failed, 2);
    assert_eq!(totals.skipped, 3);
    assert_eq!(totals.attempted(), 25);
    assert!((totals.failure_rate() - 0.08).abs() < 1e-9);

    // Alert: 8% > 5% threshold with 25 >= 20 attempts → fires.
    assert!(totals.failure_rate() > 0.05);
    assert!(totals.attempted() >= 20);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn min_attempts_guard_suppresses_noisy_alert(pool: PgPool) {
    let repo = NotificationEventRepository::new(pool.clone());
    let now = Utc::now();

    // 1 failed of 1 attempt = 100% failure, but only 1 attempt: the alert's
    // min-attempts guard (>= 20) must keep it from firing on a near-idle window.
    repo.insert_events(&[ev("email", "failed", now)])
        .await
        .expect("insert");

    let rows = repo
        .aggregate_by_channel(now - Duration::hours(1), now + Duration::hours(1), None)
        .await
        .expect("aggregate");
    let totals = total_counts(&rows);

    assert_eq!(totals.attempted(), 1);
    assert!((totals.failure_rate() - 1.0).abs() < 1e-9);
    // Rate is over threshold, but attempts are below the guard → no fire.
    assert!(totals.attempted() < 20);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn channel_filter_and_empty_window(pool: PgPool) {
    let repo = NotificationEventRepository::new(pool.clone());
    let now = Utc::now();

    let mut batch = Vec::new();
    batch.extend(many("email", "sent", 4, now));
    batch.extend(many("push", "sent", 7, now));
    repo.insert_events(&batch).await.expect("insert");

    // Channel filter: only the push row comes back.
    let push_only = repo
        .aggregate_by_channel(
            now - Duration::hours(1),
            now + Duration::hours(1),
            Some("push"),
        )
        .await
        .expect("aggregate push");
    assert_eq!(push_only.len(), 1);
    assert_eq!(push_only[0].channel, "push");
    assert_eq!(push_only[0].sent, 7);

    // Empty window (entirely in the past) → no rows, zeroed totals, no alert.
    let empty = repo
        .aggregate_by_channel(now - Duration::hours(48), now - Duration::hours(24), None)
        .await
        .expect("aggregate empty");
    assert!(empty.is_empty());
    let totals = total_counts(&empty);
    assert_eq!(totals.attempted(), 0);
    assert_eq!(
        totals.failure_rate(),
        0.0,
        "empty window must not divide-by-zero"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn service_role_rls_allows_unset_guc_denies_user(pool: PgPool) {
    let now = Utc::now();

    // NOSUPERUSER NOBYPASSRLS role so FORCE ROW LEVEL SECURITY actually binds
    // (the production owner role is also bound by FORCE).
    let role = format!("ppt_rls_notif_evt_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!("GRANT SELECT, INSERT ON notification_events TO \"{role}\""),
        // The policy evaluates is_super_admin(); the bound role must EXECUTE it.
        format!("GRANT EXECUTE ON FUNCTION is_super_admin() TO \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .expect("role setup");
    }

    // (a) SERVICE PATH: no user GUC set → policy permits insert + read. This is
    //     exactly what the pipeline write / admin analytics read pool does.
    {
        let mut conn = pool.acquire().await.expect("acquire");
        sqlx::query("SELECT set_config('app.current_user_id', '', false)")
            .execute(&mut *conn)
            .await
            .expect("clear user guc");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        sqlx::query(
            r#"INSERT INTO notification_events (notification_id, user_id, channel, event, occurred_at)
               VALUES ($1, $2, 'email', 'sent', $3)"#,
        )
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        .bind(now)
        .execute(&mut *conn)
        .await
        .expect("service-role insert (unset GUC) must be permitted");

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notification_events")
            .fetch_one(&mut *conn)
            .await
            .expect("service-role read");
        assert_eq!(count, 1, "service role (unset GUC) sees the row it wrote");

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // (b) END-USER PATH: a user GUC is set (an authenticated RLS connection).
    //     The policy denies — this analytics log is operator-only.
    {
        let mut conn = pool.acquire().await.expect("acquire");
        sqlx::query("SELECT set_config('app.current_user_id', $1, false)")
            .bind(Uuid::new_v4().to_string())
            .execute(&mut *conn)
            .await
            .expect("set user guc");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notification_events")
            .fetch_one(&mut *conn)
            .await
            .expect("user-context read");
        assert_eq!(
            count, 0,
            "an end-user RLS connection (user GUC set) must NOT read the operator analytics log"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }
}
