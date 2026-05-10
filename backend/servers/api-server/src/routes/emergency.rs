//! Emergency management routes for Epic 23.
//!
//! Handles emergency protocols, contacts, incidents, broadcasts, and drills.

use api_core::extractors::AuthUser;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::ErrorResponse;
use db::models::{
    AcknowledgeBroadcast, AddIncidentAttachment, CompleteDrill, CreateEmergencyBroadcast,
    CreateEmergencyContact, CreateEmergencyDrill, CreateEmergencyIncident, CreateEmergencyProtocol,
    CreateIncidentUpdate, EmergencyBroadcastQuery, EmergencyContactQuery, EmergencyDrillQuery,
    EmergencyIncidentQuery, EmergencyProtocolQuery, UpdateEmergencyContact, UpdateEmergencyDrill,
    UpdateEmergencyIncident, UpdateEmergencyProtocol,
};
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::state::AppState;

// ============================================
// Query Parameter Types
// ============================================

/// Organization query parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct OrgQuery {
    pub organization_id: Uuid,
}

/// Create protocol request wrapper.
#[derive(Debug, Deserialize)]
pub struct CreateProtocolRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateEmergencyProtocol,
}

/// Create contact request wrapper.
#[derive(Debug, Deserialize)]
pub struct CreateContactRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateEmergencyContact,
}

/// Create incident request wrapper.
#[derive(Debug, Deserialize)]
pub struct CreateIncidentRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateEmergencyIncident,
}

/// Create broadcast request wrapper.
#[derive(Debug, Deserialize)]
pub struct CreateBroadcastRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateEmergencyBroadcast,
}

/// Create drill request wrapper.
#[derive(Debug, Deserialize)]
pub struct CreateDrillRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateEmergencyDrill,
}

