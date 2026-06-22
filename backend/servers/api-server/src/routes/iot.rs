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

use std::time::Duration;

use crate::state::AppState;
use api_core::extractors::{validate_access_token_with_exp, RlsConnection};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{
    AlertQuery, BatchSensorReadings, CreateSensor, CreateSensorFaultCorrelation,
    CreateSensorReading, CreateSensorThreshold, ReadingQuery, SensorQuery, UpdateSensor,
    UpdateSensorThreshold,
};
use db::repositories::OrganizationMemberRepository;
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

/// Map an INSERT error on a child table to `404` when the parent is not
/// visible in the caller's RLS context (`42501` — RLS WITH CHECK rejected)
/// or does not exist (`23503` — FK violation); anything else is `500`.
fn insert_child_error(
    msg: &'static str,
    not_found_msg: &'static str,
    e: sqlx::Error,
) -> (StatusCode, Json<ErrorResponse>) {
    if let sqlx::Error::Database(ref db_err) = e {
        if matches!(db_err.code().as_deref(), Some("42501") | Some("23503")) {
            return not_found(not_found_msg);
        }
    }
    db_error(msg, e)
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
        // Realtime sensor-reading push channel (Story 14.3, gap #985-2).
        .route("/ws", get(sensor_ws_handler))
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
    // Capture the validated org before releasing the RLS connection so the
    // realtime fanout targets the caller's tenant and never a client-supplied
    // value (same authority as the SQL filter — see module RLS note).
    let org_id = rls.tenant_id();
    let result = state.sensor_repo.create_reading(rls.conn(), req).await;
    rls.release().await;

    match result {
        Ok(reading) => {
            // Story 14.3: push the new reading to org subscribers in real time.
            publish_sensor_event(
                state.pubsub_service.as_ref(),
                org_id,
                "sensor.reading.created",
                serde_json::json!(reading),
            )
            .await;
            Ok((StatusCode::CREATED, Json(serde_json::json!(reading))))
        }
        Err(e) => Err(insert_child_error(
            "Failed to add reading",
            "Sensor not found",
            e,
        )),
    }
}

