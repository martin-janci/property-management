//! Rental routes (Epic 18: Short-Term Rental Integration).
//!
//! Routes for Airbnb/Booking.com integration, guest registration, and authority reports.

use crate::state::AppState;
use api_core::extractors::TenantExtractor;
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::NaiveDate;
use db::models::{
    BookingListQuery, BookingWithGuests, BookingsResponse, CalendarBlock, CalendarEvent,
    ConnectionStatus, CreateBooking, CreateCalendarBlock, CreateGuest, CreateICalFeed,
    CreatePlatformConnection, GenerateReport, ICalFeed, PlatformConnectionSummary,
    PlatformSyncStatus, RentalBooking, RentalGuest, RentalGuestReport, RentalPlatformConnection,
    RentalStatistics, ReportPreview, ReportSummary, UpdateBooking, UpdateBookingStatus,
    UpdateGuest, UpdateICalFeed, UpdatePlatformConnection,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Create rentals router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Statistics & Dashboard
        .route("/statistics", get(get_statistics))
        .route("/sync-status", get(get_sync_status))
        // Platform Connections (Story 18.1)
        .route("/connections", get(list_connections))
        .route("/connections", post(create_connection))
        .route("/connections/{id}", get(get_connection))
        .route("/connections/{id}", put(update_connection))
        .route("/connections/{id}", delete(delete_connection))
        .route("/units/{unit_id}/connections", get(get_unit_connections))
        // OAuth Callbacks
        .route("/oauth/airbnb/callback", get(airbnb_callback))
        .route("/oauth/booking/callback", get(booking_callback))
        // Bookings (Story 18.2)
        .route("/bookings", get(list_bookings))
        .route("/bookings", post(create_booking))
        .route("/bookings/{id}", get(get_booking))
        .route("/bookings/{id}", put(update_booking))
        .route("/bookings/{id}/status", put(update_booking_status))
        .route("/bookings/{id}/guests", get(get_booking_with_guests))
        // Calendar
        .route("/calendar/{unit_id}", get(get_calendar))
        .route("/calendar/{unit_id}/availability", get(check_availability))
        .route("/calendar/blocks", post(create_calendar_block))
        .route("/calendar/blocks/{id}", delete(delete_calendar_block))
        // Guests (Story 18.3)
        .route("/guests", post(create_guest))
        .route("/guests/{id}", get(get_guest))
        .route("/guests/{id}", put(update_guest))
        .route("/guests/{id}", delete(delete_guest))
        .route("/guests/{id}/register", post(register_guest))
        .route("/checkin-reminders", get(get_checkin_reminders))
        // Reports (Story 18.4)
        .route("/reports", get(list_reports))
        .route("/reports/preview", post(generate_report_preview))
        .route("/reports", post(create_report))
        .route("/reports/{id}", get(get_report))
        .route("/reports/{id}/submit", post(submit_report))
        // iCal Feeds
        .route("/ical", post(create_ical_feed))
        .route("/ical/{id}", put(update_ical_feed))
        .route("/ical/{id}", delete(delete_ical_feed))
        .route("/units/{unit_id}/ical", get(get_unit_ical_feeds))
}

// ============================================
// Tenant-isolation guards (issue #804)
// ============================================
//
// SECURITY: the by-id read endpoints below previously took NO auth extractor
// (anonymous callers could exfiltrate OAuth tokens / guest PII by guessing a
// UUID), and the mutation endpoints extracted the tenant but discarded it
// (`_tenant`), so org B could read or mutate org A's rental data (cross-tenant
// IDOR). Each guard below confirms the addressed resource belongs to the
// caller's resolved tenant before the handler touches it; a foreign/unknown id
// is collapsed to `404` so existence is not probeable. Mirrors the
// `_belongs_to_org` idiom used by the fault / work-order / meter routes.

type RouteError = (axum::http::StatusCode, String);

fn authz_error(resource: &str, e: impl std::fmt::Display) -> RouteError {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        format!("Failed to authorize {resource}: {e}"),
    )
}

fn not_found(resource: &str) -> RouteError {
    (
        axum::http::StatusCode::NOT_FOUND,
        format!("{resource} not found"),
    )
}

/// Guard: `unit_id` must belong to the caller's tenant (via its building).
async fn require_unit_in_tenant(
    state: &AppState,
    org_id: Uuid,
    unit_id: Uuid,
) -> Result<(), RouteError> {
    if state
        .rental_repo
        .unit_belongs_to_org(unit_id, org_id)
        .await
        .map_err(|e| authz_error("unit", e))?
    {
        Ok(())
    } else {
        Err(not_found("Unit"))
    }
}

/// Guard: platform connection `id` must belong to the caller's tenant.
async fn require_connection_in_tenant(
    state: &AppState,
    org_id: Uuid,
    id: Uuid,
) -> Result<(), RouteError> {
    if state
        .rental_repo
        .connection_belongs_to_org(id, org_id)
        .await
        .map_err(|e| authz_error("connection", e))?
    {
        Ok(())
    } else {
        Err(not_found("Connection"))
    }
}

/// Guard: booking `id` must belong to the caller's tenant.
async fn require_booking_in_tenant(
    state: &AppState,
    org_id: Uuid,
    id: Uuid,
) -> Result<(), RouteError> {
    if state
        .rental_repo
        .booking_belongs_to_org(id, org_id)
        .await
        .map_err(|e| authz_error("booking", e))?
    {
        Ok(())
    } else {
        Err(not_found("Booking"))
    }
}

