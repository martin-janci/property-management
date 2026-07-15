//! Notification delivery pipeline types (Epic 2B).
//!
//! # Stories
//! - **2b-1** Channel infrastructure: `TransportAdapter` trait + `DeliveryRequest`
//! - **2b-2** Preference routing: `PreferenceRouter` resolves which channels are live
//! - **2b-3** Delivery tracking: `DeliveryRecord` + `DeliveryStatus`
//! - **2b-4** Transport adapters: `EmailTransport`, `PushTransport`, `InAppTransport` traits
//! - **2b-5** Pipeline integration: `NotificationPipeline` orchestrator
//!
//! Design goals:
//! - Zero mandatory DB round-trips on the hot path when channels are known up front.
//! - Each transport adapter is a thin async trait — easy to swap/mock in tests.
//! - Delivery tracking records are written asynchronously; pipeline does not block on them.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::{Notification, NotificationChannel, NotificationError};

// ============================================================================
// Story 2b-3 — Delivery tracking
// ============================================================================

/// Outcome of a single channel delivery attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStatus {
    /// Delivery queued / in-flight.
    Pending,
    /// Delivered successfully.
    Sent,
    /// Adapter returned an error; may be retried.
    Failed,
    /// User preference disabled this channel — skipped without an attempt.
    Skipped,
}

impl DeliveryStatus {
    /// `true` when the record represents a terminal success.
    pub fn is_sent(&self) -> bool {
        *self == DeliveryStatus::Sent
    }

    /// `true` when an error was encountered and the attempt should be retried.
    pub fn is_retryable(&self) -> bool {
        *self == DeliveryStatus::Failed
    }
}

impl std::fmt::Display for DeliveryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            DeliveryStatus::Pending => "pending",
            DeliveryStatus::Sent => "sent",
            DeliveryStatus::Failed => "failed",
            DeliveryStatus::Skipped => "skipped",
        };
        write!(f, "{s}")
    }
}

/// A record of one delivery attempt for one notification on one channel.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeliveryRecord {
    /// Unique record ID.
    pub id: Uuid,

    /// The notification this record belongs to.
    pub notification_id: Uuid,

    /// Target user.
    pub user_id: Uuid,

    /// Channel attempted.
    pub channel: NotificationChannel,

    /// Delivery outcome.
    pub status: DeliveryStatus,

    /// Human-readable error message (set when `status == Failed`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,

    /// When the attempt was made.
    pub attempted_at: DateTime<Utc>,

    /// When delivery was confirmed (set when `status == Sent`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivered_at: Option<DateTime<Utc>>,
}

impl DeliveryRecord {
    /// Create a new pending record.
    pub fn pending(notification_id: Uuid, user_id: Uuid, channel: NotificationChannel) -> Self {
        Self {
            id: Uuid::new_v4(),
            notification_id,
            user_id,
            channel,
            status: DeliveryStatus::Pending,
            error_message: None,
            attempted_at: Utc::now(),
            delivered_at: None,
        }
    }

    /// Transition to Sent.
    #[must_use]
    pub fn into_sent(mut self) -> Self {
        self.status = DeliveryStatus::Sent;
        self.delivered_at = Some(Utc::now());
        self
    }

    /// Transition to Failed with an error message.
    #[must_use]
    pub fn into_failed(mut self, error: impl Into<String>) -> Self {
        self.status = DeliveryStatus::Failed;
        self.error_message = Some(error.into());
        self
    }

    /// Transition to Skipped (preference disabled).
    #[must_use]
    pub fn into_skipped(mut self) -> Self {
        self.status = DeliveryStatus::Skipped;
        self
    }
}

// ============================================================================
// Story 2b-1 — Channel infrastructure: DeliveryRequest
// ============================================================================

/// A fully-resolved delivery request: one `Notification` destined for one user
/// on one channel.
#[derive(Debug, Clone)]
pub struct DeliveryRequest {
    /// The notification to deliver.
    pub notification: Notification,
    /// Target channel.
    pub channel: NotificationChannel,
    /// Caller-supplied context (announcement ID, vote ID, etc.).
    pub entity_id: Option<Uuid>,
}

// ============================================================================
// Story 2b-4 — Transport adapter traits
// ============================================================================

/// Result type for transport adapters.
pub type TransportResult = Result<(), NotificationError>;

/// Transport adapter for email notifications.
///
/// Implementors may use SMTP (`LetterTransportAdapter`), an external API
/// (SendGrid, SES), or a test double.
#[async_trait::async_trait]
pub trait EmailTransport: Send + Sync {
    /// Deliver a notification via email to the given address.
    async fn send(
        &self,
        to_email: &str,
        to_name: &str,
        notification: &Notification,
        locale: &str,
    ) -> TransportResult;
}

