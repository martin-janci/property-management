# Deploy Server Phase 5 — Prod on Different Server (Opt-In)

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development.

**Goal:** Make prod deployment target host-agnostic. The deploy server (running on Hetzner) can deploy prod to a different host via SSH-tunneled Docker socket and a remote Caddy admin API. Configuration is per-target in `targets.yaml`. No code rewrite needed when prod migrates — only config + secrets.

**Architecture:** Extends `DockerClient::from_socket` to support `ssh://user@host` URIs (already stubbed but rejected with `Config` error in P1.7). Routes blue-green deploy commands through a remote Docker daemon over SSH. Caddy admin API URL per-target (already in `targets.yaml` as `caddy_url`). Adds a new `staging-elsewhere` target as a smoke pad before flipping prod.

**Spec source:** `docs/superpowers/specs/2026-05-06-deploy-server-design.md` § 10 Phase 5 + locked decision #21 (`targets.yaml` declarative).

---

## File Structure

### Modified
```
backend/servers/deploy-server/src/infra/docker.rs       # implement ssh:// support
backend/servers/deploy-server/src/infra/caddy.rs        # accept per-call URL override (optional)
backend/servers/deploy-server/src/infra/blue_green.rs   # pass-through caddy_url + docker_socket from target
backend/servers/deploy-server/src/api/release.rs        # route deploy/wake to per-target docker
backend/servers/deploy-server/src/api/promote.rs        # route promote/rollback to per-target docker
backend/servers/deploy-server/src/main.rs               # construct per-target DockerClient pool
docs/runbooks/deploy-server-prereqs.md                  # section 12: prod-elsewhere setup
```

---

## Tasks

### Task P5.1: SSH-tunneled Docker socket support

`bollard` 0.16 doesn't natively dial `ssh://` URIs. Two implementation options:

**Option A (recommended for MVP):** Use `ssh -L 2375:/var/run/docker.sock host` to set up a local TCP tunnel that bollard can dial via `Docker::connect_with_http`. The tunnel persists for the lifetime of the deploy server process; restart the tunnel if it drops (maintain a watcher).

**Option B (cleaner long-term):** Use the `russh` crate to dial SSH and stream `docker run`/`docker exec` over the SSH channel directly. Higher complexity.

Implement Option A:

```rust
// backend/servers/deploy-server/src/infra/docker.rs

impl DockerClient {
    pub fn from_socket(docker_socket: &str) -> Result<Self> {
        let docker = if docker_socket.starts_with("unix://") {
            Docker::connect_with_unix(docker_socket, 30, bollard::API_DEFAULT_VERSION)?
        } else if let Some(rest) = docker_socket.strip_prefix("ssh://") {
            // Spawn ssh tunnel + connect via local TCP.
            // Format: ssh://user@host:port (port optional, default 22)
            //   → spawn `ssh -N -L 127.0.0.1:<pick_port>:/var/run/docker.sock user@host`
            let local_port = pick_local_port_for_tunnel(rest);
            spawn_ssh_tunnel(rest, local_port)?;
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
}

fn pick_local_port_for_tunnel(target: &str) -> u16 {
    // Hash target string to a stable port in 22300-22399 range.
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    target.hash(&mut h);
    22300 + (h.finish() % 100) as u16
}

fn spawn_ssh_tunnel(remote: &str, local_port: u16) -> Result<()> {
    use std::process::Stdio;
    // Skip if already running (check by attempting to connect to local_port).
    if std::net::TcpStream::connect(format!("127.0.0.1:{local_port}")).is_ok() {
        return Ok(());
    }
    std::process::Command::new("ssh")
        .args([
            "-N",
            "-L", &format!("127.0.0.1:{local_port}:/var/run/docker.sock"),
            "-o", "StrictHostKeyChecking=accept-new",
            "-o", "ServerAliveInterval=30",
            remote,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| crate::DeployError::Config(format!("ssh tunnel spawn: {e}")))?;
    // Wait briefly for tunnel to come up.
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if std::net::TcpStream::connect(format!("127.0.0.1:{local_port}")).is_ok() {
            return Ok(());
        }
    }
    Err(crate::DeployError::Config(format!("ssh tunnel did not come up on port {local_port}")))
}
```

Note: deploy server's `ppt-deploy` user needs an SSH key + known_hosts entry for the remote host. This is documented in P5.4 runbook.

Compile-check. Commit `feat(deploy-server): ssh:// docker socket via local TCP tunnel`.

---

### Task P5.2: Per-target Docker client pool

The current main.rs constructs one `DockerClient` from the staging target. For multi-target, we need a pool keyed by target name.

```rust
// in main.rs
let mut docker_pool: HashMap<String, Arc<DockerClient>> = HashMap::new();
for (name, target_cfg) in &targets.targets {
    let dc = Arc::new(DockerClient::from_socket(&target_cfg.docker_socket)?);
    docker_pool.insert(name.clone(), dc);
}
let docker_pool = Arc::new(docker_pool);
```

Pass the pool to `ReleaseService` and `PromoteService` so they can pick the right Docker per request.

