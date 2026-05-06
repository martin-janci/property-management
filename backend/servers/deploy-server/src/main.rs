// backend/servers/deploy-server/src/main.rs
use anyhow::Context;
use deploy_server::api::release::ReleaseService;
use deploy_server::api::router;
use deploy_server::auth::{ApiKeyValidator, OidcValidator};
use deploy_server::config::{load_yaml, AuthConfig, Config, TargetsConfig};
use deploy_server::infra::{
    CaddyClient, DockerClient, GhClient, GitFetcher, PostgresOps, StagingDeployer, Store,
};
use listenfd::ListenFd;
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

    let deployer = Arc::new(StagingDeployer {
        docker: docker.clone(),
        caddy: caddy.clone(),
    });
    let release_svc = Arc::new(ReleaseService {
        store: store.clone(),
        deployer,
        targets: Arc::new(targets.clone()),
        image_prefix: std::env::var("PPT_IMAGE_PREFIX")
            .unwrap_or_else(|_| "ghcr.io/martin-janci".into()),
    });

    let postgres = Arc::new(PostgresOps {
        admin_url: cfg.postgres_admin_url.clone(),
        template_db: cfg.postgres_template_db.clone(),
        user_db_prefix: cfg.postgres_user_db_prefix.clone(),
    });
    let gh = Arc::new(GhClient::new(
        auth.gh_api_token.clone(),
        cfg.gh_repo.clone(),
    ));

    let app = router::build(
        store,
        git,
        docker,
        caddy,
        api_keys,
        oidc,
        std::env::var("PPT_FRONTEND_IMAGE").unwrap_or_else(|_| "ppt-frontend-dev:local".into()),
        format!(
            "dev.ppt.{}",
            staging.domain_suffix.trim_start_matches("staging.")
        ),
        format!(
            "dev.{}",
            staging.domain_suffix.trim_start_matches("staging.")
        ),
        webhook_cfg,
        cfg_arc,
        release_svc,
        postgres,
        gh,
        cfg.backend_image_prefix.clone(),
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
