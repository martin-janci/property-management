//! Notification service for scheduled notifications (Epic 106).
//!
//! Provides a unified abstraction for sending notifications across multiple channels:
//! - Push notifications (mobile)
//! - Email notifications
//! - In-app notifications
//!
//! The service respects user notification preferences and handles deduplication.

use chrono::{DateTime, Duration, Utc};
use db::models::notification_preference::NotificationChannel as DbNotificationChannel;
use db::models::{Announcement, Locale, Vote, VoteResults};
use db::repositories::{
    GranularNotificationRepository, NotificationPreferenceRepository, UserRepository,
};
use db::DbPool;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

use common::notifications::pipeline::PushTransport;
use common::notifications::{Notification as PushNotification, NotificationCategory};

use super::email::EmailService;
use super::push::CombinedPushAdapter;

/// Notification service errors.
#[derive(Debug, Error)]
pub enum NotificationError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Email error: {0}")]
    Email(String),

    #[error("User not found: {0}")]
    UserNotFound(Uuid),

    #[error("Invalid notification type")]
    InvalidType,
}

/// Types of notification events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NotificationType {
    // Announcement notifications (Story 106.1)
    AnnouncementPublished,
    AnnouncementAcknowledgmentRequired,

    // Vote notifications (Story 106.2)
    VoteStarted,
    VoteEndingSoon,
    VoteClosed,
    VoteResultsAvailable,

    // Reminder notifications (Story 106.3)
    MeterReadingDue,
    PaymentDue,
    PaymentOverdue,
    VoteReminderNotVoted,

    // Session notifications (Story 106.4)
    SessionExpiringSoon,

    // Workflow automation notifications (Epic 94): emitted by the
    // `send_notification` workflow action via `NotificationServiceSink`.
    WorkflowAction,
}

impl NotificationType {
    /// Get the event type string for granular notification preferences.
    pub fn as_event_type(&self) -> &'static str {
        match self {
            NotificationType::AnnouncementPublished => "announcement.published",
            NotificationType::AnnouncementAcknowledgmentRequired => {
                "announcement.acknowledgment_required"
            }
            NotificationType::VoteStarted => "vote.started",
            NotificationType::VoteEndingSoon => "vote.ending_soon",
            NotificationType::VoteClosed => "vote.closed",
            NotificationType::VoteResultsAvailable => "vote.results_available",
            NotificationType::MeterReadingDue => "meter.reading_due",
            NotificationType::PaymentDue => "payment.due",
            NotificationType::PaymentOverdue => "payment.overdue",
            NotificationType::VoteReminderNotVoted => "vote.reminder_not_voted",
            NotificationType::SessionExpiringSoon => "session.expiring_soon",
            NotificationType::WorkflowAction => "workflow.action",
        }
    }
}

/// Notification channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationChannel {
    Push,
    Email,
    InApp,
}

/// A notification to be sent.
#[derive(Debug, Clone)]
pub struct Notification {
    pub notification_type: NotificationType,
    pub title: String,
    pub body: String,
    pub data: serde_json::Value,
    pub channels: Vec<NotificationChannel>,
}

/// Notification service configuration.
#[derive(Clone)]
pub struct NotificationServiceConfig {
    /// Whether notifications are enabled.
    pub enabled: bool,
    /// Deduplication window in seconds (default: 5 minutes).
    pub dedup_window_secs: i64,
}

impl Default for NotificationServiceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dedup_window_secs: 300, // 5 minutes
        }
    }
}

/// Deduplication key for notifications.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DedupKey {
    user_id: Uuid,
    notification_type: NotificationType,
    entity_id: Option<Uuid>,
}

/// Entry in the deduplication cache.
struct DedupEntry {
    sent_at: DateTime<Utc>,
}

