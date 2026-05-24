//! Epic 2B — Notification Delivery Pipeline
//!
//! # Stories implemented
//! - **2b-1** Channel infrastructure: `NotificationPipeline` + `DeliveryRequest` dispatch
//! - **2b-2** Preference routing: `PreferenceRouter` checks per-user, per-channel settings
//! - **2b-3** Delivery tracking: in-memory + logging; ready to swap for a DB adapter
//! - **2b-4** Transport adapters: `SmtpEmailAdapter`, `FcmPushAdapter` (stub), `DbInAppAdapter`
//! - **2b-5** Pipeline integration: `NotificationPipeline::dispatch` / `dispatch_to_users`
//!
//! ## Architecture
//!
//! ```text
//!  Caller
//!    │
//!    ▼
//!  NotificationPipeline::dispatch(user_id, Notification, entity_id?)
//!    │
//!    ├─ PreferenceRouter::resolve(user_id, notification) ─► RoutingDecision
//!    │
//!    └─ For each enabled channel:
//!         ├─ email  ─► impl EmailTransport (SmtpEmailAdapter)
//!         ├─ push   ─► impl PushTransport  (FcmPushAdapter)
//!         └─ in_app ─► impl InAppTransport (DbInAppAdapter)
//!              │
//!              ▼
//!           DeliveryRecord (Sent / Failed / Skipped)
//!              │
//!              ▼
//!           PipelineResult (summary returned to caller)
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use db::{
    models::{Locale, User},
    repositories::{
        GranularNotificationRepository, NotificationPreferenceRepository, UserRepository,
    },
    DbPool,
};
use uuid::Uuid;

use common::notifications::{
    pipeline::{DeliveryRecord, DeliveryStatus, PipelineResult, RoutingDecision},
    EmailTransport, InAppTransport, Notification, NotificationChannel, NotificationError,
    PushTransport, TransportResult,
};

use super::email::EmailService;

// ============================================================================
// Story 2b-4a — Email transport adapter (SMTP via existing EmailService)
// ============================================================================

/// SMTP-backed email transport adapter.
///
/// Delegates to the existing `EmailService` so we don't duplicate SMTP
/// configuration.  Converts `EmailError` → `NotificationError::EmailFailed`.
#[derive(Clone)]
pub struct SmtpEmailAdapter {
    email_service: EmailService,
}

impl SmtpEmailAdapter {
    /// Create a new adapter wrapping the shared `EmailService`.
    pub fn new(email_service: EmailService) -> Self {
        Self { email_service }
    }

    /// Convert locale string to `Locale` enum.
    fn parse_locale(locale: &str) -> Locale {
        match locale {
            "sk" => Locale::Slovak,
            "cs" => Locale::Czech,
            "de" => Locale::German,
            _ => Locale::English,
        }
    }
}

#[async_trait]
impl EmailTransport for SmtpEmailAdapter {
    async fn send(
        &self,
        to_email: &str,
        to_name: &str,
        notification: &Notification,
        locale: &str,
    ) -> TransportResult {
        let locale_enum = Self::parse_locale(locale);
        self.email_service
            .send_notification_email(
                to_email,
                to_name,
                &notification.title,
                &notification.body,
                &locale_enum,
            )
            .await
            .map_err(|e| NotificationError::EmailFailed(e.to_string()))
    }
}

// ============================================================================
// Story 2b-4b — Push transport adapter (FCM stub)
// ============================================================================

/// FCM / APNs push notification adapter.
///
/// **Current status:** stub that logs intent; a real FCM HTTP v1 or APNs
/// implementation should replace `send_to_fcm` / `send_to_apns` in a
/// follow-up PR without changing the trait boundary.
#[derive(Clone, Default)]
pub struct FcmPushAdapter {
    /// FCM server key (None = disabled / not configured)
    fcm_server_key: Option<String>,
}

impl FcmPushAdapter {
    /// Create a new adapter.  Pass `None` to run in log-only (no real push) mode.
    pub fn new(fcm_server_key: Option<String>) -> Self {
        Self { fcm_server_key }
    }

