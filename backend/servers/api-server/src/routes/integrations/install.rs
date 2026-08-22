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
    encrypt_optional_required, encrypt_required, AirbnbClient, AirbnbOAuthConfig,
    AvailabilityUpdate, BookingClient, BookingCredentials, IntegrationCrypto, PortalType,
    RateUpdate,
};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::sync::{verify_manager_role_in_org, verify_org_access, OrgIdPath, ResourceIdPath};
use common::errors::ErrorResponse;

const MAX_BATCH_SIZE: usize = 500;

// ==================== Booking.com Push Validation ====================
//
// Pure request-shape guards for the legacy Booking.com push handlers
// (issue #572, PR #607). Extracted from the inline handler bodies so the
// batch-cap and non-negative `available_count` guards have a DB-free
// regression test (see the `tests` module). The unified OTA `listing-push`
// surface has its own equivalent guard in `booking_channel::validate_listing_push`
// (PR #1045) — these are the *legacy* push-availability / push-rates guards.
//
// On failure each returns `(error_code, human_message)` so the caller can
// build a `400` [`ErrorResponse`]; on success returns `Ok(())`.

/// Validate a Booking.com push-availability request body.
///
/// Rejects:
/// * an empty `updates` list (`NO_UPDATES`),
/// * more than [`MAX_BATCH_SIZE`] updates (`BATCH_TOO_LARGE`),
/// * any negative `available_count` (`INVALID_AVAILABLE_COUNT`).
fn validate_push_availability(
    request: &BookingPushAvailabilityRequest,
) -> Result<(), (&'static str, &'static str)> {
    if request.updates.is_empty() {
        return Err(("NO_UPDATES", "At least one availability update is required"));
    }

    if request.updates.len() > MAX_BATCH_SIZE {
        return Err((
            "BATCH_TOO_LARGE",
            "A maximum of 500 updates per request is allowed",
        ));
    }

    if request.updates.iter().any(|u| u.available_count < 0) {
        return Err((
            "INVALID_AVAILABLE_COUNT",
            "available_count must be non-negative",
        ));
    }

    Ok(())
}

/// Validate a Booking.com push-rates request body.
///
/// Rejects:
/// * an empty `updates` list (`NO_UPDATES`),
/// * more than [`MAX_BATCH_SIZE`] updates (`BATCH_TOO_LARGE`).
///
/// Rate updates carry no `available_count`, so the non-negative guard does
/// not apply here.
fn validate_push_rates(
    request: &BookingPushRatesRequest,
) -> Result<(), (&'static str, &'static str)> {
    if request.updates.is_empty() {
        return Err(("NO_UPDATES", "At least one rate update is required"));
    }

    if request.updates.len() > MAX_BATCH_SIZE {
        return Err((
            "BATCH_TOO_LARGE",
            "A maximum of 500 updates per request is allowed",
        ));
    }

    Ok(())
}

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

    // Issue #765: prevent cross-org IDOR — caller must belong to this org.
    verify_org_access(&state, auth.user_id, path.org_id).await?;

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
    State(state): State<crate::state::AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
    Json(request): Json<AirbnbConnectRequest>,
) -> Result<Json<AirbnbConnectResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Initiating Airbnb OAuth connection"
    );

    // Issue #765: prevent cross-org IDOR — caller must belong to this org.
    verify_org_access(&state, auth.user_id, path.org_id).await?;

    // Issue #711: AppState carries Airbnb credentials loaded once at startup.
    let client_id = state.airbnb_config.client_id.clone();
    let client_secret = state.airbnb_config.client_secret.clone();
    let redirect_uri = request
        .redirect_uri
        .unwrap_or_else(|| state.airbnb_config.redirect_uri.clone());

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

    // Issue #765: generate a server-bound, single-use OAuth state tied to the
    // initiating org + user (persisted in Redis with a short TTL when available)
    // instead of a stateless, forgeable `{org_id}:{uuid}` string.
    let oauth_state = super::oauth_state::issue(&state, path.org_id, auth.user_id).await;
    tracing::debug!(oauth_state = %oauth_state, "Generated OAuth state parameter");

    let auth_url = client.generate_auth_url(&oauth_state);

    Ok(Json(AirbnbConnectResponse {
        auth_url,
        state: oauth_state,
    }))
}