/// Protocol list query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ProtocolListQuery {
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub protocol_type: Option<String>,
    pub is_active: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl From<&ProtocolListQuery> for EmergencyProtocolQuery {
    fn from(q: &ProtocolListQuery) -> Self {
        EmergencyProtocolQuery {
            building_id: q.building_id,
            protocol_type: q.protocol_type.clone(),
            is_active: q.is_active,
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Contact list query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ContactListQuery {
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub contact_type: Option<String>,
    pub is_active: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl From<&ContactListQuery> for EmergencyContactQuery {
    fn from(q: &ContactListQuery) -> Self {
        EmergencyContactQuery {
            building_id: q.building_id,
            contact_type: q.contact_type.clone(),
            is_active: q.is_active,
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Incident list query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct IncidentListQuery {
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub incident_type: Option<String>,
    pub severity: Option<String>,
    pub status: Option<String>,
    pub active_only: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl From<&IncidentListQuery> for EmergencyIncidentQuery {
    fn from(q: &IncidentListQuery) -> Self {
        EmergencyIncidentQuery {
            building_id: q.building_id,
            incident_type: q.incident_type.clone(),
            severity: q.severity.clone(),
            status: q.status.clone(),
            active_only: q.active_only,
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Broadcast list query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct BroadcastListQuery {
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub incident_id: Option<Uuid>,
    pub is_active: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl From<&BroadcastListQuery> for EmergencyBroadcastQuery {
    fn from(q: &BroadcastListQuery) -> Self {
        EmergencyBroadcastQuery {
            building_id: q.building_id,
            incident_id: q.incident_id,
            is_active: q.is_active,
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Drill list query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct DrillListQuery {
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub drill_type: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl From<&DrillListQuery> for EmergencyDrillQuery {
    fn from(q: &DrillListQuery) -> Self {
        EmergencyDrillQuery {
            building_id: q.building_id,
            drill_type: q.drill_type.clone(),
            status: q.status.clone(),
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Create the emergency router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Protocol routes
        .route("/protocols", post(create_protocol))
        .route("/protocols", get(list_protocols))
        .route("/protocols/{id}", get(get_protocol))
        .route("/protocols/{id}", put(update_protocol))
        .route("/protocols/{id}", delete(delete_protocol))
        // Contact routes
        .route("/contacts", post(create_contact))
        .route("/contacts", get(list_contacts))
        .route("/contacts/{id}", get(get_contact))
        .route("/contacts/{id}", put(update_contact))
        .route("/contacts/{id}", delete(delete_contact))
        // Incident routes
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
        // Broadcast routes
        .route("/broadcasts", post(create_broadcast))
        .route("/broadcasts", get(list_broadcasts))
        .route("/broadcasts/{id}", get(get_broadcast))
        .route("/broadcasts/{id}/deactivate", post(deactivate_broadcast))
        .route("/broadcasts/{id}/acknowledge", post(acknowledge_broadcast))
        .route(
            "/broadcasts/{id}/acknowledgments",
            get(list_broadcast_acknowledgments),
        )
        // Drill routes
        .route("/drills", post(create_drill))
        .route("/drills", get(list_drills))
        .route("/drills/upcoming", get(get_upcoming_drills))
        .route("/drills/{id}", get(get_drill))
        .route("/drills/{id}", put(update_drill))
        .route("/drills/{id}/start", post(start_drill))
        .route("/drills/{id}/complete", post(complete_drill))
        .route("/drills/{id}/cancel", post(cancel_drill))
        .route("/drills/{id}", delete(delete_drill))
        // Statistics
        .route("/statistics", get(get_statistics))
        .route("/statistics/incidents/by-type", get(get_incidents_by_type))
        .route(
            "/statistics/incidents/by-severity",
            get(get_incidents_by_severity),
        )
}

// ============================================
// Protocol Handlers
// ============================================

async fn create_protocol(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateProtocolRequest>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .create_protocol(req.organization_id, auth.user_id, req.data)
        .await
    {
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
    _auth: AuthUser,
    Query(query): Query<ProtocolListQuery>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .list_protocols(query.organization_id, EmergencyProtocolQuery::from(&query))
        .await
    {
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
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .find_protocol_by_id(query.organization_id, id)
        .await
    {
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

/// Update protocol request wrapper.
#[derive(Debug, Deserialize)]
pub struct UpdateProtocolRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: UpdateEmergencyProtocol,
}

async fn update_protocol(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProtocolRequest>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .update_protocol(req.organization_id, id, req.data)
        .await
    {
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
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .delete_protocol(query.organization_id, id)
        .await
    {
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

// ============================================
// Contact Handlers
// ============================================

async fn create_contact(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<CreateContactRequest>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .create_contact(req.organization_id, req.data)
        .await
    {
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
    _auth: AuthUser,
    Query(query): Query<ContactListQuery>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .list_contacts(query.organization_id, EmergencyContactQuery::from(&query))
        .await
    {
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
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .find_contact_by_id(query.organization_id, id)
        .await
    {
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

/// Update contact request wrapper.
#[derive(Debug, Deserialize)]
pub struct UpdateContactRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: UpdateEmergencyContact,
}

async fn update_contact(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateContactRequest>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .update_contact(req.organization_id, id, req.data)
        .await
    {
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
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .delete_contact(query.organization_id, id)
        .await
    {
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

// ============================================
// Incident Handlers
// ============================================

async fn create_incident(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateIncidentRequest>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .create_incident(req.organization_id, auth.user_id, req.data)
        .await
    {
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
    _auth: AuthUser,
    Query(query): Query<IncidentListQuery>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .list_incidents(query.organization_id, EmergencyIncidentQuery::from(&query))
        .await
    {
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
    _auth: AuthUser,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .get_active_incidents(query.organization_id)
        .await
    {
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
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .find_incident_by_id(query.organization_id, id)
        .await
    {
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

/// Update incident request wrapper.
#[derive(Debug, Deserialize)]
pub struct UpdateIncidentRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: UpdateEmergencyIncident,
}

async fn update_incident(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateIncidentRequest>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .update_incident(req.organization_id, id, req.data)
        .await
    {
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
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .acknowledge_incident(query.organization_id, id)
        .await
    {
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
    organization_id: Uuid,
    resolution: String,
}

async fn resolve_incident(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ResolveIncidentRequest>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .resolve_incident(req.organization_id, id, auth.user_id, &req.resolution)
        .await
    {
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
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .close_incident(query.organization_id, id)
        .await
    {
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
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<AddIncidentAttachment>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .add_incident_attachment(id, auth.user_id, data)
        .await
    {
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
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.emergency_repo.list_incident_attachments(id).await {
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
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<CreateIncidentUpdate>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .add_incident_update(id, auth.user_id, data)
        .await
    {
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
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.emergency_repo.list_incident_updates(id).await {
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

// ============================================
// Broadcast Handlers
// ============================================

async fn create_broadcast(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateBroadcastRequest>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .create_broadcast(req.organization_id, auth.user_id, req.data)
        .await
    {
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
    _auth: AuthUser,
    Query(query): Query<BroadcastListQuery>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .list_broadcasts(query.organization_id, EmergencyBroadcastQuery::from(&query))
        .await
    {
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
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .find_broadcast_by_id(query.organization_id, id)
        .await
    {
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
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .deactivate_broadcast(query.organization_id, id)
        .await
    {
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
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<AcknowledgeBroadcast>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .acknowledge_broadcast(id, auth.user_id, data)
        .await
    {
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
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .list_broadcast_acknowledgments(id)
        .await
    {
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

// ============================================
// Drill Handlers
// ============================================

async fn create_drill(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateDrillRequest>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .create_drill(req.organization_id, auth.user_id, req.data)
        .await
    {
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
    _auth: AuthUser,
    Query(query): Query<DrillListQuery>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .list_drills(query.organization_id, EmergencyDrillQuery::from(&query))
        .await
    {
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
    organization_id: Uuid,
    days: Option<i32>,
}

async fn get_upcoming_drills(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(query): Query<UpcomingDrillsQuery>,
) -> impl IntoResponse {
    let days = query.days.unwrap_or(30);
    match state
        .emergency_repo
        .get_upcoming_drills(query.organization_id, days)
        .await
    {
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
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .find_drill_by_id(query.organization_id, id)
        .await
    {
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

/// Update drill request wrapper.
#[derive(Debug, Deserialize)]
pub struct UpdateDrillRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: UpdateEmergencyDrill,
}

async fn update_drill(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateDrillRequest>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .update_drill(req.organization_id, id, req.data)
        .await
    {
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
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .start_drill(query.organization_id, id)
        .await
    {
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
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CompleteDrillRequest>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .complete_drill(req.organization_id, id, req.data)
        .await
    {
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
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .cancel_drill(query.organization_id, id)
        .await
    {
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
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .delete_drill(query.organization_id, id)
        .await
    {
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

// ============================================
// Statistics Handlers
// ============================================

async fn get_statistics(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .get_statistics(query.organization_id)
        .await
    {
        Ok(stats) => Json(stats).into_response(),
        Err(e) => {
            tracing::error!("Failed to get statistics: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn get_incidents_by_type(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .get_incident_summary_by_type(query.organization_id)
        .await
    {
        Ok(summary) => Json(summary).into_response(),
        Err(e) => {
            tracing::error!("Failed to get incidents by type: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn get_incidents_by_severity(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    match state
        .emergency_repo
        .get_incident_summary_by_severity(query.organization_id)
        .await
    {
        Ok(summary) => Json(summary).into_response(),
        Err(e) => {
            tracing::error!("Failed to get incidents by severity: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}
