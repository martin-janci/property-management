//! Sensor CRUD handlers (Story 14.1).

use super::shared::{db_error, not_found, write_error};
use crate::state::AppState;
use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use common::errors::ErrorResponse;
use db::models::{CreateSensor, SensorQuery, UpdateSensor};
use uuid::Uuid;

#[utoipa::path(
    post,
    path = "/api/v1/iot/sensors",
    request_body = CreateSensor,
    responses(
        (status = 201, description = "Sensor created"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "IoT Sensors"
)]
pub(super) async fn create_sensor(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(mut req): Json<CreateSensor>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    // Pin the new sensor to the caller's validated org so the INSERT's
    // organization_id and the RLS WITH CHECK can never disagree.
    req.organization_id = rls.tenant_id();
    let out = state
        .sensor_repo
        .create(&mut **rls.conn(), req)
        .await
        .map(|sensor| (StatusCode::CREATED, Json(serde_json::json!(sensor))))
        .map_err(|e| db_error("Failed to create sensor", e));
    rls.release().await;
    out
}

#[utoipa::path(
    get,
    path = "/api/v1/iot/sensors",
    params(SensorQuery),
    responses(
        (status = 200, description = "Sensors list"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "IoT Sensors"
)]
pub(super) async fn list_sensors(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<SensorQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .sensor_repo
        .list(&mut **rls.conn(), org_id, query)
        .await
        .map(|sensors| Json(serde_json::json!({ "sensors": sensors })))
        .map_err(|e| db_error("Failed to list sensors", e));
    rls.release().await;
    out
}

pub(super) async fn get_sensor(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .sensor_repo
        .find_by_id(&mut **rls.conn(), id)
        .await
        .map_err(|e| db_error("Failed to get sensor", e))
        .and_then(|maybe| match maybe {
            Some(sensor) => Ok(Json(serde_json::json!(sensor))),
            None => Err(not_found("Sensor not found")),
        });
    rls.release().await;
    out
}

pub(super) async fn update_sensor(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateSensor>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .sensor_repo
        .update(&mut **rls.conn(), id, req)
        .await
        .map(|sensor| Json(serde_json::json!(sensor)))
        .map_err(|e| write_error("Failed to update sensor", "Sensor not found", e));
    rls.release().await;
    out
}

pub(super) async fn delete_sensor(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .sensor_repo
        .delete(&mut **rls.conn(), id)
        .await
        .map_err(|e| db_error("Failed to delete sensor", e))
        .and_then(|deleted| {
            if deleted {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err(not_found("Sensor not found"))
            }
        });
    rls.release().await;
    out
}
