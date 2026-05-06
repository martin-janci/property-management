// backend/servers/deploy-server/src/api/promote.rs
use crate::api::release::ReleaseService;
use crate::config::TargetsConfig;
use crate::domain::ReleaseState;
use crate::infra::{BlueGreenSpec, HealthProbe};
use crate::{DeployError, Result};
use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Clone)]
pub struct PromoteService {
    pub release_svc: Arc<ReleaseService>,
    pub health: Arc<HealthProbe>,
    pub targets: Arc<TargetsConfig>,
}

#[derive(Debug, Deserialize)]
pub struct PromoteRequest {
    pub tag: String,
    pub target: String,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct PromoteResponse {
    pub previous_tag: Option<String>,
    pub promoted_tag: String,
    pub target: String,
    pub dry_run: bool,
    pub health_grace_passed: bool,
}

pub async fn promote_handler(
    State(svc): State<Arc<PromoteService>>,
    Json(req): Json<PromoteRequest>,
) -> Result<Json<PromoteResponse>> {
    let target_cfg = svc
        .targets
        .targets
        .get(&req.target)
        .ok_or_else(|| DeployError::Config(format!("unknown target {}", req.target)))?;

    let candidate = svc
        .release_svc
        .store
        .get_release(&req.tag)
        .await?
        .ok_or_else(|| DeployError::NotFound(format!("release {}", req.tag)))?;

    let live_state = if req.target == "prod" {
        "prod"
    } else {
        "staging"
    };
    let prev_release = svc
        .release_svc
        .store
        .current_release_for(&req.target, live_state)
        .await?;

    if req.dry_run {
        return Ok(Json(PromoteResponse {
            previous_tag: prev_release.map(|r| r.tag),
            promoted_tag: req.tag.clone(),
            target: req.target.clone(),
            dry_run: true,
            health_grace_passed: false,
        }));
    }

    let spec = BlueGreenSpec {
        tag: candidate.tag.clone(),
        target_name: req.target.clone(),
        api_image: candidate
            .images
            .get("api-server")
            .cloned()
            .unwrap_or_default(),
        reality_image: candidate
            .images
            .get("reality-server")
            .cloned()
            .unwrap_or_default(),
        ppt_web_image: candidate.images.get("ppt-web").cloned().unwrap_or_default(),
        reality_web_image: candidate
            .images
            .get("reality-web")
            .cloned()
            .unwrap_or_default(),
        domain_suffix: target_cfg.domain_suffix.clone(),
    };
    svc.release_svc.deployer.deploy(&spec).await?;

    let new_state = if req.target == "prod" {
        ReleaseState::Prod
    } else {
        ReleaseState::Staging
    };
    let health_grace_passed = if let Some(grace) = &target_cfg.health_grace {
        let secs = parse_duration_secs(grace).unwrap_or(60);
        let url = format!("https://api.{}/health", target_cfg.domain_suffix);
        match svc.health.grace_check(&url, 5, secs).await {
            Ok(()) => true,
            Err(e) => {
                let auto = target_cfg.rollback_mode == "auto";
                tracing::warn!(error = %e, auto = auto, "health grace failed");
                if auto {
                    if let Some(prev) = &prev_release {
                        let prev_spec = BlueGreenSpec {
                            tag: prev.tag.clone(),
                            target_name: req.target.clone(),
                            api_image: prev.images.get("api-server").cloned().unwrap_or_default(),
                            reality_image: prev
                                .images
                                .get("reality-server")
                                .cloned()
                                .unwrap_or_default(),
                            ppt_web_image: prev.images.get("ppt-web").cloned().unwrap_or_default(),
                            reality_web_image: prev
                                .images
                                .get("reality-web")
                                .cloned()
                                .unwrap_or_default(),
                            domain_suffix: target_cfg.domain_suffix.clone(),
                        };
                        let _ = svc.release_svc.deployer.deploy(&prev_spec).await;
                        return Err(DeployError::Internal(format!(
                            "health grace failed; auto-rolled back to {}",
                            prev.tag
                        )));
                    }
                    return Err(DeployError::Internal(
                        "health grace failed; no previous release to roll back to".into(),
                    ));
                }
                false
            }
        }
    } else {
        true
    };

    let mut updated = candidate;
    updated.state = new_state;
    updated.target = Some(req.target.clone());
    updated.promoted_at = Some(chrono::Utc::now());
    svc.release_svc.store.upsert_release(&updated).await?;

    if let Some(mut prev) = prev_release.clone() {
        prev.state = ReleaseState::Previous;
        svc.release_svc.store.upsert_release(&prev).await?;
    }

    Ok(Json(PromoteResponse {
        previous_tag: prev_release.map(|r| r.tag),
        promoted_tag: req.tag.clone(),
        target: req.target.clone(),
        dry_run: false,
        health_grace_passed,
    }))
}

fn parse_duration_secs(s: &str) -> Option<u64> {
    if let Some(n) = s.strip_suffix('s') {
        return n.parse().ok();
    }
    if let Some(n) = s.strip_suffix('m') {
        return n.parse::<u64>().ok().map(|x| x * 60);
    }
    if let Some(n) = s.strip_suffix('h') {
        return n.parse::<u64>().ok().map(|x| x * 3600);
    }
    s.parse().ok()
}

#[derive(Debug, Deserialize)]
pub struct RollbackRequest {
    pub target: String,
    pub to: Option<String>,
}

pub async fn rollback_handler(
    State(svc): State<Arc<PromoteService>>,
    Json(req): Json<RollbackRequest>,
) -> Result<Json<PromoteResponse>> {
    let target_cfg = svc
        .targets
        .targets
        .get(&req.target)
        .ok_or_else(|| DeployError::Config(format!("unknown target {}", req.target)))?;

    // Determine the release to roll back TO.
    let target_release = if let Some(to) = req.to.clone() {
        svc.release_svc
            .store
            .get_release(&to)
            .await?
            .ok_or_else(|| DeployError::NotFound(format!("release {to}")))?
    } else {
        svc.release_svc
            .store
            .current_release_for(&req.target, "previous")
            .await?
            .ok_or_else(|| DeployError::NotFound("no previous release recorded".into()))?
    };

    let live_state = if req.target == "prod" {
        "prod"
    } else {
        "staging"
    };
    let current = svc
        .release_svc
        .store
        .current_release_for(&req.target, live_state)
        .await?;

    let spec = BlueGreenSpec {
        tag: target_release.tag.clone(),
        target_name: req.target.clone(),
        api_image: target_release
            .images
            .get("api-server")
            .cloned()
            .unwrap_or_default(),
        reality_image: target_release
            .images
            .get("reality-server")
            .cloned()
            .unwrap_or_default(),
        ppt_web_image: target_release
            .images
            .get("ppt-web")
            .cloned()
            .unwrap_or_default(),
        reality_web_image: target_release
            .images
            .get("reality-web")
            .cloned()
            .unwrap_or_default(),
        domain_suffix: target_cfg.domain_suffix.clone(),
    };
    svc.release_svc.deployer.deploy(&spec).await?;

    // Promote rolled-back release to live, demote current to Previous.
    let mut rolled_back = target_release;
    rolled_back.state = if req.target == "prod" {
        ReleaseState::Prod
    } else {
        ReleaseState::Staging
    };
    rolled_back.target = Some(req.target.clone());
    rolled_back.promoted_at = Some(chrono::Utc::now());
    let promoted_tag = rolled_back.tag.clone();
    svc.release_svc.store.upsert_release(&rolled_back).await?;

    if let Some(mut cur) = current.clone() {
        cur.state = ReleaseState::Previous;
        svc.release_svc.store.upsert_release(&cur).await?;
    }

    Ok(Json(PromoteResponse {
        previous_tag: current.map(|r| r.tag),
        promoted_tag,
        target: req.target,
        dry_run: false,
        health_grace_passed: true,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_parse() {
        assert_eq!(parse_duration_secs("60"), Some(60));
        assert_eq!(parse_duration_secs("60s"), Some(60));
        assert_eq!(parse_duration_secs("2m"), Some(120));
        assert_eq!(parse_duration_secs("1h"), Some(3600));
        assert_eq!(parse_duration_secs("garbage"), None);
    }
}