/// Transport adapter for push notifications (FCM / APNs).
///
/// The production implementation wraps Firebase Cloud Messaging.  The stub
/// ships with this Epic; a real adapter can be added in a follow-up without
/// changing the pipeline.
#[async_trait::async_trait]
pub trait PushTransport: Send + Sync {
    /// Send a push notification to every registered device token for the user.
    ///
    /// Implementations look up the user's device tokens themselves (once, via the
    /// service-role pool) and route each to its provider. There is intentionally no
    /// `device_tokens` parameter: the pipeline does not pre-fetch tokens, and a
    /// vestigial slice only invites callers to populate it and expect it to matter.
    async fn send(&self, user_id: Uuid, notification: &Notification) -> TransportResult;
}

/// Transport adapter for in-app (stored) notifications.
///
/// Implementations persist the notification and optionally fan it out over a
/// WebSocket channel so connected clients see it in real time.
#[async_trait::async_trait]
pub trait InAppTransport: Send + Sync {
    /// Persist the notification for the user and emit a WebSocket event if
    /// a WebSocket channel is available.
    async fn send(
        &self,
        user_id: Uuid,
        notification: &Notification,
        entity_id: Option<Uuid>,
    ) -> TransportResult;
}

// ============================================================================
// Story 2b-2 — Preference routing
// ============================================================================

/// Resolved routing decision for one (user, notification) pair.
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    /// Channels that the preference router has approved for delivery.
    pub enabled_channels: Vec<NotificationChannel>,
    /// Channels that were requested but disabled by user preference.
    pub skipped_channels: Vec<NotificationChannel>,
}

impl RoutingDecision {
    /// Build a routing decision where all requested channels are enabled.
    pub fn all_enabled(channels: &[NotificationChannel]) -> Self {
        Self {
            enabled_channels: channels.to_vec(),
            skipped_channels: Vec::new(),
        }
    }

    /// `true` when there is at least one channel to deliver on.
    pub fn has_delivery_work(&self) -> bool {
        !self.enabled_channels.is_empty()
    }
}

// ============================================================================
// Story 2b-5 — Pipeline result summary
// ============================================================================

/// Summary returned by `NotificationPipeline::dispatch`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct PipelineResult {
    /// How many channel deliveries succeeded.
    pub sent: usize,
    /// How many channel deliveries were skipped (preference off).
    pub skipped: usize,
    /// How many channel deliveries failed.
    pub failed: usize,
    /// Individual delivery records (one per channel attempt).
    pub records: Vec<DeliveryRecord>,
}

impl PipelineResult {
    /// `true` when no delivery was attempted (all skipped or all failed before dispatch).
    pub fn is_fully_skipped(&self) -> bool {
        self.sent == 0 && self.failed == 0
    }

    /// Total number of channels processed.
    pub fn total(&self) -> usize {
        self.sent + self.skipped + self.failed
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notifications::{NotificationCategory, NotificationPriority};

    fn make_notification(user_id: Uuid) -> Notification {
        Notification::new(user_id, NotificationCategory::Announcements, "Test", "Body")
    }

    #[test]
    fn delivery_record_transitions() {
        let uid = Uuid::new_v4();
        let nid = Uuid::new_v4();

        let rec = DeliveryRecord::pending(nid, uid, NotificationChannel::Email);
        assert_eq!(rec.status, DeliveryStatus::Pending);

        let rec = rec.into_sent();
        assert!(rec.status.is_sent());
        assert!(rec.delivered_at.is_some());

        let rec = DeliveryRecord::pending(nid, uid, NotificationChannel::Push)
            .into_failed("FCM unreachable");
        assert!(rec.status.is_retryable());
        assert_eq!(rec.error_message.as_deref(), Some("FCM unreachable"));

        let rec = DeliveryRecord::pending(nid, uid, NotificationChannel::InApp).into_skipped();
        assert_eq!(rec.status, DeliveryStatus::Skipped);
    }

    #[test]
    fn routing_decision_helpers() {
        let dec =
            RoutingDecision::all_enabled(&[NotificationChannel::Email, NotificationChannel::Push]);
        assert!(dec.has_delivery_work());
        assert_eq!(dec.enabled_channels.len(), 2);
        assert!(dec.skipped_channels.is_empty());

        let empty = RoutingDecision {
            enabled_channels: vec![],
            skipped_channels: vec![NotificationChannel::Email],
        };
        assert!(!empty.has_delivery_work());
    }

    #[test]
    fn pipeline_result_counters() {
        let uid = Uuid::new_v4();
        let nid = Uuid::new_v4();
        let mut result = PipelineResult::default();
        result.sent += 1;
        result.skipped += 1;
        result
            .records
            .push(DeliveryRecord::pending(nid, uid, NotificationChannel::Email).into_sent());
        result
            .records
            .push(DeliveryRecord::pending(nid, uid, NotificationChannel::Push).into_skipped());

        assert_eq!(result.total(), 2);
        assert!(!result.is_fully_skipped());
    }

    #[test]
    fn notification_created_for_pipeline() {
        let user_id = Uuid::new_v4();
        let n = make_notification(user_id);
        assert_eq!(n.user_id, user_id);
        assert_eq!(n.priority, NotificationPriority::Normal);
    }
}
