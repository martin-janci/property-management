//! Routes for Epic 134: Predictive Maintenance & Equipment Intelligence.
//!
//! - Story 134.1: Equipment Registry
//! - Story 134.2: Maintenance History Tracking
//! - Story 134.3: Failure Prediction Engine
//! - Story 134.4: Predictive Maintenance Dashboard
//!
//! # RLS (PAP-80)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` on the
//! predictive-maintenance tables, so every query MUST run on a connection that
//! has `app.current_org_id` set or it collapses to deny-all. Each handler
//! therefore acquires an [`RlsConnection`] (which validates tenant membership
//! and sets the org/user GUCs on a dedicated connection) and passes
//! `&mut **rls.conn()` to the repository. The authoritative organization is
//! `rls.tenant_id()` — the tenant the caller was validated against — so the SQL
//! org filter and the RLS context can never disagree. Cross-tenant access is
//! blocked by RLS: a by-id read of another org's row returns no row (`404`), and
//! a write targeting another org fails the policy's `WITH CHECK`. `rls.release()`
//! clears the context before the connection returns to the pool.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use api_core::extractors::RlsConnection;

use crate::routes::pagination::clamp_limit;
use crate::state::AppState;

use db::models::predictive_maintenance::{
    AcknowledgeAlertRequest, CreateEquipment, CreateEquipmentDocument, CreateMaintenanceLog,
    EquipmentQuery, ResolveAlertRequest, RunPredictionRequest, SetHealthThreshold, UpdateEquipment,
    UpdateMaintenanceLog,
};

/// Create the predictive maintenance router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Equipment registry (Story 134.1)
        .route("/equipment", post(create_equipment))
        .route("/equipment", get(list_equipment))
        .route("/equipment/{id}", get(get_equipment))
        .route("/equipment/{id}", put(update_equipment))
        .route("/equipment/{id}", delete(delete_equipment))
        .route("/equipment/{id}/documents", post(add_equipment_document))
        .route("/equipment/{id}/documents", get(list_equipment_documents))
        // Maintenance logs (Story 134.2)
        .route("/maintenance-logs", post(create_maintenance_log))
        .route("/maintenance-logs/{id}", get(get_maintenance_log))
        .route("/maintenance-logs/{id}", put(update_maintenance_log))
        .route(
            "/equipment/{id}/maintenance-logs",
            get(list_equipment_maintenance_logs),
        )
        .route("/maintenance-logs/{id}/photos", post(add_maintenance_photo))
        .route(
            "/maintenance-logs/{id}/photos",
            get(list_maintenance_photos),
        )
        // Predictions (Story 134.3)
        .route("/predictions/run", post(run_prediction))
        .route("/predictions/batch", post(run_batch_predictions))
        .route(
            "/equipment/{id}/predictions",
            get(get_equipment_predictions),
        )
        // Alerts
        .route("/alerts", get(list_alerts))
        .route("/alerts/{id}/acknowledge", post(acknowledge_alert))
        .route("/alerts/{id}/resolve", post(resolve_alert))
        .route("/alerts/{id}/dismiss", post(dismiss_alert))
        // Health thresholds
        .route("/thresholds", get(list_health_thresholds))
        .route("/thresholds", post(set_health_threshold))
        // Dashboard (Story 134.4)
        .route("/dashboard", get(get_dashboard))
        .route("/equipment/by-health", get(get_equipment_by_health))
}

/// Build an internal-error response from a repository error.
fn db_error(e: impl std::fmt::Display) -> Response {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
}

// ============================================================================
// EQUIPMENT REGISTRY (Story 134.1)
// ============================================================================

/// Create new equipment.
async fn create_equipment(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateEquipment>,
) -> Response {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let resp = match s
        .predictive_maintenance_repo
        .create_equipment(&mut **rls.conn(), org_id, user_id, req)
        .await
    {
        Ok(equipment) => (StatusCode::CREATED, Json(equipment)).into_response(),
        Err(e) => db_error(e),
    };
    rls.release().await;
    resp
}

/// List equipment with filters.
async fn list_equipment(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<EquipmentQuery>,
) -> Response {
    let org_id = rls.tenant_id();
    let resp = match s
        .predictive_maintenance_repo
        .list_equipment(&mut **rls.conn(), org_id, query)
        .await
    {
        Ok(equipment) => Json(equipment).into_response(),
        Err(e) => db_error(e),
    };
    rls.release().await;
    resp
}

