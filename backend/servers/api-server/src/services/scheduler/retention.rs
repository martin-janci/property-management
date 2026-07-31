//! Retention/prune scheduled jobs for the background [`Scheduler`].
//!
//! Extracted verbatim from `scheduler/mod.rs` to keep the core tick loop
//! readable (churn-reduction refactor — no behaviour change). Holds the daily
//! append-only audit-trail prunes:
//!
//! - `support_tooling_events` retention prune (issue #2531)
//! - `layout_change_events` retention prune (issue #2563)
//!
//! Each prune is gated to a daily cadence even though the scheduler ticks every
//! ~60s. The jobs are attached to `Scheduler` via the `impl` block below and
//! are invoked from `Scheduler::run_scheduled_tasks` in the parent module.

use super::Scheduler;

/// Minimum interval between successive `support_tooling_events` retention
/// prunes (issue #2531). The scheduler ticks every ~60s, but the append-only
/// admin-audit prune only needs a daily cadence — this gate skips the prune
/// until 24h have elapsed since the last successful run.
const SUPPORT_TOOLING_PRUNE_MIN_INTERVAL_HOURS: i64 = 24;

/// Minimum interval between successive `layout_change_events` retention prunes
/// (issue #2563). Mirrors the `support_tooling_events` cadence: the scheduler
/// ticks every ~60s, but the append-only layout-publish trail only needs a
/// daily prune — this gate skips the prune until 24h have elapsed since the
/// last successful run.
const LAYOUT_CHANGE_EVENTS_PRUNE_MIN_INTERVAL_HOURS: i64 = 24;

impl Scheduler {
    // ========================================================================
    // Issue #2531: support_tooling_events retention prune (daily cadence)
    // ========================================================================

    /// Prune aged rows from the append-only `support_tooling_events` admin-audit
    /// trail past the configured retention window (issue #2531).
    ///
    /// The prune calls `PlatformAdminRepository::cleanup_old_support_tooling_events`,
    /// which routes through the sanctioned `app.retention_prune` GUC path so the
    /// table's immutability trigger permits these — and only these — deletes
    /// (migration `00222_support_tooling_events_retention.sql`). The repository
    /// method existed but nothing invoked it, so the audit trail grew without
    /// bound; this tick wires it into the scheduler, mirroring `cleanup_sessions`.
    ///
    /// Cadence: the scheduler ticks every ~60s, but retention only needs to run
    /// once per day. A stored last-run timestamp gates the prune to at most one
    /// run per [`SUPPORT_TOOLING_PRUNE_MIN_INTERVAL_HOURS`] window. The stamp is
    /// written only after a successful prune, so a failed run retries next tick.
    pub(super) async fn prune_support_tooling_events(&self) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now();
        let min_interval = chrono::Duration::hours(SUPPORT_TOOLING_PRUNE_MIN_INTERVAL_HOURS);

        // Daily gate — skip until 24h have elapsed since the last prune.
        {
            let last = self
                .last_support_tooling_prune_at
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if !support_tooling_prune_due(*last, now, min_interval) {
                return Ok(());
            }
        }

        // Note: Scheduler runs in background without user context. This prune is
        // a privileged/admin-level retention job and does not need RLS context.
        let deleted = self
            .platform_admin_repo
            .cleanup_old_support_tooling_events(self.config.support_tooling_retention_days as i32)
            .await?;

        // Stamp only after a successful prune so a transient DB error retries on
        // the next tick instead of being suppressed for a full day.
        {
            let mut last = self
                .last_support_tooling_prune_at
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            *last = Some(now);
        }

        if deleted > 0 {
            tracing::info!(
                deleted = deleted,
                retention_days = self.config.support_tooling_retention_days,
                "Pruned aged support_tooling_events"
            );
            let mut metrics = self.metrics.lock().unwrap_or_else(|e| e.into_inner());
            metrics.support_tooling_events_pruned += deleted as u64;
        }

