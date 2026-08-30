//! Equipment & predictive maintenance (Story 13.3).
//!
//! # RLS (PAP-67 / PAP-71)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` on `equipment`,
//! `equipment_maintenance`, and `maintenance_predictions`, so every query MUST
//! run on a connection that has `app.current_org_id` set or it collapses to
//! deny-all. Each handler therefore acquires an [`RlsConnection`] (which
//! validates tenant membership and sets the org/user GUCs on a dedicated
//! connection) and passes `&mut **rls.conn()` to the repository. The
//! authoritative organization is `rls.tenant_id()` — the tenant the caller was
//! validated against — not a client-supplied value, so the SQL org filter and
//! the RLS context can never disagree. Cross-tenant access is blocked by RLS: a
//! by-id read of another org's row returns no row (`404`), and a write targeting
//! another org fails the policy's `WITH CHECK`. `rls.release()` clears the
//! context before the connection returns to the pool.

use crate::routes::ai::PaginationQuery;
use crate::routes::pagination::clamp_limit;
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
    CreateEquipment, CreateMaintenance, EquipmentQuery, UpdateEquipment, UpdateMaintenance,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// Equipment Router (Story 13.3)
// ============================================================================

pub fn equipment_router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_equipment))
        .route("/", get(list_equipment))
        .route("/{id}", get(get_equipment))
        .route("/{id}", put(update_equipment))
        .route("/{id}", delete(delete_equipment))
        .route("/{id}/maintenance", get(list_maintenance))
        .route("/{id}/maintenance", post(create_maintenance))
        .route("/maintenance/{id}", put(update_maintenance))
        .route("/predictions", get(list_predictions))
        .route(
            "/predictions/{id}/acknowledge",
            post(acknowledge_prediction),
        )
        .route("/needing-maintenance", get(list_needing_maintenance))
}

async fn create_equipment(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateEquipment>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    // SECURITY: owning org is the validated RLS tenant, never the body.
    let organization_id = rls.tenant_id();
    let out = match state
        .equipment_repo
        .create(&mut **rls.conn(), organization_id, req)
        .await
    {
        Ok(equipment) => Ok((StatusCode::CREATED, Json(serde_json::json!(equipment)))),
        Err(e) => {
            tracing::error!("Failed to create equipment: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to create")),
            ))
        }
    };
    rls.release().await;
    out
}

async fn list_equipment(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<EquipmentQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = rls.tenant_id();
    let out = match state
        .equipment_repo
        .list(&mut **rls.conn(), tenant_id, query)
        .await
    {
        Ok(equipment) => Ok(Json(serde_json::json!({ "equipment": equipment }))),
        Err(e) => {
            tracing::error!("Failed to list equipment: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to list")),
            ))
        }
    };
    rls.release().await;
    out
}

async fn get_equipment(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = rls.tenant_id();
    let out = match state
        .equipment_repo
        .find_by_id(&mut **rls.conn(), id, tenant_id)
        .await
    {
        Ok(Some(equipment)) => Ok(Json(serde_json::json!(equipment))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Equipment not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get equipment: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get")),
            ))
        }
    };
    rls.release().await;
    out
}

async fn update_equipment(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateEquipment>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = rls.tenant_id();
    let out = match state
        .equipment_repo
        .update(&mut **rls.conn(), id, tenant_id, req)
        .await
    {
        Ok(equipment) => Ok(Json(serde_json::json!(equipment))),
        Err(sqlx::Error::RowNotFound) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Equipment not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to update equipment: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to update")),
            ))
        }
    };
    rls.release().await;
    out
}

async fn delete_equipment(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = rls.tenant_id();
    let out = match state
        .equipment_repo
        .delete(&mut **rls.conn(), id, tenant_id)
        .await
    {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Equipment not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to delete equipment: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to delete")),
            ))
        }
    };
    rls.release().await;
    out
}

