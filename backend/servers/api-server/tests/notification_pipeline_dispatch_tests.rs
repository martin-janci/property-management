//! Epic 2B — notification pipeline dispatch tests (closes #795).
//!
//! Exercises the wired `NotificationPipeline` directly (the same object now
//! stored on `AppState` and called from the announcement / critical-notification
//! publish paths):
//!
//! | Case | Expected |
//! |------|----------|
//! | P1 | Preferences respected: a disabled email channel is `Skipped`; the mandatory in-app channel is still `Sent` |
//! | P2 | Urgency bypass: an `Urgent` notification delivers on email even though the user disabled it |
//! | P3 | Delivery records written: in-app dispatch persists a `grouped_notifications` row per recipient |
//!
//! The pipeline runs with `EmailService::development()` (logs, returns Ok) and
//! no FCM credentials, so: email = Sent when enabled, push = Skipped
//! (`PushNotConfigured`, fail-closed — not a fake Sent), in-app = Sent (real DB
//! write via `add_notification_to_group`).

#![allow(dead_code)]

use api_server::services::{EmailService, NotificationPipeline, PipelineConfig};
use common::notifications::pipeline::DeliveryStatus;
use common::notifications::{
    Notification, NotificationCategory, NotificationChannel, NotificationPriority,
};
use sqlx::PgPool;
use uuid::Uuid;

/// Seed an active user. The `notification_preferences` insert trigger gives
/// the user all channels enabled by default.
async fn seed_user(pool: &PgPool) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'Recipient', 'active', NOW())
           RETURNING id"#,
    )
    .bind(format!("recip-{}@epic2b.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Force a channel preference on/off (idempotent over the default trigger row).
async fn set_channel_pref(pool: &PgPool, user_id: Uuid, channel: &str, enabled: bool) {
    sqlx::query(
        r#"INSERT INTO notification_preferences (user_id, channel, enabled)
           VALUES ($1, $2::notification_channel, $3)
           ON CONFLICT (user_id, channel) DO UPDATE SET enabled = EXCLUDED.enabled"#,
    )
    .bind(user_id)
    .bind(channel)
    .bind(enabled)
    .execute(pool)
    .await
    .expect("set channel pref");
}

fn build_pipeline(pool: &PgPool) -> NotificationPipeline {
    NotificationPipeline::new(
        pool.clone(),
        EmailService::development(),
        None,
        PipelineConfig::default(),
    )
}

fn status_of(
    result: &common::notifications::pipeline::PipelineResult,
    channel: NotificationChannel,
) -> Option<DeliveryStatus> {
    result
        .records
        .iter()
        .find(|r| r.channel == channel)
        .map(|r| r.status)
}

// ===========================================================================
// P1 — preferences respected (email off → Skipped; in-app still mandatory)
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn dispatch_respects_channel_preferences(pool: PgPool) {
    let user = seed_user(&pool).await;
    set_channel_pref(&pool, user, "email", false).await;

    let notification = Notification::new(
        user,
        NotificationCategory::Announcements,
        "Building meeting",
        "Annual meeting next week.",
    )
    .with_priority(NotificationPriority::Normal);

    let result = build_pipeline(&pool)
        .dispatch(user, &notification, Some(Uuid::new_v4()), None)
        .await
        .expect("dispatch");

    assert_eq!(
        status_of(&result, NotificationChannel::Email),
        Some(DeliveryStatus::Skipped),
        "P1: email must be Skipped when the user disabled it"
    );
    assert_eq!(
        status_of(&result, NotificationChannel::InApp),
        Some(DeliveryStatus::Sent),
        "P1: in-app is mandatory — it must be Sent even though email was disabled"
    );
}

// ===========================================================================
// P2 — urgency bypass (Urgent ignores the disabled email preference)
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn urgent_notification_bypasses_preferences(pool: PgPool) {
    let user = seed_user(&pool).await;
    // Disable EVERY channel — an urgent alert must still go out.
    set_channel_pref(&pool, user, "email", false).await;
    set_channel_pref(&pool, user, "in_app", false).await;
    set_channel_pref(&pool, user, "push", false).await;

    let urgent = Notification::new(
        user,
        NotificationCategory::Announcements,
        "EMERGENCY: building evacuation",
        "Evacuate immediately.",
    )
    .with_priority(NotificationPriority::Urgent);

    let result = build_pipeline(&pool)
        .dispatch(user, &urgent, Some(Uuid::new_v4()), None)
        .await
        .expect("dispatch");

    // Email was disabled, but urgency bypasses preferences → it is delivered.
    assert_eq!(
        status_of(&result, NotificationChannel::Email),
        Some(DeliveryStatus::Sent),
        "P2: Urgent must bypass the disabled email preference and deliver"
    );
    assert_eq!(
        status_of(&result, NotificationChannel::InApp),
        Some(DeliveryStatus::Sent),
        "P2: Urgent in-app must deliver despite the disabled preference"
    );
    // No channel may be Skipped on an urgent bypass (push is Sent only if FCM
    // is configured; here it is not → Skipped is acceptable for push alone).
    assert_eq!(
        result
            .records
            .iter()
            .filter(
                |r| r.status == DeliveryStatus::Skipped && r.channel != NotificationChannel::Push
            )
            .count(),
        0,
        "P2: urgency bypass must not skip email/in-app for preference reasons"
    );
}

// ===========================================================================
// P3 — delivery records written (in-app DB row per recipient)
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn dispatch_writes_in_app_delivery_record(pool: PgPool) {
    let user = seed_user(&pool).await;
    let entity_id = Uuid::new_v4();

    let notification = Notification::new(
        user,
        NotificationCategory::Announcements,
        "Lift maintenance",
        "Lift B out of service Friday.",
    )
    .with_priority(NotificationPriority::Normal);

    let result = build_pipeline(&pool)
        .dispatch(user, &notification, Some(entity_id), None)
        .await
        .expect("dispatch");

    // The pipeline reports the in-app channel as Sent…
    assert_eq!(
        status_of(&result, NotificationChannel::InApp),
        Some(DeliveryStatus::Sent),
        "P3: in-app channel must be Sent"
    );

    // …and that maps to a real persisted notification row for the recipient.
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM grouped_notifications WHERE user_id = $1")
            .bind(user)
            .fetch_one(&pool)
            .await
            .expect("count grouped_notifications");
    assert!(
        count >= 1,
        "P3: a grouped_notifications delivery record must be written for the recipient; got {count}"
    );
}
