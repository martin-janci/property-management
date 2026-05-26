//! Integration install-surface routes (Epic 83).
//!
//! Covers: Airbnb connect/status/disconnect, Booking.com connect/status/disconnect,
//! portal connection management, and portal inquiry viewing.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use db::models::infrastructure::{job_type, queue, CreateBackgroundJob};
use integrations::{
    AirbnbClient, AirbnbOAuthConfig, AvailabilityUpdate, BookingClient, BookingCredentials,
    IntegrationCrypto, PortalType, PropertyMapping, RateUpdate, RoomTypeMapping,
};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::sync::{OrgIdPath, ResourceIdPath};
use common::errors::ErrorResponse;

// ==================== Airbnb Types ====================

/// Airbnb connection status response.
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct AirbnbStatusResponse {
    pub connected: bool,
    pub external_account_id: Option<String>,
    pub last_sync_at: Option<chrono::DateTime<chrono::Utc>>,
    pub sync_error: Option<String>,
    pub listings_count: i32,
    pub reservations_count: i32,
}

/// Airbnb connect request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AirbnbConnectRequest {
    pub redirect_uri: Option<String>,
}

/// Airbnb connect response.
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct AirbnbConnectResponse {
    pub auth_url: String,
    pub state: String,
}

/// OAuth callback query parameters.
#[derive(Debug, Deserialize, IntoParams)]
pub struct OAuthCallbackQuery {
    pub code: String,
    pub state: String,
}

/// Airbnb OAuth callback response.
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct AirbnbCallbackResponse {
    pub success: bool,
    pub connection_id: Option<Uuid>,
    pub message: String,
    pub listings_count: Option<i32>,
}

/// Sync response (shared by Airbnb and Booking.com sync routes).
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct SyncResponse {
    pub success: bool,
    pub items_synced: i32,
    pub synced_at: chrono::DateTime<chrono::Utc>,
    pub error: Option<String>,
}

// ==================== Booking.com Types ====================

/// Booking.com connection status response.
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct BookingStatusResponse {
    pub connected: bool,
    pub hotel_id: Option<String>,
    pub last_sync_at: Option<chrono::DateTime<chrono::Utc>>,
    pub sync_error: Option<String>,
    pub properties_count: i32,
    pub reservations_count: i32,
}

/// Booking.com connect request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct BookingConnectRequest {
    pub hotel_id: String,
    pub username: String,
    pub password: String,
}

/// Room-type mapping entry in a push request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct PushRoomTypeMapping {
    /// Internal unit UUID.
    pub internal_unit_id: Uuid,
    /// Booking.com room-type code.
    pub external_room_type_id: String,
    /// Human-readable name (optional).
    pub external_room_type_name: Option<String>,
}

/// Single availability update DTO (mirrors integrations::AvailabilityUpdate).
#[derive(Debug, Deserialize, ToSchema)]
pub struct AvailabilityUpdateDto {
    /// Room type ID.
    pub room_type_id: String,
    /// Date for the update (YYYY-MM-DD).
    pub date: chrono::NaiveDate,
    /// Number of available rooms.
    pub available_count: i32,
    /// Stop-sell flag.
    #[serde(default)]
    pub stop_sell: bool,
    /// Closed to arrival.
    #[serde(default)]
    pub cta: bool,
    /// Closed to departure.
    #[serde(default)]
    pub ctd: bool,
    /// Minimum length of stay.
    pub min_los: Option<i32>,
    /// Maximum length of stay.
    pub max_los: Option<i32>,
}

/// Single rate update DTO (mirrors integrations::RateUpdate).
#[derive(Debug, Deserialize, ToSchema)]
pub struct RateUpdateDto {
    /// Room type ID.
    pub room_type_id: String,
    /// Rate plan code.
    pub rate_plan_code: String,
    /// Date for the rate (YYYY-MM-DD).
    pub date: chrono::NaiveDate,
    /// Base rate amount (decimal string, e.g. "129.00").
    pub base_rate: rust_decimal::Decimal,
    /// Currency code (ISO 4217, e.g. "EUR").
    pub currency: String,
    /// Extra person rate.
    pub extra_person_rate: Option<rust_decimal::Decimal>,
    /// Extra child rate.
    pub extra_child_rate: Option<rust_decimal::Decimal>,
}