async fn list_maintenance(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = rls.tenant_id();
    let out = match state
        .equipment_repo
        .list_maintenance(
            &mut **rls.conn(),
            id,
            tenant_id,
            clamp_limit(query.limit, 50),
            query.offset.unwrap_or(0),
        )
        .await
    {
        Ok(records) => Ok(Json(serde_json::json!({ "maintenance": records }))),
        Err(e) => {
            tracing::error!("Failed to list maintenance: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to list")),
            ))
        }
    };
    rls.release().await;
    out
}

async fn create_maintenance(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(_id): Path<Uuid>,
    Json(req): Json<CreateMaintenance>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    // SECURITY: org is the validated RLS tenant. The repository INSERT is guarded
    // by a sub-select that ensures req.equipment_id belongs to this tenant; a
    // foreign equipment_id yields RowNotFound → 404.
    let tenant_id = rls.tenant_id();
    let out = match state
        .equipment_repo
        .create_maintenance(&mut **rls.conn(), tenant_id, req)
        .await
    {
        Ok(record) => Ok((StatusCode::CREATED, Json(serde_json::json!(record)))),
        Err(sqlx::Error::RowNotFound) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Equipment not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to create maintenance: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to create")),
            ))
        }
    };
    rls.release().await;
    out
}

async fn update_maintenance(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateMaintenance>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = rls.tenant_id();
    let out = match state
        .equipment_repo
        .update_maintenance(rls.conn(), id, tenant_id, req)
        .await
    {
        Ok(record) => Ok(Json(serde_json::json!(record))),
        Err(sqlx::Error::RowNotFound) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Maintenance record not found",
            )),
        )),
        Err(e) => {
            tracing::error!("Failed to update maintenance: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to update")),
            ))
        }
    };
    rls.release().await;
    out
}

async fn list_predictions(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = rls.tenant_id();
    let out = match state
        .equipment_repo
        .list_high_risk_predictions(
            &mut **rls.conn(),
            tenant_id,
            50.0,
            clamp_limit(query.limit, 20),
        )
        .await
    {
        Ok(predictions) => Ok(Json(serde_json::json!({ "predictions": predictions }))),
        Err(e) => {
            tracing::error!("Failed to list predictions: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to list")),
            ))
        }
    };
    rls.release().await;
    out
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AcknowledgePredictionRequest {
    pub action_taken: Option<String>,
}

async fn acknowledge_prediction(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<AcknowledgePredictionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // SECURITY: org + user come from the validated RLS connection so a caller in
    // org B cannot acknowledge predictions belonging to org A. We return 404 for
    // both "not found" and "wrong tenant" to prevent cross-tenant ID enumeration.
    let tenant_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = match state
        .equipment_repo
        .acknowledge_prediction(
            &mut **rls.conn(),
            id,
            tenant_id,
            user_id,
            req.action_taken.as_deref(),
        )
        .await
    {
        Ok(prediction) => Ok(Json(serde_json::json!(prediction))),
        Err(sqlx::Error::RowNotFound) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Prediction not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to acknowledge: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to acknowledge",
                )),
            ))
        }
    };
    rls.release().await;
    out
}

#[derive(Debug, Serialize, Deserialize, Default, utoipa::IntoParams)]
pub struct MaintenanceDueQuery {
    pub days_ahead: Option<i32>,
    pub limit: Option<i64>,
}

async fn list_needing_maintenance(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<MaintenanceDueQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = rls.tenant_id();
    let out = match state
        .equipment_repo
        .list_needing_maintenance(
            &mut **rls.conn(),
            tenant_id,
            query.days_ahead.unwrap_or(30),
            clamp_limit(query.limit, 20),
        )
        .await
    {
        Ok(equipment) => Ok(Json(serde_json::json!({ "equipment": equipment }))),
        Err(e) => {
            tracing::error!("Failed to list: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to list")),
            ))
        }
    };
    rls.release().await;
    out
}
