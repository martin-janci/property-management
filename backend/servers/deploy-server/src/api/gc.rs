// backend/servers/deploy-server/src/api/gc.rs
//! Garbage collection / lifecycle housekeeping.
//!
//! Runs every 5 min via systemd timer. Drives worktree pause/stop/cleanup
//! state transitions and handles staging idle pause.
//!
//! NOTE: per-target traffic tracking is currently a heuristic based on
//! `release.promoted_at`. Phase 6 will replace this with proper traffic
//! tracking via Caddy access log tailing.
use crate::api::worktree::WorktreeService;
use crate::config::Config;
use crate::domain::{TargetKind, WorktreeState};
use crate::infra::CallerIdentity;
use crate::Result;
use axum::extract::State;
use axum::Json;
use chrono::Utc;
use std::sync::Arc;

/// Pure decision the GC tick can take for a single worktree, given its current
/// state and timing fields. Extracted so the timing logic can be unit-tested
/// without spinning up Docker/Postgres.
#[derive(Debug, PartialEq, Eq)]
pub enum GcDecision {
    NoOp,
    PauseRunning,
    StopPaused,
    Cleanup,
    RecoverStuckClosing,
}

#[allow(clippy::too_many_arguments)]
pub fn gc_decision(
    state: WorktreeState,
    last_traffic_at: Option<chrono::DateTime<chrono::Utc>>,
    closed_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
    ttl_seconds: i64,
    now: chrono::DateTime<chrono::Utc>,
    pause_after: chrono::Duration,
    stop_after: chrono::Duration,
) -> GcDecision {
    match state {
        WorktreeState::Running => {
            if let Some(last) = last_traffic_at {
                if now - last > pause_after {
                    return GcDecision::PauseRunning;
                }
            }
            GcDecision::NoOp
        }
        WorktreeState::Paused => {
            if let Some(last) = last_traffic_at {
                if now - last > stop_after {
                    return GcDecision::StopPaused;
                }
            }
            GcDecision::NoOp
        }
        WorktreeState::Closed => {
            if let Some(closed) = closed_at {
                if (now - closed).num_seconds() > ttl_seconds {
                    return GcDecision::Cleanup;
                }
            }
            GcDecision::NoOp
        }
        WorktreeState::Closing => {
            let stuck_threshold = chrono::Duration::minutes(5);
            if closed_at.is_none() && (now - created_at) > stuck_threshold {
                GcDecision::RecoverStuckClosing
            } else {
                GcDecision::NoOp
            }
        }
    }
}

#[derive(Clone)]
pub struct GcContext {
    pub svc: Arc<WorktreeService>,
    pub cfg: Arc<Config>,
}

/// Best-effort unregistration of a worktree's Caddy routes. Mirrors the same
/// logic used by `close_handler`; called by GC's pause/stop transitions so
/// stopped containers don't leave dangling routes that 502 forever.
///
/// All errors are swallowed (debug-logged) — Caddy may legitimately not have
/// the route (already cleaned, server restarted, etc.) and we don't want a
/// transient admin-API failure to abort the rest of the GC tick.
async fn unregister_worktree_routes(ctx: &GcContext, wt: &crate::domain::Worktree) {
    for url_opt in [
        wt.urls.ppt.as_deref(),
        wt.urls.reality.as_deref(),
        wt.urls.api.as_deref(),
    ] {
        if let Some(host) = url_opt.and_then(|u| u.strip_prefix("https://")) {
            if let Err(e) = ctx.svc.caddy.unregister_route(host).await {
                tracing::debug!(host = %host, error = %e, "caddy unregister failed during gc (ignored)");
            }
        }
    }
}

#[derive(serde::Serialize)]
pub struct GcReport {
    pub paused: Vec<String>,
    pub stopped: Vec<String>,
    pub cleaned: Vec<String>,
    pub paused_targets: Vec<String>,
}

