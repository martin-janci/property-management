//! Emergency broadcast route surface — create, list, get, deactivate,
//! acknowledge, and acknowledgment listing.

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use common::ErrorResponse;
use db::models::{AcknowledgeBroadcast, EmergencyBroadcastQuery};
use uuid::Uuid;

use super::shared::{is_emergency_manager, BroadcastListQuery, CreateBroadcastRequest, OrgQuery};
use crate::state::AppState;

/// Broadcast sub-router.
pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/broadcasts", post(create_broadcast))
        .route("/broadcasts", get(list_broadcasts))
        .route("/broadcasts/{id}", get(get_broadcast))
        .route("/broadcasts/{id}/deactivate", post(deactivate_broadcast))
        .route("/broadcasts/{id}/acknowledge", post(acknowledge_broadcast))
        .route(
            "/broadcasts/{id}/acknowledgments",
            get(list_broadcast_acknowledgments),
        )
}

async fn create_broadcast(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateBroadcastRequest>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let user = rls.user_id();
    if !is_emergency_manager(rls.role()) {
        rls.release().await;
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Manager role required for this action",
            )),
        )
            .into_response();
    }
    let result = state
        .emergency_repo
        .create_broadcast(&mut **rls.conn(), org, user, req.data)
        .await;
    rls.release().await;
    match result {
        Ok(broadcast) => (StatusCode::CREATED, Json(broadcast)).into_response(),
        Err(e) => {
            tracing::error!("Failed to create broadcast: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn list_broadcasts(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<BroadcastListQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .list_broadcasts(
            &mut **rls.conn(),
            org,
            EmergencyBroadcastQuery::from(&query),
        )
        .await;
    rls.release().await;
    match result {
        Ok(broadcasts) => Json(broadcasts).into_response(),
        Err(e) => {
            tracing::error!("Failed to list broadcasts: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn get_broadcast(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(_): Query<OrgQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .find_broadcast_by_id(&mut **rls.conn(), org, id)
        .await;
    rls.release().await;
    match result {
        Ok(Some(broadcast)) => Json(broadcast).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Broadcast not found")),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to get broadcast: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn deactivate_broadcast(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(_): Query<OrgQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    if !is_emergency_manager(rls.role()) {
        rls.release().await;
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Manager role required for this action",
            )),
        )
            .into_response();
    }
    let result = state
        .emergency_repo
        .deactivate_broadcast(&mut **rls.conn(), org, id)
        .await;
    rls.release().await;
    match result {
        Ok(true) => StatusCode::OK.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Broadcast not found")),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to deactivate broadcast: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn acknowledge_broadcast(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(_): Query<OrgQuery>,
    Json(data): Json<AcknowledgeBroadcast>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let user = rls.user_id();
    // The acknowledgments table is RLS-scoped via the parent broadcast but keys
    // only on `broadcast_id`; confirm the broadcast is in the caller's org so a
    // cross-tenant / unknown id yields 404 rather than a policy-violation 500.
    match state
        .emergency_repo
        .find_broadcast_by_id(&mut **rls.conn(), org, id)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            rls.release().await;
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Broadcast not found")),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Failed to load broadcast: {:?}", e);
            rls.release().await;
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response();
        }
    }
    let result = state
        .emergency_repo
        .acknowledge_broadcast(&mut **rls.conn(), id, user, data)
        .await;
    rls.release().await;
    match result {
        Ok(ack) => (StatusCode::CREATED, Json(ack)).into_response(),
        Err(e) => {
            tracing::error!("Failed to acknowledge broadcast: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn list_broadcast_acknowledgments(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(_): Query<OrgQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    match state
        .emergency_repo
        .find_broadcast_by_id(&mut **rls.conn(), org, id)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            rls.release().await;
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Broadcast not found")),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Failed to load broadcast: {:?}", e);
            rls.release().await;
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response();
        }
    }
    let result = state
        .emergency_repo
        .list_broadcast_acknowledgments(&mut **rls.conn(), id)
        .await;
    rls.release().await;
    match result {
        Ok(acks) => Json(acks).into_response(),
        Err(e) => {
            tracing::error!("Failed to list broadcast acknowledgments: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}
