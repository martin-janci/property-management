pub mod admin;
pub mod resolved;
pub mod tenant;
pub mod types;

use crate::state::AppState;
use axum::routing::{get, post, put};
use axum::Router;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/tenant-override",
            get(tenant::get_tenant_override).put(tenant::put_tenant_override),
        )
        .route("/resolved/{*screen}", get(resolved::get_resolved))
}

pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/screens", get(admin::list_screens))
        .route("/config", get(admin::get_config))
        .route("/draft", put(admin::put_draft))
        .route("/rails", put(admin::put_rails))
        .route(
            "/manifests",
            get(admin::list_manifests).put(admin::put_manifest),
        )
        .route("/publish", post(admin::publish))
        .route("/rollback", post(admin::rollback))
        .route("/kill", post(admin::kill))
        .route("/unkill", post(admin::unkill))
        .route("/preview-resolve", post(admin::preview_resolve))
}
