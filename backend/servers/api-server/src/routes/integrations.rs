//! External Integrations routes (Epic 61).
//!
//! Routes for calendar sync, accounting exports, e-signatures, video conferencing, and webhooks.

use api_core::{AuthUser, TenantExtractor};
use axum::{
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::Response,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{Duration, Utc};
use common::errors::ErrorResponse;
use db::models::{
    accounting_system, calendar_provider, esignature_provider, AccountingExport,
    AccountingExportSettings, CalendarConnection, CalendarSyncResult, CreateAccountingExport,
    CreateCalendarConnection, CreateESignatureWorkflow,
    CreateIntegrationCalendarEvent as CreateCalendarEvent, CreateVideoConferenceConnection,
    CreateVideoMeeting, CreateWebhookSubscription, ESignatureWorkflow,
    ESignatureWorkflowWithRecipients, IntegrationCalendarEvent as CalendarEvent,
    IntegrationStatistics, SyncCalendarRequest, TestWebhookRequest, TestWebhookResponse,
    UpdateAccountingExportSettings, UpdateCalendarConnection, UpdateVideoMeeting,
    UpdateWebhookSubscription, VideoConferenceConnection, VideoMeeting, WebhookDeliveryLog,
    WebhookDeliveryQuery, WebhookStatistics, WebhookSubscription,
};
use hmac::{Hmac, KeyInit, Mac};
use integrations::{
    // Epic 83: External Platform Integrations
    AirbnbClient,
    AirbnbOAuthConfig,
    BookingClient,
    // Epic 61: Integration Suite
    GoogleCalendarClient,
    MicrosoftCalendarClient,
    MoneyS3Exporter,
    OAuthConfig,
    PohodaExporter,
    PortalType,
};
use serde::Deserialize;
use sha2::Sha256;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::state::AppState;

// Type alias for HMAC-SHA256
type HmacSha256 = Hmac<Sha256>;

/// Create integrations router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Statistics
        .route("/organizations/{org_id}/stats", get(get_integration_stats))
        // Calendar routes (Story 61.1)
        .route(
            "/organizations/:org_id/calendars",
            get(list_calendar_connections),
        )
        .route(
            "/organizations/:org_id/calendars",
            post(create_calendar_connection),
        )
        .route("/calendars/{id}", get(get_calendar_connection))
        .route("/calendars/{id}", put(update_calendar_connection))
        .route("/calendars/{id}", delete(delete_calendar_connection))
        .route("/calendars/{id}/sync", post(sync_calendar))
        .route("/calendars/{id}/events", get(list_calendar_events))
        .route("/calendars/{id}/events", post(create_calendar_event))
        // Accounting routes (Story 61.2)
        .route(
            "/organizations/:org_id/accounting/exports",
            get(list_accounting_exports),
        )
        .route(
            "/organizations/:org_id/accounting/exports",
            post(create_accounting_export),
        )
        .route("/accounting/exports/{id}", get(get_accounting_export))
        .route(
            "/accounting/exports/:id/download",
            get(download_accounting_export),
        )
        .route(
            "/organizations/:org_id/accounting/settings/:system",
            get(get_accounting_settings),
        )
        .route(
            "/organizations/:org_id/accounting/settings/:system",
            put(update_accounting_settings),
        )
        // E-Signature routes (Story 61.3)
        .route(
            "/organizations/:org_id/esignatures",
            get(list_esignature_workflows),
        )
        .route(
            "/organizations/:org_id/esignatures",
            post(create_esignature_workflow),
        )
        .route("/esignatures/{id}", get(get_esignature_workflow))
        .route("/esignatures/{id}/send", post(send_esignature_workflow))
        .route("/esignatures/{id}/void", post(void_esignature_workflow))
        .route("/esignatures/{id}/remind", post(send_esignature_reminder))
        .route("/esignatures/webhook", post(esignature_webhook))
        // Video Conferencing routes (Story 61.4)
        .route(
            "/organizations/:org_id/video/connections",
            get(list_video_connections),
        )
        .route(
            "/organizations/:org_id/video/connections",
            post(create_video_connection),
        )
        .route("/video/connections/{id}", delete(delete_video_connection))
        .route(
            "/organizations/:org_id/video/meetings",
            get(list_video_meetings),
        )
        .route(
            "/organizations/:org_id/video/meetings",
            post(create_video_meeting),
        )
        .route("/video/meetings/{id}", get(get_video_meeting))
        .route("/video/meetings/{id}", put(update_video_meeting))
        .route("/video/meetings/{id}", delete(delete_video_meeting))
        .route("/video/meetings/{id}/start", post(start_video_meeting))
        // Webhook routes (Story 61.5)
        .route(
            "/organizations/:org_id/webhooks",
            get(list_webhook_subscriptions),
        )
        .route(
            "/organizations/:org_id/webhooks",
            post(create_webhook_subscription),
        )
        .route("/webhooks/{id}", get(get_webhook_subscription))
        .route("/webhooks/{id}", put(update_webhook_subscription))
        .route("/webhooks/{id}", delete(delete_webhook_subscription))
        .route("/webhooks/{id}/test", post(test_webhook))
        .route("/webhooks/{id}/logs", get(list_webhook_logs))
        .route("/webhooks/{id}/stats", get(get_webhook_stats))
        // ==================== Epic 83: External Platform Integrations ====================
        // Airbnb Integration (Story 83.1)
        .route(
            "/organizations/:org_id/airbnb/status",
            get(get_airbnb_status),
        )
        .route(
            "/organizations/:org_id/airbnb/connect",
            post(connect_airbnb),
        )
        .route(
            "/organizations/:org_id/airbnb/callback",
            get(airbnb_oauth_callback),
        )
        .route("/organizations/{org_id}/airbnb/sync", post(sync_airbnb))
        .route("/organizations/{org_id}/airbnb", delete(disconnect_airbnb))
        // Booking.com Integration (Story 83.2)
        .route(
            "/organizations/:org_id/booking/status",
            get(get_booking_status),
        )
        .route(
            "/organizations/:org_id/booking/connect",
            post(connect_booking),
        )
        .route("/organizations/{org_id}/booking/sync", post(sync_booking))
        .route(
            "/organizations/{org_id}/booking",
            delete(disconnect_booking),
        )
        .route("/booking/push", post(booking_push_notification))
        // Portal Webhooks (Story 83.3)
        .route(
            "/organizations/:org_id/portals",
            get(list_portal_connections),
        )
        .route(
            "/organizations/:org_id/portals",
            post(create_portal_connection),
        )
        .route("/portals/{id}", get(get_portal_connection))
        .route("/portals/{id}", delete(delete_portal_connection))
        .route(
            "/organizations/:org_id/portal-inquiries",
            get(list_portal_inquiries),
        )
        .route("/portal-inquiries/{id}", get(get_portal_inquiry))
        .route("/portal-inquiries/{id}/read", post(mark_inquiry_read))
        .route("/portal-inquiries/{id}/archive", post(archive_inquiry))
        // Portal webhook endpoint (public, no auth)
        .route(
            "/webhooks/portal/:connection_id",
            post(handle_portal_webhook),
        )
}

// ==================== Types ====================

/// Organization ID path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct OrgIdPath {
    pub org_id: Uuid,
}

/// Resource ID path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ResourceIdPath {
    pub id: Uuid,
}

/// Accounting system path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct AccountingSystemPath {
    pub org_id: Uuid,
    pub system: String,
}

/// Calendar query parameters.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct CalendarQuery {
    pub user_id: Option<Uuid>,
}

/// Calendar events query parameters.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct CalendarEventsQuery {
    pub from: Option<chrono::DateTime<chrono::Utc>>,
    pub to: Option<chrono::DateTime<chrono::Utc>>,
}

/// Accounting exports query parameters.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct AccountingExportQuery {
    pub system_type: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i32,
}

/// E-signature query parameters.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ESignatureQuery {
    pub status: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i32,
}

/// Video meetings query parameters.
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

// ==================== Authorization Helpers ====================