        Ok(())
    }

    // ========================================================================
    // Issue #2563: layout_change_events retention prune (daily cadence)
    // ========================================================================

    /// Prune aged rows from the append-only `layout_change_events` layout-publish
    /// trail past the configured retention window (issue #2563).
    ///
    /// The prune calls `LayoutRepository::cleanup_old_layout_change_events`,
    /// which routes through the sanctioned SQL `cleanup_old_layout_change_events`
    /// function so the table's immutability trigger permits these — and only
    /// these — deletes (migration `00225_create_layout_change_events.sql`). The
    /// repository method existed but nothing invoked it, so the audit trail grew
    /// without bound; this tick wires it into the scheduler, mirroring
    /// [`Scheduler::prune_support_tooling_events`] (issue #2531).
    ///
    /// Cadence: the scheduler ticks every ~60s, but retention only needs to run
    /// once per day. A stored last-run timestamp gates the prune to at most one
    /// run per [`LAYOUT_CHANGE_EVENTS_PRUNE_MIN_INTERVAL_HOURS`] window. The
    /// stamp is written only after a successful prune, so a failed run retries
    /// next tick.
    pub(super) async fn prune_layout_change_events(&self) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now();
        let min_interval = chrono::Duration::hours(LAYOUT_CHANGE_EVENTS_PRUNE_MIN_INTERVAL_HOURS);

        // Daily gate — skip until 24h have elapsed since the last prune.
        {
            let last = self
                .last_layout_change_events_prune_at
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if !layout_change_events_prune_due(*last, now, min_interval) {
                return Ok(());
            }
        }

        // Note: Scheduler runs in background without user context. This prune is
        // a privileged retention job and does not need RLS context.
        let deleted = self
            .layout_repo
            .cleanup_old_layout_change_events(
                &self.pool,
                self.config.layout_change_events_retention_days as i32,
            )
            .await?;

        // Stamp only after a successful prune so a transient DB error retries on
        // the next tick instead of being suppressed for a full day.
        {
            let mut last = self
                .last_layout_change_events_prune_at
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            *last = Some(now);
        }

        if deleted > 0 {
            tracing::info!(
                deleted = deleted,
                retention_days = self.config.layout_change_events_retention_days,
                "Pruned aged layout_change_events"
            );
            let mut metrics = self.metrics.lock().unwrap_or_else(|e| e.into_inner());
            metrics.layout_change_events_pruned += deleted as u64;
        }

        Ok(())
    }
}

/// Decide whether the daily `support_tooling_events` retention prune is due
/// (issue #2531). Returns `true` on the first run (`last` is `None`) or once at
/// least `min_interval` has elapsed since the last successful prune. Extracted
/// as a pure function so the daily-cadence gate is testable without a DB or a
/// live clock.
fn support_tooling_prune_due(
    last: Option<chrono::DateTime<chrono::Utc>>,
    now: chrono::DateTime<chrono::Utc>,
    min_interval: chrono::Duration,
) -> bool {
    match last {
        None => true,
        Some(t) => now.signed_duration_since(t) >= min_interval,
    }
}

