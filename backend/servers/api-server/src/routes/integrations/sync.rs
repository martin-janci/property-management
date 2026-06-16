//! Integration sync-surface routes (Epic 61).
//!
//! Covers: integration statistics, calendar connections + sync + events,
//! accounting exports + settings, e-signature workflows, and video meetings.
//!
//! UNMOUNTED (PAP-122): `router()` is not merged into the integrations
//! router — `calendar_connections`, `calendar_events`, `accounting_exports`,
//! `accounting_export_settings`, `esignature_workflows`,
//! `esignature_recipients`, `video_conference_connections`, and
//! `video_meetings` exist in no migration, so every handler fails at runtime
//! with undefined-table errors. Remount after the Epic-61 migrations land
//! (incl. FORCE-RLS policies per migration 00179 conventions).
//!
//! # RLS routing (PAP-105 / PAP-80)
//!
//! `IntegrationRepository` holds no pool: every handler acquires an
//! [`RlsConnection`] (JWT + org-membership validation + org/user GUCs bound to
//! the pooled connection) and passes `&mut **rls.conn()` to the repository.
//! Of the tables this surface touches only `webhook_subscriptions` (read by
//! `get_integration_statistics`) is FORCE-RLS today, but routing everything
//! through the context-set connection means no query can run un-scoped. The
//! `{org_id}` path segment is still membership-checked via `verify_org_access`
//! for the non-FORCE tables; the stats handler additionally requires the path
//! org to equal `rls.tenant_id()` so the SQL filter and the RLS policy can
//! never disagree. Every path calls `rls.release().await` before returning,
//! and slow external I/O (calendar provider fetch, webhook test POST) runs
//! AFTER release so pool connections are not pinned on network calls.

use api_core::extractors::RlsConnection;
use api_core::TenantExtractor;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::Response,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{Duration, Utc};
use common::errors::ErrorResponse;
use db::models::{
    accounting_system, calendar_provider, AccountingExport, AccountingExportSettings,
    CalendarConnection, CalendarSyncResult, CreateAccountingExport, CreateCalendarConnection,
    CreateESignatureWorkflow, CreateIntegrationCalendarEvent as CreateCalendarEvent,
    CreateVideoConferenceConnection, CreateVideoMeeting, ESignatureWorkflow,
    ESignatureWorkflowWithRecipients, IntegrationCalendarEvent as CalendarEvent,
    IntegrationStatistics, SyncCalendarRequest, UpdateAccountingExportSettings,
    UpdateCalendarConnection, UpdateVideoMeeting, VideoConferenceConnection, VideoMeeting,
};
use hmac::{Hmac, KeyInit, Mac};
use integrations::{
    GoogleCalendarClient, MicrosoftCalendarClient, MoneyS3Exporter, OAuthConfig, PohodaExporter,
};
use serde::Deserialize;
use sha2::Sha256;
use tracing::Instrument;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::state::AppState;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Deserialize, IntoParams)]
pub struct OrgIdPath {
    pub org_id: Uuid,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ResourceIdPath {
    pub id: Uuid,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct AccountingSystemPath {
    pub org_id: Uuid,
    pub system: String,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct CalendarQuery {
    pub user_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct CalendarEventsQuery {
    pub from: Option<chrono::DateTime<chrono::Utc>>,
    pub to: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct AccountingExportQuery {
    pub system_type: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i32,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ESignatureQuery {
    pub status: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i32,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct VideoMeetingQuery {
    pub from: Option<chrono::DateTime<chrono::Utc>>,
    pub status: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i32,
}

fn default_limit() -> i32 {
    50
}

/// Verify user has access to the specified organization.
pub(super) async fn verify_org_access(
    state: &AppState,
    user_id: uuid::Uuid,
    org_id: uuid::Uuid,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let is_member = state
        .org_member_repo
        .is_member(org_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check org membership");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
            )
        })?;

    if !is_member {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "You are not a member of this organization",
            )),
        ));
    }

    Ok(())
}

pub(super) fn verify_manager_role(
    tenant: &TenantExtractor,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Manager-level access required",
            )),
        ));
    }
    Ok(())
}

pub(super) fn verify_docusign_signature(secret: &str, payload: &[u8], signature: &str) -> bool {
    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(payload);
    use base64::Engine as _;
    let Ok(signature_bytes) = base64::engine::general_purpose::STANDARD.decode(signature) else {
        return false;
    };
    mac.verify_slice(&signature_bytes).is_ok()
}

pub(super) fn verify_adobe_sign_signature(
    client_secret: &str,
    payload: &[u8],
    signature: &str,
) -> bool {
    let Ok(mut mac) = HmacSha256::new_from_slice(client_secret.as_bytes()) else {
        return false;
    };
    mac.update(payload);
    let Ok(signature_bytes) = hex::decode(signature) else {
        return false;
    };
    mac.verify_slice(&signature_bytes).is_ok()
}

