// backend/servers/deploy-server/src/api/release.rs
use crate::config::TargetsConfig;
use crate::domain::{Release, ReleaseState, TargetKind};
use crate::infra::{
    BlueGreenDeployer, BlueGreenSpec, CaddyClient, CallerIdentity, DockerClient, StagingDeploySpec,
    Store,
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

pub async fn deploy_handler(
    State(svc): State<Arc<ReleaseService>>,
    axum::Extension(caller): axum::Extension<CallerIdentity>,
    Json(req): Json<DeployRequest>,
) -> Result<Json<Release>> {
    caller.require_scope("release:deploy")?;
    let target = TargetKind::from_str(&req.target)
        .map_err(|e| DeployError::Config(format!("invalid target: {e}")))?;
    if target != TargetKind::Staging {
        return Err(DeployError::BadRequest(format!(
            "target {target} not supported in Phase 2 (prod is Phase 4)",
        )));
    }
    let target_cfg = svc
        .targets
        .targets
        .get(target.as_str())
        .ok_or_else(|| DeployError::Config(format!("unknown target {target}")))?;

    let mut images = HashMap::new();
    images.insert(
        "api-server".into(),
        format!("{}/ppt-api-server:{}", svc.image_prefix, req.tag),
    );
    images.insert(
        "reality-server".into(),
        format!("{}/ppt-reality-server:{}", svc.image_prefix, req.tag),
    );
    images.insert(
        "ppt-web".into(),
        format!("{}/ppt-web:{}", svc.image_prefix, req.tag),
    );
    // Note: docker-frontend.yml pushes `ppt-web` and `reality-web` (without `ppt-` prefix on reality-web).
    // Backend images use `ppt-api-server` and `ppt-reality-server` (with prefix). Workflow inconsistency,
    // matching what's actually published to GHCR.
    images.insert(
        "reality-web".into(),
        format!("{}/reality-web:{}", svc.image_prefix, req.tag),
    );

    let spec = StagingDeploySpec {
        tag: req.tag.clone(),
        api_image: images["api-server"].clone(),
        reality_image: images["reality-server"].clone(),
        ppt_web_image: images["ppt-web"].clone(),
        reality_web_image: images["reality-web"].clone(),
        domain_suffix: target_cfg.domain_suffix.clone(),
        target_name: target.as_str().into(),
    };
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
    let spec = BlueGreenSpec::from_release(&rel, target.as_str(), &target_cfg.domain_suffix);
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
