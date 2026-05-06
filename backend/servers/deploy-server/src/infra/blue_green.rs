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

        let target = &spec.target_name;
        let next_color = if self
            .docker
            .is_running(&format!("{target}-api-blue"))
            .await
            .unwrap_or(false)
        {
            "green"
        } else {
            "blue"
        };
        let prev_color = if next_color == "blue" {
            "green"
        } else {
            "blue"
        };

        self.run_service(
            &format!("{target}-api-{next_color}"),
            &spec.api_image,
            8080,
            target,
        )
        .await?;
        self.run_service(
            &format!("{target}-reality-{next_color}"),
            &spec.reality_image,
            8081,
            target,
        )
        .await?;
        self.run_service(
            &format!("{target}-ppt-{next_color}"),
            &spec.ppt_web_image,
            80,
            target,
        )
        .await?;
        self.run_service(
            &format!("{target}-reality-web-{next_color}"),
            &spec.reality_web_image,
            3000,
            target,
        )
        .await?;

        let suffix = &spec.domain_suffix;
        self.caddy
            .register_route(
                &format!("api.{suffix}"),
                &format!("{target}-api-{next_color}:8080"),
            )
            .await?;
        self.caddy
            .register_route(
                &format!("reality-api.{suffix}"),
                &format!("{target}-reality-{next_color}:8081"),
            )
            .await?;
        self.caddy
            .register_route(
                &format!("ppt.{suffix}"),
                &format!("{target}-ppt-{next_color}:80"),
            )
            .await?;
        self.caddy
            .register_route(
                &format!("reality.{suffix}"),
                &format!("{target}-reality-web-{next_color}:3000"),
            )
            .await?;

        for s in ["api", "reality", "ppt", "reality-web"] {
            let _ = self
                .docker
                .stop_container(&format!("{target}-{s}-{prev_color}"))
                .await;
            let _ = self
                .docker
                .remove_container(&format!("{target}-{s}-{prev_color}"))
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