/// Verify user has access to the specified organization.
/// Returns Ok(()) if authorized, or an error response if not.
async fn verify_org_access(
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

/// Verify user has manager-level access to the organization.
fn verify_manager_role(tenant: &TenantExtractor) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
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

// ==================== E-Signature Webhook Verification ====================

/// Verify DocuSign webhook signature using HMAC-SHA256.
fn verify_docusign_signature(secret: &str, payload: &[u8], signature: &str) -> bool {
    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(payload);

    // DocuSign sends base64-encoded HMAC-SHA256 signature
    use base64::Engine as _;
    let Ok(signature_bytes) = base64::engine::general_purpose::STANDARD.decode(signature) else {
        return false;
    };

    // Use HMAC's verify_slice for constant-time comparison
    mac.verify_slice(&signature_bytes).is_ok()
}

/// Verify Adobe Sign webhook signature.
fn verify_adobe_sign_signature(client_secret: &str, payload: &[u8], signature: &str) -> bool {
    let Ok(mut mac) = HmacSha256::new_from_slice(client_secret.as_bytes()) else {
        return false;
    };
    mac.update(payload);

    // Adobe Sign sends hex-encoded HMAC-SHA256 signature
    let Ok(signature_bytes) = hex::decode(signature) else {
        return false;
    };

    // Use HMAC's verify_slice for constant-time comparison
    mac.verify_slice(&signature_bytes).is_ok()
}

/// Verify HelloSign webhook signature using event_hash.
fn verify_hellosign_signature(
    api_key: &str,
    event_time: &str,
    event_type: &str,
    event_hash: &str,
) -> bool {
    // HelloSign hash = HMAC-SHA256(api_key, event_time + event_type)
    let Ok(mut mac) = HmacSha256::new_from_slice(api_key.as_bytes()) else {
        return false;
    };
    mac.update(format!("{}{}", event_time, event_type).as_bytes());

    // HelloSign sends hex-encoded HMAC-SHA256 signature
    let Ok(signature_bytes) = hex::decode(event_hash) else {
        return false;
    };

    // Use HMAC's verify_slice for constant-time comparison
    mac.verify_slice(&signature_bytes).is_ok()
}

/// E-signature webhook payload with provider-specific fields.
#[derive(Debug, Deserialize)]
struct ESignatureWebhookPayload {
    /// Provider identifier (docusign, adobe_sign, hellosign)
    provider: Option<String>,
    /// Event type
    event_type: Option<String>,
    /// Envelope/document ID
    envelope_id: Option<String>,
    /// HelloSign-specific: event timestamp
    event_time: Option<String>,
    /// HelloSign-specific: event hash for verification
    event_hash: Option<String>,
    /// Additional event data
    #[serde(flatten)]
    data: serde_json::Value,
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
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
) -> Result<Json<IntegrationStatistics>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user belongs to this organization
    verify_org_access(&state, auth.user_id, path.org_id).await?;

    let stats = state
        .integration_repo
        .get_integration_statistics(path.org_id)
        .await
        .map_err(|e| {
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
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
    Query(query): Query<CalendarQuery>,
) -> Result<Json<Vec<CalendarConnection>>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user belongs to this organization
    verify_org_access(&state, auth.user_id, path.org_id).await?;

    let connections = state
        .integration_repo
        .list_calendar_connections(path.org_id, query.user_id)
        .await
        .map_err(|e| {
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
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
    Json(data): Json<CreateCalendarConnection>,
) -> Result<(StatusCode, Json<CalendarConnection>), (StatusCode, Json<ErrorResponse>)> {
    // Verify user has access to the organization
    verify_org_access(&state, auth.user_id, path.org_id).await?;

    let connection = state
        .integration_repo
        .create_calendar_connection(path.org_id, auth.user_id, data)
        .await
        .map_err(|e| {
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
    auth: AuthUser,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<CalendarConnection>, (StatusCode, Json<ErrorResponse>)> {
    // First get the connection to check organization access
    let connection = state
        .integration_repo
        .get_calendar_connection(path.id)
        .await
        .map_err(|e| {
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
            verify_org_access(&state, auth.user_id, c.organization_id).await?;
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
    auth: AuthUser,
    Path(path): Path<ResourceIdPath>,
    Json(data): Json<UpdateCalendarConnection>,
) -> Result<Json<CalendarConnection>, (StatusCode, Json<ErrorResponse>)> {
    // First get the connection to check organization access
    let existing = state
        .integration_repo
        .get_calendar_connection(path.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get calendar connection");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
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

    // Verify user has access to the organization that owns this resource
    verify_org_access(&state, auth.user_id, existing.organization_id).await?;

    let connection = state
        .integration_repo
        .update_calendar_connection(path.id, data)
        .await
        .map_err(|e| {
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
    auth: AuthUser,
    Path(path): Path<ResourceIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // First get the connection to check organization access
    let existing = state
        .integration_repo
        .get_calendar_connection(path.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get calendar connection");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
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

    // Verify user has access to the organization that owns this resource
    verify_org_access(&state, auth.user_id, existing.organization_id).await?;

    let deleted = state
        .integration_repo
        .delete_calendar_connection(path.id)
        .await
        .map_err(|e| {
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
    auth: AuthUser,
    Path(path): Path<ResourceIdPath>,
    Json(data): Json<SyncCalendarRequest>,
) -> Result<Json<CalendarSyncResult>, (StatusCode, Json<ErrorResponse>)> {
    // Get the calendar connection
    let connection = state
        .integration_repo
        .get_calendar_connection(path.id)
        .await
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
    verify_org_access(&state, auth.user_id, connection.organization_id).await?;

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

        // Update sync status to error in the background (fire and forget)
        let repo = state.integration_repo.clone();
        let connection_id = path.id;
        let error_msg = e.to_string();
        tokio::spawn(async move {
            let _ = repo
                .update_sync_status(connection_id, "error", Some(&error_msg))
                .await;
        });

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "SYNC_ERROR",
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

        // Use upsert to handle duplicates - if event with same source_id exists, skip
        match state
            .integration_repo
            .upsert_calendar_event(create_data)
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

    // Update sync status
    let _ = state
        .integration_repo
        .update_sync_status(path.id, "active", None)
        .await;

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
    Path(path): Path<ResourceIdPath>,
    Query(query): Query<CalendarEventsQuery>,
) -> Result<Json<Vec<CalendarEvent>>, (StatusCode, Json<ErrorResponse>)> {
    let events = state
        .integration_repo
        .list_calendar_events(path.id, query.from, query.to)
        .await
        .map_err(|e| {
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
    Path(_path): Path<ResourceIdPath>,
    Json(data): Json<CreateCalendarEvent>,
) -> Result<(StatusCode, Json<CalendarEvent>), (StatusCode, Json<ErrorResponse>)> {
    let event = state
        .integration_repo
        .create_calendar_event(data)
        .await
        .map_err(|e| {
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
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
    Query(query): Query<AccountingExportQuery>,
) -> Result<Json<Vec<AccountingExport>>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user belongs to this organization
    verify_org_access(&state, auth.user_id, path.org_id).await?;

    let exports = state
        .integration_repo
        .list_accounting_exports(path.org_id, query.system_type.as_deref(), query.limit)
        .await
        .map_err(|e| {
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
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
    Json(data): Json<CreateAccountingExport>,
) -> Result<(StatusCode, Json<AccountingExport>), (StatusCode, Json<ErrorResponse>)> {
    // Verify user has access to the organization
    verify_org_access(&state, auth.user_id, path.org_id).await?;

    let export = state
        .integration_repo
        .create_accounting_export(path.org_id, auth.user_id, data)
        .await
        .map_err(|e| {
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
    auth: AuthUser,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<AccountingExport>, (StatusCode, Json<ErrorResponse>)> {
    let export = state
        .integration_repo
        .get_accounting_export(path.id)
        .await
        .map_err(|e| {
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
            verify_org_access(&state, auth.user_id, e.organization_id).await?;
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
    auth: AuthUser,
    Path(path): Path<ResourceIdPath>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    // Get the export record
    let export = state
        .integration_repo
        .get_accounting_export(path.id)
        .await
        .map_err(|e| {
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
    verify_org_access(&state, auth.user_id, export.organization_id).await?;

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
    Path(path): Path<AccountingSystemPath>,
) -> Result<Json<AccountingExportSettings>, (StatusCode, Json<ErrorResponse>)> {
    let settings = state
        .integration_repo
        .get_accounting_export_settings(path.org_id, &path.system)
        .await
        .map_err(|e| {
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
    Path(path): Path<AccountingSystemPath>,
    Json(data): Json<UpdateAccountingExportSettings>,
) -> Result<Json<AccountingExportSettings>, (StatusCode, Json<ErrorResponse>)> {
    let settings = state
        .integration_repo
        .update_accounting_export_settings(path.org_id, &path.system, data)
        .await
        .map_err(|e| {
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
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
    Query(query): Query<ESignatureQuery>,
) -> Result<Json<Vec<ESignatureWorkflow>>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user belongs to this organization
    verify_org_access(&state, auth.user_id, path.org_id).await?;

    let workflows = state
        .integration_repo
        .list_esignature_workflows(path.org_id, query.status.as_deref(), query.limit)
        .await
        .map_err(|e| {
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
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
    Json(data): Json<CreateESignatureWorkflow>,
) -> Result<(StatusCode, Json<ESignatureWorkflowWithRecipients>), (StatusCode, Json<ErrorResponse>)>
{
    // Verify user has access to the organization
    verify_org_access(&state, auth.user_id, path.org_id).await?;

    let workflow = state
        .integration_repo
        .create_esignature_workflow(path.org_id, auth.user_id, data)
        .await
        .map_err(|e| {
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
    auth: AuthUser,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<ESignatureWorkflowWithRecipients>, (StatusCode, Json<ErrorResponse>)> {
    let workflow = state
        .integration_repo
        .get_esignature_workflow_with_recipients(path.id)
        .await
        .map_err(|e| {
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
            verify_org_access(&state, auth.user_id, w.workflow.organization_id).await?;
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
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<ESignatureWorkflow>, (StatusCode, Json<ErrorResponse>)> {
    let workflow = state
        .integration_repo
        .update_esignature_workflow_status(path.id, "sent")
        .await
        .map_err(|e| {
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
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<ESignatureWorkflow>, (StatusCode, Json<ErrorResponse>)> {
    let workflow = state
        .integration_repo
        .update_esignature_workflow_status(path.id, "voided")
        .await
        .map_err(|e| {
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

/// E-signature webhook endpoint.
///
/// This endpoint receives webhooks from e-signature providers (DocuSign, Adobe Sign, HelloSign).
/// It verifies the webhook signature before processing to prevent unauthorized requests.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/esignatures/webhook",
    request_body(content = String, description = "Provider-specific webhook payload"),
    responses(
        (status = 200, description = "Webhook processed"),
        (status = 400, description = "Invalid payload"),
        (status = 401, description = "Invalid signature"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Integrations"
)]
pub async fn esignature_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Parse the payload to determine the provider
    let payload: ESignatureWebhookPayload = serde_json::from_slice(&body).map_err(|e| {
        tracing::error!(error = %e, "Failed to parse e-signature webhook payload");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_PAYLOAD",
                "Invalid webhook payload",
            )),
        )
    })?;

    // Determine provider from payload or headers
    let provider = payload.provider.as_deref().unwrap_or_else(|| {
        // Try to determine from headers
        if headers.contains_key("x-docusign-signature-1") {
            esignature_provider::DOCUSIGN
        } else if headers.contains_key("x-adobesign-clientid") {
            esignature_provider::ADOBE_SIGN
        } else if payload.event_hash.is_some() {
            esignature_provider::HELLOSIGN
        } else {
            "unknown"
        }
    });

    // Verify webhook signature based on provider
    match provider {
        esignature_provider::DOCUSIGN => {
            // Get DocuSign webhook secret from environment
            let secret = std::env::var("DOCUSIGN_WEBHOOK_SECRET").unwrap_or_else(|_| String::new());
            if secret.is_empty() {
                tracing::warn!("DOCUSIGN_WEBHOOK_SECRET not configured");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "CONFIG_ERROR",
                        "Webhook verification not configured",
                    )),
                ));
            }

            // DocuSign sends signature in X-DocuSign-Signature-1 header
            let signature = headers
                .get("x-docusign-signature-1")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            if !verify_docusign_signature(&secret, &body, signature) {
                tracing::warn!("Invalid DocuSign webhook signature");
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse::new(
                        "INVALID_SIGNATURE",
                        "Invalid webhook signature",
                    )),
                ));
            }
        }
        esignature_provider::ADOBE_SIGN => {
            // Get Adobe Sign client secret from environment
            let client_secret =
                std::env::var("ADOBE_SIGN_CLIENT_SECRET").unwrap_or_else(|_| String::new());
            if client_secret.is_empty() {
                tracing::warn!("ADOBE_SIGN_CLIENT_SECRET not configured");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "CONFIG_ERROR",
                        "Webhook verification not configured",
                    )),
                ));
            }

            // Adobe Sign sends signature in X-AdobeSign-Signature header
            let signature = headers
                .get("x-adobesign-signature")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            if !verify_adobe_sign_signature(&client_secret, &body, signature) {
                tracing::warn!("Invalid Adobe Sign webhook signature");
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse::new(
                        "INVALID_SIGNATURE",
                        "Invalid webhook signature",
                    )),
                ));
            }
        }
        esignature_provider::HELLOSIGN => {
            // Get HelloSign API key from environment
            let api_key = std::env::var("HELLOSIGN_API_KEY").unwrap_or_else(|_| String::new());
            if api_key.is_empty() {
                tracing::warn!("HELLOSIGN_API_KEY not configured");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "CONFIG_ERROR",
                        "Webhook verification not configured",
                    )),
                ));
            }

            // HelloSign includes event_time, event_type, and event_hash in the payload
            let event_time = payload.event_time.as_deref().unwrap_or("");
            let event_type = payload.event_type.as_deref().unwrap_or("");
            let event_hash = payload.event_hash.as_deref().unwrap_or("");

            if !verify_hellosign_signature(&api_key, event_time, event_type, event_hash) {
                tracing::warn!("Invalid HelloSign webhook signature");
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse::new(
                        "INVALID_SIGNATURE",
                        "Invalid webhook signature",
                    )),
                ));
            }
        }
        _ => {
            tracing::warn!(provider = %provider, "Unknown e-signature provider");
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "UNKNOWN_PROVIDER",
                    "Unknown e-signature provider",
                )),
            ));
        }
    }

    // Signature verified, process the webhook
    tracing::info!(
        provider = %provider,
        event_type = ?payload.event_type,
        envelope_id = ?payload.envelope_id,
        "Processing verified e-signature webhook"
    );

    // Update workflow status based on event type
    if let (Some(envelope_id), Some(event_type)) = (
        payload.envelope_id.as_deref(),
        payload.event_type.as_deref(),
    ) {
        let new_status = match event_type {
            "envelope-completed" | "agreement_all_signed" | "signature_request_all_signed" => {
                Some("completed")
            }
            "envelope-voided" | "agreement_cancelled" | "signature_request_canceled" => {
                Some("voided")
            }
            "envelope-declined" | "agreement_rejected" | "signature_request_declined" => {
                Some("declined")
            }
            "envelope-sent" | "agreement_created" | "signature_request_sent" => Some("sent"),
            _ => None,
        };

        if let Some(status) = new_status {
            // Try to find and update the workflow by external envelope ID
            if let Err(e) = state
                .integration_repo
                .update_esignature_workflow_by_external_id(envelope_id, status)
                .await
            {
                tracing::warn!(
                    error = %e,
                    envelope_id = %envelope_id,
                    "Failed to update workflow status from webhook"
                );
            }
        }
    }

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
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
    Query(query): Query<CalendarQuery>,
) -> Result<Json<Vec<VideoConferenceConnection>>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user belongs to this organization
    verify_org_access(&state, auth.user_id, path.org_id).await?;

    let connections = state
        .integration_repo
        .list_video_conference_connections(path.org_id, query.user_id)
        .await
        .map_err(|e| {
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
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
    Json(data): Json<CreateVideoConferenceConnection>,
) -> Result<(StatusCode, Json<VideoConferenceConnection>), (StatusCode, Json<ErrorResponse>)> {
    // Verify user has access to the organization
    verify_org_access(&state, auth.user_id, path.org_id).await?;

    let connection = state
        .integration_repo
        .create_video_conference_connection(path.org_id, auth.user_id, data)
        .await
        .map_err(|e| {
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
    Path(path): Path<ResourceIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state
        .integration_repo
        .delete_video_conference_connection(path.id)
        .await
        .map_err(|e| {
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
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
    Query(query): Query<VideoMeetingQuery>,
) -> Result<Json<Vec<VideoMeeting>>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user belongs to this organization
    verify_org_access(&state, auth.user_id, path.org_id).await?;

    let meetings = state
        .integration_repo
        .list_video_meetings(
            path.org_id,
            query.from,
            query.status.as_deref(),
            query.limit,
        )
        .await
        .map_err(|e| {
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
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
    Json(data): Json<CreateVideoMeeting>,
) -> Result<(StatusCode, Json<VideoMeeting>), (StatusCode, Json<ErrorResponse>)> {
    // Verify user has access to the organization
    verify_org_access(&state, auth.user_id, path.org_id).await?;

    let meeting = state
        .integration_repo
        .create_video_meeting(path.org_id, auth.user_id, data)
        .await
        .map_err(|e| {
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
    auth: AuthUser,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<VideoMeeting>, (StatusCode, Json<ErrorResponse>)> {
    let meeting = state
        .integration_repo
        .get_video_meeting(path.id)
        .await
        .map_err(|e| {
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
            verify_org_access(&state, auth.user_id, m.organization_id).await?;
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
    auth: AuthUser,
    Path(path): Path<ResourceIdPath>,
    Json(data): Json<UpdateVideoMeeting>,
) -> Result<Json<VideoMeeting>, (StatusCode, Json<ErrorResponse>)> {
    // First get the meeting to check organization access
    let existing = state
        .integration_repo
        .get_video_meeting(path.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get video meeting");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Video meeting not found")),
            )
        })?;

    // Verify user has access to the organization that owns this resource
    verify_org_access(&state, auth.user_id, existing.organization_id).await?;

    let meeting = state
        .integration_repo
        .update_video_meeting(path.id, data)
        .await
        .map_err(|e| {
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
    auth: AuthUser,
    Path(path): Path<ResourceIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // First get the meeting to check organization access
    let existing = state
        .integration_repo
        .get_video_meeting(path.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get video meeting");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Video meeting not found")),
            )
        })?;

    // Verify user has access to the organization that owns this resource
    verify_org_access(&state, auth.user_id, existing.organization_id).await?;

    let deleted = state
        .integration_repo
        .delete_video_meeting(path.id)
        .await
        .map_err(|e| {
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
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<VideoMeeting>, (StatusCode, Json<ErrorResponse>)> {
    let meeting = state
        .integration_repo
        .update_video_meeting(
            path.id,
            UpdateVideoMeeting {
                status: Some("started".to_string()),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| {
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

// ==================== Webhooks (Story 61.5) ====================

/// List webhook subscriptions for an organization.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/organizations/{org_id}/webhooks",
    params(OrgIdPath),
    responses(
        (status = 200, description = "Subscriptions retrieved", body = Vec<WebhookSubscription>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not a member of the organization"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn list_webhook_subscriptions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
) -> Result<Json<Vec<WebhookSubscription>>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user belongs to this organization
    verify_org_access(&state, auth.user_id, path.org_id).await?;

    let subscriptions = state
        .integration_repo
        .list_webhook_subscriptions(path.org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list webhook subscriptions");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to list webhook subscriptions",
                )),
            )
        })?;

    Ok(Json(subscriptions))
}

/// Create a webhook subscription.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/organizations/{org_id}/webhooks",
    params(OrgIdPath),
    request_body = CreateWebhookSubscription,
    responses(
        (status = 201, description = "Subscription created", body = WebhookSubscription),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn create_webhook_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
    Json(data): Json<CreateWebhookSubscription>,
) -> Result<(StatusCode, Json<WebhookSubscription>), (StatusCode, Json<ErrorResponse>)> {
    // Verify user has access to the organization
    verify_org_access(&state, auth.user_id, path.org_id).await?;

    // Determine if running in production mode
    let is_production = std::env::var("RUST_ENV")
        .map(|v| v == "production")
        .unwrap_or(false);

    let subscription = state
        .integration_repo
        .create_webhook_subscription(path.org_id, auth.user_id, data, is_production)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create webhook subscription");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to create webhook subscription",
                )),
            )
        })?;

    Ok((StatusCode::CREATED, Json(subscription)))
}

/// Get a webhook subscription by ID.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/webhooks/{id}",
    params(ResourceIdPath),
    responses(
        (status = 200, description = "Subscription retrieved", body = WebhookSubscription),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not a member of the organization"),
        (status = 404, description = "Subscription not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn get_webhook_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<WebhookSubscription>, (StatusCode, Json<ErrorResponse>)> {
    let subscription = state
        .integration_repo
        .get_webhook_subscription(path.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get webhook subscription");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get webhook subscription",
                )),
            )
        })?;

    match subscription {
        Some(s) => {
            // Verify user has access to the organization that owns this resource
            verify_org_access(&state, auth.user_id, s.organization_id).await?;
            Ok(Json(s))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Webhook subscription not found",
            )),
        )),
    }
}

/// Update a webhook subscription.
#[utoipa::path(
    put,
    path = "/api/v1/integrations/webhooks/{id}",
    params(ResourceIdPath),
    request_body = UpdateWebhookSubscription,
    responses(
        (status = 200, description = "Subscription updated", body = WebhookSubscription),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not a member of the organization"),
        (status = 404, description = "Subscription not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn update_webhook_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<ResourceIdPath>,
    Json(data): Json<UpdateWebhookSubscription>,
) -> Result<Json<WebhookSubscription>, (StatusCode, Json<ErrorResponse>)> {
    // First get the subscription to check organization access
    let existing = state
        .integration_repo
        .get_webhook_subscription(path.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get webhook subscription");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Webhook subscription not found",
                )),
            )
        })?;

    // Verify user has access to the organization that owns this resource
    verify_org_access(&state, auth.user_id, existing.organization_id).await?;

    // Determine if running in production mode, consistent with creation logic
    let is_production = std::env::var("RUST_ENV")
        .map(|v| v == "production")
        .unwrap_or(false);

    let subscription = state
        .integration_repo
        .update_webhook_subscription(path.id, data, is_production)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update webhook subscription");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to update webhook subscription",
                )),
            )
        })?;

    Ok(Json(subscription))
}

/// Delete a webhook subscription.
#[utoipa::path(
    delete,
    path = "/api/v1/integrations/webhooks/{id}",
    params(ResourceIdPath),
    responses(
        (status = 204, description = "Subscription deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not a member of the organization"),
        (status = 404, description = "Subscription not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn delete_webhook_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<ResourceIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // First get the subscription to check organization access
    let existing = state
        .integration_repo
        .get_webhook_subscription(path.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get webhook subscription");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Webhook subscription not found",
                )),
            )
        })?;

    // Verify user has access to the organization that owns this resource
    verify_org_access(&state, auth.user_id, existing.organization_id).await?;

    let deleted = state
        .integration_repo
        .delete_webhook_subscription(path.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to delete webhook subscription");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to delete webhook subscription",
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
                "Webhook subscription not found",
            )),
        ))
    }
}