/// Atomically decide whether a notification identified by `key` should be sent,
/// recording the send stamp in the *same* critical section when it should.
///
/// Returns `true` when a send for this key already happened within the dedup
/// window and the caller must NOT send (duplicate). Returns `false` when the
/// caller should proceed.
///
/// The invariant that makes this race-free is that the stamp is written *before*
/// the function reports "proceed", so a concurrent caller for the same key
/// observes it and is deduplicated instead of producing a second delivery.
/// Evicts stale entries (older than twice the window) while it holds the map.
fn claim_send_in_cache(
    cache: &mut std::collections::HashMap<DedupKey, DedupEntry>,
    key: DedupKey,
    window_secs: i64,
    now: DateTime<Utc>,
) -> bool {
    let cutoff = now - Duration::seconds(window_secs * 2);
    cache.retain(|_, v| v.sent_at > cutoff);

    if let Some(entry) = cache.get(&key) {
        if now - entry.sent_at < Duration::seconds(window_secs) {
            return true;
        }
    }

    // Stamp the send *before* reporting "proceed" so a concurrent caller for the
    // same key is deduplicated instead of producing a second delivery. Recording
    // it only after the send (the previous flow) left a window in which two
    // callers both passed the check and both delivered.
    cache.insert(key, DedupEntry { sent_at: now });
    false
}

/// Derive the stable `(entity_type, entity_id)` grouping pair for an in-app
/// notification written to `notification_groups` / `grouped_notifications`.
///
/// The group key the DB function builds is `entity_type || '_' || entity_id`,
/// and `(user_id, group_key)` is unique — so the pair returned here is what
/// decides whether two notifications collapse into one group or each land in
/// their own.
///
/// * When the notification refers to a concrete entity (a fault, a vote, …)
///   `entity_id` is `Some(id)`: that id groups every notification about the
///   same entity together under the generic `"notification"` entity type.
/// * When there is **no** concrete entity, the notification must still group
///   deterministically. Using a fresh random id as the fallback dropped every
///   entity-less notification into its own singleton group, defeating grouping
///   entirely. Instead we key the group on the notification's *event type*
///   (with a fixed nil `entity_id`), so repeated entity-less notifications of
///   the same type collapse into a single, stable group.
fn in_app_group_key(event_type: &str, entity_id: Option<Uuid>) -> (&str, Uuid) {
    match entity_id {
        Some(id) => ("notification", id),
        // No concrete entity: key the group on the event type with a fixed nil
        // id so repeated entity-less notifications of the same type collapse
        // into one stable group instead of each getting a fresh random group.
        None => (event_type, Uuid::nil()),
    }
}

/// Notification service for sending notifications across channels.
pub struct NotificationService {
    pool: DbPool,
    email_service: EmailService,
    user_repo: UserRepository,
    notification_pref_repo: NotificationPreferenceRepository,
    granular_notification_repo: GranularNotificationRepository,
    config: NotificationServiceConfig,
    /// FCM/APNs delivery. Defaults to the env-configured combined adapter,
    /// which reports the channel as skipped when no provider is configured.
    push_transport: Arc<dyn PushTransport>,
    /// Deduplication cache to prevent sending duplicate notifications.
    dedup_cache: Arc<RwLock<std::collections::HashMap<DedupKey, DedupEntry>>>,
}

