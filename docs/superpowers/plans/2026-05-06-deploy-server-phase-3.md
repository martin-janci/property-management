# Deploy Server Phase 3 — Dedicated Backend per Worktree

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development.

**Goal:** When a developer opts in (`pmctl open feature-X --backend=dedicated --as=<alias>`), the deploy server creates an isolated Postgres database for that worktree (via `CREATE DATABASE ppt_wt_X TEMPLATE ppt_dev_template`), triggers a GHA build for the branch's `api-server`/`reality-server` images, and spawns dedicated backend containers wired to the per-worktree DB. On idle stop, dump the DB; on resume, restore from dump within TTL.

**Architecture:** Adds `infra/postgres.rs` (createdb/dropdb/pg_dump/pg_restore via shell-out), `infra/gh.rs` (GH API client for workflow_dispatch + status polling), and extends `WorktreeService::open_handler` to handle the dedicated path. Adds `pmctl logs` (SSE stream) and `pmctl psql` (SSH tunnel).

**Spec source:** `docs/superpowers/specs/2026-05-06-deploy-server-design.md` § 10 Phase 3.

---

## File Structure

### New
```
backend/servers/deploy-server/src/infra/postgres.rs
backend/servers/deploy-server/src/infra/gh.rs
backend/servers/deploy-server/src/api/logs.rs           # SSE log stream
backend/servers/deploy-server/migrations/0002_release_indices.sql  # if needed
```

### Modified
```
backend/servers/deploy-server/src/api/worktree.rs       # dedicated branch
backend/servers/deploy-server/src/api/mod.rs            # add logs
backend/servers/deploy-server/src/api/router.rs         # add /api/logs/{name} route
backend/servers/deploy-server/src/api/gc.rs             # pg_dump on idle stop, dropdb on TTL
backend/servers/deploy-server/src/bin/pmctl.rs          # logs + psql subcommands
backend/servers/deploy-server/src/config.rs             # postgres + gh sections
backend/servers/deploy-server/src/main.rs               # wire postgres + gh clients
```

---

## Tasks

### Task P3.1: Postgres helper (createdb/dropdb/pg_dump/pg_restore)

**Files:** Create `backend/servers/deploy-server/src/infra/postgres.rs`

```rust
use crate::Result;
use std::path::Path;
use tokio::process::Command;

#[derive(Clone)]
pub struct PostgresOps {
    pub admin_url: String,           // postgres://user:pass@host:5432/postgres (admin db for create/drop)
    pub template_db: String,         // "ppt_dev_template"
    pub user_db_prefix: String,      // "ppt_wt_"
}

impl PostgresOps {
    pub async fn create_from_template(&self, db_name: &str) -> Result<()> {
        run_psql(&self.admin_url, &format!("CREATE DATABASE \"{db_name}\" TEMPLATE \"{}\"", self.template_db)).await
    }

    pub async fn drop(&self, db_name: &str) -> Result<()> {
        // Disconnect users first.
        let _ = run_psql(&self.admin_url, &format!(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{db_name}'"
        )).await;
        run_psql(&self.admin_url, &format!("DROP DATABASE IF EXISTS \"{db_name}\"")).await
    }

    pub async fn dump(&self, db_name: &str, out: &Path) -> Result<()> {
        let url = self.admin_url.replace("/postgres", &format!("/{db_name}"));
        let status = Command::new("pg_dump")
            .args(["-Fc", "-f", out.to_str().unwrap(), "--no-owner", "--no-acl"])
            .arg(&url)
            .status().await?;
        if !status.success() { return Err(crate::DeployError::Internal("pg_dump failed".into())); }
        Ok(())
    }

    pub async fn restore(&self, db_name: &str, dump: &Path) -> Result<()> {
        self.create_from_template(db_name).await?;
        let url = self.admin_url.replace("/postgres", &format!("/{db_name}"));
        let status = Command::new("pg_restore")
            .args(["-d", &url, "--no-owner", "--no-acl", "--clean", "--if-exists"])
            .arg(dump)
            .status().await?;
        if !status.success() { return Err(crate::DeployError::Internal("pg_restore failed".into())); }
        Ok(())
    }
}

async fn run_psql(url: &str, sql: &str) -> Result<()> {
    let status = Command::new("psql").args(["-c", sql, url]).status().await?;
    if !status.success() { return Err(crate::DeployError::Internal(format!("psql failed: {sql}"))); }
    Ok(())
}
```

