# Deploy Server Phase 4 — Prod Release Flow

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development.

**Goal:** Pushing a semver tag (`v1.2.3`) triggers GHA → builds release images → POSTs `/api/release` to register a `prod-candidate`. `pmctl promote v1.2.3 --target=prod` performs blue-green swap atomically (Caddy upstream flip + health grace), with `--dry-run` showing exactly what would happen. `pmctl rollback --target=prod [--to=<tag>]` flips back. Auto-rollback on health-check failure is a per-target opt-in (default manual).

**Architecture:** Reuses `StagingDeployer` blue-green logic generalized as `BlueGreenDeployer` (works for any target). Adds `POST /api/release` (CI-friendly), `POST /api/promote`, `POST /api/rollback`. `release` table tracks `state=candidate|prod|previous`. Health-grace = sleep + curl `/health` 5× over 60 s.

**Spec source:** `docs/superpowers/specs/2026-05-06-deploy-server-design.md` § 10 Phase 4 + § 9 (locked decisions on promote/rollback strategy).

---

## File Structure

### New
```
backend/servers/deploy-server/src/api/promote.rs    # promote + rollback handlers
backend/servers/deploy-server/src/infra/health.rs   # /health probe helper for grace check
.github/workflows/release.yml                        # tag-triggered build + register
```

### Modified
```
backend/servers/deploy-server/src/infra/staging.rs  # rename + generalize → blue_green.rs (or keep + add new)
backend/servers/deploy-server/src/api/release.rs    # add register_candidate handler for /api/release
backend/servers/deploy-server/src/api/router.rs     # add /api/release, /api/promote, /api/rollback
backend/servers/deploy-server/src/api/mod.rs        # add promote module
backend/servers/deploy-server/src/bin/pmctl.rs      # add promote + rollback subcommands
backend/servers/deploy-server/src/api/release.rs    # extend ReleaseService for prod target support
```

---

## Tasks

### Task P4.1: Generalize staging → BlueGreenDeployer

