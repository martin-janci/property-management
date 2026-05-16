// backend/servers/deploy-server/src/api/release.rs
use crate::config::TargetsConfig;
use crate::domain::{Release, ReleaseState, TargetKind};
use crate::infra::{
    build_service_envs, validate_target, BlueGreenDeployer, BlueGreenSpec, CaddyClient,
    CallerIdentity, DockerClient, PreflightContext, StagingDeploySpec, Store,
};
use crate::{DeployError, Result};
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

#[derive(Clone)]
pub struct ReleaseService {
    pub store: Arc<Store>,
    pub docker_pool: Arc<HashMap<String, Arc<DockerClient>>>,
    pub caddy_pool: Arc<HashMap<String, Arc<CaddyClient>>>,
    pub targets: Arc<TargetsConfig>,
    pub image_prefix: String,
    /// Admin URL for the shared `ppt-postgres` instance, used by the
    /// preflight validator to verify per-target databases exist before a
    /// deploy starts. Same value as `Config::postgres_admin_url`.
    pub postgres_admin_url: String,
}

#[derive(Debug, Deserialize)]
pub struct DeployRequest {
    pub tag: String,
    #[serde(default = "default_target")]
    pub target: String,
}

fn default_target() -> String {
    "staging".into()
}

/// Construct the {service -> image} map published to GHCR for a given tag.
///
/// Documented inconsistency, matching what `docker-frontend.yml`/`docker-backend.yml`
/// actually push: backend images carry the `ppt-` prefix (`ppt-api-server`,
/// `ppt-reality-server`), and so does `ppt-web`. `reality-web`'s **service key**
/// remains `reality-web` (so consumers don't churn) but the registry image is
/// `ppt-reality-web` to match the `target: ppt-reality-web` matrix entry in
/// `.github/workflows/docker-frontend.yml`. Single source of truth lives here.
pub fn build_staging_images(image_prefix: &str, tag: &str) -> HashMap<String, String> {
    let mut images = HashMap::new();
    images.insert(
        "api-server".into(),
        format!("{image_prefix}/ppt-api-server:{tag}"),
    );
    images.insert(
        "reality-server".into(),
        format!("{image_prefix}/ppt-reality-server:{tag}"),
    );
    images.insert("ppt-web".into(), format!("{image_prefix}/ppt-web:{tag}"));
    images.insert(
        "reality-web".into(),
        format!("{image_prefix}/ppt-reality-web:{tag}"),
    );
    images.insert(
        "admin-web".into(),
        format!("{image_prefix}/ppt-admin-web:{tag}"),
    );
    images
}

pub async fn deploy_handler(
    State(svc): State<Arc<ReleaseService>>,
    axum::Extension(caller): axum::Extension<CallerIdentity>,
    Json(req): Json<DeployRequest>,
) -> Result<Json<Release>> {
    caller.require_scope("release:deploy")?;
    let target = TargetKind::from_str(&req.target)
        .map_err(|e| DeployError::Config(format!("invalid target: {e}")))?;
    // Allow any target that's actually configured. Earlier the handler
    // refused everything except `staging` while prod was a Phase 4 milestone;
    // with the dual-apex routing in place (PR #209) and a `prod` target in
    // `/etc/ppt-deploy/targets.yaml`, the same blue/green flow drives both
    // staging and prod. Configuration is the source of truth — if the operator
    // hasn't added a `prod:` block, we still 404 below with `unknown target`.
    let target_cfg = svc
        .targets
        .targets
        .get(target.as_str())
        .ok_or_else(|| DeployError::Config(format!("unknown target {target}")))?;

    let docker = svc
        .docker_pool
        .get(target.as_str())
        .ok_or_else(|| DeployError::Config(format!("no docker for target {target}")))?
        .clone();
    let caddy = svc
        .caddy_pool
        .get(target.as_str())
        .ok_or_else(|| DeployError::Config(format!("no caddy for target {target}")))?
        .clone();

    // Preflight: every required dependency must already be in place. Reports
    // *all* failures together — operators can fix everything in one pass and
    // re-trigger, instead of playing whack-a-mole one error at a time.
    let target_db = format!("ppt_{}", target.as_str());
    let target_network = format!("ppt-{}", target.as_str());
    let pf = validate_target(&PreflightContext {
        target_name: target.as_str(),
        target: target_cfg,
        docker: &docker,
        caddy: &caddy,
        postgres_admin_url: &svc.postgres_admin_url,
        target_db: &target_db,
        postgres_container: "ppt-postgres",
        caddy_container: "ppt-caddy",
        target_network: &target_network,
    })
    .await;
    if !pf.is_empty() {
        let detail = pf
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(DeployError::BadRequest(format!(
            "preflight failed for target '{}' ({} issue{}): {}",
            target.as_str(),
            pf.len(),
            if pf.len() == 1 { "" } else { "s" },
            detail
        )));
    }

    let images = build_staging_images(&svc.image_prefix, &req.tag);
    let service_envs = build_service_envs(target.as_str(), target_cfg)?;

    let spec = StagingDeploySpec {
        tag: req.tag.clone(),
        api_image: images["api-server"].clone(),
        reality_image: images["reality-server"].clone(),
        ppt_web_image: images["ppt-web"].clone(),
        reality_web_image: images["reality-web"].clone(),
        admin_web_image: Some(images["admin-web"].clone()),
        reality_apex: target_cfg.reality_apex.clone(),
        ppt_apex: target_cfg.ppt_apex.clone(),
        target_name: target.as_str().into(),
        service_envs,
    };
    let deployer = BlueGreenDeployer { docker, caddy };
    deployer.deploy(&spec).await?;

    let rel = Release {
        tag: req.tag.clone(),
        images,
        state: ReleaseState::Staging,
        target: Some(target.as_str().into()),
        promoted_at: Some(chrono::Utc::now()),
        notes: None,
    };
    svc.store.upsert_release(&rel).await?;
    Ok(Json(rel))
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub tag: String,
    pub images: HashMap<String, String>,
    #[serde(default)]
    pub notes: Option<String>,
}

