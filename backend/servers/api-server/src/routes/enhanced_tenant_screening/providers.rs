// Epic 135: Enhanced Tenant Screening — Provider Configs
//
// See the module-level RLS note in `mod.rs` (PAP-67 / PAP-74).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use api_core::extractors::RlsConnection;
use db::models::enhanced_tenant_screening::*;

use crate::state::AppState;

/// List provider configs.
pub(super) async fn list_provider_configs(
    State(s): State<AppState>,
    mut rls: RlsConnection,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .list_provider_configs(&mut **rls.conn(), org_id)
        .await;
    rls.release().await;

    match result {
        Ok(configs) => Json(configs).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Create provider config.
pub(super) async fn create_provider_config(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateScreeningProviderConfig>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .create_provider_config(&mut **rls.conn(), org_id, req)
        .await;
    rls.release().await;

    match result {
        Ok(config) => (StatusCode::CREATED, Json(config)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Get provider config.
pub(super) async fn get_provider_config(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .get_provider_config(&mut **rls.conn(), org_id, id)
        .await;
    rls.release().await;

    match result {
        Ok(Some(config)) => Json(config).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Provider config not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Delete provider config.
pub(super) async fn delete_provider_config(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let result = s
        .enhanced_tenant_screening_repo
        .delete_provider_config(&mut **rls.conn(), id)
        .await;
    rls.release().await;

    match result {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "Provider config not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct UpdateProviderStatusRequest {
    status: ProviderIntegrationStatus,
    error_message: Option<String>,
}

/// Update provider status.
pub(super) async fn update_provider_status(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProviderStatusRequest>,
) -> impl IntoResponse {
    let result = s
        .enhanced_tenant_screening_repo
        .update_provider_status(
            &mut **rls.conn(),
            id,
            req.status,
            req.error_message.as_deref(),
        )
        .await;
    rls.release().await;

    match result {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
