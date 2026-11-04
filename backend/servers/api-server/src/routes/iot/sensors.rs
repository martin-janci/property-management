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
use db::models::{
    sensor_connection_type, sensor_status, sensor_type, CreateSensor, SensorQuery, UpdateSensor,
};
use uuid::Uuid;

/// Reject a value outside the column's CHECK constraint with a `400` naming
/// the allowed values, instead of letting the INSERT/UPDATE fail as a `500`.
fn check_allowed(
    field: &str,
    value: Option<&str>,
    allowed: &[&str],
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    match value {
        Some(v) if !allowed.contains(&v) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "VALIDATION_ERROR",
                format!(
                    "Invalid {field} '{v}'; expected one of: {}",
                    allowed.join(", ")
                ),
            )),
        )),
        _ => Ok(()),
    }
}

/// Validate the enum-like fields of a create request (Story 14.1).
pub(super) fn validate_create_sensor(
    req: &CreateSensor,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    check_allowed("sensor_type", Some(&req.sensor_type), sensor_type::ALL)?;
    check_allowed(
        "connection_type",
        req.connection_type.as_deref(),
        sensor_connection_type::ALL,
    )
}

/// Validate the enum-like fields of an update request (Story 14.1).
pub(super) fn validate_update_sensor(
    req: &UpdateSensor,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    check_allowed(
        "connection_type",
        req.connection_type.as_deref(),
        sensor_connection_type::ALL,
    )?;
    check_allowed("status", req.status.as_deref(), sensor_status::ALL)
}

#[utoipa::path(
    post,
    path = "/api/v1/iot/sensors",
    request_body = CreateSensor,
    responses(
        (status = 201, description = "Sensor created"),
        (status = 400, description = "Invalid sensor_type or connection_type", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "IoT Sensors"
)]
pub(super) async fn create_sensor(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(mut req): Json<CreateSensor>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    // Validate before touching the connection so a bad value never reaches
    // the DB CHECK constraint (which surfaced as a 500).
    if let Err(e) = validate_create_sensor(&req) {
        rls.release().await;
        return Err(e);
    }
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
    if let Err(e) = validate_update_sensor(&req) {
        rls.release().await;
        return Err(e);
    }
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