Wire mod.rs. Compile-check. Commit `feat(deploy-server): postgres ops (create/drop/dump/restore)`.

---

### Task P3.2: GitHub API client (workflow_dispatch + status polling)

**Files:** Create `backend/servers/deploy-server/src/infra/gh.rs`

```rust
use crate::Result;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct GhClient {
    token: String,
    repo: String,         // "martin-janci/property-management"
    http: reqwest::Client,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowRun {
    pub id: u64,
    pub status: String,    // "queued" | "in_progress" | "completed"
    pub conclusion: Option<String>,  // "success" | "failure" | ... when completed
    pub html_url: String,
}

impl GhClient {
    pub fn new(token: impl Into<String>, repo: impl Into<String>) -> Self {
        Self { token: token.into(), repo: repo.into(), http: reqwest::Client::new() }
    }

    /// POST /repos/{owner}/{repo}/actions/workflows/{workflow_id}/dispatches
    /// Returns immediately; the workflow run isn't queryable by id from this response,
    /// so we list runs and pick the most recent matching branch+workflow.
    pub async fn dispatch_workflow(&self, workflow_file: &str, branch: &str) -> Result<()> {
        let url = format!("https://api.github.com/repos/{}/actions/workflows/{}/dispatches", self.repo, workflow_file);
        let resp = self.http.post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "ppt-deploy")
            .json(&serde_json::json!({"ref": branch}))
            .send().await?;
        resp.error_for_status()?;
        Ok(())
    }

    pub async fn latest_run(&self, workflow_file: &str, branch: &str) -> Result<Option<WorkflowRun>> {
        let url = format!(
            "https://api.github.com/repos/{}/actions/workflows/{}/runs?branch={}&per_page=1",
            self.repo, workflow_file, branch
        );
        let resp = self.http.get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "ppt-deploy")
            .send().await?;
        let body: serde_json::Value = resp.error_for_status()?.json().await?;
        let runs = body["workflow_runs"].as_array().cloned().unwrap_or_default();
        if let Some(run) = runs.into_iter().next() {
            let parsed: WorkflowRun = serde_json::from_value(run)
                .map_err(|e| crate::DeployError::Internal(format!("parse run: {e}")))?;
            return Ok(Some(parsed));
        }
        Ok(None)
    }
}
```

Add `gh_api_token` to AuthConfig (already present from P0.6). Wire mod.rs. Compile. Commit `feat(deploy-server): GitHub API client for workflow_dispatch`.

---

### Task P3.3: Wire postgres + gh into config + main

**Files:**
- Modify: `backend/servers/deploy-server/src/config.rs` — add `postgres_admin_url: String` to Config.
- Modify: `backend/servers/deploy-server/src/main.rs` — construct `PostgresOps` and `GhClient`, pass to `WorktreeService`.

`WorktreeService` gets new fields:
```rust
pub postgres: Arc<PostgresOps>,
pub gh: Arc<GhClient>,
pub repo: String,
pub backend_image_prefix: String,
```

Wire from main. Commit `feat(deploy-server): wire postgres + gh clients`.

---

### Task P3.4: open_handler — dedicated backend branch

**Files:** Modify `backend/servers/deploy-server/src/api/worktree.rs`.

Replace the `BackendMode::Dedicated` rejection with the actual implementation:

