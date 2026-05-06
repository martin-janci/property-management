// backend/servers/deploy-server/src/infra/docker.rs
use crate::Result;
use bollard::container::{
    Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions,
    StopContainerOptions,
};
use bollard::models::{HostConfig, Mount, MountTypeEnum, PortBinding};
use bollard::Docker;
use std::collections::HashMap;

pub struct DockerClient {
    docker: Docker,
}

#[derive(Debug, Clone)]
pub struct FrontendDevSpec {
    pub container_name: String,
    pub app: String, // "ppt-web" or "reality-web"
    pub source_path: String,
    pub host_port: u16,
    pub pnpm_volume: String,
    pub image: String,
}

impl DockerClient {
    pub fn from_socket(docker_socket: &str) -> Result<Self> {
        let docker = if docker_socket.starts_with("unix://") {
            Docker::connect_with_unix(docker_socket, 30, bollard::API_DEFAULT_VERSION)?
        } else if docker_socket.starts_with("ssh://") {
            return Err(crate::DeployError::Config(format!(
                "ssh:// docker socket not supported in MVP: {docker_socket}"
            )));
        } else {
            Docker::connect_with_local_defaults()?
        };
        Ok(Self { docker })
    }

    pub fn bollard(&self) -> &Docker {
        &self.docker
    }

    pub async fn is_running(&self, name: &str) -> Result<bool> {
        match self.docker.inspect_container(name, None).await {
            Ok(c) => Ok(c.state.and_then(|s| s.running).unwrap_or(false)),
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            }) => Ok(false),
            Err(e) => Err(crate::DeployError::Docker(e)),
        }
    }

    pub async fn run_frontend_dev(&self, spec: &FrontendDevSpec) -> Result<String> {
        // Idempotency: remove existing container with the same name first.
        let _ = self
            .docker
            .remove_container(
                &spec.container_name,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await;

        let env = vec![
            format!("APP={}", spec.app),
            format!(
                "PORT={}",
                if spec.app == "reality-web" {
                    3000
                } else {
                    5173
                }
            ),
            "PNPM_HOME=/pnpm".to_string(),
        ];

        let container_port = if spec.app == "reality-web" {
            "3000/tcp"
        } else {
            "5173/tcp"
        };

        let mut port_bindings = HashMap::new();
        port_bindings.insert(
            container_port.to_string(),
            Some(vec![PortBinding {
                host_ip: Some("127.0.0.1".to_string()),
                host_port: Some(spec.host_port.to_string()),
            }]),
        );

        let mut exposed_ports = HashMap::new();
        exposed_ports.insert(container_port.to_string(), HashMap::<(), ()>::new());

        let mounts = vec![
            Mount {
                target: Some("/app".to_string()),
                source: Some(spec.source_path.clone()),
                typ: Some(MountTypeEnum::BIND),
                read_only: Some(false),
                ..Default::default()
            },
            Mount {
                target: Some("/pnpm".to_string()),
                source: Some(spec.pnpm_volume.clone()),
                typ: Some(MountTypeEnum::VOLUME),
                ..Default::default()
            },
        ];

        let host_config = HostConfig {
            mounts: Some(mounts),
            port_bindings: Some(port_bindings),
            restart_policy: Some(bollard::models::RestartPolicy {
                name: Some(bollard::models::RestartPolicyNameEnum::UNLESS_STOPPED),
                ..Default::default()
            }),
            ..Default::default()
        };

        let config = Config {
            image: Some(spec.image.clone()),
            env: Some(env),
            exposed_ports: Some(exposed_ports),
            host_config: Some(host_config),
            ..Default::default()
        };

        let create = self
            .docker
            .create_container(
                Some(CreateContainerOptions {
                    name: spec.container_name.clone(),
                    platform: None,
                }),
                config,
            )
            .await?;

        self.docker
            .start_container(&create.id, None::<StartContainerOptions<String>>)
            .await?;

        Ok(create.id)
    }

    pub async fn stop_container(&self, name: &str) -> Result<()> {
        let _ = self
            .docker
            .stop_container(name, Some(StopContainerOptions { t: 10 }))
            .await;
        Ok(())
    }

    pub async fn remove_container(&self, name: &str) -> Result<()> {
        let _ = self
            .docker
            .remove_container(
                name,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test against local docker daemon. Skipped in CI; run manually with --ignored.
    #[tokio::test]
    #[ignore]
    async fn run_frontend_dev_against_local_docker() {
        let client = DockerClient::from_socket("unix:///var/run/docker.sock").unwrap();
        let spec = FrontendDevSpec {
            container_name: "ppt-deploy-test-fe".into(),
            app: "ppt-web".into(),
            source_path: std::env::current_dir()
                .unwrap()
                .parent()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            host_port: 51999,
            pnpm_volume: "ppt-deploy-test-pnpm".into(),
            image: "ppt-frontend-dev:local".into(),
        };
        client.run_frontend_dev(&spec).await.unwrap();
        client.stop_container(&spec.container_name).await.unwrap();
        client.remove_container(&spec.container_name).await.unwrap();
    }
}
