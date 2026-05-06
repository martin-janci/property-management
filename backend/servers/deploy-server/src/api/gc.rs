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
use crate::domain::WorktreeState;
use crate::Result;
use axum::extract::State;
use axum::Json;
use chrono::Utc;
use std::sync::Arc;

#[derive(Clone)]
pub struct GcContext {
    pub svc: Arc<WorktreeService>,
    pub cfg: Arc<Config>,
}

#[derive(serde::Serialize)]
pub struct GcReport {
    pub paused: Vec<String>,
    pub stopped: Vec<String>,
    pub cleaned: Vec<String>,
    pub paused_targets: Vec<String>,
}

pub async fn tick_handler(State(ctx): State<GcContext>) -> Result<Json<GcReport>> {
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
        match wt.state {
            WorktreeState::Running => {
                if let Some(last) = wt.last_traffic_at {
                    if now - last > pause_after {
                        for c in &wt.containers {
                            let _ = ctx.svc.docker.stop_container(c).await;
                        }
                        wt.state = WorktreeState::Paused;
                        ctx.svc.store.upsert_worktree(&wt).await?;
                        report.paused.push(wt.name.clone());
                    }
                }
            }
            WorktreeState::Paused => {
                if let Some(last) = wt.last_traffic_at {
                    if now - last > stop_after {
                        for c in &wt.containers {
                            let _ = ctx.svc.docker.remove_container(c).await;
                        }
                        // Phase 1 has shared backend → no DB to dump. Phase 3 will add pg_dump here.
                        wt.state = WorktreeState::Closed;
                        wt.closed_at = Some(now);
                        ctx.svc.store.upsert_worktree(&wt).await?;
                        report.stopped.push(wt.name.clone());
                    }
                }
            }
            WorktreeState::Closed => {
                if let Some(closed_at) = wt.closed_at {
                    if (now - closed_at).num_seconds() > wt.ttl_seconds {
                        let dir = std::path::PathBuf::from(&ctx.cfg.worktree_dir)
                            .join(crate::infra::git::sanitize(&wt.branch));
                        let _ = tokio::fs::remove_dir_all(&dir).await;
                        report.cleaned.push(wt.name.clone());
                    }
                }
            }
            WorktreeState::Closing => {} // transient, leave for next tick
        }
    }

    // Staging idle 8h heuristic.
    // NOTE: Phase 2 lacks per-target traffic tracking; we rely on `promoted_at`
    // as a proxy. Phase 6 will add proper traffic tracking via Caddy access log tail.
    let staging_idle = chrono::Duration::seconds(8 * 3600);
    if let Some(rel) = ctx
        .svc
        .store
        .current_release_for("staging", "staging")
        .await?
    {
        if let Some(promoted) = rel.promoted_at {
            if Utc::now() - promoted > staging_idle {
                for color in ["blue", "green"] {
                    for service in ["api", "reality", "ppt", "reality-web"] {
                        let _ = ctx
                            .svc
                            .docker
                            .stop_container(&format!("staging-{service}-{color}"))
                            .await;
                    }
                }
                report.paused_targets.push("staging".into());
            }
        }
    }

    Ok(Json(report))
}