impl NotificationService {
    /// Create a new notification service.
    pub fn new(
        pool: DbPool,
        email_service: EmailService,
        config: NotificationServiceConfig,
    ) -> Self {
        Self {
            user_repo: UserRepository::new(pool.clone()),
            notification_pref_repo: NotificationPreferenceRepository::new(pool.clone()),
            granular_notification_repo: GranularNotificationRepository::new(pool.clone()),
            push_transport: Arc::new(CombinedPushAdapter::from_env(pool.clone())),
            pool,
            email_service,
            config,
            dedup_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Replace the push transport (tests, or a shared pre-built adapter).
    pub fn with_push_transport(mut self, push_transport: Arc<dyn PushTransport>) -> Self {
        self.push_transport = push_transport;
        self
    }

    /// Create a development notification service (logs instead of sending).
    pub fn development(pool: DbPool) -> Self {
        Self::new(
            pool,
            EmailService::development(),
            NotificationServiceConfig {
                enabled: false,
                ..Default::default()
            },
        )
    }

    /// Atomically claim this `(user, type, entity)` for sending.
    ///
    /// Returns `true` when a notification for this key was already sent within
    /// the dedup window (the caller MUST skip delivery). Returns `false` after
    /// recording the send stamp, so the caller may proceed and any concurrent
    /// caller for the same key is deduplicated.
    ///
    /// The check and the stamp happen under a single write lock, so the record
    /// is made *before* the send. This closes the duplicate-send race in which
    /// two callers both passed a read-only check and then both delivered because
    /// the stamp was only written after the send.
    async fn claim_for_send(
        &self,
        user_id: Uuid,
        notification_type: NotificationType,
        entity_id: Option<Uuid>,
    ) -> bool {
        let key = DedupKey {
            user_id,
            notification_type,
            entity_id,
        };

        let mut cache = self.dedup_cache.write().await;
        claim_send_in_cache(&mut cache, key, self.config.dedup_window_secs, Utc::now())
    }

    /// Check if a user has enabled a specific notification channel for an event type.
    async fn is_channel_enabled(
        &self,
        user_id: Uuid,
        event_type: &str,
        channel: NotificationChannel,
    ) -> Result<bool, NotificationError> {
        // First check granular preferences for this specific event type
        if let Some(pref) = self
            .granular_notification_repo
            .get_user_event_preference(user_id, event_type)
            .await?
        {
            return Ok(match channel {
                NotificationChannel::Push => pref.push_enabled,
                NotificationChannel::Email => pref.email_enabled,
                NotificationChannel::InApp => pref.in_app_enabled,
            });
        }

        // Fall back to channel-level preferences
        let db_channel = match channel {
            NotificationChannel::Push => DbNotificationChannel::Push,
            NotificationChannel::Email => DbNotificationChannel::Email,
            NotificationChannel::InApp => DbNotificationChannel::InApp,
        };

        // Note: NotificationService is used in background tasks without user RLS context.
        // These are system-level lookups for notification preferences.
        #[allow(deprecated)]
        if let Some(pref) = self
            .notification_pref_repo
            .get_by_user_and_channel(user_id, db_channel)
            .await?
        {
            return Ok(pref.enabled);
        }

        // Default to enabled
        Ok(true)
    }

    /// Deliver a notification on the Push channel through the configured
    /// FCM/APNs transport.
    ///
    /// Extracted from `send_to_user`'s per-channel match so the "Push dispatches
    /// through the transport — it is not a log-only no-op" contract is
    /// unit-testable with a spy transport and without a live database (see
    /// `push_channel_dispatches_through_transport`).
    ///
    /// This arm used to only log ("Push notification (integration pending)"), so
    /// scheduled events (vote started/closed, announcement published, meter and
    /// payment reminders) that requested Push never reached a device while the
    /// service still reported success. A delivery error is logged and swallowed
    /// so it does not abort the remaining channels; `PushNotConfigured` (no
    /// provider, or the user has no registered device) is an expected skip the
    /// adapter already logged.
    async fn deliver_push_channel(
        &self,
        user_id: Uuid,
        notification: &Notification,
        entity_id: Option<Uuid>,
    ) {
        let push = to_push_notification(user_id, notification, entity_id);
        match self.push_transport.send(user_id, &push).await {
            Ok(()) => {}
            Err(common::notifications::NotificationError::PushNotConfigured) => {}
            Err(e) => {
                tracing::warn!(
                    user_id = %user_id,
                    notification_type = ?notification.notification_type,
                    error = %e,
                    "Push notification not delivered"
                );
            }
        }
    }

    /// Send a notification to a single user.
    pub async fn send_to_user(
        &self,
        user_id: Uuid,
        notification: &Notification,
        entity_id: Option<Uuid>,
    ) -> Result<(), NotificationError> {
        if !self.config.enabled {
            tracing::debug!(
                user_id = %user_id,
                notification_type = ?notification.notification_type,
                "Notification skipped (disabled)"
            );
            return Ok(());
        }

        // Atomically claim this notification for sending BEFORE delivering it.
        // Claiming stamps the dedup record up front, so a concurrent caller for
        // the same (user, type, entity) is deduplicated instead of producing a
        // second delivery. A `true` return means it was already sent in-window.
        if self
            .claim_for_send(user_id, notification.notification_type, entity_id)
            .await
        {
            tracing::debug!(
                user_id = %user_id,
                notification_type = ?notification.notification_type,
                "Notification deduplicated"
            );
            return Ok(());
        }

        // Get user
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or(NotificationError::UserNotFound(user_id))?;

        let event_type = notification.notification_type.as_event_type();

        // Send via each enabled channel
        for channel in &notification.channels {
            if !self
                .is_channel_enabled(user_id, event_type, *channel)
                .await?
            {
                tracing::debug!(
                    user_id = %user_id,
                    channel = ?channel,
                    event_type = %event_type,
                    "Channel disabled for user"
                );
                continue;
            }

            match channel {
                NotificationChannel::Email => {
                    let locale = match user.locale.as_str() {
                        "sk" => Locale::Slovak,
                        "cs" => Locale::Czech,
                        "de" => Locale::German,
                        _ => Locale::English,
                    };

                    if let Err(e) = self
                        .email_service
                        .send_notification_email(
                            &user.email,
                            &user.name,
                            &notification.title,
                            &notification.body,
                            &locale,
                        )
                        .await
                    {
                        tracing::error!(
                            user_id = %user_id,
                            error = %e,
                            "Failed to send email notification"
                        );
                    }
                }
                NotificationChannel::Push => {
                    self.deliver_push_channel(user_id, notification, entity_id)
                        .await;
                }
                NotificationChannel::InApp => {
                    // Create in-app notification using granular notification repository.
                    // Derive a stable grouping key so related notifications group
                    // together instead of each landing in its own group.
                    let (group_entity_type, group_entity_id) =
                        in_app_group_key(event_type, entity_id);
                    if let Err(e) = self
                        .granular_notification_repo
                        .add_notification_to_group_for_user(
                            user_id,
                            group_entity_type,
                            group_entity_id,
                            &notification.title,
                            event_type,
                            &notification.title,
                            Some(&notification.body),
                            Some(notification.data.clone()),
                            None,
                            None,
                        )
                        .await
                    {
                        tracing::error!(
                            user_id = %user_id,
                            error = %e,
                            "Failed to create in-app notification"
                        );
                    }
                }
            }
        }

        // The dedup record was already stamped by `claim_for_send` above, before
        // delivery, so there is nothing to record here.
        tracing::info!(
            user_id = %user_id,
            notification_type = ?notification.notification_type,
            "Notification sent"
        );

        Ok(())
    }

    /// Send a notification to multiple users.
    pub async fn send_to_users(
        &self,
        user_ids: &[Uuid],
        notification: &Notification,
        entity_id: Option<Uuid>,
    ) -> Result<usize, NotificationError> {
        let mut sent_count = 0;

        for user_id in user_ids {
            match self.send_to_user(*user_id, notification, entity_id).await {
                Ok(()) => sent_count += 1,
                Err(e) => {
                    tracing::warn!(
                        user_id = %user_id,
                        error = %e,
                        "Failed to send notification to user"
                    );
                }
            }
        }

        Ok(sent_count)
    }

    // ========================================================================
    // Announcement Notifications (Story 106.1)
    // ========================================================================

    /// Send notification when an announcement is published.
    pub async fn notify_announcement_published(
        &self,
        announcement: &Announcement,
        target_user_ids: &[Uuid],
    ) -> Result<usize, NotificationError> {
        let notification = Notification {
            notification_type: if announcement.acknowledgment_required {
                NotificationType::AnnouncementAcknowledgmentRequired
            } else {
                NotificationType::AnnouncementPublished
            },
            title: format!("New Announcement: {}", announcement.title),
            body: if announcement.acknowledgment_required {
                format!(
                    "{} - Please acknowledge this announcement.",
                    truncate_text(&announcement.content, 100)
                )
            } else {
                truncate_text(&announcement.content, 150)
            },
            data: serde_json::json!({
                "announcement_id": announcement.id,
                "organization_id": announcement.organization_id,
                "acknowledgment_required": announcement.acknowledgment_required,
            }),
            channels: vec![
                NotificationChannel::InApp,
                NotificationChannel::Email,
                NotificationChannel::Push,
            ],
        };

        self.send_to_users(target_user_ids, &notification, Some(announcement.id))
            .await
    }

    // ========================================================================
    // Vote Notifications (Story 106.2)
    // ========================================================================

    /// Send notification when a vote starts.
    pub async fn notify_vote_started(
        &self,
        vote: &Vote,
        eligible_user_ids: &[Uuid],
    ) -> Result<usize, NotificationError> {
        let notification = Notification {
            notification_type: NotificationType::VoteStarted,
            title: format!("New Vote: {}", vote.title),
            body: format!(
                "A new vote has started: \"{}\". Deadline: {}",
                vote.title,
                vote.end_at.format("%Y-%m-%d %H:%M")
            ),
            data: serde_json::json!({
                "vote_id": vote.id,
                "organization_id": vote.organization_id,
                "building_id": vote.building_id,
                "end_at": vote.end_at.to_rfc3339(),
            }),
            channels: vec![
                NotificationChannel::InApp,
                NotificationChannel::Email,
                NotificationChannel::Push,
            ],
        };

        self.send_to_users(eligible_user_ids, &notification, Some(vote.id))
            .await
    }

    /// Send notification when a vote is ending soon.
    pub async fn notify_vote_ending_soon(
        &self,
        vote: &Vote,
        user_ids_not_voted: &[Uuid],
    ) -> Result<usize, NotificationError> {
        let notification = Notification {
            notification_type: NotificationType::VoteEndingSoon,
            title: format!("Vote Ending Soon: {}", vote.title),
            body: format!(
                "The vote \"{}\" ends at {}. Don't forget to cast your vote!",
                vote.title,
                vote.end_at.format("%Y-%m-%d %H:%M")
            ),
            data: serde_json::json!({
                "vote_id": vote.id,
                "end_at": vote.end_at.to_rfc3339(),
            }),
            channels: vec![NotificationChannel::InApp, NotificationChannel::Push],
        };

        self.send_to_users(user_ids_not_voted, &notification, Some(vote.id))
            .await
    }

    /// Send notification when a vote is closed with results.
    pub async fn notify_vote_closed(
        &self,
        vote: &Vote,
        results: &VoteResults,
        participant_ids: &[Uuid],
    ) -> Result<usize, NotificationError> {
        let quorum_text = if results.quorum_met {
            "Quorum was met."
        } else {
            "Quorum was not met."
        };

        let notification = Notification {
            notification_type: NotificationType::VoteClosed,
            title: format!("Vote Closed: {}", vote.title),
            body: format!(
                "The vote \"{}\" has closed. {} Participation: {:.1}%",
                vote.title, quorum_text, results.participation_rate
            ),
            data: serde_json::json!({
                "vote_id": vote.id,
                "quorum_met": results.quorum_met,
                "participation_rate": results.participation_rate,
                "participation_count": results.participation_count,
            }),
            channels: vec![NotificationChannel::InApp, NotificationChannel::Email],
        };

        self.send_to_users(participant_ids, &notification, Some(vote.id))
            .await
    }

    // ========================================================================
    // Reminder Notifications (Story 106.3)
    // ========================================================================

    /// Send meter reading reminder.
    pub async fn notify_meter_reading_due(
        &self,
        user_id: Uuid,
        meter_id: Uuid,
        meter_number: &str,
        due_date: DateTime<Utc>,
    ) -> Result<(), NotificationError> {
        let notification = Notification {
            notification_type: NotificationType::MeterReadingDue,
            title: "Meter Reading Due".to_string(),
            body: format!(
                "Please submit your meter reading for meter {} by {}.",
                meter_number,
                due_date.format("%Y-%m-%d")
            ),
            data: serde_json::json!({
                "meter_id": meter_id,
                "meter_number": meter_number,
                "due_date": due_date.to_rfc3339(),
            }),
            channels: vec![NotificationChannel::InApp, NotificationChannel::Push],
        };

        self.send_to_user(user_id, &notification, Some(meter_id))
            .await
    }

    /// Send payment due reminder.
    pub async fn notify_payment_due(
        &self,
        user_id: Uuid,
        invoice_id: Uuid,
        amount: &str,
        due_date: DateTime<Utc>,
        is_overdue: bool,
    ) -> Result<(), NotificationError> {
        let notification_type = if is_overdue {
            NotificationType::PaymentOverdue
        } else {
            NotificationType::PaymentDue
        };

        let (title, body) = if is_overdue {
            (
                "Payment Overdue".to_string(),
                format!(
                    "Your payment of {} was due on {}. Please pay immediately to avoid late fees.",
                    amount,
                    due_date.format("%Y-%m-%d")
                ),
            )
        } else {
            (
                "Payment Reminder".to_string(),
                format!(
                    "Your payment of {} is due on {}.",
                    amount,
                    due_date.format("%Y-%m-%d")
                ),
            )
        };

        let notification = Notification {
            notification_type,
            title,
            body,
            data: serde_json::json!({
                "invoice_id": invoice_id,
                "amount": amount,
                "due_date": due_date.to_rfc3339(),
                "is_overdue": is_overdue,
            }),
            channels: vec![
                NotificationChannel::InApp,
                NotificationChannel::Email,
                NotificationChannel::Push,
            ],
        };

        self.send_to_user(user_id, &notification, Some(invoice_id))
            .await
    }

    /// Send vote reminder for users who haven't voted yet.
    pub async fn notify_vote_reminder(
        &self,
        vote: &Vote,
        user_ids_not_voted: &[Uuid],
    ) -> Result<usize, NotificationError> {
        let notification = Notification {
            notification_type: NotificationType::VoteReminderNotVoted,
            title: format!("Reminder: Vote on \"{}\"", vote.title),
            body: format!(
                "You haven't voted yet on \"{}\". The vote ends {}.",
                vote.title,
                vote.end_at.format("%Y-%m-%d %H:%M")
            ),
            data: serde_json::json!({
                "vote_id": vote.id,
                "end_at": vote.end_at.to_rfc3339(),
            }),
            channels: vec![NotificationChannel::InApp, NotificationChannel::Push],
        };

        self.send_to_users(user_ids_not_voted, &notification, Some(vote.id))
            .await
    }

    // ========================================================================
    // Cleanup and Maintenance
    // ========================================================================

    /// Clear the deduplication cache.
    pub async fn clear_dedup_cache(&self) {
        let mut cache = self.dedup_cache.write().await;
        cache.clear();
    }

    /// Get the current size of the deduplication cache.
    pub async fn dedup_cache_size(&self) -> usize {
        let cache = self.dedup_cache.read().await;
        cache.len()
    }
}

/// Helper function to truncate text to a maximum length.
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        let mut truncated = text.chars().take(max_len - 3).collect::<String>();
        truncated.push_str("...");
        truncated
    }
}