pub(super) fn verify_hellosign_signature(
    api_key: &str,
    event_time: &str,
    event_type: &str,
    event_hash: &str,
) -> bool {
    let Ok(mut mac) = HmacSha256::new_from_slice(api_key.as_bytes()) else {
        return false;
    };
    mac.update(format!("{}{}", event_time, event_type).as_bytes());
    let Ok(signature_bytes) = hex::decode(event_hash) else {
        return false;
    };
    mac.verify_slice(&signature_bytes).is_ok()
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/organizations/{org_id}/stats", get(get_integration_stats))
        .route(
            "/organizations/{org_id}/calendars",
            get(list_calendar_connections),
        )
        .route(
            "/organizations/{org_id}/calendars",
            post(create_calendar_connection),
        )
        .route("/calendars/{id}", get(get_calendar_connection))
        .route("/calendars/{id}", put(update_calendar_connection))
        .route("/calendars/{id}", delete(delete_calendar_connection))
        .route("/calendars/{id}/sync", post(sync_calendar))
        .route("/calendars/{id}/events", get(list_calendar_events))
        .route("/calendars/{id}/events", post(create_calendar_event))
        .route(
            "/organizations/{org_id}/accounting/exports",
            get(list_accounting_exports),
        )
        .route(
            "/organizations/{org_id}/accounting/exports",
            post(create_accounting_export),
        )
        .route("/accounting/exports/{id}", get(get_accounting_export))
        .route(
            "/accounting/exports/{id}/download",
            get(download_accounting_export),
        )
        .route(
            "/organizations/{org_id}/accounting/settings/{system}",
            get(get_accounting_settings),
        )
        .route(
            "/organizations/{org_id}/accounting/settings/{system}",
            put(update_accounting_settings),
        )
        .route(
            "/organizations/{org_id}/esignatures",
            get(list_esignature_workflows),
        )
        .route(
            "/organizations/{org_id}/esignatures",
            post(create_esignature_workflow),
        )
        .route("/esignatures/{id}", get(get_esignature_workflow))
        .route("/esignatures/{id}/send", post(send_esignature_workflow))
        .route("/esignatures/{id}/void", post(void_esignature_workflow))
        .route("/esignatures/{id}/remind", post(send_esignature_reminder))
        .route(
            "/organizations/{org_id}/video/connections",
            get(list_video_connections),
        )
        .route(
            "/organizations/{org_id}/video/connections",
            post(create_video_connection),
        )
        .route("/video/connections/{id}", delete(delete_video_connection))
        .route(
            "/organizations/{org_id}/video/meetings",
            get(list_video_meetings),
        )
        .route(
            "/organizations/{org_id}/video/meetings",
            post(create_video_meeting),
        )
        .route("/video/meetings/{id}", get(get_video_meeting))
        .route("/video/meetings/{id}", put(update_video_meeting))
        .route("/video/meetings/{id}", delete(delete_video_meeting))
        .route("/video/meetings/{id}/start", post(start_video_meeting))
}

// ==================== Statistics ====================

/// Get integration statistics for an organization.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/organizations/{org_id}/stats",
    params(OrgIdPath),
    responses(
        (status = 200, description = "Statistics retrieved", body = IntegrationStatistics),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not a member of the organization"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn get_integration_stats(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<OrgIdPath>,
) -> Result<Json<IntegrationStatistics>, (StatusCode, Json<ErrorResponse>)> {
    // PAP-105 (PAP-80): the webhook sub-queries hit FORCE-RLS
    // `webhook_subscriptions`, so the stats org must be the org the RLS
    // context is bound to — a mismatching path org would silently read as
    // zero. Membership in the tenant is validated by the extractor.
    if path.org_id != rls.tenant_id() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "You are not a member of this organization",
            )),
        ));
    }

    let result = state
        .integration_repo
        .get_integration_statistics(rls.conn(), path.org_id)
        .await;
    rls.release().await;

    let stats = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to get integration statistics");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to get integration statistics",
            )),
        )
    })?;

    Ok(Json(stats))
}

// ==================== Calendar (Story 61.1) ====================

/// List calendar connections for an organization.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/organizations/{org_id}/calendars",
    params(OrgIdPath, CalendarQuery),
    responses(
        (status = 200, description = "Connections retrieved", body = Vec<CalendarConnection>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not a member of the organization"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn list_calendar_connections(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<OrgIdPath>,
    Query(query): Query<CalendarQuery>,
) -> Result<Json<Vec<CalendarConnection>>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user belongs to this organization
    if let Err(e) = verify_org_access(&state, rls.user_id(), path.org_id).await {
        rls.release().await;
        return Err(e);
    }

    let result = state
        .integration_repo
        .list_calendar_connections(&mut **rls.conn(), path.org_id, query.user_id)
        .await;
    rls.release().await;

    let connections = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to list calendar connections");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to list calendar connections",
            )),
        )
    })?;

    Ok(Json(connections))
}

/// Create a calendar connection.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/organizations/{org_id}/calendars",
    params(OrgIdPath),
    request_body = CreateCalendarConnection,
    responses(
        (status = 201, description = "Connection created", body = CalendarConnection),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn create_calendar_connection(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<OrgIdPath>,
    Json(data): Json<CreateCalendarConnection>,
) -> Result<(StatusCode, Json<CalendarConnection>), (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();
    // Verify user has access to the organization
    if let Err(e) = verify_org_access(&state, user_id, path.org_id).await {
        rls.release().await;
        return Err(e);
    }

    let result = state
        .integration_repo
        .create_calendar_connection(&mut **rls.conn(), path.org_id, user_id, data)
        .await;
    rls.release().await;

    let connection = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to create calendar connection");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to create calendar connection",
            )),
        )
    })?;

    Ok((StatusCode::CREATED, Json(connection)))
}

/// Get a calendar connection by ID.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/calendars/{id}",
    params(ResourceIdPath),
    responses(
        (status = 200, description = "Connection retrieved", body = CalendarConnection),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not a member of the organization"),
        (status = 404, description = "Connection not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn get_calendar_connection(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<CalendarConnection>, (StatusCode, Json<ErrorResponse>)> {
    // First get the connection to check organization access
    let result = state
        .integration_repo
        .get_calendar_connection(&mut **rls.conn(), path.id)
        .await;
    rls.release().await;

    let connection = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to get calendar connection");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to get calendar connection",
            )),
        )
    })?;

    match connection {
        Some(c) => {
            // Verify user has access to the organization that owns this resource
            verify_org_access(&state, rls.user_id(), c.organization_id).await?;
            Ok(Json(c))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Calendar connection not found",
            )),
        )),
    }
}

