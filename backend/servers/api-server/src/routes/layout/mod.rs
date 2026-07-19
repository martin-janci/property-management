pub mod admin;
pub mod types;

use crate::state::AppState;
use axum::routing::{get, put};
use axum::Router;

pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/screens", get(admin::list_screens))
        .route("/config", get(admin::get_config))
        .route("/draft", put(admin::put_draft))
        .route("/rails", put(admin::put_rails))
        .route("/manifests", get(admin::list_manifests).put(admin::put_manifest))
}