/// Guard: guest `id` must belong to the caller's tenant.
async fn require_guest_in_tenant(
    state: &AppState,
    org_id: Uuid,
    id: Uuid,
) -> Result<(), RouteError> {
    if state
        .rental_repo
        .guest_belongs_to_org(id, org_id)
        .await
        .map_err(|e| authz_error("guest", e))?
    {
        Ok(())
    } else {
        Err(not_found("Guest"))
    }
}

/// Guard: guest report `id` must belong to the caller's tenant.
async fn require_report_in_tenant(
    state: &AppState,
    org_id: Uuid,
    id: Uuid,
) -> Result<(), RouteError> {
    if state
        .rental_repo
        .report_belongs_to_org(id, org_id)
        .await
        .map_err(|e| authz_error("report", e))?
    {
        Ok(())
    } else {
        Err(not_found("Report"))
    }
}

/// Guard: iCal feed `id` must belong to the caller's tenant.
async fn require_ical_feed_in_tenant(
    state: &AppState,
    org_id: Uuid,
    id: Uuid,
) -> Result<(), RouteError> {
    if state
        .rental_repo
        .ical_feed_belongs_to_org(id, org_id)
        .await
        .map_err(|e| authz_error("feed", e))?
    {
        Ok(())
    } else {
        Err(not_found("Feed"))
    }
}

/// Guard: calendar block `id` must belong to the caller's tenant.
async fn require_calendar_block_in_tenant(
    state: &AppState,
    org_id: Uuid,
    id: Uuid,
) -> Result<(), RouteError> {
    if state
        .rental_repo
        .calendar_block_belongs_to_org(id, org_id)
        .await
        .map_err(|e| authz_error("block", e))?
    {
        Ok(())
    } else {
        Err(not_found("Block"))
    }
}

// ============================================
// Statistics & Dashboard
// ============================================

/// Get rental statistics.
#[utoipa::path(
    get,
    path = "/api/v1/rentals/statistics",
    tag = "Rentals",
    responses(
        (status = 200, description = "Rental statistics", body = RentalStatistics),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_statistics(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
) -> Result<Json<RentalStatistics>, (axum::http::StatusCode, String)> {
    let stats = state
        .rental_repo
        .get_statistics(tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get statistics: {}", e),
            )
        })?;

    Ok(Json(stats))
}

/// Get platform sync status.
#[utoipa::path(
    get,
    path = "/api/v1/rentals/sync-status",
    tag = "Rentals",
    responses(
        (status = 200, description = "Platform sync status", body = Vec<PlatformSyncStatus>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_sync_status(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
) -> Result<Json<Vec<PlatformSyncStatus>>, (axum::http::StatusCode, String)> {
    let status = state
        .rental_repo
        .get_platform_sync_status(tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get sync status: {}", e),
            )
        })?;

    Ok(Json(status))
}

// ============================================
// Platform Connections (Story 18.1)
// ============================================

/// List platform connections.
#[utoipa::path(
    get,
    path = "/api/v1/rentals/connections",
    tag = "Rentals",
    responses(
        (status = 200, description = "List of platform connections", body = Vec<PlatformConnectionSummary>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_connections(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
) -> Result<Json<Vec<PlatformConnectionSummary>>, (axum::http::StatusCode, String)> {
    let connections = state
        .rental_repo
        .get_connections_for_org(tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to list connections: {}", e),
            )
        })?;

    Ok(Json(connections))
}

/// Create platform connection.
#[utoipa::path(
    post,
    path = "/api/v1/rentals/connections",
    tag = "Rentals",
    request_body = CreatePlatformConnection,
    responses(
        (status = 201, description = "Connection created", body = RentalPlatformConnection),
        (status = 400, description = "Invalid data"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn create_connection(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Json(data): Json<CreatePlatformConnection>,
) -> Result<Json<RentalPlatformConnection>, (axum::http::StatusCode, String)> {
    // SECURITY (issue #804): the unit_id is client-supplied — verify it belongs
    // to the caller's tenant so a connection can't be attached to another org's unit.
    require_unit_in_tenant(&state, tenant.tenant_id, data.unit_id).await?;
    let connection = state
        .rental_repo
        .create_connection(tenant.tenant_id, data)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create connection: {}", e),
            )
        })?;

    Ok(Json(connection))
}

/// Get platform connection.
#[utoipa::path(
    get,
    path = "/api/v1/rentals/connections/{id}",
    tag = "Rentals",
    params(("id" = Uuid, Path, description = "Connection ID")),
    responses(
        (status = 200, description = "Connection details", body = RentalPlatformConnection),
        (status = 404, description = "Connection not found")
    )
)]
pub async fn get_connection(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<RentalPlatformConnection>, (axum::http::StatusCode, String)> {
    // SECURITY (issue #804): this handler took no auth extractor — any anonymous
    // caller could read a connection (incl. stored OAuth tokens) by guessing its
    // UUID. Require a tenant, authorize the connection against it, and strip the
    // OAuth credentials from the response (data minimization).
    require_connection_in_tenant(&state, tenant.tenant_id, id).await?;

    let connection = state
        .rental_repo
        .find_connection_by_id(id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get connection: {}", e),
            )
        })?;

    match connection {
        Some(mut c) => {
            // Never return stored OAuth credentials over the API.
            c.access_token = None;
            c.refresh_token = None;
            c.token_expires_at = None;
            Ok(Json(c))
        }
        None => Err((
            axum::http::StatusCode::NOT_FOUND,
            "Connection not found".to_string(),
        )),
    }
}

