// backend/servers/deploy-server/src/api/bootstrap.rs
//! One-shot target bootstrap.
//!
//! Brings a deploy target's infrastructure prerequisites to the state the
//! preflight validator expects, so the operator doesn't have to remember
//! which `docker network create` / `psql CREATE DATABASE` / `docker network
//! connect` lines to run by hand. Idempotent — re-running on a healthy
//! target reports each step as `already-ok` and exits 0.
//!
//! What this DOES:
//!   - Create `ppt_<target>` from `ppt_dev_template` (skipped if exists).
//!   - Create docker network `ppt-<target>` (skipped if exists).
//!   - Connect `ppt-postgres` to the target network (idempotent).
//!   - Connect `ppt-caddy` to the target network (idempotent).
//!
//! What this DOESN'T do (deferred to the first deploy / `pmctl deploy`):
//!   - Pull/start backend or frontend containers — those are the deploy's job.
//!   - Run `ppt-seed` for OAuth client + admin — the api-server container
//!     doesn't exist yet, and `ppt-seed` is shipped inside that image. After
//!     the first successful deploy the operator runs:
//!     `docker exec <target>-api-blue ppt-seed --reality-portal-secret …`
use crate::api::release::ReleaseService;
use crate::config::TargetsConfig;
use crate::infra::{CallerIdentity, PostgresOps};
use crate::{DeployError, Result};
use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use std::sync::Arc;

#[derive(Clone)]
pub struct BootstrapService {
    pub targets: Arc<TargetsConfig>,
    pub postgres: Arc<PostgresOps>,
    /// Reused so `bootstrap` and `deploy` always agree on which Docker /
    /// Caddy clients belong to a target — no chance of the bootstrap
    /// connecting `ppt-caddy` to a network that the deploy then can't see.
    pub release_svc: Arc<ReleaseService>,
}

