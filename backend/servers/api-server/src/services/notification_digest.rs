//! 24h-inactivity email digest worker — Story 2B.3 (PM #969 gap 1).
//!
//! Story 2B.3 AC1: *"users who haven't been active for 24 hours and have
//! pending notifications -> email digest is sent with a summary."* The DB write
//! path (`create_digest`) and the read API (`GET /digests`) already existed, but
//! nothing ever drove them — `notification_digests` was write-dead-code. This
//! background worker closes that gap. It mirrors the [`PushFanoutWorker`] /
//! [`QuietHoursDrainWorker`] lifecycle so the server always starts cleanly
//! (disabled → no-op) and never panics.
//!
//! Each tick the worker:
//!   1. Retries any digest rows whose email send previously failed (transient
//!      `EmailService` errors) — the row is preserved and only marked sent on
//!      success, so a digest is never lost or silently duplicated.
//!   2. Finds opted-in users (`digest_enabled`) who have been inactive past the
//!      cutoff (no refresh-token use — the closest proxy for "last seen") and
//!      have unread notification groups, aggregates per-category unread counts,
//!      persists a digest row, and emails a localized summary with an
//!      unsubscribe link.
//!
//! Quiet hours are respected: a user inside their quiet-hours window is skipped
//! this tick (no row created, no email) and naturally picked up on a later tick
//! once the window has passed.

use std::time::Duration;

use chrono::{DateTime, Utc};
use db::models::Locale;
use db::{repositories::GranularNotificationRepository, DbPool};
use tokio::time::interval;
use tracing::Instrument;

use super::email::EmailService;
use super::quiet_hours;

/// Digest period type recorded on every row. The 24h-inactivity sweep is a
/// `daily` digest (the schema CHECK allows `hourly`/`daily`/`weekly`).
const DIGEST_TYPE: &str = "daily";

/// Configuration for the inactivity digest worker.
#[derive(Debug, Clone)]
pub struct NotificationDigestConfig {
    /// When false, the worker starts but does nothing (no-op heartbeat).
    pub enabled: bool,
    /// How often to sweep for inactive users with pending notifications.
    pub poll_interval_secs: u64,
    /// A user must have been inactive for at least this long to be digested.
    pub inactivity_hours: i64,
    /// Minimum gap between digests for the same user. Also bounds how far back
    /// unsent digest rows are retried, so a failed send retries within the
    /// window but the user is never sent two digests in one window.
    pub resend_window_hours: i64,
    /// Safety cap on users processed per tick.
    pub batch_limit: i64,
}

impl Default for NotificationDigestConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            poll_interval_secs: 3600,
            inactivity_hours: 24,
            resend_window_hours: 20,
            batch_limit: 200,
        }
    }
}

