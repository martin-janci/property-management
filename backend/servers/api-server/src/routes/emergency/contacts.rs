//! Emergency contact route surface (CRUD).

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::ErrorResponse;
use db::models::EmergencyContactQuery;
use uuid::Uuid;

use super::shared::{ContactListQuery, CreateContactRequest, OrgQuery, UpdateContactRequest};
use crate::state::AppState;

/// Contact sub-router.
pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/contacts", post(create_contact))
        .route("/contacts", get(list_contacts))
        .route("/contacts/{id}", get(get_contact))
        .route("/contacts/{id}", put(update_contact))
        .route("/contacts/{id}", delete(delete_contact))
}

async fn create_contact(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateContactRequest>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .create_contact(&mut **rls.conn(), org, req.data)
        .await;
    rls.release().await;
    match result {
        Ok(contact) => (StatusCode::CREATED, Json(contact)).into_response(),
        Err(e) => {
            tracing::error!("Failed to create contact: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn list_contacts(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ContactListQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .list_contacts(&mut **rls.conn(), org, EmergencyContactQuery::from(&query))
        .await;
    rls.release().await;
    match result {
        Ok(contacts) => Json(contacts).into_response(),
        Err(e) => {
            tracing::error!("Failed to list contacts: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn get_contact(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(_): Query<OrgQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .find_contact_by_id(&mut **rls.conn(), org, id)
        .await;
    rls.release().await;
    match result {
        Ok(Some(contact)) => Json(contact).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Contact not found")),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to get contact: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn update_contact(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateContactRequest>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .update_contact(&mut **rls.conn(), org, id, req.data)
        .await;
    rls.release().await;
    match result {
        Ok(Some(contact)) => Json(contact).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Contact not found")),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to update contact: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn delete_contact(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(_): Query<OrgQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .delete_contact(&mut **rls.conn(), org, id)
        .await;
    rls.release().await;
    match result {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Contact not found")),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to delete contact: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}