/// Request body for pushing availability to Booking.com.
///
/// Sends OTA_HotelAvailNotifRQ to the Booking.com Supply XML API.
#[derive(Debug, Deserialize, ToSchema)]
pub struct BookingPushAvailabilityRequest {
    /// Room-type mappings (internal unit → Booking.com room type).
    pub room_mappings: Vec<PushRoomTypeMapping>,
    /// Availability updates to push.
    pub updates: Vec<AvailabilityUpdateDto>,
}

/// Request body for pushing rates to Booking.com.
///
/// Sends OTA_HotelRateAmountNotifRQ to the Booking.com Supply XML API.
#[derive(Debug, Deserialize, ToSchema)]
pub struct BookingPushRatesRequest {
    /// Rate updates to push.
    pub updates: Vec<RateUpdateDto>,
}

/// Push result response.
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct BookingPushResponse {
    pub success: bool,
    pub items_pushed: i32,
    pub pushed_at: chrono::DateTime<chrono::Utc>,
    pub error: Option<String>,
}

// ==================== Portal Types ====================

/// Portal connection response.
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct PortalConnectionResponse {
    pub id: Uuid,
    pub portal_type: String,
    pub name: String,
    pub webhook_url: String,
    pub is_active: bool,
    pub last_webhook_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Create portal connection request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePortalConnectionRequest {
    pub portal_type: String,
    pub name: String,
}

/// Portal inquiry response.
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct PortalInquiryResponse {
    pub id: Uuid,
    pub portal_type: String,
    pub contact_name: String,
    pub contact_email: String,
    pub contact_phone: Option<String>,
    pub message: String,
    pub status: String,
    pub received_at: chrono::DateTime<chrono::Utc>,
    pub read_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Portal inquiry query parameters.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct PortalInquiryQuery {
    pub portal_type: Option<String>,
    pub status: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i32,
}

fn default_limit() -> i32 {
    50
}

/// Connection ID path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ConnectionIdPath {
    pub connection_id: Uuid,
}

// ==================== Gap 83-1 Types ====================

/// Direct-connect request (supply pre-obtained tokens instead of OAuth redirect).
///
/// The `org_id` is taken from the URL path (`/organizations/{org_id}/airbnb/direct-connect`)
/// to prevent IDOR — callers cannot influence which organisation they connect to.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AirbnbDirectConnectRequest {
    /// Airbnb access token obtained outside the OAuth flow.
    pub access_token: String,
    /// Optional refresh token.
    pub refresh_token: Option<String>,
    /// Optional Airbnb account / listing ID to associate.
    pub airbnb_account_id: Option<String>,
}

/// Direct-connect response.
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct AirbnbDirectConnectResponse {
    pub success: bool,
    pub connection_id: Uuid,
    pub listings_count: Option<i32>,
    pub message: String,
}

/// Availability-sync enqueue response.
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct AirbnbAvailabilitySyncResponse {
    pub job_id: Uuid,
    pub queued: bool,
    pub message: String,
}

// ==================== Router ====================