/// Trigger Airbnb sync.
///
/// Gap 83-1: Uses `with_token_refresh` so the stored OAuth access token is
/// proactively refreshed before expiry and automatically rotated on 401.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/organizations/{org_id}/airbnb/sync",
    params(OrgIdPath),
    responses(
        (status = 200, description = "Sync initiated", body = SyncResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "No Airbnb connection found"),
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

    // Issue #765: prevent cross-org IDOR — caller must belong to this org.
    verify_org_access(&state, auth.user_id, path.org_id).await?;

    // Issue #711 + Gap 83-1: use cached config; wrap the Airbnb API calls in
    // `with_token_refresh` so the access token is proactively renewed near
    // expiry and auto-rotated on 401.
    let oauth_config = AirbnbOAuthConfig {
        client_id: state.airbnb_config.client_id.clone(),
        client_secret: state.airbnb_config.client_secret.clone(),
        redirect_uri: state.airbnb_config.redirect_uri.clone(),
    };
    let state_ref = &state;
    let org_id = path.org_id;

    let (listings, connection_id) =
        match super::token_rotation::with_token_refresh(state_ref, org_id, |access_token| {
            let client = AirbnbClient::new(oauth_config.clone());
            async move {
                let listings = client.fetch_listings(&access_token).await?;
                Ok(listings)
            }
        })
        .await
        {
            super::token_rotation::TokenRotationOutcome::Ok(listings) => {
                // We need the connection_id for the last_sync update below.
                // Re-fetch the (now-updated) connection briefly.
                let conn_id = state
                    .rental_repo
                    .find_airbnb_connection_by_org(org_id)
                    .await
                    .ok()
                    .flatten()
                    .map(|c| c.id);
                (listings, conn_id)
            }
            super::token_rotation::TokenRotationOutcome::NoConnection => {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new(
                        "NOT_FOUND",
                        "No Airbnb connection found",
                    )),
                ));
            }
            super::token_rotation::TokenRotationOutcome::ExpiredNoRefresh => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse::new(
                        "TOKEN_EXPIRED",
                        "Airbnb token expired and no refresh token available",
                    )),
                ));
            }
            super::token_rotation::TokenRotationOutcome::DecryptionFailed(e) => {
                tracing::error!(org_id = %org_id, error = %e, "Token decryption failed");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "DECRYPTION_ERROR",
                        "Failed to decrypt Airbnb credentials",
                    )),
                ));
            }
            super::token_rotation::TokenRotationOutcome::RefreshFailed(e) => {
                tracing::error!(org_id = %org_id, error = %e, "Token refresh failed");
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse::new(
                        "TOKEN_REFRESH_FAILED",
                        "Failed to refresh Airbnb token",
                    )),
                ));
            }
            super::token_rotation::TokenRotationOutcome::CallFailed(e) => {
                tracing::error!(org_id = %org_id, error = %e, "Airbnb API call failed");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "SYNC_ERROR",
                        format!("Failed to sync: {}", e),
                    )),
                ));
            }
        };

    // Fetch reservations for each listing (best-effort; errors are warnings).
    let client_for_res = AirbnbClient::new(AirbnbOAuthConfig {
        client_id: state.airbnb_config.client_id.clone(),
        client_secret: state.airbnb_config.client_secret.clone(),
        redirect_uri: state.airbnb_config.redirect_uri.clone(),
    });

    // We need the plaintext token again for reservation fetches.  Rather than
    // calling with_token_refresh again (double refresh risk), decrypt the now-
    // current token from the connection.
    let current_access_token = state
        .rental_repo
        .find_airbnb_connection_by_org(org_id)
        .await
        .ok()
        .flatten()
        .and_then(|c| {
            let crypto = integrations::IntegrationCrypto::try_from_env();
            c.canonical_encrypted_token()
                .map(|t| integrations::decrypt_if_available(crypto.as_ref(), t))
        });

    let mut total_items = listings.len();
    if let Some(ref token) = current_access_token {
        for listing in &listings {
            match client_for_res
                .fetch_reservations(token, &listing.id, None, None)
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
    }

    if let Some(cid) = connection_id {
        let _ = state
            .rental_repo
            .update_connection_last_sync(cid, chrono::Utc::now())
            .await;
    }

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

    // Issue #765: prevent cross-org IDOR — caller must belong to this org.
    verify_org_access(&state, auth.user_id, path.org_id).await?;
    // SECURITY: wiping the org's Airbnb connection is a manager-only mutation;
    // `verify_org_access` alone lets any org member tear down the integration.
    verify_manager_role_in_org(&state, auth.user_id, path.org_id).await?;

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

    // Issue #765: prevent cross-org IDOR — caller must belong to this org.
    verify_org_access(&state, auth.user_id, path.org_id).await?;

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

    // Issue #765: prevent cross-org IDOR — caller must belong to this org.
    verify_org_access(&state, auth.user_id, path.org_id).await?;
    // SECURITY: storing Booking.com credentials is a manager-only mutation.
    // `verify_org_access` passes for any org member (including plain residents);
    // without this gate a non-manager could overwrite the org's active OTA
    // credentials with attacker-controlled ones. Mirror the manager gate already
    // enforced by `booking_token_exchange` (oauth.rs) and `direct_connect_airbnb`.
    verify_manager_role_in_org(&state, auth.user_id, path.org_id).await?;

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

    // BIT-99: encrypt the Supply-XML basic-auth credentials before storage,
    // mirroring the mandatory Airbnb token encryption (Issue #765). Booking.com
    // reuses access_token/refresh_token for (username, password); if
    // INTEGRATION_ENCRYPTION_KEY is unset we fail closed rather than persisting
    // plaintext credentials.
    let crypto = IntegrationCrypto::try_from_env();
    let stored_username = encrypt_required(crypto.as_ref(), &request.username).map_err(|e| {
        tracing::error!(error = %e, "Refusing to store Booking.com username without encryption");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "ENCRYPTION_REQUIRED",
                "Integration credential encryption is not configured",
            )),
        )
    })?;
    let stored_password = encrypt_required(crypto.as_ref(), &request.password).map_err(|e| {
        tracing::error!(error = %e, "Refusing to store Booking.com password without encryption");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "ENCRYPTION_REQUIRED",
                "Integration credential encryption is not configured",
            )),
        )
    })?;

    let _connection = rental_repo
        .create_or_update_booking_connection(
            path.org_id,
            &request.hotel_id,
            &stored_username,
            &stored_password,
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

    // Issue #765: prevent cross-org IDOR — caller must belong to this org.
    verify_org_access(&state, auth.user_id, path.org_id).await?;

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

    // BIT-99: credentials are stored encrypted; decrypt for OTA API use.
    // decrypt_if_available tolerates legacy plaintext rows (no "enc:" prefix).
    let crypto = IntegrationCrypto::try_from_env();
    let username = integrations::decrypt_if_available(
        crypto.as_ref(),
        &connection.access_token.clone().unwrap_or_default(),
    );
    let password = integrations::decrypt_if_available(
        crypto.as_ref(),
        &connection.refresh_token.clone().unwrap_or_default(),
    );

    let credentials = integrations::BookingCredentials::new(hotel_id.clone(), username, password);
    let client = BookingClient::new(credentials);

    // Sync window: today through one year out (matches the prior
    // `sync_reservations` default before the OTA-XML refactor moved the date
    // range to the caller).
    let sync_start = chrono::Utc::now().date_naive();
    let sync_end = sync_start + chrono::Duration::days(365);
    let reservations = client
        .fetch_reservations(&hotel_id, sync_start, sync_end)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to sync Booking.com reservations");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "SYNC_ERROR",
                    format!("Failed to sync: {}", e),
                )),
            )
        })?;

    let pulled_count = reservations.len() as i32;
    let mut persisted_count = 0i32;

    // Attempt to persist reservations when a unit-level connection exists for the
    // room_type_id.  Org-level connections (unit_id = nil UUID) cannot satisfy the
    // rental_bookings.unit_id NOT NULL FK, so those are counted but not stored —
    // the operator must map room types via the listing-push / room-mapping API.
    for reservation in &reservations {
        // Try to parse the Booking.com room_type_id as a UUID (set by the operator
        // during room-type mapping).  String IDs like "DBL" won't parse and result
        // in Uuid::nil(), which matches the org-level connection → we skip.
        let room_uuid = reservation
            .room_type_id
            .parse::<uuid::Uuid>()
            .ok()
            .unwrap_or(uuid::Uuid::nil());

        if room_uuid == uuid::Uuid::nil() {
            tracing::debug!(
                reservation_id = %reservation.reservation_id,
                room_type_id = %reservation.room_type_id,
                "Skipping reservation: room_type_id is not a UUID (room mapping not configured)"
            );
            continue;
        }

        let unit_conn = rental_repo
            .find_connection_by_unit_platform(room_uuid, "booking")
            .await
            .ok()
            .flatten();

        let unit_id = match unit_conn {
            Some(ref conn) if conn.unit_id != uuid::Uuid::nil() => conn.unit_id,
            _ => continue,
        };

        // Skip if already persisted.
        let already_exists = rental_repo
            .find_booking_by_external_id("booking", &reservation.reservation_id)
            .await
            .ok()
            .flatten()
            .is_some();

        if already_exists {
            continue;
        }

        // Conflict gate: check the calendar before inserting.
        // Fail-safe: on DB error treat as unavailable to avoid double-booking.
        let is_available = rental_repo
            .check_availability(unit_id, reservation.check_in, reservation.check_out)
            .await
            .unwrap_or(false);

        if !is_available {
            tracing::warn!(
                reservation_id = %reservation.reservation_id,
                unit_id = %unit_id,
                check_in = %reservation.check_in,
                check_out = %reservation.check_out,
                "Booking.com reservation conflicts with existing calendar block — skipping"
            );
            continue;
        }

        let guest_name = format!(
            "{} {}",
            reservation.guest.first_name.trim(),
            reservation.guest.last_name.trim()
        );

        let create = db::models::CreateBooking {
            unit_id,
            platform: "booking".to_string(),
            external_booking_id: Some(reservation.reservation_id.clone()),
            guest_name,
            guest_email: reservation.guest.email.clone(),
            guest_phone: reservation.guest.phone.clone(),
            guest_count: reservation.adults + reservation.children,
            check_in: reservation.check_in,
            check_out: reservation.check_out,
            check_in_time: None,
            check_out_time: None,
            total_amount: Some(reservation.total_price),
            currency: Some(reservation.currency.clone()),
            platform_fee: Some(reservation.commission),
            cleaning_fee: None,
            guest_notes: reservation.special_requests.clone(),
            internal_notes: None,
        };

        match rental_repo.create_booking(path.org_id, create).await {
            Ok(_) => {
                persisted_count += 1;
                tracing::info!(
                    reservation_id = %reservation.reservation_id,
                    unit_id = %unit_id,
                    "Persisted Booking.com reservation"
                );
            }
            Err(e) => {
                tracing::warn!(
                    reservation_id = %reservation.reservation_id,
                    error = %e,
                    "Failed to persist Booking.com reservation"
                );
            }
        }
    }

    let _ = rental_repo
        .update_connection_last_sync(connection.id, chrono::Utc::now())
        .await;

    tracing::info!(
        org_id = %path.org_id,
        hotel_id = %hotel_id,
        pulled_count = pulled_count,
        persisted_count = persisted_count,
        "Booking.com sync completed"
    );

    Ok(Json(SyncResponse {
        success: true,
        items_synced: pulled_count,
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

    // Issue #765: prevent cross-org IDOR — caller must belong to this org.
    verify_org_access(&state, auth.user_id, path.org_id).await?;
    // SECURITY: wiping the org's Booking.com connection is a manager-only
    // mutation; `verify_org_access` alone lets any org member tear it down.
    verify_manager_role_in_org(&state, auth.user_id, path.org_id).await?;

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

    validate_push_availability(&request)
        .map_err(|(code, msg)| (StatusCode::BAD_REQUEST, Json(ErrorResponse::new(code, msg))))?;

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

    // BIT-99: credentials are stored encrypted; decrypt for OTA API use.
    // decrypt_if_available tolerates legacy plaintext rows (no "enc:" prefix).
    let crypto = IntegrationCrypto::try_from_env();
    let username = integrations::decrypt_if_available(
        crypto.as_ref(),
        &connection.access_token.clone().unwrap_or_default(),
    );
    let password = integrations::decrypt_if_available(
        crypto.as_ref(),
        &connection.refresh_token.clone().unwrap_or_default(),
    );

    let credentials = BookingCredentials::new(hotel_id.clone(), username, password);
    let client = BookingClient::new(credentials);

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
        .push_availability(&hotel_id, &availability_updates)
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

    validate_push_rates(&request)
        .map_err(|(code, msg)| (StatusCode::BAD_REQUEST, Json(ErrorResponse::new(code, msg))))?;

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

    // BIT-99: credentials are stored encrypted; decrypt for OTA API use.
    // decrypt_if_available tolerates legacy plaintext rows (no "enc:" prefix).
    let crypto = IntegrationCrypto::try_from_env();
    let username = integrations::decrypt_if_available(
        crypto.as_ref(),
        &connection.access_token.clone().unwrap_or_default(),
    );
    let password = integrations::decrypt_if_available(
        crypto.as_ref(),
        &connection.refresh_token.clone().unwrap_or_default(),
    );

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

    match client.push_rates(&hotel_id, &rate_updates).await {
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
    State(state): State<crate::state::AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
) -> Result<Json<Vec<PortalConnectionResponse>>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Listing portal connections"
    );

    // Issue #765: prevent cross-org IDOR — caller must belong to this org.
    verify_org_access(&state, auth.user_id, path.org_id).await?;

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
    State(state): State<crate::state::AppState>,
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

    // Issue #765: prevent cross-org IDOR — caller must belong to this org.
    verify_org_access(&state, auth.user_id, path.org_id).await?;

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
    State(state): State<crate::state::AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
    Query(_query): Query<PortalInquiryQuery>,
) -> Result<Json<Vec<PortalInquiryResponse>>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Listing portal inquiries"
    );

    // Issue #765: prevent cross-org IDOR — caller must belong to this org.
    verify_org_access(&state, auth.user_id, path.org_id).await?;

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

    // Issue #711: credentials are loaded once at server startup and cached
    // on AppState. Per-request env reads are gone — misconfiguration is
    // surfaced here (empty string -> NOT_CONFIGURED) but never round-trips
    // through `std::env::var`.
    let client_id = state.airbnb_config.client_id.clone();
    if client_id.is_empty() {
        tracing::error!("AIRBNB_CLIENT_ID is not configured");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "NOT_CONFIGURED",
                "Airbnb integration is not configured",
            )),
        ));
    }
    let client_secret = state.airbnb_config.client_secret.clone();
    if client_secret.is_empty() {
        tracing::error!("AIRBNB_CLIENT_SECRET is not configured");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "NOT_CONFIGURED",
                "Airbnb integration is not configured",
            )),
        ));
    }
    let redirect_uri = state.airbnb_config.redirect_uri.clone();

    // Verify the token is valid by fetching listings.
    let oauth_config = AirbnbOAuthConfig {
        client_id,
        client_secret,
        redirect_uri,
    };
    // Issue #2240: build via the base-URL seam so this write path is testable
    // against a stub server. `api_base` defaults to the production endpoint in
    // production (`AirbnbAppConfig::from_env`).
    let client = AirbnbClient::with_base_url(oauth_config, state.airbnb_config.api_base.clone());

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

    // Encrypt tokens before storage. Issue #765: encryption is MANDATORY for
    // persisted secrets — if INTEGRATION_ENCRYPTION_KEY is unset we fail closed
    // rather than storing tokens in plaintext.
    let crypto = IntegrationCrypto::try_from_env();
    let stored_access = encrypt_required(crypto.as_ref(), &request.access_token).map_err(|e| {
        tracing::error!(error = %e, "Refusing to store Airbnb access token without encryption");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "ENCRYPTION_REQUIRED",
                "Integration token encryption is not configured",
            )),
        )
    })?;
    let stored_refresh = encrypt_optional_required(
        crypto.as_ref(),
        request.refresh_token.as_deref(),
    )
    .map_err(|e| {
        tracing::error!(error = %e, "Refusing to store Airbnb refresh token without encryption");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "ENCRYPTION_REQUIRED",
                "Integration token encryption is not configured",
            )),
        )
    })?;

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

