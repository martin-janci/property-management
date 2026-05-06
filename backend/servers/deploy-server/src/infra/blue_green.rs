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
    pub domain_suffix: String,
    pub target_name: String,
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

        const SERVICES: &[&str] = &["api", "reality", "ppt", "reality-web"];

        // Check how many of each color is running. Pick the OPPOSITE color of whichever
        // has more services running. If tied (everything down or split), default to "blue".
        let target_name = &spec.target_name;
        let mut blue_count = 0u8;
        let mut green_count = 0u8;
        for service in SERVICES {
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
        // Tie-breaker (both 0 or equal): "blue" — first deploy goes blue.
        let next_color = if blue_count > green_count {
            "green"
        } else if green_count > blue_count {
            "blue"
        } else if blue_count == 0 && green_count == 0 {
            "blue" // first deploy
        } else {
            // Tied with both running — partial mixed state. Pick "blue" to recover but log warning.
            tracing::warn!(
                target = %target_name,
                blue_count,
                green_count,
                "blue/green target is in mixed state — both colors have running services; recovering by deploying blue"
            );
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

        let suffix = &spec.domain_suffix;
        self.caddy
            .register_route(
                &format!("api.{suffix}"),
                &format!("{target_name}-api-{next_color}:8080"),
            )
            .await?;
        self.caddy
            .register_route(
                &format!("reality-api.{suffix}"),
                &format!("{target_name}-reality-{next_color}:8081"),
            )
            .await?;
        self.caddy
            .register_route(
                &format!("ppt.{suffix}"),
                &format!("{target_name}-ppt-{next_color}:80"),
            )
            .await?;
        self.caddy
            .register_route(
                &format!("reality.{suffix}"),
                &format!("{target_name}-reality-web-{next_color}:3000"),
            )
            .await?;

        for service in SERVICES {
            let _ = self
                .docker
                .stop_container(&format!("{target_name}-{service}-{prev_color}"))
                .await;
            let _ = self
                .docker
                .remove_container(&format!("{target_name}-{service}-{prev_color}"))
                .await;
        }
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
}

// Backward-compat aliases (callers from Phase 2 use the staging names).
pub type StagingDeployer = BlueGreenDeployer;
pub type StagingDeploySpec = BlueGreenSpec;