Update:
```rust
// in api/release.rs ReleaseService:
pub docker_pool: Arc<HashMap<String, Arc<DockerClient>>>,
```

In `deploy_handler` and `wake_handler`, pick the docker:
```rust
let docker = svc.docker_pool.get(&req.target)
    .ok_or_else(|| DeployError::Config(format!("no docker for target {}", req.target)))?
    .clone();
```

Construct `BlueGreenDeployer { docker, caddy: ... }` per-request. (If the per-request construction is expensive, cache per-target deployers in `ReleaseService` — but bollard `Docker` is lightweight and Arc-cloneable.)

Same for `PromoteService::promote_handler` and `rollback_handler`.

Note: `BlueGreenDeployer` currently holds an `Arc<DockerClient>` directly. For per-target, build the deployer inline in the handler rather than at startup.

Compile-check. Commit `feat(deploy-server): per-target Docker client pool`.

---

### Task P5.3: Per-target Caddy client

Same shape as P5.2 — build a `HashMap<String, Arc<CaddyClient>>` from `targets.caddy_url`. Use the matching client for each target's deploy/promote.

Compile. Commit `feat(deploy-server): per-target Caddy client pool`.

---

### Task P5.4: Operator runbook — prod-elsewhere setup

Append section 12 to `docs/runbooks/deploy-server-prereqs.md`:

```markdown
## 12. Prod-on-different-server (Phase 5 opt-in)

When prod migrates to a separate host, follow these steps. Until then, prod and staging share the Hetzner box; this section is only relevant for migration.

### 12.1 Provision new prod host

- New VPS (e.g. cloud provider X). Install Docker.
- Create user `prod-deploy` with `docker` group membership.
- Add deploy server's SSH public key to `~prod-deploy/.ssh/authorized_keys`.

### 12.2 Tighten SSH access

On the deploy server:
```bash
sudo -u ppt-deploy ssh-keygen -t ed25519 -N '' -f /var/lib/ppt-deploy/.ssh/prod-host
```
Copy `prod-host.pub` to the new prod host. Restrict the key in `authorized_keys`:
```
command="docker $SSH_ORIGINAL_COMMAND",no-port-forwarding,no-agent-forwarding,no-X11-forwarding ssh-ed25519 AAAA... prod-host
```
(Or allow only `-L /var/run/docker.sock` forwarding; adjust based on your security posture.)

### 12.3 Caddy on prod host

Install custom Caddy build (same image as Hetzner: `ghcr.io/martin-janci/ppt-caddy:latest`). Bind admin API on Tailnet IP only:

```
{
    admin <tailnet-ip>:2019
}
```

### 12.4 Update `targets.yaml`

```yaml
targets:
  staging:
    docker_socket: unix:///var/run/docker.sock
    caddy_url: http://localhost:2019
    domain_suffix: staging.rlt.sk
    idle_timeout: 8h
    rollback_mode: manual
  prod:
    docker_socket: ssh://prod-deploy@new-prod.rlt.sk
    caddy_url: http://<prod-tailnet-ip>:2019
    domain_suffix: rlt.sk
    promote_strategy: blue-green
    rollback_mode: manual
    health_grace: 60s
  staging-elsewhere:
    # Use this to test the SSH path before flipping prod.
    docker_socket: ssh://prod-deploy@new-prod.rlt.sk
    caddy_url: http://<prod-tailnet-ip>:2019
    domain_suffix: staging.rlt.sk
```

### 12.5 Test on `staging-elsewhere` first

```bash
pmctl deploy staging-elsewhere --tag=v1.2.3
```

Verify backend + frontend respond on `*.staging.rlt.sk` resolved against the new host's Caddy. Once green, do a real prod promote.

### 12.6 Real prod promote

```bash
git tag v1.2.3 && git push --tags        # CI registers candidate
pmctl promote v1.2.3 --target=prod --dry-run
pmctl promote v1.2.3 --target=prod
```

Watch `pmctl logs` and `journalctl -u ppt-deploy.service` for errors.

### 12.7 Rollback

If anything goes wrong:
```bash
pmctl rollback --target=prod
```
This redeploys the previous Release marked `state=previous` for prod.
```

Commit `docs(runbook): Phase 5 prod-elsewhere setup`.

---

### Task P5.5: Smoke test for staging-elsewhere

Conceptually, this would require an SSH-reachable Docker host. For Phase 5 MVP, skip the integration test and rely on manual operator testing per runbook 12.5. Just compile-check ssh:// path is reachable in `DockerClient::from_socket`.

No commit.

---

## Self-Review Coverage

| Spec deliverable (Phase 5) | Plan task |
|---|---|
| `targets.yaml` declarative for prod-elsewhere | P5.4 |
| `docker_socket: ssh://...` support | P5.1 |
| Caddy on remote host with admin API | P5.4 |
| Per-target Docker client pool | P5.2 |
| Per-target Caddy client | P5.3 |
| Test via `staging-elsewhere` before prod | P5.4 (operator runbook) |

Phase 5 deferrals (acceptable):
- Tailscale ACL automation (operator manually configures)
- Multi-region Caddy load balancing (out of scope)
- SSH connection pooling / health-watch (Phase 6 polish)
