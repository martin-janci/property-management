// backend/servers/deploy-server/src/api/worktree.rs
use crate::domain::{BackendMode, Worktree, WorktreeState, WorktreeUrls};
use crate::infra::git::sanitize;
use crate::infra::{
    CaddyClient, CallerIdentity, DockerClient, FrontendDevSpec, GhClient, GitFetcher, PostgresOps,
    Store, WorktreeLockRegistry,
};
use crate::{DeployError, Result};
use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Max time we'll poll Docker for a freshly-started container's bridge IP
/// before bailing. Docker normally assigns within a few hundred ms; 10 s gives
/// plenty of margin for a stressed daemon without making a hung start hang the
/// caller for a wt-open lifetime.
const BRIDGE_IP_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub struct WorktreeService {
    pub store: Arc<Store>,
    pub git: Arc<GitFetcher>,
    pub docker: Arc<DockerClient>,
    pub caddy: Arc<CaddyClient>,
    pub frontend_image: String,
    pub domain_dev_ppt: String,     // "dev.ppt.rlt.sk"
    pub domain_dev_reality: String, // "dev.rlt.sk"
    /// Upstream `host:port` that shared-mode worktrees route `/api/*` to
    /// on the ppt-web host (`wt-<name>.<domain_dev_ppt>/api/*`). Defaults
    /// to the Caddyfile's `api-server:8080` Docker DNS name; can be
    /// overridden per-host via `PPT_SHARED_API_UPSTREAM_PPT` (e.g.
    /// `dev-api-blue:8080` when the dev shared backend runs under the
    /// blue_green naming scheme). See #453 — without this route every
    /// ppt-web SPA API call to a shared worktree 502'd because ppt-web's
    /// Vite bundle has the API base URL baked in at build time and so
    /// makes same-origin `/api/*` calls.
    pub shared_api_upstream_ppt: String,
    /// Upstream `host:port` that shared-mode worktrees route `/api/*` to
    /// on the reality-web host (`wt-<name>.<domain_dev_reality>/api/*`).
    /// Defaults to `reality-server:8081` (matching infra/caddy/Caddyfile).
    /// Override via `PPT_SHARED_API_UPSTREAM_REALITY`.
    pub shared_api_upstream_reality: String,
    pub postgres: Arc<PostgresOps>,
    pub gh: Arc<GhClient>,
    pub backend_image_prefix: String,
    pub worktree_locks: Arc<WorktreeLockRegistry>,
    /// Filesystem path where dedicated-backend pg_dumps are written on close.
    pub snapshot_dir: String,
}

#[derive(Debug, Deserialize)]
pub struct OpenRequest {
    pub branch: String,
    pub alias: Option<String>,
    #[serde(default = "default_backend")]
    pub backend: BackendMode,
    pub ttl_seconds: Option<i64>,
}

fn default_backend() -> BackendMode {
    BackendMode::Shared
}

#[derive(Debug, Serialize)]
pub struct OpenResponse {
    pub worktree: Worktree,
    pub backend_status: String, // "ready" | "building"
}

