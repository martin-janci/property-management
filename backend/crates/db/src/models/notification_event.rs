//! Notification delivery-event models (Epic 2B, Story 2B-C.3 / gap #969-4).
//!
//! `notification_events` is an append-only log the notification pipeline writes
//! `DeliveryRecord`s to (asynchronously). The admin analytics endpoint reads
//! aggregates from it to compute per-channel delivery/failure rates and drive
//! the >5% failure-rate alert.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// A single persisted delivery event (one row of `notification_events`).
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize, ToSchema)]
pub struct NotificationEvent {
    pub id: Uuid,
    pub notification_id: Uuid,
    pub user_id: Uuid,
    /// `push` | `email` | `in_app`.
    pub channel: String,
    /// `pending` | `sent` | `delivered` | `failed` | `skipped` | `opened` | `clicked`.
    pub event: String,
    pub error_message: Option<String>,
    pub occurred_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// An event to append. Built from a `DeliveryRecord` by the pipeline; `id` /
/// `created_at` are assigned by the database.
#[derive(Debug, Clone)]
pub struct NewNotificationEvent {
    pub notification_id: Uuid,
    pub user_id: Uuid,
    pub channel: String,
    pub event: String,
    pub error_message: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

/// Per-channel aggregate counts over a time window. Raw `BIGINT` counts come
/// straight from the `COUNT(*) FILTER (...)` query; the rate is derived in the
/// handler so the SQL stays a pure GROUP BY.
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize, ToSchema)]
pub struct ChannelEventCounts {
    /// Channel (`push` | `email` | `in_app`), or `all` for the synthesized total row.
    pub channel: String,
    pub sent: i64,
    pub delivered: i64,
    pub failed: i64,
    pub skipped: i64,
    pub opened: i64,
    pub clicked: i64,
}

impl ChannelEventCounts {
    /// Delivery attempts that resolved to success or failure. Skipped channels
    /// are intentional non-attempts (preference/quiet-hours) and are excluded
    /// from the failure-rate denominator.
    pub fn attempted(&self) -> i64 {
        self.sent + self.failed
    }

    /// Failure rate over attempts in `[0.0, 1.0]`; `0.0` when there were no
    /// attempts (empty window → no false alert).
    pub fn failure_rate(&self) -> f64 {
        let attempted = self.attempted();
        if attempted <= 0 {
            0.0
        } else {
            self.failed as f64 / attempted as f64
        }
    }
}