/// Test a webhook subscription.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/webhooks/{id}/test",
    params(ResourceIdPath),
    request_body = TestWebhookRequest,
    responses(
        (status = 200, description = "Test completed", body = TestWebhookResponse),
        (status = 404, description = "Subscription not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn test_webhook(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<ResourceIdPath>,
    Json(data): Json<TestWebhookRequest>,
) -> Result<Json<TestWebhookResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Get the webhook subscription
    let subscription = state
        .integration_repo
        .get_webhook_subscription(path.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get webhook subscription");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get webhook subscription",
                )),
            )
        })?;

    let subscription = match subscription {
        Some(s) => s,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Webhook subscription not found",
                )),
            ))
        }
    };

    // Verify user has access to the organization that owns this webhook
    verify_org_access(&state, auth.user_id, subscription.organization_id).await?;

    // Build the test payload
    let test_payload = data.payload.unwrap_or_else(|| {
        serde_json::json!({
            "event": data.event_type,
            "test": true,
            "timestamp": Utc::now().to_rfc3339(),
            "data": {
                "message": "This is a test webhook delivery"
            }
        })
    });

    // Create HTTP client with security safeguards:
    // - No redirect following to prevent open redirects/SSRF
    // - Identifies the service in logs
    // - Limits connection pooling
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("PropertyManagement-Webhook-Test/1.0")
        .pool_max_idle_per_host(0)
        .build()
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create HTTP client");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "CLIENT_ERROR",
                    "Failed to create HTTP client",
                )),
            )
        })?;

    // Build the request
    let mut request = client.post(&subscription.url).json(&test_payload);

    // Add custom headers if configured, with security blocklist
    // These headers could be used for SSRF or security bypass
    const BLOCKED_HEADERS: &[&str] = &[
        "host",
        "authorization",
        "cookie",
        "x-forwarded-for",
        "x-real-ip",
        "x-forwarded-host",
        "x-forwarded-proto",
    ];

    if let Some(headers) = &subscription.headers {
        if let Some(headers_obj) = headers.as_object() {
            for (key, value) in headers_obj {
                // Skip blocked headers that could be used for security bypass
                if BLOCKED_HEADERS.contains(&key.to_lowercase().as_str()) {
                    tracing::warn!(header = %key, "Blocked webhook header injection attempt");
                    continue;
                }
                if let Some(value_str) = value.as_str() {
                    request = request.header(key, value_str);
                }
            }
        }
    }

    // Add signature header if secret is configured (using HMAC-SHA256 for security)
    if let Some(secret) = &subscription.secret {
        let payload_str = serde_json::to_string(&test_payload).unwrap_or_default();
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).map_err(|e| {
            tracing::error!(error = ?e, "Failed to create HMAC for webhook test signature");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "CRYPTO_ERROR",
                    "Failed to compute webhook signature",
                )),
            )
        })?;
        mac.update(payload_str.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());
        request = request.header("X-Webhook-Signature", format!("sha256={}", signature));
    }

    // Add standard webhook headers
    request = request
        .header("Content-Type", "application/json")
        .header("X-Webhook-Event", &data.event_type)
        .header("X-Webhook-Test", "true")
        .header("X-Webhook-ID", Uuid::new_v4().to_string());

    // Measure response time
    let start_time = std::time::Instant::now();

    // Send the request
    let response_result = request.send().await;
    let response_time_ms = start_time.elapsed().as_millis() as i32;

    // Process the result
    match response_result {
        Ok(response) => {
            let status_code = response.status().as_u16() as i32;
            let success = response.status().is_success();

            // Try to get error message from response body if not successful
            // Sanitize to prevent leaking sensitive information from the target endpoint
            let error = if !success {
                let body = response.text().await.ok();
                body.map(|b| {
                    // Sanitize and truncate: only keep first few lines and cap length
                    let sanitized = b.lines().take(5).collect::<Vec<_>>().join("\n");
                    if sanitized.len() > 500 {
                        format!("{}...", &sanitized[..500])
                    } else {
                        sanitized
                    }
                })
            } else {
                None
            };

            Ok(Json(TestWebhookResponse {
                success,
                status_code: Some(status_code),
                response_time_ms: Some(response_time_ms),
                error,
            }))
        }
        Err(e) => {
            // Log detailed error for debugging but return sanitized message to client
            tracing::warn!(error = ?e, "Webhook test request failed");

            let error_message = if e.is_timeout() {
                "Request timed out after 30 seconds".to_string()
            } else if e.is_connect() {
                "Failed to connect to webhook URL".to_string()
            } else {
                "Request failed while testing webhook".to_string()
            };

            Ok(Json(TestWebhookResponse {
                success: false,
                status_code: None,
                response_time_ms: Some(response_time_ms),
                error: Some(error_message),
            }))
        }
    }
}