/// Get equipment by ID.
async fn get_equipment(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Response {
    let org_id = rls.tenant_id();
    let resp = match s
        .predictive_maintenance_repo
        .get_equipment(&mut **rls.conn(), id, org_id)
        .await
    {
        Ok(Some(equipment)) => Json(equipment).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => db_error(e),
    };
    rls.release().await;
    resp
}

/// Update equipment.
async fn update_equipment(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateEquipment>,
) -> Response {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let resp = match s
        .predictive_maintenance_repo
        .update_equipment(&mut **rls.conn(), id, org_id, user_id, req)
        .await
    {
        Ok(Some(equipment)) => Json(equipment).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => db_error(e),
    };
    rls.release().await;
    resp
}

/// Delete equipment.
async fn delete_equipment(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Response {
    let org_id = rls.tenant_id();
    let resp = match s
        .predictive_maintenance_repo
        .delete_equipment(&mut **rls.conn(), id, org_id)
        .await
    {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => db_error(e),
    };
    rls.release().await;
    resp
}

/// Add document to equipment.
async fn add_equipment_document(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateEquipmentDocument>,
) -> Response {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let resp = match s
        .predictive_maintenance_repo
        .add_equipment_document(&mut **rls.conn(), id, org_id, user_id, req)
        .await
    {
        Ok(doc) => (StatusCode::CREATED, Json(doc)).into_response(),
        Err(e) => db_error(e),
    };
    rls.release().await;
    resp
}

/// List equipment documents.
async fn list_equipment_documents(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Response {
    let org_id = rls.tenant_id();
    let resp = match s
        .predictive_maintenance_repo
        .list_equipment_documents(&mut **rls.conn(), id, org_id)
        .await
    {
        Ok(docs) => Json(docs).into_response(),
        Err(e) => db_error(e),
    };
    rls.release().await;
    resp
}

// ============================================================================
// MAINTENANCE LOGS (Story 134.2)
// ============================================================================

/// Create maintenance log.
async fn create_maintenance_log(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateMaintenanceLog>,
) -> Response {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let resp = match s
        .predictive_maintenance_repo
        .create_maintenance_log(&mut **rls.conn(), org_id, user_id, req)
        .await
    {
        Ok(log) => (StatusCode::CREATED, Json(log)).into_response(),
        Err(e) => db_error(e),
    };
    rls.release().await;
    resp
}

/// Get maintenance log by ID.
async fn get_maintenance_log(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Response {
    let org_id = rls.tenant_id();
    let resp = match s
        .predictive_maintenance_repo
        .get_maintenance_log(&mut **rls.conn(), id, org_id)
        .await
    {
        Ok(Some(log)) => Json(log).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => db_error(e),
    };
    rls.release().await;
    resp
}

/// Update maintenance log.
async fn update_maintenance_log(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateMaintenanceLog>,
) -> Response {
    let org_id = rls.tenant_id();
    let resp = match s
        .predictive_maintenance_repo
        .update_maintenance_log(&mut **rls.conn(), id, org_id, req)
        .await
    {
        Ok(Some(log)) => Json(log).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => db_error(e),
    };
    rls.release().await;
    resp
}

/// List maintenance logs for equipment.
#[derive(Debug, Deserialize)]
struct PaginationQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

async fn list_equipment_maintenance_logs(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(equipment_id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Response {
    let org_id = rls.tenant_id();
    let limit = clamp_limit(query.limit, 50);
    let offset = query.offset.unwrap_or(0);

    let resp = match s
        .predictive_maintenance_repo
        .list_maintenance_logs(&mut **rls.conn(), equipment_id, org_id, limit, offset)
        .await
    {
        Ok(logs) => Json(logs).into_response(),
        Err(e) => db_error(e),
    };
    rls.release().await;
    resp
}

/// Add photo to maintenance log.
#[derive(Debug, Deserialize, ToSchema)]
struct AddPhotoRequest {
    file_path: String,
    file_size: Option<i32>,
    mime_type: Option<String>,
    caption: Option<String>,
    photo_type: Option<String>,
}

async fn add_maintenance_photo(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(log_id): Path<Uuid>,
    Json(req): Json<AddPhotoRequest>,
) -> Response {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();

    // Only allow attaching photos to a maintenance log the caller's org owns.
    // Under RLS a foreign-org log_id is invisible and surfaces as 404.
    match s
        .predictive_maintenance_repo
        .get_maintenance_log(&mut **rls.conn(), log_id, org_id)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            rls.release().await;
            return StatusCode::NOT_FOUND.into_response();
        }
        Err(e) => {
            rls.release().await;
            return db_error(e);
        }
    }

    let resp = match s
        .predictive_maintenance_repo
        .add_maintenance_photo(
            &mut **rls.conn(),
            log_id,
            user_id,
            &req.file_path,
            req.file_size,
            req.mime_type.as_deref(),
            req.caption.as_deref(),
            req.photo_type.as_deref(),
        )
        .await
    {
        Ok(photo) => (StatusCode::CREATED, Json(photo)).into_response(),
        Err(e) => db_error(e),
    };
    rls.release().await;
    resp
}

/// List photos for maintenance log.
async fn list_maintenance_photos(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(log_id): Path<Uuid>,
) -> Response {
    let org_id = rls.tenant_id();

    // Confirm the parent maintenance log belongs to the caller's org before
    // returning any photos. A foreign-org log_id surfaces as 404, never a
    // cross-tenant read (IDOR #848).
    match s
        .predictive_maintenance_repo
        .get_maintenance_log(&mut **rls.conn(), log_id, org_id)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            rls.release().await;
            return StatusCode::NOT_FOUND.into_response();
        }
        Err(e) => {
            rls.release().await;
            return db_error(e);
        }
    }

    let resp = match s
        .predictive_maintenance_repo
        .list_maintenance_photos(&mut **rls.conn(), log_id, org_id)
        .await
    {
        Ok(photos) => Json(photos).into_response(),
        Err(e) => db_error(e),
    };
    rls.release().await;
    resp
}

// ============================================================================
// PREDICTIONS (Story 134.3)
// ============================================================================

/// Run prediction for equipment.
async fn run_prediction(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<RunPredictionRequest>,
) -> Response {
    let org_id = rls.tenant_id();

    // Run prediction for specified equipment or all
    let equipment_ids = if let Some(ids) = req.equipment_ids {
        ids
    } else {
        // Get all equipment IDs for the building or org
        let query = db::models::predictive_maintenance::EquipmentQuery {
            building_id: req.building_id,
            ..Default::default()
        };
        match s
            .predictive_maintenance_repo
            .list_equipment(&mut **rls.conn(), org_id, query)
            .await
        {
            Ok(equipment) => equipment.into_iter().map(|e| e.id).collect(),
            Err(e) => {
                rls.release().await;
                return db_error(e);
            }
        }
    };

    // Run predictions for each equipment
    let mut results = Vec::new();
    for equipment_id in equipment_ids {
        match s
            .predictive_maintenance_repo
            .run_prediction(rls.conn(), equipment_id, org_id)
            .await
        {
            Ok(result) => results.push(result),
            Err(e) => {
                tracing::warn!("Failed to run prediction for {}: {}", equipment_id, e);
            }
        }
    }

    rls.release().await;
    Json(results).into_response()
}

/// Run batch predictions.
async fn run_batch_predictions(
    State(s): State<AppState>,
    rls: RlsConnection,
    Json(req): Json<RunPredictionRequest>,
) -> Response {
    run_prediction(State(s), rls, Json(req)).await
}

/// Get prediction history for equipment.
#[derive(Debug, Deserialize)]
struct PredictionHistoryQuery {
    limit: Option<i64>,
}

async fn get_equipment_predictions(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(equipment_id): Path<Uuid>,
    Query(query): Query<PredictionHistoryQuery>,
) -> Response {
    let org_id = rls.tenant_id();
    let limit = clamp_limit(query.limit, 20);

    let resp = match s
        .predictive_maintenance_repo
        .get_prediction_history(&mut **rls.conn(), equipment_id, org_id, limit)
        .await
    {
        Ok(predictions) => Json(predictions).into_response(),
        Err(e) => db_error(e),
    };
    rls.release().await;
    resp
}

// ============================================================================
// ALERTS
// ============================================================================

/// List alerts.
#[derive(Debug, Deserialize)]
struct AlertQuery {
    status: Option<String>,
    severity: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

async fn list_alerts(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<AlertQuery>,
) -> Response {
    let org_id = rls.tenant_id();
    let limit = clamp_limit(query.limit, 50);
    let offset = query.offset.unwrap_or(0);

    let resp = match s
        .predictive_maintenance_repo
        .list_alerts(
            &mut **rls.conn(),
            org_id,
            query.status.as_deref(),
            query.severity.as_deref(),
            limit,
            offset,
        )
        .await
    {
        Ok(alerts) => Json(alerts).into_response(),
        Err(e) => db_error(e),
    };
    rls.release().await;
    resp
}

/// Acknowledge alert.
async fn acknowledge_alert(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(_req): Json<AcknowledgeAlertRequest>,
) -> Response {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let resp = match s
        .predictive_maintenance_repo
        .acknowledge_alert(&mut **rls.conn(), id, org_id, user_id)
        .await
    {
        Ok(Some(alert)) => Json(alert).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => db_error(e),
    };
    rls.release().await;
    resp
}

/// Resolve alert.
async fn resolve_alert(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<ResolveAlertRequest>,
) -> Response {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let resp = match s
        .predictive_maintenance_repo
        .resolve_alert(
            &mut **rls.conn(),
            id,
            org_id,
            user_id,
            req.maintenance_log_id,
        )
        .await
    {
        Ok(Some(alert)) => Json(alert).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => db_error(e),
    };
    rls.release().await;
    resp
}

/// Dismiss alert.
async fn dismiss_alert(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Response {
    let org_id = rls.tenant_id();
    let resp = match s
        .predictive_maintenance_repo
        .dismiss_alert(&mut **rls.conn(), id, org_id)
        .await
    {
        Ok(Some(alert)) => Json(alert).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => db_error(e),
    };
    rls.release().await;
    resp
}

// ============================================================================
// HEALTH THRESHOLDS
// ============================================================================

/// List health thresholds.
async fn list_health_thresholds(State(s): State<AppState>, mut rls: RlsConnection) -> Response {
    let org_id = rls.tenant_id();
    let resp = match s
        .predictive_maintenance_repo
        .list_health_thresholds(&mut **rls.conn(), org_id)
        .await
    {
        Ok(thresholds) => Json(thresholds).into_response(),
        Err(e) => db_error(e),
    };
    rls.release().await;
    resp
}

/// Set health threshold.
async fn set_health_threshold(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<SetHealthThreshold>,
) -> Response {
    let org_id = rls.tenant_id();
    let resp = match s
        .predictive_maintenance_repo
        .set_health_threshold(&mut **rls.conn(), org_id, req)
        .await
    {
        Ok(threshold) => Json(threshold).into_response(),
        Err(e) => db_error(e),
    };
    rls.release().await;
    resp
}

// ============================================================================
// DASHBOARD (Story 134.4)
// ============================================================================

/// Dashboard query parameters.
#[derive(Debug, Deserialize)]
struct DashboardQuery {
    building_id: Option<Uuid>,
}

/// Get maintenance dashboard.
async fn get_dashboard(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<DashboardQuery>,
) -> Response {
    let org_id = rls.tenant_id();
    let resp = match s
        .predictive_maintenance_repo
        .get_dashboard(rls.conn(), org_id, query.building_id)
        .await
    {
        Ok(dashboard) => Json(dashboard).into_response(),
        Err(e) => db_error(e),
    };
    rls.release().await;
    resp
}

/// Get equipment sorted by health score.
#[derive(Debug, Deserialize)]
struct ByHealthQuery {
    building_id: Option<Uuid>,
    limit: Option<i64>,
}

async fn get_equipment_by_health(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ByHealthQuery>,
) -> Response {
    let org_id = rls.tenant_id();
    let limit = clamp_limit(query.limit, 20);

    let resp = match s
        .predictive_maintenance_repo
        .get_equipment_by_health(&mut **rls.conn(), org_id, query.building_id, limit)
        .await
    {
        Ok(equipment) => Json(equipment).into_response(),
        Err(e) => db_error(e),
    };
    rls.release().await;
    resp
}
