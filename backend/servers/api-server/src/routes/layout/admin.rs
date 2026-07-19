use crate::routes::platform_admin::extract_super_admin_token;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use db::repositories::LayoutRepository;

use super::types::{KillRequest, PublishRequest, PutDraftRequest, PutManifestRequest,
                   PutRailsRequest, RollbackRequest, ScreenQuery, ValidationErrorsResponse};

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

#[utoipa::path(post, path = "/api/v1/platform-admin/layout/publish", tag = "Layout Admin",
    security(("bearer_auth" = [])), request_body = PublishRequest,
    responses((status = 200, description = "Published"),
              (status = 404, description = "Unknown screen"),
              (status = 409, description = "No registry manifests uploaded yet"),
              (status = 422, description = "Validation errors — publish blocked")))]
pub async fn publish(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<PublishRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ValidationErrorsResponse>)> {
    let (admin_id, _) = extract_super_admin_token(&headers, &state)
        .map_err(|_| (StatusCode::FORBIDDEN, Json(ValidationErrorsResponse { errors: vec!["forbidden".into()] })))?;
    let repo = LayoutRepository::new();

    let cfg_row = repo
        .get_config(&state.db, &req.screen)
        .await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?
        .ok_or((StatusCode::NOT_FOUND, Json(ValidationErrorsResponse { errors: vec!["unknown screen".into()] })))?;

    let draft: layout_core::ScreenConfig = serde_json::from_value(cfg_row.draft.clone())
        .map_err(|e| bad_request(vec![format!("stored draft is not a valid ScreenConfig: {e}")]))?;

    let manifest_rows = repo
        .list_manifests(&state.db)
        .await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?;
    if manifest_rows.is_empty() {
        return Err((StatusCode::CONFLICT, Json(ValidationErrorsResponse {
            errors: vec!["no registry manifests uploaded; cannot validate publish".into()],
        })));
    }
    let manifests: Vec<layout_core::RegistryManifest> = manifest_rows
        .iter()
        .map(|r| serde_json::from_value(r.manifest.clone()))
        .collect::<Result<_, _>>()
        .map_err(|e| bad_request(vec![format!("stored manifest is invalid: {e}")]))?;

    let errors = layout_core::validate_publish(&draft, &manifests);
    if !errors.is_empty() {
        return Err(bad_request(errors.iter().map(|e| e.to_string()).collect()));
    }

    let mut conn = state.db.acquire().await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?;
    let row = repo
        .publish(&mut conn, &req.screen, Some(admin_id))
        .await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?;
    Ok(Json(serde_json::to_value(row).unwrap_or_default()))
}

#[utoipa::path(post, path = "/api/v1/platform-admin/layout/rollback", tag = "Layout Admin",
    security(("bearer_auth" = [])), request_body = RollbackRequest,
    responses((status = 200, description = "Rolled back"), (status = 404, description = "Unknown screen or version")))]
pub async fn rollback(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<RollbackRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ValidationErrorsResponse>)> {
    let (admin_id, _) = extract_super_admin_token(&headers, &state)
        .map_err(|_| (StatusCode::FORBIDDEN, Json(ValidationErrorsResponse { errors: vec!["forbidden".into()] })))?;
    let mut conn = state.db.acquire().await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?;
    let row = LayoutRepository::new()
        .rollback(&mut conn, &req.screen, req.version, Some(admin_id))
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND,
                Json(ValidationErrorsResponse { errors: vec!["unknown screen or version".into()] })),
            other => bad_request(vec![format!("db error: {other}")]),
        })?;
    Ok(Json(serde_json::to_value(row).unwrap_or_default()))
}

#[utoipa::path(post, path = "/api/v1/platform-admin/layout/kill", tag = "Layout Admin",
    security(("bearer_auth" = [])), request_body = KillRequest,
    responses((status = 204, description = "Section killed — bypasses the publish gate (spec §5)")))]
pub async fn kill(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<KillRequest>,
) -> Result<StatusCode, (StatusCode, Json<ValidationErrorsResponse>)> {
    let (admin_id, _) = extract_super_admin_token(&headers, &state)
        .map_err(|_| (StatusCode::FORBIDDEN, Json(ValidationErrorsResponse { errors: vec!["forbidden".into()] })))?;
    LayoutRepository::new()
        .kill(&state.db, &req.screen, &req.section_type, Some(admin_id))
        .await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/api/v1/platform-admin/layout/unkill", tag = "Layout Admin",
    security(("bearer_auth" = [])), request_body = KillRequest,
    responses((status = 204, description = "Kill flag removed"), (status = 404, description = "No such kill flag")))]
pub async fn unkill(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<KillRequest>,
) -> Result<StatusCode, (StatusCode, Json<ValidationErrorsResponse>)> {
    let _admin = extract_super_admin_token(&headers, &state)
        .map_err(|_| (StatusCode::FORBIDDEN, Json(ValidationErrorsResponse { errors: vec!["forbidden".into()] })))?;
    let removed = LayoutRepository::new()
        .unkill(&state.db, &req.screen, &req.section_type)
        .await
        .map_err(|e| bad_request(vec![format!("db error: {e}")]))?;
    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, Json(ValidationErrorsResponse { errors: vec!["no such kill flag".into()] })))
    }
}
