//! Emergency drill route surface — CRUD plus lifecycle (start / complete /
//! cancel) and the upcoming-drills listing.

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::ErrorResponse;
use db::models::{CompleteDrill, EmergencyDrillQuery};
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use super::shared::{CreateDrillRequest, DrillListQuery, OrgQuery, UpdateDrillRequest};
use crate::state::AppState;

/// Drill sub-router.
pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/drills", post(create_drill))
        .route("/drills", get(list_drills))
        .route("/drills/upcoming", get(get_upcoming_drills))
        .route("/drills/{id}", get(get_drill))
        .route("/drills/{id}", put(update_drill))
        .route("/drills/{id}/start", post(start_drill))
        .route("/drills/{id}/complete", post(complete_drill))
        .route("/drills/{id}/cancel", post(cancel_drill))
        .route("/drills/{id}", delete(delete_drill))
}

async fn create_drill(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateDrillRequest>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let user = rls.user_id();
    let result = state
        .emergency_repo
        .create_drill(&mut **rls.conn(), org, user, req.data)
        .await;
    rls.release().await;
    match result {
        Ok(drill) => (StatusCode::CREATED, Json(drill)).into_response(),
        Err(e) => {
            tracing::error!("Failed to create drill: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn list_drills(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<DrillListQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .list_drills(&mut **rls.conn(), org, EmergencyDrillQuery::from(&query))
        .await;
    rls.release().await;
    match result {
        Ok(drills) => Json(drills).into_response(),
        Err(e) => {
            tracing::error!("Failed to list drills: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
struct UpcomingDrillsQuery {
    days: Option<i32>,
}

async fn get_upcoming_drills(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<UpcomingDrillsQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let days = query.days.unwrap_or(30);
    let result = state
        .emergency_repo
        .get_upcoming_drills(&mut **rls.conn(), org, days)
        .await;
    rls.release().await;
    match result {
        Ok(drills) => Json(drills).into_response(),
        Err(e) => {
            tracing::error!("Failed to get upcoming drills: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn get_drill(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(_): Query<OrgQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .find_drill_by_id(&mut **rls.conn(), org, id)
        .await;
    rls.release().await;
    match result {
        Ok(Some(drill)) => Json(drill).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Drill not found")),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to get drill: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn update_drill(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateDrillRequest>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .update_drill(&mut **rls.conn(), org, id, req.data)
        .await;
    rls.release().await;
    match result {
        Ok(Some(drill)) => Json(drill).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Drill not found")),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to update drill: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn start_drill(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(_): Query<OrgQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .start_drill(&mut **rls.conn(), org, id)
        .await;
    rls.release().await;
    match result {
        Ok(Some(drill)) => Json(drill).into_response(),
        Ok(None) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_STATE",
                "Drill cannot be started",
            )),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to start drill: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

/// Complete drill request wrapper.
#[derive(Debug, Deserialize)]
pub struct CompleteDrillRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CompleteDrill,
}

async fn complete_drill(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<CompleteDrillRequest>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .complete_drill(&mut **rls.conn(), org, id, req.data)
        .await;
    rls.release().await;
    match result {
        Ok(Some(drill)) => Json(drill).into_response(),
        Ok(None) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_STATE",
                "Drill cannot be completed",
            )),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to complete drill: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn cancel_drill(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(_): Query<OrgQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .cancel_drill(&mut **rls.conn(), org, id)
        .await;
    rls.release().await;
    match result {
        Ok(Some(drill)) => Json(drill).into_response(),
        Ok(None) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_STATE",
                "Drill cannot be cancelled",
            )),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to cancel drill: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn delete_drill(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(_): Query<OrgQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .delete_drill(&mut **rls.conn(), org, id)
        .await;
    rls.release().await;
    match result {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_STATE",
                "Only scheduled drills can be deleted",
            )),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to delete drill: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}
