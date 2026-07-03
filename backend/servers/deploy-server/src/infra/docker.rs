// backend/servers/deploy-server/src/infra/docker.rs
use crate::Result;
use bollard::container::{
    Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions,
    StopContainerOptions,
};
use bollard::models::{HostConfig, Mount, MountTypeEnum, PortBinding};
use bollard::Docker;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::Instant;

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
    /// Additional `KEY=value` env vars to set on the dev container in addition
    /// to APP/PORT/PNPM_HOME. Used to inject `NEXT_PUBLIC_API_URL` and
    /// `NEXT_PUBLIC_SITE_URL` for reality-web so its `/env.js` route emits
    /// the right values at runtime — without these the dev container's
    /// process.env has no API URL and the client falls back to
    /// `http://localhost:8081`, which is unreachable from a real browser.
    pub extra_env: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BackendDedicatedSpec {
    pub container_name: String,
    pub image: String,
    pub host_port: u16,
    pub container_port: u16, // 8080 (api) or 8081 (reality)
    pub db_url: String,
    pub jwt_secret: String,
    pub extra_env: Vec<String>,
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

    /// Resolve a container's IPv4 address on the default `bridge` network.
    ///
    /// Used to construct Caddy reverse-proxy upstreams that point at the
    /// container directly rather than at `127.0.0.1:<host_port>`. The latter
    /// fails when Caddy itself runs in a container — its loopback is its own,
    /// not the host's. The bridge network is shared with `ppt-caddy` so the
    /// container IP is reachable from there.
    ///
    /// Caveat: bridge IPs are NOT stable across container restarts on the
    /// default bridge network. For full stability, switch worktree+Caddy
    /// containers to a user-defined network and use container *names* as
    /// upstreams (Docker's embedded DNS resolves them); tracked as a
    /// follow-up.
    pub async fn bridge_ip(&self, name: &str) -> Result<String> {
        let info = self.docker.inspect_container(name, None).await?;
        info.network_settings
            .and_then(|ns| {
                ns.networks
                    .and_then(|nets| nets.get("bridge").cloned())
                    .and_then(|n| n.ip_address)
            })
            .filter(|s| !s.is_empty())
            // Distinct error variant (not `Internal`) so the retry loop in
            // `bridge_ip_with_retry` can match this case structurally instead
            // of grepping the message text. A refactor of the message can't
            // silently break retry behavior.
            .ok_or_else(|| crate::DeployError::BridgeIpNotReady(name.to_string()))
    }

    /// Like `bridge_ip`, but polls until the IP is assigned or `timeout` elapses.
    ///
    /// Right after `start_container` Docker may not have assigned a bridge IP yet,
    /// so a direct `bridge_ip` call returns `BridgeIpNotReady(name)`. This helper
    /// polls every 100 ms until either an IP appears or `timeout` is reached. Real
    /// Docker errors (socket gone, container disappeared) propagate immediately —
    /// only the typed `BridgeIpNotReady` variant is retried.
    pub async fn bridge_ip_with_retry(&self, name: &str, timeout: Duration) -> Result<String> {
        let deadline = Instant::now() + timeout;
        loop {
            match self.bridge_ip(name).await {
                Ok(ip) => return Ok(ip),
                Err(e) => {
                    let is_not_ready = matches!(&e, crate::DeployError::BridgeIpNotReady(_));
                    if !is_not_ready || Instant::now() >= deadline {
                        return Err(e);
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    /// Idempotently connect `container` to `network`.
    ///
    /// Caddy in `ppt-caddy` defaults to the host bridge only. To resolve
    /// `prod-api-blue:8080` upstreams over Docker DNS it has to share the
    /// `ppt-prod` network with the backend containers. The deploy-server
    /// brings up new backends on each blue/green flip and registers Caddy
    /// routes that point at their container names — without this hookup
    /// Caddy would 502 on every prod request because DNS wouldn't resolve.
    ///
    /// Inspects the container first; no-ops if already on `network`. Real
    /// errors (container missing, network missing, daemon unreachable)
    /// propagate.
    /// Returns `Ok(true)` if a connect was actually performed; `Ok(false)` if
    /// `container` was already on `network`. Callers (notably the bootstrap
    /// endpoint) report `created` vs `already-ok` to the operator based on
    /// this signal — without it everything looks like a no-op.
    pub async fn ensure_network_membership(&self, network: &str, container: &str) -> Result<bool> {
        let info = self.docker.inspect_container(container, None).await?;
        let already_connected = info
            .network_settings
            .as_ref()
            .and_then(|ns| ns.networks.as_ref())
            .map(|nets| nets.contains_key(network))
            .unwrap_or(false);
        if already_connected {
            return Ok(false);
        }

        use bollard::network::ConnectNetworkOptions;
        self.docker
            .connect_network(
                network,
                ConnectNetworkOptions {
                    container: container.to_string(),
                    ..Default::default()
                },
            )
            .await
            .map_err(crate::DeployError::Docker)?;
        // Generic message: this helper is called for both `ppt-caddy` and
        // `ppt-postgres` (and any future shared service). The earlier
        // hardcoded "connected ppt-caddy to per-target network" was wrong
        // for postgres bootstraps and showed up as misleading log noise.
        tracing::info!(
            network = %network,
            container = %container,
            "connected container to per-target network"
        );
        Ok(true)
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

        let mut env = vec![
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
        env.extend(spec.extra_env.iter().cloned());

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

        // Container needs to reach Postgres via Docker DNS, not host loopback.
        let container_db_url = spec
            .db_url
            .replace("127.0.0.1", "ppt-postgres")
            .replace("localhost", "ppt-postgres");
        let mut env = vec![
            format!("DATABASE_URL={}", container_db_url),
            format!("JWT_SECRET={}", spec.jwt_secret),
            "RUST_LOG=info".to_string(),
            // Dev mode disables strict env-var checks (PM_CLIENT_SECRET,
            // TOTP_ENCRYPTION_KEY, INTEGRATION_ENCRYPTION_KEY) that prod
            // demands but worktree dev doesn't need.
            "RUST_ENV=development".to_string(),
        ];
        env.extend(spec.extra_env.iter().cloned());

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

        // Attach to the network where ppt-postgres lives so DATABASE_URL using
        // `ppt-postgres:5432` actually resolves. Idempotent across re-opens —
        // ensure_network_membership inspects the container first and only
        // connects when needed (no error-string matching).
        if let Err(e) = self
            .ensure_network_membership("ppt-prod", &spec.container_name)
            .await
        {
            tracing::warn!(
                error = %e,
                container = %spec.container_name,
                "failed to attach backend container to ppt-prod network — DB connection will fail"
            );
        }
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
    #[ignore = "requires local docker daemon; run manually with --ignored"]
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
            extra_env: vec![],
        };
        client.run_frontend_dev(&spec).await.unwrap();
        client.stop_container(&spec.container_name).await.unwrap();
        client.remove_container(&spec.container_name).await.unwrap();
    }
}