/// Metrics for the notification service.
#[derive(Debug, Clone, Default, Serialize)]
pub struct NotificationMetrics {
    pub total_sent: u64,
    pub email_sent: u64,
    pub push_sent: u64,
    pub in_app_sent: u64,
    pub deduplicated: u64,
    pub failed: u64,
}

/// Category the push payload is filed under, by notification type.
fn push_category(notification_type: NotificationType) -> NotificationCategory {
    match notification_type {
        NotificationType::AnnouncementPublished
        | NotificationType::AnnouncementAcknowledgmentRequired => {
            NotificationCategory::Announcements
        }
        NotificationType::VoteStarted
        | NotificationType::VoteEndingSoon
        | NotificationType::VoteClosed
        | NotificationType::VoteResultsAvailable
        | NotificationType::VoteReminderNotVoted => NotificationCategory::Votes,
        NotificationType::PaymentDue | NotificationType::PaymentOverdue => {
            NotificationCategory::Financial
        }
        NotificationType::MeterReadingDue
        | NotificationType::SessionExpiringSoon
        | NotificationType::WorkflowAction => NotificationCategory::System,
    }
}

/// Entity kind the mobile app routes a tap on (`data.type`), when the
/// notification points at one. The app maps it to a list or detail screen.
fn push_entity_type(notification_type: NotificationType) -> Option<&'static str> {
    match push_category(notification_type) {
        NotificationCategory::Announcements => Some("announcement"),
        NotificationCategory::Votes => Some("vote"),
        _ => None,
    }
}

