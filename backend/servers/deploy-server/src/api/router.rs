// backend/servers/deploy-server/src/api/router.rs
use crate::api::{health, webhook, worktree};
use crate::auth::{ApiKeyValidator, OidcValidator};
use crate::infra::{audit, AuthState, CaddyClient, DockerClient, GitFetcher, Store};
use axum::middleware::from_fn_with_state;
use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;

#[allow(clippy::too_many_arguments)]
pub fn build(
    store: Arc<Store>,
    git: Arc<GitFetcher>,
    docker: Arc<DockerClient>,
    caddy: Arc<CaddyClient>,
    api_keys: Arc<ApiKeyValidator>,
    oidc: Arc<OidcValidator>,
    frontend_image: String,
    domain_dev_ppt: String,
    domain_dev_reality: String,
    webhook_cfg: webhook::WebhookConfig,
) -> Router {
    let svc = Arc::new(worktree::WorktreeService {
        store: store.clone(),
        git,
        docker,
        caddy,
        frontend_image,
        domain_dev_ppt,
        domain_dev_reality,
    });

    let auth_state = AuthState {
        api_keys,
        oidc,
        store: store.clone(),
    };

    Router::new()
        .route("/api/worktree", post(worktree::open_handler))
        .route("/api/worktrees", get(worktree::list_handler))
        .route("/api/worktree/:name", get(worktree::get_handler))
        .route("/api/worktree/:name/close", post(worktree::close_handler))
        .with_state(svc.clone())
        .layer(from_fn_with_state(auth_state, audit::auth_and_audit))
        .route("/health", get(health::handler))
        .merge(
            Router::new()
                .route("/api/webhook/github", post(webhook::handler))
                .with_state((svc, webhook_cfg)),
        )
}