pub async fn tick_handler(
    State(ctx): State<GcContext>,
    axum::Extension(caller): axum::Extension<CallerIdentity>,
) -> Result<Json<GcReport>> {
    caller.require_scope("gc:tick")?;
    let now = Utc::now();
    let pause_after = chrono::Duration::seconds(ctx.cfg.idle_pause_seconds);
    let stop_after = chrono::Duration::seconds(ctx.cfg.idle_stop_seconds);

    let mut report = GcReport {
        paused: vec![],
        stopped: vec![],
        cleaned: vec![],
        paused_targets: vec![],
    };
    let worktrees = ctx.svc.store.list_worktrees().await?;
    for mut wt in worktrees {
        // Skip worktrees currently being mutated by an open/close call.
        // Prevents GC from dropping a dedicated DB while open_handler is mid-flight (#12).
        let _lock = match ctx.svc.worktree_locks.try_acquire(&wt.name).await {
            Some(g) => g,
            None => {
                tracing::debug!(name = %wt.name, "skipping GC tick — worktree is being mutated");
                continue;
            }
        };
        let decision = gc_decision(
            wt.state.clone(),
            wt.last_traffic_at,
            wt.closed_at,
            wt.created_at,
            wt.ttl_seconds,
            now,
            pause_after,
            stop_after,
        );
        match decision {
            GcDecision::NoOp => continue,
            GcDecision::PauseRunning => {
                ctx.svc.docker.cleanup_containers(&wt.containers).await;
                // Unregister Caddy routes — without this, the routes keep
                // pointing at dead upstreams and visitors hit persistent 502s
                // until the worktree is reopened and the route gets overwritten.
                unregister_worktree_routes(&ctx, &wt).await;
                wt.state = WorktreeState::Paused;
                ctx.svc.store.upsert_worktree(&wt).await?;
                report.paused.push(wt.name.clone());
            }
            GcDecision::StopPaused => {
                ctx.svc.docker.cleanup_containers(&wt.containers).await;
                unregister_worktree_routes(&ctx, &wt).await;
                // If dedicated backend, dump DB before drop.
                if let Some(db) = wt.db_name.clone() {
                    let dump_path_str = format!(
                        "{}/{}-{}.dump",
                        ctx.cfg.snapshot_dir,
                        wt.name,
                        now.timestamp()
                    );
                    let dump_path = std::path::Path::new(&dump_path_str);
                    if let Err(e) = ctx.svc.postgres.dump(&db, dump_path).await {
                        tracing::warn!(error = %e, db = %db, "pg_dump failed during gc stop");
                    } else {
                        wt.dump_path = Some(dump_path_str.clone());
                        if let Err(e) = ctx.svc.postgres.drop_db(&db).await {
                            tracing::warn!(error = %e, db = %db, "pg drop failed");
                        } else {
                            wt.db_name = None;
                        }
                    }
                }
                wt.state = WorktreeState::Closed;
                wt.closed_at = Some(now);
                ctx.svc.store.upsert_worktree(&wt).await?;
                report.stopped.push(wt.name.clone());
            }
            GcDecision::Cleanup => {
                let dir = std::path::PathBuf::from(&ctx.cfg.worktree_dir)
                    .join(crate::infra::git::sanitize(&wt.branch));
                let _ = tokio::fs::remove_dir_all(&dir).await;
                // Remove DB dump if present.
                if let Some(ref dump) = wt.dump_path {
                    let _ = tokio::fs::remove_file(dump).await;
                }
                // Drop the row last — only after fs/dump removals have run, so
                // a partial cleanup leaves enough breadcrumbs for the next tick
                // to retry. After this the worktree is fully gone; subsequent
                // ticks won't see it in `list_worktrees`.
                if let Err(e) = ctx.svc.store.delete_worktree(&wt.name).await {
                    tracing::warn!(name = %wt.name, error = %e, "delete_worktree failed");
                }
                report.cleaned.push(wt.name.clone());
            }
            GcDecision::RecoverStuckClosing => {
                // Recovery: a previous close_handler crash left this stuck (#8).
                ctx.svc.docker.cleanup_containers(&wt.containers).await;
                unregister_worktree_routes(&ctx, &wt).await;
                wt.state = WorktreeState::Closed;
                wt.closed_at = Some(now);
                ctx.svc.store.upsert_worktree(&wt).await?;
                report.stopped.push(wt.name.clone());
            }
        }
    }

    // Staging idle 8h heuristic.
    // NOTE: Phase 2 lacks per-target traffic tracking; we rely on `promoted_at`
    // as a proxy. Phase 6 will add proper traffic tracking via Caddy access log tail.
    let staging_idle = chrono::Duration::seconds(8 * 3600);
    let staging = TargetKind::Staging;
    if let Some(rel) = ctx
        .svc
        .store
        .current_release_for(staging.as_str(), staging.live_release_state_str())
        .await?
    {
        if let Some(promoted) = rel.promoted_at {
            if Utc::now() - promoted > staging_idle {
                let staging_containers: Vec<String> = ["blue", "green"]
                    .iter()
                    .flat_map(|color| {
                        crate::infra::BG_SERVICES
                            .iter()
                            .map(move |service| format!("{}-{service}-{color}", staging.as_str()))
                    })
                    .collect();
                ctx.svc.docker.cleanup_containers(&staging_containers).await;
                report.paused_targets.push(staging.as_str().into());
            }
        }
    }

    Ok(Json(report))
}