/// List webhook delivery logs.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/webhooks/{id}/logs",
    params(ResourceIdPath),
    responses(
        (status = 200, description = "Logs retrieved", body = Vec<WebhookDeliveryLog>),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn list_webhook_logs(
    State(state): State<AppState>,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<Vec<WebhookDeliveryLog>>, (StatusCode, Json<ErrorResponse>)> {
    let logs = state
        .integration_repo
        .list_webhook_delivery_logs(WebhookDeliveryQuery {
            subscription_id: Some(path.id),
            event_type: None,
            status: None,
            from_date: None,
            to_date: None,
            limit: Some(100),
            offset: None,
        })
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list webhook logs");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to list webhook logs",
                )),
            )
        })?;

    Ok(Json(logs))
}

/// Get webhook statistics.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/webhooks/{id}/stats",
    params(ResourceIdPath),
    responses(
        (status = 200, description = "Statistics retrieved", body = WebhookStatistics),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn get_webhook_stats(
    State(state): State<AppState>,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<WebhookStatistics>, (StatusCode, Json<ErrorResponse>)> {
    let stats = state
        .integration_repo
        .get_webhook_statistics(path.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get webhook statistics");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get webhook statistics",
                )),
            )
        })?;

    Ok(Json(stats))
}

// ==================== Epic 83: External Platform Integrations ====================

