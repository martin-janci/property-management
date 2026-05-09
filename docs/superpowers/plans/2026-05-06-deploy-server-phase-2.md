# Deploy Server Phase 2 — Staging Auto-Deploy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Merging to `main` triggers GHA → builds images → POSTs to deploy server → deploy server pulls new images, swaps staging containers (blue-green), updates Caddy upstream, persists state. Idle timeout 8h pauses staging overnight; `pmctl wake staging` manually resumes.

**Architecture:** Extends Phase 1 server with `POST /api/deploy` and `POST /api/wake/{target}` endpoints. New `release` table rows track which tag is currently live per target. Blue-green swap holds two containers briefly during cutover.

**Tech Stack:** Same as Phase 1 (Rust, axum, bollard, sqlx-sqlite). GitHub Actions extension via `curl` POST after docker-build push.

**Spec source:** [docs/superpowers/specs/2026-05-06-deploy-server-design.md](../specs/2026-05-06-deploy-server-design.md) § 10 Phase 2.

---

## File Structure

### New
```
backend/servers/deploy-server/src/api/release.rs       # POST /api/deploy + /api/wake/{target}
backend/servers/deploy-server/src/infra/staging.rs     # blue-green swap logic
.github/workflows/staging-deploy.yml                    # OR extend docker-build.yml
```

### Modified
```
backend/servers/deploy-server/src/api/mod.rs            # add release module
backend/servers/deploy-server/src/api/router.rs         # add /api/deploy, /api/wake routes
backend/servers/deploy-server/src/api/gc.rs             # add staging idle 8h handling
backend/servers/deploy-server/src/bin/pmctl.rs          # add deploy + wake subcommands
backend/servers/deploy-server/src/infra/store.rs        # add release CRUD
backend/servers/deploy-server/src/domain/release.rs     # ensure ReleaseState helpers
.github/workflows/docker-build.yml                       # POST /api/deploy after image push
```

---

## Tasks

### Task P2.1: Release CRUD in Store

**Files:**
- Modify: `backend/servers/deploy-server/src/infra/store.rs`

- [ ] **Step 1: Add release methods to Store impl**

```rust
// In Store impl block, add:
pub async fn upsert_release(&self, rel: &crate::domain::Release) -> Result<()> {
    let images = serde_json::to_string(&rel.images).unwrap();
    let state = match rel.state {
        crate::domain::ReleaseState::Candidate => "candidate",
        crate::domain::ReleaseState::Staging => "staging",
        crate::domain::ReleaseState::Prod => "prod",
        crate::domain::ReleaseState::Previous => "previous",
    };
    sqlx::query(
        r#"INSERT INTO release (tag, images, state, target, promoted_at, notes)
           VALUES (?, ?, ?, ?, ?, ?)
           ON CONFLICT(tag) DO UPDATE SET
             images=excluded.images, state=excluded.state, target=excluded.target,
             promoted_at=excluded.promoted_at, notes=excluded.notes"#,
    )
    .bind(&rel.tag)
    .bind(images)
    .bind(state)
    .bind(rel.target.as_deref())
    .bind(rel.promoted_at.map(|t| t.timestamp()))
    .bind(rel.notes.as_deref())
    .execute(&self.pool)
    .await?;
    Ok(())
}

pub async fn get_release(&self, tag: &str) -> Result<Option<crate::domain::Release>> {
    let row = sqlx::query_as::<_, ReleaseRow>(
        r#"SELECT tag, images, state, target, promoted_at, notes FROM release WHERE tag = ?"#,
    )
    .bind(tag)
    .fetch_optional(&self.pool).await?;
    row.map(ReleaseRow::into_domain).transpose()
}

pub async fn current_release_for(&self, target: &str, state: &str) -> Result<Option<crate::domain::Release>> {
    let row = sqlx::query_as::<_, ReleaseRow>(
        r#"SELECT tag, images, state, target, promoted_at, notes FROM release
           WHERE target = ? AND state = ? ORDER BY promoted_at DESC LIMIT 1"#,
    )
    .bind(target).bind(state)
    .fetch_optional(&self.pool).await?;
    row.map(ReleaseRow::into_domain).transpose()
}
```

