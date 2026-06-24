// Epic 135: Enhanced Tenant Screening — Credit / Background / Eviction results
//
// These three result kinds share an identical get-by-screening + create shape.
// See the module-level RLS note in `mod.rs` (PAP-67 / PAP-74).

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

// -----------------------------------------------------------------------------
// Credit Results
// -----------------------------------------------------------------------------

/// Get credit result.
pub(super) async fn get_credit_result(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(screening_id): Path<Uuid>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .get_credit_result_by_screening(&mut **rls.conn(), org_id, screening_id)
        .await;
    rls.release().await;

    match result {
        Ok(Some(result)) => Json(result).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Credit result not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Create credit result.
pub(super) async fn create_credit_result(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateScreeningCreditResult>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .create_credit_result(&mut **rls.conn(), org_id, req)
        .await;
    rls.release().await;

    match result {
        Ok(result) => (StatusCode::CREATED, Json(result)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// -----------------------------------------------------------------------------
// Background Results
// -----------------------------------------------------------------------------

/// Get background result.
pub(super) async fn get_background_result(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(screening_id): Path<Uuid>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .get_background_result_by_screening(&mut **rls.conn(), org_id, screening_id)
        .await;
    rls.release().await;

    match result {
        Ok(Some(result)) => Json(result).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Background result not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Create background result.
pub(super) async fn create_background_result(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateScreeningBackgroundResult>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .create_background_result(&mut **rls.conn(), org_id, req)
        .await;
    rls.release().await;

    match result {
        Ok(result) => (StatusCode::CREATED, Json(result)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// -----------------------------------------------------------------------------
// Eviction Results
// -----------------------------------------------------------------------------

/// Get eviction result.
pub(super) async fn get_eviction_result(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(screening_id): Path<Uuid>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .get_eviction_result_by_screening(&mut **rls.conn(), org_id, screening_id)
        .await;
    rls.release().await;

    match result {
        Ok(Some(result)) => Json(result).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Eviction result not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Create eviction result.
pub(super) async fn create_eviction_result(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateScreeningEvictionResult>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .create_eviction_result(&mut **rls.conn(), org_id, req)
        .await;
    rls.release().await;

    match result {
        Ok(result) => (StatusCode::CREATED, Json(result)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
