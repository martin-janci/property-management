//! Emergency incident route surface — CRUD, lifecycle (acknowledge / resolve /
//! close), attachments, and updates.

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use common::ErrorResponse;
use db::models::{AddIncidentAttachment, CreateIncidentUpdate, EmergencyIncidentQuery};
use serde::Deserialize;
use uuid::Uuid;

use super::shared::{
    is_emergency_manager, CreateIncidentRequest, IncidentListQuery, OrgQuery, UpdateIncidentRequest,
};
use crate::state::AppState;

/// Incident sub-router.
pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/incidents", post(create_incident))
        .route("/incidents", get(list_incidents))
        .route("/incidents/active", get(get_active_incidents))
        .route("/incidents/{id}", get(get_incident))
        .route("/incidents/{id}", put(update_incident))
        .route("/incidents/{id}/acknowledge", post(acknowledge_incident))
        .route("/incidents/{id}/resolve", post(resolve_incident))
        .route("/incidents/{id}/close", post(close_incident))
        .route("/incidents/{id}/attachments", post(add_incident_attachment))
        .route(
            "/incidents/{id}/attachments",
            get(list_incident_attachments),
        )
        .route("/incidents/{id}/updates", post(add_incident_update))
        .route("/incidents/{id}/updates", get(list_incident_updates))
}

async fn create_incident(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateIncidentRequest>,
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
        .create_incident(&mut **rls.conn(), org, user, req.data)
        .await;
    rls.release().await;
    match result {
        Ok(incident) => (StatusCode::CREATED, Json(incident)).into_response(),
        Err(e) => {
            tracing::error!("Failed to create incident: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn list_incidents(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<IncidentListQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .list_incidents(&mut **rls.conn(), org, EmergencyIncidentQuery::from(&query))
        .await;
    rls.release().await;
    match result {
        Ok(incidents) => Json(incidents).into_response(),
        Err(e) => {
            tracing::error!("Failed to list incidents: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn get_active_incidents(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(_): Query<OrgQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .get_active_incidents(&mut **rls.conn(), org)
        .await;
    rls.release().await;
    match result {
        Ok(incidents) => Json(incidents).into_response(),
        Err(e) => {
            tracing::error!("Failed to get active incidents: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn get_incident(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(_): Query<OrgQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .find_incident_by_id(&mut **rls.conn(), org, id)
        .await;
    rls.release().await;
    match result {
        Ok(Some(incident)) => Json(incident).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Incident not found")),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to get incident: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn update_incident(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateIncidentRequest>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .update_incident(&mut **rls.conn(), org, id, req.data)
        .await;
    rls.release().await;
    match result {
        Ok(Some(incident)) => Json(incident).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Incident not found")),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to update incident: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn acknowledge_incident(
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
        .acknowledge_incident(&mut **rls.conn(), org, id)
        .await;
    rls.release().await;
    match result {
        Ok(Some(incident)) => Json(incident).into_response(),
        Ok(None) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_STATE",
                "Incident cannot be acknowledged",
            )),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to acknowledge incident: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

#[derive(Debug, Deserialize)]
struct ResolveIncidentRequest {
    resolution: String,
}

async fn resolve_incident(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<ResolveIncidentRequest>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let user = rls.user_id();
    let result = state
        .emergency_repo
        .resolve_incident(&mut **rls.conn(), org, id, user, &req.resolution)
        .await;
    rls.release().await;
    match result {
        Ok(Some(incident)) => Json(incident).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Incident not found")),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to resolve incident: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn close_incident(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(_): Query<OrgQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .close_incident(&mut **rls.conn(), org, id)
        .await;
    rls.release().await;
    match result {
        Ok(Some(incident)) => Json(incident).into_response(),
        Ok(None) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_STATE",
                "Incident cannot be closed",
            )),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to close incident: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn add_incident_attachment(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(_): Query<OrgQuery>,
    Json(data): Json<AddIncidentAttachment>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let user = rls.user_id();
    // The attachments table is RLS-scoped via the parent incident, but it keys
    // only on `incident_id`; confirm the incident is in the caller's org first
    // so an unknown / cross-tenant id yields a 404 instead of a policy-violation
    // 500 on the INSERT.
    match state
        .emergency_repo
        .find_incident_by_id(&mut **rls.conn(), org, id)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            rls.release().await;
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Incident not found")),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Failed to load incident: {:?}", e);
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
        .add_incident_attachment(&mut **rls.conn(), id, user, data)
        .await;
    rls.release().await;
    match result {
        Ok(attachment) => (StatusCode::CREATED, Json(attachment)).into_response(),
        Err(e) => {
            tracing::error!("Failed to add incident attachment: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn list_incident_attachments(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(_): Query<OrgQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    match state
        .emergency_repo
        .find_incident_by_id(&mut **rls.conn(), org, id)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            rls.release().await;
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Incident not found")),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Failed to load incident: {:?}", e);
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
        .list_incident_attachments(&mut **rls.conn(), id)
        .await;
    rls.release().await;
    match result {
        Ok(attachments) => Json(attachments).into_response(),
        Err(e) => {
            tracing::error!("Failed to list incident attachments: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn add_incident_update(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(_): Query<OrgQuery>,
    Json(data): Json<CreateIncidentUpdate>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let user = rls.user_id();
    match state
        .emergency_repo
        .find_incident_by_id(&mut **rls.conn(), org, id)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            rls.release().await;
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Incident not found")),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Failed to load incident: {:?}", e);
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
        .add_incident_update(&mut **rls.conn(), id, user, data)
        .await;
    rls.release().await;
    match result {
        Ok(update) => (StatusCode::CREATED, Json(update)).into_response(),
        Err(e) => {
            tracing::error!("Failed to add incident update: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn list_incident_updates(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(_): Query<OrgQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    match state
        .emergency_repo
        .find_incident_by_id(&mut **rls.conn(), org, id)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            rls.release().await;
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Incident not found")),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Failed to load incident: {:?}", e);
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
        .list_incident_updates(&mut **rls.conn(), id)
        .await;
    rls.release().await;
    match result {
        Ok(updates) => Json(updates).into_response(),
        Err(e) => {
            tracing::error!("Failed to list incident updates: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}