- [ ] **Step 2: Add ReleaseRow struct + into_domain**

```rust
#[derive(sqlx::FromRow)]
struct ReleaseRow {
    tag: String,
    images: String,
    state: String,
    target: Option<String>,
    promoted_at: Option<i64>,
    notes: Option<String>,
}

impl ReleaseRow {
    fn into_domain(self) -> Result<crate::domain::Release> {
        use crate::domain::ReleaseState;
        let state = match self.state.as_str() {
            "candidate" => ReleaseState::Candidate,
            "staging" => ReleaseState::Staging,
            "prod" => ReleaseState::Prod,
            "previous" => ReleaseState::Previous,
            other => return Err(crate::DeployError::Internal(format!("bad release state {other}"))),
        };
        let images: std::collections::HashMap<String, String> = serde_json::from_str(&self.images)
            .map_err(|e| crate::DeployError::Internal(format!("bad images json: {e}")))?;
        Ok(crate::domain::Release {
            tag: self.tag, images, state, target: self.target,
            promoted_at: self.promoted_at.map(|t| chrono::Utc.timestamp_opt(t, 0).unwrap()),
            notes: self.notes,
        })
    }
}
```

(Add `use chrono::TimeZone;` at top of file if not already.)

- [ ] **Step 3: Tests**

```rust
#[tokio::test]
async fn release_upsert_and_get() {
    use crate::domain::{Release, ReleaseState};
    use std::collections::HashMap;
    let dir = tempdir().unwrap();
    let store = Store::open(&dir.path().join("state.db")).await.unwrap();
    let mut images = HashMap::new();
    images.insert("api-server".into(), "ghcr.io/x/api:v1".into());
    let rel = Release {
        tag: "v1.0.0".into(), images, state: ReleaseState::Candidate,
        target: Some("staging".into()), promoted_at: None, notes: None,
    };
    store.upsert_release(&rel).await.unwrap();
    let got = store.get_release("v1.0.0").await.unwrap().unwrap();
    assert_eq!(got.tag, "v1.0.0");
    assert!(matches!(got.state, ReleaseState::Candidate));
}
```

- [ ] **Step 4: Run + commit**

```bash
cd backend && cargo test -p deploy-server infra::store::tests::release
git add backend/servers/deploy-server/src/infra/store.rs
git commit -m "feat(deploy-server): release CRUD in Store"
```

---

### Task P2.2: Staging blue-green deploy logic

**Files:**
- Create: `backend/servers/deploy-server/src/infra/staging.rs`
- Modify: `backend/servers/deploy-server/src/infra/mod.rs`

- [ ] **Step 1: Implement StagingDeployer**