#[cfg(test)]
mod tests {
    //! Regression tests for the legacy Booking.com push request guards
    //! (issue #572, PR #607): the `MAX_BATCH_SIZE` batch-cap and the
    //! non-negative `available_count` validation on the `push-availability`
    //! / `push-rates` endpoints. These guards previously had no coverage —
    //! the unified `listing-push` surface (PR #1045) tests its own
    //! `booking_channel::validate_listing_push`, not these handlers.
    use super::*;
    use chrono::NaiveDate;

    /// Build `avail` availability-update DTOs, all with a non-negative count.
    fn avail_updates(avail: usize) -> Vec<AvailabilityUpdateDto> {
        (0..avail)
            .map(|i| AvailabilityUpdateDto {
                room_type_id: "DBL".to_string(),
                date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap()
                    + chrono::Duration::days(i as i64),
                available_count: 1,
                stop_sell: false,
                cta: false,
                ctd: false,
                min_los: None,
                max_los: None,
            })
            .collect()
    }

    /// Build a minimal availability push request with `avail` updates.
    fn make_availability_request(avail: usize) -> BookingPushAvailabilityRequest {
        BookingPushAvailabilityRequest {
            room_mappings: Vec::new(),
            updates: avail_updates(avail),
        }
    }

    /// Build a minimal rates push request with `rates` updates.
    fn make_rates_request(rates: usize) -> BookingPushRatesRequest {
        BookingPushRatesRequest {
            updates: (0..rates)
                .map(|i| RateUpdateDto {
                    room_type_id: "DBL".to_string(),
                    rate_plan_code: "STD".to_string(),
                    date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap()
                        + chrono::Duration::days(i as i64),
                    base_rate: rust_decimal::Decimal::new(10000, 2),
                    currency: "EUR".to_string(),
                    extra_person_rate: None,
                    extra_child_rate: None,
                })
                .collect(),
        }
    }