/// Create install-surface router.
pub fn router() -> Router<crate::state::AppState> {
    Router::new()
        // Airbnb Install (Story 83.1 — install/status/disconnect)
        .route(
            "/organizations/{org_id}/airbnb/status",
            get(get_airbnb_status),
        )
        .route(
            "/organizations/{org_id}/airbnb/connect",
            post(connect_airbnb),
        )
        .route("/organizations/{org_id}/airbnb/sync", post(sync_airbnb))
        .route("/organizations/{org_id}/airbnb", delete(disconnect_airbnb))
        // Gap 83-1: direct token connect + availability-sync enqueue
        .route(
            "/organizations/{org_id}/airbnb/direct-connect",
            post(direct_connect_airbnb),
        )
        .route(
            "/organizations/{org_id}/airbnb/availability-sync",
            post(enqueue_airbnb_availability_sync),
        )
        // Booking.com Install (Story 83.2 — install/status/disconnect)
        .route(
            "/organizations/{org_id}/booking/status",
            get(get_booking_status),
        )
        .route(
            "/organizations/{org_id}/booking/connect",
            post(connect_booking),
        )
        .route("/organizations/{org_id}/booking/sync", post(sync_booking))
        .route(
            "/organizations/{org_id}/booking/push-availability",
            post(push_booking_availability),
        )
        .route(
            "/organizations/{org_id}/booking/push-rates",
            post(push_booking_rates),
        )
        .route(
            "/organizations/{org_id}/booking",
            delete(disconnect_booking),
        )
        // Portal Connections (Story 83.3 — install side)
        .route(
            "/organizations/{org_id}/portals",
            get(list_portal_connections),
        )
        .route(
            "/organizations/{org_id}/portals",
            post(create_portal_connection),
        )
        .route("/portals/{id}", get(get_portal_connection))
        .route("/portals/{id}", delete(delete_portal_connection))
        // Portal Inquiries (read-only view)
        .route(
            "/organizations/{org_id}/portal-inquiries",
            get(list_portal_inquiries),
        )
        .route("/portal-inquiries/{id}", get(get_portal_inquiry))
        .route("/portal-inquiries/{id}/read", post(mark_inquiry_read))
        .route("/portal-inquiries/{id}/archive", post(archive_inquiry))
}

// ==================== Airbnb Handlers ====================

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
    State(state): State<crate::state::AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
) -> Result<Json<AirbnbStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Getting Airbnb status"
    );

    let rental_repo = &state.rental_repo;

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
    State(_state): State<crate::state::AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
    Json(request): Json<AirbnbConnectRequest>,
) -> Result<Json<AirbnbConnectResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Initiating Airbnb OAuth connection"
    );

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

    let oauth_state = format!("{}:{}", path.org_id, uuid::Uuid::new_v4());
    tracing::debug!(oauth_state = %oauth_state, "Generated OAuth state parameter");

    let auth_url = client.generate_auth_url(&oauth_state);

    Ok(Json(AirbnbConnectResponse {
        auth_url,
        state: oauth_state,
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
    State(state): State<crate::state::AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
) -> Result<Json<SyncResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Syncing Airbnb"
    );

    let rental_repo = &state.rental_repo;

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

    let access_token = connection.access_token.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "Airbnb not authorized",
            )),
        )
    })?;

    let oauth_config = AirbnbOAuthConfig {
        client_id: std::env::var("AIRBNB_CLIENT_ID").unwrap_or_default(),
        client_secret: std::env::var("AIRBNB_CLIENT_SECRET").unwrap_or_default(),
        redirect_uri: std::env::var("AIRBNB_REDIRECT_URI").unwrap_or_default(),
    };
    let client = AirbnbClient::new(oauth_config);

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

    let _ = rental_repo
        .update_connection_last_sync(connection.id, chrono::Utc::now())
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
        synced_at: chrono::Utc::now(),
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
    State(state): State<crate::state::AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Disconnecting Airbnb"
    );

    let rental_repo = &state.rental_repo;

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

    if let Some(conn) = &connection {
        if conn.access_token.is_some() {
            tracing::info!(
                connection_id = %conn.id,
                "Clearing Airbnb tokens (API revocation not available)"
            );
        }
    }

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