```rust
// backend/servers/deploy-server/src/infra/staging.rs
use crate::infra::{CaddyClient, DockerClient};
use crate::Result;
use bollard::container::{Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions};
use bollard::image::CreateImageOptions;
use bollard::models::{HostConfig, PortBinding};
use bollard::Docker;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;

pub struct StagingDeployer {
    pub docker: Arc<DockerClient>,
    pub caddy: Arc<CaddyClient>,
}

#[derive(Debug, Clone)]
pub struct StagingDeploySpec {
    pub tag: String,
    pub api_image: String,         // "ghcr.io/.../api-server:v1.2.3"
    pub reality_image: String,
    pub ppt_web_image: String,
    pub reality_web_image: String,
    pub domain_suffix: String,     // "staging.rlt.sk"
}

impl StagingDeployer {
    /// Pull all images for the new tag, then swap containers + Caddy upstream.
    pub async fn deploy(&self, spec: &StagingDeploySpec) -> Result<()> {
        // Pull images first (fail-fast if any missing).
        let docker = self.docker.bollard();
        for img in [&spec.api_image, &spec.reality_image, &spec.ppt_web_image, &spec.reality_web_image] {
            self.pull_image(docker, img).await?;
        }

        // Determine current "color": staging-blue or staging-green.
        let next_color = match self.docker.is_running("staging-api-blue").await {
            Ok(true) => "green",
            _ => "blue",
        };
        let prev_color = if next_color == "blue" { "green" } else { "blue" };

        // Start new containers with the next color.
        self.run_service(&format!("staging-api-{next_color}"), &spec.api_image, 8080).await?;
        self.run_service(&format!("staging-reality-{next_color}"), &spec.reality_image, 8081).await?;
        self.run_service(&format!("staging-ppt-{next_color}"), &spec.ppt_web_image, 80).await?;
        self.run_service(&format!("staging-reality-web-{next_color}"), &spec.reality_web_image, 3000).await?;

        // Update Caddy upstreams to point at new color.
        let suffix = &spec.domain_suffix;
        self.caddy.register_route(&format!("api.{suffix}"), &format!("staging-api-{next_color}:8080")).await?;
        self.caddy.register_route(&format!("reality-api.{suffix}"), &format!("staging-reality-{next_color}:8081")).await?;
        self.caddy.register_route(&format!("ppt.{suffix}"), &format!("staging-ppt-{next_color}:80")).await?;
        self.caddy.register_route(&format!("reality.{suffix}"), &format!("staging-reality-web-{next_color}:3000")).await?;

        // Stop + remove old color (best-effort).
        for s in ["api", "reality", "ppt", "reality-web"] {
            let _ = self.docker.stop_container(&format!("staging-{s}-{prev_color}")).await;
            let _ = self.docker.remove_container(&format!("staging-{s}-{prev_color}")).await;
        }
        Ok(())
    }

    async fn pull_image(&self, docker: &Docker, image: &str) -> Result<()> {
        let opts = CreateImageOptions { from_image: image, ..Default::default() };
        let mut stream = docker.create_image(Some(opts), None, None);
        while let Some(item) = stream.next().await {
            item.map_err(crate::DeployError::Docker)?;
        }
        Ok(())
    }

    async fn run_service(&self, name: &str, image: &str, container_port: u16) -> Result<()> {
        let docker = self.docker.bollard();
        let _ = docker.remove_container(name, Some(RemoveContainerOptions { force: true, ..Default::default() })).await;

        let mut exposed = HashMap::new();
        exposed.insert(format!("{container_port}/tcp"), HashMap::<(), ()>::new());

        let host_config = HostConfig {
            network_mode: Some("ppt-staging".into()),
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

        let create = docker.create_container(
            Some(CreateContainerOptions { name: name.to_string(), platform: None }),
            cfg,
        ).await.map_err(crate::DeployError::Docker)?;
        docker.start_container(&create.id, None::<StartContainerOptions<String>>).await.map_err(crate::DeployError::Docker)?;
        Ok(())
    }
}
```

- [ ] **Step 2: Expose `bollard()` and `is_running()` on DockerClient**

Edit `src/infra/docker.rs` to add:

```rust
impl DockerClient {
    pub fn bollard(&self) -> &Docker { &self.docker }

    pub async fn is_running(&self, name: &str) -> Result<bool> {
        match self.docker.inspect_container(name, None).await {
            Ok(c) => Ok(c.state.and_then(|s| s.running).unwrap_or(false)),
            Err(bollard::errors::Error::DockerResponseServerError { status_code: 404, .. }) => Ok(false),
            Err(e) => Err(crate::DeployError::Docker(e)),
        }
    }
}
```

- [ ] **Step 3: Wire mod.rs**

```rust
// backend/servers/deploy-server/src/infra/mod.rs
pub mod audit;
pub mod caddy;
pub mod docker;
pub mod git;
pub mod staging;
pub mod store;

pub use audit::{auth_and_audit, AuthState, CallerIdentity};
pub use caddy::CaddyClient;
pub use docker::{DockerClient, FrontendDevSpec};
pub use git::GitFetcher;
pub use staging::{StagingDeployer, StagingDeploySpec};
pub use store::Store;
```

- [ ] **Step 4: Compile-check + commit**

```bash
cd backend && cargo check -p deploy-server
git add backend/servers/deploy-server/src/infra/
git commit -m "feat(deploy-server): blue-green staging deploy logic"
```

---

### Task P2.3: `POST /api/deploy` + `POST /api/wake/{target}` handlers

