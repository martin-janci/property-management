//! DB-backed regression tests for the support_tooling_events retention policy
//! (migration `00222_support_tooling_events_retention.sql`).
//!
//! Background
//! ----------
//! `support_tooling_events` (migration 00163) is an append-only admin audit
//! trail: triggers reject every UPDATE/DELETE so it is tamper-evident. It had
//! NO retention until migration 00222, which added
//! `cleanup_old_support_tooling_events(retention_days)` — a DB-native prune that
//! deletes aged rows via the sanctioned `app.retention_prune` GUC path while
//! leaving the immutability guarantee intact for every other mutation.
//!
//! These tests pin three properties:
//!   1. The retention prune deletes rows older than the retention window and
//!      keeps newer ones (returning the deleted count).
//!   2. An ordinary DELETE (outside the sanctioned path) is still rejected —
//!      the append-only guarantee is preserved.
//!   3. An ordinary UPDATE is still rejected.
//!
//! `#[sqlx::test]` connects as the Postgres superuser (bypasses RLS), so these
//! exercise the trigger + retention function directly, independent of the
//! FORCE-RLS policy added in migration 00165.

use db::repositories::PlatformAdminRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_admin_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Retention Admin', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed admin user")
}

/// Insert a support_tooling_events row with an explicit `occurred_at`.
///
/// `occurred_at` must be set at INSERT time because the immutability trigger
/// forbids UPDATE — a test cannot backdate a row after the fact.
async fn insert_event_at(pool: &PgPool, admin_user_id: Uuid, days_ago: i64) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO support_tooling_events (event_kind, admin_user_id, props, occurred_at)
        VALUES ('support_data_viewed', $1, '{}'::jsonb, NOW() - ($2 || ' days')::INTERVAL)
        RETURNING id
        "#,
    )
    .bind(admin_user_id)
    .bind(days_ago.to_string())
    .fetch_one(pool)
    .await
    .expect("insert support_tooling_events row")
}

async fn count_events(pool: &PgPool) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM support_tooling_events")
        .fetch_one(pool)
        .await
        .expect("count events")
}

#[sqlx::test]
async fn retention_prune_deletes_aged_rows_and_keeps_recent(pool: PgPool) {
    let admin = seed_admin_user(&pool, "retention-admin@test.local").await;

    // One row well past the 730-day window, one comfortably inside it.
    let old_id = insert_event_at(&pool, admin, 800).await;
    let recent_id = insert_event_at(&pool, admin, 10).await;
    assert_eq!(count_events(&pool).await, 2);

    let repo = PlatformAdminRepository::new(pool.clone());
    let deleted = repo
        .cleanup_old_support_tooling_events(730)
        .await
        .expect("retention prune should succeed");

    assert_eq!(deleted, 1, "exactly the aged row should be pruned");

    // The recent row survives; the aged row is gone.
    let surviving: Vec<Uuid> =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM support_tooling_events")
            .fetch_all(&pool)
            .await
            .expect("select surviving ids");
    assert_eq!(surviving, vec![recent_id]);
    assert!(
        !surviving.contains(&old_id),
        "aged row must have been pruned"
    );
}

#[sqlx::test]
async fn ordinary_delete_is_still_rejected(pool: PgPool) {
    let admin = seed_admin_user(&pool, "no-delete-admin@test.local").await;
    let id = insert_event_at(&pool, admin, 5).await;

    // A DELETE outside the sanctioned retention path (no app.retention_prune
    // GUC) must be rejected by the immutability trigger.
    let err = sqlx::query("DELETE FROM support_tooling_events WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .expect_err("ordinary DELETE must be rejected");
    assert!(
        err.to_string().contains("immutable"),
        "unexpected error: {err}"
    );

    // Row is still present.
    assert_eq!(count_events(&pool).await, 1);
}

#[sqlx::test]
async fn update_is_still_rejected(pool: PgPool) {
    let admin = seed_admin_user(&pool, "no-update-admin@test.local").await;
    let id = insert_event_at(&pool, admin, 5).await;

    let err = sqlx::query(
        "UPDATE support_tooling_events SET event_kind = 'support_data_viewed' WHERE id = $1",
    )
    .bind(id)
    .execute(&pool)
    .await
    .expect_err("UPDATE must always be rejected");
    assert!(
        err.to_string().contains("immutable"),
        "unexpected error: {err}"
    );
}