```rust
let (db_name, dump_path, api_container, reality_container, api_port, reality_port) =
    if matches!(req.backend, BackendMode::Dedicated) {
        let db_name = format!("ppt_wt_{name}");

        // 1. Create DB from template.
        svc.postgres.create_from_template(&db_name).await?;

        // 2. Dispatch GHA workflow to build branch images.
        // Workflow accepts inputs.branch, builds + tags as `branch-<sanitized>` or commit SHA.
        // Phase 3 MVP: rely on existing docker-build.yml `workflow_dispatch` triggered manually
        // (to be added to docker-build.yml in P3.5).
        svc.gh.dispatch_workflow("docker-build.yml", &req.branch).await?;

        // 3. Wait for workflow completion (poll up to 10 min).
        let mut completed = false;
        for _ in 0..60 {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            if let Some(run) = svc.gh.latest_run("docker-build.yml", &req.branch).await? {
                if run.status == "completed" {
                    if run.conclusion.as_deref() != Some("success") {
                        return Err(DeployError::Internal(format!("workflow failed: {}", run.html_url)));
                    }
                    completed = true;
                    break;
                }
            }
        }
        if !completed {
            return Err(DeployError::Internal("workflow timed out after 10 min".into()));
        }

        // 4. Pull + run dedicated backend containers.
        // NOTE: requires backend image to be tagged with branch SHA — workflow contract.
        let api_image = format!("{}/ppt-api-server:branch-{}",
            svc.backend_image_prefix, sanitize(&req.branch));
        let reality_image = format!("{}/ppt-reality-server:branch-{}",
            svc.backend_image_prefix, sanitize(&req.branch));

        let api_port = pick_port(&format!("{name}-api"));
        let reality_port = pick_port(&format!("{name}-reality"));
        let api_c = format!("wt-{name}-api");
        let reality_c = format!("wt-{name}-reality");

        svc.docker.run_backend_dedicated(&BackendDedicatedSpec {
            container_name: api_c.clone(),
            image: api_image,
            host_port: api_port,
            db_url: format!("{}/{}", svc.postgres.admin_url.trim_end_matches("/postgres"), db_name),
            jwt_secret: std::env::var("PPT_JWT_SECRET").unwrap_or_else(|_| "dev-secret-min-32-chars-please-replace".into()),
        }).await?;
        svc.docker.run_backend_dedicated(&BackendDedicatedSpec {
            container_name: reality_c.clone(),
            image: reality_image,
            host_port: reality_port,
            db_url: format!("{}/{}", svc.postgres.admin_url.trim_end_matches("/postgres"), db_name),
            jwt_secret: std::env::var("PPT_JWT_SECRET").unwrap_or_else(|_| "dev-secret-min-32-chars-please-replace".into()),
        }).await?;

        (Some(db_name), None, Some(api_c), Some(reality_c), Some(api_port), Some(reality_port))
    } else {
        (None, None, None, None, None, None)
    };
```

Add `BackendDedicatedSpec` and `run_backend_dedicated` to `infra/docker.rs`:

```rust
#[derive(Debug, Clone)]
pub struct BackendDedicatedSpec {
    pub container_name: String,
    pub image: String,
    pub host_port: u16,
    pub db_url: String,
    pub jwt_secret: String,
}

impl DockerClient {
    pub async fn run_backend_dedicated(&self, spec: &BackendDedicatedSpec) -> Result<String> {
        // Similar to run_frontend_dev but with backend env vars.
        // ... (see existing run_frontend_dev for skeleton)
    }
}
```

Caddy routes for dedicated:
```rust
let host_api = format!("api.wt-{name}.{}", svc.domain_dev_ppt);
svc.caddy.register_route(&host_api, &format!("127.0.0.1:{api_port}")).await?;
```

Update `Worktree.urls.api` to `Some(format!("https://{host_api}"))`.

Commit `feat(deploy-server): dedicated backend branch in worktree open`.

---

### Task P3.5: docker-build.yml — workflow_dispatch input + branch SHA tag

**Files:** Modify `.github/workflows/docker-build.yml`.

Add `workflow_dispatch` input for branch (already present per existing `inputs.push`). Verify `metadata-action` tags include `type=ref,event=branch` so a feature branch produces tag `branch-<branch-name>`. Test by running the workflow_dispatch manually for a feature branch and confirming the tag.

Commit `ci: ensure workflow_dispatch produces branch-tagged images`.

---

### Task P3.6: GC — pg_dump on idle stop, drop on TTL

**Files:** Modify `backend/servers/deploy-server/src/api/gc.rs`.

In the `Paused` arm before `wt.state = Closed`, if `wt.db_name.is_some()`:

```rust
if let Some(ref db) = wt.db_name {
    let dump_path = format!("{}/{}-{}.dump", ctx.cfg.snapshot_dir, wt.name, chrono::Utc::now().timestamp());
    ctx.svc.postgres.dump(db, std::path::Path::new(&dump_path)).await?;
    ctx.svc.postgres.drop(db).await?;
    wt.dump_path = Some(dump_path);
    wt.db_name = None;  // dropped
}
```

