//! Epic 2B — Notification Delivery Pipeline
//!
//! # Stories implemented
//! - **2b-1** Channel infrastructure: `NotificationPipeline` + `DeliveryRequest` dispatch
//! - **2b-2** Preference routing: `PreferenceRouter` checks per-user, per-channel settings
//! - **2b-3** Delivery tracking: in-memory + logging; ready to swap for a DB adapter
//! - **2b-4** Transport adapters: `SmtpEmailAdapter`, `CombinedPushAdapter` (FCM + APNs), `DbInAppAdapter`
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
//!         ├─ push   ─► impl PushTransport  (CombinedPushAdapter: FCM + APNs)
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
    models::{CreateHeldNotification, HeldNotification, Locale, NewNotificationEvent, User},
    repositories::{
        GranularNotificationRepository, NotificationEventRepository,
        NotificationPreferenceRepository, UserRepository,
    },
    DbPool,
};
use uuid::Uuid;

use common::notifications::{
    pipeline::{DeliveryRecord, DeliveryStatus, PipelineResult, RoutingDecision},
    EmailTransport, InAppTransport, Notification, NotificationCategory, NotificationChannel,
    NotificationError, NotificationPriority, PushTransport, TransportResult,
};

use super::email::EmailService;
use super::push_fanout::CombinedPushAdapter;
use super::quiet_hours;

