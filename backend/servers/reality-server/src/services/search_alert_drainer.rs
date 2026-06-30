//! Saved-search alert email/push **transport drainer** — BIT-139 (Epic 16,
//! issue #983 follow-up).
//!
//! The saved-search matching engine ([`super::SavedSearchAlertWorker`]) enqueues
//! `search_alert_queue` rows and the portal surfaces them as an in-app feed
//! (`GET /api/v1/saved-searches/alerts`). Until now that in-app pull was the
//! *only* delivery channel — reality-server had no email/push transport.
//!
//! This background worker drains the same queue out-of-band: for each alert it
//! has not yet delivered it dispatches an email to the owning user and a push
//! notification to each of their registered devices (`device_push_tokens`), then
//! records delivery in `notified_at` (migration 00195). That column is tracked
//! independently of `status` — which belongs to the in-app read channel — so the
//! drainer never re-sends on every poll and never clobbers the unread badge.
//!
//! **First cut (this change):** the transports are pluggable behind the
//! [`AlertEmailTransport`] / [`AlertPushTransport`] traits, defaulting to
//! logging stubs ([`LogEmailTransport`] / [`LogPushTransport`]). reality-server
//! has no real email service wired yet, so the stubs let the full drain →
//! device-token fan-out → mark-delivered pipeline land and be exercised by tests
//! now; a real SMTP/FCM/APNs adapter can be injected later via
//! [`SearchAlertDrainerWorker::with_transports`] without touching the loop.
//!
//! **Cross-tenant note:** the drainer runs service-role (no `app.current_user_id`
//! on its connections). `search_alert_queue` / `portal_saved_searches` / `users`
//! are not RLS-gated, and `device_push_tokens` exposes a service-role SELECT
//! policy. Every row carries its *own* owner's contact details and we fan tokens
//! out strictly by that row's `user_id`, so one user's alert can never be
//! delivered to another user's address or device.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use db::repositories::{DevicePushTokenRepository, RealityPortalRepository};
use db::DbPool;
use tokio::time::interval;
use tracing::Instrument;

/// A composed alert notification, ready to hand to a transport.
#[derive(Debug, Clone)]
pub struct AlertNotification {
    pub subject: String,
    pub body: String,
    /// Recipient locale (`sk` | `cs` | `de` | `en`), for localized copy later.
    pub locale: String,
}

/// Email side of the drainer's transport. The default is a logging stub; a real
/// SMTP-backed adapter can be injected via
/// [`SearchAlertDrainerWorker::with_transports`].
#[async_trait]
pub trait AlertEmailTransport: Send + Sync {
    /// Deliver `notification` to `to_email`. `Err` is a transient failure: the
    /// drainer records an attempt and retries on a later poll.
    async fn send_email(
        &self,
        to_email: &str,
        to_name: &str,
        notification: &AlertNotification,
    ) -> Result<(), String>;
}

/// Push side of the drainer's transport (one call per device token).
#[async_trait]
pub trait AlertPushTransport: Send + Sync {
    async fn send_push(
        &self,
        token: &str,
        platform: &str,
        notification: &AlertNotification,
    ) -> Result<(), String>;
}

/// Logging email stub — records what *would* be sent. Used until a real email
/// service is wired into reality-server.
pub struct LogEmailTransport;

#[async_trait]
impl AlertEmailTransport for LogEmailTransport {
    async fn send_email(
        &self,
        to_email: &str,
        _to_name: &str,
        notification: &AlertNotification,
    ) -> Result<(), String> {
        tracing::info!(
            target: "bg.search_alert_drainer",
            to = %to_email,
            subject = %notification.subject,
            "[BIT-139] (stub) email alert dispatched"
        );
        Ok(())
    }
}

/// Logging push stub — records what *would* be sent per device token.
pub struct LogPushTransport;

#[async_trait]
impl AlertPushTransport for LogPushTransport {
    async fn send_push(
        &self,
        token: &str,
        platform: &str,
        notification: &AlertNotification,
    ) -> Result<(), String> {
        // Never log the full token (a credential): a short prefix is enough to
        // correlate without leaking it into logs.
        let token_prefix: String = token.chars().take(8).collect();
        tracing::info!(
            target: "bg.search_alert_drainer",
            platform = %platform,
            token_prefix = %token_prefix,
            subject = %notification.subject,
            "[BIT-139] (stub) push alert dispatched"
        );
        Ok(())
    }
}

/// Configuration for the saved-search alert transport drainer.
#[derive(Debug, Clone)]
pub struct SearchAlertDrainerConfig {
    pub enabled: bool,
    pub poll_interval_secs: u64,
    /// Max alerts drained per poll.
    pub batch_size: i64,
    /// Give up retrying a row after this many failed delivery attempts.
    pub max_attempts: i32,
}

impl Default for SearchAlertDrainerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            poll_interval_secs: 60,
            batch_size: 100,
            max_attempts: 5,
        }
    }
}

