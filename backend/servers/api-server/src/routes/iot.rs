//! IoT routes (Epic 14: IoT & Smart Building).
//!
//! Handles sensor registration, data ingestion, dashboards, alerts, and correlations.

use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post, put},
    Json, Router,
};
use common::{errors::ErrorResponse, TenantContext};
use db::models::{
    AlertQuery, BatchSensorReadings, CreateSensor, CreateSensorFaultCorrelation,
    CreateSensorReading, CreateSensorThreshold, ReadingQuery, SensorQuery, UpdateSensor,
    UpdateSensorThreshold,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract tenant context from request headers.
fn extract_tenant_context(
    headers: &HeaderMap,
) -> Result<TenantContext, (StatusCode, Json<ErrorResponse>)> {
    let tenant_header = headers
        .get("X-Tenant-Context")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "MISSING_CONTEXT",
                    "Tenant context required",
                )),
            )
        })?;

    serde_json::from_str(tenant_header).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_CONTEXT",
                "Invalid tenant context format",
            )),
        )
    })
}

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}

// ============================================================================
// Query Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Default, utoipa::IntoParams)]
pub struct PaginationQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ============================================================================
// Sensor Router (Story 14.1)
// ============================================================================

pub fn sensor_router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_sensor))
        .route("/", get(list_sensors))
        .route("/{id}", get(get_sensor))
        .route("/{id}", put(update_sensor))
        .route("/{id}", delete(delete_sensor))
        .route("/{id}/readings", get(list_readings))
        .route("/{id}/readings", post(add_reading))
        .route("/{id}/readings/batch", post(add_batch_readings))
        .route("/{id}/readings/aggregated", get(get_aggregated_readings))
        .route("/{id}/thresholds", get(list_thresholds))
        .route("/{id}/thresholds", post(create_threshold))
        .route("/thresholds/{threshold_id}", put(update_threshold))
        .route("/thresholds/{threshold_id}", delete(delete_threshold))
        .route("/{id}/alerts", get(list_sensor_alerts))
        .route("/alerts/{alert_id}/acknowledge", post(acknowledge_alert))
        .route("/alerts/{alert_id}/resolve", post(resolve_alert))
        .route("/{id}/correlations", get(list_correlations))
        .route("/{id}/correlations", post(create_correlation))
        .route("/correlations/{correlation_id}", delete(delete_correlation))
        .route("/templates", get(list_threshold_templates))
        .route("/templates/{template_id}/apply", post(apply_template))
        .route("/dashboard", get(get_dashboard))
}

// ============================================================================
// Sensor CRUD (Story 14.1)
// ============================================================================

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
async fn create_sensor(
    State(state): State<AppState>,
    Json(req): Json<CreateSensor>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    match state.sensor_repo.create(req).await {
        Ok(sensor) => Ok((StatusCode::CREATED, Json(serde_json::json!(sensor)))),
        Err(e) => {
            tracing::error!("Failed to create sensor: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to create sensor",
                )),
            ))
        }
    }
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
async fn list_sensors(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SensorQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    match state.sensor_repo.list(tenant.tenant_id, query).await {
        Ok(sensors) => Ok(Json(serde_json::json!({ "sensors": sensors }))),
        Err(e) => {
            tracing::error!("Failed to list sensors: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list sensors",
                )),
            ))
        }
    }
}

async fn get_sensor(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.sensor_repo.find_by_id(id).await {
        Ok(Some(sensor)) => Ok(Json(serde_json::json!(sensor))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Sensor not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get sensor: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get sensor")),
            ))
        }
    }
}

async fn update_sensor(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateSensor>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.sensor_repo.update(id, req).await {
        Ok(sensor) => Ok(Json(serde_json::json!(sensor))),
        Err(e) => {
            tracing::error!("Failed to update sensor: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to update sensor",
                )),
            ))
        }
    }
}

async fn delete_sensor(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match state.sensor_repo.delete(id).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Sensor not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to delete sensor: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to delete sensor",
                )),
            ))
        }
    }
}

// ============================================================================
// Sensor Readings (Story 14.2)
// ============================================================================

async fn list_readings(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<ReadingQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.sensor_repo.list_readings(id, query).await {
        Ok(readings) => Ok(Json(serde_json::json!({ "readings": readings }))),
        Err(e) => {
            tracing::error!("Failed to list readings: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list readings",
                )),
            ))
        }
    }
}

async fn add_reading(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(mut req): Json<CreateSensorReading>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    req.sensor_id = id;

    match state.sensor_repo.create_reading(req).await {
        Ok(reading) => Ok((StatusCode::CREATED, Json(serde_json::json!(reading)))),
        Err(e) => {
            tracing::error!("Failed to add reading: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to add reading",
                )),
            ))
        }
    }
}

async fn add_batch_readings(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<BatchSensorReadings>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    match state
        .sensor_repo
        .create_batch_readings(id, req.readings)
        .await
    {
        Ok(count) => Ok((
            StatusCode::CREATED,
            Json(serde_json::json!({ "inserted": count })),
        )),
        Err(e) => {
            tracing::error!("Failed to add batch readings: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to add batch readings",
                )),
            ))
        }
    }
}

async fn get_aggregated_readings(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<ReadingQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let aggregation = query
        .aggregation
        .clone()
        .unwrap_or_else(|| "hour".to_string());
    let from = query
        .from_time
        .unwrap_or_else(|| chrono::Utc::now() - chrono::Duration::hours(24));
    let to = query.to_time.unwrap_or_else(chrono::Utc::now);

    match state
        .sensor_repo
        .list_aggregated_readings(id, from, to, &aggregation)
        .await
    {
        Ok(readings) => Ok(Json(serde_json::json!({ "aggregated_readings": readings }))),
        Err(e) => {
            tracing::error!("Failed to get aggregated readings: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get aggregated readings",
                )),
            ))
        }
    }
}