/// Map a stored `held_notifications.event_type` string back to a category for
/// re-delivery (best-effort; unknown values become `System`). The string is the
/// `NotificationCategory::as_str()` we wrote when holding.
fn category_from_str(s: &str) -> NotificationCategory {
    match s {
        "announcements" => NotificationCategory::Announcements,
        "faults" => NotificationCategory::Faults,
        "votes" => NotificationCategory::Votes,
        "messages" => NotificationCategory::Messages,
        "community" => NotificationCategory::Community,
        "financial" => NotificationCategory::Financial,
        "documents" => NotificationCategory::Documents,
        _ => NotificationCategory::System,
    }
}

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

        // Use the context-setting variant: the pipeline runs on the service
        // pool with no per-request user context, but the notification tables
        // are RLS-gated on `user_id = app.current_user_id`. This sets the
        // recipient's context for the write so the mandatory in-app record is
        // actually persisted in production (not just in owner-role tests).
        self.granular_repo
            .add_notification_to_group_for_user(
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
            let msg = integrations::PubSubMessage::new(
                &channel,
                "notification.created",
                serde_json::json!({
                    "notification_id": notification.id,
                    "category": notification.category,
                    "title": notification.title,
                    "entity_id": eid,
                }),
            );
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
/// Map the pipeline's in-memory `DeliveryRecord`s into persistable analytics
/// events (Story 2B-C.3 / #969-4). The `DeliveryStatus` display form
/// (`sent`/`failed`/`skipped`/`pending`) is exactly the `event` value the
/// `notification_events` table accepts. `occurred_at` prefers the confirmed
/// `delivered_at` and falls back to the attempt time.
fn records_to_events(records: &[DeliveryRecord]) -> Vec<NewNotificationEvent> {
    records
        .iter()
        .map(|r| NewNotificationEvent {
            notification_id: r.notification_id,
            user_id: r.user_id,
            channel: r.channel.as_str().to_string(),
            event: r.status.to_string(),
            error_message: r.error_message.clone(),
            occurred_at: r.delivered_at.unwrap_or(r.attempted_at),
        })
        .collect()
}

#[derive(Clone)]
pub struct NotificationPipeline {
    config: PipelineConfig,
    user_repo: UserRepository,
    preference_router: PreferenceRouter,
    email_adapter: Arc<dyn EmailTransport>,
    push_adapter: Arc<dyn PushTransport>,
    in_app_adapter: Arc<dyn InAppTransport>,
    /// Story 8B.3 / #980: read quiet-hours schedules and persist held pushes.
    granular_repo: GranularNotificationRepository,
    /// Story 2B-C.3 / #969-4: append delivery records for analytics,
    /// asynchronously off the dispatch path.
    events_repo: NotificationEventRepository,
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
        let events_repo = NotificationEventRepository::new(pool.clone());

        let preference_router =
            PreferenceRouter::new(notification_pref_repo, granular_repo.clone());

        let email_adapter =
            Arc::new(SmtpEmailAdapter::new(email_service)) as Arc<dyn EmailTransport>;
        // Epic 8A-3 / gap-84-4: use the *combined* FCM + APNs adapter so push
        // fans out to real device transports on BOTH platforms — Android (FCM)
        // and iOS (APNs) — for the synchronous in-process dispatch path, not
        // just the background fanout worker. Previously the pipeline wired the
        // FCM-only adapter, which treated APNs tokens as log-only ("not yet
        // implemented"), so iOS devices had no real device transport here.
        // Each provider independently falls back to `PushNotConfigured`
        // (→ recorded as `skipped`, not `sent`) when its credentials are unset.
        let push_adapter =
            Arc::new(CombinedPushAdapter::from_env(pool.clone())) as Arc<dyn PushTransport>;
        let in_app_adapter =
            Arc::new(DbInAppAdapter::new(granular_repo.clone(), pubsub)) as Arc<dyn InAppTransport>;

        Self {
            config,
            user_repo,
            preference_router,
            email_adapter,
            push_adapter,
            in_app_adapter,
            granular_repo,
            events_repo,
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
        let requested: &[NotificationChannel] =
            channels.unwrap_or_else(|| &self.config.default_channels);

        // Routing (2b-2) with two overrides driven by Epic 2B requirements:
        //
        // 1. URGENCY BYPASS — `Urgent` (critical/urgent) notifications bypass
        //    user preferences entirely. Safety-critical alerts (emergencies,
        //    critical notifications) must reach the recipient regardless of
        //    opt-outs, so every requested channel is force-enabled.
        //
        // 2. MANDATORY IN-APP — for all other priorities we honour preferences
        //    for email/push, but the in-app channel is never skipped: every
        //    recipient gets a durable stored notification (the DB record) even
        //    if they muted in-app, so nothing is silently dropped. Users still
        //    control read state; they just can't lose the record.
        let routing = if matches!(notification.priority, NotificationPriority::Urgent) {
            RoutingDecision::all_enabled(requested)
        } else {
            let mut routing = self
                .preference_router
                .resolve(user_id, notification, requested)
                .await?;
            if requested.contains(&NotificationChannel::InApp)
                && !routing
                    .enabled_channels
                    .contains(&NotificationChannel::InApp)
            {
                routing
                    .skipped_channels
                    .retain(|c| *c != NotificationChannel::InApp);
                routing.enabled_channels.push(NotificationChannel::InApp);
            }
            routing
        };

        let mut result = PipelineResult::default();

        // Record skipped channels
        for ch in &routing.skipped_channels {
            result.skipped += 1;
            result
                .records
                .push(DeliveryRecord::pending(notification.id, user_id, *ch).into_skipped());
        }

        // Issue #980 (Story 8B.3): resolve the user's quiet-hours window once.
        // During an active window, the Push channel is *held* for delivery when
        // quiet hours end; in-app and email are unaffected. Urgent notifications
        // bypass quiet hours entirely (same carve-out as the preference bypass
        // above), so we don't even fetch the schedule for them.
        let quiet = if matches!(notification.priority, NotificationPriority::Urgent) {
            None
        } else {
            self.granular_repo
                .get_user_schedule(user_id)
                .await
                .ok()
                .flatten()
                .map(|s| quiet_hours::evaluate(&s, chrono::Utc::now()))
                .filter(|state| state.active)
        };
        let mut held = 0usize;

        // Deliver on each enabled channel (2b-4 adapters)
        for channel in &routing.enabled_channels {
            if *channel == NotificationChannel::Push {
                if let Some(state) = &quiet {
                    // Default to an 8h horizon if the window end couldn't be
                    // computed, so a held push is never lost forever.
                    let release_at = state
                        .ends_at
                        .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::hours(8));
                    match self
                        .granular_repo
                        .create_held_notification(CreateHeldNotification {
                            user_id,
                            event_type: notification.category.as_str().to_string(),
                            title: notification.title.clone(),
                            body: Some(notification.body.clone()),
                            data: notification
                                .action_url
                                .as_ref()
                                .map(|u| serde_json::json!({ "action_url": u })),
                            channels: vec![NotificationChannel::Push.as_str().to_string()],
                            release_at,
                            is_priority: false,
                        })
                        .await
                    {
                        Ok(_) => {
                            held += 1;
                            // A held push wasn't sent — record it as skipped so
                            // `sent`/`failed` stay honest (see `held` log below).
                            result.skipped += 1;
                            result.records.push(
                                DeliveryRecord::pending(notification.id, user_id, *channel)
                                    .into_skipped(),
                            );
                            tracing::info!(
                                user_id = %user_id,
                                release_at = %release_at,
                                "[#980] Push held during quiet hours"
                            );
                            continue;
                        }
                        Err(e) => {
                            // Failing to persist the hold must not drop the push;
                            // fall through to immediate delivery.
                            tracing::warn!(
                                user_id = %user_id,
                                error = %e,
                                "[#980] Failed to hold push during quiet hours; delivering now"
                            );
                        }
                    }
                }
            }

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
            held = held,
            failed = result.failed,
            "[Epic 2B] Notification dispatch complete"
        );

        // Story 2B-C.3 (#969-4): persist delivery records for analytics. This is
        // strictly off the dispatch path — we spawn a detached task so a slow or
        // failing write can never delay or fail delivery, and we log (never
        // propagate) any error. Records are cloned out before the result is
        // returned to the caller so nothing is dropped on the success path.
        let events = records_to_events(&result.records);
        if !events.is_empty() {
            let repo = self.events_repo.clone();
            tokio::spawn(async move {
                if let Err(e) = repo.insert_events(&events).await {
                    tracing::warn!(
                        error = %e,
                        "[Epic 2B] Failed to persist notification delivery events"
                    );
                }
            });
        }

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
        // Issue #484: bounded-concurrency fan-out so a building-wide
        // announcement to 200 residents finishes in ~max-per-user time
        // (≈10 ms) rather than ~sum-of-all-times (≈1 s). Concurrency cap
        // protects the Postgres pool.
        use futures_util::stream::{self, StreamExt};
        const DISPATCH_CONCURRENCY: usize = 20;

        stream::iter(user_ids.iter().copied())
            .map(|user_id| async move {
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
                        PipelineResult::default()
                    }
                };
                (user_id, r)
            })
            .buffer_unordered(DISPATCH_CONCURRENCY)
            .collect::<Vec<_>>()
            .await
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

        per_user.iter().fold((0, 0, 0), |(s, sk, f), (_, r)| {
            (s + r.sent, sk + r.skipped, f + r.failed)
        })
    }

    /// Issue #980: deliver a previously held notification on its stored
    /// channels, bypassing the quiet-hours gate (the drain worker only releases
    /// rows whose `release_at` has passed, i.e. quiet hours have ended).
    ///
    /// Returns a [`PipelineResult`] summarising the attempt. The drain worker
    /// keys off `failed`: a non-zero `failed` means at least one channel hit a
    /// transient error and the held row must be left for the next tick rather
    /// than marked released (otherwise the notification is permanently dropped
    /// on a transient failure). A fully-skipped result (`sent == 0 &&
    /// failed == 0`, e.g. push not configured or unknown user) carries no
    /// failures and is safe to release — retrying it would never make progress.
    pub async fn deliver_held(&self, held: &HeldNotification) -> PipelineResult {
        let user = match self.user_repo.find_by_id(held.user_id).await {
            Ok(Some(u)) => u,
            Ok(None) => {
                tracing::warn!(user_id = %held.user_id, "[#980] Held notification for unknown user; dropping");
                // Nothing deliverable and no point retrying — no failure.
                return PipelineResult::default();
            }
            Err(e) => {
                tracing::warn!(user_id = %held.user_id, error = %e, "[#980] Failed to resolve user for held notification");
                // Transient lookup error — report as a failure so the drain
                // worker retries on the next tick instead of dropping the row.
                return PipelineResult {
                    failed: 1,
                    ..PipelineResult::default()
                };
            }
        };

        let mut notification = Notification::new(
            held.user_id,
            category_from_str(&held.event_type),
            held.title.clone(),
            held.body.clone().unwrap_or_default(),
        );
        if let Some(url) = held
            .data
            .as_ref()
            .and_then(|d| d.get("action_url"))
            .and_then(|u| u.as_str())
        {
            notification = notification.with_action_url(url);
        }

        let mut result = PipelineResult::default();
        for ch in &held.channels {
            let channel = match ch.as_str() {
                "push" => NotificationChannel::Push,
                "email" => NotificationChannel::Email,
                "in_app" => NotificationChannel::InApp,
                other => {
                    tracing::warn!(channel = %other, "[#980] Unknown held channel; skipping");
                    // Unrecognised channel is a skip, not a failure — it must
                    // not block the row from being released.
                    result.skipped += 1;
                    continue;
                }
            };
            let record = self
                .deliver_channel(held.user_id, &user, &notification, None, channel)
                .await;
            match record.status {
                DeliveryStatus::Sent => result.sent += 1,
                DeliveryStatus::Failed => result.failed += 1,
                DeliveryStatus::Skipped | DeliveryStatus::Pending => result.skipped += 1,
            }
            result.records.push(record);
        }
        result
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
                // Epic 8A-3 / gap-84-4: `CombinedPushAdapter` fetches device
                // tokens internally from `device_push_tokens` via the
                // service-role pool and routes each token to its provider
                // (FCM for Android, APNs for iOS).
                self.push_adapter.send(user_id, notification).await
            }
            NotificationChannel::InApp => {
                self.in_app_adapter
                    .send(user_id, notification, entity_id)
                    .await
            }
        };

        match outcome {
            Ok(()) => record.into_sent(),
            // Issue #484: transport-not-configured is a `skipped`
            // outcome, not a failure. Keeps `sent` counters honest.
            Err(NotificationError::PushNotConfigured) => {
                // `PushNotConfigured` is returned both when no provider is
                // configured AND when providers are configured but the user has
                // no deliverable device tokens (`combine()` NoTargets + NoTargets).
                // Both are Skipped, not Failed — keep the wording accurate for
                // operators diagnosing why a push was skipped.
                tracing::debug!(
                    user_id = %user_id,
                    channel = %channel,
                    "[Epic 2B] Push channel skipped — provider not configured or no deliverable device targets"
                );
                record.into_skipped()
            }
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