In the `Closed` arm at TTL expiry:

```rust
if let Some(ref dump) = wt.dump_path {
    let _ = tokio::fs::remove_file(dump).await;
}
```

Commit `feat(deploy-server): pg_dump on idle stop + cleanup on TTL`.

---

### Task P3.7: Resume from dump in open_handler

In `worktree::open_handler`, before creating a fresh DB, check if there's an existing closed worktree with a dump:

```rust
if let Some(existing) = svc.store.get_worktree(&name).await? {
    if matches!(existing.state, WorktreeState::Closed) {
        if let Some(dump_path) = &existing.dump_path {
            // Resume from dump.
            let db_name = existing.db_name.clone()
                .unwrap_or_else(|| format!("ppt_wt_{name}"));
            svc.postgres.restore(&db_name, std::path::Path::new(dump_path)).await?;
            // ... continue with container start, etc.
        }
    }
}
```

Commit `feat(deploy-server): resume worktree from pg dump on open`.

---

### Task P3.8: pmctl logs (SSE stream)

**Files:**
- Create: `backend/servers/deploy-server/src/api/logs.rs`
- Modify: `backend/servers/deploy-server/src/bin/pmctl.rs` — add `Logs { name, follow, service }` subcommand.

Server `GET /api/logs/{name}?follow=true&service=api`:
```rust
pub async fn handler(
    State(svc): State<Arc<WorktreeService>>,
    Path(name): Path<String>,
    Query(q): Query<LogsQuery>,
) -> Result<Sse<...>> {
    // Use bollard's logs stream.
    // Wrap in tokio_stream → axum SSE response.
}
```

Client uses reqwest event-source-style streaming and prints lines.

Commit `feat(deploy-server): SSE log stream + pmctl logs subcommand`.

---

### Task P3.9: pmctl psql

**Files:** Modify `backend/servers/deploy-server/src/bin/pmctl.rs`.

Add `Psql { name }` subcommand:

```rust
Cmd::Psql { name } => {
    // Get worktree info, extract db_name.
    let resp = http.get(format!("{}/api/worktree/{name}", cli.url))
        .header("Authorization", &auth).send().await?
        .error_for_status()?;
    let wt: serde_json::Value = resp.json().await?;
    let db_name = wt["db_name"].as_str()
        .ok_or_else(|| anyhow::anyhow!("worktree {name} has no dedicated DB"))?;
    
    // Open SSH tunnel + psql interactive session.
    // Phase 3 MVP: just print the command for the user to run manually.
    println!("ssh deploy@hetzner -L 5433:localhost:5432 -N &");
    println!("psql postgres://ppt:<pass>@localhost:5433/{db_name}");
}
```

Commit `feat(pmctl): psql subcommand (manual SSH tunnel for MVP)`.

---

### Task P3.10: Smoke verify dedicated path

Extend `tests/smoke.rs` to test dedicated open (will fail without GHA mock or skip the workflow_dispatch step). Mark `#[ignore]` since it requires GH API + Postgres + Docker.

Commit `test(deploy-server): smoke for dedicated backend path`.

---

## Self-Review Coverage

| Spec deliverable (Phase 3) | Plan task |
|---|---|
| DB-per-worktree (createdb FROM TEMPLATE) | P3.1, P3.4 |
| GHA workflow_dispatch trigger | P3.2, P3.4, P3.5 |
| Backend container per worktree | P3.4 (run_backend_dedicated) |
| pg_dump/restore for resume | P3.6, P3.7 |
| Sync resume in pmctl open (progress) | P3.7 (synchronous restore) |
| pmctl logs <name> [-f] via SSE | P3.8 |
| pmctl psql <name> via SSH tunnel | P3.9 |
| Background warmup (Claude skill) | Skill update — out of plan, manual |

Phase 3 deferrals (acceptable):
- Real-time progress callbacks during workflow polling (just a final result for now)
- pmctl psql auto-tunneled (MVP prints the command)
- Concurrent dedicated worktrees throttling (MVP doesn't limit)