/// Update platform connection.
#[utoipa::path(
    put,
    path = "/api/v1/rentals/connections/{id}",
    tag = "Rentals",
    params(("id" = Uuid, Path, description = "Connection ID")),
    request_body = UpdatePlatformConnection,
    responses(
        (status = 200, description = "Connection updated", body = RentalPlatformConnection),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Connection not found")
    )
)]
pub async fn update_connection(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdatePlatformConnection>,
) -> Result<Json<RentalPlatformConnection>, (axum::http::StatusCode, String)> {
    // SECURITY (issue #804): verify the connection belongs to the caller's tenant.
    require_connection_in_tenant(&state, tenant.tenant_id, id).await?;
    let connection = state
        .rental_repo
        .update_connection(id, data)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update connection: {}", e),
            )
        })?;

    Ok(Json(connection))
}

/// Delete platform connection.
#[utoipa::path(
    delete,
    path = "/api/v1/rentals/connections/{id}",
    tag = "Rentals",
    params(("id" = Uuid, Path, description = "Connection ID")),
    responses(
        (status = 204, description = "Connection deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Connection not found")
    )
)]
pub async fn delete_connection(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    // SECURITY (issue #804): verify the connection belongs to the caller's tenant.
    require_connection_in_tenant(&state, tenant.tenant_id, id).await?;
    let deleted = state.rental_repo.delete_connection(id).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to delete connection: {}", e),
        )
    })?;

    if !deleted {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "Connection not found".to_string(),
        ));
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// Get connections for a unit.
#[utoipa::path(
    get,
    path = "/api/v1/rentals/units/{unit_id}/connections",
    tag = "Rentals",
    params(("unit_id" = Uuid, Path, description = "Unit ID")),
    responses(
        (status = 200, description = "Connection statuses", body = Vec<ConnectionStatus>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_unit_connections(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(unit_id): Path<Uuid>,
) -> Result<Json<Vec<ConnectionStatus>>, (axum::http::StatusCode, String)> {
    // SECURITY (issue #804): verify the unit belongs to the caller's tenant.
    require_unit_in_tenant(&state, tenant.tenant_id, unit_id).await?;
    let connections = state
        .rental_repo
        .get_connections_for_unit(unit_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get connections: {}", e),
            )
        })?;

    Ok(Json(connections))
}

// ============================================
// OAuth Callbacks
// ============================================

/// OAuth callback query params.
#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

/// Airbnb OAuth callback.
///
/// Story 98.2: Rental Platform OAuth Implementation
pub async fn airbnb_callback(
    State(state): State<AppState>,
    Query(params): Query<OAuthCallbackQuery>,
) -> Result<axum::response::Redirect, (axum::http::StatusCode, String)> {
    use integrations::{encrypt_if_available, AirbnbClient, AirbnbOAuthConfig, IntegrationCrypto};

    // Check for OAuth error
    if let Some(error) = params.error {
        tracing::error!("Airbnb OAuth error: {}", error);
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            format!("OAuth error: {}", error),
        ));
    }

    // Validate required parameters
    let code = params.code.ok_or_else(|| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Missing authorization code".to_string(),
        )
    })?;

    let oauth_state = params.state.ok_or_else(|| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Missing state parameter".to_string(),
        )
    })?;

    // Parse the OAuth state which contains the connection ID
    // Format: "{connection_id}:{random_nonce}"
    let connection_id: Uuid = oauth_state
        .split(':')
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| {
            tracing::warn!("Invalid OAuth state format: {}", oauth_state);
            (
                axum::http::StatusCode::BAD_REQUEST,
                "Invalid state parameter format".to_string(),
            )
        })?;

    // Find the connection by ID
    let connection = state
        .rental_repo
        .find_connection_by_id(connection_id)
        .await
        .map_err(|e| {
            tracing::error!("Database error finding connection: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?
        .ok_or_else(|| {
            tracing::warn!("Connection not found: {}", connection_id);
            (
                axum::http::StatusCode::BAD_REQUEST,
                "Connection not found".to_string(),
            )
        })?;

    // Issue #711: Airbnb OAuth credentials live on AppState (loaded once at
    // boot). Missing values still surface as 500 NOT_CONFIGURED, but the
    // per-request `std::env::var` round-trip is gone.
    let client_id = state.airbnb_config.client_id.clone();
    if client_id.is_empty() {
        tracing::error!("AIRBNB_CLIENT_ID not configured");
        return Err((
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Airbnb OAuth not configured".to_string(),
        ));
    }
    let client_secret = state.airbnb_config.client_secret.clone();
    if client_secret.is_empty() {
        tracing::error!("AIRBNB_CLIENT_SECRET not configured");
        return Err((
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Airbnb OAuth not configured".to_string(),
        ));
    }
    let redirect_uri = if state.airbnb_config.redirect_uri.is_empty() {
        "https://ppt.three-two-bit.com/api/v1/rentals/oauth/airbnb/callback".to_string()
    } else {
        state.airbnb_config.redirect_uri.clone()
    };

    // Exchange code for tokens
    let airbnb_config = AirbnbOAuthConfig {
        client_id,
        client_secret,
        redirect_uri,
    };
    let airbnb_client = AirbnbClient::new(airbnb_config);

    let tokens = airbnb_client.exchange_code(&code).await.map_err(|e| {
        tracing::error!("Airbnb token exchange failed: {}", e);
        (
            axum::http::StatusCode::BAD_REQUEST,
            format!("Token exchange failed: {}", e),
        )
    })?;

    // Encrypt tokens for storage
    let crypto = IntegrationCrypto::try_from_env();
    let access_encrypted = encrypt_if_available(crypto.as_ref(), &tokens.access_token);
    let refresh_encrypted = tokens
        .refresh_token
        .as_ref()
        .map(|rt| encrypt_if_available(crypto.as_ref(), rt));

    // Update connection with tokens and mark as connected
    state
        .rental_repo
        .update_connection_tokens(
            connection.id,
            &access_encrypted,
            refresh_encrypted.as_deref(),
            tokens.expires_at,
            true, // is_connected
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to update connection tokens: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to save tokens".to_string(),
            )
        })?;

    tracing::info!(
        "Airbnb OAuth completed successfully for connection {}",
        connection.id
    );

    // Redirect to success page
    let success_url = std::env::var("FRONTEND_SUCCESS_URL").unwrap_or_else(|_| {
        "https://ppt.three-two-bit.com/rentals/connections?success=airbnb".to_string()
    });
    Ok(axum::response::Redirect::to(&success_url))
}