**Files:**
- Create: `backend/servers/deploy-server/src/api/release.rs`
- Modify: `backend/servers/deploy-server/src/api/mod.rs`
- Modify: `backend/servers/deploy-server/src/api/router.rs`
- Modify: `backend/servers/deploy-server/src/main.rs`

- [ ] **Step 1: release.rs handlers**

```rust
// backend/servers/deploy-server/src/api/release.rs
use crate::config::TargetsConfig;
use crate::domain::{Release, ReleaseState};
use crate::infra::{CallerIdentity, StagingDeployer, StagingDeploySpec, Store};
use crate::{DeployError, Result};
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct ReleaseService {
    pub store: Arc<Store>,
    pub deployer: Arc<StagingDeployer>,
    pub targets: Arc<TargetsConfig>,
    pub image_prefix: String,    // "ghcr.io/martin-janci"
}

#[derive(Debug, Deserialize)]
pub struct DeployRequest {
    pub tag: String,
    #[serde(default = "default_target")]
    pub target: String,
}

fn default_target() -> String { "staging".into() }

pub async fn deploy_handler(
    State(svc): State<Arc<ReleaseService>>,
    axum::Extension(_caller): axum::Extension<CallerIdentity>,
    Json(req): Json<DeployRequest>,
) -> Result<Json<Release>> {
    if req.target != "staging" {
        return Err(DeployError::BadRequest(format!(
            "target {} not supported in Phase 2 (prod is Phase 4)", req.target
        )));
    }
    let target_cfg = svc.targets.targets.get(&req.target)
        .ok_or_else(|| DeployError::Config(format!("unknown target {}", req.target)))?;

    let mut images = HashMap::new();
    images.insert("api-server".into(), format!("{}/ppt-api-server:{}", svc.image_prefix, req.tag));
    images.insert("reality-server".into(), format!("{}/ppt-reality-server:{}", svc.image_prefix, req.tag));
    images.insert("ppt-web".into(), format!("{}/ppt-web:{}", svc.image_prefix, req.tag));
    images.insert("reality-web".into(), format!("{}/ppt-reality-web:{}", svc.image_prefix, req.tag));

    let spec = StagingDeploySpec {
        tag: req.tag.clone(),
        api_image: images["api-server"].clone(),
        reality_image: images["reality-server"].clone(),
        ppt_web_image: images["ppt-web"].clone(),
        reality_web_image: images["reality-web"].clone(),
        domain_suffix: target_cfg.domain_suffix.clone(),
    };
    svc.deployer.deploy(&spec).await?;

    let rel = Release {
        tag: req.tag.clone(),
        images,
        state: ReleaseState::Staging,
        target: Some("staging".into()),
        promoted_at: Some(chrono::Utc::now()),
        notes: None,
    };
    svc.store.upsert_release(&rel).await?;
    Ok(Json(rel))
}

pub async fn wake_handler(
    State(svc): State<Arc<ReleaseService>>,
    Path(target): Path<String>,
) -> Result<Json<serde_json::Value>> {
    if target != "staging" {
        return Err(DeployError::BadRequest("only staging supported in Phase 2".into()));
    }
    // Find current release for staging and re-deploy (no image pull needed if already cached).
    let rel = svc.store.current_release_for("staging", "staging").await?
        .ok_or_else(|| DeployError::NotFound("no staging release recorded".into()))?;
    let target_cfg = svc.targets.targets.get("staging")
        .ok_or_else(|| DeployError::Config("staging target missing".into()))?;
    let spec = StagingDeploySpec {
        tag: rel.tag.clone(),
        api_image: rel.images.get("api-server").cloned().unwrap_or_default(),
        reality_image: rel.images.get("reality-server").cloned().unwrap_or_default(),
        ppt_web_image: rel.images.get("ppt-web").cloned().unwrap_or_default(),
        reality_web_image: rel.images.get("reality-web").cloned().unwrap_or_default(),
        domain_suffix: target_cfg.domain_suffix.clone(),
    };
    svc.deployer.deploy(&spec).await?;
    Ok(Json(serde_json::json!({"woke": "staging", "tag": rel.tag})))
}
```

- [ ] **Step 2: Wire api/mod.rs + router**