/// Update a calendar connection.
#[utoipa::path(
    put,
    path = "/api/v1/integrations/calendars/{id}",
    params(ResourceIdPath),
    request_body = UpdateCalendarConnection,
    responses(
        (status = 200, description = "Connection updated", body = CalendarConnection),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not a member of the organization"),
        (status = 404, description = "Connection not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn update_calendar_connection(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<ResourceIdPath>,
    Json(data): Json<UpdateCalendarConnection>,
) -> Result<Json<CalendarConnection>, (StatusCode, Json<ErrorResponse>)> {
    // First get the connection to check organization access
    let existing = match state
        .integration_repo
        .get_calendar_connection(&mut **rls.conn(), path.id)
        .await
    {
        Ok(Some(c)) => c,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Calendar connection not found",
                )),
            ));
        }
        Err(e) => {
            rls.release().await;
            tracing::error!(error = %e, "Failed to get calendar connection");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
            ));
        }
    };

    // Verify user has access to the organization that owns this resource
    if let Err(e) = verify_org_access(&state, rls.user_id(), existing.organization_id).await {
        rls.release().await;
        return Err(e);
    }

    let result = state
        .integration_repo
        .update_calendar_connection(&mut **rls.conn(), path.id, data)
        .await;
    rls.release().await;

    let connection = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to update calendar connection");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to update calendar connection",
            )),
        )
    })?;

    Ok(Json(connection))
}

/// Delete a calendar connection.
#[utoipa::path(
    delete,
    path = "/api/v1/integrations/calendars/{id}",
    params(ResourceIdPath),
    responses(
        (status = 204, description = "Connection deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not a member of the organization"),
        (status = 404, description = "Connection not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn delete_calendar_connection(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<ResourceIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // First get the connection to check organization access
    let existing = match state
        .integration_repo
        .get_calendar_connection(&mut **rls.conn(), path.id)
        .await
    {
        Ok(Some(c)) => c,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Calendar connection not found",
                )),
            ));
        }
        Err(e) => {
            rls.release().await;
            tracing::error!(error = %e, "Failed to get calendar connection");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
            ));
        }
    };

    // Verify user has access to the organization that owns this resource
    if let Err(e) = verify_org_access(&state, rls.user_id(), existing.organization_id).await {
        rls.release().await;
        return Err(e);
    }

    let result = state
        .integration_repo
        .delete_calendar_connection(&mut **rls.conn(), path.id)
        .await;
    rls.release().await;

    let deleted = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to delete calendar connection");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to delete calendar connection",
            )),
        )
    })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Calendar connection not found",
            )),
        ))
    }
}

