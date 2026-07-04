//! Favorite alert worker — BIT-138 / issue #983 follow-up.
//!
//! `portal_favorites` and `listing_price_history` are both protected by FORCE
//! RLS on the owning listing org. A context-less background worker sees
//! nothing, so this worker iterates organizations explicitly and sets the org
//! context on one connection before reading candidates for that org.

use std::time::Duration;

use db::{repositories::RealityPortalRepository, DbPool};
use sqlx::{pool::PoolConnection, Connection, Error as SqlxError, Postgres};
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

            // Price path: enqueue + advance run as ONE REPEATABLE READ
            // transaction so both statements observe a single MVCC snapshot.
            // The enqueue MUST run first — it reads the pre-update watermark.
            match self.run_price_path(&mut conn).await {
                Ok(n) => queued_price += n as usize,
                Err(e) => {
                    tracing::warn!(org_id = %org_id, error = %e, "[BIT-138] favorite price-alert pass failed");
                }
            }

            // Back-on-market path: one set-based enqueue (favorites whose listing
            // just became active), then snapshot ALL changed favorites' status —
            // also under one REPEATABLE READ transaction. back_on_market alerts
            // are intentionally not opt-out-able — see
            // `enqueue_pending_favorite_status_alerts` (#1852 finding-3).
            match self.run_status_path(&mut conn).await {
                Ok(n) => queued_status += n as usize,
                Err(e) => {
                    tracing::warn!(org_id = %org_id, error = %e, "[BIT-138] favorite back-on-market pass failed");
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

    /// Enqueue price-change alerts and advance the price watermarks inside a
    /// single `REPEATABLE READ` transaction (#1999 finding-1).
    ///
    /// The enqueue and the advance compute their working set independently:
    /// the enqueue inserts rows for `changed_at > watermark`, and the advance
    /// re-derives `MAX(changed_at)` over the *same* predicate. Run as separate
    /// autocommitted statements, a `listing_price_history` row committed by a
    /// concurrent writer *between* them would be missed by the enqueue yet
    /// swept past the watermark by the advance's re-evaluated `MAX` — the alert
    /// for that price change would then be silently dropped forever. Binding
    /// both statements to one MVCC snapshot closes that race. The org RLS
    /// context set on `conn` is session-scoped (`set_config(..., FALSE)`), so
    /// it remains in force inside the transaction.
    async fn run_price_path(&self, conn: &mut PoolConnection<Postgres>) -> Result<u64, SqlxError> {
        let mut tx = conn.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await?;
        let queued = self
            .repo
            .enqueue_pending_favorite_price_alerts(&mut *tx)
            .await?;
        self.repo
            .advance_favorite_price_watermarks(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(queued)
    }

    /// Enqueue back-on-market alerts and advance the status snapshots inside a
    /// single `REPEATABLE READ` transaction (#1999 finding-1), for the same
    /// snapshot-consistency reason as [`Self::run_price_path`]: the advance
    /// snapshots `last_seen_listing_status` for every changed favorite, so a
    /// status transition committed between the two statements must not let the
    /// advance move a favorite past a transition the enqueue never saw.
    async fn run_status_path(&self, conn: &mut PoolConnection<Postgres>) -> Result<u64, SqlxError> {
        let mut tx = conn.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await?;
        let queued = self
            .repo
            .enqueue_pending_favorite_status_alerts(&mut *tx)
            .await?;
        self.repo
            .advance_favorite_status_watermarks(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(queued)
    }
}
