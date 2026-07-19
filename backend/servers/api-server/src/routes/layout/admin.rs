use crate::routes::platform_admin::extract_super_admin_token;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use db::repositories::LayoutRepository;

use super::types::{PutDraftRequest, PutManifestRequest, PutRailsRequest, ScreenQuery,
                   ValidationErrorsResponse};

fn bad_request(errors: Vec<String>) -> (StatusCode, Json<ValidationErrorsResponse>) {
    (StatusCode::UNPROCESSABLE_ENTITY, Json(ValidationErrorsResponse { errors }))
}

#[utoipa::path(get, path = "/api/v1/platform-admin/layout/screens", tag = "Layout Admin",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "All layout configs")))]
pub async fn list_screens(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ValidationErrorsResponse>)> {
    let _admin = extract_super_admin_token(&headers, &state)
        .map_err(|_| (StatusCode::FORBIDDEN, Json(ValidationErrorsResponse { errors: vec!["forbidden".into()] })))?;
    let rows = LayoutRepository::new()
        .list_configs(&state.db)
        .await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?;
    Ok(Json(serde_json::to_value(rows).unwrap_or_default()))
}

#[utoipa::path(get, path = "/api/v1/platform-admin/layout/config", tag = "Layout Admin",
    security(("bearer_auth" = [])), params(("screen" = String, Query, description = "Screen id")),
    responses((status = 200, description = "Config with versions and kills"), (status = 404, description = "Unknown screen")))]
pub async fn get_config(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(q): Query<ScreenQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ValidationErrorsResponse>)> {
    let _admin = extract_super_admin_token(&headers, &state)
        .map_err(|_| (StatusCode::FORBIDDEN, Json(ValidationErrorsResponse { errors: vec!["forbidden".into()] })))?;
    let repo = LayoutRepository::new();
    let cfg = repo
        .get_config(&state.db, &q.screen)
        .await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?
        .ok_or((StatusCode::NOT_FOUND, Json(ValidationErrorsResponse { errors: vec!["unknown screen".into()] })))?;
    let versions = repo.list_versions(&state.db, &q.screen).await.unwrap_or_default();
    let kills = repo.list_kills(&state.db, &q.screen).await.unwrap_or_default();
    Ok(Json(serde_json::json!({ "config": cfg, "versions": versions, "kills": kills })))
}

#[utoipa::path(put, path = "/api/v1/platform-admin/layout/draft", tag = "Layout Admin",
    security(("bearer_auth" = [])), request_body = PutDraftRequest,
    responses((status = 200, description = "Draft saved"), (status = 422, description = "Config does not parse as a ScreenConfig")))]
pub async fn put_draft(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<PutDraftRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ValidationErrorsResponse>)> {
    let (admin_id, _) = extract_super_admin_token(&headers, &state)
        .map_err(|_| (StatusCode::FORBIDDEN, Json(ValidationErrorsResponse { errors: vec!["forbidden".into()] })))?;
    // shape gate: must parse as the layout-core contract type
    if let Err(e) = serde_json::from_value::<layout_core::ScreenConfig>(req.config.clone()) {
        return Err(bad_request(vec![format!("invalid ScreenConfig: {e}")]));
    }
    let row = LayoutRepository::new()
        .upsert_draft(&state.db, &req.screen, &req.config, Some(admin_id))
        .await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?;
    Ok(Json(serde_json::to_value(row).unwrap_or_default()))
}

#[utoipa::path(put, path = "/api/v1/platform-admin/layout/rails", tag = "Layout Admin",
    security(("bearer_auth" = [])), request_body = PutRailsRequest,
    responses((status = 200, description = "Rails saved"), (status = 422, description = "Rails do not parse")))]
pub async fn put_rails(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<PutRailsRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ValidationErrorsResponse>)> {
    let (admin_id, _) = extract_super_admin_token(&headers, &state)
        .map_err(|_| (StatusCode::FORBIDDEN, Json(ValidationErrorsResponse { errors: vec!["forbidden".into()] })))?;
    if let Err(e) = serde_json::from_value::<layout_core::Rails>(req.rails.clone()) {
        return Err(bad_request(vec![format!("invalid Rails: {e}")]));
    }
    let row = LayoutRepository::new()
        .set_rails(&state.db, &req.screen, &req.rails, Some(admin_id))
        .await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?;
    Ok(Json(serde_json::to_value(row).unwrap_or_default()))
}

#[utoipa::path(get, path = "/api/v1/platform-admin/layout/manifests", tag = "Layout Admin",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "All registry manifests")))]
pub async fn list_manifests(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ValidationErrorsResponse>)> {
    let _admin = extract_super_admin_token(&headers, &state)
        .map_err(|_| (StatusCode::FORBIDDEN, Json(ValidationErrorsResponse { errors: vec!["forbidden".into()] })))?;
    let rows = LayoutRepository::new()
        .list_manifests(&state.db)
        .await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?;
    Ok(Json(serde_json::to_value(rows).unwrap_or_default()))
}

#[utoipa::path(put, path = "/api/v1/platform-admin/layout/manifests", tag = "Layout Admin",
    security(("bearer_auth" = [])), request_body = PutManifestRequest,
    responses((status = 200, description = "Manifest saved"), (status = 422, description = "Manifest invalid")))]
pub async fn put_manifest(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<PutManifestRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ValidationErrorsResponse>)> {
    let (admin_id, _) = extract_super_admin_token(&headers, &state)
        .map_err(|_| (StatusCode::FORBIDDEN, Json(ValidationErrorsResponse { errors: vec!["forbidden".into()] })))?;
    let parsed: layout_core::RegistryManifest = serde_json::from_value(req.manifest.clone())
        .map_err(|e| bad_request(vec![format!("invalid RegistryManifest: {e}")]))?;
    let platform_str = match parsed.platform {
        layout_core::Platform::Web => "web",
        layout_core::Platform::Mobile => "mobile",
    };
    if platform_str != req.platform {
        return Err(bad_request(vec![format!(
            "platform mismatch: body says {}, manifest says {platform_str}", req.platform
        )]));
    }
    let row = LayoutRepository::new()
        .upsert_manifest(&state.db, &req.platform, &req.manifest, Some(admin_id))
        .await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?;
    Ok(Json(serde_json::to_value(row).unwrap_or_default()))
}