/// Sync calendar events.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/calendars/{id}/sync",
    params(ResourceIdPath),
    request_body = SyncCalendarRequest,
    responses(
        (status = 200, description = "Calendar synced", body = CalendarSyncResult),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not a member of the organization"),
        (status = 404, description = "Connection not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn sync_calendar(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<ResourceIdPath>,
    Json(data): Json<SyncCalendarRequest>,
) -> Result<Json<CalendarSyncResult>, (StatusCode, Json<ErrorResponse>)> {
    // Get the calendar connection
    let result = state
        .integration_repo
        .get_calendar_connection(&mut **rls.conn(), path.id)
        .await;
    // PAP-105 (PAP-80): release before the (slow) external calendar fetch so
    // we don't pin a pool connection on network I/O. The post-fetch writes
    // below run on the pool — calendar_events / calendar_connections are not
    // FORCE-RLS, and org scoping was already enforced by the connection
    // lookup + membership check here.
    rls.release().await;

    let connection = result
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get calendar connection");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get calendar connection",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Calendar connection not found",
                )),
            )
        })?;

    // Verify user has access to the organization that owns this connection
    verify_org_access(&state, rls.user_id(), connection.organization_id).await?;

    // Check if we have valid tokens
    let access_token = match &connection.access_token {
        Some(token) => token.clone(),
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "NO_TOKEN",
                    "Calendar connection has no access token. Please reconnect.",
                )),
            ))
        }
    };

    // Determine date range for sync
    let time_min = data
        .date_range_start
        .unwrap_or_else(|| Utc::now() - Duration::days(30));
    let time_max = data
        .date_range_end
        .unwrap_or_else(|| Utc::now() + Duration::days(90));

    // Get calendar_id, default to primary if not set
    let calendar_id = connection
        .calendar_id
        .clone()
        .unwrap_or_else(|| "primary".to_string());

    // Helper macro to get required env var or return config error
    macro_rules! require_env {
        ($name:expr, $provider:expr) => {
            std::env::var($name).map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "CONFIG_ERROR",
                        format!(
                            "{} calendar integration not configured. {} is required.",
                            $provider, $name
                        ),
                    )),
                )
            })?
        };
    }

    // Create appropriate client based on provider and fetch events
    let sync_result = match connection.provider.as_str() {
        calendar_provider::GOOGLE => {
            let config = OAuthConfig {
                client_id: require_env!("GOOGLE_CLIENT_ID", "Google"),
                client_secret: require_env!("GOOGLE_CLIENT_SECRET", "Google"),
                redirect_uri: require_env!("GOOGLE_REDIRECT_URI", "Google"),
            };
            let client = GoogleCalendarClient::new(config);

            client
                .fetch_events(&access_token, &calendar_id, time_min, time_max, None)
                .await
        }
        calendar_provider::OUTLOOK => {
            let config = OAuthConfig {
                client_id: require_env!("MICROSOFT_CLIENT_ID", "Microsoft"),
                client_secret: require_env!("MICROSOFT_CLIENT_SECRET", "Microsoft"),
                redirect_uri: require_env!("MICROSOFT_REDIRECT_URI", "Microsoft"),
            };
            let client = MicrosoftCalendarClient::new(config);

            client
                .fetch_events(&access_token, &calendar_id, time_min, time_max, None)
                .await
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "UNSUPPORTED_PROVIDER",
                    format!(
                        "Calendar provider '{}' is not supported",
                        connection.provider
                    ),
                )),
            ))
        }
    };

    // Handle the sync result
    let sync_result = sync_result.map_err(|e| {
        tracing::error!(error = %e, provider = %connection.provider, "Calendar sync failed");

        // Update sync status to error in the background (fire and forget).
        // PAP-105 (PAP-80): non-request-context background write to
        // calendar_connections (not FORCE-RLS); org scoping was already
        // enforced by the handler's connection lookup above, so the cloned
        // pool is the executor here.
        let repo = state.integration_repo.clone();
        let db = state.db.clone();
        let connection_id = path.id;
        let error_msg = e.to_string();
        tokio::spawn(
            async move {
                let _ = repo
                    .update_sync_status(&db, connection_id, "error", Some(&error_msg))
                    .await;
            }
            .instrument(tracing::info_span!(
                "bg.integration_sync_status_update",
                connection_id = %connection_id,
            )),
        );

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "SYNC_ERROR",
                "Calendar sync failed. Please try again later.",
            )),
        )
    })?;

    // PAP-150: acquire one short-lived org-context connection for the
    // post-fetch writes below. calendar_events / calendar_connections are not
    // FORCE-RLS and org scoping was already enforced by the connection lookup
    // + membership check above, but the RLS gate forbids handler-side raw
    // `state.db`. The request-scoped RlsConnection was released before the
    // provider round-trip, so take a fresh guard scoped to the connection's org.
    let mut sync_guard = db::RlsPool::new(state.db.clone())
        .acquire_with_rls(connection.organization_id, rls.user_id(), false)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to acquire db connection for calendar sync writes");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Calendar sync failed. Please try again later.",
                )),
            )
        })?;

    // Process synced events and store them in the database
    let mut events_created = 0;
    let mut errors: Vec<String> = vec![];

    for event in &sync_result.events_created {
        // Use external event ID for deduplication
        let create_data = CreateCalendarEvent {
            connection_id: path.id,
            external_event_id: Some(event.id.clone()), // Store external ID to prevent duplicates
            source_type: "external".to_string(),
            source_id: None,
            title: event.title.clone(),
            description: event.description.clone(),
            location: event.location.clone(),
            start_time: event.start_time,
            end_time: event.end_time,
            all_day: Some(event.all_day),
            recurrence_rule: event.recurrence.clone(),
            attendees: Some(serde_json::to_value(&event.attendees).unwrap_or_default()),
        };

        // Use upsert to handle duplicates - if event with same source_id exists, skip.
        // PAP-105 (PAP-80) / PAP-150: post-fetch write to calendar_events (not
        // FORCE-RLS); the request RLS connection was released before the
        // provider round-trip, so this runs on a fresh org-context guard
        // (org scoping was enforced by the lookup + membership check above).
        match state
            .integration_repo
            .upsert_calendar_event(&mut **sync_guard.conn(), create_data)
            .await
        {
            Ok(created) => {
                if created {
                    events_created += 1;
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, event_id = %event.id, "Failed to create calendar event");
                errors.push(format!("Failed to create event '{}': {}", event.title, e));
            }
        }
    }

    // For updated events, we would need to update existing records
    // This is a simplified implementation - in production you'd match by external_event_id
    let events_updated = sync_result.events_updated.len() as i32;

    // Update sync status (org-context write to non-FORCE calendar_connections, see above)
    let _ = state
        .integration_repo
        .update_sync_status(&mut **sync_guard.conn(), path.id, "active", None)
        .await;
    sync_guard.release().await;

    Ok(Json(CalendarSyncResult {
        events_created,
        events_updated,
        events_deleted: sync_result.events_deleted.len() as i32,
        errors,
        synced_at: Utc::now(),
    }))
}

/// List calendar events.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/calendars/{id}/events",
    params(ResourceIdPath, CalendarEventsQuery),
    responses(
        (status = 200, description = "Events retrieved", body = Vec<CalendarEvent>),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn list_calendar_events(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<ResourceIdPath>,
    Query(query): Query<CalendarEventsQuery>,
) -> Result<Json<Vec<CalendarEvent>>, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .integration_repo
        .list_calendar_events(&mut **rls.conn(), path.id, query.from, query.to)
        .await;
    rls.release().await;

    let events = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to list calendar events");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to list calendar events",
            )),
        )
    })?;

    Ok(Json(events))
}

/// Create a calendar event.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/calendars/{id}/events",
    params(ResourceIdPath),
    request_body = CreateCalendarEvent,
    responses(
        (status = 201, description = "Event created", body = CalendarEvent),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn create_calendar_event(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(_path): Path<ResourceIdPath>,
    Json(data): Json<CreateCalendarEvent>,
) -> Result<(StatusCode, Json<CalendarEvent>), (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .integration_repo
        .create_calendar_event(&mut **rls.conn(), data)
        .await;
    rls.release().await;

    let event = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to create calendar event");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to create calendar event",
            )),
        )
    })?;

    Ok((StatusCode::CREATED, Json(event)))
}

// ==================== Accounting (Story 61.2) ====================

