//! DB-backed regression tests for the layout_change_events sink + retention
//! policy (migration `00225_create_layout_change_events.sql`, issue #2532).
//!
//! Background
//! ----------
//! `layout_change_events` is the append-only sink for the layout publish →
//! webhook → revalidate pipeline. Like `support_tooling_events` it is
//! tamper-evident: triggers reject every UPDATE/DELETE except the sanctioned
//! retention prune (`cleanup_old_layout_change_events`, gated by the
//! `app.retention_prune` GUC). Unlike support_tooling_events it carries no
//! RLS (global platform-admin table, precedent: layout_configs).
//!
//! These tests pin four properties:
//!   1. `record_change_event` inserts a row and returns its id.
//!   2. The retention prune deletes rows older than the window and keeps newer
//!      ones (returning the deleted count).
//!   3. An ordinary DELETE (outside the sanctioned path) is still rejected.
//!   4. An ordinary UPDATE is still rejected.

use db::repositories::{LayoutChangeEventKind, LayoutRepository};
use sqlx::PgPool;
use uuid::Uuid;

async fn count_events(pool: &PgPool) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM layout_change_events")
        .fetch_one(pool)
        .await
        .expect("count events")
}

/// Insert a row with an explicit `occurred_at` — the immutability trigger
/// forbids UPDATE, so a test cannot backdate a row after the fact.
async fn insert_event_at(pool: &PgPool, days_ago: i64) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO layout_change_events (event_kind, screen, props, occurred_at)
        VALUES ('layout_change_published', 'reality/listing-detail', '{}'::jsonb,
                NOW() - ($1 || ' days')::INTERVAL)
        RETURNING id
        "#,
    )
    .bind(days_ago.to_string())
    .fetch_one(pool)
    .await
    .expect("insert layout_change_events row")
}

#[sqlx::test]
async fn record_change_event_inserts_and_returns_id(pool: PgPool) {
    let repo = LayoutRepository::new();
    let delivery_id = Uuid::new_v4();
    let props = serde_json::json!({ "change_kind": "published", "layout_version": 3 });

    let id = repo
        .record_change_event(
            &pool,
            LayoutChangeEventKind::Published,
            Some("reality/listing-detail"),
            Some(delivery_id),
            None,
            &props,
        )
        .await
        .expect("record_change_event should succeed");

    let (kind, screen, got_delivery): (String, Option<String>, Option<Uuid>) = sqlx::query_as(
        "SELECT event_kind, screen, delivery_id FROM layout_change_events WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .expect("fetch inserted row");
    assert_eq!(kind, "layout_change_published");
    assert_eq!(screen.as_deref(), Some("reality/listing-detail"));
    assert_eq!(got_delivery, Some(delivery_id));
}

#[sqlx::test]
async fn retention_prune_deletes_aged_rows_and_keeps_recent(pool: PgPool) {
    // One row past the 730-day window, one comfortably inside it.
    let old_id = insert_event_at(&pool, 800).await;
    let recent_id = insert_event_at(&pool, 10).await;
    assert_eq!(count_events(&pool).await, 2);

    let deleted = LayoutRepository::new()
        .cleanup_old_layout_change_events(&pool, 730)
        .await
        .expect("retention prune should succeed");
    assert_eq!(deleted, 1, "exactly the aged row should be pruned");

    let surviving: Vec<Uuid> = sqlx::query_scalar::<_, Uuid>("SELECT id FROM layout_change_events")
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
    let id = insert_event_at(&pool, 5).await;

    let err = sqlx::query("DELETE FROM layout_change_events WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .expect_err("ordinary DELETE must be rejected");
    assert!(
        err.to_string().contains("immutable"),
        "unexpected error: {err}"
    );

    assert_eq!(count_events(&pool).await, 1);
}

#[sqlx::test]
async fn update_is_still_rejected(pool: PgPool) {
    let id = insert_event_at(&pool, 5).await;

    let err = sqlx::query(
        "UPDATE layout_change_events SET event_kind = 'layout_webhook_dispatched' WHERE id = $1",
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