Rename `infra/staging.rs` → `infra/blue_green.rs`. Rename `StagingDeployer` → `BlueGreenDeployer`. `StagingDeploySpec` → `BlueGreenSpec` with extra field `target_name: String` (used for container naming `{target}-{service}-{color}` so prod and staging don't collide).

Keep type aliases for backward compatibility:
```rust
pub type StagingDeployer = BlueGreenDeployer;
pub type StagingDeploySpec = BlueGreenSpec;
```

Update internal `staging-api-blue` etc. references to use `{target_name}-...`.

Compile-check. Commit `refactor(deploy-server): rename StagingDeployer → BlueGreenDeployer`.

---

### Task P4.2: Health probe helper

Create `infra/health.rs`:

```rust
use crate::Result;
use std::time::Duration;
use tokio::time::sleep;

pub struct HealthProbe {
    http: reqwest::Client,
}

impl HealthProbe {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build().unwrap(),
        }
    }

    /// Check N times over `total_secs` (one check every `total_secs / N` seconds).
    /// Returns Ok(()) if all checks pass; Err if any fail.
    pub async fn grace_check(&self, url: &str, attempts: u32, total_secs: u64) -> Result<()> {
        let interval = total_secs / attempts as u64;
        for i in 0..attempts {
            sleep(Duration::from_secs(interval)).await;
            let resp = self.http.get(url).send().await
                .map_err(crate::DeployError::Http)?;
            if !resp.status().is_success() {
                return Err(crate::DeployError::Internal(format!(
                    "health check {} failed: {}", i + 1, resp.status()
                )));
            }
        }
        Ok(())
    }
}
```

Wire mod.rs. Commit `feat(deploy-server): HealthProbe helper`.

---

### Task P4.3: POST /api/release (CI registers candidate)

Extend `api/release.rs`. Add `register_candidate_handler`:

```rust
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub tag: String,
    pub images: HashMap<String, String>,    // {api-server: ..., reality-server: ..., ppt-web: ..., reality-web: ...}
    #[serde(default)]
    pub notes: Option<String>,
}

pub async fn register_candidate_handler(
    State(svc): State<Arc<ReleaseService>>,
    axum::Extension(_caller): axum::Extension<CallerIdentity>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<Release>> {
    let rel = Release {
        tag: req.tag,
        images: req.images,
        state: ReleaseState::Candidate,
        target: Some("prod".into()),
        promoted_at: None,
        notes: req.notes,
    };
    svc.store.upsert_release(&rel).await?;
    Ok(Json(rel))
}
```

Wire route `POST /api/release` in router. Commit `feat(deploy-server): POST /api/release for CI candidate registration`.

---

### Task P4.4: Promote handler

Create `api/promote.rs`:

```rust
use crate::api::release::ReleaseService;
use crate::config::TargetsConfig;
use crate::domain::{Release, ReleaseState};
use crate::infra::{BlueGreenSpec, HealthProbe};
use crate::{DeployError, Result};
use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Clone)]
pub struct PromoteService {
    pub release_svc: Arc<ReleaseService>,
    pub health: Arc<HealthProbe>,
    pub targets: Arc<TargetsConfig>,
}

#[derive(Debug, Deserialize)]
pub struct PromoteRequest {
    pub tag: String,
    pub target: String,        // "prod" | "staging"
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct PromoteResponse {
    pub previous_tag: Option<String>,
    pub promoted_tag: String,
    pub target: String,
    pub dry_run: bool,
    pub health_grace_passed: bool,
}

pub async fn promote_handler(
    State(svc): State<Arc<PromoteService>>,
    Json(req): Json<PromoteRequest>,
) -> Result<Json<PromoteResponse>> {
    let target_cfg = svc.targets.targets.get(&req.target)
        .ok_or_else(|| DeployError::Config(format!("unknown target {}", req.target)))?;

    let candidate = svc.release_svc.store.get_release(&req.tag).await?
        .ok_or_else(|| DeployError::NotFound(format!("release {}", req.tag)))?;

    // Find current released for this target.
    let prev_release = svc.release_svc.store
        .current_release_for(&req.target,
            if req.target == "prod" { "prod" } else { "staging" }).await?;

    if req.dry_run {
        return Ok(Json(PromoteResponse {
            previous_tag: prev_release.map(|r| r.tag),
            promoted_tag: req.tag.clone(),
            target: req.target.clone(),
            dry_run: true,
            health_grace_passed: false,
        }));
    }

    // Blue-green swap (reuse deployer with different target_name).
    let spec = BlueGreenSpec {
        tag: candidate.tag.clone(),
        target_name: req.target.clone(),
        api_image: candidate.images.get("api-server").cloned().unwrap_or_default(),
        reality_image: candidate.images.get("reality-server").cloned().unwrap_or_default(),
        ppt_web_image: candidate.images.get("ppt-web").cloned().unwrap_or_default(),
        reality_web_image: candidate.images.get("reality-web").cloned().unwrap_or_default(),
        domain_suffix: target_cfg.domain_suffix.clone(),
    };
    svc.release_svc.deployer.deploy(&spec).await?;

    // Health grace check.
    let new_state = if req.target == "prod" { ReleaseState::Prod } else { ReleaseState::Staging };
    let health_grace_passed = if let Some(grace) = &target_cfg.health_grace {
        let secs = parse_duration_secs(grace).unwrap_or(60);
        let url = format!("https://api.{}/health", target_cfg.domain_suffix);
        match svc.health.grace_check(&url, 5, secs).await {
            Ok(()) => true,
            Err(e) => {
                let auto = target_cfg.rollback_mode == "auto";
                tracing::warn!(error = %e, auto = auto, "health grace failed");
                if auto {
                    if let Some(prev) = &prev_release {
                        let prev_spec = BlueGreenSpec {
                            tag: prev.tag.clone(),
                            target_name: req.target.clone(),
                            api_image: prev.images.get("api-server").cloned().unwrap_or_default(),
                            reality_image: prev.images.get("reality-server").cloned().unwrap_or_default(),
                            ppt_web_image: prev.images.get("ppt-web").cloned().unwrap_or_default(),
                            reality_web_image: prev.images.get("reality-web").cloned().unwrap_or_default(),
                            domain_suffix: target_cfg.domain_suffix.clone(),
                        };
                        let _ = svc.release_svc.deployer.deploy(&prev_spec).await;
                        return Err(DeployError::Internal(format!(
                            "health grace failed; auto-rolled back to {}", prev.tag
                        )));
                    }
                    return Err(DeployError::Internal(format!(
                        "health grace failed; no previous release to roll back to"
                    )));
                }
                false   // manual mode: surface warning, don't fail
            }
        }
    } else {
        true
    };

    // Mark candidate as new state, mark previous as Previous.
    let mut updated = candidate;
    updated.state = new_state;
    updated.target = Some(req.target.clone());
    updated.promoted_at = Some(chrono::Utc::now());
    svc.release_svc.store.upsert_release(&updated).await?;

    if let Some(mut prev) = prev_release.clone() {
        prev.state = ReleaseState::Previous;
        svc.release_svc.store.upsert_release(&prev).await?;
    }

    Ok(Json(PromoteResponse {
        previous_tag: prev_release.map(|r| r.tag),
        promoted_tag: req.tag.clone(),
        target: req.target.clone(),
        dry_run: false,
        health_grace_passed,
    }))
}

fn parse_duration_secs(s: &str) -> Option<u64> {
    if let Some(n) = s.strip_suffix("s") { return n.parse().ok(); }
    if let Some(n) = s.strip_suffix("m") { return n.parse::<u64>().ok().map(|x| x * 60); }
    if let Some(n) = s.strip_suffix("h") { return n.parse::<u64>().ok().map(|x| x * 3600); }
    s.parse().ok()
}
```

Wire mod.rs + router. Commit `feat(deploy-server): promote handler with blue-green + health grace + auto-rollback opt-in`.

---

### Task P4.5: Rollback handler

In `api/promote.rs`:

```rust
#[derive(Debug, Deserialize)]
pub struct RollbackRequest {
    pub target: String,
    pub to: Option<String>,         // explicit tag, else use Previous
}

pub async fn rollback_handler(
    State(svc): State<Arc<PromoteService>>,
    Json(req): Json<RollbackRequest>,
) -> Result<Json<PromoteResponse>> {
    let target_cfg = svc.targets.targets.get(&req.target)
        .ok_or_else(|| DeployError::Config(format!("unknown target {}", req.target)))?;

    let target_release = if let Some(to) = req.to.clone() {
        svc.release_svc.store.get_release(&to).await?
            .ok_or_else(|| DeployError::NotFound(format!("release {to}")))?
    } else {
        svc.release_svc.store
            .current_release_for(&req.target, "previous").await?
            .ok_or_else(|| DeployError::NotFound("no previous release recorded".into()))?
    };

    let current = svc.release_svc.store
        .current_release_for(&req.target,
            if req.target == "prod" { "prod" } else { "staging" }).await?;

    let spec = BlueGreenSpec {
        tag: target_release.tag.clone(),
        target_name: req.target.clone(),
        api_image: target_release.images.get("api-server").cloned().unwrap_or_default(),
        reality_image: target_release.images.get("reality-server").cloned().unwrap_or_default(),
        ppt_web_image: target_release.images.get("ppt-web").cloned().unwrap_or_default(),
        reality_web_image: target_release.images.get("reality-web").cloned().unwrap_or_default(),
        domain_suffix: target_cfg.domain_suffix.clone(),
    };
    svc.release_svc.deployer.deploy(&spec).await?;

    // Update state: rolled-back becomes the live one, current becomes Previous.
    let mut rolled_back = target_release;
    rolled_back.state = if req.target == "prod" { ReleaseState::Prod } else { ReleaseState::Staging };
    rolled_back.target = Some(req.target.clone());
    rolled_back.promoted_at = Some(chrono::Utc::now());
    svc.release_svc.store.upsert_release(&rolled_back).await?;

    if let Some(mut cur) = current.clone() {
        cur.state = ReleaseState::Previous;
        svc.release_svc.store.upsert_release(&cur).await?;
    }

    Ok(Json(PromoteResponse {
        previous_tag: current.map(|r| r.tag),
        promoted_tag: rolled_back.tag,
        target: req.target,
        dry_run: false,
        health_grace_passed: true,
    }))
}
```

Add to router. Commit `feat(deploy-server): rollback handler`.

---

### Task P4.6: pmctl promote + rollback subcommands

Update `bin/pmctl.rs`:

```rust
/// Promote a registered candidate to a target.
Promote {
    tag: String,
    #[arg(long)]
    target: String,
    #[arg(long)]
    dry_run: bool,
},
/// Rollback a target to its previous release (or explicit tag).
Rollback {
    #[arg(long)]
    target: String,
    #[arg(long)]
    to: Option<String>,
},
```

Match arms POST to `/api/promote` and `/api/rollback`. Commit `feat(pmctl): promote + rollback subcommands`.

---

### Task P4.7: GHA tag → register release

Create `.github/workflows/release.yml`:

```yaml
name: Release tag → register prod-candidate

on:
  push:
    tags:
      - 'v*'

permissions:
  contents: read
  packages: write
  id-token: write

jobs:
  trigger:
    needs: []   # depends on docker-build.yml + docker-frontend.yml completing for the same SHA;
                # we rely on tag-triggered runs of those workflows producing the images.
    runs-on: ubuntu-latest
    steps:
      - name: Wait for image workflows (poll)
        run: |
          # Wait up to 15 min for both backend and frontend workflows to finish for this tag.
          TAG="${GITHUB_REF#refs/tags/}"
          echo "Waiting for image builds for tag $TAG"
          for i in $(seq 1 90); do
            backend_done=$(gh run list --workflow=docker-build.yml --branch="$TAG" --limit 1 --json status -q '.[0].status' || echo "")
            frontend_done=$(gh run list --workflow=docker-frontend.yml --branch="$TAG" --limit 1 --json status -q '.[0].status' || echo "")
            if [ "$backend_done" = "completed" ] && [ "$frontend_done" = "completed" ]; then
              break
            fi
            sleep 10
          done
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}

      - name: Get OIDC token
        uses: actions/github-script@v7
        id: oidc
        with:
          script: |
            const token = await core.getIDToken('ppt-deploy');
            core.setOutput('token', token);

      - name: POST /api/release
        env:
          DEPLOY_URL: https://onyx.rlt.sk
          IMAGE_PREFIX: ghcr.io/martin-janci
        run: |
          TAG="${GITHUB_REF#refs/tags/}"
          IMAGES=$(jq -n \
            --arg api "$IMAGE_PREFIX/ppt-api-server:$TAG" \
            --arg reality "$IMAGE_PREFIX/ppt-reality-server:$TAG" \
            --arg pptweb "$IMAGE_PREFIX/ppt-web:$TAG" \
            --arg realityweb "$IMAGE_PREFIX/reality-web:$TAG" \
            '{"api-server":$api,"reality-server":$reality,"ppt-web":$pptweb,"reality-web":$realityweb}')
          curl -fsS -X POST "$DEPLOY_URL/api/release" \
            -H "Authorization: Bearer ${{ steps.oidc.outputs.token }}" \
            -H "Content-Type: application/json" \
            -d "{\"tag\":\"$TAG\",\"images\":$IMAGES}"
```

Commit `ci: tag → register prod-candidate via /api/release`.

---

### Task P4.8: Smoke tests for promote + rollback

Extend `tests/smoke.rs` with `#[ignore]`'d tests for `/api/promote` and `/api/rollback`.

Commit `test(deploy-server): smoke for promote + rollback`.

---

## Self-Review Coverage

| Spec deliverable (Phase 4) | Plan task |
|---|---|
| Tag → GHA build → register candidate | P4.7 |
| `pmctl promote <tag> --target=prod [--dry-run]` | P4.4, P4.6 |
| Blue-green swap on prod | P4.1 (generalized BlueGreenDeployer) |
| `pmctl rollback --target=prod [--to=<tag>]` | P4.5, P4.6 |
| Health grace 60 s + warning output | P4.2, P4.4 |
| Auto-rollback flag in targets.yaml (default off) | P4.4 (rollback_mode == "auto") |

Phase 4 deferrals (acceptable):
- Multi-region prod (Phase 5)
- Canary % traffic shifting (out of scope)
- Webhook for slack/email on promote (Phase 6 polish)