impl NotificationDigestConfig {
    /// Build from environment:
    /// - `NOTIFICATION_DIGEST_ENABLED` (default `true`)
    /// - `NOTIFICATION_DIGEST_INTERVAL_SECS` (default `3600`)
    /// - `NOTIFICATION_DIGEST_INACTIVITY_HOURS` (default `24`)
    /// - `NOTIFICATION_DIGEST_RESEND_WINDOW_HOURS` (default `20`)
    /// - `NOTIFICATION_DIGEST_BATCH_LIMIT` (default `200`)
    pub fn from_env() -> Self {
        let default = Self::default();
        let enabled = std::env::var("NOTIFICATION_DIGEST_ENABLED")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(default.enabled);
        let poll_interval_secs = std::env::var("NOTIFICATION_DIGEST_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.poll_interval_secs);
        let inactivity_hours = std::env::var("NOTIFICATION_DIGEST_INACTIVITY_HOURS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.inactivity_hours);
        let resend_window_hours = std::env::var("NOTIFICATION_DIGEST_RESEND_WINDOW_HOURS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.resend_window_hours);
        let batch_limit = std::env::var("NOTIFICATION_DIGEST_BATCH_LIMIT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.batch_limit);
        Self {
            enabled,
            poll_interval_secs,
            inactivity_hours,
            resend_window_hours,
            batch_limit,
        }
    }
}

/// Background worker that emails notification digests to inactive users.
pub struct NotificationDigestWorker {
    granular_repo: GranularNotificationRepository,
    email_service: EmailService,
    config: NotificationDigestConfig,
}

impl NotificationDigestWorker {
    /// Construct the worker from shared resources.
    pub fn new(
        pool: DbPool,
        email_service: EmailService,
        config: NotificationDigestConfig,
    ) -> Self {
        let granular_repo = GranularNotificationRepository::new(pool);
        Self {
            granular_repo,
            email_service,
            config,
        }
    }

    /// Spawn the background task and return its `JoinHandle`.
    pub fn start(self) -> tokio::task::JoinHandle<()> {
        let poll_secs = self.config.poll_interval_secs;
        tokio::spawn(
            async move {
                if !self.config.enabled {
                    tracing::info!("[2B.3] NotificationDigestWorker disabled — not starting");
                    return;
                }
                tracing::info!(
                    poll_interval_secs = self.config.poll_interval_secs,
                    inactivity_hours = self.config.inactivity_hours,
                    "[2B.3] NotificationDigestWorker started"
                );
                let mut ticker = interval(Duration::from_secs(self.config.poll_interval_secs));
                loop {
                    ticker.tick().await;
                    self.process_due(Utc::now()).await;
                }
            }
            .instrument(tracing::info_span!(
                "bg.notification_digest",
                poll_secs = poll_secs
            )),
        )
    }

    /// One sweep: retry unsent digests, then build+send new ones. Best-effort;
    /// never panics so a single bad row can't take down the worker.
    async fn process_due(&self, now: DateTime<Utc>) {
        let resend_cutoff = now - chrono::Duration::hours(self.config.resend_window_hours);
        let inactivity_cutoff = now - chrono::Duration::hours(self.config.inactivity_hours);

        let retried = self.retry_unsent(resend_cutoff, now).await;
        let sent = self.send_new(inactivity_cutoff, resend_cutoff, now).await;

        if retried > 0 || sent > 0 {
            tracing::info!(
                retried = retried,
                sent = sent,
                "[2B.3] NotificationDigestWorker tick complete"
            );
        }
    }

    /// Re-attempt digest rows whose email send previously failed. Returns the
    /// number successfully sent this tick.
    async fn retry_unsent(&self, resend_cutoff: DateTime<Utc>, now: DateTime<Utc>) -> usize {
        let pending = match self
            .granular_repo
            .get_unsent_digests(resend_cutoff, self.config.batch_limit)
            .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!(error = %e, "[2B.3] Failed to fetch unsent digests for retry");
                return 0;
            }
        };

        let mut sent = 0usize;
        for d in pending {
            if self.is_in_quiet_hours(d.user_id, now).await {
                continue;
            }
            let locale = Locale::parse(&d.locale);
            match self
                .email_service
                .send_digest_email(
                    &d.email,
                    &locale,
                    d.notification_count as i64,
                    &d.category_counts,
                )
                .await
            {
                Ok(()) => {
                    if let Err(e) = self.granular_repo.mark_digest_email_sent(d.digest_id).await {
                        tracing::warn!(digest_id = %d.digest_id, error = %e, "[2B.3] Re-sent digest email but failed to mark sent; may resend");
                        continue;
                    }
                    sent += 1;
                }
                Err(e) => {
                    tracing::warn!(digest_id = %d.digest_id, error = %e, "[2B.3] Retry of digest email failed; will retry next tick");
                }
            }
        }
        sent
    }

    /// Find inactive opted-in users with pending notifications, persist a digest
    /// row, and email it. Returns the number of digests emailed this tick.
    async fn send_new(
        &self,
        inactivity_cutoff: DateTime<Utc>,
        resend_cutoff: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> usize {
        let candidates = match self
            .granular_repo
            .find_inactive_digest_candidates(
                inactivity_cutoff,
                resend_cutoff,
                self.config.batch_limit,
            )
            .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!(error = %e, "[2B.3] Failed to fetch inactivity digest candidates");
                return 0;
            }
        };

        if candidates.is_empty() {
            return 0;
        }

        let mut sent = 0usize;
        for c in candidates {
            // Respect quiet hours: defer the whole digest (no row, no email)
            // until the user is outside their quiet-hours window.
            if self.is_in_quiet_hours(c.user_id, now).await {
                continue;
            }

            // Persist the digest row first so the work is never lost; the email
            // is marked sent only on a successful send (a failure is retried in
            // place next tick by `retry_unsent`).
            let digest = match self
                .granular_repo
                .create_digest(
                    c.user_id,
                    DIGEST_TYPE,
                    c.period_start,
                    c.period_end,
                    c.notification_count.try_into().unwrap_or(i32::MAX),
                    c.category_counts.clone(),
                    None,
                    None,
                )
                .await
            {
                Ok(d) => d,
                Err(e) => {
                    tracing::error!(user_id = %c.user_id, error = %e, "[2B.3] Failed to persist digest row");
                    continue;
                }
            };

            let locale = Locale::parse(&c.locale);
            match self
                .email_service
                .send_digest_email(&c.email, &locale, c.notification_count, &c.category_counts)
                .await
            {
                Ok(()) => {
                    if let Err(e) = self.granular_repo.mark_digest_email_sent(digest.id).await {
                        tracing::warn!(digest_id = %digest.id, error = %e, "[2B.3] Sent digest email but failed to mark sent; may resend");
                        continue;
                    }
                    sent += 1;
                    tracing::debug!(user_id = %c.user_id, digest_id = %digest.id, count = c.notification_count, "[2B.3] Sent inactivity digest");
                }
                Err(e) => {
                    tracing::warn!(user_id = %c.user_id, digest_id = %digest.id, error = %e, "[2B.3] Digest email send failed; row preserved for retry");
                }
            }
        }
        sent
    }

    /// True when the user is currently inside their quiet-hours window. A user
    /// with no schedule row (cannot happen for fresh candidates, possible for
    /// retried rows after a preference change) is treated as not quiet.
    async fn is_in_quiet_hours(&self, user_id: uuid::Uuid, now: DateTime<Utc>) -> bool {
        match self.granular_repo.get_user_schedule(user_id).await {
            Ok(Some(schedule)) => quiet_hours::evaluate(&schedule, now).active,
            Ok(None) => false,
            Err(e) => {
                // On a transient read error, don't block the digest.
                tracing::warn!(user_id = %user_id, error = %e, "[2B.3] Failed to read schedule for quiet-hours check; proceeding");
                false
            }
        }
    }
}
