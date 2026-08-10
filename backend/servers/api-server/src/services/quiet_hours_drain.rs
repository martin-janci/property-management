//! Quiet-hours hold-queue drain worker — Story 8B.3 (issue #980).
//!
//! Push notifications received during a user's quiet-hours window are persisted
//! to `held_notifications` by the [`NotificationPipeline`] gate with a
//! `release_at` equal to the end of the window. This background worker polls for
//! rows whose `release_at` has passed and re-delivers them, then marks them
//! released — so the held push lands when quiet hours end instead of being
//! dropped. Mirrors the `PushFanoutWorker` lifecycle so the server always starts
//! cleanly (disabled → no-op).

use std::time::Duration;

use common::notifications::PipelineResult;
use db::{repositories::GranularNotificationRepository, DbPool};
use tokio::time::interval;
use tracing::Instrument;

use super::email::EmailService;
use super::notification_pipeline::NotificationPipeline;
use crate::services::notification_pipeline::PipelineConfig;

/// Configuration for the quiet-hours drain worker.
#[derive(Debug, Clone)]
pub struct QuietHoursDrainConfig {
    /// When false, the worker starts but does nothing (no-op heartbeat).
    pub enabled: bool,
    /// How often to poll `held_notifications` for due rows.
    pub poll_interval_secs: u64,
    /// Safety cap on rows processed per tick.
    pub batch_limit: usize,
}

impl Default for QuietHoursDrainConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            poll_interval_secs: 60,
            batch_limit: 500,
        }
    }
}

impl QuietHoursDrainConfig {
    /// Build from environment:
    /// - `QUIET_HOURS_DRAIN_ENABLED` (default `true`)
    /// - `QUIET_HOURS_DRAIN_INTERVAL_SECS` (default `60`)
    pub fn from_env() -> Self {
        let default = Self::default();
        let enabled = std::env::var("QUIET_HOURS_DRAIN_ENABLED")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(default.enabled);
        let poll_interval_secs = std::env::var("QUIET_HOURS_DRAIN_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.poll_interval_secs);
        Self {
            enabled,
            poll_interval_secs,
            ..default
        }
    }
}

/// Background worker that releases held notifications when their quiet-hours
/// window has ended.
pub struct QuietHoursDrainWorker {
    granular_repo: GranularNotificationRepository,
    pipeline: NotificationPipeline,
    config: QuietHoursDrainConfig,
}

impl QuietHoursDrainWorker {
    /// Construct the worker from shared resources. Uses a fully-enabled
    /// [`NotificationPipeline`] so released notifications actually deliver
    /// (the pipeline's own quiet-hours gate is bypassed via `deliver_held`).
    pub fn new(
        pool: DbPool,
        email_service: EmailService,
        pubsub: Option<integrations::PubSubService>,
        config: QuietHoursDrainConfig,
    ) -> Self {
        let granular_repo = GranularNotificationRepository::new(pool.clone());
        let pipeline =
            NotificationPipeline::new(pool, email_service, pubsub, PipelineConfig::default());
        Self {
            granular_repo,
            pipeline,
            config,
        }
    }

    /// Spawn the background task and return its `JoinHandle`.
    pub fn start(self) -> tokio::task::JoinHandle<()> {
        let poll_secs = self.config.poll_interval_secs;
        tokio::spawn(
            async move {
                if !self.config.enabled {
                    tracing::info!("[#980] QuietHoursDrainWorker disabled — not starting");
                    return;
                }
                tracing::info!(
                    poll_interval_secs = self.config.poll_interval_secs,
                    "[#980] QuietHoursDrainWorker started"
                );
                let mut ticker = interval(Duration::from_secs(self.config.poll_interval_secs));
                loop {
                    ticker.tick().await;
                    self.drain_due().await;
                }
            }
            .instrument(tracing::info_span!(
                "bg.quiet_hours_drain",
                poll_secs = poll_secs
            )),
        )
    }

    /// Release every held notification whose `release_at` has passed.
    async fn drain_due(&self) {
        let due = match self.granular_repo.get_notifications_to_release().await {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!(error = %e, "[#980] Failed to fetch held notifications to release");
                return;
            }
        };
        if due.is_empty() {
            return;
        }

        let mut released = 0usize;
        for held in due.into_iter().take(self.config.batch_limit) {
            // Deliver first, then decide. A held row is marked released ONLY
            // when delivery left nothing to retry (no channel reported a
            // transient `Failed`). If any channel failed we leave `released_at`
            // NULL so the next tick re-attempts, rather than permanently
            // dropping the notification on a transient failure. `deliver_held`
            // is best-effort per channel and never panics.
            let outcome = self.pipeline.deliver_held(&held).await;
            if !should_mark_released(&outcome) {
                tracing::warn!(
                    id = %held.id,
                    sent = outcome.sent,
                    failed = outcome.failed,
                    "[#980] Held notification delivery failed on ≥1 channel; leaving held for retry"
                );
                continue;
            }
            if let Err(e) = self.granular_repo.mark_notification_released(held.id).await {
                tracing::warn!(id = %held.id, error = %e, "[#980] Delivered held notification but failed to mark released; may re-deliver");
                continue;
            }
            released += 1;
            tracing::debug!(
                id = %held.id,
                sent = outcome.sent,
                skipped = outcome.skipped,
                "[#980] Released held notification"
            );
        }

        if released > 0 {
            tracing::info!(released = released, "[#980] Drained held notifications");
        }
    }
}

/// Decide whether a drained held-notification row may be marked released.
///
/// Releasing is permanent (`released_at` is set once and the row is never
/// re-fetched), so it must only happen when there is nothing left to retry.
///
/// - `failed > 0` — at least one channel hit a transient error. Return `false`
///   so the row stays held and the next tick re-attempts delivery. Releasing
///   here is the bug this guards against: a `sent == 0` transient failure would
///   otherwise mark the row released and lose the notification forever.
/// - `failed == 0` — everything was either sent or skipped (e.g. push not
///   configured, unknown user, unrecognised channel). Return `true`: the row is
///   accounted for and retrying it would loop forever with no progress.
fn should_mark_released(outcome: &PipelineResult) -> bool {
    outcome.failed == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(sent: usize, skipped: usize, failed: usize) -> PipelineResult {
        PipelineResult {
            sent,
            skipped,
            failed,
            ..PipelineResult::default()
        }
    }

    #[test]
    fn releases_when_delivered_cleanly() {
        assert!(should_mark_released(&result(1, 0, 0)));
        assert!(should_mark_released(&result(2, 1, 0)));
    }

    #[test]
    fn releases_when_fully_skipped_nothing_to_retry() {
        // e.g. push not configured / unknown user — sent==0 but no failure.
        assert!(should_mark_released(&result(0, 1, 0)));
        assert!(should_mark_released(&result(0, 0, 0)));
    }

    #[test]
    fn holds_for_retry_when_any_channel_failed() {
        // Regression: a transient failure (sent==0, failed>0) must NOT be
        // released — it would permanently drop the held notification.
        assert!(!should_mark_released(&result(0, 0, 1)));
        // Partial success with a failed channel still retries.
        assert!(!should_mark_released(&result(1, 0, 1)));
    }
}
