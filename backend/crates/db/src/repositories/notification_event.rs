//! Notification delivery-event repository (Epic 2B, Story 2B-C.3 / gap #969-4).
//!
//! Append-only writes from the notification pipeline plus the time-windowed
//! aggregation the admin analytics endpoint consumes. Both run on the
//! service-role pool (no per-request user GUC), which the table's RLS policy
//! permits — see migration `00187_create_notification_events.sql`.

use chrono::{DateTime, Utc};
use sqlx::Error as SqlxError;
use uuid::Uuid;

use crate::models::notification_event::{ChannelEventCounts, NewNotificationEvent};
use crate::DbPool;

#[derive(Clone)]
pub struct NotificationEventRepository {
    pool: DbPool,
}

impl NotificationEventRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Append a batch of delivery events in a single multi-row insert.
    ///
    /// Best-effort analytics: callers spawn this off the dispatch hot path and
    /// log (never propagate) errors so persistence can never affect delivery.
    /// Returns the number of rows written. An empty batch is a no-op.
    pub async fn insert_events(&self, events: &[NewNotificationEvent]) -> Result<u64, SqlxError> {
        if events.is_empty() {
            return Ok(0);
        }

        // Decompose into column arrays for a single UNNEST insert — one round
        // trip regardless of batch size.
        let notification_ids: Vec<Uuid> = events.iter().map(|e| e.notification_id).collect();
        let user_ids: Vec<Uuid> = events.iter().map(|e| e.user_id).collect();
        let channels: Vec<String> = events.iter().map(|e| e.channel.clone()).collect();
        let event_kinds: Vec<String> = events.iter().map(|e| e.event.clone()).collect();
        let error_messages: Vec<Option<String>> =
            events.iter().map(|e| e.error_message.clone()).collect();
        let occurred_ats: Vec<DateTime<Utc>> = events.iter().map(|e| e.occurred_at).collect();

        let result = sqlx::query(
            r#"
            INSERT INTO notification_events
                (notification_id, user_id, channel, event, error_message, occurred_at)
            SELECT * FROM UNNEST(
                $1::uuid[], $2::uuid[], $3::text[], $4::text[], $5::text[], $6::timestamptz[]
            )
            "#,
        )
        .bind(&notification_ids)
        .bind(&user_ids)
        .bind(&channels)
        .bind(&event_kinds)
        .bind(&error_messages)
        .bind(&occurred_ats)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Aggregate event counts per channel over `[from, to]`, optionally
    /// restricted to a single channel. Channels with no events in the window
    /// are simply absent from the result (callers synthesize zeroed rows if a
    /// fixed channel set is required).
    pub async fn aggregate_by_channel(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        channel: Option<&str>,
    ) -> Result<Vec<ChannelEventCounts>, SqlxError> {
        let rows = sqlx::query_as::<_, ChannelEventCounts>(
            r#"
            SELECT
                channel,
                COUNT(*) FILTER (WHERE event = 'sent')      AS sent,
                COUNT(*) FILTER (WHERE event = 'delivered') AS delivered,
                COUNT(*) FILTER (WHERE event = 'failed')    AS failed,
                COUNT(*) FILTER (WHERE event = 'skipped')   AS skipped,
                COUNT(*) FILTER (WHERE event = 'opened')    AS opened,
                COUNT(*) FILTER (WHERE event = 'clicked')   AS clicked
            FROM notification_events
            WHERE occurred_at >= $1
              AND occurred_at <= $2
              AND ($3::text IS NULL OR channel = $3)
            GROUP BY channel
            ORDER BY channel
            "#,
        )
        .bind(from)
        .bind(to)
        .bind(channel)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}

/// Roll a per-channel breakdown up into a single `all`-channel total. Kept as a
/// free function (not SQL) so the window query stays a pure GROUP BY and the
/// total is always consistent with the rows returned.
pub fn total_counts(rows: &[ChannelEventCounts]) -> ChannelEventCounts {
    rows.iter().fold(
        ChannelEventCounts {
            channel: "all".to_string(),
            sent: 0,
            delivered: 0,
            failed: 0,
            skipped: 0,
            opened: 0,
            clicked: 0,
        },
        |mut acc, r| {
            acc.sent += r.sent;
            acc.delivered += r.delivered;
            acc.failed += r.failed;
            acc.skipped += r.skipped;
            acc.opened += r.opened;
            acc.clicked += r.clicked;
            acc
        },
    )
}