    // ---- push-availability guards ----

    #[test]
    fn availability_accepts_valid_batch() {
        let req = make_availability_request(10);
        assert!(validate_push_availability(&req).is_ok());
    }

    #[test]
    fn availability_accepts_max_batch_boundary() {
        // Exactly MAX_BATCH_SIZE updates must be allowed (boundary).
        let req = make_availability_request(MAX_BATCH_SIZE);
        assert!(validate_push_availability(&req).is_ok());
    }

    #[test]
    fn availability_rejects_empty_updates() {
        let req = make_availability_request(0);
        let err = validate_push_availability(&req).unwrap_err();
        assert_eq!(err.0, "NO_UPDATES");
    }

    #[test]
    fn availability_rejects_oversized_batch() {
        // One over the cap -> BATCH_TOO_LARGE (400).
        let req = make_availability_request(MAX_BATCH_SIZE + 1);
        let err = validate_push_availability(&req).unwrap_err();
        assert_eq!(err.0, "BATCH_TOO_LARGE");
    }

    #[test]
    fn availability_rejects_negative_available_count() {
        let mut req = make_availability_request(3);
        req.updates[2].available_count = -1;
        let err = validate_push_availability(&req).unwrap_err();
        assert_eq!(err.0, "INVALID_AVAILABLE_COUNT");
    }

    #[test]
    fn availability_zero_available_count_is_allowed() {
        // Zero (sold out) is non-negative and must pass the guard.
        let mut req = make_availability_request(1);
        req.updates[0].available_count = 0;
        assert!(validate_push_availability(&req).is_ok());
    }

    // ---- push-rates guards ----

    #[test]
    fn rates_accepts_valid_batch() {
        let req = make_rates_request(10);
        assert!(validate_push_rates(&req).is_ok());
    }

    #[test]
    fn rates_accepts_max_batch_boundary() {
        let req = make_rates_request(MAX_BATCH_SIZE);
        assert!(validate_push_rates(&req).is_ok());
    }

    #[test]
    fn rates_rejects_empty_updates() {
        let req = make_rates_request(0);
        let err = validate_push_rates(&req).unwrap_err();
        assert_eq!(err.0, "NO_UPDATES");
    }

    #[test]
    fn rates_rejects_oversized_batch() {
        let req = make_rates_request(MAX_BATCH_SIZE + 1);
        let err = validate_push_rates(&req).unwrap_err();
        assert_eq!(err.0, "BATCH_TOO_LARGE");
    }
}