// ==================== Booking.com Handlers ====================

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
    State(state): State<crate::state::AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
) -> Result<Json<BookingStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Getting Booking.com status"
    );

    let rental_repo = &state.rental_repo;

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
        Some(conn) => Ok(Json(BookingStatusResponse {
            connected: conn.is_active,
            hotel_id: conn.external_property_id.clone(),
            last_sync_at: conn.last_sync_at,
            sync_error: conn.sync_error.clone(),
            properties_count: if conn.is_active { 1 } else { 0 },
            reservations_count: 0,
        })),
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
    State(state): State<crate::state::AppState>,
    auth: api_core::AuthUser,
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

    let credentials = integrations::BookingCredentials::new(
        request.hotel_id.clone(),
        request.username.clone(),
        request.password.clone(),
    );
    let client = BookingClient::new(credentials);

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
        }
    }

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
    State(state): State<crate::state::AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
) -> Result<Json<SyncResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Syncing Booking.com"
    );

    let rental_repo = &state.rental_repo;

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

    let hotel_id = connection.external_property_id.clone().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "NOT_CONFIGURED",
                "Hotel ID not configured",
            )),
        )
    })?;

    let username = connection.access_token.clone().unwrap_or_default();
    let password = connection.refresh_token.clone().unwrap_or_default();

    let credentials = integrations::BookingCredentials::new(hotel_id.clone(), username, password);
    let client = BookingClient::new(credentials);

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

    let _ = rental_repo
        .update_connection_last_sync(connection.id, chrono::Utc::now())
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
        synced_at: chrono::Utc::now(),
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
    State(state): State<crate::state::AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Disconnecting Booking.com"
    );

    let rental_repo = &state.rental_repo;

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

// ==================== Booking.com Push Handlers ====================

/// Push availability updates to Booking.com.
///
/// Sends OTA_HotelAvailNotifRQ to the Booking.com Supply XML API for the
/// connected property.  Requires an active Booking.com connection with stored
/// credentials.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/organizations/{org_id}/booking/push-availability",
    params(OrgIdPath),
    request_body = BookingPushAvailabilityRequest,
    responses(
        (status = 200, description = "Availability pushed", body = BookingPushResponse),
        (status = 400, description = "No updates provided or connection not configured"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "No Booking.com connection found"),
        (status = 500, description = "Internal server error"),
        (status = 502, description = "Booking.com upstream API failure")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Booking.com"
)]
pub async fn push_booking_availability(
    State(state): State<crate::state::AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
    Json(request): Json<BookingPushAvailabilityRequest>,
) -> Result<Json<BookingPushResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        update_count = request.updates.len(),
        "Pushing availability to Booking.com"
    );

    if auth.tenant_id != Some(path.org_id) && !auth.is_platform_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new("FORBIDDEN", "Access denied")),
        ));
    }

    if request.updates.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "NO_UPDATES",
                "At least one availability update is required",
            )),
        ));
    }

    let rental_repo = &state.rental_repo;

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

    let hotel_id = connection.external_property_id.clone().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "NOT_CONFIGURED",
                "Hotel ID not configured for this connection",
            )),
        )
    })?;

    let username = connection.access_token.clone().unwrap_or_default();
    let password = connection.refresh_token.clone().unwrap_or_default();

    let credentials = BookingCredentials::new(hotel_id.clone(), username, password);
    let client = BookingClient::new(credentials);

    // Build property mapping from the request room mappings
    let mapping = PropertyMapping {
        internal_property_id: path.org_id,
        external_property_id: hotel_id.clone(),
        external_property_name: None,
        room_mappings: request
            .room_mappings
            .iter()
            .map(|rm| RoomTypeMapping {
                internal_unit_id: rm.internal_unit_id,
                external_room_type_id: rm.external_room_type_id.clone(),
                external_room_type_name: rm.external_room_type_name.clone(),
            })
            .collect(),
        sync_enabled: true,
        last_sync_at: None,
    };

    let items_count = request.updates.len() as i32;

    // Convert DTOs to integration types
    let availability_updates: Vec<AvailabilityUpdate> = request
        .updates
        .into_iter()
        .map(|u| AvailabilityUpdate {
            room_type_id: u.room_type_id,
            date: u.date,
            available_count: u.available_count,
            stop_sell: u.stop_sell,
            cta: u.cta,
            ctd: u.ctd,
            min_los: u.min_los,
            max_los: u.max_los,
        })
        .collect();

    match client
        .push_availability(&mapping, availability_updates)
        .await
    {
        Ok(()) => {
            tracing::info!(
                org_id = %path.org_id,
                hotel_id = %hotel_id,
                items_pushed = items_count,
                "Booking.com availability push succeeded"
            );
            Ok(Json(BookingPushResponse {
                success: true,
                items_pushed: items_count,
                pushed_at: chrono::Utc::now(),
                error: None,
            }))
        }
        Err(e) => {
            tracing::error!(
                org_id = %path.org_id,
                hotel_id = %hotel_id,
                error = %e,
                "Booking.com availability push failed"
            );
            Err((
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse::new(
                    "UPSTREAM_FAILURE",
                    format!("Booking.com push failed: {e}"),
                )),
            ))
        }
    }
}