/// Booking.com OAuth callback.
///
/// Story 98.2: Rental Platform OAuth Implementation
pub async fn booking_callback(
    State(state): State<AppState>,
    Query(params): Query<OAuthCallbackQuery>,
) -> Result<axum::response::Redirect, (axum::http::StatusCode, String)> {
    use integrations::{
        encrypt_if_available, BookingClient, BookingCredentials, IntegrationCrypto,
    };

    // Check for OAuth error
    if let Some(error) = params.error {
        tracing::error!("Booking.com OAuth error: {}", error);
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            format!("OAuth error: {}", error),
        ));
    }

    // Validate required parameters
    let code = params.code.ok_or_else(|| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Missing authorization code".to_string(),
        )
    })?;

    let oauth_state = params.state.ok_or_else(|| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Missing state parameter".to_string(),
        )
    })?;

    // Parse the OAuth state which contains the connection ID
    // Format: "{connection_id}:{random_nonce}"
    let connection_id: Uuid = oauth_state
        .split(':')
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| {
            tracing::warn!("Invalid OAuth state format: {}", oauth_state);
            (
                axum::http::StatusCode::BAD_REQUEST,
                "Invalid state parameter format".to_string(),
            )
        })?;

    // Find the connection by ID
    let connection = state
        .rental_repo
        .find_connection_by_id(connection_id)
        .await
        .map_err(|e| {
            tracing::error!("Database error finding connection: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?
        .ok_or_else(|| {
            tracing::warn!("Connection not found: {}", connection_id);
            (
                axum::http::StatusCode::BAD_REQUEST,
                "Connection not found".to_string(),
            )
        })?;

    // Get Booking.com OAuth configuration from environment
    let username = std::env::var("BOOKING_USERNAME").map_err(|_| {
        tracing::error!("BOOKING_USERNAME not configured");
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Booking.com OAuth not configured".to_string(),
        )
    })?;

    let password = std::env::var("BOOKING_PASSWORD").map_err(|_| {
        tracing::error!("BOOKING_PASSWORD not configured");
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Booking.com OAuth not configured".to_string(),
        )
    })?;

    let hotel_id = std::env::var("BOOKING_HOTEL_ID").unwrap_or_else(|_| "default".to_string());

    let api_url = std::env::var("BOOKING_API_URL")
        .unwrap_or_else(|_| "https://supply-xml.booking.com".to_string());

    // Booking.com uses API credentials rather than OAuth tokens
    // The 'code' here represents a confirmation/authorization code
    let credentials = BookingCredentials {
        username,
        password,
        hotel_id,
        api_url,
    };

    let booking_client = BookingClient::new(credentials);

    // Validate credentials by attempting to fetch properties
    let validation_result = booking_client.fetch_properties().await;
    if let Err(e) = validation_result {
        tracing::error!("Booking.com credential validation failed: {}", e);
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            format!("Credential validation failed: {}", e),
        ));
    }

    // Encrypt authorization code for storage
    let crypto = IntegrationCrypto::try_from_env();
    let code_encrypted = encrypt_if_available(crypto.as_ref(), &code);

    // Update connection with credentials and mark as connected
    state
        .rental_repo
        .update_connection_tokens(
            connection.id,
            &code_encrypted,
            None,
            None,
            true, // is_connected
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to update connection credentials: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to save credentials".to_string(),
            )
        })?;

    tracing::info!(
        "Booking.com OAuth completed successfully for connection {}",
        connection.id
    );

    // Redirect to success page
    let success_url = std::env::var("FRONTEND_SUCCESS_URL").unwrap_or_else(|_| {
        "https://ppt.three-two-bit.com/rentals/connections?success=booking".to_string()
    });
    Ok(axum::response::Redirect::to(&success_url))
}

// ============================================
// Bookings (Story 18.2)
// ============================================