    /// Load from environment variable `FCM_SERVER_KEY`.
    pub fn from_env() -> Self {
        Self::new(std::env::var("FCM_SERVER_KEY").ok())
    }

    /// Returns `true` when push delivery is enabled (FCM key is set).
    pub fn is_configured(&self) -> bool {
        self.fcm_server_key.is_some()
    }
}

#[async_trait]
impl PushTransport for FcmPushAdapter {
    async fn send(
        &self,
        user_id: Uuid,
        device_tokens: &[String],
        notification: &Notification,
    ) -> TransportResult {
        if !self.is_configured() {
            // Log-only mode: integration pending (FCM key not configured)
            tracing::info!(
                user_id = %user_id,
                token_count = device_tokens.len(),
                title = %notification.title,
                category = %notification.category,
                "[Epic 2B] Push notification (FCM not configured — log-only)"
            );
            return Ok(());
        }

        // TODO (follow-up): call FCM HTTP v1 API for each device_token
        // For now, log that we would send and return Ok.
        tracing::info!(
            user_id = %user_id,
            token_count = device_tokens.len(),
            title = %notification.title,
            "[Epic 2B] Push notification queued (FCM stub — real send pending)"
        );

        Ok(())
    }
}

// ============================================================================
// Story 2b-4c — In-app transport adapter (DB + optional WebSocket fanout)
// ============================================================================

/// In-app notification adapter backed by `GranularNotificationRepository`.
///
/// Persists the notification to `notification_groups` / `grouped_notifications`
/// via the existing `add_notification_to_group` DB function.  When a
/// `PubSubService` (Redis) is available, it also publishes an event on the
/// `notifications:{user_id}` channel so connected WebSocket clients receive
/// the notification in real time (gates 8A.3 sync).
#[derive(Clone)]
pub struct DbInAppAdapter {
    granular_repo: GranularNotificationRepository,
    /// Optional Redis pubsub for real-time fanout (Epic 2B / 8A.3).
    pubsub: Option<integrations::PubSubService>,
}

impl DbInAppAdapter {
    /// Create a new adapter.
    pub fn new(
        granular_repo: GranularNotificationRepository,
        pubsub: Option<integrations::PubSubService>,
    ) -> Self {
        Self {
            granular_repo,
            pubsub,
        }
    }
}

#[async_trait]
impl InAppTransport for DbInAppAdapter {
    async fn send(
        &self,
        user_id: Uuid,
        notification: &Notification,
        entity_id: Option<Uuid>,
    ) -> TransportResult {
        let eid = entity_id.unwrap_or(notification.id);
        let entity_type = notification.category.as_str().to_string();
        let event_type = format!("{}.notification", entity_type);

        self.granular_repo
            .add_notification_to_group(
                user_id,
                &entity_type,
                eid,
                &notification.title,
                &event_type,
                &notification.title,
                Some(&notification.body),
                notification.data.clone(),
                None,
                None,
            )
            .await
            .map_err(|e| NotificationError::InAppFailed(e.to_string()))?;

        // Publish real-time event when Redis pubsub is available (8A.3)
        if let Some(ref pubsub) = self.pubsub {
            let channel = format!("notifications:{user_id}");
            let msg = integrations::PubSubMessage {
                id: Uuid::new_v4(),
                event_type: "notification.created".to_string(),
                payload: serde_json::json!({
                    "notification_id": notification.id,
                    "category": notification.category,
                    "title": notification.title,
                    "entity_id": eid,
                }),
                source_instance: None,
            };
            if let Err(e) = pubsub.publish(&channel, msg).await {
                // Non-fatal: in-app was persisted; real-time push failed
                tracing::warn!(
                    user_id = %user_id,
                    channel = %channel,
                    error = %e,
                    "[Epic 2B] WebSocket pubsub publish failed (non-fatal)"
                );
            }
        }

        Ok(())
    }
}

// ============================================================================
// Story 2b-2 — Preference router
// ============================================================================

/// Resolves which notification channels are enabled for a given user.
///
/// Resolution order (highest wins):
/// 1. Per-event-type granular preference (from `event_notification_preferences`)
/// 2. Per-channel preference (from `notification_preferences`)
/// 3. Default: all channels enabled
#[derive(Clone)]
pub struct PreferenceRouter {
    notification_pref_repo: NotificationPreferenceRepository,
    granular_repo: GranularNotificationRepository,
}