// ==================== Airbnb Integration Types ====================

/// Airbnb connection status response.
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct AirbnbStatusResponse {
    /// Whether Airbnb is connected.
    pub connected: bool,
    /// External account ID if connected.
    pub external_account_id: Option<String>,
    /// Last sync timestamp.
    pub last_sync_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Last sync error if any.
    pub sync_error: Option<String>,
    /// Number of synced listings.
    pub listings_count: i32,
    /// Number of synced reservations.
    pub reservations_count: i32,
}

/// Airbnb connect request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AirbnbConnectRequest {
    /// The redirect URL after OAuth.
    pub redirect_uri: Option<String>,
}

/// Airbnb connect response.
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct AirbnbConnectResponse {
    /// OAuth authorization URL to redirect to.
    pub auth_url: String,
    /// State parameter for CSRF protection.
    pub state: String,
}

/// OAuth callback query parameters.
#[derive(Debug, Deserialize, IntoParams)]
pub struct OAuthCallbackQuery {
    /// Authorization code.
    pub code: String,
    /// State parameter for CSRF protection.
    pub state: String,
}

/// Sync response.
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct SyncResponse {
    /// Whether sync was successful.
    pub success: bool,
    /// Number of items synced.
    pub items_synced: i32,
    /// Sync timestamp.
    pub synced_at: chrono::DateTime<chrono::Utc>,
    /// Error message if any.
    pub error: Option<String>,
}

// ==================== Airbnb Integration Handlers ====================

/// Get Airbnb connection status.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/organizations/{org_id}/airbnb/status",
    params(OrgIdPath),
    responses(
        (status = 200, description = "Airbnb status retrieved", body = AirbnbStatusResponse),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Airbnb"
)]
pub async fn get_airbnb_status(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
) -> Result<Json<AirbnbStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Getting Airbnb status"
    );

    let rental_repo = &state.rental_repo;

    // Get aggregated Airbnb status
    let (connected_count, listings_count, last_sync_at, sync_error) = rental_repo
        .get_airbnb_status(path.org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get Airbnb status");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get Airbnb status",
                )),
            )
        })?;

    // Get external account ID from first connection
    let connection = rental_repo
        .find_airbnb_connection_by_org(path.org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to find Airbnb connection");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to find connection",
                )),
            )
        })?;

    let external_account_id = connection.and_then(|c| c.external_property_id);

    // Get reservations count
    let reservations_count = rental_repo
        .count_airbnb_reservations(path.org_id)
        .await
        .unwrap_or(0);

    Ok(Json(AirbnbStatusResponse {
        connected: connected_count > 0,
        external_account_id,
        last_sync_at,
        sync_error,
        listings_count: listings_count as i32,
        reservations_count: reservations_count as i32,
    }))
}