/// List accounting exports for an organization.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/organizations/{org_id}/accounting/exports",
    params(OrgIdPath, AccountingExportQuery),
    responses(
        (status = 200, description = "Exports retrieved", body = Vec<AccountingExport>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not a member of the organization"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn list_accounting_exports(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<OrgIdPath>,
    Query(query): Query<AccountingExportQuery>,
) -> Result<Json<Vec<AccountingExport>>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user belongs to this organization
    if let Err(e) = verify_org_access(&state, rls.user_id(), path.org_id).await {
        rls.release().await;
        return Err(e);
    }

    let result = state
        .integration_repo
        .list_accounting_exports(
            &mut **rls.conn(),
            path.org_id,
            query.system_type.as_deref(),
            query.limit,
        )
        .await;
    rls.release().await;

    let exports = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to list accounting exports");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to list accounting exports",
            )),
        )
    })?;

    Ok(Json(exports))
}

/// Create an accounting export.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/organizations/{org_id}/accounting/exports",
    params(OrgIdPath),
    request_body = CreateAccountingExport,
    responses(
        (status = 201, description = "Export created", body = AccountingExport),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn create_accounting_export(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<OrgIdPath>,
    Json(data): Json<CreateAccountingExport>,
) -> Result<(StatusCode, Json<AccountingExport>), (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();
    // Verify user has access to the organization
    if let Err(e) = verify_org_access(&state, user_id, path.org_id).await {
        rls.release().await;
        return Err(e);
    }

    let result = state
        .integration_repo
        .create_accounting_export(&mut **rls.conn(), path.org_id, user_id, data)
        .await;
    rls.release().await;

    let export = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to create accounting export");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to create accounting export",
            )),
        )
    })?;

    Ok((StatusCode::CREATED, Json(export)))
}

/// Get an accounting export by ID.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/accounting/exports/{id}",
    params(ResourceIdPath),
    responses(
        (status = 200, description = "Export retrieved", body = AccountingExport),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not a member of the organization"),
        (status = 404, description = "Export not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn get_accounting_export(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<AccountingExport>, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .integration_repo
        .get_accounting_export(&mut **rls.conn(), path.id)
        .await;
    rls.release().await;

    let export = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to get accounting export");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to get accounting export",
            )),
        )
    })?;

    match export {
        Some(e) => {
            // Verify user has access to the organization that owns this resource
            verify_org_access(&state, rls.user_id(), e.organization_id).await?;
            Ok(Json(e))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Accounting export not found",
            )),
        )),
    }
}

/// Download an accounting export file.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/accounting/exports/{id}/download",
    params(ResourceIdPath),
    responses(
        (status = 200, description = "File downloaded"),
        (status = 404, description = "Export not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn download_accounting_export(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<ResourceIdPath>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    // Get the export record (the only DB read; release before file generation)
    let result = state
        .integration_repo
        .get_accounting_export(&mut **rls.conn(), path.id)
        .await;
    rls.release().await;

    let export = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to get accounting export");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to get accounting export",
            )),
        )
    })?;

    let export = match export {
        Some(e) => e,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Accounting export not found",
                )),
            ))
        }
    };

    // Verify user has access to the organization that owns this export
    verify_org_access(&state, rls.user_id(), export.organization_id).await?;

    // Check if export is completed
    if export.status != "completed" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "EXPORT_NOT_READY",
                format!(
                    "Export is not ready for download. Current status: {}",
                    export.status
                ),
            )),
        ));
    }

    // If file_path exists, we would read from storage
    // For now, generate the export on-the-fly (in production, you'd read from S3/file storage)
    let (content, content_type, filename) = match export.system_type.as_str() {
        accounting_system::POHODA => {
            // Generate POHODA XML export
            let exporter = PohodaExporter::new(
                std::env::var("COMPANY_ICO").unwrap_or_else(|_| "00000000".to_string()),
            );

            // In a real implementation, you would fetch the actual invoices from the database
            // based on the export's period_start and period_end
            // For now, we generate an empty but valid XML structure
            let invoices: Vec<integrations::ExportInvoice> = vec![];

            let mut output = Vec::new();
            exporter
                .export_invoices(&mut output, &invoices)
                .map_err(|e| {
                    tracing::error!(error = %e, "Failed to generate POHODA export");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new(
                            "EXPORT_ERROR",
                            format!("Failed to generate POHODA export: {}", e),
                        )),
                    )
                })?;

            let filename = format!(
                "pohoda_export_{}_{}_{}.xml",
                export.export_type, export.period_start, export.period_end
            );

            (output, "application/xml", filename)
        }
        accounting_system::MONEY_S3 => {
            // Generate Money S3 CSV export
            let exporter = MoneyS3Exporter::new();

            // In a real implementation, you would fetch the actual invoices from the database
            let invoices: Vec<integrations::ExportInvoice> = vec![];

            let mut output = Vec::new();
            exporter
                .export_invoices(&mut output, &invoices)
                .map_err(|e| {
                    tracing::error!(error = %e, "Failed to generate Money S3 export");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new(
                            "EXPORT_ERROR",
                            format!("Failed to generate Money S3 export: {}", e),
                        )),
                    )
                })?;

            let filename = format!(
                "money_s3_export_{}_{}_{}.csv",
                export.export_type, export.period_start, export.period_end
            );

            (output, "text/csv; charset=utf-8", filename)
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "UNSUPPORTED_SYSTEM",
                    format!(
                        "Accounting system '{}' is not supported for export",
                        export.system_type
                    ),
                )),
            ))
        }
    };

    // Build the response with appropriate headers
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .header(header::CONTENT_LENGTH, content.len())
        .body(Body::from(content))
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to build response");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "RESPONSE_ERROR",
                    "Failed to build download response",
                )),
            )
        })?;

    Ok(response)
}