/// Push rate updates to Booking.com.
///
/// Sends OTA_HotelRateAmountNotifRQ to the Booking.com Supply XML API for the
/// connected property.  Requires an active Booking.com connection with stored
/// credentials.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/organizations/{org_id}/booking/push-rates",
    params(OrgIdPath),
    request_body = BookingPushRatesRequest,
    responses(
        (status = 200, description = "Rates pushed", body = BookingPushResponse),
        (status = 400, description = "No updates provided or connection not configured"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "No Booking.com connection found"),
        (status = 500, description = "Internal server error"),
        (status = 502, description = "Booking.com upstream API failure")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Booking.com"
)]
pub async fn push_booking_rates(
    State(state): State<crate::state::AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
    Json(request): Json<BookingPushRatesRequest>,
) -> Result<Json<BookingPushResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        update_count = request.updates.len(),
        "Pushing rates to Booking.com"
    );

    if auth.tenant_id != Some(path.org_id) && !auth.is_platform_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new("FORBIDDEN", "Access denied")),
        ));
    }

    if request.updates.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "NO_UPDATES",
                "At least one rate update is required",
            )),
        ));
    }

    let rental_repo = &state.rental_repo;

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

    let hotel_id = connection.external_property_id.clone().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "NOT_CONFIGURED",
                "Hotel ID not configured for this connection",
            )),
        )
    })?;

    let username = connection.access_token.clone().unwrap_or_default();
    let password = connection.refresh_token.clone().unwrap_or_default();

    let credentials = BookingCredentials::new(hotel_id.clone(), username, password);
    let client = BookingClient::new(credentials);

    let items_count = request.updates.len() as i32;

    // Convert DTOs to integration types
    let rate_updates: Vec<RateUpdate> = request
        .updates
        .into_iter()
        .map(|u| RateUpdate {
            room_type_id: u.room_type_id,
            rate_plan_code: u.rate_plan_code,
            date: u.date,
            base_rate: u.base_rate,
            currency: u.currency,
            extra_person_rate: u.extra_person_rate,
            extra_child_rate: u.extra_child_rate,
        })
        .collect();

    match client.push_rates(&hotel_id, rate_updates).await {
        Ok(()) => {
            tracing::info!(
                org_id = %path.org_id,
                hotel_id = %hotel_id,
                items_pushed = items_count,
                "Booking.com rates push succeeded"
            );
            Ok(Json(BookingPushResponse {
                success: true,
                items_pushed: items_count,
                pushed_at: chrono::Utc::now(),
                error: None,
            }))
        }
        Err(e) => {
            tracing::error!(
                org_id = %path.org_id,
                hotel_id = %hotel_id,
                error = %e,
                "Booking.com rates push failed"
            );
            Err((
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse::new(
                    "UPSTREAM_FAILURE",
                    format!("Booking.com push failed: {e}"),
                )),
            ))
        }
    }
}