#[cfg(test)]
mod gc_decision_tests {
    use super::*;
    use crate::domain::WorktreeState;
    use chrono::{Duration, Utc};

    #[test]
    fn running_with_recent_traffic_no_op() {
        let now = Utc::now();
        let d = gc_decision(
            WorktreeState::Running,
            Some(now - Duration::seconds(60)),
            None,
            now,
            86400,
            now,
            Duration::seconds(1800),
            Duration::seconds(86400),
        );
        assert_eq!(d, GcDecision::NoOp);
    }

    #[test]
    fn running_with_stale_traffic_pauses() {
        let now = Utc::now();
        let d = gc_decision(
            WorktreeState::Running,
            Some(now - Duration::seconds(2000)),
            None,
            now,
            86400,
            now,
            Duration::seconds(1800),
            Duration::seconds(86400),
        );
        assert_eq!(d, GcDecision::PauseRunning);
    }

    #[test]
    fn paused_idle_24h_stops() {
        let now = Utc::now();
        let d = gc_decision(
            WorktreeState::Paused,
            Some(now - Duration::seconds(90000)),
            None,
            now,
            86400,
            now,
            Duration::seconds(1800),
            Duration::seconds(86400),
        );
        assert_eq!(d, GcDecision::StopPaused);
    }

    #[test]
    fn closed_past_ttl_cleans_up() {
        let now = Utc::now();
        let d = gc_decision(
            WorktreeState::Closed,
            None,
            Some(now - Duration::seconds(200000)),
            now,
            86400,
            now,
            Duration::seconds(1800),
            Duration::seconds(86400),
        );
        assert_eq!(d, GcDecision::Cleanup);
    }

    #[test]
    fn closed_within_ttl_no_op() {
        let now = Utc::now();
        let d = gc_decision(
            WorktreeState::Closed,
            None,
            Some(now - Duration::seconds(60)),
            now,
            86400,
            now,
            Duration::seconds(1800),
            Duration::seconds(86400),
        );
        assert_eq!(d, GcDecision::NoOp);
    }

    #[test]
    fn closing_stuck_over_5min_recovers() {
        let now = Utc::now();
        let d = gc_decision(
            WorktreeState::Closing,
            None,
            None,
            now - Duration::seconds(400),
            86400,
            now,
            Duration::seconds(1800),
            Duration::seconds(86400),
        );
        assert_eq!(d, GcDecision::RecoverStuckClosing);
    }

    #[test]
    fn closing_recent_no_op() {
        let now = Utc::now();
        let d = gc_decision(
            WorktreeState::Closing,
            None,
            None,
            now - Duration::seconds(60),
            86400,
            now,
            Duration::seconds(1800),
            Duration::seconds(86400),
        );
        assert_eq!(d, GcDecision::NoOp);
    }
}
