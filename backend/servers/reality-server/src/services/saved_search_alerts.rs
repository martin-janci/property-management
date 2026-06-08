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
//! Out of scope (documented follow-ups, blocked on missing infrastructure):
//! - **Delivery**: turning queued `search_alert_queue` rows into emails/in-app
//!   notifications. reality-server has no notification/email transport today;
//!   delivery needs either a cross-server call to api-server's notification
//!   pipeline or a dedicated drainer. The queue is the hand-off point.
//! - **Favorite price-drop alerts (16.2)**: `portal_favorites` and
//!   `listing_price_history` are `FORCE ROW LEVEL SECURITY` org-isolated, so a
//!   context-less worker reads nothing; that half needs per-org / super-admin
//!   context, a deliberate privilege decision.

use std::time::Duration;

use db::models::PublicListingQuery;
use db::{repositories::RealityPortalRepository, DbPool};
use tokio::time::interval;
use tracing::Instrument;

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

        let mut queued = 0usize;
        for search in searches {
            // First sighting: establish the watermark, don't alert on history.
            let Some(since) = search.last_matched_at else {
                let _ = self
                    .repo
                    .mark_saved_search_matched(&mut *conn, search.id, 0)
                    .await;
                continue;
            };

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

            if ids.is_empty() {
                continue;
            }

            if let Err(e) = self
                .repo
                .enqueue_search_alert(&mut *conn, search.id, search.user_id, &ids, "new_listing")
                .await
            {
                tracing::warn!(id = %search.id, error = %e, "[#983] failed to enqueue alert; skipping watermark advance");
                continue;
            }
            let _ = self
                .repo
                .mark_saved_search_matched(&mut *conn, search.id, ids.len() as i64)
                .await;
            queued += 1;
        }

        if queued > 0 {
            tracing::info!(
                searches_alerted = queued,
                "[#983] saved-search alerts queued"
            );
        }
    }
}
