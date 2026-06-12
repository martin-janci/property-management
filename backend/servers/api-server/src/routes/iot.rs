//! IoT routes (Epic 14: IoT & Smart Building).
//!
//! Handles sensor registration, data ingestion, dashboards, alerts, and correlations.
//!
//! # RLS (PAP-67)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` on every sensor table
//! (`sensors`, `sensor_readings`, `sensor_alerts`, `sensor_thresholds`,
//! `sensor_threshold_templates`, `sensor_fault_correlations`), so every query
//! MUST run on a connection that has `app.current_org_id` set or it collapses to
//! deny-all. Each handler therefore acquires an [`RlsConnection`] (which
//! validates tenant membership and sets the org/user GUCs on a dedicated
//! connection) and passes `&mut **rls.conn()` to the repository. The
//! authoritative organization is `rls.tenant_id()` — the tenant the caller was
//! validated against — not a client-supplied `organization_id`, so the SQL org
//! filter and the RLS context can never disagree. Cross-tenant access is blocked
//! by RLS: a by-id read of another org's row returns no row (`404`), and a write
//! targeting another org fails the policy's `WITH CHECK`. `rls.release()` clears
//! the context before the connection returns to the pool.

use crate::state::AppState;
use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{
    AlertQuery, BatchSensorReadings, CreateSensor, CreateSensorFaultCorrelation,
    CreateSensorReading, CreateSensorThreshold, ReadingQuery, SensorQuery, UpdateSensor,
    UpdateSensorThreshold,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Error Helpers
// ============================================================================

/// Map a repository error to a `500` with a stable code, logging the cause.
fn db_error(msg: &'static str, e: sqlx::Error) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!("{}: {:?}", msg, e);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new("INTERNAL_ERROR", msg)),
    )
}

/// Build a `404` response.
fn not_found(msg: &'static str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", msg)),
    )
}

/// Map a `fetch_one` error from a by-id write to either `404` (the row is not
/// visible under the caller's RLS context — cross-tenant or genuinely missing)
/// or `500` for any other database failure.
fn write_error(
    msg: &'static str,
    not_found_msg: &'static str,
    e: sqlx::Error,
) -> (StatusCode, Json<ErrorResponse>) {
    match e {
        sqlx::Error::RowNotFound => not_found(not_found_msg),
        other => db_error(msg, other),
    }
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
async fn list_sensors(
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

async fn get_sensor(
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

async fn update_sensor(
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

async fn delete_sensor(
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

// ============================================================================
// Sensor Readings (Story 14.2)
// ============================================================================

async fn list_readings(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<ReadingQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .sensor_repo
        .list_readings(&mut **rls.conn(), id, query)
        .await
        .map(|readings| Json(serde_json::json!({ "readings": readings })))
        .map_err(|e| db_error("Failed to list readings", e));
    rls.release().await;
    out
}

async fn add_reading(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(mut req): Json<CreateSensorReading>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    req.sensor_id = id;
    let out = state
        .sensor_repo
        .create_reading(&mut **rls.conn(), req)
        .await
        .map(|reading| (StatusCode::CREATED, Json(serde_json::json!(reading))))
        .map_err(|e| db_error("Failed to add reading", e));
    rls.release().await;
    out
}

async fn add_batch_readings(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<BatchSensorReadings>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .sensor_repo
        .create_batch_readings(&mut **rls.conn(), id, req.readings)
        .await
        .map(|count| {
            (
                StatusCode::CREATED,
                Json(serde_json::json!({ "inserted": count })),
            )
        })
        .map_err(|e| db_error("Failed to add batch readings", e));
    rls.release().await;
    out
}

async fn get_aggregated_readings(
    State(state): State<AppState>,
    mut rls: RlsConnection,
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

    let out = state
        .sensor_repo
        .list_aggregated_readings(&mut **rls.conn(), id, from, to, &aggregation)
        .await
        .map(|readings| Json(serde_json::json!({ "aggregated_readings": readings })))
        .map_err(|e| db_error("Failed to get aggregated readings", e));
    rls.release().await;
    out
}

// ============================================================================
// Sensor Thresholds (Story 14.6)
// ============================================================================

async fn list_thresholds(
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

async fn create_threshold(
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
        .map_err(|e| db_error("Failed to create threshold", e));
    rls.release().await;
    out
}

async fn update_threshold(
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

async fn delete_threshold(
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
// Sensor Alerts (Story 14.4)
// ============================================================================

async fn list_sensor_alerts(
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

async fn acknowledge_alert(
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

async fn resolve_alert(
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

// ============================================================================
// Sensor-Fault Correlations (Story 14.5)
// ============================================================================

async fn list_correlations(
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

async fn create_correlation(
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
        .map_err(|e| db_error("Failed to create correlation", e));
    rls.release().await;
    out
}

async fn delete_correlation(
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

// ============================================================================
// Threshold Templates
// ============================================================================

async fn list_threshold_templates(
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

async fn apply_template(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(template_id): Path<Uuid>,
    Json(req): Json<ApplyTemplateRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .sensor_repo
        .apply_threshold_template(&mut **rls.conn(), template_id, req.sensor_id)
        .await
        .map(|threshold| (StatusCode::CREATED, Json(serde_json::json!(threshold))))
        .map_err(|e| write_error("Failed to apply template", "Template not found", e));
    rls.release().await;
    out
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
    mut rls: RlsConnection,
    Query(query): Query<DashboardQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .sensor_repo
        .get_dashboard(&mut **rls.conn(), org_id, query.building_id)
        .await
        .map(|dashboard| Json(serde_json::json!(dashboard)))
        .map_err(|e| db_error("Failed to get dashboard", e));
    rls.release().await;
    out
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DashboardQuery {
    pub building_id: Option<Uuid>,
}
