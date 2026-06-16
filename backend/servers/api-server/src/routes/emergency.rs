//! Emergency management routes for Epic 23.
//!
//! Handles emergency protocols, contacts, incidents, broadcasts, and drills.
//!
//! # RLS routing (PAP-80)
//!
//! Every handler acquires an [`RlsConnection`], which validates the caller's
//! JWT + org membership and opens a pooled connection with the RLS context
//! (`app.current_org_id` / user GUCs) bound to it. All `emergency_*` tables run
//! under `FORCE ROW LEVEL SECURITY` (migration `00179`), so the connection is
//! the only thing that scopes a query to the caller's tenant — the repository
//! holds no pool of its own.
//!
//! The authoritative organization is therefore `rls.tenant_id()` (the
//! membership-validated org from the session), **not** any client-supplied
//! `organization_id`. The org-keyed repo queries combine that value with the
//! RLS context, so the explicit `organization_id = $N` SQL filter and the
//! policy can never disagree; a foreign-tenant id resolves to `None` → `404`.
//! Membership is enforced by the extractor, so the previous `verify_org_access`
//! helper is no longer needed; manager-gated actions still check `rls.role()`.
//!
//! **IMPORTANT**: every path calls `rls.release().await` before returning so the
//! RLS context is cleared and the connection returns clean to the pool.

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::{ErrorResponse, TenantRole};
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
// Authorization helper (issue #827 / PAP-80)
// ============================================
//
// Org membership is enforced up-front by the `RlsConnection` extractor (it
// rejects non-members before the handler body runs), and RLS scopes every
// query to `rls.tenant_id()`. The only remaining handler-level check is the
// manager gate on mass-notification / incident-lifecycle actions, which mirrors
// the original `verify_org_manager` role set: org_admin / manager plus the
// platform-level admin roles. `TechnicalManager` is intentionally excluded to
// preserve the pre-conversion authorization surface.

/// True if `role` may perform manager-gated emergency actions (broadcast +
/// incident create/acknowledge — issue #827).
fn is_emergency_manager(role: TenantRole) -> bool {
    matches!(
        role,
        TenantRole::SuperAdmin
            | TenantRole::PlatformAdmin
            | TenantRole::OrgAdmin
            | TenantRole::Manager
    )
}

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

/// Update protocol request wrapper.
#[derive(Debug, Deserialize)]
pub struct UpdateProtocolRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: UpdateEmergencyProtocol,
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

// ============================================
// Contact Handlers
// ============================================

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

/// Update contact request wrapper.
#[derive(Debug, Deserialize)]
pub struct UpdateContactRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: UpdateEmergencyContact,
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

// ============================================
// Incident Handlers
// ============================================

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

/// Update incident request wrapper.
#[derive(Debug, Deserialize)]
pub struct UpdateIncidentRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: UpdateEmergencyIncident,
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

// ============================================
// Broadcast Handlers
// ============================================

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

// ============================================
// Drill Handlers
// ============================================

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

/// Update drill request wrapper.
#[derive(Debug, Deserialize)]
pub struct UpdateDrillRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: UpdateEmergencyDrill,
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

// ============================================
// Statistics Handlers
// ============================================

async fn get_statistics(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(_): Query<OrgQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .get_statistics(&mut **rls.conn(), org)
        .await;
    rls.release().await;
    match result {
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
    mut rls: RlsConnection,
    Query(_): Query<OrgQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .get_incident_summary_by_type(&mut **rls.conn(), org)
        .await;
    rls.release().await;
    match result {
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
    mut rls: RlsConnection,
    Query(_): Query<OrgQuery>,
) -> impl IntoResponse {
    let org = rls.tenant_id();
    let result = state
        .emergency_repo
        .get_incident_summary_by_severity(&mut **rls.conn(), org)
        .await;
    rls.release().await;
    match result {
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
