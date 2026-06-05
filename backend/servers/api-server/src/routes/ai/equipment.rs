//! Equipment & predictive maintenance (Story 13.3).

use crate::routes::ai::{require_tenant_id, PaginationQuery};
use crate::state::AppState;
use api_core::extractors::principal::RequestPrincipal;
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
    principal: RequestPrincipal,
    Json(req): Json<CreateEquipment>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    // SECURITY: owning org is derived from the verified principal, never the body.
    let organization_id = require_tenant_id(&principal)?;
    match state.equipment_repo.create(organization_id, req).await {
        Ok(equipment) => Ok((StatusCode::CREATED, Json(serde_json::json!(equipment)))),
        Err(e) => {
            tracing::error!("Failed to create equipment: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to create")),
            ))
        }
    }
}

async fn list_equipment(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Query(query): Query<EquipmentQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = require_tenant_id(&principal)?;

    match state.equipment_repo.list(tenant_id, query).await {
        Ok(equipment) => Ok(Json(serde_json::json!({ "equipment": equipment }))),
        Err(e) => {
            tracing::error!("Failed to list equipment: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to list")),
            ))
        }
    }
}

async fn get_equipment(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // SECURITY: derive the tenant from the verified JWT — never trust client input.
    let tenant_id = require_tenant_id(&principal)?;
    match state.equipment_repo.find_by_id(id, tenant_id).await {
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
    }
}

async fn update_equipment(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateEquipment>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // SECURITY: derive the tenant from the verified JWT — never trust client input.
    let tenant_id = require_tenant_id(&principal)?;
    match state.equipment_repo.update(id, tenant_id, req).await {
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
    }
}

async fn delete_equipment(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // SECURITY: derive the tenant from the verified JWT — never trust client input.
    let tenant_id = require_tenant_id(&principal)?;
    match state.equipment_repo.delete(id, tenant_id).await {
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
    }
}

async fn list_maintenance(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // SECURITY: derive the tenant from the verified JWT — never trust client input.
    let tenant_id = require_tenant_id(&principal)?;
    match state
        .equipment_repo
        .list_maintenance(
            id,
            tenant_id,
            query.limit.unwrap_or(50),
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
    }
}

async fn create_maintenance(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(_id): Path<Uuid>,
    Json(req): Json<CreateMaintenance>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    // SECURITY: derive the tenant from the verified JWT — never trust client input.
    // The repository INSERT is guarded by a sub-select that ensures req.equipment_id
    // belongs to this tenant; a foreign equipment_id yields RowNotFound → 404.
    let tenant_id = require_tenant_id(&principal)?;
    match state
        .equipment_repo
        .create_maintenance(tenant_id, req)
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
    }
}

async fn update_maintenance(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateMaintenance>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // SECURITY: derive the tenant from the verified JWT — never trust client input.
    let tenant_id = require_tenant_id(&principal)?;
    match state
        .equipment_repo
        .update_maintenance(id, tenant_id, req)
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
    }
}

async fn list_predictions(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = require_tenant_id(&principal)?;

    match state
        .equipment_repo
        .list_high_risk_predictions(tenant_id, 50.0, query.limit.unwrap_or(20))
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
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AcknowledgePredictionRequest {
    pub action_taken: Option<String>,
}

async fn acknowledge_prediction(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
    Json(req): Json<AcknowledgePredictionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // SECURITY: tenant_id is derived from the verified JWT and passed to the
    // repository so a caller in org B cannot acknowledge predictions belonging
    // to org A. We return 404 for both "not found" and "wrong tenant" to
    // prevent cross-tenant ID enumeration.
    let tenant_id = require_tenant_id(&principal)?;

    match state
        .equipment_repo
        .acknowledge_prediction(
            id,
            tenant_id,
            principal.user_id,
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
    }
}

#[derive(Debug, Serialize, Deserialize, Default, utoipa::IntoParams)]
pub struct MaintenanceDueQuery {
    pub days_ahead: Option<i32>,
    pub limit: Option<i64>,
}

async fn list_needing_maintenance(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Query(query): Query<MaintenanceDueQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = require_tenant_id(&principal)?;

    match state
        .equipment_repo
        .list_needing_maintenance(
            tenant_id,
            query.days_ahead.unwrap_or(30),
            query.limit.unwrap_or(20),
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
    }
}
