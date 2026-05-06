// backend/servers/deploy-server/src/api/worktree.rs
use crate::domain::{BackendMode, Worktree, WorktreeState, WorktreeUrls};
use crate::infra::git::sanitize;
use crate::infra::{CaddyClient, CallerIdentity, DockerClient, FrontendDevSpec, GitFetcher, Store};
use crate::{DeployError, Result};
use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct WorktreeService {
    pub store: Arc<Store>,
    pub git: Arc<GitFetcher>,
    pub docker: Arc<DockerClient>,
    pub caddy: Arc<CaddyClient>,
    pub frontend_image: String,
    pub domain_dev_ppt: String,     // "dev.ppt.rlt.sk"
    pub domain_dev_reality: String, // "dev.rlt.sk"
}

#[derive(Debug, Deserialize)]
pub struct OpenRequest {
    pub branch: String,
    pub alias: Option<String>,
    #[serde(default = "default_backend")]
    pub backend: BackendMode,
    pub ttl_seconds: Option<i64>,
}

fn default_backend() -> BackendMode {
    BackendMode::Shared
}

#[derive(Debug, Serialize)]
pub struct OpenResponse {
    pub worktree: Worktree,
    pub backend_status: String, // "ready" | "building"
}

pub async fn open_handler(
    State(svc): State<Arc<WorktreeService>>,
    axum::Extension(caller): axum::Extension<CallerIdentity>,
    Json(req): Json<OpenRequest>,
) -> Result<Json<OpenResponse>> {
    let name = req.alias.clone().unwrap_or_else(|| sanitize(&req.branch));
    if name.is_empty() {
        return Err(DeployError::BadRequest(
            "alias resolves to empty name".into(),
        ));
    }

    if matches!(req.backend, BackendMode::Dedicated) {
        return Err(DeployError::BadRequest(
            "dedicated backend mode is implemented in Phase 3".into(),
        ));
    }

    // 1. Fetch source.
    let source_path = svc.git.fetch_branch(&req.branch).await?;

    // 2. Allocate ports (deterministic hash of name → 51000–51999 range).
    let port_ppt = pick_port(&format!("{name}-ppt"));
    let port_reality = pick_port(&format!("{name}-reality"));

    // 3. Spawn frontend dev containers.
    let pnpm_volume = format!("ppt-deploy-pnpm-{name}");
    let ppt_container = format!("wt-{name}-ppt");
    let reality_container = format!("wt-{name}-reality");

    svc.docker
        .run_frontend_dev(&FrontendDevSpec {
            container_name: ppt_container.clone(),
            app: "ppt-web".into(),
            source_path: source_path.to_string_lossy().to_string(),
            host_port: port_ppt,
            pnpm_volume: pnpm_volume.clone(),
            image: svc.frontend_image.clone(),
        })
        .await?;
    svc.docker
        .run_frontend_dev(&FrontendDevSpec {
            container_name: reality_container.clone(),
            app: "reality-web".into(),
            source_path: source_path.to_string_lossy().to_string(),
            host_port: port_reality,
            pnpm_volume: pnpm_volume.clone(),
            image: svc.frontend_image.clone(),
        })
        .await?;

    // 4. Register Caddy routes.
    let host_ppt = format!("wt-{name}.{}", svc.domain_dev_ppt);
    let host_reality = format!("wt-{name}.{}", svc.domain_dev_reality);
    svc.caddy
        .register_route(&host_ppt, &format!("127.0.0.1:{port_ppt}"))
        .await?;
    svc.caddy
        .register_route(&host_reality, &format!("127.0.0.1:{port_reality}"))
        .await?;

    // 5. Persist state.
    let now = chrono::Utc::now();
    let wt = Worktree {
        name: name.clone(),
        branch: req.branch.clone(),
        backend_mode: req.backend.clone(),
        state: WorktreeState::Running,
        urls: WorktreeUrls {
            ppt: Some(format!("https://{host_ppt}")),
            reality: Some(format!("https://{host_reality}")),
            api: None,
        },
        containers: vec![ppt_container, reality_container],
        db_name: None,
        dump_path: None,
        ttl_seconds: req.ttl_seconds.unwrap_or(172_800),
        last_traffic_at: Some(now),
        closed_at: None,
        created_at: now,
        created_by: format!("{}:{}", caller.kind, caller.id),
    };
    svc.store.upsert_worktree(&wt).await?;

    Ok(Json(OpenResponse {
        worktree: wt,
        backend_status: "ready".into(),
    }))
}

pub async fn get_handler(
    State(svc): State<Arc<WorktreeService>>,
    Path(name): Path<String>,
) -> Result<Json<Worktree>> {
    let wt = svc
        .store
        .get_worktree(&name)
        .await?
        .ok_or_else(|| DeployError::NotFound(format!("worktree {name}")))?;
    Ok(Json(wt))
}

pub async fn list_handler(State(svc): State<Arc<WorktreeService>>) -> Result<Json<Vec<Worktree>>> {
    Ok(Json(svc.store.list_worktrees().await?))
}

pub async fn close_handler(
    State(svc): State<Arc<WorktreeService>>,
    Path(name): Path<String>,
) -> Result<Json<Worktree>> {
    let mut wt = svc
        .store
        .get_worktree(&name)
        .await?
        .ok_or_else(|| DeployError::NotFound(format!("worktree {name}")))?;

    // Stop containers, ignore individual errors (best-effort cleanup).
    for c in &wt.containers {
        let _ = svc.docker.stop_container(c).await;
    }

    // Unregister Caddy routes.
    if let Some(host) = wt
        .urls
        .ppt
        .as_deref()
        .and_then(|u| u.strip_prefix("https://"))
    {
        let _ = svc.caddy.unregister_route(host).await;
    }
    if let Some(host) = wt
        .urls
        .reality
        .as_deref()
        .and_then(|u| u.strip_prefix("https://"))
    {
        let _ = svc.caddy.unregister_route(host).await;
    }

    wt.state = WorktreeState::Closed;
    wt.closed_at = Some(chrono::Utc::now());
    svc.store.upsert_worktree(&wt).await?;
    Ok(Json(wt))
}

fn pick_port(seed: &str) -> u16 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    seed.hash(&mut h);
    51000 + (h.finish() % 1000) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_port_deterministic_and_in_range() {
        let a = pick_port("foo");
        let b = pick_port("foo");
        assert_eq!(a, b);
        assert!((51000..52000).contains(&a));
    }
}
