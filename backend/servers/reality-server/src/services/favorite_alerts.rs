//! Favorite alert worker — BIT-138 / issue #983 follow-up.
//!
//! `portal_favorites` and `listing_price_history` are both protected by FORCE
//! RLS on the owning listing org. A context-less background worker sees
//! nothing, so this worker iterates organizations explicitly and sets the org
//! context on one connection before reading candidates for that org.

use std::time::Duration;

use db::models::listing_status;
use db::{repositories::RealityPortalRepository, DbPool};
use tokio::time::interval;
use tracing::Instrument;

/// Configuration for favorite price / status alerts.
#[derive(Debug, Clone)]
pub struct FavoriteAlertConfig {
    pub enabled: bool,
    pub poll_interval_secs: u64,
}

impl Default for FavoriteAlertConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            poll_interval_secs: 3600,
        }
    }
}

impl FavoriteAlertConfig {
    pub fn from_env() -> Self {
        let default = Self::default();
        let enabled = std::env::var("FAVORITE_ALERT_ENABLED")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(default.enabled);
        let poll_interval_secs = std::env::var("FAVORITE_ALERT_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.poll_interval_secs);
        Self {
            enabled,
            poll_interval_secs,
        }
    }
}

/// Background worker that queues favorite price-change and back-on-market alerts.
pub struct FavoriteAlertWorker {
    db: DbPool,
    repo: RealityPortalRepository,
    config: FavoriteAlertConfig,
}

impl FavoriteAlertWorker {
    pub fn new(db: DbPool, config: FavoriteAlertConfig) -> Self {
        let repo = RealityPortalRepository::new(db.clone());
        Self { db, repo, config }
    }

    pub fn start(self) -> tokio::task::JoinHandle<()> {
        let poll_secs = self.config.poll_interval_secs;
        tokio::spawn(
            async move {
                if !self.config.enabled {
                    tracing::info!("[BIT-138] FavoriteAlertWorker disabled — not starting");
                    return;
                }
                tracing::info!(
                    poll_interval_secs = self.config.poll_interval_secs,
                    "[BIT-138] FavoriteAlertWorker started"
                );
                let mut ticker = interval(Duration::from_secs(self.config.poll_interval_secs));
                loop {
                    ticker.tick().await;
                    self.run_once().await;
                }
            }
            .instrument(tracing::info_span!(
                "bg.favorite_alerts",
                poll_secs = poll_secs
            )),
        )
    }

    /// One pass over all listing-owning organizations.
    pub async fn run_once(&self) {
        let org_ids = match self.repo.list_listing_org_ids().await {
            Ok(ids) => ids,
            Err(e) => {
                tracing::error!(error = %e, "[BIT-138] failed to list listing orgs");
                return;
            }
        };

        let mut conn = match self.db.acquire().await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(error = %e, "[BIT-138] could not acquire DB connection");
                return;
            }
        };

        let mut queued_price = 0usize;
        let mut queued_status = 0usize;

        for org_id in org_ids {
            if let Err(e) =
                db::tenant_context::set_request_context(&mut *conn, Some(org_id), None, false).await
            {
                tracing::error!(org_id = %org_id, error = %e, "[BIT-138] failed to set org context");
                continue;
            }

            let price_candidates = match self
                .repo
                .list_pending_favorite_price_alerts(&mut *conn)
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(org_id = %org_id, error = %e, "[BIT-138] price candidate query failed");
                    continue;
                }
            };
            for candidate in price_candidates {
                if let Err(e) = self
                    .repo
                    .enqueue_favorite_price_alert(&mut *conn, &candidate)
                    .await
                {
                    tracing::warn!(
                        org_id = %org_id,
                        favorite_id = %candidate.favorite_id,
                        error = %e,
                        "[BIT-138] failed to enqueue favorite price alert"
                    );
                    continue;
                }
                if let Err(e) = self
                    .repo
                    .mark_favorite_price_alert_seen(
                        &mut *conn,
                        candidate.favorite_id,
                        candidate.changed_at,
                    )
                    .await
                {
                    tracing::warn!(
                        org_id = %org_id,
                        favorite_id = %candidate.favorite_id,
                        error = %e,
                        "[BIT-138] failed to advance favorite price watermark"
                    );
                    continue;
                }
                queued_price += 1;
            }

            let status_candidates = match self
                .repo
                .list_pending_favorite_status_alerts(&mut *conn)
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(org_id = %org_id, error = %e, "[BIT-138] status candidate query failed");
                    continue;
                }
            };
            for candidate in status_candidates {
                let should_alert = candidate.new_status == listing_status::ACTIVE
                    && candidate.previous_status.as_deref() != Some(listing_status::ACTIVE);
                if should_alert {
                    if let Err(e) = self
                        .repo
                        .enqueue_favorite_status_alert(&mut *conn, &candidate)
                        .await
                    {
                        tracing::warn!(
                            org_id = %org_id,
                            favorite_id = %candidate.favorite_id,
                            error = %e,
                            "[BIT-138] failed to enqueue back-on-market alert"
                        );
                    } else {
                        queued_status += 1;
                    }
                }

                if let Err(e) = self
                    .repo
                    .mark_favorite_listing_status_seen(
                        &mut *conn,
                        candidate.favorite_id,
                        &candidate.new_status,
                    )
                    .await
                {
                    tracing::warn!(
                        org_id = %org_id,
                        favorite_id = %candidate.favorite_id,
                        error = %e,
                        "[BIT-138] failed to advance listing status snapshot"
                    );
                }
            }
        }

        if queued_price > 0 || queued_status > 0 {
            tracing::info!(
                queued_price,
                queued_status,
                "[BIT-138] favorite alerts queued"
            );
        }
    }
}