// ============================================================================
// Sensor Thresholds (Story 14.6)
// ============================================================================

async fn list_thresholds(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.sensor_repo.list_thresholds(id).await {
        Ok(thresholds) => Ok(Json(serde_json::json!({ "thresholds": thresholds }))),
        Err(e) => {
            tracing::error!("Failed to list thresholds: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list thresholds",
                )),
            ))
        }
    }
}

async fn create_threshold(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(mut req): Json<CreateSensorThreshold>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    req.sensor_id = id;

    match state.sensor_repo.create_threshold(req).await {
        Ok(threshold) => Ok((StatusCode::CREATED, Json(serde_json::json!(threshold)))),
        Err(e) => {
            tracing::error!("Failed to create threshold: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to create threshold",
                )),
            ))
        }
    }
}

async fn update_threshold(
    State(state): State<AppState>,
    Path(threshold_id): Path<Uuid>,
    Json(req): Json<UpdateSensorThreshold>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.sensor_repo.update_threshold(threshold_id, req).await {
        Ok(threshold) => Ok(Json(serde_json::json!(threshold))),
        Err(e) => {
            tracing::error!("Failed to update threshold: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to update threshold",
                )),
            ))
        }
    }
}

async fn delete_threshold(
    State(state): State<AppState>,
    Path(threshold_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match state.sensor_repo.delete_threshold(threshold_id).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Threshold not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to delete threshold: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to delete threshold",
                )),
            ))
        }
    }
}

// ============================================================================
// Sensor Alerts (Story 14.4)
// ============================================================================

async fn list_sensor_alerts(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Query(mut query): Query<AlertQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;
    query.sensor_id = Some(id);

    match state.sensor_repo.list_alerts(tenant.tenant_id, query).await {
        Ok(alerts) => Ok(Json(serde_json::json!({ "alerts": alerts }))),
        Err(e) => {
            tracing::error!("Failed to list alerts: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list alerts",
                )),
            ))
        }
    }
}

async fn acknowledge_alert(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(alert_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    match state
        .sensor_repo
        .acknowledge_alert(alert_id, tenant.user_id)
        .await
    {
        Ok(alert) => Ok(Json(serde_json::json!(alert))),
        Err(e) => {
            tracing::error!("Failed to acknowledge alert: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to acknowledge alert",
                )),
            ))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResolveAlertRequest {
    pub resolved_value: Option<f64>,
}

async fn resolve_alert(
    State(state): State<AppState>,
    Path(alert_id): Path<Uuid>,
    Json(req): Json<ResolveAlertRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Pass resolved_value as Option - NULL is valid when value wasn't captured
    match state
        .sensor_repo
        .resolve_alert(alert_id, req.resolved_value)
        .await
    {
        Ok(alert) => Ok(Json(serde_json::json!(alert))),
        Err(e) => {
            tracing::error!("Failed to resolve alert: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to resolve alert",
                )),
            ))
        }
    }
}

// ============================================================================
// Sensor-Fault Correlations (Story 14.5)
// ============================================================================

async fn list_correlations(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.sensor_repo.list_correlations_for_sensor(id).await {
        Ok(correlations) => Ok(Json(serde_json::json!({ "correlations": correlations }))),
        Err(e) => {
            tracing::error!("Failed to list correlations: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list correlations",
                )),
            ))
        }
    }
}

async fn create_correlation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(mut req): Json<CreateSensorFaultCorrelation>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;
    req.sensor_id = id;
    req.created_by = Some(tenant.user_id);

    match state.sensor_repo.create_correlation(req).await {
        Ok(correlation) => Ok((StatusCode::CREATED, Json(serde_json::json!(correlation)))),
        Err(e) => {
            tracing::error!("Failed to create correlation: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to create correlation",
                )),
            ))
        }
    }
}

async fn delete_correlation(
    State(state): State<AppState>,
    Path(correlation_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match state.sensor_repo.delete_correlation(correlation_id).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Correlation not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to delete correlation: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to delete correlation",
                )),
            ))
        }
    }
}

// ============================================================================
// Threshold Templates
// ============================================================================

async fn list_threshold_templates(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<TemplateQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    match state
        .sensor_repo
        .list_threshold_templates(Some(tenant.tenant_id), query.sensor_type.as_deref())
        .await
    {
        Ok(templates) => Ok(Json(serde_json::json!({ "templates": templates }))),
        Err(e) => {
            tracing::error!("Failed to list templates: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list templates",
                )),
            ))
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TemplateQuery {
    pub sensor_type: Option<String>,
}

async fn apply_template(
    State(state): State<AppState>,
    Path(template_id): Path<Uuid>,
    Json(req): Json<ApplyTemplateRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    match state
        .sensor_repo
        .apply_threshold_template(template_id, req.sensor_id)
        .await
    {
        Ok(threshold) => Ok((StatusCode::CREATED, Json(serde_json::json!(threshold)))),
        Err(e) => {
            tracing::error!("Failed to apply template: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to apply template",
                )),
            ))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApplyTemplateRequest {
    pub sensor_id: Uuid,
}

// ============================================================================
// Dashboard (Story 14.3)
// ============================================================================

async fn get_dashboard(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<DashboardQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    match state
        .sensor_repo
        .get_dashboard(tenant.tenant_id, query.building_id)
        .await
    {
        Ok(dashboard) => Ok(Json(serde_json::json!(dashboard))),
        Err(e) => {
            tracing::error!("Failed to get dashboard: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get dashboard",
                )),
            ))
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DashboardQuery {
    pub building_id: Option<Uuid>,
}