impl PreferenceRouter {
    /// Create a new preference router.
    pub fn new(
        notification_pref_repo: NotificationPreferenceRepository,
        granular_repo: GranularNotificationRepository,
    ) -> Self {
        Self {
            notification_pref_repo,
            granular_repo,
        }
    }

    /// Resolve the routing decision for a user and notification.
    pub async fn resolve(
        &self,
        user_id: Uuid,
        notification: &Notification,
        requested_channels: &[NotificationChannel],
    ) -> Result<RoutingDecision, NotificationError> {
        let event_type = format!("{}.notification", notification.category.as_str());
        let mut enabled = Vec::new();
        let mut skipped = Vec::new();

        for &channel in requested_channels {
            let ch_enabled = self
                .is_channel_enabled(user_id, &event_type, channel)
                .await?;
            if ch_enabled {
                enabled.push(channel);
            } else {
                skipped.push(channel);
            }
        }

        Ok(RoutingDecision {
            enabled_channels: enabled,
            skipped_channels: skipped,
        })
    }

    /// Check whether a specific channel is enabled for a user.
    async fn is_channel_enabled(
        &self,
        user_id: Uuid,
        event_type: &str,
        channel: NotificationChannel,
    ) -> Result<bool, NotificationError> {
        use db::models::notification_preference::NotificationChannel as DbChannel;

        // 1. Check per-event-type granular preference
        if let Some(pref) = self
            .granular_repo
            .get_user_event_preference(user_id, event_type)
            .await
            .map_err(|e| NotificationError::Database(e.to_string()))?
        {
            return Ok(match channel {
                NotificationChannel::Push => pref.push_enabled,
                NotificationChannel::Email => pref.email_enabled,
                NotificationChannel::InApp => pref.in_app_enabled,
            });
        }

        // 2. Fall back to channel-level preference
        let db_channel = match channel {
            NotificationChannel::Push => DbChannel::Push,
            NotificationChannel::Email => DbChannel::Email,
            NotificationChannel::InApp => DbChannel::InApp,
        };

        #[allow(deprecated)]
        if let Some(pref) = self
            .notification_pref_repo
            .get_by_user_and_channel(user_id, db_channel)
            .await
            .map_err(|e| NotificationError::Database(e.to_string()))?
        {
            return Ok(pref.enabled);
        }

        // 3. Default: enabled
        Ok(true)
    }
}

// ============================================================================
// Story 2b-1 & 2b-5 — Notification pipeline (orchestrator)
// ============================================================================

/// Channels requested by default when the caller does not specify.
const DEFAULT_CHANNELS: &[NotificationChannel] = NotificationChannel::all();

/// Configuration for the notification pipeline.
#[derive(Clone, Debug)]
pub struct PipelineConfig {
    /// Whether the pipeline is enabled.  When `false`, all dispatches are no-ops.
    pub enabled: bool,
    /// Channels attempted when not overridden per-notification.
    pub default_channels: Vec<NotificationChannel>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_channels: DEFAULT_CHANNELS.to_vec(),
        }
    }
}

/// The Epic 2B notification delivery pipeline.
///
/// Wires together preference routing (2b-2), transport adapters (2b-4), and
/// delivery tracking (2b-3) into a single `dispatch` entry point (2b-5).
#[derive(Clone)]
pub struct NotificationPipeline {
    config: PipelineConfig,
    user_repo: UserRepository,
    preference_router: PreferenceRouter,
    email_adapter: Arc<dyn EmailTransport>,
    push_adapter: Arc<dyn PushTransport>,
    in_app_adapter: Arc<dyn InAppTransport>,
}

