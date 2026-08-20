//! Saved-search alert matching engine — Story 16.3 (issue #983).
//!
//! Before this worker, saved-search matching only ran on demand
//! (`POST /saved-searches/{id}/run`); nothing iterated alert-enabled searches
//! against newly published listings, so "new listing matches your search →
//! you're notified" was unimplemented. This background worker periodically:
//!
//! 1. opens a connection in **global-read** RLS context (so it can see
//!    published listings across all orgs — the same context the public portal
//!    request path uses),
//! 2. for each alert-enabled saved search, finds listings published since the
//!    search's `last_matched_at` watermark that match its criteria,
//!    3. enqueues a pending row in `search_alert_queue` and advances the
//!    watermark (`last_matched_at`, `match_count`).
//!
//! On a search's first sighting (no watermark) it only sets the watermark — it
//! does not alert on the entire back-catalogue.
//!
//! Cadence (Story 16.3 follow-up): each search carries an `alert_frequency`
//! (`instant` / `daily` / `weekly`). The worker still polls on a fixed interval,
//! but only *runs matching* for a search once its frequency window has elapsed
//! since the last match run (`last_matched_at`): `instant` every poll, `daily`
//! at most once per 24 h, `weekly` once per 7 days. After a due run the watermark
//! always advances — even when nothing matched — so the next run starts a fresh
//! window (otherwise a no-match daily search would rescan on every poll and the
//! cadence would have no effect). An unrecognised frequency falls back to daily.
//!
//! Delivery (#983): queued rows are surfaced to the owning portal user as an
//! in-app feed via `GET /api/v1/saved-searches/alerts` (+ `…/{id}/read` and
//! `…/read-all`) — see `routes::saved_searches`. reality-server has no email
//! transport, so in-app pull is the delivery channel; an email/push drainer off
//! the same queue remains a future follow-up.
//!
//! Favorite price-drop / back-on-market delivery now lives in
//! [`crate::services::favorite_alerts`] because those reads must run in an
//! explicit per-org RLS loop rather than the global-read context used here.

use std::time::Duration;

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use db::models::PublicListingQuery;
use db::{repositories::RealityPortalRepository, DbPool};
use tokio::time::interval;
use tracing::Instrument;

/// Minimum spacing between match runs for a saved search's `alert_frequency`.
/// Unknown/missing frequencies fall back to `daily` (the create-time default).
fn min_match_interval(frequency: &str) -> ChronoDuration {
    match frequency {
        "instant" => ChronoDuration::zero(),
        "weekly" => ChronoDuration::weeks(1),
        _ => ChronoDuration::days(1),
    }
}

/// Whether a saved search is due for a match run given its last run watermark and
/// the current time. A search with no watermark (first sighting) is always due —
/// the worker uses that run to establish the watermark without alerting on history.
fn is_due(frequency: &str, last_matched_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    match last_matched_at {
        None => true,
        Some(last) => now - last >= min_match_interval(frequency),
    }
}

/// Configuration for the saved-search alert worker.
#[derive(Debug, Clone)]
pub struct SavedSearchAlertConfig {
    pub enabled: bool,
    pub poll_interval_secs: u64,
    /// Max matching listings recorded per search per run (alert payload cap).
    pub match_limit: i64,
}

impl Default for SavedSearchAlertConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            poll_interval_secs: 3600,
            match_limit: 100,
        }
    }
}