// ==================== Portal Connection Handlers ====================

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
    State(_state): State<crate::state::AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
) -> Result<Json<Vec<PortalConnectionResponse>>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Listing portal connections"
    );

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
    State(_state): State<crate::state::AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
    Json(request): Json<CreatePortalConnectionRequest>,
) -> Result<(StatusCode, Json<PortalConnectionResponse>), (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        portal_type = %request.portal_type,
        "Creating portal connection"
    );

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
    State(_state): State<crate::state::AppState>,
    _auth: api_core::AuthUser,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<PortalConnectionResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(connection_id = %path.id, "Getting portal connection");

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
    State(_state): State<crate::state::AppState>,
    _auth: api_core::AuthUser,
    Path(path): Path<ResourceIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(connection_id = %path.id, "Deleting portal connection");

    Ok(StatusCode::NO_CONTENT)
}

// ==================== Portal Inquiry Handlers ====================

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
    State(_state): State<crate::state::AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
    Query(_query): Query<PortalInquiryQuery>,
) -> Result<Json<Vec<PortalInquiryResponse>>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Listing portal inquiries"
    );

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
    State(_state): State<crate::state::AppState>,
    _auth: api_core::AuthUser,
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
    State(_state): State<crate::state::AppState>,
    _auth: api_core::AuthUser,
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
    State(_state): State<crate::state::AppState>,
    _auth: api_core::AuthUser,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<PortalInquiryResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(inquiry_id = %path.id, "Archiving inquiry");

    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", "Portal inquiry not found")),
    ))
}

// ==================== Gap 83-1 Handlers ====================

/// Direct-connect Airbnb using a pre-obtained access token (no OAuth redirect).
///
/// The caller supplies a valid Airbnb access token. This handler verifies the
/// token by fetching listings, encrypts the tokens at rest (if
/// `INTEGRATION_ENCRYPTION_KEY` is set), and upserts the connection record.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/organizations/{org_id}/airbnb/direct-connect",
    params(OrgIdPath),
    request_body = AirbnbDirectConnectRequest,
    responses(
        (status = 200, description = "Airbnb connected", body = AirbnbDirectConnectResponse),
        (status = 400, description = "Invalid or expired token"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Airbnb"
)]
pub async fn direct_connect_airbnb(
    State(state): State<crate::state::AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
    Json(request): Json<AirbnbDirectConnectRequest>,
) -> Result<Json<AirbnbDirectConnectResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Org ownership guard — prevent IDOR.
    if auth.tenant_id != Some(path.org_id) && !auth.is_platform_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "You do not have access to this organization",
            )),
        ));
    }
    let role = auth.role.unwrap_or(common::TenantRole::Guest);
    if !matches!(
        role,
        common::TenantRole::SuperAdmin
            | common::TenantRole::PlatformAdmin
            | common::TenantRole::OrgAdmin
            | common::TenantRole::Manager
    ) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Insufficient permissions to manage integrations",
            )),
        ));
    }

    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Direct-connecting Airbnb with pre-obtained token"
    );

    let org_id = path.org_id;

    // Validate required env vars before making any external calls.
    let client_id = std::env::var("AIRBNB_CLIENT_ID").map_err(|_| {
        tracing::error!("AIRBNB_CLIENT_ID is not configured");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "NOT_CONFIGURED",
                "Airbnb integration is not configured",
            )),
        )
    })?;
    let client_secret = std::env::var("AIRBNB_CLIENT_SECRET").map_err(|_| {
        tracing::error!("AIRBNB_CLIENT_SECRET is not configured");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "NOT_CONFIGURED",
                "Airbnb integration is not configured",
            )),
        )
    })?;
    let redirect_uri = std::env::var("AIRBNB_REDIRECT_URI").unwrap_or_default();

    // Verify the token is valid by fetching listings.
    let oauth_config = AirbnbOAuthConfig {
        client_id,
        client_secret,
        redirect_uri,
    };
    let client = AirbnbClient::new(oauth_config);

    let listings = client
        .fetch_listings(&request.access_token)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "Airbnb token validation failed");
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_TOKEN",
                    "Airbnb access token is invalid or expired",
                )),
            )
        })?;

    let listings_count = listings.len() as i32;

    // Optionally encrypt tokens before storage.
    let (stored_access, stored_refresh) = match IntegrationCrypto::try_from_env() {
        Some(crypto) => {
            let encrypted_access = crypto.encrypt(&request.access_token).map_err(|e| {
                tracing::error!(error = %e, "Failed to encrypt Airbnb access token");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "CRYPTO_ERROR",
                        "Failed to encrypt token",
                    )),
                )
            })?;
            let encrypted_refresh = request
                .refresh_token
                .as_deref()
                .map(|rt| crypto.encrypt(rt))
                .transpose()
                .map_err(|e| {
                    tracing::error!(error = %e, "Failed to encrypt Airbnb refresh token");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new(
                            "CRYPTO_ERROR",
                            "Failed to encrypt refresh token",
                        )),
                    )
                })?;
            (encrypted_access, encrypted_refresh)
        }
        None => {
            tracing::warn!(
                "INTEGRATION_ENCRYPTION_KEY is not set; Airbnb tokens will be stored in plaintext"
            );
            (request.access_token.clone(), request.refresh_token.clone())
        }
    };

    let connection = state
        .rental_repo
        .upsert_airbnb_connection(
            org_id,
            None,
            &stored_access,
            stored_refresh.as_deref(),
            None,
            request.airbnb_account_id.as_deref(),
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to upsert Airbnb connection");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to store Airbnb connection",
                )),
            )
        })?;

    tracing::info!(
        connection_id = %connection.id,
        listings = listings_count,
        "Airbnb direct connect succeeded"
    );

    Ok(Json(AirbnbDirectConnectResponse {
        success: true,
        connection_id: connection.id,
        listings_count: Some(listings_count),
        message: format!(
            "Airbnb connected successfully. {} listing(s) found.",
            listings_count
        ),
    }))
}

