// backend/servers/deploy-server/src/api/promote.rs
use crate::api::release::ReleaseService;
use crate::config::TargetsConfig;
use crate::domain::ReleaseState;
use crate::infra::{BlueGreenDeployer, BlueGreenSpec, CallerIdentity, HealthProbe};
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
    axum::Extension(caller): axum::Extension<CallerIdentity>,
    Json(req): Json<PromoteRequest>,
) -> Result<Json<PromoteResponse>> {
    caller.require_scope("release:promote")?;
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

    let spec = BlueGreenSpec::from_release(&candidate, &req.target, &target_cfg.domain_suffix);
    let docker = svc
        .release_svc
        .docker_pool
        .get(&req.target)
        .ok_or_else(|| DeployError::Config(format!("no docker for target {}", req.target)))?
        .clone();
    let caddy = svc
        .release_svc
        .caddy_pool
        .get(&req.target)
        .ok_or_else(|| DeployError::Config(format!("no caddy for target {}", req.target)))?
        .clone();
    let deployer = BlueGreenDeployer { docker, caddy };
    deployer.deploy(&spec).await?;

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
                        let prev_spec = BlueGreenSpec::from_release(
                            prev,
                            &req.target,
                            &target_cfg.domain_suffix,
                        );
                        match deployer.deploy(&prev_spec).await {
                            Ok(_) => {
                                tracing::warn!(prev_tag = %prev.tag, "auto-rolled back after health grace failure");
                                return Err(DeployError::Internal(format!(
                                    "health grace failed; AUTO-ROLLED BACK to {}",
                                    prev.tag
                                )));
                            }
                            Err(rollback_err) => {
                                tracing::error!(
                                    error = %rollback_err,
                                    attempted_rollback_tag = %prev.tag,
                                    "AUTO-ROLLBACK FAILED — system is in indeterminate state, manual intervention required"
                                );
                                return Err(DeployError::Internal(format!(
                                    "health grace failed AND auto-rollback to {} ALSO FAILED ({rollback_err}); \
                                     system in indeterminate state — manual intervention required",
                                    prev.tag
                                )));
                            }
                        }
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

    // Demote any older Previous rows for this target before adding a new Previous (#10).
    if let Some(older_prev) = svc
        .release_svc
        .store
        .current_release_for(&req.target, "previous")
        .await?
    {
        // Don't archive the just-promoted candidate or the soon-to-be Previous.
        let about_to_become_prev = prev_release.as_ref().map(|r| r.tag.clone());
        if older_prev.tag != updated.tag && Some(older_prev.tag.clone()) != about_to_become_prev {
            let mut older = older_prev;
            older.state = ReleaseState::Archived;
            svc.release_svc.store.upsert_release(&older).await?;
        }
    }

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
    axum::Extension(caller): axum::Extension<CallerIdentity>,
    Json(req): Json<RollbackRequest>,
) -> Result<Json<PromoteResponse>> {
    caller.require_scope("release:rollback")?;
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

    let spec = BlueGreenSpec::from_release(&target_release, &req.target, &target_cfg.domain_suffix);
    let docker = svc
        .release_svc
        .docker_pool
        .get(&req.target)
        .ok_or_else(|| DeployError::Config(format!("no docker for target {}", req.target)))?
        .clone();
    let caddy = svc
        .release_svc
        .caddy_pool
        .get(&req.target)
        .ok_or_else(|| DeployError::Config(format!("no caddy for target {}", req.target)))?
        .clone();
    let deployer = BlueGreenDeployer { docker, caddy };
    deployer.deploy(&spec).await?;

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

    // Demote any older Previous rows for this target before adding a new Previous (#10).
    if let Some(older_prev) = svc
        .release_svc
        .store
        .current_release_for(&req.target, "previous")
        .await?
    {
        // Don't archive the rolled-back tag (might still be marked as previous in DB)
        // or the row that's about to become the new Previous.
        let about_to_become_prev = current.as_ref().map(|r| r.tag.clone());
        if older_prev.tag != rolled_back.tag && Some(older_prev.tag.clone()) != about_to_become_prev
        {
            let mut older = older_prev;
            older.state = ReleaseState::Archived;
            svc.release_svc.store.upsert_release(&older).await?;
        }
    }

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
