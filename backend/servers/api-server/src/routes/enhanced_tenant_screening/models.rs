// Epic 135: Enhanced Tenant Screening — AI Risk Scoring Models
//
// See the module-level RLS note in `mod.rs` (PAP-67 / PAP-74): every handler
// acquires an [`RlsConnection`], passes `&mut **rls.conn()` to the repository,
// and calls `rls.release()` on every Ok/Err path.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use api_core::extractors::RlsConnection;
use db::models::enhanced_tenant_screening::*;

use crate::state::AppState;

/// List all risk models for organization.
pub(super) async fn list_risk_models(
    State(s): State<AppState>,
    mut rls: RlsConnection,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .list_risk_models(&mut **rls.conn(), org_id)
        .await;
    rls.release().await;

    match result {
        Ok(models) => Json(models).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Create a new AI risk scoring model.
pub(super) async fn create_risk_model(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateAiRiskScoringModel>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let result = s
        .enhanced_tenant_screening_repo
        .create_risk_model(&mut **rls.conn(), org_id, user_id, req)
        .await;
    rls.release().await;

    match result {
        Ok(model) => (StatusCode::CREATED, Json(model)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Get a risk model by ID.
pub(super) async fn get_risk_model(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .get_risk_model(&mut **rls.conn(), org_id, id)
        .await;
    rls.release().await;

    match result {
        Ok(Some(model)) => Json(model).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Risk model not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Delete a risk model.
pub(super) async fn delete_risk_model(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let result = s
        .enhanced_tenant_screening_repo
        .delete_risk_model(&mut **rls.conn(), id)
        .await;
    rls.release().await;

    match result {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "Risk model not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Activate a risk model.
pub(super) async fn activate_risk_model(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .activate_risk_model(rls.conn(), org_id, id)
        .await;
    rls.release().await;

    match result {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