/// List bookings.
#[utoipa::path(
    get,
    path = "/api/v1/rentals/bookings",
    tag = "Rentals",
    params(
        ("unit_id" = Option<Uuid>, Query, description = "Filter by unit"),
        ("building_id" = Option<Uuid>, Query, description = "Filter by building"),
        ("platform" = Option<String>, Query, description = "Filter by platform"),
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("from_date" = Option<NaiveDate>, Query, description = "Start date filter"),
        ("to_date" = Option<NaiveDate>, Query, description = "End date filter"),
        ("page" = Option<i32>, Query, description = "Page number"),
        ("limit" = Option<i32>, Query, description = "Items per page")
    ),
    responses(
        (status = 200, description = "List of bookings", body = BookingsResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_bookings(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Query(query): Query<BookingListQuery>,
) -> Result<Json<BookingsResponse>, (axum::http::StatusCode, String)> {
    let (bookings, total) = state
        .rental_repo
        .list_bookings(tenant.tenant_id, query.clone())
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to list bookings: {}", e),
            )
        })?;

    Ok(Json(BookingsResponse {
        bookings,
        total,
        page: query.page.unwrap_or(1),
        limit: query.limit.unwrap_or(20),
    }))
}

/// Create booking.
#[utoipa::path(
    post,
    path = "/api/v1/rentals/bookings",
    tag = "Rentals",
    request_body = CreateBooking,
    responses(
        (status = 201, description = "Booking created", body = RentalBooking),
        (status = 400, description = "Invalid data or dates conflict"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn create_booking(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Json(data): Json<CreateBooking>,
) -> Result<Json<RentalBooking>, (axum::http::StatusCode, String)> {
    // SECURITY (issue #804): the unit_id is client-supplied — verify it belongs
    // to the caller's tenant before booking against it.
    require_unit_in_tenant(&state, tenant.tenant_id, data.unit_id).await?;
    // Check availability first
    let available = state
        .rental_repo
        .check_availability(data.unit_id, data.check_in, data.check_out)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to check availability: {}", e),
            )
        })?;

    if !available {
        return Err((
            axum::http::StatusCode::CONFLICT,
            "Unit is not available for selected dates".to_string(),
        ));
    }

    let booking = state
        .rental_repo
        .create_booking(tenant.tenant_id, data)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create booking: {}", e),
            )
        })?;

    Ok(Json(booking))
}

/// Get booking.
#[utoipa::path(
    get,
    path = "/api/v1/rentals/bookings/{id}",
    tag = "Rentals",
    params(("id" = Uuid, Path, description = "Booking ID")),
    responses(
        (status = 200, description = "Booking details", body = RentalBooking),
        (status = 404, description = "Booking not found")
    )
)]
pub async fn get_booking(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<RentalBooking>, (axum::http::StatusCode, String)> {
    // SECURITY (issue #804): this handler took no auth extractor — booking
    // records (guest names, contact, financials) leaked to anonymous callers.
    require_booking_in_tenant(&state, tenant.tenant_id, id).await?;
    let booking = state
        .rental_repo
        .find_booking_by_id(id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get booking: {}", e),
            )
        })?;

    match booking {
        Some(b) => Ok(Json(b)),
        None => Err((
            axum::http::StatusCode::NOT_FOUND,
            "Booking not found".to_string(),
        )),
    }
}

/// Update booking.
#[utoipa::path(
    put,
    path = "/api/v1/rentals/bookings/{id}",
    tag = "Rentals",
    params(("id" = Uuid, Path, description = "Booking ID")),
    request_body = UpdateBooking,
    responses(
        (status = 200, description = "Booking updated", body = RentalBooking),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Booking not found")
    )
)]
pub async fn update_booking(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateBooking>,
) -> Result<Json<RentalBooking>, (axum::http::StatusCode, String)> {
    // SECURITY (issue #804): verify the booking belongs to the caller's tenant.
    require_booking_in_tenant(&state, tenant.tenant_id, id).await?;
    let booking = state
        .rental_repo
        .update_booking(id, data)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update booking: {}", e),
            )
        })?;

    Ok(Json(booking))
}

/// Update booking status.
#[utoipa::path(
    put,
    path = "/api/v1/rentals/bookings/{id}/status",
    tag = "Rentals",
    params(("id" = Uuid, Path, description = "Booking ID")),
    request_body = UpdateBookingStatus,
    responses(
        (status = 200, description = "Status updated", body = RentalBooking),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Booking not found")
    )
)]
pub async fn update_booking_status(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateBookingStatus>,
) -> Result<Json<RentalBooking>, (axum::http::StatusCode, String)> {
    // SECURITY (issue #804): verify the booking belongs to the caller's tenant.
    require_booking_in_tenant(&state, tenant.tenant_id, id).await?;
    let booking = state
        .rental_repo
        .update_booking_status(id, data)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update status: {}", e),
            )
        })?;

    Ok(Json(booking))
}

/// Get booking with guests.
#[utoipa::path(
    get,
    path = "/api/v1/rentals/bookings/{id}/guests",
    tag = "Rentals",
    params(("id" = Uuid, Path, description = "Booking ID")),
    responses(
        (status = 200, description = "Booking with guests", body = BookingWithGuests),
        (status = 404, description = "Booking not found")
    )
)]
pub async fn get_booking_with_guests(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<BookingWithGuests>, (axum::http::StatusCode, String)> {
    // SECURITY (issue #804): this handler took no auth extractor — booking +
    // guest PII leaked to anonymous callers.
    require_booking_in_tenant(&state, tenant.tenant_id, id).await?;
    let result = state
        .rental_repo
        .get_booking_with_guests(id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get booking: {}", e),
            )
        })?;

    match result {
        Some(b) => Ok(Json(b)),
        None => Err((
            axum::http::StatusCode::NOT_FOUND,
            "Booking not found".to_string(),
        )),
    }
}

