//! Sensor reading handlers (Story 14.2) — list, single ingest, batch ingest,
//! and aggregated reads. Ingest handlers publish realtime events (Story 14.3)
//! via [`super::realtime`].

use super::realtime::{publish_sensor_event, EVENT_READINGS_BATCH, EVENT_READING_CREATED};
use super::shared::{db_error, ensure_in_org, not_found, IotManager, IotScope};
use crate::state::AppState;
use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use common::errors::ErrorResponse;
use db::models::{BatchSensorReadings, CreateSensorReading, ReadingQuery};
use uuid::Uuid;

pub(super) async fn list_readings(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<ReadingQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    ensure_in_org(&state, &mut rls, IotScope::Sensor(id)).await?;
    let out = state
        .sensor_repo
        .list_readings(&mut **rls.conn(), id, query)
        .await
        .map(|readings| Json(serde_json::json!({ "readings": readings })))
        .map_err(|e| db_error("Failed to list readings", e));
    rls.release().await;
    out
}

pub(super) async fn add_reading(
    State(state): State<AppState>,
    _manager: IotManager,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(mut req): Json<CreateSensorReading>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    req.sensor_id = id;
    // The validated org is both the realtime-fanout target and the scope folded
    // into the INSERT, so the sensor-in-org check and the insert happen in one
    // statement — no separate SELECT EXISTS round-trip or TOCTOU window on this
    // hot single-ingest path (issue #3305). Authority is `rls.tenant_id()`, the
    // validated tenant, never a client-supplied value (see module RLS note).
    let org_id = rls.tenant_id();
    let result = state
        .sensor_repo
        .create_reading(rls.conn(), org_id, req)
        .await;
    rls.release().await;

    match result {
        Ok(Some(reading)) => {
            // Story 14.3: push the new reading to org subscribers in real time.
            publish_sensor_event(
                state.pubsub_service.as_ref(),
                org_id,
                EVENT_READING_CREATED,
                serde_json::json!(reading),
            )
            .await;
            Ok((StatusCode::CREATED, Json(serde_json::json!(reading))))
        }
        Ok(None) => Err(not_found("Sensor not found")),
        Err(e) => Err(db_error("Failed to add reading", e)),
    }
}

pub(super) async fn add_batch_readings(
    State(state): State<AppState>,
    _manager: IotManager,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<BatchSensorReadings>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    // Fold the sensor-in-org guard into the bulk insert (issue #3305): a
    // cross-org or missing sensor writes zero rows and is reported as `None`.
    let org_id = rls.tenant_id();
    let result = state
        .sensor_repo
        .create_batch_readings(rls.conn(), id, org_id, req.readings)
        .await;
    rls.release().await;

    match result {
        Ok(Some(count)) => {
            // Story 14.3: one batch event carries the sensor and inserted count
            // so subscribers can refetch the affected series without a per-row
            // fanout.
            publish_sensor_event(
                state.pubsub_service.as_ref(),
                org_id,
                EVENT_READINGS_BATCH,
                serde_json::json!({ "sensor_id": id, "inserted": count }),
            )
            .await;
            Ok((
                StatusCode::CREATED,
                Json(serde_json::json!({ "inserted": count })),
            ))
        }
        Ok(None) => Err(not_found("Sensor not found")),
        Err(e) => Err(db_error("Failed to add batch readings", e)),
    }
}

pub(super) async fn get_aggregated_readings(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<ReadingQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    ensure_in_org(&state, &mut rls, IotScope::Sensor(id)).await?;
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