pub async fn register_candidate_handler(
    State(svc): State<Arc<ReleaseService>>,
    axum::Extension(caller): axum::Extension<CallerIdentity>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<Release>> {
    caller.require_scope("release:register")?;
    let rel = Release {
        tag: req.tag,
        images: req.images,
        state: ReleaseState::Candidate,
        target: Some(TargetKind::Prod.as_str().into()),
        promoted_at: None,
        notes: req.notes,
    };
    svc.store.upsert_release(&rel).await?;
    Ok(Json(rel))
}

pub async fn wake_handler(
    State(svc): State<Arc<ReleaseService>>,
    axum::Extension(caller): axum::Extension<CallerIdentity>,
    Path(target): Path<String>,
) -> Result<Json<serde_json::Value>> {
    caller.require_scope("release:wake")?;
    let target = TargetKind::from_str(&target)
        .map_err(|e| DeployError::Config(format!("invalid target: {e}")))?;
    if target != TargetKind::Staging {
        return Err(DeployError::BadRequest(
            "only staging supported in Phase 2".into(),
        ));
    }
    let rel = svc
        .store
        .current_release_for(target.as_str(), target.live_release_state_str())
        .await?
        .ok_or_else(|| DeployError::NotFound("no staging release recorded".into()))?;
    let target_cfg = svc
        .targets
        .targets
        .get(target.as_str())
        .ok_or_else(|| DeployError::Config("staging target missing".into()))?;
    let service_envs = build_service_envs(target.as_str(), target_cfg)?;
    let spec = BlueGreenSpec::from_release(&rel, target.as_str(), target_cfg, service_envs)?;
    let docker = svc
        .docker_pool
        .get(target.as_str())
        .ok_or_else(|| DeployError::Config(format!("no docker for target {target}")))?
        .clone();
    let caddy = svc
        .caddy_pool
        .get(target.as_str())
        .ok_or_else(|| DeployError::Config(format!("no caddy for target {target}")))?
        .clone();
    let deployer = BlueGreenDeployer { docker, caddy };
    deployer.deploy(&spec).await?;
    Ok(Json(
        serde_json::json!({"woke": target.as_str(), "tag": rel.tag}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staging_image_map_uses_ppt_prefix_for_all_images() {
        // Pinned invariant: every backend AND frontend image is published under
        // the `ppt-` prefix in GHCR (`ppt-api-server`, `ppt-reality-server`,
        // `ppt-web`, `ppt-reality-web`, `ppt-admin-web`). The **service key**
        // stays `reality-web` / `admin-web` (so promote/wake/release callers
        // don't churn) but the **image** path matches docker-frontend.yml's
        // `target: ppt-*` matrix entry.
        let images = build_staging_images("ghcr.io/test", "abc1234");
        assert_eq!(images["api-server"], "ghcr.io/test/ppt-api-server:abc1234");
        assert_eq!(
            images["reality-server"],
            "ghcr.io/test/ppt-reality-server:abc1234"
        );
        assert_eq!(images["ppt-web"], "ghcr.io/test/ppt-web:abc1234");
        assert_eq!(
            images["reality-web"],
            "ghcr.io/test/ppt-reality-web:abc1234"
        );
        assert_eq!(images["admin-web"], "ghcr.io/test/ppt-admin-web:abc1234");
        assert_eq!(images.len(), 5);
    }
}