#[derive(Debug, Serialize)]
pub struct StepResult {
    pub step: &'static str,
    /// One of `created`, `already-ok`, or `failed`. Same vocabulary across
    /// every step so callers can group/colorize uniformly. We don't fail
    /// the whole bootstrap on `failed` — operator gets the full report and
    /// can act on each line independently.
    pub status: &'static str,
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct BootstrapResult {
    pub target: String,
    pub steps: Vec<StepResult>,
}

pub async fn bootstrap_handler(
    State(svc): State<Arc<BootstrapService>>,
    axum::Extension(caller): axum::Extension<CallerIdentity>,
    Path(target_name): Path<String>,
) -> Result<Json<BootstrapResult>> {
    // Same scope as deploy — anyone who can deploy a target can prepare it.
    caller.require_scope("release:deploy")?;

    // Defense-in-depth: the target name flows into a Postgres identifier
    // (`CREATE DATABASE "ppt_<name>" …`) and a Docker network name. We
    // already trust `targets.yaml` (operator-controlled), but constraining
    // to `[a-z0-9_-]+` rejects anything that could change SQL parsing or
    // produce an invalid Docker resource — cheaper to check here than to
    // chase a mojibake'd identifier through psql later.
    validate_target_name(&target_name)?;

    if !svc.targets.targets.contains_key(&target_name) {
        return Err(DeployError::NotFound(format!(
            "no target '{target_name}' in targets.yaml; add a `{target_name}:` block first"
        )));
    }

    let target_db = format!("ppt_{target_name}");
    let target_network = format!("ppt-{target_name}");

    let mut steps = Vec::with_capacity(4);

    // ── 1. Postgres database ────────────────────────────────────────────
    steps.push(ensure_database(&svc.postgres, &target_db).await);

    // ── 2. Docker network ───────────────────────────────────────────────
    let docker = svc
        .release_svc
        .docker_pool
        .get(&target_name)
        .ok_or_else(|| DeployError::Config(format!("no docker for target '{target_name}'")))?;
    steps.push(ensure_network(docker, &target_network).await);

    // ── 3. ppt-postgres ⇆ target network ────────────────────────────────
    steps.push(
        ensure_membership(
            docker,
            &target_network,
            "ppt-postgres",
            "postgres ⇆ network",
        )
        .await,
    );

    // ── 4. ppt-caddy ⇆ target network ───────────────────────────────────
    steps.push(ensure_membership(docker, &target_network, "ppt-caddy", "caddy ⇆ network").await);

    Ok(Json(BootstrapResult {
        target: target_name,
        steps,
    }))
}

async fn ensure_database(pg: &PostgresOps, db_name: &str) -> StepResult {
    // Cheap existence check via `psql -tAc "SELECT 1 FROM pg_database WHERE
    // datname = …"` — same pattern as the preflight validator. Avoids the
    // cost of a CREATE-DATABASE that fails halfway through if the template
    // is in use by another connection.
    match db_exists(&pg.admin_url, db_name).await {
        Ok(true) => StepResult {
            step: "database",
            status: "already-ok",
            detail: format!("'{db_name}' already exists"),
        },
        Ok(false) => match pg.create_from_template(db_name).await {
            Ok(()) => StepResult {
                step: "database",
                status: "created",
                detail: format!("'{db_name}' created from template '{}'", pg.template_db),
            },
            Err(e) => StepResult {
                step: "database",
                status: "failed",
                detail: format!("CREATE DATABASE '{db_name}' failed: {e}"),
            },
        },
        Err(e) => StepResult {
            step: "database",
            status: "failed",
            detail: format!("could not check existence of '{db_name}': {e}"),
        },
    }
}

async fn ensure_network(docker: &crate::infra::DockerClient, name: &str) -> StepResult {
    use bollard::network::{CreateNetworkOptions, InspectNetworkOptions};
    match docker
        .bollard()
        .inspect_network(name, None::<InspectNetworkOptions<&str>>)
        .await
    {
        Ok(_) => StepResult {
            step: "network",
            status: "already-ok",
            detail: format!("'{name}' already exists"),
        },
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => match docker
            .bollard()
            .create_network(CreateNetworkOptions {
                name: name.to_string(),
                driver: "bridge".into(),
                ..Default::default()
            })
            .await
        {
            Ok(_) => StepResult {
                step: "network",
                status: "created",
                detail: format!("'{name}' created (driver=bridge)"),
            },
            Err(e) => StepResult {
                step: "network",
                status: "failed",
                detail: format!("create network '{name}': {e}"),
            },
        },
        Err(e) => StepResult {
            step: "network",
            status: "failed",
            detail: format!("inspect network '{name}': {e}"),
        },
    }
}

async fn ensure_membership(
    docker: &crate::infra::DockerClient,
    network: &str,
    container: &str,
    label: &'static str,
) -> StepResult {
    match docker.ensure_network_membership(network, container).await {
        Ok(true) => StepResult {
            step: label,
            status: "created",
            detail: format!("connected '{container}' to '{network}'"),
        },
        Ok(false) => StepResult {
            step: label,
            status: "already-ok",
            detail: format!("'{container}' already on '{network}'"),
        },
        Err(e) => StepResult {
            step: label,
            status: "failed",
            detail: format!("connect '{container}' to '{network}': {e}"),
        },
    }
}

/// Reject any target name that would be unsafe to interpolate into a
/// Postgres identifier or Docker resource name. The set is intentionally
/// narrow — `[a-z0-9_-]+`, no leading dash or underscore — to keep the
/// generated names predictable across both surfaces.
fn validate_target_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 32 {
        return Err(DeployError::BadRequest(format!(
            "target name must be 1–32 characters (got {})",
            name.len()
        )));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    {
        return Err(DeployError::BadRequest(format!(
            "target name '{name}' must match [a-z0-9_-]+"
        )));
    }
    if name.starts_with('-') || name.starts_with('_') {
        return Err(DeployError::BadRequest(format!(
            "target name '{name}' must not start with '-' or '_'"
        )));
    }
    Ok(())
}

async fn db_exists(admin_url: &str, db_name: &str) -> std::result::Result<bool, String> {
    let escaped = db_name.replace('\'', "''");
    let sql = format!("SELECT 1 FROM pg_database WHERE datname = '{}'", escaped);
    let out = tokio::process::Command::new("psql")
        .args(["-tAc", &sql, admin_url])
        .output()
        .await
        .map_err(|e| format!("psql exec: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "psql exited {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim() == "1")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_accepts_typical_names() {
        for ok in ["staging", "prod", "dev-1", "wt_42", "a"] {
            assert!(validate_target_name(ok).is_ok(), "{ok} should pass");
        }
    }

    #[test]
    fn validate_rejects_unsafe_names() {
        for bad in [
            "",
            "Staging",       // uppercase
            "prod;DROP",     // SQL meta
            "prod\"",        // quote
            "prod space",    // whitespace
            "-prod",         // leading dash
            "_prod",         // leading underscore
            &"x".repeat(33), // too long
            "🎉",            // non-ASCII
        ] {
            assert!(validate_target_name(bad).is_err(), "{bad:?} should fail");
        }
    }
}
