// Epic 135: Enhanced Tenant Screening — Reports and statistics
//
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
// Reports
// -----------------------------------------------------------------------------

/// Get reports for screening.
pub(super) async fn get_reports(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(screening_id): Path<Uuid>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .get_reports_by_screening(&mut **rls.conn(), org_id, screening_id)
        .await;
    rls.release().await;

    match result {
        Ok(reports) => Json(reports).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Create report.
pub(super) async fn create_report(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateScreeningReport>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let result = s
        .enhanced_tenant_screening_repo
        .create_report(&mut **rls.conn(), org_id, user_id, req)
        .await;
    rls.release().await;

    match result {
        Ok(report) => (StatusCode::CREATED, Json(report)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// -----------------------------------------------------------------------------
// Statistics
// -----------------------------------------------------------------------------

/// Get screening statistics.
pub(super) async fn get_statistics(
    State(s): State<AppState>,
    mut rls: RlsConnection,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .get_statistics(&mut **rls.conn(), org_id)
        .await;
    rls.release().await;

    match result {
        Ok(stats) => Json(stats).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Get risk distribution.
pub(super) async fn get_risk_distribution(
    State(s): State<AppState>,
    mut rls: RlsConnection,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .get_risk_distribution(&mut **rls.conn(), org_id)
        .await;
    rls.release().await;

    match result {
        Ok(dist) => Json(dist).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
