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
//!   3. Marks any digest that has exhausted the retry window as abandoned: a
//!      row whose email send kept failing drops out of step 1 once it ages past
//!      the resend window, so it would otherwise be silently given up on. Each
//!      such row is stamped with a durable `abandoned_at` timestamp (a
//!      queryable terminal state) and logged once at error level, so a
//!      persistent send failure is both recorded in the table and never dropped
//!      without a trace.
//!
//! Quiet hours are respected: a user inside their quiet-hours window is skipped
//! this tick (no row created, no email) and naturally picked up on a later tick
//! once the window has passed.
//!
//! The worker runs in every api-server replica, so each sweep is guarded by a
//! cross-replica Postgres advisory lock (`notification_digest_sweep`): exactly
//! one replica runs the sweep per tick and peers skip, so digests are never
//! duplicated (duplicate rows or duplicate emails) when more than one replica
//! is deployed. See [`GranularNotificationRepository::try_claim_digest_sweep`].

use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use db::models::Locale;
use db::{repositories::GranularNotificationRepository, DbPool};
use metrics::counter;
use tokio::time::interval;
use tracing::Instrument;

use super::email::EmailService;
use super::quiet_hours;

/// Digest period type recorded on every row. The 24h-inactivity sweep is a
/// `daily` digest (the schema CHECK allows `hourly`/`daily`/`weekly`).
const DIGEST_TYPE: &str = "daily";

// --- Digest sweep KPI metric names (Prometheus counters) -------------------
//
// One flat `notification_digest_*_total` family, matching the repo's
// `metrics`-crate convention (snake_case, `_total` suffix — see
// `observability.rs`). Each is incremented once per claimed sweep with that
// sweep's tally, so rate() over the series yields digests/sec by outcome.

/// Sweeps actually run by this replica (the replica that won the advisory lock).
const METRIC_SWEEPS: &str = "notification_digest_sweeps_total";
/// Candidate digest rows the sweep looked at (retry rows + new candidates).
const METRIC_CANDIDATES: &str = "notification_digest_candidates_considered_total";
/// Digests successfully emailed and durably marked sent.
const METRIC_SENT: &str = "notification_digest_sent_total";
/// Email send (or sent-but-mark) attempts that failed — retried or abandoned.
const METRIC_FAILED: &str = "notification_digest_failed_total";
/// Candidates skipped by a filter (quiet hours) and deferred to a later tick.
const METRIC_EXCLUDED: &str = "notification_digest_excluded_by_filter_total";
/// Digest rows newly marked abandoned (exhausted the retry window).
const METRIC_ABANDONED: &str = "notification_digest_abandoned_total";

/// Per-sweep KPI tally for the digest worker.
///
/// Filled as the sweep processes rows, then emitted exactly once per claimed
/// sweep — both as a single structured summary log line and as the
/// `notification_digest_*_total` Prometheus counters. The send-path outcomes
/// are mutually exclusive and recorded through the `record_*` helpers, so the
/// invariant `candidates_considered == sent + failed + excluded_by_filter`
/// always holds by construction. `abandoned` is a separate terminal-state
/// tally (the aged-out retry window), independent of the candidates looked at
/// this tick, so it is not folded into `candidates_considered`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct SweepCounters {
    /// Send-path candidate rows looked at this tick (retry rows + new candidates).
    candidates_considered: usize,
    /// Digests successfully emailed and marked sent this tick.
    sent: usize,
    /// Send attempts that failed this tick (send error, persist error, or a
    /// sent-but-not-marked row that will be retried).
    failed: usize,
    /// Candidates skipped by the quiet-hours filter (deferred, not sent).
    excluded_by_filter: usize,
    /// Digest rows newly marked abandoned this tick (exhausted the retry window).
    abandoned: usize,
}

impl SweepCounters {
    /// A considered candidate that was emailed and durably marked sent.
    fn record_sent(&mut self) {
        self.candidates_considered += 1;
        self.sent += 1;
    }

    /// A considered candidate whose send attempt failed (will retry/abandon).
    fn record_failed(&mut self) {
        self.candidates_considered += 1;
        self.failed += 1;
    }

    /// A considered candidate skipped by a filter (quiet hours) this tick.
    fn record_excluded(&mut self) {
        self.candidates_considered += 1;
        self.excluded_by_filter += 1;
    }

