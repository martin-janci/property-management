//! Emergency statistics route surface — aggregate counts plus incident
//! breakdowns by type and severity.

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use common::ErrorResponse;

use super::shared::OrgQuery;
use crate::state::AppState;

/// Statistics sub-router.
pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/statistics", get(get_statistics))
        .route("/statistics/incidents/by-type", get(get_incidents_by_type))
        .route(
            "/statistics/incidents/by-severity",
            get(get_incidents_by_severity),
        )
}

async fn get_statistics(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(_): Query<OrgQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .get_statistics(&mut **rls.conn(), org)
        .await;
    rls.release().await;
    match result {
        Ok(stats) => Json(stats).into_response(),
        Err(e) => {
            tracing::error!("Failed to get statistics: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn get_incidents_by_type(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(_): Query<OrgQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .get_incident_summary_by_type(&mut **rls.conn(), org)
        .await;
    rls.release().await;
    match result {
        Ok(summary) => Json(summary).into_response(),
        Err(e) => {
            tracing::error!("Failed to get incidents by type: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn get_incidents_by_severity(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(_): Query<OrgQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .get_incident_summary_by_severity(&mut **rls.conn(), org)
        .await;
    rls.release().await;
    match result {
        Ok(summary) => Json(summary).into_response(),
        Err(e) => {
            tracing::error!("Failed to get incidents by severity: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}