```rust
// backend/servers/deploy-server/src/api/mod.rs
pub mod gc;
pub mod health;
pub mod release;
pub mod router;
pub mod webhook;
pub mod worktree;
```

In `router.rs`, extend `build()` with a `release_svc: Arc<ReleaseService>` parameter and add routes inside the auth-protected branch:

```rust
.route("/api/deploy", post(release::deploy_handler))
.route("/api/wake/:target", post(release::wake_handler))
```

For state, use `.merge` of a sub-router with `ReleaseService` state, similar to how GC was wired.

- [ ] **Step 3: main.rs — construct ReleaseService**

```rust
let deployer = Arc::new(StagingDeployer { docker: docker.clone(), caddy: caddy.clone() });
let release_svc = Arc::new(ReleaseService {
    store: store.clone(),
    deployer,
    targets: Arc::new(targets.clone()),
    image_prefix: std::env::var("PPT_IMAGE_PREFIX")
        .unwrap_or_else(|_| "ghcr.io/martin-janci".into()),
});
```

Pass to `router::build`.

- [ ] **Step 4: Compile + commit**

```bash
cd backend && cargo build -p deploy-server --bin ppt-deploy
git add backend/servers/deploy-server/src/api/ backend/servers/deploy-server/src/main.rs
git commit -m "feat(deploy-server): POST /api/deploy + /api/wake/{target}"
```

---

### Task P2.4: pmctl deploy + wake subcommands

**Files:**
- Modify: `backend/servers/deploy-server/src/bin/pmctl.rs`

- [ ] **Step 1: Add subcommands**

In the `Cmd` enum:

```rust
/// Deploy a tag to a target (staging only in Phase 2).
Deploy {
    target: String,    // "staging"
    #[arg(long)]
    tag: String,
},
/// Resume a paused target on demand.
Wake { target: String },
```

In the match:

```rust
Cmd::Deploy { target, tag } => {
    let body = serde_json::json!({"tag": tag, "target": target});
    let resp = http.post(format!("{}/api/deploy", cli.url))
        .header("Authorization", &auth)
        .json(&body).send().await?;
    print_resp(resp, cli.json).await?;
}
Cmd::Wake { target } => {
    let resp = http.post(format!("{}/api/wake/{target}", cli.url))
        .header("Authorization", &auth).send().await?;
    print_resp(resp, cli.json).await?;
}
```

- [ ] **Step 2: Build + commit**

```bash
cd backend && cargo build -p deploy-server --bin pmctl
git add backend/servers/deploy-server/src/bin/pmctl.rs
git commit -m "feat(pmctl): deploy + wake subcommands"
```

---

### Task P2.5: GC tick — staging idle 8h handling

**Files:**
- Modify: `backend/servers/deploy-server/src/api/gc.rs`

- [ ] **Step 1: Extend GcReport + tick logic**

Add `paused_targets: Vec<String>` to GcReport.

After the worktree loop, add staging idle check:

```rust
// Check staging containers — pause if idle 8h.
let staging_idle = chrono::Duration::seconds(8 * 3600);
if let Some(rel) = ctx.svc.store.current_release_for("staging", "staging").await? {
    // We don't track per-target traffic in Phase 2; rely on a heuristic:
    // if `promoted_at` is older than 8h AND no recent traffic, stop containers.
    // Phase 6 will add proper traffic tracking via Caddy access log tail.
    if let Some(promoted) = rel.promoted_at {
        if Utc::now() - promoted > staging_idle {
            for color in ["blue", "green"] {
                for service in ["api", "reality", "ppt", "reality-web"] {
                    let _ = ctx.svc.docker.stop_container(&format!("staging-{service}-{color}")).await;
                }
            }
            report.paused_targets.push("staging".into());
        }
    }
}
```

- [ ] **Step 2: Note proper traffic tracking is Phase 6**

Add a doc comment at the top of gc.rs explaining the heuristic limitation.

- [ ] **Step 3: Compile + commit**

```bash
cd backend && cargo build -p deploy-server --bin ppt-deploy
git add backend/servers/deploy-server/src/api/gc.rs
git commit -m "feat(deploy-server): staging idle 8h pause in GC"
```

---

### Task P2.6: GHA hook — POST /api/deploy after image push

