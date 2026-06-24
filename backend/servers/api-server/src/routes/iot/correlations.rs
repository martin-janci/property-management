//! Sensor-fault correlation handlers (Story 14.5) — list, create, delete.

use super::shared::{db_error, insert_child_error, not_found};
use crate::state::AppState;
use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use common::errors::ErrorResponse;
use db::models::CreateSensorFaultCorrelation;
use uuid::Uuid;

pub(super) async fn list_correlations(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .sensor_repo
        .list_correlations_for_sensor(&mut **rls.conn(), id)
        .await
        .map(|correlations| Json(serde_json::json!({ "correlations": correlations })))
        .map_err(|e| db_error("Failed to list correlations", e));
    rls.release().await;
    out
}

pub(super) async fn create_correlation(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(mut req): Json<CreateSensorFaultCorrelation>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    req.sensor_id = id;
    req.created_by = Some(rls.user_id());
    let out = state
        .sensor_repo
        .create_correlation(&mut **rls.conn(), req)
        .await
        .map(|correlation| (StatusCode::CREATED, Json(serde_json::json!(correlation))))
        .map_err(|e| insert_child_error("Failed to create correlation", "Sensor not found", e));
    rls.release().await;
    out
}

pub(super) async fn delete_correlation(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(correlation_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .sensor_repo
        .delete_correlation(&mut **rls.conn(), correlation_id)
        .await
        .map_err(|e| db_error("Failed to delete correlation", e))
        .and_then(|deleted| {
            if deleted {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err(not_found("Correlation not found"))
            }
        });
    rls.release().await;
    out
}
