//! Sensor threshold handlers (Story 14.6) plus threshold-template listing and
//! application.

use super::shared::{db_error, insert_child_error, not_found, write_error};
use crate::state::AppState;
use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use common::errors::ErrorResponse;
use db::models::{CreateSensorThreshold, UpdateSensorThreshold};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(super) async fn list_thresholds(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .sensor_repo
        .list_thresholds(&mut **rls.conn(), id)
        .await
        .map(|thresholds| Json(serde_json::json!({ "thresholds": thresholds })))
        .map_err(|e| db_error("Failed to list thresholds", e));
    rls.release().await;
    out
}

pub(super) async fn create_threshold(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(mut req): Json<CreateSensorThreshold>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    req.sensor_id = id;
    let out = state
        .sensor_repo
        .create_threshold(&mut **rls.conn(), req)
        .await
        .map(|threshold| (StatusCode::CREATED, Json(serde_json::json!(threshold))))
        .map_err(|e| insert_child_error("Failed to create threshold", "Sensor not found", e));
    rls.release().await;
    out
}

pub(super) async fn update_threshold(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(threshold_id): Path<Uuid>,
    Json(req): Json<UpdateSensorThreshold>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .sensor_repo
        .update_threshold(&mut **rls.conn(), threshold_id, req)
        .await
        .map(|threshold| Json(serde_json::json!(threshold)))
        .map_err(|e| write_error("Failed to update threshold", "Threshold not found", e));
    rls.release().await;
    out
}

pub(super) async fn delete_threshold(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(threshold_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .sensor_repo
        .delete_threshold(&mut **rls.conn(), threshold_id)
        .await
        .map_err(|e| db_error("Failed to delete threshold", e))
        .and_then(|deleted| {
            if deleted {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err(not_found("Threshold not found"))
            }
        });
    rls.release().await;
    out
}

// ============================================================================
// Threshold Templates
// ============================================================================

pub(super) async fn list_threshold_templates(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<TemplateQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .sensor_repo
        .list_threshold_templates(
            &mut **rls.conn(),
            Some(org_id),
            query.sensor_type.as_deref(),
        )
        .await
        .map(|templates| Json(serde_json::json!({ "templates": templates })))
        .map_err(|e| db_error("Failed to list templates", e));
    rls.release().await;
    out
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TemplateQuery {
    pub sensor_type: Option<String>,
}

pub(super) async fn apply_template(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(template_id): Path<Uuid>,
    Json(req): Json<ApplyTemplateRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .sensor_repo
        .apply_threshold_template(rls.conn(), template_id, req.sensor_id)
        .await
        .map(|threshold| (StatusCode::CREATED, Json(serde_json::json!(threshold))))
        .map_err(|e| {
            insert_child_error(
                "Failed to apply template",
                "Template or sensor not found",
                e,
            )
        });
    rls.release().await;
    out
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApplyTemplateRequest {
    pub sensor_id: Uuid,
}
