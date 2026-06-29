//! Sensor alert handlers (Story 14.4) — list, acknowledge, resolve.

use super::shared::{db_error, write_error};
use crate::state::AppState;
use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use common::errors::ErrorResponse;
use db::models::AlertQuery;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(super) async fn list_sensor_alerts(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(mut query): Query<AlertQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    query.sensor_id = Some(id);
    let out = state
        .sensor_repo
        .list_alerts(&mut **rls.conn(), org_id, query)
        .await
        .map(|alerts| Json(serde_json::json!({ "alerts": alerts })))
        .map_err(|e| db_error("Failed to list alerts", e));
    rls.release().await;
    out
}

pub(super) async fn acknowledge_alert(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(alert_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();
    let out = state
        .sensor_repo
        .acknowledge_alert(&mut **rls.conn(), alert_id, user_id)
        .await
        .map(|alert| Json(serde_json::json!(alert)))
        .map_err(|e| write_error("Failed to acknowledge alert", "Alert not found", e));
    rls.release().await;
    out
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResolveAlertRequest {
    pub resolved_value: Option<f64>,
}

pub(super) async fn resolve_alert(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(alert_id): Path<Uuid>,
    Json(req): Json<ResolveAlertRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Pass resolved_value as Option - NULL is valid when value wasn't captured
    let out = state
        .sensor_repo
        .resolve_alert(&mut **rls.conn(), alert_id, req.resolved_value)
        .await
        .map(|alert| Json(serde_json::json!(alert)))
        .map_err(|e| write_error("Failed to resolve alert", "Alert not found", e));
    rls.release().await;
    out
}