async fn add_batch_readings(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<BatchSensorReadings>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let result = state
        .sensor_repo
        .create_batch_readings(rls.conn(), id, req.readings)
        .await;
    rls.release().await;

    match result {
        Ok(count) => {
            // Story 14.3: one batch event carries the sensor and inserted count
            // so subscribers can refetch the affected series without a per-row
            // fanout.
            publish_sensor_event(
                state.pubsub_service.as_ref(),
                org_id,
                "sensor.readings.batch",
                serde_json::json!({ "sensor_id": id, "inserted": count }),
            )
            .await;
            Ok((
                StatusCode::CREATED,
                Json(serde_json::json!({ "inserted": count })),
            ))
        }
        Err(e) => Err(insert_child_error(
            "Failed to add batch readings",
            "Sensor not found",
            e,
        )),
    }
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
        .map_err(|e| insert_child_error("Failed to create threshold", "Sensor not found", e));
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
        .map_err(|e| insert_child_error("Failed to create correlation", "Sensor not found", e));
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
        .get_dashboard(rls.conn(), org_id, query.building_id)
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

// ============================================================================
// Realtime sensor channel (Story 14.3 — "values update in real-time")
// ============================================================================
//
// Story 14.3 AC requires sensor values to update in real time over WebSocket.
// The router above exposes only REST, so this section adds a push channel that
// mirrors the `ws_notifications.rs` pattern (Epic 8A.3): an authenticated WS
// upgrade subscribes to a Redis pub/sub channel and forwards events as JSON
// text frames. Here the channel is **org-scoped** (`sensors:{org_id}`) and the
// ingest handlers (`add_reading` / `add_batch_readings`) publish onto it.
//
// # Authentication & tenant scoping
//
// WS upgrades cannot carry an `Authorization` header from browsers, so auth is
// a short-lived JWT in the `token` query param (same verifier as REST). The
// caller also passes the `organization_id` they want to subscribe to, and we
// validate active membership against the DB exactly like
// `ValidatedTenantExtractor` does — a non-member is rejected with `403` before
// the upgrade, so the channel can never leak another tenant's readings.

/// Maximum idle time before the server closes the connection (60 s).
const WS_IDLE_TIMEOUT_SECS: u64 = 60;

/// Maximum lifetime of a WebSocket session regardless of activity (4 h).
/// Clients reconnect with a fresh token before their JWT (15 m) expires.
const WS_MAX_SESSION_SECS: u64 = 4 * 60 * 60;

/// Query parameters for the sensor WebSocket upgrade request.
#[derive(Debug, Deserialize)]
pub struct SensorWsQuery {
    /// JWT access token — required because WS upgrades cannot carry a custom
    /// `Authorization` header from browser clients.
    pub token: String,
    /// Organization (tenant) whose sensor stream to subscribe to. Validated
    /// against the caller's active membership before the upgrade.
    pub organization_id: Uuid,
}

/// Envelope for every server-to-client sensor WebSocket frame.
#[derive(Debug, Serialize)]
pub struct SensorWsEvent {
    /// Mirrors `PubSubMessage::event_type` (`sensor.reading.created`,
    /// `sensor.readings.batch`).
    pub event: String,
    /// Opaque JSON payload forwarded from the Redis pub/sub message.
    pub payload: serde_json::Value,
}

/// Build the org-scoped Redis pub/sub channel name for sensor readings.
fn sensor_channel(org_id: Uuid) -> String {
    format!("sensors:{org_id}")
}

/// Publish a sensor realtime event to the org-scoped channel.
///
/// Best-effort: the reading is already persisted, so a pub/sub failure (or no
/// Redis configured in local dev) is logged and swallowed rather than failing
/// the ingest request — subscribers still see the value on their next fetch.
async fn publish_sensor_event(
    pubsub: Option<&integrations::PubSubService>,
    org_id: Uuid,
    event_type: &str,
    payload: serde_json::Value,
) {
    let Some(pubsub) = pubsub else {
        return;
    };
    let channel = sensor_channel(org_id);
    let msg = integrations::PubSubMessage::new(&channel, event_type, payload);
    if let Err(e) = pubsub.publish(&channel, msg).await {
        tracing::warn!(
            org_id = %org_id,
            channel = %channel,
            event = %event_type,
            error = %e,
            "[iot] Failed to publish sensor event to WebSocket channel (non-fatal)"
        );
    }
}

/// WebSocket upgrade handler for realtime sensor readings (Story 14.3).
///
/// Validates the `token` query param as a JWT access token, confirms the caller
/// is an active member of `organization_id`, then upgrades the connection and
/// forwards the org's Redis pub/sub sensor events to the client.
pub async fn sensor_ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<SensorWsQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // --- JWT validation (same verifier/lifetime as REST auth) ---
    let (user_id, exp) = match validate_access_token_with_exp(&params.token) {
        Ok(t) => t,
        Err(msg) => {
            // Never log `params.token` — it is a bearer JWT.
            tracing::warn!(msg = %msg, "Sensor WS rejected: invalid token");
            return (StatusCode::UNAUTHORIZED, msg).into_response();
        }
    };

    // --- Tenant membership check (mirrors ValidatedTenantExtractor) ---
    let org_id = params.organization_id;
    let member_repo = OrganizationMemberRepository::new(state.db.clone());
    match member_repo.is_member(org_id, user_id).await {
        Ok(true) => {}
        Ok(false) => {
            tracing::warn!(
                user_id = %user_id,
                org_id = %org_id,
                "Sensor WS rejected: user is not a member of the requested org"
            );
            return (StatusCode::FORBIDDEN, "Not a member of this organization").into_response();
        }
        Err(e) => {
            tracing::error!(
                user_id = %user_id,
                org_id = %org_id,
                error = %e,
                "Sensor WS: failed to verify org membership"
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to verify organization access",
            )
                .into_response();
        }
    }

    let pubsub_service = state.pubsub_service.clone();

    // Echo the `ppt.v1` subprotocol so browsers that offer it complete the
    // handshake (same compat shim as ws_notifications, issue #438).
    ws.protocols(["ppt.v1"])
        .on_upgrade(move |socket| {
            handle_sensor_ws_session(socket, org_id, user_id, exp, pubsub_service)
        })
        .into_response()
}