impl SavedSearchAlertConfig {
    /// Build from environment:
    /// - `SAVED_SEARCH_ALERT_ENABLED` (default `true`)
    /// - `SAVED_SEARCH_ALERT_INTERVAL_SECS` (default `3600`)
    pub fn from_env() -> Self {
        let default = Self::default();
        let enabled = std::env::var("SAVED_SEARCH_ALERT_ENABLED")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(default.enabled);
        let poll_interval_secs = std::env::var("SAVED_SEARCH_ALERT_INTERVAL_SECS")
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

/// Background worker that matches alert-enabled saved searches against newly
/// published listings and enqueues alerts.
pub struct SavedSearchAlertWorker {
    db: DbPool,
    repo: RealityPortalRepository,
    config: SavedSearchAlertConfig,
}

impl SavedSearchAlertWorker {
    pub fn new(db: DbPool, config: SavedSearchAlertConfig) -> Self {
        let repo = RealityPortalRepository::new(db.clone());
        Self { db, repo, config }
    }

    /// Spawn the background task and return its `JoinHandle`.
    pub fn start(self) -> tokio::task::JoinHandle<()> {
        let poll_secs = self.config.poll_interval_secs;
        tokio::spawn(
            async move {
                if !self.config.enabled {
                    tracing::info!("[#983] SavedSearchAlertWorker disabled — not starting");
                    return;
                }
                tracing::info!(
                    poll_interval_secs = self.config.poll_interval_secs,
                    "[#983] SavedSearchAlertWorker started"
                );
                let mut ticker = interval(Duration::from_secs(self.config.poll_interval_secs));
                loop {
                    ticker.tick().await;
                    self.run_once().await;
                }
            }
            .instrument(tracing::info_span!(
                "bg.saved_search_alerts",
                poll_secs = poll_secs
            )),
        )
    }

    /// One matching pass over all alert-enabled saved searches.
    async fn run_once(&self) {
        // A dedicated connection in global-read context: lets the matching query
        // see published listings across orgs. The pool's after_release hook
        // clears the context when the connection is returned.
        let mut conn = match self.db.acquire().await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(error = %e, "[#983] could not acquire DB connection");
                return;
            }
        };
        if let Err(e) = db::tenant_context::set_global_read_context(&mut *conn, true).await {
            tracing::error!(error = %e, "[#983] failed to set global-read context");
            return;
        }

        let searches = match self.repo.list_alertable_saved_searches(&mut *conn).await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(error = %e, "[#983] failed to list alertable saved searches");
                return;
            }
        };

        let now = Utc::now();
        let mut queued = 0usize;
        for search in searches {
            // First sighting: establish the watermark, don't alert on history.
            let Some(since) = search.last_matched_at else {
                if let Err(e) = self
                    .repo
                    .enqueue_and_advance_saved_search(search.id, search.user_id, &[])
                    .await
                {
                    tracing::warn!(id = %search.id, error = %e, "[#983] failed to establish first-sighting watermark; will retry next run");
                }
                continue;
            };

            // Cadence: skip searches whose alert_frequency window hasn't elapsed
            // since their last match run. `instant` is always due.
            if !is_due(&search.alert_frequency, Some(since), now) {
                continue;
            }

            let query: PublicListingQuery = match serde_json::from_value(search.criteria.clone()) {
                Ok(q) => q,
                Err(e) => {
                    tracing::warn!(id = %search.id, error = %e, "[#983] unparseable saved-search criteria; skipping");
                    continue;
                }
            };

            let ids = match self
                .repo
                .find_new_match_listing_ids(
                    &mut *conn,
                    &query,
                    Some(since),
                    self.config.match_limit,
                )
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(id = %search.id, error = %e, "[#983] match query failed; skipping");
                    continue;
                }
            };

            // Enqueue any new matches AND advance the watermark atomically. The
            // watermark advances after every completed scan (matched or not) so
            // the cadence window restarts — otherwise a no-match search would be
            // due again on the very next poll, defeating daily/weekly spacing.
            //
            // #983: this used to enqueue and advance as two separate statements,
            // discarding the advance error with `let _ = …`. When the enqueue
            // succeeded but the advance failed, the alert stayed queued while the
            // watermark never moved, so the next due run re-enqueued a duplicate.
            // `enqueue_and_advance_saved_search` commits both writes in one
            // transaction, so a failed advance rolls the enqueue back (nothing is
            // sent, the window is retried cleanly) and the error is surfaced here
            // rather than silently dropped.
            match self
                .repo
                .enqueue_and_advance_saved_search(search.id, search.user_id, &ids)
                .await
            {
                Ok(()) => {
                    if !ids.is_empty() {
                        queued += 1;
                    }
                }
                Err(e) => {
                    tracing::warn!(id = %search.id, error = %e, "[#983] failed to enqueue alert / advance watermark; leaving watermark in place to retry next run");
                    continue;
                }
            }
        }

        if queued > 0 {
            tracing::info!(
                searches_alerted = queued,
                "[#983] saved-search alerts queued"
            );
        }
    }
}

#[cfg(test)]
mod cadence_tests {
    use super::{is_due, min_match_interval};
    use chrono::{Duration as ChronoDuration, TimeZone, Utc};

    #[test]
    fn unknown_frequency_falls_back_to_daily() {
        assert_eq!(min_match_interval("instant"), ChronoDuration::zero());
        assert_eq!(min_match_interval("daily"), ChronoDuration::days(1));
        assert_eq!(min_match_interval("weekly"), ChronoDuration::weeks(1));
        // Anything unrecognised (or an empty/garbled value) behaves like daily.
        assert_eq!(min_match_interval("hourly"), ChronoDuration::days(1));
        assert_eq!(min_match_interval(""), ChronoDuration::days(1));
    }

    #[test]
    fn first_sighting_is_always_due() {
        let now = Utc.with_ymd_and_hms(2026, 6, 25, 12, 0, 0).unwrap();
        assert!(is_due("daily", None, now));
        assert!(is_due("weekly", None, now));
        assert!(is_due("instant", None, now));
    }

    #[test]
    fn instant_is_due_on_every_poll() {
        let now = Utc.with_ymd_and_hms(2026, 6, 25, 12, 0, 0).unwrap();
        // Even one second after the last run, an instant search is due.
        let last = now - ChronoDuration::seconds(1);
        assert!(is_due("instant", Some(last), now));
        assert!(is_due("instant", Some(now), now));
    }

    #[test]
    fn daily_throttles_within_24h_and_releases_after() {
        let now = Utc.with_ymd_and_hms(2026, 6, 25, 12, 0, 0).unwrap();
        // 23h59m since last run -> not yet due.
        assert!(!is_due(
            "daily",
            Some(now - ChronoDuration::hours(23) - ChronoDuration::minutes(59)),
            now
        ));
        // Exactly 24h -> due (boundary inclusive).
        assert!(is_due("daily", Some(now - ChronoDuration::hours(24)), now));
        // Well past the window -> due.
        assert!(is_due("daily", Some(now - ChronoDuration::days(3)), now));
    }

    #[test]
    fn weekly_throttles_within_7d_and_releases_after() {
        let now = Utc.with_ymd_and_hms(2026, 6, 25, 12, 0, 0).unwrap();
        // 6 days since last run -> not yet due (a daily-spaced poll must not fire).
        assert!(!is_due("weekly", Some(now - ChronoDuration::days(6)), now));
        // Exactly 7 days -> due.
        assert!(is_due("weekly", Some(now - ChronoDuration::days(7)), now));
    }
}