impl SearchAlertDrainerConfig {
    /// Build from environment:
    /// - `SEARCH_ALERT_DRAINER_ENABLED` (default `true`)
    /// - `SEARCH_ALERT_DRAINER_INTERVAL_SECS` (default `60`)
    /// - `SEARCH_ALERT_DRAINER_BATCH_SIZE` (default `100`)
    /// - `SEARCH_ALERT_DRAINER_MAX_ATTEMPTS` (default `5`)
    pub fn from_env() -> Self {
        let default = Self::default();
        let enabled = std::env::var("SEARCH_ALERT_DRAINER_ENABLED")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(default.enabled);
        let poll_interval_secs = std::env::var("SEARCH_ALERT_DRAINER_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.poll_interval_secs);
        let batch_size = std::env::var("SEARCH_ALERT_DRAINER_BATCH_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.batch_size);
        let max_attempts = std::env::var("SEARCH_ALERT_DRAINER_MAX_ATTEMPTS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.max_attempts);
        Self {
            enabled,
            poll_interval_secs,
            batch_size,
            max_attempts,
        }
    }
}

/// Background worker that drains undelivered `search_alert_queue` rows to email
/// and push transports.
pub struct SearchAlertDrainerWorker {
    repo: RealityPortalRepository,
    push_tokens: DevicePushTokenRepository,
    email: Arc<dyn AlertEmailTransport>,
    push: Arc<dyn AlertPushTransport>,
    config: SearchAlertDrainerConfig,
}

impl SearchAlertDrainerWorker {
    /// Construct with the default logging stubs (no real email/push service).
    pub fn new(db: DbPool, config: SearchAlertDrainerConfig) -> Self {
        Self::with_transports(
            db,
            config,
            Arc::new(LogEmailTransport),
            Arc::new(LogPushTransport),
        )
    }

    /// Construct with explicit transports — used by tests and, later, by
    /// production wiring once real SMTP/FCM/APNs adapters exist.
    pub fn with_transports(
        db: DbPool,
        config: SearchAlertDrainerConfig,
        email: Arc<dyn AlertEmailTransport>,
        push: Arc<dyn AlertPushTransport>,
    ) -> Self {
        Self {
            repo: RealityPortalRepository::new(db.clone()),
            push_tokens: DevicePushTokenRepository::new(db),
            email,
            push,
            config,
        }
    }

    /// Spawn the background task and return its `JoinHandle`.
    pub fn start(self) -> tokio::task::JoinHandle<()> {
        let poll_secs = self.config.poll_interval_secs;
        tokio::spawn(
            async move {
                if !self.config.enabled {
                    tracing::info!("[BIT-139] SearchAlertDrainerWorker disabled — not starting");
                    return;
                }
                tracing::info!(
                    poll_interval_secs = self.config.poll_interval_secs,
                    batch_size = self.config.batch_size,
                    "[BIT-139] SearchAlertDrainerWorker started"
                );
                let mut ticker = interval(Duration::from_secs(self.config.poll_interval_secs));
                loop {
                    ticker.tick().await;
                    self.run_once().await;
                }
            }
            .instrument(tracing::info_span!(
                "bg.search_alert_drainer",
                poll_secs = poll_secs
            )),
        )
    }

    /// One drain pass. Returns the number of alerts marked delivered (handy for
    /// tests; ignored by the loop).
    pub async fn run_once(&self) -> usize {
        let alerts = match self
            .repo
            .list_undelivered_search_alerts(self.config.batch_size, self.config.max_attempts)
            .await
        {
            Ok(a) => a,
            Err(e) => {
                tracing::error!(error = %e, "[BIT-139] failed to list undelivered alerts");
                return 0;
            }
        };

        let mut delivered = 0usize;
        for alert in alerts {
            let count = alert.matching_listing_ids.len();
            let notification = AlertNotification {
                subject: format!(
                    "{count} new listing{} match your saved search \"{}\"",
                    if count == 1 { "" } else { "s" },
                    alert.saved_search_name
                ),
                body: format!(
                    "Your saved search \"{}\" has {count} new matching listing{}. \
                     Open the app to view {}.",
                    alert.saved_search_name,
                    if count == 1 { "" } else { "s" },
                    if count == 1 { "it" } else { "them" },
                ),
                locale: alert
                    .recipient_locale
                    .clone()
                    .unwrap_or_else(|| "en".into()),
            };

            // An alert counts as delivered if *any* channel succeeds. Email is
            // the primary channel; push is best-effort fan-out across devices.
            let mut any_ok = false;

            match self
                .email
                .send_email(&alert.recipient_email, &alert.recipient_name, &notification)
                .await
            {
                Ok(()) => any_ok = true,
                Err(e) => tracing::warn!(
                    alert_id = %alert.id, error = %e,
                    "[BIT-139] email transport failed"
                ),
            }

            // Push fan-out: strictly this row's owner's tokens (service-role read).
            match self.push_tokens.get_tokens_for_user(alert.user_id).await {
                Ok(tokens) => {
                    for tok in &tokens {
                        match self
                            .push
                            .send_push(&tok.token, &tok.platform, &notification)
                            .await
                        {
                            Ok(()) => any_ok = true,
                            Err(e) => tracing::warn!(
                                alert_id = %alert.id, error = %e,
                                "[BIT-139] push transport failed for a device"
                            ),
                        }
                    }
                }
                Err(e) => tracing::warn!(
                    alert_id = %alert.id, error = %e,
                    "[BIT-139] failed to load device tokens"
                ),
            }

            if any_ok {
                if let Err(e) = self.repo.mark_search_alert_notified(alert.id).await {
                    tracing::error!(alert_id = %alert.id, error = %e, "[BIT-139] mark-notified failed");
                } else {
                    delivered += 1;
                }
            } else if let Err(e) = self.repo.record_search_alert_notify_failure(alert.id).await {
                tracing::error!(alert_id = %alert.id, error = %e, "[BIT-139] record-failure failed");
            }
        }

        if delivered > 0 {
            tracing::info!(delivered, "[BIT-139] saved-search alerts delivered");
        }
        delivered
    }
}