/// Drive one sensor WebSocket session for `org_id`.
///
/// Subscribes to `sensors:{org_id}` on Redis (when available) and forwards
/// matching pub/sub events. Falls back to a heartbeat-only loop when Redis is
/// not configured. Closes once the JWT expires (issue #480 parity) or the max
/// session lifetime is reached.
async fn handle_sensor_ws_session(
    mut socket: WebSocket,
    org_id: Uuid,
    user_id: Uuid,
    jwt_exp_unix: i64,
    pubsub_service: Option<integrations::PubSubService>,
) {
    tracing::info!(org_id = %org_id, user_id = %user_id, "Sensor WS session opened");

    let channel = sensor_channel(org_id);

    let mut pubsub_rx = match pubsub_service {
        Some(ref svc) => match svc.subscribe(&channel).await {
            Ok(rx) => {
                tracing::debug!(channel = %channel, "Subscribed to Redis sensor channel");
                Some(rx)
            }
            Err(e) => {
                tracing::warn!(
                    channel = %channel,
                    error = %e,
                    "Failed to subscribe to Redis sensor channel; heartbeat-only mode"
                );
                None
            }
        },
        None => {
            tracing::debug!(
                org_id = %org_id,
                "Redis not configured; sensor WS running in heartbeat-only mode"
            );
            None
        }
    };

    let idle_timeout = Duration::from_secs(WS_IDLE_TIMEOUT_SECS);
    let max_session = Duration::from_secs(WS_MAX_SESSION_SECS);
    let session_deadline = tokio::time::Instant::now() + max_session;

    loop {
        if tokio::time::Instant::now() >= session_deadline {
            tracing::info!(org_id = %org_id, "Sensor WS max-lifetime reached; closing");
            break;
        }

        // Close as soon as the JWT expires so a logged-out user does not retain
        // a live channel (issue #480 parity).
        if chrono::Utc::now().timestamp() >= jwt_exp_unix {
            tracing::info!(org_id = %org_id, "Sensor WS JWT expired; closing");
            break;
        }

        tokio::select! {
            // ---- Redis pub/sub event ----
            pubsub_msg = async {
                match pubsub_rx.as_mut() {
                    Some(rx) => rx.recv().await.ok(),
                    None => {
                        tokio::time::sleep(Duration::from_secs(30)).await;
                        None
                    }
                }
            } => {
                if let Some(msg) = pubsub_msg {
                    let envelope = SensorWsEvent {
                        event: msg.event_type.clone(),
                        payload: msg.payload.clone(),
                    };
                    let text = match serde_json::to_string(&envelope) {
                        Ok(t) => t,
                        Err(e) => {
                            tracing::warn!(error = %e, "Failed to serialise SensorWsEvent; skipping");
                            continue;
                        }
                    };
                    if socket.send(Message::Text(text.into())).await.is_err() {
                        tracing::debug!(org_id = %org_id, "Sensor WS send failed; closing");
                        break;
                    }
                }
            }

            // ---- Inbound client frame (heartbeat / close) ----
            inbound = tokio::time::timeout(idle_timeout, socket.recv()) => {
                match inbound {
                    Err(_elapsed) => {
                        if socket.send(Message::Ping(vec![].into())).await.is_err() {
                            tracing::debug!(org_id = %org_id, "Sensor WS ping failed; closing");
                            break;
                        }
                    }
                    Ok(Some(Ok(Message::Close(_)))) | Ok(None) => {
                        tracing::debug!(org_id = %org_id, "Sensor WS closed by client");
                        break;
                    }
                    Ok(Some(Err(e))) => {
                        tracing::debug!(org_id = %org_id, error = %e, "Sensor WS receive error; closing");
                        break;
                    }
                    Ok(Some(Ok(_other))) => {
                        tracing::trace!(org_id = %org_id, "Sensor WS heartbeat received");
                    }
                }
            }
        }
    }

    tracing::info!(org_id = %org_id, user_id = %user_id, "Sensor WS session closed");
}

#[cfg(test)]
mod realtime_tests {
    use super::*;

    #[test]
    fn sensor_channel_is_org_scoped() {
        let org = Uuid::parse_str("00000000-0000-0000-0000-0000000000aa").unwrap();
        assert_eq!(
            sensor_channel(org),
            "sensors:00000000-0000-0000-0000-0000000000aa"
        );
    }

    #[test]
    fn different_orgs_get_distinct_channels() {
        let a = Uuid::parse_str("00000000-0000-0000-0000-0000000000a1").unwrap();
        let b = Uuid::parse_str("00000000-0000-0000-0000-0000000000b2").unwrap();
        assert_ne!(sensor_channel(a), sensor_channel(b));
    }

    #[test]
    fn sensor_ws_event_serialises_with_event_and_payload() {
        let evt = SensorWsEvent {
            event: "sensor.reading.created".to_string(),
            payload: serde_json::json!({
                "id": "00000000-0000-0000-0000-000000000001",
                "sensor_id": "00000000-0000-0000-0000-000000000002",
                "value": 21.5,
                "unit": "C"
            }),
        };
        let text = serde_json::to_string(&evt).unwrap();
        assert!(text.contains("\"event\":\"sensor.reading.created\""));
        assert!(text.contains("\"payload\""));
        assert!(text.contains("\"value\":21.5"));
    }

    #[test]
    fn batch_event_carries_sensor_and_count() {
        let evt = SensorWsEvent {
            event: "sensor.readings.batch".to_string(),
            payload: serde_json::json!({
                "sensor_id": "00000000-0000-0000-0000-000000000002",
                "inserted": 7
            }),
        };
        let text = serde_json::to_string(&evt).unwrap();
        assert!(text.contains("\"event\":\"sensor.readings.batch\""));
        assert!(text.contains("\"inserted\":7"));
    }
}