/// Decide whether the daily `layout_change_events` retention prune is due
/// (issue #2563). Returns `true` on the first run (`last` is `None`) or once at
/// least `min_interval` has elapsed since the last successful prune. Extracted
/// as a pure function so the daily-cadence gate is testable without a DB or a
/// live clock.
fn layout_change_events_prune_due(
    last: Option<chrono::DateTime<chrono::Utc>>,
    now: chrono::DateTime<chrono::Utc>,
    min_interval: chrono::Duration,
) -> bool {
    match last {
        None => true,
        Some(t) => now.signed_duration_since(t) >= min_interval,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Issue #2531: daily-cadence gate for the support_tooling_events prune.
    //
    // The scheduler ticks every ~60s but the retention prune must run at
    // most once per day. These pin the gate that stops the ~60s tick loop
    // from re-pruning on every tick — the behaviour that distinguishes the
    // fix (bounded daily prune) from a naive "prune every tick" wiring.
    // ------------------------------------------------------------------

    #[test]
    fn test_support_tooling_prune_due_on_first_run() {
        // Never pruned before → due immediately.
        let now = chrono::Utc::now();
        let min_interval = chrono::Duration::hours(SUPPORT_TOOLING_PRUNE_MIN_INTERVAL_HOURS);
        assert!(support_tooling_prune_due(None, now, min_interval));
    }

    #[test]
    fn test_support_tooling_prune_skipped_within_interval() {
        // Pruned 1h ago, interval is 24h → NOT due (the every-60s tick must
        // not re-prune).
        let now = chrono::Utc::now();
        let min_interval = chrono::Duration::hours(SUPPORT_TOOLING_PRUNE_MIN_INTERVAL_HOURS);
        let last = now - chrono::Duration::hours(1);
        assert!(!support_tooling_prune_due(Some(last), now, min_interval));
    }

    #[test]
    fn test_support_tooling_prune_due_after_interval() {
        // Pruned 25h ago, interval is 24h → due again.
        let now = chrono::Utc::now();
        let min_interval = chrono::Duration::hours(SUPPORT_TOOLING_PRUNE_MIN_INTERVAL_HOURS);
        let last = now - chrono::Duration::hours(25);
        assert!(support_tooling_prune_due(Some(last), now, min_interval));
    }

    #[test]
    fn test_support_tooling_prune_due_at_exact_boundary() {
        // Exactly at the interval boundary → due (>= comparison).
        let now = chrono::Utc::now();
        let min_interval = chrono::Duration::hours(SUPPORT_TOOLING_PRUNE_MIN_INTERVAL_HOURS);
        let last = now - min_interval;
        assert!(support_tooling_prune_due(Some(last), now, min_interval));
    }

    // ------------------------------------------------------------------
    // Issue #2563: daily-cadence gate for the layout_change_events prune.
    //
    // The scheduler ticks every ~60s but the retention prune must run at
    // most once per day. These pin the gate that stops the ~60s tick loop
    // from re-pruning on every tick — the behaviour that distinguishes the
    // fix (bounded daily prune) from a naive "prune every tick" wiring.
    // ------------------------------------------------------------------

    #[test]
    fn test_layout_change_events_prune_due_on_first_run() {
        // Never pruned before → due immediately.
        let now = chrono::Utc::now();
        let min_interval = chrono::Duration::hours(LAYOUT_CHANGE_EVENTS_PRUNE_MIN_INTERVAL_HOURS);
        assert!(layout_change_events_prune_due(None, now, min_interval));
    }

    #[test]
    fn test_layout_change_events_prune_skipped_within_interval() {
        // Pruned 1h ago, interval is 24h → NOT due (the every-60s tick must
        // not re-prune).
        let now = chrono::Utc::now();
        let min_interval = chrono::Duration::hours(LAYOUT_CHANGE_EVENTS_PRUNE_MIN_INTERVAL_HOURS);
        let last = now - chrono::Duration::hours(1);
        assert!(!layout_change_events_prune_due(
            Some(last),
            now,
            min_interval
        ));
    }

    #[test]
    fn test_layout_change_events_prune_due_after_interval() {
        // Pruned 25h ago, interval is 24h → due again.
        let now = chrono::Utc::now();
        let min_interval = chrono::Duration::hours(LAYOUT_CHANGE_EVENTS_PRUNE_MIN_INTERVAL_HOURS);
        let last = now - chrono::Duration::hours(25);
        assert!(layout_change_events_prune_due(
            Some(last),
            now,
            min_interval
        ));
    }

    #[test]
    fn test_layout_change_events_prune_due_at_exact_boundary() {
        // Exactly at the interval boundary → due (>= comparison).
        let now = chrono::Utc::now();
        let min_interval = chrono::Duration::hours(LAYOUT_CHANGE_EVENTS_PRUNE_MIN_INTERVAL_HOURS);
        let last = now - min_interval;
        assert!(layout_change_events_prune_due(
            Some(last),
            now,
            min_interval
        ));
    }
}