// ============================================
// Calendar
// ============================================

/// Calendar query params.
#[derive(Debug, Deserialize)]
pub struct CalendarQuery {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

/// Get calendar events.
#[utoipa::path(
    get,
    path = "/api/v1/rentals/calendar/{unit_id}",
    tag = "Rentals",
    params(
        ("unit_id" = Uuid, Path, description = "Unit ID"),
        ("start_date" = NaiveDate, Query, description = "Start date"),
        ("end_date" = NaiveDate, Query, description = "End date")
    ),
    responses(
        (status = 200, description = "Calendar events", body = Vec<CalendarEvent>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_calendar(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(unit_id): Path<Uuid>,
    Query(query): Query<CalendarQuery>,
) -> Result<Json<Vec<CalendarEvent>>, (axum::http::StatusCode, String)> {
    // SECURITY (issue #804): verify the unit belongs to the caller's tenant.
    require_unit_in_tenant(&state, tenant.tenant_id, unit_id).await?;
    let events = state
        .rental_repo
        .get_calendar_events(unit_id, query.start_date, query.end_date)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get calendar: {}", e),
            )
        })?;

    Ok(Json(events))
}

/// Availability check query params.
#[derive(Debug, Deserialize)]
pub struct AvailabilityQuery {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

/// Availability response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AvailabilityResponse {
    pub available: bool,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

/// Check availability.
#[utoipa::path(
    get,
    path = "/api/v1/rentals/calendar/{unit_id}/availability",
    tag = "Rentals",
    params(
        ("unit_id" = Uuid, Path, description = "Unit ID"),
        ("start_date" = NaiveDate, Query, description = "Start date"),
        ("end_date" = NaiveDate, Query, description = "End date")
    ),
    responses(
        (status = 200, description = "Availability status", body = AvailabilityResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn check_availability(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(unit_id): Path<Uuid>,
    Query(query): Query<AvailabilityQuery>,
) -> Result<Json<AvailabilityResponse>, (axum::http::StatusCode, String)> {
    // SECURITY (issue #804): verify the unit belongs to the caller's tenant.
    require_unit_in_tenant(&state, tenant.tenant_id, unit_id).await?;
    let available = state
        .rental_repo
        .check_availability(unit_id, query.start_date, query.end_date)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to check availability: {}", e),
            )
        })?;

    Ok(Json(AvailabilityResponse {
        available,
        start_date: query.start_date,
        end_date: query.end_date,
    }))
}

/// Create calendar block.
#[utoipa::path(
    post,
    path = "/api/v1/rentals/calendar/blocks",
    tag = "Rentals",
    request_body = CreateCalendarBlock,
    responses(
        (status = 201, description = "Block created", body = CalendarBlock),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn create_calendar_block(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Json(data): Json<CreateCalendarBlock>,
) -> Result<Json<CalendarBlock>, (axum::http::StatusCode, String)> {
    // SECURITY (issue #804): verify the client-supplied unit_id belongs to the tenant.
    require_unit_in_tenant(&state, tenant.tenant_id, data.unit_id).await?;
    let block = state
        .rental_repo
        .create_calendar_block(tenant.tenant_id, data)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create block: {}", e),
            )
        })?;

    Ok(Json(block))
}

/// Delete calendar block.
#[utoipa::path(
    delete,
    path = "/api/v1/rentals/calendar/blocks/{id}",
    tag = "Rentals",
    params(("id" = Uuid, Path, description = "Block ID")),
    responses(
        (status = 204, description = "Block deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Block not found")
    )
)]
pub async fn delete_calendar_block(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    // SECURITY (issue #804): verify the block belongs to the caller's tenant.
    require_calendar_block_in_tenant(&state, tenant.tenant_id, id).await?;
    let deleted = state
        .rental_repo
        .delete_calendar_block(id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to delete block: {}", e),
            )
        })?;

    if !deleted {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "Block not found or is linked to a booking".to_string(),
        ));
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}

// ============================================
// Guests (Story 18.3)
// ============================================

/// Create guest.
#[utoipa::path(
    post,
    path = "/api/v1/rentals/guests",
    tag = "Rentals",
    request_body = CreateGuest,
    responses(
        (status = 201, description = "Guest created", body = RentalGuest),
        (status = 400, description = "Invalid data"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn create_guest(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Json(data): Json<CreateGuest>,
) -> Result<Json<RentalGuest>, (axum::http::StatusCode, String)> {
    let guest = state
        .rental_repo
        .create_guest(tenant.tenant_id, data)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create guest: {}", e),
            )
        })?;

    Ok(Json(guest))
}

/// Get guest.
#[utoipa::path(
    get,
    path = "/api/v1/rentals/guests/{id}",
    tag = "Rentals",
    params(("id" = Uuid, Path, description = "Guest ID")),
    responses(
        (status = 200, description = "Guest details", body = RentalGuest),
        (status = 404, description = "Guest not found")
    )
)]
pub async fn get_guest(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<RentalGuest>, (axum::http::StatusCode, String)> {
    // SECURITY (issue #804): this handler took no auth extractor — guest PII
    // (name, document numbers, contact) leaked to anonymous callers.
    require_guest_in_tenant(&state, tenant.tenant_id, id).await?;
    let guest = state.rental_repo.find_guest_by_id(id).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get guest: {}", e),
        )
    })?;

    match guest {
        Some(g) => Ok(Json(g)),
        None => Err((
            axum::http::StatusCode::NOT_FOUND,
            "Guest not found".to_string(),
        )),
    }
}