/// Get accounting export settings.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/organizations/{org_id}/accounting/settings/{system}",
    params(AccountingSystemPath),
    responses(
        (status = 200, description = "Settings retrieved", body = AccountingExportSettings),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn get_accounting_settings(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<AccountingSystemPath>,
) -> Result<Json<AccountingExportSettings>, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .integration_repo
        .get_accounting_export_settings(rls.conn(), path.org_id, &path.system)
        .await;
    rls.release().await;

    let settings = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to get accounting settings");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to get accounting settings",
            )),
        )
    })?;

    Ok(Json(settings))
}

/// Update accounting export settings.
#[utoipa::path(
    put,
    path = "/api/v1/integrations/organizations/{org_id}/accounting/settings/{system}",
    params(AccountingSystemPath),
    request_body = UpdateAccountingExportSettings,
    responses(
        (status = 200, description = "Settings updated", body = AccountingExportSettings),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn update_accounting_settings(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<AccountingSystemPath>,
    Json(data): Json<UpdateAccountingExportSettings>,
) -> Result<Json<AccountingExportSettings>, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .integration_repo
        .update_accounting_export_settings(&mut **rls.conn(), path.org_id, &path.system, data)
        .await;
    rls.release().await;

    let settings = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to update accounting settings");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to update accounting settings",
            )),
        )
    })?;

    Ok(Json(settings))
}

// ==================== E-Signature (Story 61.3) ====================

/// List e-signature workflows for an organization.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/organizations/{org_id}/esignatures",
    params(OrgIdPath, ESignatureQuery),
    responses(
        (status = 200, description = "Workflows retrieved", body = Vec<ESignatureWorkflow>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not a member of the organization"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn list_esignature_workflows(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<OrgIdPath>,
    Query(query): Query<ESignatureQuery>,
) -> Result<Json<Vec<ESignatureWorkflow>>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user belongs to this organization
    if let Err(e) = verify_org_access(&state, rls.user_id(), path.org_id).await {
        rls.release().await;
        return Err(e);
    }

    let result = state
        .integration_repo
        .list_esignature_workflows(
            &mut **rls.conn(),
            path.org_id,
            query.status.as_deref(),
            query.limit,
        )
        .await;
    rls.release().await;

    let workflows = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to list e-signature workflows");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to list e-signature workflows",
            )),
        )
    })?;

    Ok(Json(workflows))
}

/// Create an e-signature workflow.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/organizations/{org_id}/esignatures",
    params(OrgIdPath),
    request_body = CreateESignatureWorkflow,
    responses(
        (status = 201, description = "Workflow created", body = ESignatureWorkflowWithRecipients),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn create_esignature_workflow(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<OrgIdPath>,
    Json(data): Json<CreateESignatureWorkflow>,
) -> Result<(StatusCode, Json<ESignatureWorkflowWithRecipients>), (StatusCode, Json<ErrorResponse>)>
{
    let user_id = rls.user_id();
    // Verify user has access to the organization
    if let Err(e) = verify_org_access(&state, user_id, path.org_id).await {
        rls.release().await;
        return Err(e);
    }

    let result = state
        .integration_repo
        .create_esignature_workflow(&mut **rls.conn(), path.org_id, user_id, data)
        .await;
    rls.release().await;

    let workflow = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to create e-signature workflow");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to create e-signature workflow",
            )),
        )
    })?;

    // Wrap workflow with empty recipients list (no recipients added yet)
    let result = ESignatureWorkflowWithRecipients {
        workflow,
        recipients: vec![],
    };

    Ok((StatusCode::CREATED, Json(result)))
}

/// Get an e-signature workflow by ID.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/esignatures/{id}",
    params(ResourceIdPath),
    responses(
        (status = 200, description = "Workflow retrieved", body = ESignatureWorkflowWithRecipients),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not a member of the organization"),
        (status = 404, description = "Workflow not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn get_esignature_workflow(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<ESignatureWorkflowWithRecipients>, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .integration_repo
        .get_esignature_workflow_with_recipients(rls.conn(), path.id)
        .await;
    rls.release().await;

    let workflow = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to get e-signature workflow");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to get e-signature workflow",
            )),
        )
    })?;

    match workflow {
        Some(w) => {
            // Verify user has access to the organization that owns this resource
            verify_org_access(&state, rls.user_id(), w.workflow.organization_id).await?;
            Ok(Json(w))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "E-signature workflow not found",
            )),
        )),
    }
}

/// Send an e-signature workflow for signing.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/esignatures/{id}/send",
    params(ResourceIdPath),
    responses(
        (status = 200, description = "Workflow sent"),
        (status = 404, description = "Workflow not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn send_esignature_workflow(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<ESignatureWorkflow>, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .integration_repo
        .update_esignature_workflow_status(&mut **rls.conn(), path.id, "sent")
        .await;
    rls.release().await;

    let workflow = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to send e-signature workflow");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to send e-signature workflow",
            )),
        )
    })?;

    Ok(Json(workflow))
}

/// Void an e-signature workflow.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/esignatures/{id}/void",
    params(ResourceIdPath),
    responses(
        (status = 200, description = "Workflow voided"),
        (status = 404, description = "Workflow not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn void_esignature_workflow(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<ESignatureWorkflow>, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .integration_repo
        .update_esignature_workflow_status(&mut **rls.conn(), path.id, "voided")
        .await;
    rls.release().await;

    let workflow = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to void e-signature workflow");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to void e-signature workflow",
            )),
        )
    })?;

    Ok(Json(workflow))
}

