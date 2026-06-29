//! Emergency protocol route surface (CRUD).

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::ErrorResponse;
use db::models::EmergencyProtocolQuery;
use uuid::Uuid;

use super::shared::{CreateProtocolRequest, OrgQuery, ProtocolListQuery, UpdateProtocolRequest};
use crate::state::AppState;

/// Protocol sub-router.
pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/protocols", post(create_protocol))
        .route("/protocols", get(list_protocols))
        .route("/protocols/{id}", get(get_protocol))
        .route("/protocols/{id}", put(update_protocol))
        .route("/protocols/{id}", delete(delete_protocol))
}

async fn create_protocol(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateProtocolRequest>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let user = rls.user_id();
    let result = state
        .emergency_repo
        .create_protocol(&mut **rls.conn(), org, user, req.data)
        .await;
    rls.release().await;
    match result {
        Ok(protocol) => (StatusCode::CREATED, Json(protocol)).into_response(),
        Err(e) => {
            tracing::error!("Failed to create protocol: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn list_protocols(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ProtocolListQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .list_protocols(&mut **rls.conn(), org, EmergencyProtocolQuery::from(&query))
        .await;
    rls.release().await;
    match result {
        Ok(protocols) => Json(protocols).into_response(),
        Err(e) => {
            tracing::error!("Failed to list protocols: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn get_protocol(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(_): Query<OrgQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .find_protocol_by_id(&mut **rls.conn(), org, id)
        .await;
    rls.release().await;
    match result {
        Ok(Some(protocol)) => Json(protocol).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Protocol not found")),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to get protocol: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn update_protocol(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProtocolRequest>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .update_protocol(&mut **rls.conn(), org, id, req.data)
        .await;
    rls.release().await;
    match result {
        Ok(Some(protocol)) => Json(protocol).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Protocol not found")),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to update protocol: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn delete_protocol(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(_): Query<OrgQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .delete_protocol(&mut **rls.conn(), org, id)
        .await;
    rls.release().await;
    match result {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Protocol not found")),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to delete protocol: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}
