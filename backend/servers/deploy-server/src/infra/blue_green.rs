// backend/servers/deploy-server/src/infra/blue_green.rs
use crate::infra::{CaddyClient, DockerClient};
use crate::Result;
use bollard::container::{
    Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::models::HostConfig;
use bollard::Docker;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;

/// Service names participating in blue/green deploys.
/// Used by the deployer itself and by lifecycle code (gc, etc.) that needs to
/// enumerate per-target containers without hardcoding the list.
pub const BG_SERVICES: &[&str] = &["api", "reality", "ppt", "reality-web"];

pub struct BlueGreenDeployer {
    pub docker: Arc<DockerClient>,
    pub caddy: Arc<CaddyClient>,
}

#[derive(Debug, Clone)]
pub struct BlueGreenSpec {
    pub tag: String,
    pub api_image: String,
    pub reality_image: String,
    pub ppt_web_image: String,
    pub reality_web_image: String,
    /// Reality Portal apex (e.g. `rlt.sk`, `staging.rlt.sk`). reality-web is
    /// served at this exact host; reality-server at `api.<reality_apex>`.
    pub reality_apex: String,
    /// Property Management apex (e.g. `ppt.rlt.sk`, `staging.ppt.rlt.sk`).
    /// ppt-web is served at this exact host; api-server at `api.<ppt_apex>`.
    pub ppt_apex: String,
    pub target_name: String,
}

impl BlueGreenSpec {
    /// Build a deploy spec from a Release row + target config.
    /// Use this instead of inlining the boilerplate in deploy/promote/rollback handlers.
    /// Fails with a clear error if any required service image is missing — better than
    /// running `docker pull` on an empty ref and getting cryptic registry errors.
    pub fn from_release(
        rel: &crate::domain::Release,
        target_name: &str,
        target: &crate::config::Target,
    ) -> crate::Result<Self> {
        fn require(rel: &crate::domain::Release, key: &str) -> crate::Result<String> {
            rel.images.get(key).cloned().ok_or_else(|| {
                crate::DeployError::BadRequest(format!(
                    "release {} is missing image for service '{}'",
                    rel.tag, key
                ))
            })
        }
        Ok(Self {
            tag: rel.tag.clone(),
            target_name: target_name.to_string(),
            api_image: require(rel, "api-server")?,
            reality_image: require(rel, "reality-server")?,
            ppt_web_image: require(rel, "ppt-web")?,
            reality_web_image: require(rel, "reality-web")?,
            reality_apex: target.reality_apex.clone(),
            ppt_apex: target.ppt_apex.clone(),
        })
    }
}

impl BlueGreenDeployer {
    pub async fn deploy(&self, spec: &BlueGreenSpec) -> Result<()> {
        let docker = self.docker.bollard();
        for img in [
            &spec.api_image,
            &spec.reality_image,
            &spec.ppt_web_image,
            &spec.reality_web_image,
        ] {
            self.pull_image(docker, img).await?;
        }

        // Check how many of each color is running. Pick the OPPOSITE color of whichever
        // has more services running. If tied (everything down or split), default to "blue".
        let target_name = &spec.target_name;
        let mut blue_count = 0u8;
        let mut green_count = 0u8;
        for service in BG_SERVICES {
            if self
                .docker
                .is_running(&format!("{target_name}-{service}-blue"))
                .await
                .unwrap_or(false)
            {
                blue_count += 1;
            }
            if self
                .docker
                .is_running(&format!("{target_name}-{service}-green"))
                .await
                .unwrap_or(false)
            {
                green_count += 1;
            }
        }

        // Decide next_color: the color that has FEWER (or no) live services.
        // Tie-breaker: "blue" for first deploy AND for mixed state (split-recovery).
        // We log the mixed-state path explicitly because it's an unusual recovery action.
        let next_color = if blue_count > green_count {
            "green"
        } else if green_count > blue_count {
            "blue"
        } else {
            // blue_count == green_count: either both 0 (cold start) or both > 0 (mixed state).
            if blue_count > 0 {
                tracing::warn!(
                    target = %target_name,
                    blue_count,
                    green_count,
                    "blue/green target is in mixed state — both colors have running services; recovering by deploying blue"
                );
            }
            "blue"
        };
        let prev_color = if next_color == "blue" {
            "green"
        } else {
            "blue"
        };

        self.run_service(
            &format!("{target_name}-api-{next_color}"),
            &spec.api_image,
            8080,
            target_name,
        )
        .await?;
        self.run_service(
            &format!("{target_name}-reality-{next_color}"),
            &spec.reality_image,
            8081,
            target_name,
        )
        .await?;
        self.run_service(
            &format!("{target_name}-ppt-{next_color}"),
            &spec.ppt_web_image,
            80,
            target_name,
        )
        .await?;
        self.run_service(
            &format!("{target_name}-reality-web-{next_color}"),
            &spec.reality_web_image,
            3000,
            target_name,
        )
        .await?;

        // Wait for each container to reach a ready state before flipping Caddy upstream.
        self.wait_until_ready(&format!("{target_name}-api-{next_color}"), 8080, 30)
            .await?;
        self.wait_until_ready(&format!("{target_name}-reality-{next_color}"), 8081, 30)
            .await?;
        self.wait_until_ready(&format!("{target_name}-ppt-{next_color}"), 80, 30)
            .await?;
        self.wait_until_ready(&format!("{target_name}-reality-web-{next_color}"), 3000, 30)
            .await?;

        // Per-service Caddy routes use the dual-apex layout:
        //   <reality_apex>           → reality-web   (Reality Portal UI, bare apex)
        //   api.<reality_apex>       → reality-server (Reality public API)
        //   <ppt_apex>               → ppt-web       (Property Management UI)
        //   api.<ppt_apex>           → api-server    (PM API)
        // For prod with reality_apex="rlt.sk" + ppt_apex="ppt.rlt.sk" this gives
        // rlt.sk / api.rlt.sk / ppt.rlt.sk / api.ppt.rlt.sk respectively.
        let reality_apex = &spec.reality_apex;
        let ppt_apex = &spec.ppt_apex;
        self.caddy
            .register_route(
                &format!("api.{ppt_apex}"),
                &format!("{target_name}-api-{next_color}:8080"),
            )
            .await?;
        self.caddy
            .register_route(
                &format!("api.{reality_apex}"),
                &format!("{target_name}-reality-{next_color}:8081"),
            )
            .await?;
        self.caddy
            .register_route(ppt_apex, &format!("{target_name}-ppt-{next_color}:80"))
            .await?;
        self.caddy
            .register_route(
                reality_apex,
                &format!("{target_name}-reality-web-{next_color}:3000"),
            )
            .await?;

        let prev_containers: Vec<String> = BG_SERVICES
            .iter()
            .map(|s| format!("{target_name}-{s}-{prev_color}"))
            .collect();
        self.docker.cleanup_containers(&prev_containers).await;
        Ok(())
    }

    async fn pull_image(&self, docker: &Docker, image: &str) -> Result<()> {
        let opts = CreateImageOptions {
            from_image: image.to_string(),
            ..Default::default()
        };
        let mut stream = docker.create_image(Some(opts), None, None);
        while let Some(item) = stream.next().await {
            item.map_err(crate::DeployError::Docker)?;
        }
        Ok(())
    }

    async fn run_service(
        &self,
        name: &str,
        image: &str,
        container_port: u16,
        target: &str,
    ) -> Result<()> {
        let docker = self.docker.bollard();
        let _ = docker
            .remove_container(
                name,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await;

        let mut exposed = HashMap::new();
        exposed.insert(format!("{container_port}/tcp"), HashMap::<(), ()>::new());

        let host_config = HostConfig {
            network_mode: Some(format!("ppt-{target}")),
            restart_policy: Some(bollard::models::RestartPolicy {
                name: Some(bollard::models::RestartPolicyNameEnum::UNLESS_STOPPED),
                ..Default::default()
            }),
            ..Default::default()
        };

        let cfg = Config {
            image: Some(image.to_string()),
            exposed_ports: Some(exposed),
            host_config: Some(host_config),
            ..Default::default()
        };

        let create = docker
            .create_container(
                Some(CreateContainerOptions {
                    name: name.to_string(),
                    platform: None,
                }),
                cfg,
            )
            .await
            .map_err(crate::DeployError::Docker)?;
        docker
            .start_container(&create.id, None::<StartContainerOptions<String>>)
            .await
            .map_err(crate::DeployError::Docker)?;
        Ok(())
    }

    /// Poll the container's runtime state until it's "ready enough" to receive traffic.
    ///
    /// The deploy-server host doesn't share the `ppt-{target}` bridge network with the
    /// staging containers, so we can't TCP-connect to `container_name:port` directly.
    /// Instead, we inspect the container and treat it as ready when:
    ///   - a Docker healthcheck is configured AND has reported HEALTHY, OR
    ///   - no healthcheck exists, but the container has been in `running` state
    ///     for at least `grace` seconds (typical axum/next.js processes bind their
    ///     listener within the first 2-3 seconds).
    ///
    /// This eliminates the worst 502 windows (image still pulling, container exited
    /// immediately) while keeping the dependency surface small. A future pass should
    /// either join the bridge network or expose a host-side port for a real probe.
    async fn wait_until_ready(
        &self,
        container_name: &str,
        _container_port: u16,
        timeout_secs: u64,
    ) -> crate::Result<()> {
        use std::time::Duration;
        use tokio::time::sleep;

        let docker = self.docker.bollard();
        let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);
        let mut first_running_at: Option<std::time::Instant> = None;
        let grace = Duration::from_secs(3);

        while std::time::Instant::now() < deadline {
            match docker.inspect_container(container_name, None).await {
                Ok(info) => {
                    let state = info.state.as_ref();
                    let running = state.and_then(|s| s.running).unwrap_or(false);
                    let health = state.and_then(|s| s.health.as_ref()).and_then(|h| h.status);

                    if running {
                        // If a healthcheck is configured AND it has reported healthy → ready immediately.
                        if matches!(health, Some(bollard::models::HealthStatusEnum::HEALTHY)) {
                            return Ok(());
                        }
                        // No healthcheck (or still starting): running for >=grace seconds → assume ready.
                        let now = std::time::Instant::now();
                        let first = first_running_at.get_or_insert(now);
                        if now.duration_since(*first) >= grace {
                            return Ok(());
                        }
                    } else {
                        first_running_at = None;
                    }
                }
                Err(e) => {
                    tracing::debug!(error = %e, container = %container_name, "inspect during readiness");
                }
            }
            sleep(Duration::from_millis(500)).await;
        }

        Err(crate::DeployError::Internal(format!(
            "container {container_name} not ready within {timeout_secs}s"
        )))
    }
}

// Backward-compat aliases (callers from Phase 2 use the staging names).
pub type StagingDeployer = BlueGreenDeployer;
pub type StagingDeploySpec = BlueGreenSpec;