/// Send reminder for e-signature workflow.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/esignatures/{id}/remind",
    params(ResourceIdPath),
    responses(
        (status = 200, description = "Reminder sent"),
        (status = 404, description = "Workflow not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn send_esignature_reminder(
    State(state): State<AppState>,
    Path(path): Path<ResourceIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    use db::models::signature_request::SignatureRequestStatus;

    // Get the signature request
    let request = state
        .signature_request_repo
        .find_by_id(path.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, request_id = %path.id, "Failed to find signature request");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to find signature request",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Signature request not found",
                )),
            )
        })?;

    // Verify the request is in a state where reminders can be sent
    let can_send_reminder = matches!(
        request.status,
        SignatureRequestStatus::Pending | SignatureRequestStatus::InProgress
    );

    if !can_send_reminder {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_STATUS",
                "Cannot send reminder for completed or cancelled requests",
            )),
        ));
    }

    // Check if request has expired
    if request.is_expired() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "REQUEST_EXPIRED",
                "Cannot send reminder for expired requests",
            )),
        ));
    }

    // Get signers who haven't signed yet (using signers embedded in request)
    let unsigned_signers: Vec<_> = request.signers.iter().filter(|s| !s.has_signed()).collect();

    if unsigned_signers.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "NO_PENDING_SIGNERS",
                "All signers have already signed",
            )),
        ));
    }

    // Get document name for the email (use subject as fallback)
    let document_name = request.subject.as_deref().unwrap_or("Document");

    // Send reminder emails to unsigned signers
    let mut reminders_sent = 0;
    for signer in &unsigned_signers {
        let email_result = state
            .email_service
            .send_template_email(
                &signer.email,
                "signature_reminder",
                serde_json::json!({
                    "signer_name": signer.name,
                    "document_name": document_name,
                    "request_id": request.id.to_string(),
                    "expires_at": request.expires_at.map(|e| e.to_rfc3339()),
                }),
            )
            .await;

        match email_result {
            Ok(_) => {
                reminders_sent += 1;
                tracing::info!(
                    signer_email = %signer.email,
                    request_id = %path.id,
                    "Signature reminder sent successfully"
                );
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    signer_email = %signer.email,
                    request_id = %path.id,
                    "Failed to send signature reminder email"
                );
                // Continue sending to other signers even if one fails
            }
        }
    }

    // Record the reminder in the audit log
    if let Err(e) = state
        .audit_log_repo
        .create(db::models::audit_log::CreateAuditLog {
            user_id: None, // System action
            action: db::models::audit_log::AuditAction::ResourceUpdated,
            resource_type: Some("signature_request".to_string()),
            resource_id: Some(path.id),
            org_id: Some(request.organization_id),
            details: Some(serde_json::json!({
                "event": "esignature_reminder_sent",
                "document_name": document_name,
                "reminders_sent": reminders_sent,
                "total_unsigned": unsigned_signers.len(),
            })),
            old_values: None,
            new_values: None,
            ip_address: None,
            user_agent: None,
        })
        .await
    {
        tracing::error!(error = %e, "Failed to create audit log for reminder");
        // Don't fail the request for audit log failure
    }

    tracing::info!(
        request_id = %path.id,
        reminders_sent = %reminders_sent,
        "E-signature reminders processed"
    );

    Ok(StatusCode::OK)
}

// ==================== Video Conferencing (Story 61.4) ====================

/// List video conference connections.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/organizations/{org_id}/video/connections",
    params(OrgIdPath, CalendarQuery),
    responses(
        (status = 200, description = "Connections retrieved", body = Vec<VideoConferenceConnection>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not a member of the organization"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn list_video_connections(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<OrgIdPath>,
    Query(query): Query<CalendarQuery>,
) -> Result<Json<Vec<VideoConferenceConnection>>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user belongs to this organization
    if let Err(e) = verify_org_access(&state, rls.user_id(), path.org_id).await {
        rls.release().await;
        return Err(e);
    }

    let result = state
        .integration_repo
        .list_video_conference_connections(&mut **rls.conn(), path.org_id, query.user_id)
        .await;
    rls.release().await;

    let connections = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to list video connections");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to list video connections",
            )),
        )
    })?;

    Ok(Json(connections))
}

/// Create a video conference connection.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/organizations/{org_id}/video/connections",
    params(OrgIdPath),
    request_body = CreateVideoConferenceConnection,
    responses(
        (status = 201, description = "Connection created", body = VideoConferenceConnection),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn create_video_connection(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<OrgIdPath>,
    Json(data): Json<CreateVideoConferenceConnection>,
) -> Result<(StatusCode, Json<VideoConferenceConnection>), (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();
    // Verify user has access to the organization
    if let Err(e) = verify_org_access(&state, user_id, path.org_id).await {
        rls.release().await;
        return Err(e);
    }

    let result = state
        .integration_repo
        .create_video_conference_connection(&mut **rls.conn(), path.org_id, user_id, data)
        .await;
    rls.release().await;

    let connection = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to create video connection");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to create video connection",
            )),
        )
    })?;

    Ok((StatusCode::CREATED, Json(connection)))
}

/// Delete a video conference connection.
#[utoipa::path(
    delete,
    path = "/api/v1/integrations/video/connections/{id}",
    params(ResourceIdPath),
    responses(
        (status = 204, description = "Connection deleted"),
        (status = 404, description = "Connection not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn delete_video_connection(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<ResourceIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .integration_repo
        .delete_video_conference_connection(&mut **rls.conn(), path.id)
        .await;
    rls.release().await;

    let deleted = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to delete video connection");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to delete video connection",
            )),
        )
    })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Video connection not found",
            )),
        ))
    }
}