pub async fn open_handler(
    State(svc): State<Arc<WorktreeService>>,
    axum::Extension(caller): axum::Extension<CallerIdentity>,
    Json(req): Json<OpenRequest>,
) -> Result<Json<OpenResponse>> {
    caller.require_scope("worktree:open")?;
    let name = req.alias.clone().unwrap_or_else(|| sanitize(&req.branch));
    if name.is_empty() {
        return Err(DeployError::BadRequest(
            "alias resolves to empty name".into(),
        ));
    }

    // Strict input validation — defense against SQL injection (db_name interpolation)
    // and command/option injection (git fetch <branch>, image tag interpolation).
    crate::infra::git::validate_branch_strict(&req.branch)?;
    crate::infra::git::validate_alias_strict(&name)?;

    // Serialize concurrent open/close/GC for the same worktree name.
    // Held until function returns; prevents duplicate containers, port collisions,
    // and double pg_restore from racing /api/worktree calls (#11).
    let _lock = svc.worktree_locks.acquire(&name).await;

    // Resume from dump if a closed worktree with this name exists within TTL window.
    //
    // We track the dump path that was used to resume so we can carry it forward
    // onto the new Worktree row. Without this, a future GC `Cleanup` (after the
    // next close) wouldn't know which file on disk to delete and old dumps would
    // accumulate forever in `snapshot_dir`.
    let existing = svc.store.get_worktree(&name).await?;
    let mut resumed_dump_path: Option<String> = None;
    let resume_db: Option<String> = if let Some(ref ex) = existing {
        if matches!(ex.state, WorktreeState::Closed)
            && matches!(req.backend, BackendMode::Dedicated)
        {
            if let (Some(db), Some(dump_path)) = (
                ex.db_name.clone().or_else(|| {
                    // db was dropped during gc; reconstruct expected name
                    Some(format!("{}{}", svc.postgres.user_db_prefix, name))
                }),
                ex.dump_path.clone(),
            ) {
                tracing::info!(name=%name, dump=%dump_path, "resuming worktree from pg dump");
                svc.postgres
                    .restore(&db, std::path::Path::new(&dump_path))
                    .await?;
                resumed_dump_path = Some(dump_path);
                Some(db)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // 1. Fetch source.
    let source_path = svc.git.fetch_branch(&req.branch).await?;

    // 2. Allocate frontend ports.
    let port_ppt = pick_port(&format!("{name}-ppt"));
    let port_reality = pick_port(&format!("{name}-reality"));

    // 3. Spawn frontend dev containers.
    let pnpm_volume = format!("ppt-deploy-pnpm-{name}");
    let ppt_container = format!("wt-{name}-ppt");
    let reality_container = format!("wt-{name}-reality");

    // Construct reality-web env vars. The dev container's `pnpm dev` reads
    // `process.env.NEXT_PUBLIC_API_URL` for SSR and serves it via the /env.js
    // route to the client. Without this, the bundle's localhost fallback is
    // used and every API call ERR_CONNECTION_REFUSEDs in the user's browser.
    //
    // - Shared mode  → api.<reality_apex> (the prod reality-server). Derived
    //   by stripping the conventional "dev." prefix from `domain_dev_reality`.
    // - Dedicated    → api.wt-<name>.<domain_dev_reality> — the dedicated
    //   reality-API container's public Caddy route registered below.
    let host_ppt_for_env = format!("wt-{name}.{}", svc.domain_dev_ppt);
    let host_reality_for_env = format!("wt-{name}.{}", svc.domain_dev_reality);
    let reality_api_url = match req.backend {
        BackendMode::Shared => format!(
            "https://api.{}",
            svc.domain_dev_reality
                .strip_prefix("dev.")
                .unwrap_or(&svc.domain_dev_reality)
        ),
        BackendMode::Dedicated => {
            format!("https://api.wt-{name}.{}", svc.domain_dev_reality)
        }
    };
    let reality_extra_env = vec![
        format!("NEXT_PUBLIC_API_URL={reality_api_url}"),
        format!("NEXT_PUBLIC_SITE_URL=https://{host_reality_for_env}"),
    ];
    // ppt-web is Vite/SPA — env is build-time-baked into the bundle, so
    // runtime container env has no effect on client code. Leaving extra_env
    // empty here matches the existing blue-green prod path (which also sets
    // no extra env for the ppt entry; a comment in blue_green.rs explains).
    let _ = host_ppt_for_env; // currently unused; reserved for future ppt-web wiring

    svc.docker
        .run_frontend_dev(&FrontendDevSpec {
            container_name: ppt_container.clone(),
            app: "ppt-web".into(),
            source_path: source_path.to_string_lossy().to_string(),
            host_port: port_ppt,
            pnpm_volume: pnpm_volume.clone(),
            image: svc.frontend_image.clone(),
            extra_env: vec![],
        })
        .await?;
    svc.docker
        .run_frontend_dev(&FrontendDevSpec {
            container_name: reality_container.clone(),
            app: "reality-web".into(),
            source_path: source_path.to_string_lossy().to_string(),
            host_port: port_reality,
            pnpm_volume,
            image: svc.frontend_image.clone(),
            extra_env: reality_extra_env,
        })
        .await?;

    // 4. Register frontend Caddy routes.
    //
    // Upstream MUST be the container's docker-bridge IP (not the host's
    // `127.0.0.1:<host_port>`). Caddy itself runs in a container; its
    // loopback is its own, not the host's. Pointing at the bridge IP works
    // because Caddy and the worktree dev containers share the default bridge
    // network. The container's *internal* port is also fixed by the
    // `ppt-frontend-dev:local` Dockerfile (5173 for Vite/ppt-web, 3000 for
    // Next/reality-web).
    //
    // `bridge_ip_with_retry` polls because Docker doesn't always assign a
    // bridge IP synchronously with `start_container`; without the poll loop a
    // fast caller can race the daemon and see "no bridge network IP yet".
    //
    // Caveat: default-bridge IPs aren't stable across container restarts.
    // Tracked as a follow-up to migrate to a user-defined network with
    // container-name DNS, which would survive restarts.
    let host_ppt = format!("wt-{name}.{}", svc.domain_dev_ppt);
    let host_reality = format!("wt-{name}.{}", svc.domain_dev_reality);
    let route_result: Result<()> = async {
        let ppt_bridge_ip = svc
            .docker
            .bridge_ip_with_retry(&ppt_container, BRIDGE_IP_TIMEOUT)
            .await?;
        let reality_bridge_ip = svc
            .docker
            .bridge_ip_with_retry(&reality_container, BRIDGE_IP_TIMEOUT)
            .await?;
        // Shared mode: register the `/api/*` reverse-proxy entries FIRST,
        // before the host-only frontend routes (fix for #453). Caddy
        // dispatches routes in array order and does not auto-sort by
        // matcher specificity, so a more-specific host+path matcher must
        // be earlier in the array than a host-only matcher on the same
        // host. Dedicated mode uses a separate `api.wt-<name>.<domain>`
        // host (registered later in section 5) and doesn't need this.
        if matches!(req.backend, BackendMode::Shared) {
            svc.caddy
                .register_path_route(&host_ppt, "/api/*", &svc.shared_api_upstream_ppt, "api")
                .await?;
            svc.caddy
                .register_path_route(
                    &host_reality,
                    "/api/*",
                    &svc.shared_api_upstream_reality,
                    "api",
                )
                .await?;
        }
        svc.caddy
            .register_route(&host_ppt, &format!("{ppt_bridge_ip}:5173"))
            .await?;
        svc.caddy
            .register_route(&host_reality, &format!("{reality_bridge_ip}:3000"))
            .await?;
        Ok(())
    }
    .await;
    if let Err(e) = route_result {
        // We've already started both frontend containers but haven't persisted
        // the worktree row yet — roll the side effects back so we don't leak
        // orphans the next `pmctl close` won't know about.
        let started = vec![ppt_container.clone(), reality_container.clone()];
        tracing::warn!(
            error = %e,
            containers = ?started,
            "frontend bridge-IP/route registration failed; cleaning up containers"
        );
        svc.docker.cleanup_containers(&started).await;
        // Best-effort cleanup: an `unregister_route` failure here means the
        // route is still in Caddy pointing at the dead container we just
        // killed. Log so on-call sees the orphan (PO2-001).
        if let Err(unreg_err) = svc.caddy.unregister_route(&host_ppt).await {
            tracing::warn!(host = %host_ppt, error = %unreg_err, "caddy unregister failed during open-rollback");
        }
        if let Err(unreg_err) = svc.caddy.unregister_route(&host_reality).await {
            tracing::warn!(host = %host_reality, error = %unreg_err, "caddy unregister failed during open-rollback");
        }
        // Shared-mode `/api/*` path routes (fix for #453). No-op (404) for
        // dedicated mode or when the partial open never reached the
        // path-route registration step — best-effort.
        if matches!(req.backend, BackendMode::Shared) {
            if let Err(unreg_err) = svc.caddy.unregister_path_route(&host_ppt, "api").await {
                tracing::warn!(host = %host_ppt, error = %unreg_err, "caddy unregister path-route failed during open-rollback");
            }
            if let Err(unreg_err) = svc.caddy.unregister_path_route(&host_reality, "api").await {
                tracing::warn!(host = %host_reality, error = %unreg_err, "caddy unregister path-route failed during open-rollback");
            }
        }
        return Err(e);
    }

    let mut containers = vec![ppt_container, reality_container];
    let mut api_url: Option<String> = None;
    let mut db_name: Option<String> = None;
    let mut backend_status = "ready".to_string();

    // 5. Dedicated backend branch.
    if matches!(req.backend, BackendMode::Dedicated) {
        // Resolve which Postgres DB to use. Three paths:
        //   (a) Resuming a Closed worktree: `resume_db` is already restored above.
        //   (b) Idempotent retry — a previous `open` for this name already
        //       created/restored the DB but the GHA build wasn't ready, so we
        //       returned `backend_status="building"` and persisted the row with
        //       `db_name = Some(...)`. Calling `open` again must NOT
        //       `CREATE DATABASE` again (it would fail with "already exists").
        //       Instead reuse the stored db_name.
        //   (c) First open: create from template.
        let db = if let Some(existing_db) = resume_db.clone() {
            existing_db
        } else if let Some(ref ex) = existing {
            // Idempotent retry. The previous open landed but the dedicated
            // build wasn't ready in the polling window — `db_name` is already
            // set on the stored Worktree row. Reuse it.
            if let Some(ref existing_db) = ex.db_name {
                tracing::info!(
                    name = %name,
                    db = %existing_db,
                    state = ?ex.state,
                    "reusing existing dedicated DB on idempotent open (previous build was still in progress)"
                );
                existing_db.clone()
            } else {
                // Stored row exists but no db_name (e.g. previous attempt was Shared mode
                // or crashed before the dedicated branch ran). Create fresh.
                let db = format!("{}{}", svc.postgres.user_db_prefix, name);
                tracing::info!(name = %name, db = %db, "creating dedicated DB");
                svc.postgres.create_from_template(&db).await?;
                tracing::info!(name = %name, db = %db, "DB created");
                db
            }
        } else {
            let db = format!("{}{}", svc.postgres.user_db_prefix, name);
            tracing::info!(name = %name, db = %db, "creating dedicated DB (fresh path)");
            svc.postgres.create_from_template(&db).await?;
            tracing::info!(name = %name, db = %db, "DB created (fresh path)");
            db
        };
        db_name = Some(db.clone());

        // Dispatch the docker-build.yml workflow (skip if resuming — images
        // cached from previous open). The handler is intentionally non-blocking:
        // the fast-path below short-circuits to `completed=true` if a green run
        // already exists for the branch; otherwise we dispatch and immediately
        // return `backend_status="building"` so the operator (or pmctl) can
        // re-invoke `open` once GHA finishes pushing images. We never poll the
        // workflow ourselves — that keeps the deploy-server's request path bounded
        // and the runner-minute usage symmetrical with normal commit-push CI.
        let completed = if resume_db.is_some() {
            true
        } else {
            // Fast-path: check if a successful run already exists for this branch.
            // If so, skip dispatch + poll entirely — the images are already pushed
            // to ghcr.io. This makes the open call return in seconds instead of 10
            // minutes when the build was already done by an earlier dispatch (or
            // a normal commit-push CI flow).
            let existing_success = match svc.gh.latest_run("docker-build.yml", &req.branch).await? {
                Some(r)
                    if r.status == "completed" && r.conclusion.as_deref() == Some("success") =>
                {
                    tracing::info!(
                        run_id = r.id,
                        "fast-path: existing success run detected, skipping dispatch+poll"
                    );
                    true
                }
                Some(r) => {
                    tracing::info!(run_id = r.id, status = %r.status, conclusion = ?r.conclusion, "fast-path: latest run not success, dispatching new build");
                    false
                }
                None => {
                    tracing::info!("fast-path: no run found, dispatching first build");
                    false
                }
            };
            if existing_success {
                true
            } else {
                tracing::info!(branch = %req.branch, "dispatching docker-build.yml");
                svc.gh
                    .dispatch_workflow("docker-build.yml", &req.branch)
                    .await?;
                tracing::info!(branch = %req.branch, "dispatched, returning building status (caller should retry after GHA completes)");
                false
            }
        };
        if !completed {
            // Build still running. The Worktree row is persisted below with
            // db_name set; the caller should re-invoke `open` once the GHA
            // workflow finishes — that retry hits the idempotent branch above
            // and reuses the existing DB instead of recreating it.
            // (No background task and no polling: keeping the deploy-server
            // stateless on this path is intentional. A future Phase 6+
            // improvement could spawn a best-effort follow-up that finishes
            // the deploy when the build signals completion.)
            backend_status = "building".into();
            tracing::info!(
                name = %name,
                "dedicated backend build not yet ready; returning building. Caller should re-open after build completes."
            );
        } else {
            // Run backend containers.
            // Image tag matches docker-build.yml's `type=ref,event=branch` which
            // replaces `/` with `-` but preserves case. So `feature/UC-14` → `feature-UC-14`.
            let branch_tag = req.branch.replace(['/', '_'], "-");
            let api_image = format!("{}/ppt-api-server:{}", svc.backend_image_prefix, branch_tag);
            let reality_image = format!(
                "{}/ppt-reality-server:{}",
                svc.backend_image_prefix, branch_tag
            );

            let api_port = pick_port(&format!("{name}-api"));
            let reality_port = pick_port(&format!("{name}-reality-api"));
            let api_c = format!("wt-{name}-api");
            let reality_c = format!("wt-{name}-reality-api");

            // Use PostgresOps::url_for so the per-DB connection string preserves
            // the admin URL's user/host/port/query (e.g. `?sslmode=require`) and
            // doesn't break when the admin path is anything other than `/postgres`.
            let db_url = svc.postgres.url_for(&db)?;

            let jwt_secret = std::env::var("PPT_JWT_SECRET").map_err(|_| {
                DeployError::Config(
                    "PPT_JWT_SECRET env var is not set; refusing to start a dedicated backend with a default secret. \
                     Set it in /etc/ppt-deploy/secrets.env (referenced by the systemd unit's EnvironmentFile)".into()
                )
            })?;
            if jwt_secret.len() < 32 {
                return Err(DeployError::Config(
                    "PPT_JWT_SECRET must be at least 32 characters".into(),
                ));
            }

            // Run both backend containers and register the api Caddy route in
            // a single try block; on any failure here, force-remove all
            // containers (frontend + whatever backend we managed to start) and
            // unregister the api route. Keeps the worktree row from being
            // persisted with orphan side effects on disk.
            let host_api = format!("api.wt-{name}.{}", svc.domain_dev_ppt);
            // Reality-API gets its own subdomain on the reality apex so the
            // frontend (running on `wt-{name}.{domain_dev_reality}`) can hit
            // it without a CORS preflight by going through a same-origin
            // Next.js rewrite. Without this route, the dedicated reality
            // container is reachable only on the bridge network — fine for
            // SSR, broken for any client-side fetch from the browser.
            let host_reality_api = format!("api.wt-{name}.{}", svc.domain_dev_reality);
            let backend_result: Result<()> = async {
                tracing::info!(container = %api_c, image = %api_image, "starting dedicated api container");
                svc.docker.run_backend_dedicated(&crate::infra::BackendDedicatedSpec {
                        container_name: api_c.clone(),
                        image: api_image,
                        host_port: api_port,
                        container_port: 8080,
                        db_url: db_url.clone(),
                        jwt_secret: jwt_secret.clone(),
                        extra_env: vec![],
                    })
                    .await?;
                tracing::info!(container = %reality_c, image = %reality_image, "starting dedicated reality container");
                let reality_cors_origins = format!(
                    "https://{host},https://{host_ppt},http://localhost:3000,http://localhost:3001",
                    host = host_reality_for_env,
                    host_ppt = host_ppt_for_env,
                );
                svc.docker.run_backend_dedicated(&crate::infra::BackendDedicatedSpec {
                        container_name: reality_c.clone(),
                        image: reality_image,
                        host_port: reality_port,
                        container_port: 8081,
                        db_url,
                        jwt_secret,
                        extra_env: vec![
                            format!("CORS_ALLOWED_ORIGINS={}", reality_cors_origins),
                            // PM_API_URL points reality-server at the dedicated
                            // api-server for SSO. Mirrors blue_green prod path.
                            format!("PM_API_URL=https://{}", host_api),
                            format!("SSO_CALLBACK_URL=https://{}/api/v1/sso/callback", host_reality_for_env),
                        ],
                    })
                    .await?;

                // Caddy routes for backend — same bridge-IP rationale as the
                // frontend routes above. api container's internal port is 8080;
                // reality-api's internal port is 8081.
                let api_bridge_ip = svc
                    .docker
                    .bridge_ip_with_retry(&api_c, BRIDGE_IP_TIMEOUT)
                    .await?;
                tracing::info!(host = %host_api, ip = %api_bridge_ip, "registering api Caddy route");
                svc.caddy.register_route(&host_api, &format!("{api_bridge_ip}:8080")).await?;
                tracing::info!(host = %host_api, "api Caddy route registered");
                let reality_bridge_ip = svc
                    .docker
                    .bridge_ip_with_retry(&reality_c, BRIDGE_IP_TIMEOUT)
                    .await?;
                tracing::info!(host = %host_reality_api, ip = %reality_bridge_ip, "registering reality_api Caddy route");
                svc.caddy.register_route(&host_reality_api, &format!("{reality_bridge_ip}:8081")).await?;
                tracing::info!(host = %host_reality_api, "reality_api Caddy route registered");
                Ok(())
            }
            .await;
            if let Err(e) = backend_result {
                let mut to_clean = containers.clone();
                to_clean.push(api_c.clone());
                to_clean.push(reality_c.clone());
                tracing::warn!(
                    error = %e,
                    containers = ?to_clean,
                    "dedicated backend bring-up failed; cleaning up all worktree containers"
                );
                svc.docker.cleanup_containers(&to_clean).await;
                // Best-effort cleanup: log Caddy unregister failures so a
                // dedicated-backend bring-up rollback doesn't silently
                // orphan routes (PO2-001).
                for h in [&host_ppt, &host_reality, &host_api, &host_reality_api] {
                    if let Err(unreg_err) = svc.caddy.unregister_route(h).await {
                        tracing::warn!(host = %h, error = %unreg_err, "caddy unregister failed during dedicated-backend rollback");
                    }
                }
                return Err(e);
            }
            api_url = Some(format!("https://{host_api}"));

            containers.push(api_c);
            containers.push(reality_c);
        }
    }

    // 6. Persist state.
    let now = chrono::Utc::now();
    let wt = Worktree {
        name: name.clone(),
        branch: req.branch.clone(),
        backend_mode: req.backend.clone(),
        state: WorktreeState::Running,
        urls: WorktreeUrls {
            ppt: Some(format!("https://{host_ppt}")),
            reality: Some(format!("https://{host_reality}")),
            api: api_url,
        },
        containers,
        db_name,
        // Keep the previous dump on the row when resuming from one. The dump
        // file is still on disk (restore reads from it) and we want the next
        // GC `Cleanup` after the next close to find and remove it. The next
        // close will overwrite this with a fresh dump anyway.
        dump_path: resumed_dump_path,
        ttl_seconds: req.ttl_seconds.unwrap_or(172_800),
        last_traffic_at: Some(now),
        closed_at: None,
        created_at: now,
        created_by: format!("{}:{}", caller.kind, caller.id),
    };
    svc.store.upsert_worktree(&wt).await?;

    Ok(Json(OpenResponse {
        worktree: wt,
        backend_status,
    }))
}

pub async fn get_handler(
    State(svc): State<Arc<WorktreeService>>,
    axum::Extension(caller): axum::Extension<CallerIdentity>,
    Path(name): Path<String>,
) -> Result<Json<Worktree>> {
    caller.require_scope("worktree:read")?;
    // Validate the path param with the same strict rules `open_handler` applies
    // to a freshly-created alias (#769 finding 6). The name flows into Caddy host
    // strings and Postgres DB-name reconstruction downstream; reject anything that
    // isn't `[A-Za-z0-9_-]` (no path separators, no leading dash) at the boundary.
    crate::infra::git::validate_alias_strict(&name)?;
    let wt = svc
        .store
        .get_worktree(&name)
        .await?
        .ok_or_else(|| DeployError::NotFound(format!("worktree {name}")))?;
    Ok(Json(wt))
}

pub async fn list_handler(
    State(svc): State<Arc<WorktreeService>>,
    axum::Extension(caller): axum::Extension<CallerIdentity>,
) -> Result<Json<Vec<Worktree>>> {
    caller.require_scope("worktree:read")?;
    Ok(Json(svc.store.list_worktrees().await?))
}

pub async fn close_handler(
    State(svc): State<Arc<WorktreeService>>,
    axum::Extension(caller): axum::Extension<CallerIdentity>,
    Path(name): Path<String>,
) -> Result<Json<Worktree>> {
    caller.require_scope("worktree:close")?;
    // Validate the path param before it reaches the lock registry, the store, or
    // the Caddy/Postgres cleanup paths (#769 finding 6). Same strict rules as
    // `open_handler`.
    crate::infra::git::validate_alias_strict(&name)?;
    // Serialize against concurrent open/close/GC for the same worktree (#11, #12).
    let _lock = svc.worktree_locks.acquire(&name).await;

    let mut wt = svc
        .store
        .get_worktree(&name)
        .await?
        .ok_or_else(|| DeployError::NotFound(format!("worktree {name}")))?;

    // Refuse to re-close already-closed worktrees (no-op idempotency) (#8).
    if matches!(wt.state, WorktreeState::Closed) {
        return Ok(Json(wt));
    }

    // Atomic: mark in-progress before any side effects, so a crash leaves a recoverable
    // trace that GC can pick up after the stuck-Closing threshold (#8).
    wt.state = WorktreeState::Closing;
    svc.store.upsert_worktree(&wt).await?;

    // Stop+remove containers, best-effort cleanup with debug-level logging on failure.
    svc.docker.cleanup_containers(&wt.containers).await;

    // Unregister Caddy routes for all worktree URLs.
    for url_opt in [
        wt.urls.ppt.as_deref(),
        wt.urls.reality.as_deref(),
        wt.urls.api.as_deref(),
    ] {
        if let Some(host) = url_opt.and_then(|u| u.strip_prefix("https://")) {
            // Best-effort: close runs on a healthy worktree, so 404 (most
            // common case) is already Ok. The new Err paths from caddy.rs
            // (16-iter exhaustion, 15s deadline) are worth surfacing so
            // operators notice a wedged Caddy admin (PO2-001).
            if let Err(unreg_err) = svc.caddy.unregister_route(host).await {
                tracing::warn!(host = %host, error = %unreg_err, "caddy unregister failed during worktree close");
            }
        }
    }
    // Dedicated mode also has a reality-API route at
    // `api.wt-{name}.{domain_dev_reality}` (registered alongside api-server
    // in open_handler). Not stored in `wt.urls` to avoid a serde-compat
    // change, so derive it here from the same constants. No-op in shared
    // mode — the route doesn't exist, unregister is a 404.
    if matches!(wt.backend_mode, BackendMode::Dedicated) {
        let host_reality_api = format!("api.wt-{}.{}", wt.name, svc.domain_dev_reality);
        if let Err(unreg_err) = svc.caddy.unregister_route(&host_reality_api).await {
            tracing::warn!(host = %host_reality_api, error = %unreg_err, "caddy unregister (reality-api) failed during worktree close");
        }
    }

    // Shared mode has path-matched `/api/*` routes on the two frontend
    // hosts (fix for #453). 404 (route absent) is a normal no-op for
    // worktrees opened before this fix shipped — best-effort.
    if matches!(wt.backend_mode, BackendMode::Shared) {
        let host_ppt = format!("wt-{}.{}", wt.name, svc.domain_dev_ppt);
        let host_reality = format!("wt-{}.{}", wt.name, svc.domain_dev_reality);
        if let Err(unreg_err) = svc.caddy.unregister_path_route(&host_ppt, "api").await {
            tracing::warn!(host = %host_ppt, error = %unreg_err, "caddy unregister path-route (ppt /api/*) failed during worktree close");
        }
        if let Err(unreg_err) = svc.caddy.unregister_path_route(&host_reality, "api").await {
            tracing::warn!(host = %host_reality, error = %unreg_err, "caddy unregister path-route (reality /api/*) failed during worktree close");
        }
    }

    // Dedicated backend: dump → drop the per-worktree Postgres DB so resume-from-dump
    // works on the next open. Without this, GC's Paused → Closed transition handles it
    // eventually (P3.6), but explicit close should not skip it — otherwise open_handler
    // will hit `database already exists` when create_from_template runs.
    if matches!(wt.backend_mode, BackendMode::Dedicated) {
        if let Some(ref db) = wt.db_name.clone() {
            let dump_path_str = format!(
                "{}/{}-{}.dump",
                svc.snapshot_dir,
                wt.name,
                chrono::Utc::now().timestamp()
            );
            let dump_path = std::path::Path::new(&dump_path_str);
            match svc.postgres.dump(db, dump_path).await {
                Ok(()) => {
                    wt.dump_path = Some(dump_path_str);
                    if let Err(e) = svc.postgres.drop_db(db).await {
                        tracing::warn!(error = %e, db = %db, "drop_db failed during close (kept dump)");
                    } else {
                        wt.db_name = None;
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, db = %db, "pg_dump failed during close — leaving DB live");
                }
            }
        }
    }

    wt.state = WorktreeState::Closed;
    wt.closed_at = Some(chrono::Utc::now());
    svc.store.upsert_worktree(&wt).await?;
    Ok(Json(wt))
}

fn pick_port(_seed: &str) -> u16 {
    // Probe for an OS-assigned free port (#11). Closing the listener releases the port
    // back to the kernel; there's a tiny race window before docker binds it, but
    // Docker handles "address already in use" with a clear error.
    use std::net::TcpListener;
    if let Ok(listener) = TcpListener::bind("127.0.0.1:0") {
        if let Ok(addr) = listener.local_addr() {
            return addr.port();
        }
    }
    // Fallback: deterministic in 51000-51999 — best-effort.
    51000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_port_returns_nonzero_port() {
        // Probe-based: returns an OS-assigned port or the 51000 fallback.
        let p = pick_port("foo");
        assert!(p > 0, "pick_port must return a non-zero port, got {p}");
    }

    #[test]
    fn pick_port_returns_valid_ports_across_calls() {
        // Probe-based: both calls return either OS-assigned ports or the 51000 fallback.
        let a = pick_port("foo");
        let b = pick_port("bar");
        assert!(a > 0 && b > 0);
    }
}
