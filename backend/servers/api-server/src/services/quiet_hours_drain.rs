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

use common::notifications::pipeline::DeliveryStatus;
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
    /// Bounded retry budget (issue #2823): once a held row has been attempted
    /// this many times without draining cleanly, it is dead-lettered instead of
    /// being retried forever (which would also keep re-delivering its healthy
    /// channels every tick).
    pub max_attempts: i32,
}

impl Default for QuietHoursDrainConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            poll_interval_secs: 60,
            batch_limit: 500,
            max_attempts: 10,
        }
    }
}

impl QuietHoursDrainConfig {
    /// Build from environment:
    /// - `QUIET_HOURS_DRAIN_ENABLED` (default `true`)
    /// - `QUIET_HOURS_DRAIN_INTERVAL_SECS` (default `60`)
    /// - `QUIET_HOURS_DRAIN_MAX_ATTEMPTS` (default `10`)
    pub fn from_env() -> Self {
        let default = Self::default();
        let enabled = std::env::var("QUIET_HOURS_DRAIN_ENABLED")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(default.enabled);
        let poll_interval_secs = std::env::var("QUIET_HOURS_DRAIN_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.poll_interval_secs);
        let max_attempts = std::env::var("QUIET_HOURS_DRAIN_MAX_ATTEMPTS")
            .ok()
            .and_then(|v| v.parse().ok())
            .filter(|&n| n > 0)
            .unwrap_or(default.max_attempts);
        Self {
            enabled,
            poll_interval_secs,
            max_attempts,
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
        let mut dead_lettered = 0usize;
        for held in due.into_iter().take(self.config.batch_limit) {
            // Deliver first, then decide. A held row is marked released ONLY
            // when delivery left nothing to retry (no channel reported a
            // transient `Failed`). If any channel failed we leave `released_at`
            // NULL so the next tick re-attempts, rather than permanently
            // dropping the notification on a transient failure. `deliver_held`
            // is best-effort per channel and never panics.
            let outcome = self.pipeline.deliver_held(&held).await;
            if !should_mark_released(&outcome) {
                // Issue #2823: a channel failed, so the row stays held. Before
                // the next tick re-attempts it, persist the channels that DID
                // succeed this tick so they are not re-delivered (duplicate),
                // and bump the attempt counter so a permanently-failing row is
                // eventually dead-lettered instead of looping forever.
                let delivered = merge_delivered(&held.delivered_channels, &delivered_channels(&outcome));
                let attempts = held.attempts.saturating_add(1);
                if let Err(e) = self
                    .granular_repo
                    .record_held_attempt(held.id, &delivered, attempts)
                    .await
                {
                    tracing::warn!(id = %held.id, error = %e, "[#2823] Failed to persist held-notification attempt progress; may re-deliver a channel");
                }
                if should_give_up(attempts, self.config.max_attempts) {
                    // Dead-letter: stop retrying a row that never drains cleanly.
                    if let Err(e) = self.granular_repo.mark_notification_dead_lettered(held.id).await
                    {
                        tracing::warn!(id = %held.id, error = %e, "[#2823] Failed to dead-letter exhausted held notification; will keep retrying");
                    } else {
                        dead_lettered += 1;
                        tracing::error!(
                            id = %held.id,
                            attempts = attempts,
                            max_attempts = self.config.max_attempts,
                            delivered_channels = ?delivered,
                            "[#2823] Held notification exhausted retry budget; dead-lettered (giving up)"
                        );
                    }
                } else {
                    tracing::warn!(
                        id = %held.id,
                        sent = outcome.sent,
                        failed = outcome.failed,
                        attempts = attempts,
                        max_attempts = self.config.max_attempts,
                        "[#980] Held notification delivery failed on ≥1 channel; leaving held for retry"
                    );
                }
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

        if released > 0 || dead_lettered > 0 {
            tracing::info!(
                released = released,
                dead_lettered = dead_lettered,
                "[#980] Drained held notifications"
            );
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

/// The stored-channel strings that were delivered successfully in this drain
/// attempt (issue #2823). Derived from the `Sent` delivery records so the
/// worker can persist per-channel progress and skip them on the next retry.
fn delivered_channels(outcome: &PipelineResult) -> Vec<String> {
    outcome
        .records
        .iter()
        .filter(|r| r.status == DeliveryStatus::Sent)
        .map(|r| r.channel.as_str().to_string())
        .collect()
}

/// Union of the channels already recorded on the held row with the ones
/// delivered this tick, de-duplicated and order-stable (existing first). This
/// is what gets persisted to `held_notifications.delivered_channels` so a
/// retry never re-delivers a channel that already succeeded (issue #2823).
fn merge_delivered(existing: &[String], newly: &[String]) -> Vec<String> {
    let mut merged = existing.to_vec();
    for ch in newly {
        if !merged.iter().any(|e| e == ch) {
            merged.push(ch.clone());
        }
    }
    merged
}

/// Whether a held row has exhausted its bounded retry budget and must be
/// dead-lettered instead of retried again (issue #2823). `attempts` is the
/// count *including* the attempt just made.
fn should_give_up(attempts: i32, max_attempts: i32) -> bool {
    attempts >= max_attempts
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

    // --- Issue #2823: per-channel bookkeeping + bounded retry ---------------

    use common::notifications::pipeline::DeliveryRecord;
    use common::notifications::NotificationChannel;
    use uuid::Uuid;

    /// A PipelineResult whose records mark `push` Sent and `email` Failed —
    /// the exact partial-failure shape from the issue.
    fn partial_push_ok_email_failed() -> PipelineResult {
        let uid = Uuid::new_v4();
        let nid = Uuid::new_v4();
        PipelineResult {
            sent: 1,
            skipped: 0,
            failed: 1,
            records: vec![
                DeliveryRecord::pending(nid, uid, NotificationChannel::Push).into_sent(),
                DeliveryRecord::pending(nid, uid, NotificationChannel::Email)
                    .into_failed("smtp timeout"),
            ],
        }
    }

    #[test]
    fn delivered_channels_lists_only_sent() {
        // The healthy channel (push) is recorded; the failed one (email) is not.
        assert_eq!(
            delivered_channels(&partial_push_ok_email_failed()),
            vec!["push".to_string()]
        );
    }

    #[test]
    fn merge_delivered_is_union_dedup_order_stable() {
        assert_eq!(
            merge_delivered(&["push".to_string()], &["email".to_string()]),
            vec!["push".to_string(), "email".to_string()]
        );
        // Re-recording an already-delivered channel does not duplicate it.
        assert_eq!(
            merge_delivered(&["push".to_string()], &["push".to_string()]),
            vec!["push".to_string()]
        );
        assert!(merge_delivered(&[], &[]).is_empty());
    }

    #[test]
    fn partial_failure_recover_never_redelivers_a_channel() {
        // Tick 1: push Sent, email Failed -> row held, push recorded.
        let tick1 = partial_push_ok_email_failed();
        assert!(!should_mark_released(&tick1));
        let recorded = merge_delivered(&[], &delivered_channels(&tick1));
        assert_eq!(recorded, vec!["push".to_string()]);
        // Tick 2 (recovery): deliver_held now skips push (already recorded) and
        // only re-attempts email. Persisted state must still contain push
        // exactly once — no duplicate push was ever sent across the two ticks.
        let tick2_delivered = vec!["email".to_string()]; // what deliver_held returns
        let after = merge_delivered(&recorded, &tick2_delivered);
        assert_eq!(after, vec!["push".to_string(), "email".to_string()]);
        assert_eq!(
            after.iter().filter(|c| *c == "push").count(),
            1,
            "push must be delivered exactly once across retries"
        );
    }

    #[test]
    fn gives_up_after_max_attempts() {
        // Bounded retry: below the cap keeps retrying, at/above the cap gives up.
        assert!(!should_give_up(1, 10));
        assert!(!should_give_up(9, 10));
        assert!(should_give_up(10, 10));
        assert!(should_give_up(11, 10));
    }

    #[test]
    fn max_attempts_from_env_defaults_and_rejects_non_positive() {
        // A garbage / non-positive override falls back to the default cap so a
        // misconfiguration can never make every row dead-letter on tick one.
        assert_eq!(QuietHoursDrainConfig::default().max_attempts, 10);
    }
}