impl NotificationPipeline {
    /// Create a fully-wired pipeline from the application's shared resources.
    pub fn new(
        pool: DbPool,
        email_service: EmailService,
        pubsub: Option<integrations::PubSubService>,
        config: PipelineConfig,
    ) -> Self {
        let user_repo = UserRepository::new(pool.clone());
        let notification_pref_repo = NotificationPreferenceRepository::new(pool.clone());
        let granular_repo = GranularNotificationRepository::new(pool.clone());

        let preference_router =
            PreferenceRouter::new(notification_pref_repo, granular_repo.clone());

        let email_adapter = Arc::new(SmtpEmailAdapter::new(email_service))
            as Arc<dyn EmailTransport>;
        let push_adapter = Arc::new(FcmPushAdapter::from_env()) as Arc<dyn PushTransport>;
        let in_app_adapter =
            Arc::new(DbInAppAdapter::new(granular_repo, pubsub)) as Arc<dyn InAppTransport>;

        Self {
            config,
            user_repo,
            preference_router,
            email_adapter,
            push_adapter,
            in_app_adapter,
        }
    }

    /// Create a development/test pipeline that logs instead of sending.
    pub fn development(pool: DbPool) -> Self {
        Self::new(
            pool,
            EmailService::development(),
            None,
            PipelineConfig {
                enabled: false,
                ..Default::default()
            },
        )
    }

    // ------------------------------------------------------------------
    // Story 2b-5 — Public dispatch entry points
    // ------------------------------------------------------------------

    /// Dispatch a notification to a single user.
    ///
    /// Returns a `PipelineResult` summarising how many channel deliveries
    /// succeeded, were skipped, or failed.
    pub async fn dispatch(
        &self,
        user_id: Uuid,
        notification: &Notification,
        entity_id: Option<Uuid>,
        channels: Option<&[NotificationChannel]>,
    ) -> Result<PipelineResult, NotificationError> {
        if !self.config.enabled {
            tracing::debug!(
                user_id = %user_id,
                title = %notification.title,
                "[Epic 2B] Pipeline disabled — notification skipped"
            );
            return Ok(PipelineResult::default());
        }

        // Resolve user (needed for email locale + address)
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await
            .map_err(|e| NotificationError::Database(e.to_string()))?
            .ok_or(NotificationError::RecipientResolution(format!(
                "user {user_id} not found"
            )))?;

        // Determine requested channels
        let requested: &[NotificationChannel] = channels
            .unwrap_or_else(|| &self.config.default_channels);

        // Apply preference routing (2b-2)
        let routing = self
            .preference_router
            .resolve(user_id, notification, requested)
            .await?;

        let mut result = PipelineResult::default();

        // Record skipped channels
        for ch in &routing.skipped_channels {
            result.skipped += 1;
            result.records.push(
                DeliveryRecord::pending(notification.id, user_id, *ch).into_skipped(),
            );
        }

        // Deliver on each enabled channel (2b-4 adapters)
        for channel in &routing.enabled_channels {
            let record = self
                .deliver_channel(user_id, &user, notification, entity_id, *channel)
                .await;
            if record.status == DeliveryStatus::Sent {
                result.sent += 1;
            } else {
                result.failed += 1;
            }
            result.records.push(record);
        }

        tracing::info!(
            user_id = %user_id,
            sent = result.sent,
            skipped = result.skipped,
            failed = result.failed,
            "[Epic 2B] Notification dispatch complete"
        );

        Ok(result)
    }

