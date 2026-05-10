//! Routes for Epic 134: Predictive Maintenance & Equipment Intelligence.
//!
//! - Story 134.1: Equipment Registry
//! - Story 134.2: Maintenance History Tracking
//! - Story 134.3: Failure Prediction Engine
//! - Story 134.4: Predictive Maintenance Dashboard

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use api_core::extractors::AuthUser;

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

// Helper to get org_id from AuthUser
fn get_org_id(auth: &AuthUser) -> Result<Uuid, (StatusCode, String)> {
    auth.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "No organization context".to_string(),
    ))
}

// ============================================================================
// EQUIPMENT REGISTRY (Story 134.1)
// ============================================================================

/// Create new equipment.
async fn create_equipment(
    State(s): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateEquipment>,
) -> impl IntoResponse {
    let org_id = match get_org_id(&auth) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    match s
        .predictive_maintenance_repo
        .create_equipment(org_id, auth.user_id, req)
        .await
    {
        Ok(equipment) => (StatusCode::CREATED, Json(equipment)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// List equipment with filters.
async fn list_equipment(
    State(s): State<AppState>,
    auth: AuthUser,
    Query(query): Query<EquipmentQuery>,
) -> impl IntoResponse {
    let org_id = match get_org_id(&auth) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    match s
        .predictive_maintenance_repo
        .list_equipment(org_id, query)
        .await
    {
        Ok(equipment) => Json(equipment).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Get equipment by ID.
async fn get_equipment(
    State(s): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let org_id = match get_org_id(&auth) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    match s
        .predictive_maintenance_repo
        .get_equipment(id, org_id)
        .await
    {
        Ok(Some(equipment)) => Json(equipment).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Update equipment.
async fn update_equipment(
    State(s): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateEquipment>,
) -> impl IntoResponse {
    let org_id = match get_org_id(&auth) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    match s
        .predictive_maintenance_repo
        .update_equipment(id, org_id, auth.user_id, req)
        .await
    {
        Ok(Some(equipment)) => Json(equipment).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Delete equipment.
async fn delete_equipment(
    State(s): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let org_id = match get_org_id(&auth) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    match s
        .predictive_maintenance_repo
        .delete_equipment(id, org_id)
        .await
    {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Add document to equipment.
async fn add_equipment_document(
    State(s): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateEquipmentDocument>,
) -> impl IntoResponse {
    let org_id = match get_org_id(&auth) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    match s
        .predictive_maintenance_repo
        .add_equipment_document(id, org_id, auth.user_id, req)
        .await
    {
        Ok(doc) => (StatusCode::CREATED, Json(doc)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// List equipment documents.
async fn list_equipment_documents(
    State(s): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let org_id = match get_org_id(&auth) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    match s
        .predictive_maintenance_repo
        .list_equipment_documents(id, org_id)
        .await
    {
        Ok(docs) => Json(docs).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ============================================================================
// MAINTENANCE LOGS (Story 134.2)
// ============================================================================

/// Create maintenance log.
async fn create_maintenance_log(
    State(s): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateMaintenanceLog>,
) -> impl IntoResponse {
    let org_id = match get_org_id(&auth) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    match s
        .predictive_maintenance_repo
        .create_maintenance_log(org_id, auth.user_id, req)
        .await
    {
        Ok(log) => (StatusCode::CREATED, Json(log)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Get maintenance log by ID.
async fn get_maintenance_log(
    State(s): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let org_id = match get_org_id(&auth) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    match s
        .predictive_maintenance_repo
        .get_maintenance_log(id, org_id)
        .await
    {
        Ok(Some(log)) => Json(log).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Update maintenance log.
async fn update_maintenance_log(
    State(s): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateMaintenanceLog>,
) -> impl IntoResponse {
    let org_id = match get_org_id(&auth) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    match s
        .predictive_maintenance_repo
        .update_maintenance_log(id, org_id, req)
        .await
    {
        Ok(Some(log)) => Json(log).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// List maintenance logs for equipment.
#[derive(Debug, Deserialize)]
struct PaginationQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

async fn list_equipment_maintenance_logs(
    State(s): State<AppState>,
    auth: AuthUser,
    Path(equipment_id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> impl IntoResponse {
    let org_id = match get_org_id(&auth) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    match s
        .predictive_maintenance_repo
        .list_maintenance_logs(equipment_id, org_id, limit, offset)
        .await
    {
        Ok(logs) => Json(logs).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
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
    auth: AuthUser,
    Path(log_id): Path<Uuid>,
    Json(req): Json<AddPhotoRequest>,
) -> impl IntoResponse {
    match s
        .predictive_maintenance_repo
        .add_maintenance_photo(
            log_id,
            auth.user_id,
            &req.file_path,
            req.file_size,
            req.mime_type.as_deref(),
            req.caption.as_deref(),
            req.photo_type.as_deref(),
        )
        .await
    {
        Ok(photo) => (StatusCode::CREATED, Json(photo)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// List photos for maintenance log.
async fn list_maintenance_photos(
    State(s): State<AppState>,
    _auth: AuthUser,
    Path(log_id): Path<Uuid>,
) -> impl IntoResponse {
    match s
        .predictive_maintenance_repo
        .list_maintenance_photos(log_id)
        .await
    {
        Ok(photos) => Json(photos).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ============================================================================
// PREDICTIONS (Story 134.3)
// ============================================================================

/// Run prediction for equipment.
async fn run_prediction(
    State(s): State<AppState>,
    auth: AuthUser,
    Json(req): Json<RunPredictionRequest>,
) -> impl IntoResponse {
    let org_id = match get_org_id(&auth) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

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
            .list_equipment(org_id, query)
            .await
        {
            Ok(equipment) => equipment.into_iter().map(|e| e.id).collect(),
            Err(e) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
            }
        }
    };

    // Run predictions for each equipment
    let mut results = Vec::new();
    for equipment_id in equipment_ids {
        match s
            .predictive_maintenance_repo
            .run_prediction(equipment_id, org_id)
            .await
        {
            Ok(result) => results.push(result),
            Err(e) => {
                tracing::warn!("Failed to run prediction for {}: {}", equipment_id, e);
            }
        }
    }

    Json(results).into_response()
}

/// Run batch predictions.
async fn run_batch_predictions(
    State(s): State<AppState>,
    auth: AuthUser,
    Json(req): Json<RunPredictionRequest>,
) -> impl IntoResponse {
    run_prediction(State(s), auth, Json(req)).await
}

/// Get prediction history for equipment.
#[derive(Debug, Deserialize)]
struct PredictionHistoryQuery {
    limit: Option<i64>,
}

async fn get_equipment_predictions(
    State(s): State<AppState>,
    auth: AuthUser,
    Path(equipment_id): Path<Uuid>,
    Query(query): Query<PredictionHistoryQuery>,
) -> impl IntoResponse {
    let org_id = match get_org_id(&auth) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    let limit = query.limit.unwrap_or(20);

    match s
        .predictive_maintenance_repo
        .get_prediction_history(equipment_id, org_id, limit)
        .await
    {
        Ok(predictions) => Json(predictions).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
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
    auth: AuthUser,
    Query(query): Query<AlertQuery>,
) -> impl IntoResponse {
    let org_id = match get_org_id(&auth) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    match s
        .predictive_maintenance_repo
        .list_alerts(
            org_id,
            query.status.as_deref(),
            query.severity.as_deref(),
            limit,
            offset,
        )
        .await
    {
        Ok(alerts) => Json(alerts).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Acknowledge alert.
async fn acknowledge_alert(
    State(s): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(_req): Json<AcknowledgeAlertRequest>,
) -> impl IntoResponse {
    let org_id = match get_org_id(&auth) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    match s
        .predictive_maintenance_repo
        .acknowledge_alert(id, org_id, auth.user_id)
        .await
    {
        Ok(Some(alert)) => Json(alert).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Resolve alert.
async fn resolve_alert(
    State(s): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ResolveAlertRequest>,
) -> impl IntoResponse {
    let org_id = match get_org_id(&auth) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    match s
        .predictive_maintenance_repo
        .resolve_alert(id, org_id, auth.user_id, req.maintenance_log_id)
        .await
    {
        Ok(Some(alert)) => Json(alert).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Dismiss alert.
async fn dismiss_alert(
    State(s): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let org_id = match get_org_id(&auth) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    match s
        .predictive_maintenance_repo
        .dismiss_alert(id, org_id)
        .await
    {
        Ok(Some(alert)) => Json(alert).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ============================================================================
// HEALTH THRESHOLDS
// ============================================================================

/// List health thresholds.
async fn list_health_thresholds(State(s): State<AppState>, auth: AuthUser) -> impl IntoResponse {
    let org_id = match get_org_id(&auth) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    match s
        .predictive_maintenance_repo
        .list_health_thresholds(org_id)
        .await
    {
        Ok(thresholds) => Json(thresholds).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Set health threshold.
async fn set_health_threshold(
    State(s): State<AppState>,
    auth: AuthUser,
    Json(req): Json<SetHealthThreshold>,
) -> impl IntoResponse {
    let org_id = match get_org_id(&auth) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    match s
        .predictive_maintenance_repo
        .set_health_threshold(org_id, req)
        .await
    {
        Ok(threshold) => Json(threshold).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
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
    auth: AuthUser,
    Query(query): Query<DashboardQuery>,
) -> impl IntoResponse {
    let org_id = match get_org_id(&auth) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    match s
        .predictive_maintenance_repo
        .get_dashboard(org_id, query.building_id)
        .await
    {
        Ok(dashboard) => Json(dashboard).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Get equipment sorted by health score.
#[derive(Debug, Deserialize)]
struct ByHealthQuery {
    building_id: Option<Uuid>,
    limit: Option<i64>,
}

async fn get_equipment_by_health(
    State(s): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ByHealthQuery>,
) -> impl IntoResponse {
    let org_id = match get_org_id(&auth) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    let limit = query.limit.unwrap_or(20);

    match s
        .predictive_maintenance_repo
        .get_equipment_by_health(org_id, query.building_id, limit)
        .await
    {
        Ok(equipment) => Json(equipment).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
