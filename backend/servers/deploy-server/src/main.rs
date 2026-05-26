// backend/servers/deploy-server/src/main.rs
use anyhow::Context;
use deploy_server::api::promote::PromoteService;
use deploy_server::api::release::ReleaseService;
use deploy_server::api::router;
use deploy_server::auth::{ApiKeyValidator, OidcValidator};
use deploy_server::config::{load_yaml, AuthConfig, Config, TargetsConfig};
use deploy_server::infra::{
    CaddyClient, DockerClient, GhClient, GitFetcher, HealthProbe, PostgresOps, Store,
};
use listenfd::ListenFd;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .init();

    let etc =
        PathBuf::from(std::env::var("PPT_DEPLOY_ETC").unwrap_or_else(|_| "/etc/ppt-deploy".into()));
    let cfg: Config = load_yaml(&etc.join("config.yaml")).context("load config.yaml")?;
    let cfg_arc = Arc::new(cfg.clone());
    let targets: TargetsConfig =
        load_yaml(&etc.join("targets.yaml")).context("load targets.yaml")?;
    let auth: AuthConfig = load_yaml(&etc.join("auth.yaml")).context("load auth.yaml")?;

    let staging = targets
        .targets
        .get("staging")
        .context("targets.staging missing")?;

    let store = Arc::new(Store::open(&PathBuf::from(&cfg.state_dir).join("state.db")).await?);
    let worktree_locks = Arc::new(deploy_server::infra::WorktreeLockRegistry::new());
    let git = Arc::new(GitFetcher::new(
        &cfg.git_repo_url,
        &cfg.worktree_dir,
        &auth.gh_deploy_key_path,
    ));
    let docker = Arc::new(DockerClient::from_socket(&staging.docker_socket)?);
    let caddy = Arc::new(CaddyClient::new(&staging.caddy_url));
    let api_keys = Arc::new(ApiKeyValidator::new(auth.api_keys.clone()));
    let oidc = Arc::new(OidcValidator::new(auth.oidc.clone()));

    let webhook_cfg = deploy_server::api::webhook::WebhookConfig {
        secret: auth.webhook_secret.clone(),
    };

    let mut docker_pool: HashMap<String, Arc<DockerClient>> = HashMap::new();
    let mut caddy_pool: HashMap<String, Arc<CaddyClient>> = HashMap::new();
    for (name, t) in &targets.targets {
        docker_pool.insert(
            name.clone(),
            Arc::new(DockerClient::from_socket(&t.docker_socket)?),
        );
        caddy_pool.insert(name.clone(), Arc::new(CaddyClient::new(&t.caddy_url)));
    }

    // PO-002 one-shot reconciliation: sweep routes left by older
    // deploy-server versions that POST-appended without `@id`. Those
    // routes are invisible to `unregister_route`'s DELETE-by-id sweep,
    // so without this purge the first deploy on each Caddy would still
    // see duplicates. Best-effort — log and continue if any target is
    // unreachable so a single bad Caddy doesn't block startup.
    //
    // `caddy_pool` already contains the staging entry (built from the
    // same targets.targets map below), so iterating the pool is the
    // single canonical pass — no separate `caddy.purge()` call.
    let wt_host_prefix = "wt-";
    for (name, c) in &caddy_pool {
        if let Err(e) = c.purge_legacy_untagged_routes(wt_host_prefix).await {
            tracing::warn!(
                target = %name,
                error = %e,
                "caddy purge_legacy_untagged_routes failed at startup; continuing",
            );
        }
    }

    let docker_pool = Arc::new(docker_pool);
    let caddy_pool = Arc::new(caddy_pool);

    let release_svc = Arc::new(ReleaseService {
        store: store.clone(),
        docker_pool,
        caddy_pool,
        targets: Arc::new(targets.clone()),
        image_prefix: std::env::var("PPT_IMAGE_PREFIX")
            .unwrap_or_else(|_| "ghcr.io/martin-janci".into()),
        postgres_admin_url: cfg.postgres_admin_url.clone(),
    });

    let promote_svc = Arc::new(PromoteService {
        release_svc: release_svc.clone(),
        health: Arc::new(HealthProbe::new()),
        targets: Arc::new(targets.clone()),
    });

    if let Ok(log_path) = std::env::var("CADDY_ACCESS_LOG") {
        let store_clone = store.clone();
        tokio::spawn(async move {
            deploy_server::infra::traffic::tail_caddy_log(log_path, store_clone).await;
        });
    }

    let postgres = Arc::new(PostgresOps {
        admin_url: cfg.postgres_admin_url.clone(),
        template_db: cfg.postgres_template_db.clone(),
        user_db_prefix: cfg.postgres_user_db_prefix.clone(),
    });
    let gh = Arc::new(GhClient::new(
        auth.gh_api_token.clone(),
        cfg.gh_repo.clone(),
    ));

    // Bootstrap reuses release_svc.docker_pool so deploy + bootstrap never
    // disagree about which Docker socket drives which target — bootstrap
    // can't create a network on a different daemon than the one the
    // subsequent deploy will see. Bootstrap doesn't talk to Caddy itself
    // (it only attaches `ppt-caddy` to the new network); the Caddy admin
    // client lives elsewhere in the release pipeline.
    let bootstrap_svc = Arc::new(deploy_server::api::bootstrap::BootstrapService {
        targets: Arc::new(targets.clone()),
        postgres: postgres.clone(),
        release_svc: release_svc.clone(),
    });

    let app = router::build(
        store,
        git,
        docker,
        caddy,
        api_keys,
        oidc,
        std::env::var("PPT_FRONTEND_IMAGE").unwrap_or_else(|_| "ppt-frontend-dev:local".into()),
        // Worktree dev hosts (`wt-<name>.dev.ppt.rlt.sk`, `wt-<name>.dev.rlt.sk`)
        // mirror the staging apex with `staging.` swapped for `dev.`. Example:
        // staging.ppt_apex="staging.ppt.rlt.sk" → "dev.ppt.rlt.sk".
        // Future improvement: make these explicit fields on the staging
        // target (or top-level config) instead of derived from staging.*.
        format!(
            "dev.{}",
            staging
                .ppt_apex
                .strip_prefix("staging.")
                .unwrap_or(&staging.ppt_apex)
        ),
        format!(
            "dev.{}",
            staging
                .reality_apex
                .strip_prefix("staging.")
                .unwrap_or(&staging.reality_apex)
        ),
        // Shared-mode `/api/*` upstreams for worktrees (fix for #453).
        // Defaults match the Caddyfile's Docker DNS names; override per
        // host via env when the dev shared backend runs under different
        // container names (e.g. `dev-api-blue:8080` under blue_green).
        std::env::var("PPT_SHARED_API_UPSTREAM_PPT")
            .unwrap_or_else(|_| "api-server:8080".into()),
        std::env::var("PPT_SHARED_API_UPSTREAM_REALITY")
            .unwrap_or_else(|_| "reality-server:8081".into()),
        webhook_cfg,
        cfg_arc,
        release_svc,
        postgres,
        gh,
        cfg.backend_image_prefix.clone(),
        promote_svc,
        worktree_locks,
        bootstrap_svc,
    );

    let mut fd = ListenFd::from_env();
    let listener = if let Some(l) = fd.take_tcp_listener(0)? {
        l.set_nonblocking(true)?;
        TcpListener::from_std(l)?
    } else {
        TcpListener::bind(&cfg.bind).await?
    };
    tracing::info!(bind = %listener.local_addr()?, "ppt-deploy listening");
    axum::serve(listener, app).await?;
    Ok(())
}