    /// Fold another phase's tally (e.g. the retry phase into the sweep total).
    fn merge(&mut self, other: SweepCounters) {
        self.candidates_considered += other.candidates_considered;
        self.sent += other.sent;
        self.failed += other.failed;
        self.excluded_by_filter += other.excluded_by_filter;
        self.abandoned += other.abandoned;
    }

    /// True when the sweep did anything worth an info-level summary beyond the
    /// always-on metrics (keeps idle ticks quiet in the logs).
    fn did_work(&self) -> bool {
        self.candidates_considered > 0 || self.abandoned > 0
    }

    /// Emit the per-sweep KPI counters. Called once per claimed sweep so the
    /// Prometheus series always exist after the first sweep (a 0 increment
    /// registers the series without perturbing the rate).
    fn emit_metrics(&self) {
        counter!(METRIC_SWEEPS).increment(1);
        counter!(METRIC_CANDIDATES).increment(self.candidates_considered as u64);
        counter!(METRIC_SENT).increment(self.sent as u64);
        counter!(METRIC_FAILED).increment(self.failed as u64);
        counter!(METRIC_EXCLUDED).increment(self.excluded_by_filter as u64);
        counter!(METRIC_ABANDONED).increment(self.abandoned as u64);
    }
}

/// Pre-register the digest sweep counters with the Prometheus exporter so they
/// carry help text and appear in `/metrics` before the first sweep increments
/// them. Called once when the worker starts (matches the `describe_*` pattern
/// used in `observability.rs` / `rag_metrics.rs`).
fn describe_sweep_metrics() {
    metrics::describe_counter!(METRIC_SWEEPS, "Digest sweeps run by this replica");
    metrics::describe_counter!(
        METRIC_CANDIDATES,
        "Digest candidate rows considered per sweep (retry rows + new candidates)"
    );
    metrics::describe_counter!(METRIC_SENT, "Digest emails successfully sent");
    metrics::describe_counter!(METRIC_FAILED, "Digest send attempts that failed");
    metrics::describe_counter!(
        METRIC_EXCLUDED,
        "Digest candidates excluded by a filter (quiet hours)"
    );
    metrics::describe_counter!(
        METRIC_ABANDONED,
        "Digest rows marked abandoned after exhausting the retry window"
    );
}

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
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    /// Pure core of [`Self::from_env`]: build the config from an arbitrary
    /// key→value `lookup` instead of the process environment. Extracted so the
    /// parse/default/fallback semantics are unit-testable without mutating
    /// process-global env state (which races across parallel tests). `from_env`
    /// is the thin `std::env::var` adapter over this; keep the two in lockstep.
    fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Self {
        let default = Self::default();
        let enabled = lookup("NOTIFICATION_DIGEST_ENABLED")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(default.enabled);
        let poll_interval_secs = lookup("NOTIFICATION_DIGEST_INTERVAL_SECS")
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.poll_interval_secs);
        let inactivity_hours = lookup("NOTIFICATION_DIGEST_INACTIVITY_HOURS")
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.inactivity_hours);
        let resend_window_hours = lookup("NOTIFICATION_DIGEST_RESEND_WINDOW_HOURS")
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.resend_window_hours);
        let batch_limit = lookup("NOTIFICATION_DIGEST_BATCH_LIMIT")
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
                describe_sweep_metrics();
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

    /// One tick. The worker runs in every api-server replica, so the sweep is
    /// guarded by a cross-replica advisory lock: without it every replica would
    /// independently select the same candidates/unsent rows and send duplicate
    /// digest emails (and `create_digest` would insert duplicate rows). Exactly
    /// one replica claims the lock and runs [`Self::run_sweep`]; peers that fail
    /// to claim skip this tick and retry on their next one.
    async fn process_due(&self, now: DateTime<Utc>) {
        let lock_conn = match self.granular_repo.try_claim_digest_sweep().await {
            Ok(Some(conn)) => conn,
            Ok(None) => {
                tracing::debug!(
                    "[2B.3] Digest sweep already running on another replica; skipping tick"
                );
                return;
            }
            Err(e) => {
                tracing::error!(error = %e, "[2B.3] Failed to claim digest sweep lock; skipping tick");
                return;
            }
        };

        self.run_sweep(now).await;

        // Release the session-scoped advisory lock. On failure
        // `release_digest_sweep` discards (detaches + closes) the connection
        // instead of returning a possibly-still-locked session to the pool, so
        // the lock is dropped with the torn-down session rather than stranded —
        // a failure here only delays the next tick's eligibility, never
        // deadlocks the sweep.
        if let Err(e) = self.granular_repo.release_digest_sweep(lock_conn).await {
            tracing::warn!(error = %e, "[2B.3] Failed to release digest sweep lock; connection discarded so the lock drops with the closed session");
        }
    }

    /// The claimed sweep: retry unsent digests, then build+send new ones.
    /// Best-effort; never panics so a single bad row can't take down the worker.
    /// Only ever called by the replica that won [`Self::process_due`]'s lock.
    ///
    /// Tallies the per-sweep KPIs across every phase and emits them exactly
    /// once — as the `notification_digest_*_total` Prometheus counters (always)
    /// and as a single structured summary log line (when the sweep did work).
    async fn run_sweep(&self, now: DateTime<Utc>) {
        let started = Instant::now();
        let resend_cutoff = now - chrono::Duration::hours(self.config.resend_window_hours);
        let inactivity_cutoff = now - chrono::Duration::hours(self.config.inactivity_hours);

        let mut counters = SweepCounters::default();
        counters.merge(self.retry_unsent(resend_cutoff, now).await);
        counters.merge(self.send_new(inactivity_cutoff, resend_cutoff, now).await);
        counters.abandoned += self.report_abandoned(resend_cutoff).await;

        let elapsed_ms = started.elapsed().as_millis();

        // Always export the counters so the Prometheus series stay live even on
        // idle sweeps; only log a summary line when there was something to say.
        counters.emit_metrics();
        if counters.did_work() {
            tracing::info!(
                candidates_considered = counters.candidates_considered,
                sent = counters.sent,
                failed = counters.failed,
                excluded_by_filter = counters.excluded_by_filter,
                abandoned = counters.abandoned,
                elapsed_ms = elapsed_ms as u64,
                "[2B.3] NotificationDigestWorker sweep complete"
            );
        }
    }

    /// Record digests that have exhausted the retry window as abandoned. A
    /// digest whose email send kept failing stops being picked up by
    /// [`Self::retry_unsent`] once it ages past the resend window; without this
    /// it would be silently given up on — never retried and never recorded. We
    /// stamp `abandoned_at` on each newly aged-out row, turning abandonment into
    /// a durable, queryable terminal state (not just a log line), and log each
    /// one once at error level so operators can act on the permanent failure.
    /// The DB mark is idempotent (only rows with `abandoned_at IS NULL` are
    /// stamped), so a row is recorded and logged exactly once even across
    /// skipped ticks. Returns the number newly abandoned. Best-effort: a query
    /// error is logged, not propagated.
    async fn report_abandoned(&self, resend_cutoff: DateTime<Utc>) -> usize {
        // Any unsent, not-yet-abandoned row created at or before the resend
        // cutoff has aged out of the retry window. `abandoned_at IS NULL` in the
        // repo query makes this mark-once, so we no longer need a tick-boundary
        // window to avoid re-reporting.
        match self
            .granular_repo
            .mark_newly_abandoned_digests(resend_cutoff, self.config.batch_limit)
            .await
        {
            Ok(ids) => {
                if !ids.is_empty() {
                    tracing::error!(
                        count = ids.len(),
                        digest_ids = ?ids,
                        resend_window_hours = self.config.resend_window_hours,
                        "[2B.3] Digest email permanently failed — exhausted the retry window and marked abandoned; manual follow-up may be required"
                    );
                }
                ids.len()
            }
            Err(e) => {
                tracing::warn!(error = %e, "[2B.3] Failed to mark abandoned digests");
                0
            }
        }
    }

    /// Re-attempt digest rows whose email send previously failed. Returns this
    /// phase's KPI tally (candidates looked at, sent, failed, excluded).
    async fn retry_unsent(
        &self,
        resend_cutoff: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> SweepCounters {
        let mut counters = SweepCounters::default();
        let pending = match self
            .granular_repo
            .get_unsent_digests(resend_cutoff, self.config.batch_limit)
            .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!(error = %e, "[2B.3] Failed to fetch unsent digests for retry");
                return counters;
            }
        };

        for d in pending {
            if self.is_in_quiet_hours(d.user_id, now).await {
                counters.record_excluded();
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
                        counters.record_failed();
                        continue;
                    }
                    counters.record_sent();
                }
                Err(e) => {
                    tracing::warn!(digest_id = %d.digest_id, error = %e, "[2B.3] Retry of digest email failed; will retry next tick");
                    counters.record_failed();
                }
            }
        }
        counters
    }

    /// Find inactive opted-in users with pending notifications, persist a digest
    /// row, and email it. Returns this phase's KPI tally (candidates looked at,
    /// sent, failed, excluded).
    async fn send_new(
        &self,
        inactivity_cutoff: DateTime<Utc>,
        resend_cutoff: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> SweepCounters {
        let mut counters = SweepCounters::default();
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
                return counters;
            }
        };

        for c in candidates {
            // Respect quiet hours: defer the whole digest (no row, no email)
            // until the user is outside their quiet-hours window.
            if self.is_in_quiet_hours(c.user_id, now).await {
                counters.record_excluded();
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
                    counters.record_failed();
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
                        counters.record_failed();
                        continue;
                    }
                    counters.record_sent();
                    tracing::debug!(user_id = %c.user_id, digest_id = %digest.id, count = c.notification_count, "[2B.3] Sent inactivity digest");
                }
                Err(e) => {
                    tracing::warn!(user_id = %c.user_id, digest_id = %digest.id, error = %e, "[2B.3] Digest email send failed; row preserved for retry");
                    counters.record_failed();
                }
            }
        }
        counters
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a `from_lookup`-compatible closure from a fixed set of key/value
    /// pairs — a deterministic stand-in for the process environment so the
    /// config-parsing semantics are testable without mutating global env state.
    fn lookup_from(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let owned: std::collections::HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        move |key: &str| owned.get(key).cloned()
    }

    /// The compiled-in defaults are churn-prone magic numbers (24h inactivity,
    /// 20h resend window, hourly poll, 200/batch). Pin them so an accidental
    /// edit to the `Default` impl is caught, and confirm an empty lookup yields
    /// exactly the defaults (every var falls back).
    #[test]
    fn from_lookup_empty_yields_defaults() {
        let parsed = NotificationDigestConfig::from_lookup(lookup_from(&[]));
        let default = NotificationDigestConfig::default();

        assert!(default.enabled);
        assert_eq!(default.poll_interval_secs, 3600);
        assert_eq!(default.inactivity_hours, 24);
        assert_eq!(default.resend_window_hours, 20);
        assert_eq!(default.batch_limit, 200);

        assert_eq!(parsed.enabled, default.enabled);
        assert_eq!(parsed.poll_interval_secs, default.poll_interval_secs);
        assert_eq!(parsed.inactivity_hours, default.inactivity_hours);
        assert_eq!(parsed.resend_window_hours, default.resend_window_hours);
        assert_eq!(parsed.batch_limit, default.batch_limit);
    }

    /// `enabled` is intentionally permissive: ONLY the literals `"false"` and
    /// `"0"` disable the worker; every other value (including typos and `"no"`)
    /// leaves it enabled, and a missing var keeps the default. Characterizes the
    /// exact fail-open semantics so a future "tidy-up" can't silently flip them.
    #[test]
    fn enabled_flag_only_false_or_zero_disables() {
        let dis = |v: &str| {
            NotificationDigestConfig::from_lookup(lookup_from(&[(
                "NOTIFICATION_DIGEST_ENABLED",
                v,
            )]))
            .enabled
        };
        assert!(!dis("false"), "\"false\" disables");
        assert!(!dis("0"), "\"0\" disables");
        assert!(dis("true"), "\"true\" stays enabled");
        assert!(dis("1"), "\"1\" stays enabled");
        assert!(dis(""), "empty string is not a disable token");
        assert!(dis("no"), "\"no\" is NOT a disable token (fail-open)");
        assert!(dis("False"), "case-sensitive: \"False\" does not disable");
        // Missing var → compiled-in default (enabled).
        assert!(NotificationDigestConfig::from_lookup(lookup_from(&[])).enabled);
    }

    /// Numeric vars parse when valid and fall back to the default when the value
    /// is unparsable (empty, non-numeric, or wrong sign for the unsigned field)
    /// — the worker must never fail to start over a malformed env value.
    #[test]
    fn numeric_vars_parse_or_fall_back() {
        let default = NotificationDigestConfig::default();

        // All valid → all taken.
        let parsed = NotificationDigestConfig::from_lookup(lookup_from(&[
            ("NOTIFICATION_DIGEST_INTERVAL_SECS", "60"),
            ("NOTIFICATION_DIGEST_INACTIVITY_HOURS", "48"),
            ("NOTIFICATION_DIGEST_RESEND_WINDOW_HOURS", "12"),
            ("NOTIFICATION_DIGEST_BATCH_LIMIT", "10"),
        ]));
        assert_eq!(parsed.poll_interval_secs, 60);
        assert_eq!(parsed.inactivity_hours, 48);
        assert_eq!(parsed.resend_window_hours, 12);
        assert_eq!(parsed.batch_limit, 10);

        // Garbage / wrong-type → each field independently falls back.
        let parsed = NotificationDigestConfig::from_lookup(lookup_from(&[
            ("NOTIFICATION_DIGEST_INTERVAL_SECS", "-5"), // negative into u64 → fail
            ("NOTIFICATION_DIGEST_INACTIVITY_HOURS", "abc"),
            ("NOTIFICATION_DIGEST_RESEND_WINDOW_HOURS", ""),
            ("NOTIFICATION_DIGEST_BATCH_LIMIT", "3.5"),
        ]));
        assert_eq!(parsed.poll_interval_secs, default.poll_interval_secs);
        assert_eq!(parsed.inactivity_hours, default.inactivity_hours);
        assert_eq!(parsed.resend_window_hours, default.resend_window_hours);
        assert_eq!(parsed.batch_limit, default.batch_limit);
    }

    /// Every send-path outcome bumps `candidates_considered`, so the KPI
    /// invariant `candidates_considered == sent + failed + excluded_by_filter`
    /// holds by construction — the guarantee the summary line and dashboards
    /// rely on. `abandoned` is a separate terminal tally and must NOT be folded
    /// into candidates.
    #[test]
    fn counters_preserve_send_path_invariant() {
        let mut c = SweepCounters::default();
        c.record_sent();
        c.record_sent();
        c.record_failed();
        c.record_excluded();
        c.record_excluded();
        c.record_excluded();
        c.abandoned += 4; // terminal state, independent of the send path

        assert_eq!(c.sent, 2);
        assert_eq!(c.failed, 1);
        assert_eq!(c.excluded_by_filter, 3);
        assert_eq!(
            c.candidates_considered,
            c.sent + c.failed + c.excluded_by_filter,
            "candidates_considered must equal the sum of send-path outcomes"
        );
        assert_eq!(c.candidates_considered, 6);
        assert_eq!(c.abandoned, 4, "abandoned is not folded into candidates");
    }

    /// `merge` folds two phases' tallies field-wise, so the sweep total is the
    /// retry phase plus the new-send phase. Regression guard: an off-by-one or
    /// dropped field here silently corrupts every per-sweep KPI.
    #[test]
    fn merge_sums_all_fields() {
        let mut total = SweepCounters {
            candidates_considered: 3,
            sent: 2,
            failed: 1,
            excluded_by_filter: 0,
            abandoned: 0,
        };
        total.merge(SweepCounters {
            candidates_considered: 5,
            sent: 1,
            failed: 2,
            excluded_by_filter: 2,
            abandoned: 7,
        });

        assert_eq!(
            total,
            SweepCounters {
                candidates_considered: 8,
                sent: 3,
                failed: 3,
                excluded_by_filter: 2,
                abandoned: 7,
            }
        );
    }

    /// `did_work` gates the info-level summary line: idle sweeps (no candidates,
    /// nothing abandoned) stay quiet, while any considered candidate or
    /// abandonment produces the summary.
    #[test]
    fn did_work_reflects_activity() {
        assert!(!SweepCounters::default().did_work());

        let mut only_excluded = SweepCounters::default();
        only_excluded.record_excluded();
        assert!(
            only_excluded.did_work(),
            "an excluded candidate is still activity"
        );

        let only_abandoned = SweepCounters {
            abandoned: 1,
            ..Default::default()
        };
        assert!(
            only_abandoned.did_work(),
            "an abandonment alone warrants the summary"
        );
    }
}