/// Initiate Airbnb OAuth connection.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/organizations/{org_id}/airbnb/connect",
    params(OrgIdPath),
    request_body = AirbnbConnectRequest,
    responses(
        (status = 200, description = "OAuth URL generated", body = AirbnbConnectResponse),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Airbnb"
)]
pub async fn connect_airbnb(
    State(_state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
    Json(request): Json<AirbnbConnectRequest>,
) -> Result<Json<AirbnbConnectResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Initiating Airbnb OAuth connection"
    );

    // Get OAuth config from environment (would be in settings in production)
    let client_id = std::env::var("AIRBNB_CLIENT_ID").unwrap_or_default();
    let client_secret = std::env::var("AIRBNB_CLIENT_SECRET").unwrap_or_default();
    let redirect_uri = request
        .redirect_uri
        .unwrap_or_else(|| std::env::var("AIRBNB_REDIRECT_URI").unwrap_or_default());

    if client_id.is_empty() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new(
                "NOT_CONFIGURED",
                "Airbnb integration is not configured",
            )),
        ));
    }

    let client = AirbnbClient::new(AirbnbOAuthConfig {
        client_id,
        client_secret,
        redirect_uri,
    });

    // Generate state for CSRF protection (includes org_id for callback verification)
    let oauth_state = format!("{}:{}", path.org_id, uuid::Uuid::new_v4());

    // Store state in session repository for callback verification
    // Note: In production, this could use Redis for distributed state management
    // For now, we embed the org_id in the state parameter itself for stateless verification
    tracing::debug!(oauth_state = %oauth_state, "Generated OAuth state parameter");

    let auth_url = client.generate_auth_url(&oauth_state);

    Ok(Json(AirbnbConnectResponse {
        auth_url,
        state: oauth_state,
    }))
}

/// Airbnb OAuth callback response.
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct AirbnbCallbackResponse {
    /// Whether the connection was successful.
    pub success: bool,
    /// Connection ID.
    pub connection_id: Option<Uuid>,
    /// Message describing the result.
    pub message: String,
    /// Listing count retrieved after connection.
    pub listings_count: Option<i32>,
}

/// Handle Airbnb OAuth callback.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/organizations/{org_id}/airbnb/callback",
    params(OrgIdPath, OAuthCallbackQuery),
    responses(
        (status = 200, description = "OAuth callback processed", body = AirbnbCallbackResponse),
        (status = 400, description = "Invalid callback"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Airbnb"
)]
pub async fn airbnb_oauth_callback(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<Json<AirbnbCallbackResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Processing Airbnb OAuth callback"
    );

    // Verify state parameter
    if query.state.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_STATE",
                "Invalid state parameter",
            )),
        ));
    }

    // Verify state contains valid org_id (stateless verification)
    let state_parts: Vec<&str> = query.state.split(':').collect();
    if state_parts.len() < 2 {
        tracing::warn!(state = %query.state, "Invalid OAuth state format");
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_STATE",
                "Invalid OAuth state format",
            )),
        ));
    }

    // Verify org_id in state matches the path
    let state_org_id = Uuid::parse_str(state_parts[0]).ok();
    if state_org_id != Some(path.org_id) {
        tracing::warn!(
            state_org = ?state_org_id,
            path_org = %path.org_id,
            "OAuth state org_id mismatch"
        );
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "STATE_MISMATCH",
                "OAuth state does not match organization",
            )),
        ));
    }

    // Get OAuth config
    let client_id = std::env::var("AIRBNB_CLIENT_ID").unwrap_or_default();
    let client_secret = std::env::var("AIRBNB_CLIENT_SECRET").unwrap_or_default();
    let redirect_uri = std::env::var("AIRBNB_REDIRECT_URI").unwrap_or_default();

    if client_id.is_empty() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new(
                "NOT_CONFIGURED",
                "Airbnb integration is not configured",
            )),
        ));
    }

    let client = AirbnbClient::new(AirbnbOAuthConfig {
        client_id,
        client_secret,
        redirect_uri,
    });

    // Exchange code for tokens
    let tokens = client.exchange_code(&query.code).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to exchange Airbnb authorization code");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "TOKEN_EXCHANGE_FAILED",
                "Failed to exchange authorization code for tokens",
            )),
        )
    })?;

    // Encrypt tokens before storage
    let crypto = integrations::IntegrationCrypto::from_env().ok();
    let (encrypted_access, encrypted_refresh) = match &crypto {
        Some(c) => (
            c.encrypt(&tokens.access_token)
                .unwrap_or(tokens.access_token.clone()),
            tokens
                .refresh_token
                .as_ref()
                .map(|rt| c.encrypt(rt).unwrap_or(rt.clone())),
        ),
        None => (tokens.access_token.clone(), tokens.refresh_token.clone()),
    };

    // Store tokens in database
    let rental_repo = &state.rental_repo;
    let connection = rental_repo
        .upsert_airbnb_connection(
            path.org_id,
            None, // Will be updated when listing is mapped
            &encrypted_access,
            encrypted_refresh.as_deref(),
            tokens.expires_at,
            None, // External account ID will be fetched later
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to store Airbnb connection");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to store connection",
                )),
            )
        })?;

    // Try to fetch listings to get external account info
    let listings_count = match client.fetch_listings(&tokens.access_token).await {
        Ok(listings) => {
            // Store first listing ID if available
            if let Some(first_listing) = listings.first() {
                let _ = rental_repo
                    .update_airbnb_listing_mapping(
                        connection.id,
                        &first_listing.id,
                        Some(&format!(
                            "https://www.airbnb.com/rooms/{}",
                            first_listing.id
                        )),
                    )
                    .await;
            }
            Some(listings.len() as i32)
        }
        Err(e) => {
            tracing::warn!(error = %e, "Failed to fetch Airbnb listings after OAuth");
            None
        }
    };

    // Note: Webhook subscription setup would be done here if Airbnb API supports it
    // Currently, Airbnb webhooks are configured through their partner dashboard
    let webhook_url = std::env::var("AIRBNB_WEBHOOK_URL").ok();
    if webhook_url.is_some() {
        tracing::info!(
            "Airbnb webhook URL configured, webhook setup should be done via partner dashboard"
        );
    }

    tracing::info!(
        connection_id = %connection.id,
        org_id = %path.org_id,
        listings_count = ?listings_count,
        "Airbnb OAuth completed successfully"
    );

    Ok(Json(AirbnbCallbackResponse {
        success: true,
        connection_id: Some(connection.id),
        message: "Airbnb connected successfully".to_string(),
        listings_count,
    }))
}

/// Trigger Airbnb sync.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/organizations/{org_id}/airbnb/sync",
    params(OrgIdPath),
    responses(
        (status = 200, description = "Sync initiated", body = SyncResponse),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Airbnb"
)]
pub async fn sync_airbnb(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
) -> Result<Json<SyncResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Syncing Airbnb"
    );

    let rental_repo = &state.rental_repo;

    // Get Airbnb connection for org
    let connection = rental_repo
        .find_airbnb_connection_by_org(path.org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to find Airbnb connection");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check connection",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "No Airbnb connection found",
                )),
            )
        })?;

    // Check if we have valid tokens
    let access_token = connection.access_token.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "Airbnb not authorized",
            )),
        )
    })?;

    // Create Airbnb client
    let oauth_config = AirbnbOAuthConfig {
        client_id: std::env::var("AIRBNB_CLIENT_ID").unwrap_or_default(),
        client_secret: std::env::var("AIRBNB_CLIENT_SECRET").unwrap_or_default(),
        redirect_uri: std::env::var("AIRBNB_REDIRECT_URI").unwrap_or_default(),
    };
    let client = AirbnbClient::new(oauth_config);

    // Fetch listings using the access token
    let listings = client.fetch_listings(&access_token).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to fetch Airbnb listings");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "SYNC_ERROR",
                format!("Failed to sync: {}", e),
            )),
        )
    })?;

    // Fetch reservations for each listing
    let mut total_items = listings.len();
    for listing in &listings {
        match client
            .fetch_reservations(&access_token, &listing.id, None, None)
            .await
        {
            Ok(reservations) => {
                total_items += reservations.len();
                tracing::info!(
                    listing_id = %listing.id,
                    reservation_count = reservations.len(),
                    "Fetched reservations for listing"
                );
            }
            Err(e) => {
                tracing::warn!(
                    listing_id = %listing.id,
                    error = %e,
                    "Failed to fetch reservations for listing"
                );
            }
        }
    }

    // Update last sync timestamp
    let _ = rental_repo
        .update_connection_last_sync(connection.id, Utc::now())
        .await;

    tracing::info!(
        org_id = %path.org_id,
        listings_count = listings.len(),
        total_items = total_items,
        "Airbnb sync completed"
    );

    Ok(Json(SyncResponse {
        success: true,
        items_synced: total_items as i32,
        synced_at: Utc::now(),
        error: None,
    }))
}