/// Enqueue an Airbnb availability-sync background job for an organisation.
///
/// Returns HTTP 202 Accepted with the job ID so the caller can poll for
/// completion via the background-jobs API.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/organizations/{org_id}/airbnb/availability-sync",
    params(OrgIdPath),
    responses(
        (status = 202, description = "Availability sync job queued", body = AirbnbAvailabilitySyncResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "No Airbnb connection found for organisation"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Airbnb"
)]
pub async fn enqueue_airbnb_availability_sync(
    State(state): State<crate::state::AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
) -> Result<(StatusCode, Json<AirbnbAvailabilitySyncResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Org ownership guard.
    if auth.tenant_id != Some(path.org_id) && !auth.is_platform_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "You do not have access to this organization",
            )),
        ));
    }
    let role = auth.role.unwrap_or(common::TenantRole::Guest);
    if !matches!(
        role,
        common::TenantRole::SuperAdmin
            | common::TenantRole::PlatformAdmin
            | common::TenantRole::OrgAdmin
            | common::TenantRole::Manager
    ) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Insufficient permissions to manage integrations",
            )),
        ));
    }

    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Enqueueing Airbnb availability sync"
    );

    // Verify that a connection exists before queueing work.
    let connection = state
        .rental_repo
        .find_airbnb_connection_by_org(path.org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to look up Airbnb connection");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check Airbnb connection",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "No Airbnb connection found for this organisation",
                )),
            )
        })?;

    let payload = serde_json::json!({
        "org_id": path.org_id,
        "connection_id": connection.id,
        "sync_type": "availability",
    });

    let job_data = CreateBackgroundJob {
        job_type: job_type::SYNC_EXTERNAL.to_string(),
        priority: Some(1),
        payload,
        scheduled_at: None,
        queue: Some(queue::LOW_PRIORITY.to_string()),
        max_attempts: Some(3),
        org_id: Some(path.org_id),
    };

    let job = state
        .background_job_repo
        .create(job_data, Some(auth.user_id))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create availability sync job");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to enqueue availability sync job",
                )),
            )
        })?;

    tracing::info!(job_id = %job.id, org_id = %path.org_id, "Airbnb availability sync job queued");

    Ok((
        StatusCode::ACCEPTED,
        Json(AirbnbAvailabilitySyncResponse {
            job_id: job.id,
            queued: true,
            message: "Availability sync job queued successfully".to_string(),
        }),
    ))
}