/// Build the transport-level push payload from a service notification. The
/// event type and entity id ride along in `data`, together with the
/// `type` / `id` pair the mobile app reads to open the matching screen.
fn to_push_notification(
    user_id: Uuid,
    notification: &Notification,
    entity_id: Option<Uuid>,
) -> PushNotification {
    let mut push = PushNotification::new(
        user_id,
        push_category(notification.notification_type),
        notification.title.clone(),
        notification.body.clone(),
    );
    let mut data = match &notification.data {
        serde_json::Value::Object(map) => map.clone(),
        _ => serde_json::Map::new(),
    };
    data.insert(
        "event_type".to_string(),
        serde_json::Value::String(notification.notification_type.as_event_type().to_string()),
    );
    let entity_type = push_entity_type(notification.notification_type);
    if let Some(entity_type) = entity_type {
        data.insert(
            "type".to_string(),
            serde_json::Value::String(entity_type.to_string()),
        );
    }
    if let Some(id) = entity_id {
        data.insert(
            "entity_id".to_string(),
            serde_json::Value::String(id.to_string()),
        );
        if entity_type.is_some() {
            data.insert("id".to_string(), serde_json::Value::String(id.to_string()));
        }
    }
    push.data = Some(serde_json::Value::Object(data));
    push
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Spy push transport that records every `send` call so a test can assert
    /// the Push channel actually dispatched through it.
    #[derive(Default)]
    struct RecordingPushTransport {
        calls: Mutex<Vec<(Uuid, PushNotification)>>,
    }

    #[async_trait::async_trait]
    impl PushTransport for RecordingPushTransport {
        async fn send(
            &self,
            user_id: Uuid,
            notification: &PushNotification,
        ) -> common::notifications::TransportResult {
            self.calls
                .lock()
                .unwrap()
                .push((user_id, notification.clone()));
            Ok(())
        }
    }

    /// A lazy pool pointed at an unreachable address — `deliver_push_channel`
    /// never queries the DB, so this is never connected to.
    fn unused_pool() -> DbPool {
        sqlx::PgPool::connect_lazy("postgres://never-used:never@127.0.0.1:1/never")
            .expect("connect_lazy builds without connecting")
    }

    /// Regression (`code-review-api-core-notif-push-channel-noop`): the Push
    /// channel must DISPATCH through the configured push transport, not silently
    /// "succeed" without delivering. The arm was once a log-only no-op ("Push
    /// notification (integration pending)"), so every push a user explicitly
    /// opted into (vote reminders, payment-due, announcements) was dropped while
    /// `send_to_user` still returned `Ok(())` and logged "Notification sent".
    ///
    /// This drives the exact delegation `send_to_user`'s Push arm performs,
    /// through a spy transport and with no live database, and pins that the push
    /// payload reaches the transport with the right recipient, category and
    /// routing data — so a revert to the silent no-op fails here.
    #[tokio::test]
    async fn push_channel_dispatches_through_transport() {
        let spy = Arc::new(RecordingPushTransport::default());
        let service = NotificationService::new(
            unused_pool(),
            EmailService::development(),
            NotificationServiceConfig::default(),
        )
        .with_push_transport(spy.clone());

        let user_id = Uuid::new_v4();
        let vote_id = Uuid::new_v4();
        let notification = Notification {
            notification_type: NotificationType::VoteReminderNotVoted,
            title: "Reminder: Vote".to_string(),
            body: "Cast your vote".to_string(),
            data: serde_json::json!({ "vote_id": vote_id }),
            channels: vec![NotificationChannel::Push],
        };

        service
            .deliver_push_channel(user_id, &notification, Some(vote_id))
            .await;

        let calls = spy.calls.lock().unwrap();
        assert_eq!(
            calls.len(),
            1,
            "Push channel must dispatch exactly one push through the transport, \
             not silently drop it"
        );
        assert_eq!(calls[0].0, user_id, "push must target the intended user");
        let push = &calls[0].1;
        assert_eq!(push.title, "Reminder: Vote");
        assert_eq!(push.category, NotificationCategory::Votes);
        let data = push.data.as_ref().expect("push routing data payload");
        assert_eq!(data["event_type"], "vote.reminder_not_voted");
        assert_eq!(data["entity_id"], vote_id.to_string());
    }

    #[test]
    fn push_payload_carries_title_body_category_and_routing_data() {
        let user_id = Uuid::new_v4();
        let vote_id = Uuid::new_v4();
        let notification = Notification {
            notification_type: NotificationType::VoteStarted,
            title: "New vote".to_string(),
            body: "Roof repair".to_string(),
            data: serde_json::json!({ "vote_id": vote_id }),
            channels: vec![NotificationChannel::Push],
        };

        let push = to_push_notification(user_id, &notification, Some(vote_id));

        assert_eq!(push.user_id, user_id);
        assert_eq!(push.title, "New vote");
        assert_eq!(push.body, "Roof repair");
        assert_eq!(push.category, NotificationCategory::Votes);
        let data = push.data.expect("data payload");
        assert_eq!(data["event_type"], "vote.started");
        assert_eq!(data["entity_id"], vote_id.to_string());
        assert_eq!(data["vote_id"], vote_id.to_string());
        // The pair the mobile app reads to open the vote on tap.
        assert_eq!(data["type"], "vote");
        assert_eq!(data["id"], vote_id.to_string());
    }

    #[test]
    fn push_category_covers_every_reminder_type() {
        assert_eq!(
            push_category(NotificationType::AnnouncementPublished),
            NotificationCategory::Announcements
        );
        assert_eq!(
            push_category(NotificationType::PaymentOverdue),
            NotificationCategory::Financial
        );
        assert_eq!(
            push_category(NotificationType::VoteReminderNotVoted),
            NotificationCategory::Votes
        );
    }

    #[test]
    fn test_notification_type_event_type() {
        assert_eq!(
            NotificationType::AnnouncementPublished.as_event_type(),
            "announcement.published"
        );
        assert_eq!(NotificationType::VoteClosed.as_event_type(), "vote.closed");
        assert_eq!(
            NotificationType::MeterReadingDue.as_event_type(),
            "meter.reading_due"
        );
    }

    #[test]
    fn test_truncate_text() {
        assert_eq!(truncate_text("Hello", 10), "Hello");
        assert_eq!(truncate_text("Hello World", 10), "Hello W...");
        assert_eq!(truncate_text("Hi", 10), "Hi");
    }

    fn dedup_key() -> DedupKey {
        DedupKey {
            user_id: Uuid::new_v4(),
            notification_type: NotificationType::AnnouncementPublished,
            entity_id: Some(Uuid::new_v4()),
        }
    }

    // Regression: the send path must record the send stamp *before* delivering,
    // so two callers racing on the same (user, type, entity) result in exactly
    // one notification. Recording the stamp only after the send (the old flow)
    // let both callers pass the dedup check and double-notify.
    #[test]
    fn claim_send_dedups_concurrent_callers() {
        let mut cache = std::collections::HashMap::new();
        let key = dedup_key();
        let now = Utc::now();

        // Caller A claims the send and proceeds to deliver.
        let a_duplicate = claim_send_in_cache(&mut cache, key.clone(), 300, now);
        assert!(!a_duplicate, "first caller must be allowed to send");

        // Caller B races in before A has finished delivering: it must be
        // deduplicated, because A's claim already stamped the record.
        let b_duplicate = claim_send_in_cache(&mut cache, key, 300, now);
        assert!(
            b_duplicate,
            "second concurrent caller must be deduplicated (no duplicate send)"
        );
    }

    // A stamp older than the dedup window must not suppress a fresh send.
    #[test]
    fn claim_send_allows_after_window() {
        let mut cache = std::collections::HashMap::new();
        let key = dedup_key();
        let now = Utc::now();

        assert!(!claim_send_in_cache(&mut cache, key.clone(), 300, now));
        // 10 minutes later — well past the 5-minute window.
        let later = now + Duration::seconds(600);
        assert!(
            !claim_send_in_cache(&mut cache, key, 300, later),
            "a send past the dedup window must be allowed again"
        );
    }

    /// Regression (`code-review-api-core-notification-random-group-key`): an
    /// in-app notification with no concrete entity must still group
    /// deterministically. The grouping key used a random `Uuid::new_v4()`
    /// fallback, so every entity-less notification of the same event type for a
    /// user produced a *different* `(entity_type, entity_id)` pair — i.e. a
    /// distinct group — defeating grouping entirely.
    ///
    /// Two entity-less notifications of the same event type must therefore
    /// resolve to the same grouping key.
    #[test]
    fn in_app_group_key_is_deterministic_for_entity_less_notifications() {
        let event_type = NotificationType::SessionExpiringSoon.as_event_type();

        let first = in_app_group_key(event_type, None);
        let second = in_app_group_key(event_type, None);

        assert_eq!(
            first, second,
            "entity-less notifications of the same event type must share one \
             stable grouping key, not a random per-notification one"
        );
    }

    /// Different event types must still fall into different groups, and a
    /// concrete entity id must group by that entity (not by event type).
    #[test]
    fn in_app_group_key_separates_event_types_and_honours_entity_id() {
        let vote = in_app_group_key(NotificationType::VoteStarted.as_event_type(), None);
        let payment = in_app_group_key(NotificationType::PaymentDue.as_event_type(), None);
        assert_ne!(
            vote, payment,
            "distinct event types must not collapse into the same group"
        );

        // A concrete entity groups by that entity under the generic type.
        let entity = Uuid::new_v4();
        let with_entity =
            in_app_group_key(NotificationType::VoteStarted.as_event_type(), Some(entity));
        assert_eq!(
            with_entity,
            ("notification", entity),
            "a concrete entity id must drive the group key"
        );
        // Same entity id → same key regardless of how many times we ask.
        assert_eq!(
            with_entity,
            in_app_group_key(NotificationType::VoteStarted.as_event_type(), Some(entity))
        );
    }
}
