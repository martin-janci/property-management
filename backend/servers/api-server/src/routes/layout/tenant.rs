use crate::state::AppState;
use api_core::extractors::RlsConnection;
use api_core::{AuthUser, TenantExtractor};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use db::repositories::LayoutRepository;

use super::types::{PutTenantOverrideRequest, ScreenQuery, ValidationErrorsResponse};

#[utoipa::path(get, path = "/api/v1/layout/tenant-override", tag = "Layout",
    security(("bearer_auth" = [])), params(("screen" = String, Query, description = "Screen id")),
    responses((status = 200, description = "Override + rails + published base for the org")))]
pub async fn get_tenant_override(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Query(q): Query<ScreenQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ValidationErrorsResponse>)> {
    let repo = LayoutRepository::new();
    let org_id = tenant.tenant_id;
    let ov = repo
        .get_tenant_override(&mut **rls.conn(), org_id, &q.screen)
        .await;
    rls.release().await;
    let ov = ov.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR,
        Json(ValidationErrorsResponse { errors: vec![format!("db error: {e}")] })))?;
    let cfg = repo
        .get_config(&state.db, &q.screen)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR,
            Json(ValidationErrorsResponse { errors: vec![format!("db error: {e}")] })))?
        .ok_or((StatusCode::NOT_FOUND,
            Json(ValidationErrorsResponse { errors: vec!["unknown screen".into()] })))?;
    Ok(Json(serde_json::json!({
        "override": ov,
        "rails": cfg.rails,
        "published": cfg.published,
    })))
}

#[utoipa::path(put, path = "/api/v1/layout/tenant-override", tag = "Layout",
    security(("bearer_auth" = [])), request_body = PutTenantOverrideRequest,
    responses((status = 200, description = "Override saved"),
              (status = 403, description = "Org admin role required"),
              (status = 404, description = "Screen not published"),
              (status = 422, description = "Out-of-rails edits rejected")))]
pub async fn put_tenant_override(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Json(req): Json<PutTenantOverrideRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ValidationErrorsResponse>)> {
    if !tenant.role.is_admin() {
        rls.release().await;
        return Err((StatusCode::FORBIDDEN,
            Json(ValidationErrorsResponse { errors: vec!["org admin role required".into()] })));
    }
    let org_id = tenant.tenant_id;
    let repo = LayoutRepository::new();

    let parse_err = |e: serde_json::Error, what: &str| (StatusCode::UNPROCESSABLE_ENTITY,
        Json(ValidationErrorsResponse { errors: vec![format!("invalid {what}: {e}")] }));

    let ov: layout_core::TenantOverride = match serde_json::from_value(req.override_config.clone()) {
        Ok(v) => v,
        Err(e) => { rls.release().await; return Err(parse_err(e, "TenantOverride")); }
    };

    let cfg = match repo.get_config(&state.db, &req.screen).await {
        Ok(Some(c)) => c,
        Ok(None) => { rls.release().await; return Err((StatusCode::NOT_FOUND,
            Json(ValidationErrorsResponse { errors: vec!["unknown screen".into()] }))); }
        Err(e) => { rls.release().await; return Err((StatusCode::INTERNAL_SERVER_ERROR,
            Json(ValidationErrorsResponse { errors: vec![format!("db error: {e}")] }))); }
    };
    let Some(published) = cfg.published.clone() else {
        rls.release().await;
        return Err((StatusCode::NOT_FOUND,
            Json(ValidationErrorsResponse { errors: vec!["screen has no published config".into()] })));
    };
    let base: layout_core::ScreenConfig = match serde_json::from_value(published) {
        Ok(v) => v,
        Err(e) => { rls.release().await; return Err(parse_err(e, "stored published config")); }
    };
    let rails: layout_core::Rails = match serde_json::from_value(cfg.rails.clone()) {
        Ok(v) => v,
        Err(e) => { rls.release().await; return Err(parse_err(e, "stored rails")); }
    };

    let errors = layout_core::validate_tenant_override(&ov, &base, &rails);
    if !errors.is_empty() {
        rls.release().await;
        return Err((StatusCode::UNPROCESSABLE_ENTITY, Json(ValidationErrorsResponse {
            errors: errors.iter().map(|e| e.to_string()).collect(),
        })));
    }

    let saved = repo
        .upsert_tenant_override(&mut **rls.conn(), org_id, &req.screen,
                                &req.override_config, None)
        .await;
    rls.release().await;
    let saved = saved.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR,
        Json(ValidationErrorsResponse { errors: vec![format!("db error: {e}")] })))?;
    Ok(Json(serde_json::to_value(saved).unwrap_or_default()))
}