/// Update guest.
#[utoipa::path(
    put,
    path = "/api/v1/rentals/guests/{id}",
    tag = "Rentals",
    params(("id" = Uuid, Path, description = "Guest ID")),
    request_body = UpdateGuest,
    responses(
        (status = 200, description = "Guest updated", body = RentalGuest),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Guest not found")
    )
)]
pub async fn update_guest(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateGuest>,
) -> Result<Json<RentalGuest>, (axum::http::StatusCode, String)> {
    // SECURITY (issue #804): verify the guest belongs to the caller's tenant.
    require_guest_in_tenant(&state, tenant.tenant_id, id).await?;
    let guest = state
        .rental_repo
        .update_guest(id, data)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update guest: {}", e),
            )
        })?;

    Ok(Json(guest))
}

/// Delete guest.
#[utoipa::path(
    delete,
    path = "/api/v1/rentals/guests/{id}",
    tag = "Rentals",
    params(("id" = Uuid, Path, description = "Guest ID")),
    responses(
        (status = 204, description = "Guest deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Guest not found")
    )
)]
pub async fn delete_guest(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    // SECURITY (issue #804): verify the guest belongs to the caller's tenant.
    require_guest_in_tenant(&state, tenant.tenant_id, id).await?;
    let deleted = state.rental_repo.delete_guest(id).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to delete guest: {}", e),
        )
    })?;

    if !deleted {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "Guest not found".to_string(),
        ));
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// Register guest.
#[utoipa::path(
    post,
    path = "/api/v1/rentals/guests/{id}/register",
    tag = "Rentals",
    params(("id" = Uuid, Path, description = "Guest ID")),
    responses(
        (status = 200, description = "Guest registered", body = RentalGuest),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Guest not found")
    )
)]
pub async fn register_guest(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<RentalGuest>, (axum::http::StatusCode, String)> {
    // SECURITY (issue #804): verify the guest belongs to the caller's tenant.
    require_guest_in_tenant(&state, tenant.tenant_id, id).await?;
    let guest = state.rental_repo.register_guest(id).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to register guest: {}", e),
        )
    })?;

    Ok(Json(guest))
}

/// Check-in reminders query params.
#[derive(Debug, Deserialize)]
pub struct CheckInRemindersQuery {
    #[serde(default = "default_days_ahead")]
    pub days_ahead: i32,
}

fn default_days_ahead() -> i32 {
    1
}

/// Get check-in reminders.
#[utoipa::path(
    get,
    path = "/api/v1/rentals/checkin-reminders",
    tag = "Rentals",
    params(("days_ahead" = Option<i32>, Query, description = "Days ahead to check")),
    responses(
        (status = 200, description = "Check-in reminders", body = Vec<db::models::CheckInReminder>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_checkin_reminders(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Query(query): Query<CheckInRemindersQuery>,
) -> Result<Json<Vec<db::models::CheckInReminder>>, (axum::http::StatusCode, String)> {
    let reminders = state
        .rental_repo
        .get_upcoming_checkins_needing_registration(tenant.tenant_id, query.days_ahead)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get reminders: {}", e),
            )
        })?;

    Ok(Json(reminders))
}

// ============================================
// Reports (Story 18.4)
// ============================================

/// List reports query params.
#[derive(Debug, Deserialize)]
pub struct ListReportsQuery {
    pub building_id: Option<Uuid>,
    #[serde(default = "default_reports_limit")]
    pub limit: i32,
}

fn default_reports_limit() -> i32 {
    20
}

/// List reports.
#[utoipa::path(
    get,
    path = "/api/v1/rentals/reports",
    tag = "Rentals",
    params(
        ("building_id" = Option<Uuid>, Query, description = "Filter by building"),
        ("limit" = Option<i32>, Query, description = "Limit results")
    ),
    responses(
        (status = 200, description = "List of reports", body = Vec<ReportSummary>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_reports(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Query(query): Query<ListReportsQuery>,
) -> Result<Json<Vec<ReportSummary>>, (axum::http::StatusCode, String)> {
    let reports = state
        .rental_repo
        .list_reports(tenant.tenant_id, query.building_id, query.limit)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to list reports: {}", e),
            )
        })?;

    Ok(Json(reports))
}

/// Preview request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ReportPreviewRequest {
    pub building_id: Uuid,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
}

/// Generate report preview.
#[utoipa::path(
    post,
    path = "/api/v1/rentals/reports/preview",
    tag = "Rentals",
    request_body = ReportPreviewRequest,
    responses(
        (status = 200, description = "Report preview", body = ReportPreview),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn generate_report_preview(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Json(data): Json<ReportPreviewRequest>,
) -> Result<Json<ReportPreview>, (axum::http::StatusCode, String)> {
    let preview = state
        .rental_repo
        .generate_report_preview(
            tenant.tenant_id,
            data.building_id,
            data.period_start,
            data.period_end,
        )
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to generate preview: {}", e),
            )
        })?;

    Ok(Json(preview))
}