**Files:**
- Modify: `.github/workflows/docker-build.yml`

- [ ] **Step 1: Add deploy-trigger step**

After the existing matrix build job, add a new job that runs once after all matrix images are pushed:

```yaml
trigger-staging-deploy:
  needs: build
  if: github.event_name == 'push' && github.ref == 'refs/heads/main'
  runs-on: ubuntu-latest
  permissions:
    id-token: write    # for GH OIDC
    contents: read
  steps:
    - name: Get OIDC token
      uses: actions/github-script@v7
      id: oidc
      with:
        script: |
          const token = await core.getIDToken('ppt-deploy');
          core.setOutput('token', token);

    - name: POST /api/deploy
      env:
        DEPLOY_URL: https://onyx.rlt.sk
        TAG: ${{ github.sha }}
      run: |
        curl -fsS -X POST "$DEPLOY_URL/api/deploy" \
          -H "Authorization: Bearer ${{ steps.oidc.outputs.token }}" \
          -H "Content-Type: application/json" \
          -d "{\"tag\": \"$TAG\", \"target\": \"staging\"}"
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/docker-build.yml
git commit -m "ci: trigger staging deploy after main push"
```

---

### Task P2.7: Frontend image build workflow

The current `docker-build.yml` only builds `api-server` and `reality-server`. The `docker-frontend.yml` builds `ppt-web` and `reality-web` (already exists). For staging deploy to find frontend images at `ghcr.io/.../ppt-web:<sha>`, the frontend workflow must also tag with `${{ github.sha }}`.

**Files:**
- Modify: `.github/workflows/docker-frontend.yml`

- [ ] **Step 1: Verify metadata tags include sha**

Read existing `docker-frontend.yml`. If `metadata-action` already includes `type=sha,prefix=`, no change needed. If not, add it.

- [ ] **Step 2: Add same `trigger-staging-deploy` job here OR ensure it only runs once**

Option A: Add the trigger job to docker-frontend.yml too (then both workflows trigger). This is wasteful.

Option B (recommended): Make `docker-build.yml` wait for `docker-frontend.yml` completion before triggering. Use `workflow_run` event. This is a more complex setup; for MVP, accept that the trigger may fire before frontend images are pushed. Deploy server's `pull_image` will fail-fast and the operator manually retries.

For Phase 2 simplicity, accept the race; document in commit message.

- [ ] **Step 3: Commit if changes made**

```bash
git add .github/workflows/docker-frontend.yml
git commit -m "ci: ensure frontend images tagged with commit sha for staging deploy"
```

---

### Task P2.8: Smoke verification

**Files:**
- Modify: `backend/servers/deploy-server/tests/smoke.rs`

- [ ] **Step 1: Add deploy + wake test**

Extend the existing smoke test to include:

```rust
// After open/list/close, exercise deploy:
let deploy = server.post("/api/deploy")
    .add_header("Authorization", "Bearer test-token")
    .json(&serde_json::json!({"tag": "v0.0.0-test", "target": "staging"}))
    .await;
// In a real test we'd need docker images available; this is #[ignore]'d so OK.
deploy.assert_status_ok();
```

Mark `#[ignore]` so it doesn't break CI without docker daemon + images.

- [ ] **Step 2: Compile-check**

```bash
cd backend && cargo test -p deploy-server --test smoke --no-run
```

- [ ] **Step 3: Commit**

```bash
git add backend/servers/deploy-server/tests/smoke.rs
git commit -m "test(deploy-server): smoke for staging deploy + wake"
```

---

## Self-Review Coverage

| Spec deliverable (Phase 2) | Plan task |
|---|---|
| GHA → POST /api/deploy on main merge | P2.6 |
| Blue-green swap on staging | P2.2 |
| `pmctl deploy staging` | P2.4 |
| `pmctl wake staging` | P2.4 |
| Staging idle 8h pause | P2.5 |
| Resume on demand | P2.3 (wake_handler) |
| Frontend image build for staging | P2.7 |

Phase 2 deliverables NOT in this plan (deferred):
- Per-target traffic tracking (heuristic only here; proper tracking in Phase 6)
- Auto-rollback on staging health failure (Phase 6 polish)