/// Disconnect Airbnb.
#[utoipa::path(
    delete,
    path = "/api/v1/integrations/organizations/{org_id}/airbnb",
    params(OrgIdPath),
    responses(
        (status = 204, description = "Airbnb disconnected"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "No connection found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Airbnb"
)]
pub async fn disconnect_airbnb(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Disconnecting Airbnb"
    );

    let rental_repo = &state.rental_repo;

    // First check if there's a connection to disconnect
    let connection = rental_repo
        .find_airbnb_connection_by_org(path.org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to find Airbnb connection");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check connection",
                )),
            )
        })?;

    if connection.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "No Airbnb connection found for this organization",
            )),
        ));
    }

    // Note: Airbnb API doesn't provide a token revocation endpoint
    // We simply clear the tokens locally. The tokens will naturally expire.
    // If needed, user can revoke access through Airbnb's account settings.
    if let Some(conn) = &connection {
        if conn.access_token.is_some() {
            tracing::info!(
                connection_id = %conn.id,
                "Clearing Airbnb tokens (API revocation not available)"
            );
        }
    }

    // Revoke locally (clear tokens from database)
    let revoked_count = rental_repo
        .revoke_airbnb_connection(path.org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to revoke Airbnb connection");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to disconnect")),
            )
        })?;

    tracing::info!(
        org_id = %path.org_id,
        revoked_count = revoked_count,
        "Airbnb disconnected successfully"
    );

    Ok(StatusCode::NO_CONTENT)
}

// ==================== Booking.com Integration Types ====================

/// Booking.com connection status response.
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct BookingStatusResponse {
    /// Whether Booking.com is connected.
    pub connected: bool,
    /// Hotel ID if connected.
    pub hotel_id: Option<String>,
    /// Last sync timestamp.
    pub last_sync_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Last sync error if any.
    pub sync_error: Option<String>,
    /// Number of synced properties.
    pub properties_count: i32,
    /// Number of synced reservations.
    pub reservations_count: i32,
}

/// Booking.com connect request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct BookingConnectRequest {
    /// Booking.com hotel ID.
    pub hotel_id: String,
    /// API username.
    pub username: String,
    /// API password.
    pub password: String,
}

// ==================== Booking.com Integration Handlers ====================

/// Get Booking.com connection status.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/organizations/{org_id}/booking/status",
    params(OrgIdPath),
    responses(
        (status = 200, description = "Booking.com status retrieved", body = BookingStatusResponse),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Booking.com"
)]
pub async fn get_booking_status(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
) -> Result<Json<BookingStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Getting Booking.com status"
    );

    let rental_repo = &state.rental_repo;

    // Check for Booking.com connection
    let connection = rental_repo
        .find_booking_connection_by_org(path.org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check Booking.com connection");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check connection",
                )),
            )
        })?;

    match connection {
        Some(conn) => {
            // Get counts (in a real implementation, we'd query the database)
            Ok(Json(BookingStatusResponse {
                connected: conn.is_active,
                hotel_id: conn.external_property_id.clone(),
                last_sync_at: conn.last_sync_at,
                sync_error: conn.sync_error.clone(),
                properties_count: if conn.is_active { 1 } else { 0 },
                reservations_count: 0, // Would query reservation count
            }))
        }
        None => Ok(Json(BookingStatusResponse {
            connected: false,
            hotel_id: None,
            last_sync_at: None,
            sync_error: None,
            properties_count: 0,
            reservations_count: 0,
        })),
    }
}

/// Connect to Booking.com.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/organizations/{org_id}/booking/connect",
    params(OrgIdPath),
    request_body = BookingConnectRequest,
    responses(
        (status = 200, description = "Connected to Booking.com"),
        (status = 400, description = "Invalid credentials"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Booking.com"
)]
pub async fn connect_booking(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
    Json(request): Json<BookingConnectRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        hotel_id = %request.hotel_id,
        "Connecting to Booking.com"
    );

    let rental_repo = &state.rental_repo;

    // Validate credentials by trying to fetch the property
    let credentials = integrations::BookingCredentials::new(
        request.hotel_id.clone(),
        request.username.clone(),
        request.password.clone(),
    );
    let client = BookingClient::new(credentials);

    // Test the connection by fetching the property
    match client.fetch_property(&request.hotel_id).await {
        Ok(property) => {
            tracing::info!(
                hotel_id = %request.hotel_id,
                property_name = %property.name,
                "Booking.com credentials validated"
            );
        }
        Err(e) => {
            tracing::warn!(
                hotel_id = %request.hotel_id,
                error = %e,
                "Failed to validate Booking.com credentials"
            );
            // Don't fail - credentials might be valid but property info unavailable
        }
    }

    // Store the connection
    let _connection = rental_repo
        .create_or_update_booking_connection(
            path.org_id,
            &request.hotel_id,
            &request.username,
            &request.password,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to store Booking.com connection");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to store connection",
                )),
            )
        })?;

    tracing::info!(
        org_id = %path.org_id,
        hotel_id = %request.hotel_id,
        "Booking.com connected successfully"
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Booking.com connected successfully",
        "hotel_id": request.hotel_id
    })))
}

/// Trigger Booking.com sync.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/organizations/{org_id}/booking/sync",
    params(OrgIdPath),
    responses(
        (status = 200, description = "Sync initiated", body = SyncResponse),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Booking.com"
)]
pub async fn sync_booking(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
) -> Result<Json<SyncResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Syncing Booking.com"
    );

    let rental_repo = &state.rental_repo;

    // Get Booking.com connection for org
    let connection = rental_repo
        .find_booking_connection_by_org(path.org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to find Booking.com connection");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check connection",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "No Booking.com connection found",
                )),
            )
        })?;

    // Get credentials from connection
    // For Booking.com, we store hotel_id in external_property_id
    // and use access_token/refresh_token to store username/password
    let hotel_id = connection.external_property_id.clone().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "NOT_CONFIGURED",
                "Hotel ID not configured",
            )),
        )
    })?;

    // Get credentials (stored in access_token and refresh_token for Booking.com)
    let username = connection.access_token.clone().unwrap_or_default();
    let password = connection.refresh_token.clone().unwrap_or_default();

    // Create Booking.com client
    let credentials = integrations::BookingCredentials::new(hotel_id.clone(), username, password);
    let client = BookingClient::new(credentials);

    // Sync reservations
    let reservations = client.sync_reservations(&hotel_id).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to sync Booking.com reservations");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "SYNC_ERROR",
                format!("Failed to sync: {}", e),
            )),
        )
    })?;

    let items_synced = reservations.len() as i32;

    // Update last sync timestamp
    let _ = rental_repo
        .update_connection_last_sync(connection.id, Utc::now())
        .await;

    tracing::info!(
        org_id = %path.org_id,
        hotel_id = %hotel_id,
        reservations_count = items_synced,
        "Booking.com sync completed"
    );

    Ok(Json(SyncResponse {
        success: true,
        items_synced,
        synced_at: Utc::now(),
        error: None,
    }))
}

/// Disconnect Booking.com.
#[utoipa::path(
    delete,
    path = "/api/v1/integrations/organizations/{org_id}/booking",
    params(OrgIdPath),
    responses(
        (status = 204, description = "Booking.com disconnected"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Booking.com"
)]
pub async fn disconnect_booking(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Disconnecting Booking.com"
    );

    let rental_repo = &state.rental_repo;

    // Check if there's a connection to disconnect
    let connection = rental_repo
        .find_booking_connection_by_org(path.org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to find Booking.com connection");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check connection",
                )),
            )
        })?;

    if connection.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "No Booking.com connection found for this organization",
            )),
        ));
    }

    // Revoke connection (clear credentials from database)
    let revoked_count = rental_repo
        .revoke_booking_connection(path.org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to revoke Booking.com connection");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to disconnect")),
            )
        })?;

    tracing::info!(
        org_id = %path.org_id,
        revoked_count = revoked_count,
        "Booking.com disconnected successfully"
    );

    Ok(StatusCode::NO_CONTENT)
}

