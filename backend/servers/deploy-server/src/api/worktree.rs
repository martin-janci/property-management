// backend/servers/deploy-server/src/api/worktree.rs
use crate::domain::{BackendMode, Worktree, WorktreeState, WorktreeUrls};
use crate::infra::git::sanitize;
use crate::infra::{
    CaddyClient, CallerIdentity, DockerClient, FrontendDevSpec, GhClient, GitFetcher, PostgresOps,
    Store,
};
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
    pub postgres: Arc<PostgresOps>,
    pub gh: Arc<GhClient>,
    pub backend_image_prefix: String,
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

    // Strict input validation — defense against SQL injection (db_name interpolation)
    // and command/option injection (git fetch <branch>, image tag interpolation).
    crate::infra::git::validate_branch_strict(&req.branch)?;
    crate::infra::git::validate_alias_strict(&name)?;

    // Resume from dump if a closed worktree with this name exists within TTL window.
    let existing = svc.store.get_worktree(&name).await?;
    let resume_db: Option<String> = if let Some(ref ex) = existing {
        if matches!(ex.state, WorktreeState::Closed)
            && matches!(req.backend, BackendMode::Dedicated)
        {
            if let (Some(db), Some(dump_path)) = (
                ex.db_name.clone().or_else(|| {
                    // db was dropped during gc; reconstruct expected name
                    Some(format!("{}{}", svc.postgres.user_db_prefix, name))
                }),
                ex.dump_path.clone(),
            ) {
                tracing::info!(name=%name, dump=%dump_path, "resuming worktree from pg dump");
                svc.postgres
                    .restore(&db, std::path::Path::new(&dump_path))
                    .await?;
                Some(db)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // 1. Fetch source.
    let source_path = svc.git.fetch_branch(&req.branch).await?;

    // 2. Allocate frontend ports.
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
            pnpm_volume,
            image: svc.frontend_image.clone(),
        })
        .await?;

    // 4. Register frontend Caddy routes.
    let host_ppt = format!("wt-{name}.{}", svc.domain_dev_ppt);
    let host_reality = format!("wt-{name}.{}", svc.domain_dev_reality);
    svc.caddy
        .register_route(&host_ppt, &format!("127.0.0.1:{port_ppt}"))
        .await?;
    svc.caddy
        .register_route(&host_reality, &format!("127.0.0.1:{port_reality}"))
        .await?;

    let mut containers = vec![ppt_container, reality_container];
    let mut api_url: Option<String> = None;
    let mut db_name: Option<String> = None;
    let mut backend_status = "ready".to_string();

    // 5. Dedicated backend branch.
    if matches!(req.backend, BackendMode::Dedicated) {
        let db = if let Some(existing_db) = resume_db.clone() {
            // Already restored above
            existing_db
        } else {
            let db = format!("{}{}", svc.postgres.user_db_prefix, name);
            svc.postgres.create_from_template(&db).await?;
            db
        };
        db_name = Some(db.clone());

        // Dispatch GHA workflow (skip if resuming — images cached from previous open).
        let completed = if resume_db.is_some() {
            true
        } else {
            svc.gh
                .dispatch_workflow("docker-build.yml", &req.branch)
                .await?;
            let mut completed = false;
            for _ in 0..60 {
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                if let Some(run) = svc.gh.latest_run("docker-build.yml", &req.branch).await? {
                    if run.status == "completed" {
                        if run.conclusion.as_deref() != Some("success") {
                            return Err(DeployError::Internal(format!(
                                "workflow failed: {}",
                                run.html_url
                            )));
                        }
                        completed = true;
                        break;
                    }
                }
            }
            completed
        };
        if !completed {
            backend_status = "building".into();
            // Don't fail — return URLs for frontend, backend will come online when build finishes.
            // Operator can call status to check.
        } else {
            // Run backend containers.
            // Image tag matches docker-build.yml's `type=ref,event=branch` which
            // replaces `/` with `-` but preserves case. So `feature/UC-14` → `feature-UC-14`.
            let branch_tag = req.branch.replace(['/', '_'], "-");
            let api_image = format!("{}/ppt-api-server:{}", svc.backend_image_prefix, branch_tag);
            let reality_image = format!(
                "{}/ppt-reality-server:{}",
                svc.backend_image_prefix, branch_tag
            );

            let api_port = pick_port(&format!("{name}-api"));
            let reality_port = pick_port(&format!("{name}-reality-api"));
            let api_c = format!("wt-{name}-api");
            let reality_c = format!("wt-{name}-reality-api");

            let db_url = format!(
                "{}/{}",
                svc.postgres.admin_url.trim_end_matches("/postgres"),
                db
            );

            let jwt_secret = std::env::var("PPT_JWT_SECRET")
                .unwrap_or_else(|_| "dev-secret-min-32-chars-please-replace-replace".into());

            svc.docker
                .run_backend_dedicated(&crate::infra::BackendDedicatedSpec {
                    container_name: api_c.clone(),
                    image: api_image,
                    host_port: api_port,
                    container_port: 8080,
                    db_url: db_url.clone(),
                    jwt_secret: jwt_secret.clone(),
                })
                .await?;
            svc.docker
                .run_backend_dedicated(&crate::infra::BackendDedicatedSpec {
                    container_name: reality_c.clone(),
                    image: reality_image,
                    host_port: reality_port,
                    container_port: 8081,
                    db_url,
                    jwt_secret,
                })
                .await?;

            // Caddy routes for backend
            let host_api = format!("api.wt-{name}.{}", svc.domain_dev_ppt);
            svc.caddy
                .register_route(&host_api, &format!("127.0.0.1:{api_port}"))
                .await?;
            api_url = Some(format!("https://{host_api}"));

            containers.push(api_c);
            containers.push(reality_c);
        }
    }

    // 6. Persist state.
    let now = chrono::Utc::now();
    let wt = Worktree {
        name: name.clone(),
        branch: req.branch.clone(),
        backend_mode: req.backend.clone(),
        state: WorktreeState::Running,
        urls: WorktreeUrls {
            ppt: Some(format!("https://{host_ppt}")),
            reality: Some(format!("https://{host_reality}")),
            api: api_url,
        },
        containers,
        db_name,
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
        backend_status,
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