/// Create report.
#[utoipa::path(
    post,
    path = "/api/v1/rentals/reports",
    tag = "Rentals",
    request_body = GenerateReport,
    responses(
        (status = 201, description = "Report created", body = RentalGuestReport),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn create_report(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Json(data): Json<GenerateReport>,
) -> Result<Json<RentalGuestReport>, (axum::http::StatusCode, String)> {
    let report = state
        .rental_repo
        .create_report(tenant.tenant_id, data, tenant.user_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create report: {}", e),
            )
        })?;

    Ok(Json(report))
}

/// Get report.
#[utoipa::path(
    get,
    path = "/api/v1/rentals/reports/{id}",
    tag = "Rentals",
    params(("id" = Uuid, Path, description = "Report ID")),
    responses(
        (status = 200, description = "Report details", body = RentalGuestReport),
        (status = 404, description = "Report not found")
    )
)]
pub async fn get_report(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<RentalGuestReport>, (axum::http::StatusCode, String)> {
    // SECURITY (issue #804): this handler took no auth extractor — authority
    // guest reports leaked to anonymous callers.
    require_report_in_tenant(&state, tenant.tenant_id, id).await?;
    let report = state.rental_repo.find_report_by_id(id).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get report: {}", e),
        )
    })?;

    match report {
        Some(r) => Ok(Json(r)),
        None => Err((
            axum::http::StatusCode::NOT_FOUND,
            "Report not found".to_string(),
        )),
    }
}

/// Submit report.
#[utoipa::path(
    post,
    path = "/api/v1/rentals/reports/{id}/submit",
    tag = "Rentals",
    params(("id" = Uuid, Path, description = "Report ID")),
    responses(
        (status = 200, description = "Report submitted", body = RentalGuestReport),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Report not found")
    )
)]
pub async fn submit_report(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<RentalGuestReport>, (axum::http::StatusCode, String)> {
    // SECURITY (issue #804): verify the report belongs to the caller's tenant.
    require_report_in_tenant(&state, tenant.tenant_id, id).await?;
    let report = state
        .rental_repo
        .submit_report(id, tenant.user_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to submit report: {}", e),
            )
        })?;

    Ok(Json(report))
}

// ============================================
// iCal Feeds
// ============================================

/// Create iCal feed.
#[utoipa::path(
    post,
    path = "/api/v1/rentals/ical",
    tag = "Rentals",
    request_body = CreateICalFeed,
    responses(
        (status = 201, description = "Feed created", body = ICalFeed),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn create_ical_feed(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Json(data): Json<CreateICalFeed>,
) -> Result<Json<ICalFeed>, (axum::http::StatusCode, String)> {
    // SECURITY (issue #804): verify the client-supplied unit_id belongs to the tenant.
    require_unit_in_tenant(&state, tenant.tenant_id, data.unit_id).await?;
    let feed = state
        .rental_repo
        .create_ical_feed(tenant.tenant_id, data)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create feed: {}", e),
            )
        })?;

    Ok(Json(feed))
}

/// Update iCal feed.
#[utoipa::path(
    put,
    path = "/api/v1/rentals/ical/{id}",
    tag = "Rentals",
    params(("id" = Uuid, Path, description = "Feed ID")),
    request_body = UpdateICalFeed,
    responses(
        (status = 200, description = "Feed updated", body = ICalFeed),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Feed not found")
    )
)]
pub async fn update_ical_feed(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateICalFeed>,
) -> Result<Json<ICalFeed>, (axum::http::StatusCode, String)> {
    // SECURITY (issue #804): verify the feed belongs to the caller's tenant.
    require_ical_feed_in_tenant(&state, tenant.tenant_id, id).await?;
    let feed = state
        .rental_repo
        .update_ical_feed(id, data)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update feed: {}", e),
            )
        })?;

    Ok(Json(feed))
}

/// Delete iCal feed.
#[utoipa::path(
    delete,
    path = "/api/v1/rentals/ical/{id}",
    tag = "Rentals",
    params(("id" = Uuid, Path, description = "Feed ID")),
    responses(
        (status = 204, description = "Feed deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Feed not found")
    )
)]
pub async fn delete_ical_feed(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    // SECURITY (issue #804): verify the feed belongs to the caller's tenant.
    require_ical_feed_in_tenant(&state, tenant.tenant_id, id).await?;
    let deleted = state.rental_repo.delete_ical_feed(id).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to delete feed: {}", e),
        )
    })?;

    if !deleted {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "Feed not found".to_string(),
        ));
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// Get iCal feeds for unit.
#[utoipa::path(
    get,
    path = "/api/v1/rentals/units/{unit_id}/ical",
    tag = "Rentals",
    params(("unit_id" = Uuid, Path, description = "Unit ID")),
    responses(
        (status = 200, description = "List of feeds", body = Vec<ICalFeed>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_unit_ical_feeds(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(unit_id): Path<Uuid>,
) -> Result<Json<Vec<ICalFeed>>, (axum::http::StatusCode, String)> {
    // SECURITY (issue #804): verify the unit belongs to the caller's tenant.
    require_unit_in_tenant(&state, tenant.tenant_id, unit_id).await?;
    let feeds = state
        .rental_repo
        .get_ical_feeds_for_unit(unit_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get feeds: {}", e),
            )
        })?;

    Ok(Json(feeds))
}
