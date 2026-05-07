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

#[derive(Debug, Clone)]
pub struct BackendDedicatedSpec {
    pub container_name: String,
    pub image: String,
    pub host_port: u16,
    pub container_port: u16, // 8080 (api) or 8081 (reality)
    pub db_url: String,
    pub jwt_secret: String,
}

impl DockerClient {
    pub fn from_socket(docker_socket: &str) -> Result<Self> {
        let docker = if docker_socket.starts_with("unix://") {
            Docker::connect_with_unix(docker_socket, 30, bollard::API_DEFAULT_VERSION)?
        } else if docker_socket.starts_with("ssh://") {
            let url = url::Url::parse(docker_socket)
                .map_err(|e| crate::DeployError::Config(format!("invalid ssh URI: {e}")))?;
            let user = url.username();
            let host = url.host_str().ok_or_else(|| {
                crate::DeployError::Config(format!("ssh URI missing host: {docker_socket}"))
            })?;
            let port = url.port().unwrap_or(22);

            // Strict validation: user and host must be alphanumeric + .-_ only.
            // No leading dash (option-injection), no shell metacharacters.
            let user_ok = user.is_empty()
                || user
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
            let host_ok = host
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
            if !user_ok || !host_ok || user.starts_with('-') || host.starts_with('-') {
                return Err(crate::DeployError::Config(format!(
                    "ssh URI user/host contains disallowed characters: {docker_socket}"
                )));
            }

            let local_port = pick_local_port_for_tunnel(docker_socket);
            spawn_ssh_tunnel(user, host, port, local_port)?;
            Docker::connect_with_http(
                &format!("tcp://127.0.0.1:{local_port}"),
                30,
                bollard::API_DEFAULT_VERSION,
            )?
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

    pub async fn run_backend_dedicated(&self, spec: &BackendDedicatedSpec) -> Result<String> {
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
            format!("DATABASE_URL={}", spec.db_url),
            format!("JWT_SECRET={}", spec.jwt_secret),
            "RUST_LOG=info".to_string(),
        ];

        let port_str = format!("{}/tcp", spec.container_port);
        let mut port_bindings = HashMap::new();
        port_bindings.insert(
            port_str.clone(),
            Some(vec![PortBinding {
                host_ip: Some("127.0.0.1".to_string()),
                host_port: Some(spec.host_port.to_string()),
            }]),
        );

        let mut exposed_ports = HashMap::new();
        exposed_ports.insert(port_str, HashMap::<(), ()>::new());

        let host_config = HostConfig {
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

    /// Best-effort stop+remove of a list of containers.
    /// Logs each individual failure but never returns an error — used during cleanup paths
    /// where progress matters more than completeness.
    pub async fn cleanup_containers(&self, names: &[String]) {
        for name in names {
            if let Err(e) = self
                .docker
                .stop_container(name, Some(StopContainerOptions { t: 10 }))
                .await
            {
                tracing::debug!(
                    container = %name,
                    error = %e,
                    "stop_container failed during cleanup (ignored)"
                );
            }
            if let Err(e) = self
                .docker
                .remove_container(
                    name,
                    Some(RemoveContainerOptions {
                        force: true,
                        ..Default::default()
                    }),
                )
                .await
            {
                tracing::debug!(
                    container = %name,
                    error = %e,
                    "remove_container failed during cleanup (ignored)"
                );
            }
        }
    }
}

/// Process-wide cache mapping `ssh://...` Docker socket URIs to the local
/// loopback port we already opened a tunnel on. Without this, every call to
/// `DockerClient::from_socket` for the same target would pick a new ephemeral
/// port and call `spawn_ssh_tunnel`, accumulating orphan `ssh -N -L` processes
/// (each pinned to a different local port that nothing reuses).
///
/// Using `Mutex<HashMap>` instead of `OnceLock<DashMap>` because the lookup is
/// cold-path (only runs at client construction) and we want the simplest
/// dependency-free primitive.
fn tunnel_port_registry() -> &'static std::sync::Mutex<std::collections::HashMap<String, u16>> {
    static REGISTRY: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, u16>>> =
        std::sync::OnceLock::new();
    REGISTRY.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

fn pick_local_port_for_tunnel(target: &str) -> u16 {
    let registry = tunnel_port_registry();

    // Reuse a previously-allocated port for the same ssh target so the existing
    // tunnel is reused (`spawn_ssh_tunnel` is idempotent — it short-circuits if
    // the local port is already accepting connections).
    if let Ok(map) = registry.lock() {
        if let Some(&port) = map.get(target) {
            return port;
        }
    }

    // First call for this target — probe for a free local port. Closing the
    // listener releases it before ssh -L binds, so the standard race window
    // applies — ssh will fail loudly if a different process snatches it in
    // between (#11).
    use std::net::TcpListener;
    let port = if let Ok(listener) = TcpListener::bind("127.0.0.1:0") {
        listener.local_addr().map(|a| a.port()).unwrap_or(22300)
    } else {
        22300
    };

    // Cache the mapping. `entry().or_insert()` handles the race where two
    // threads probe in parallel — the second insert is a no-op and we return
    // whichever port "won". That's fine because `spawn_ssh_tunnel` short-
    // circuits when the cached port already responds.
    if let Ok(mut map) = registry.lock() {
        return *map.entry(target.to_string()).or_insert(port);
    }
    port
}

fn spawn_ssh_tunnel(user: &str, host: &str, port: u16, local_port: u16) -> crate::Result<()> {
    use std::process::Stdio;
    if std::net::TcpStream::connect(format!("127.0.0.1:{local_port}")).is_ok() {
        return Ok(());
    }

    // Build ssh args with explicit -p and -l flags rather than user@host concatenation.
    let mut args = vec![
        "-N".to_string(),
        "-L".to_string(),
        format!("127.0.0.1:{local_port}:/var/run/docker.sock"),
        "-o".to_string(),
        "StrictHostKeyChecking=accept-new".to_string(),
        "-o".to_string(),
        "ServerAliveInterval=30".to_string(),
        "-p".to_string(),
        port.to_string(),
    ];
    if !user.is_empty() {
        args.push("-l".to_string());
        args.push(user.to_string());
    }
    args.push(host.to_string());

    std::process::Command::new("ssh")
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| crate::DeployError::Config(format!("ssh tunnel spawn: {e}")))?;

    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if std::net::TcpStream::connect(format!("127.0.0.1:{local_port}")).is_ok() {
            return Ok(());
        }
    }
    Err(crate::DeployError::Config(format!(
        "ssh tunnel did not come up on port {local_port}"
    )))
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