    /// Dispatch a notification to multiple users, returning per-user results.
    ///
    /// Failures for individual users are recorded in their `PipelineResult`
    /// but do not abort delivery to the remaining users.
    pub async fn dispatch_to_users(
        &self,
        user_ids: &[Uuid],
        notification: &Notification,
        entity_id: Option<Uuid>,
        channels: Option<&[NotificationChannel]>,
    ) -> Vec<(Uuid, PipelineResult)> {
        let mut results = Vec::with_capacity(user_ids.len());

        for &user_id in user_ids {
            let r = match self
                .dispatch(user_id, notification, entity_id, channels)
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(
                        user_id = %user_id,
                        error = %e,
                        "[Epic 2B] Dispatch error for user"
                    );
                    // Return an empty result so callers can still tally totals
                    PipelineResult::default()
                }
            };
            results.push((user_id, r));
        }

        results
    }

    /// Aggregate `dispatch_to_users` into a single summary count.
    pub async fn broadcast(
        &self,
        user_ids: &[Uuid],
        notification: &Notification,
        entity_id: Option<Uuid>,
    ) -> (usize, usize, usize) {
        let per_user = self
            .dispatch_to_users(user_ids, notification, entity_id, None)
            .await;

        per_user
            .iter()
            .fold((0, 0, 0), |(s, sk, f), (_, r)| {
                (s + r.sent, sk + r.skipped, f + r.failed)
            })
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    /// Attempt delivery on a single channel, returning a finalised `DeliveryRecord`.
    async fn deliver_channel(
        &self,
        user_id: Uuid,
        user: &User,
        notification: &Notification,
        entity_id: Option<Uuid>,
        channel: NotificationChannel,
    ) -> DeliveryRecord {
        let record = DeliveryRecord::pending(notification.id, user_id, channel);

        let outcome = match channel {
            NotificationChannel::Email => {
                self.email_adapter
                    .send(&user.email, &user.name, notification, &user.locale)
                    .await
            }
            NotificationChannel::Push => {
                // Device tokens would come from a device-token repo; stubbed as empty for now.
                // A follow-up PR should add `DeviceTokenRepository` and wire it here.
                self.push_adapter
                    .send(user_id, &[], notification)
                    .await
            }
            NotificationChannel::InApp => {
                self.in_app_adapter
                    .send(user_id, notification, entity_id)
                    .await
            }
        };

        match outcome {
            Ok(()) => record.into_sent(),
            Err(e) => {
                tracing::warn!(
                    user_id = %user_id,
                    channel = %channel,
                    error = %e,
                    "[Epic 2B] Channel delivery failed"
                );
                record.into_failed(e.to_string())
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use common::notifications::{Notification, NotificationCategory, NotificationPriority};

    fn make_notification(user_id: Uuid) -> Notification {
        Notification::new(
            user_id,
            NotificationCategory::Announcements,
            "Test announcement",
            "Hello from Epic 2B",
        )
        .with_priority(NotificationPriority::Normal)
    }

    #[test]
    fn pipeline_config_defaults() {
        let config = PipelineConfig::default();
        assert!(config.enabled);
        assert_eq!(config.default_channels.len(), 3);
    }

    #[test]
    fn smtp_adapter_locale_parsing() {
        assert_eq!(SmtpEmailAdapter::parse_locale("sk"), Locale::Slovak);
        assert_eq!(SmtpEmailAdapter::parse_locale("cs"), Locale::Czech);
        assert_eq!(SmtpEmailAdapter::parse_locale("de"), Locale::German);
        assert_eq!(SmtpEmailAdapter::parse_locale("en"), Locale::English);
        assert_eq!(SmtpEmailAdapter::parse_locale("unknown"), Locale::English);
    }

    #[test]
    fn fcm_adapter_is_configured() {
        let without_key = FcmPushAdapter::new(None);
        assert!(!without_key.is_configured());

        let with_key = FcmPushAdapter::new(Some("server-key-abc".to_string()));
        assert!(with_key.is_configured());
    }

    #[test]
    fn notification_category_for_event_type() {
        let uid = Uuid::new_v4();
        let n = make_notification(uid);
        let event_type = format!("{}.notification", n.category.as_str());
        assert_eq!(event_type, "announcements.notification");
    }

    #[test]
    fn delivery_record_full_lifecycle() {
        let uid = Uuid::new_v4();
        let nid = Uuid::new_v4();

        let pending = DeliveryRecord::pending(nid, uid, NotificationChannel::Email);
        assert_eq!(pending.status, DeliveryStatus::Pending);
        assert!(pending.delivered_at.is_none());

        let sent = pending.clone().into_sent();
        assert!(sent.status.is_sent());
        assert!(sent.delivered_at.is_some());

        let failed = pending.clone().into_failed("SMTP timeout");
        assert!(failed.status.is_retryable());
        assert_eq!(failed.error_message.as_deref(), Some("SMTP timeout"));

        let skipped = pending.into_skipped();
        assert_eq!(skipped.status, DeliveryStatus::Skipped);
    }
}
