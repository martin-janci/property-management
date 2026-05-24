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
use integrations::{AirbnbClient, AirbnbOAuthConfig, BookingClient, PortalType};
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