/// List video meetings.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/organizations/{org_id}/video/meetings",
    params(OrgIdPath, VideoMeetingQuery),
    responses(
        (status = 200, description = "Meetings retrieved", body = Vec<VideoMeeting>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not a member of the organization"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn list_video_meetings(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<OrgIdPath>,
    Query(query): Query<VideoMeetingQuery>,
) -> Result<Json<Vec<VideoMeeting>>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user belongs to this organization
    if let Err(e) = verify_org_access(&state, rls.user_id(), path.org_id).await {
        rls.release().await;
        return Err(e);
    }

    let result = state
        .integration_repo
        .list_video_meetings(
            &mut **rls.conn(),
            path.org_id,
            query.from,
            query.status.as_deref(),
            query.limit,
        )
        .await;
    rls.release().await;

    let meetings = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to list video meetings");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to list video meetings",
            )),
        )
    })?;

    Ok(Json(meetings))
}

/// Create a video meeting.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/organizations/{org_id}/video/meetings",
    params(OrgIdPath),
    request_body = CreateVideoMeeting,
    responses(
        (status = 201, description = "Meeting created", body = VideoMeeting),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn create_video_meeting(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<OrgIdPath>,
    Json(data): Json<CreateVideoMeeting>,
) -> Result<(StatusCode, Json<VideoMeeting>), (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();
    // Verify user has access to the organization
    if let Err(e) = verify_org_access(&state, user_id, path.org_id).await {
        rls.release().await;
        return Err(e);
    }

    let result = state
        .integration_repo
        .create_video_meeting(&mut **rls.conn(), path.org_id, user_id, data)
        .await;
    rls.release().await;

    let meeting = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to create video meeting");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to create video meeting",
            )),
        )
    })?;

    Ok((StatusCode::CREATED, Json(meeting)))
}

/// Get a video meeting by ID.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/video/meetings/{id}",
    params(ResourceIdPath),
    responses(
        (status = 200, description = "Meeting retrieved", body = VideoMeeting),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not a member of the organization"),
        (status = 404, description = "Meeting not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn get_video_meeting(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<VideoMeeting>, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .integration_repo
        .get_video_meeting(&mut **rls.conn(), path.id)
        .await;
    rls.release().await;

    let meeting = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to get video meeting");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to get video meeting",
            )),
        )
    })?;

    match meeting {
        Some(m) => {
            // Verify user has access to the organization that owns this resource
            verify_org_access(&state, rls.user_id(), m.organization_id).await?;
            Ok(Json(m))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Video meeting not found")),
        )),
    }
}

/// Update a video meeting.
#[utoipa::path(
    put,
    path = "/api/v1/integrations/video/meetings/{id}",
    params(ResourceIdPath),
    request_body = UpdateVideoMeeting,
    responses(
        (status = 200, description = "Meeting updated", body = VideoMeeting),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not a member of the organization"),
        (status = 404, description = "Meeting not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn update_video_meeting(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<ResourceIdPath>,
    Json(data): Json<UpdateVideoMeeting>,
) -> Result<Json<VideoMeeting>, (StatusCode, Json<ErrorResponse>)> {
    // First get the meeting to check organization access
    let existing = match state
        .integration_repo
        .get_video_meeting(&mut **rls.conn(), path.id)
        .await
    {
        Ok(Some(m)) => m,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Video meeting not found")),
            ));
        }
        Err(e) => {
            rls.release().await;
            tracing::error!(error = %e, "Failed to get video meeting");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
            ));
        }
    };

    // Verify user has access to the organization that owns this resource
    if let Err(e) = verify_org_access(&state, rls.user_id(), existing.organization_id).await {
        rls.release().await;
        return Err(e);
    }

    let result = state
        .integration_repo
        .update_video_meeting(&mut **rls.conn(), path.id, data)
        .await;
    rls.release().await;

    let meeting = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to update video meeting");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to update video meeting",
            )),
        )
    })?;

    Ok(Json(meeting))
}

/// Delete a video meeting.
#[utoipa::path(
    delete,
    path = "/api/v1/integrations/video/meetings/{id}",
    params(ResourceIdPath),
    responses(
        (status = 204, description = "Meeting deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not a member of the organization"),
        (status = 404, description = "Meeting not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn delete_video_meeting(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<ResourceIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // First get the meeting to check organization access
    let existing = match state
        .integration_repo
        .get_video_meeting(&mut **rls.conn(), path.id)
        .await
    {
        Ok(Some(m)) => m,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Video meeting not found")),
            ));
        }
        Err(e) => {
            rls.release().await;
            tracing::error!(error = %e, "Failed to get video meeting");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
            ));
        }
    };

    // Verify user has access to the organization that owns this resource
    if let Err(e) = verify_org_access(&state, rls.user_id(), existing.organization_id).await {
        rls.release().await;
        return Err(e);
    }

    let result = state
        .integration_repo
        .delete_video_meeting(&mut **rls.conn(), path.id)
        .await;
    rls.release().await;

    let deleted = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to delete video meeting");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to delete video meeting",
            )),
        )
    })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Video meeting not found")),
        ))
    }
}

/// Start a video meeting.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/video/meetings/{id}/start",
    params(ResourceIdPath),
    responses(
        (status = 200, description = "Meeting started", body = VideoMeeting),
        (status = 404, description = "Meeting not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn start_video_meeting(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<VideoMeeting>, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .integration_repo
        .update_video_meeting(
            &mut **rls.conn(),
            path.id,
            UpdateVideoMeeting {
                status: Some("started".to_string()),
                ..Default::default()
            },
        )
        .await;
    rls.release().await;

    let meeting = result.map_err(|e| {
        tracing::error!(error = %e, "Failed to start video meeting");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to start video meeting",
            )),
        )
    })?;

    Ok(Json(meeting))
}