/// Handle Booking.com push notification.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/booking/push",
    responses(
        (status = 200, description = "Push notification processed"),
        (status = 400, description = "Invalid notification"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Integrations - Booking.com"
)]
pub async fn booking_push_notification(
    State(_state): State<AppState>,
    body: String,
) -> Result<Response<Body>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!("Received Booking.com push notification");

    // Parse the OTA notification
    let _notification = BookingClient::parse_push_notification(&body).map_err(|e| {
        tracing::error!(error = %e, "Failed to parse Booking.com notification");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "PARSE_ERROR",
                "Invalid notification format",
            )),
        )
    })?;

    // Process reservations (stub)
    // In production, would iterate through notifications and update database

    // Return OTA success response
    let response_xml = BookingClient::generate_push_response(true, None);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/xml")
        .body(Body::from(response_xml))
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to build response");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "RESPONSE_ERROR",
                    "Failed to build response",
                )),
            )
        })
}

// ==================== Portal Webhooks Types ====================

/// Portal connection response.
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct PortalConnectionResponse {
    /// Connection ID.
    pub id: Uuid,
    /// Portal type.
    pub portal_type: String,
    /// Connection name.
    pub name: String,
    /// Webhook URL.
    pub webhook_url: String,
    /// Whether active.
    pub is_active: bool,
    /// Last webhook received.
    pub last_webhook_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Created timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Create portal connection request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePortalConnectionRequest {
    /// Portal type (sreality, bezrealitky, immowelt, custom).
    pub portal_type: String,
    /// Connection name.
    pub name: String,
}

/// Portal inquiry response.
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct PortalInquiryResponse {
    /// Inquiry ID.
    pub id: Uuid,
    /// Portal type.
    pub portal_type: String,
    /// Contact name.
    pub contact_name: String,
    /// Contact email.
    pub contact_email: String,
    /// Contact phone.
    pub contact_phone: Option<String>,
    /// Message.
    pub message: String,
    /// Status.
    pub status: String,
    /// Received timestamp.
    pub received_at: chrono::DateTime<chrono::Utc>,
    /// Read timestamp.
    pub read_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Portal inquiry query parameters.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct PortalInquiryQuery {
    /// Filter by portal type.
    pub portal_type: Option<String>,
    /// Filter by status.
    pub status: Option<String>,
    /// Limit.
    #[serde(default = "default_limit")]
    pub limit: i32,
}

/// Connection ID path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ConnectionIdPath {
    pub connection_id: Uuid,
}

// ==================== Portal Webhooks Handlers ====================

/// List portal connections.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/organizations/{org_id}/portals",
    params(OrgIdPath),
    responses(
        (status = 200, description = "Portal connections listed", body = Vec<PortalConnectionResponse>),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Portals"
)]
pub async fn list_portal_connections(
    State(_state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
) -> Result<Json<Vec<PortalConnectionResponse>>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Listing portal connections"
    );

    // Stub implementation
    Ok(Json(Vec::new()))
}

/// Create portal connection.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/organizations/{org_id}/portals",
    params(OrgIdPath),
    request_body = CreatePortalConnectionRequest,
    responses(
        (status = 201, description = "Portal connection created", body = PortalConnectionResponse),
        (status = 400, description = "Invalid request"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Portals"
)]
pub async fn create_portal_connection(
    State(_state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
    Json(request): Json<CreatePortalConnectionRequest>,
) -> Result<(StatusCode, Json<PortalConnectionResponse>), (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        portal_type = %request.portal_type,
        "Creating portal connection"
    );

    // Validate portal type
    let portal_type = PortalType::from_str(&request.portal_type).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_PORTAL_TYPE",
                "Invalid portal type. Use: sreality, bezrealitky, immowelt, or custom",
            )),
        )
    })?;

    let connection_id = Uuid::new_v4();
    let base_url =
        std::env::var("API_BASE_URL").unwrap_or_else(|_| "https://api.ppt.example.com".to_string());

    // Stub implementation - would store in database
    Ok((
        StatusCode::CREATED,
        Json(PortalConnectionResponse {
            id: connection_id,
            portal_type: portal_type.to_string(),
            name: request.name,
            webhook_url: format!(
                "{}/api/v1/integrations/webhooks/portal/{}",
                base_url, connection_id
            ),
            is_active: true,
            last_webhook_at: None,
            created_at: chrono::Utc::now(),
        }),
    ))
}

/// Get portal connection.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/portals/{id}",
    params(ResourceIdPath),
    responses(
        (status = 200, description = "Portal connection retrieved", body = PortalConnectionResponse),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Portals"
)]
pub async fn get_portal_connection(
    State(_state): State<AppState>,
    _auth: AuthUser,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<PortalConnectionResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(connection_id = %path.id, "Getting portal connection");

    // Stub implementation
    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new(
            "NOT_FOUND",
            "Portal connection not found",
        )),
    ))
}

/// Delete portal connection.
#[utoipa::path(
    delete,
    path = "/api/v1/integrations/portals/{id}",
    params(ResourceIdPath),
    responses(
        (status = 204, description = "Portal connection deleted"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Portals"
)]
pub async fn delete_portal_connection(
    State(_state): State<AppState>,
    _auth: AuthUser,
    Path(path): Path<ResourceIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(connection_id = %path.id, "Deleting portal connection");

    // Stub implementation
    Ok(StatusCode::NO_CONTENT)
}

/// List portal inquiries.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/organizations/{org_id}/portal-inquiries",
    params(OrgIdPath, PortalInquiryQuery),
    responses(
        (status = 200, description = "Portal inquiries listed", body = Vec<PortalInquiryResponse>),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Portals"
)]
pub async fn list_portal_inquiries(
    State(_state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
    Query(_query): Query<PortalInquiryQuery>,
) -> Result<Json<Vec<PortalInquiryResponse>>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Listing portal inquiries"
    );

    // Stub implementation
    Ok(Json(Vec::new()))
}

/// Get portal inquiry.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/portal-inquiries/{id}",
    params(ResourceIdPath),
    responses(
        (status = 200, description = "Portal inquiry retrieved", body = PortalInquiryResponse),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Portals"
)]
pub async fn get_portal_inquiry(
    State(_state): State<AppState>,
    _auth: AuthUser,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<PortalInquiryResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(inquiry_id = %path.id, "Getting portal inquiry");

    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", "Portal inquiry not found")),
    ))
}

/// Mark portal inquiry as read.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/portal-inquiries/{id}/read",
    params(ResourceIdPath),
    responses(
        (status = 200, description = "Inquiry marked as read", body = PortalInquiryResponse),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Portals"
)]
pub async fn mark_inquiry_read(
    State(_state): State<AppState>,
    _auth: AuthUser,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<PortalInquiryResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(inquiry_id = %path.id, "Marking inquiry as read");

    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", "Portal inquiry not found")),
    ))
}

/// Archive portal inquiry.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/portal-inquiries/{id}/archive",
    params(ResourceIdPath),
    responses(
        (status = 200, description = "Inquiry archived", body = PortalInquiryResponse),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Portals"
)]
pub async fn archive_inquiry(
    State(_state): State<AppState>,
    _auth: AuthUser,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<PortalInquiryResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(inquiry_id = %path.id, "Archiving inquiry");

    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", "Portal inquiry not found")),
    ))
}

/// Handle incoming portal webhook.
/// This endpoint is public (no auth) as it receives webhooks from external portals.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/webhooks/portal/{connection_id}",
    params(ConnectionIdPath),
    responses(
        (status = 200, description = "Webhook processed"),
        (status = 400, description = "Invalid webhook"),
        (status = 401, description = "Invalid signature"),
        (status = 404, description = "Connection not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Integrations - Portals"
)]
pub async fn handle_portal_webhook(
    State(_state): State<AppState>,
    Path(path): Path<ConnectionIdPath>,
    headers: HeaderMap,
    body: String,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        connection_id = %path.connection_id,
        "Received portal webhook"
    );

    // In production, would:
    // 1. Fetch connection from database
    // 2. Verify signature
    // 3. Parse webhook based on portal type
    // 4. Store inquiry
    // 5. Send notifications

    // Stub: Just validate we can parse the body as JSON
    let _: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        tracing::warn!(error = %e, "Failed to parse webhook body");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("PARSE_ERROR", "Invalid JSON body")),
        )
    })?;

    // Get signature header (optional validation)
    if let Some(signature) = headers.get("X-Webhook-Signature") {
        let _sig = signature.to_str().unwrap_or_default();
        // Would verify signature against connection.webhook_secret
        tracing::debug!("Webhook signature present");
    }

    Ok(StatusCode::OK)
}
