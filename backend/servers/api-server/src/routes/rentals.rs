//! Rental routes (Epic 18: Short-Term Rental Integration).
//!
//! Routes for Airbnb/Booking.com integration, guest registration, and authority reports.

use crate::services::id_ocr::{GuestIdExtraction, IdOcrError};
use crate::state::AppState;
use api_core::extractors::TenantExtractor;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use axum_extra::extract::Multipart;
use chrono::NaiveDate;
use common::errors::ErrorResponse;
use common::tenant::TenantRole;
use db::models::{
    BookingListQuery, BookingWithGuests, BookingsResponse, CalendarBlock, CalendarEvent,
    ConnectionStatus, CreateBooking, CreateCalendarBlock, CreateGuest, CreateGuestIdDocument,
    CreateICalFeed, CreatePlatformConnection, GenerateReport, ICalFeed, PlatformConnectionDetail,
    PlatformConnectionSummary, PlatformSyncStatus, RentalBooking, RentalGuest,
    RentalGuestIdDocument, RentalGuestReport, RentalPlatformConnection, RentalStatistics,
    ReportPreview, ReportSummary, UpdateBooking, UpdateBookingStatus, UpdateGuest, UpdateICalFeed,
    UpdatePlatformConnection,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Maximum accepted ID-document size: ~10 MiB (#1687).
const MAX_ID_DOC_SIZE: usize = 10 * 1024 * 1024;

/// MIME allowlist for ID-document uploads (#1687).
const ALLOWED_ID_DOC_MIME: &[&str] = &["image/png", "image/jpeg", "application/pdf"];

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
        // OAuth callbacks: the legacy Story 98.2 unauthenticated callbacks
        // (`/oauth/{airbnb,booking}/callback`) were removed (PAP-141). They
        // trusted a client-supplied connection id from the OAuth `state`
        // parameter with no auth, no org check, and no CSRF-state validation,
        // letting any caller bind tokens to another org's connection. The
        // Airbnb flow is served securely by Story 83.1
        // (`/api/v1/integrations/organizations/{org_id}/airbnb/callback`,
        // Redis single-use `oauth_state` + `verify_org_access`).
        //
        // The Booking.com connect flow is likewise served securely by the
        // authenticated, org-scoped integrations routes
        // (`POST /api/v1/integrations/organizations/{org_id}/booking/connect`,
        // `connect_booking`, behind `verify_org_access`), with credential
        // token-exchange at `.../booking/token/exchange` (#1374). The legacy
        // basic-auth credential persistence is interim; the migration to a real
        // Booking.com OAuth/Connectivity flow (real access+refresh token
        // rotation, replacing the repurposed credential columns) is tracked in
        // #1374 / BIT-27, not in this removed callback.
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
        // ID-document upload + OCR extract (Story 18.2, #1687)
        .route("/guests/{id}/id-document", post(upload_guest_id_document))
        .route(
            "/guests/{id}/id-document/extract",
            post(extract_guest_id_document),
        )
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

/// SECURITY (#804): assert that `unit_id` belongs to `org_id`.
///
/// Returns 404 (not 403) on a foreign/missing unit so the endpoint does not
/// leak whether a given unit UUID exists in another organization.
async fn ensure_unit_in_org(
    state: &AppState,
    org_id: Uuid,
    unit_id: Uuid,
) -> Result<(), (axum::http::StatusCode, String)> {
    let owns = state
        .rental_repo
        .unit_belongs_to_org(org_id, unit_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to verify unit ownership: {}", e),
            )
        })?;

    if owns {
        Ok(())
    } else {
        Err((
            axum::http::StatusCode::NOT_FOUND,
            "Unit not found".to_string(),
        ))
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
        (status = 200, description = "Connection details", body = PlatformConnectionDetail),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Connection not found")
    )
)]
pub async fn get_connection(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<PlatformConnectionDetail>, (axum::http::StatusCode, String)> {
    // SECURITY (#887 / #804): scope the lookup to the authenticated tenant so a
    // caller cannot read another org's connection, and return a token-free DTO
    // so `access_token` / `refresh_token` are never serialized to the response.
    let connection = state
        .rental_repo
        .find_connection_for_org(tenant.tenant_id, id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get connection: {}", e),
            )
        })?;

    match connection {
        Some(c) => Ok(Json(PlatformConnectionDetail::from(c))),
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
        (status = 200, description = "Connection updated", body = PlatformConnectionDetail),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Connection not found")
    )
)]
pub async fn update_connection(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdatePlatformConnection>,
) -> Result<Json<PlatformConnectionDetail>, (axum::http::StatusCode, String)> {
    // SECURITY (#887 / #804): org-scoped update — cross-tenant ids return 404.
    let connection = state
        .rental_repo
        .update_connection_for_org(tenant.tenant_id, id, data)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update connection: {}", e),
            )
        })?;

    match connection {
        Some(c) => Ok(Json(PlatformConnectionDetail::from(c))),
        None => Err((
            axum::http::StatusCode::NOT_FOUND,
            "Connection not found".to_string(),
        )),
    }
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
    // SECURITY (#887 / #804): org-scoped delete — cross-tenant ids return 404.
    let deleted = state
        .rental_repo
        .delete_connection_for_org(tenant.tenant_id, id)
        .await
        .map_err(|e| {
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
    // SECURITY (#887 / #804): scope to the authenticated tenant so a caller
    // cannot enumerate another org's unit connections by guessing a UUID.
    let connections = state
        .rental_repo
        .get_connections_for_unit_in_org(tenant.tenant_id, unit_id)
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
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Booking not found")
    )
)]
pub async fn get_booking(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<RentalBooking>, (axum::http::StatusCode, String)> {
    // SECURITY (#804): scope to the authenticated tenant so a caller cannot
    // read another org's booking PII by guessing a UUID.
    let booking = state
        .rental_repo
        .find_booking_for_org(tenant.tenant_id, id)
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
    // SECURITY (#804): org-scoped update — cross-tenant ids return 404.
    let booking = state
        .rental_repo
        .update_booking_for_org(tenant.tenant_id, id, data)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update booking: {}", e),
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
    // SECURITY (#804): org-scoped status update — cross-tenant ids return 404.
    let booking = state
        .rental_repo
        .update_booking_status_for_org(tenant.tenant_id, id, data)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update status: {}", e),
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

/// Get booking with guests.
#[utoipa::path(
    get,
    path = "/api/v1/rentals/bookings/{id}/guests",
    tag = "Rentals",
    params(("id" = Uuid, Path, description = "Booking ID")),
    responses(
        (status = 200, description = "Booking with guests", body = BookingWithGuests),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Booking not found")
    )
)]
pub async fn get_booking_with_guests(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<BookingWithGuests>, (axum::http::StatusCode, String)> {
    // SECURITY (#804): scope to the authenticated tenant so a caller cannot
    // read another org's booking + guest PII by guessing a UUID.
    let result = state
        .rental_repo
        .get_booking_with_guests_for_org(tenant.tenant_id, id)
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
    // SECURITY (#804): verify the unit belongs to the caller's org before
    // returning its calendar — otherwise any authed user could read any unit.
    ensure_unit_in_org(&state, tenant.tenant_id, unit_id).await?;

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
    // SECURITY (#804): verify the unit belongs to the caller's org.
    ensure_unit_in_org(&state, tenant.tenant_id, unit_id).await?;

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
    // SECURITY (#804): org-scoped delete — cross-tenant ids return 404.
    let deleted = state
        .rental_repo
        .delete_calendar_block_for_org(tenant.tenant_id, id)
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
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Guest not found")
    )
)]
pub async fn get_guest(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<RentalGuest>, (axum::http::StatusCode, String)> {
    // SECURITY (#804): scope to the authenticated tenant so a caller cannot
    // read another org's guest PII by guessing a UUID.
    let guest = state
        .rental_repo
        .find_guest_for_org(tenant.tenant_id, id)
        .await
        .map_err(|e| {
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
    // SECURITY (#804): org-scoped update — cross-tenant ids return 404.
    let guest = state
        .rental_repo
        .update_guest_for_org(tenant.tenant_id, id, data)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update guest: {}", e),
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
    // SECURITY (#804): org-scoped delete — cross-tenant ids return 404.
    let deleted = state
        .rental_repo
        .delete_guest_for_org(tenant.tenant_id, id)
        .await
        .map_err(|e| {
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

// ============================================================================
// Guest ID-document upload + OCR extract (Story 18.2, #1687)
// ============================================================================

/// Manager-role gate for guest ID-document endpoints (#1687).
///
/// Mirrors `integrations::sync::verify_manager_role_in_org` (which is
/// `pub(super)` to its module, so it cannot be reused here): the manager
/// decision is derived from the canonical `TenantRole::is_manager` predicate so
/// this authz gate cannot silently drift. Returns `403 FORBIDDEN` for
/// non-managers (and `500` on a lookup failure).
async fn require_manager_in_org(
    state: &AppState,
    org_id: Uuid,
    user_id: Uuid,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let role_type = state
        .org_member_repo
        .get_user_role_type(org_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to look up org role for ID-document manager gate");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
            )
        })?;

    let is_manager = role_type
        .as_deref()
        .and_then(TenantRole::from_role_type)
        .map(|role| role.is_manager())
        .unwrap_or(false);

    if !is_manager {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Manager-level access required for this organization",
            )),
        ));
    }
    Ok(())
}

/// Map a multipart read error to a structured response, preserving the
/// underlying status so a body/length-limit overflow surfaces as `413` (not a
/// flat `400`). axum-extra sets `413 PAYLOAD_TOO_LARGE` on its
/// `MultipartError` when the request/field exceeds the configured body limit.
fn map_multipart_err(
    e: axum_extra::extract::multipart::MultipartError,
) -> (StatusCode, Json<ErrorResponse>) {
    let status = e.status();
    tracing::warn!(error = %e, %status, "Failed to read multipart field");
    if status == StatusCode::PAYLOAD_TOO_LARGE {
        (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(ErrorResponse::new(
                "FILE_TOO_LARGE",
                "Uploaded file is too large.",
            )),
        )
    } else {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                format!("Failed to read multipart field: {e}"),
            )),
        )
    }
}

/// Upload a guest ID document (multipart `file`).
///
/// Stores the bytes in S3 (best-effort — skipped when storage is unconfigured,
/// mirroring `documents/core.rs::upload_document`), records a
/// `rental_guest_id_documents` row, and sets the guest's `id_document_url` so
/// the existing registration UI can link to it. Manager-only (#1687).
#[utoipa::path(
    post,
    path = "/api/v1/rentals/guests/{id}/id-document",
    tag = "Rentals",
    params(("id" = Uuid, Path, description = "Guest ID")),
    request_body(content = String, content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "Document uploaded", body = RentalGuestIdDocument),
        (status = 400, description = "Bad request / unsupported file type", body = ErrorResponse),
        (status = 403, description = "Manager access required", body = ErrorResponse),
        (status = 404, description = "Guest not found", body = ErrorResponse),
        (status = 413, description = "File too large", body = ErrorResponse),
    )
)]
pub async fn upload_guest_id_document(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<RentalGuestIdDocument>), (StatusCode, Json<ErrorResponse>)> {
    let org_id = tenant.tenant_id;
    require_manager_in_org(&state, org_id, tenant.user_id).await?;

    // SECURITY (#804/#1687): scope the guest lookup to the caller's org so a
    // cross-tenant guest id returns 404 (not 403) and never leaks existence.
    let guest = state
        .rental_repo
        .find_guest_for_org(org_id, id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to look up guest: {e}"),
                )),
            )
        })?;
    if guest.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Guest not found")),
        ));
    }

    // --- Parse the `file` part ---
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut mime_type: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(map_multipart_err)? {
        if field.name() == Some("file") {
            file_name = field.file_name().map(|s| s.to_string());
            mime_type = field.content_type().map(|ct| ct.to_string());
            let data = field.bytes().await.map_err(map_multipart_err)?;
            file_bytes = Some(data.to_vec());
        } else {
            // Drain unknown fields so the parser can advance.
            let _ = field.bytes().await;
        }
    }

    let bytes = file_bytes.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Missing 'file' field in multipart body",
            )),
        )
    })?;

    let file_name = file_name.unwrap_or_else(|| "id-document".to_string());
    let resolved_mime =
        mime_type.unwrap_or_else(|| integrations::get_content_type(&file_name).to_string());

    // --- Validate MIME allowlist ---
    if !ALLOWED_ID_DOC_MIME.contains(&resolved_mime.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "UNSUPPORTED_FILE_TYPE",
                format!("File type '{resolved_mime}' is not supported. Allowed: PNG, JPEG, PDF"),
            )),
        ));
    }

    // --- Validate size cap ---
    if bytes.len() > MAX_ID_DOC_SIZE {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(ErrorResponse::new(
                "FILE_TOO_LARGE",
                format!("File exceeds maximum size of {MAX_ID_DOC_SIZE} bytes (10 MiB)"),
            )),
        ));
    }

    // --- Build the storage key + best-effort S3 upload ---
    let file_key = format!(
        "id-documents/{}",
        integrations::generate_storage_key(org_id, &file_name)
    );
    if let Some(ref storage_service) = state.storage_service {
        if storage_service.has_s3_client() {
            storage_service
                .upload(&file_key, bytes, &resolved_mime)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, file_key = %file_key, "Failed to upload ID document to S3");
                    (
                        StatusCode::SERVICE_UNAVAILABLE,
                        Json(ErrorResponse::new(
                            "STORAGE_ERROR",
                            "Failed to upload file to storage. Please try again.",
                        )),
                    )
                })?;
        } else {
            tracing::warn!(file_key = %file_key, "Storage service present but S3 client not initialised — skipping S3 upload");
        }
    } else {
        tracing::warn!(file_key = %file_key, "No storage service configured — skipping S3 upload");
    }

    // --- Record the document row ---
    let doc = state
        .rental_repo
        .create_guest_id_document(
            org_id,
            CreateGuestIdDocument {
                guest_id: id,
                file_key: file_key.clone(),
                mime: resolved_mime,
                uploaded_by: Some(tenant.user_id),
            },
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to record ID document: {e}"),
                )),
            )
        })?;

    // --- Point the guest record at the uploaded document ---
    let update = UpdateGuest {
        id_document_url: Some(file_key),
        ..Default::default()
    };
    state
        .rental_repo
        .update_guest_for_org(org_id, id, update)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to link document to guest: {e}"),
                )),
            )
        })?;

    Ok((StatusCode::CREATED, Json(doc)))
}

/// Run OCR over a guest's most-recently-uploaded ID document (#1687).
///
/// Returns the extracted fields for the manager to confirm — the result is
/// **never** auto-applied to the guest record (the manager edits via
/// `PUT /api/v1/rentals/guests/{id}`). Stage A wires only the not-configured
/// stub, so this degrades to `501 OCR_NOT_CONFIGURED` (mirroring
/// `routes/ai/ocr.rs`) until a real provider is wired in Stage B. Manager-only.
#[utoipa::path(
    post,
    path = "/api/v1/rentals/guests/{id}/id-document/extract",
    tag = "Rentals",
    params(("id" = Uuid, Path, description = "Guest ID")),
    responses(
        (status = 200, description = "OCR extraction result", body = GuestIdExtraction),
        (status = 403, description = "Manager access required", body = ErrorResponse),
        (status = 404, description = "No ID document on file", body = ErrorResponse),
        (status = 501, description = "OCR provider not configured", body = ErrorResponse),
    )
)]
pub async fn extract_guest_id_document(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<GuestIdExtraction>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = tenant.tenant_id;
    require_manager_in_org(&state, org_id, tenant.user_id).await?;

    // SECURITY (#1687): org-scoped lookup; a cross-tenant guest (or one with no
    // uploaded document) returns 404 and never reveals existence.
    let doc = state
        .rental_repo
        .latest_guest_id_document_for_org(org_id, id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to look up ID document: {e}"),
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "No ID document on file for this guest",
                )),
            )
        })?;

    // Fetch the bytes back from storage when available; the stub provider does
    // not read them, so an empty slice is fine when storage is unconfigured.
    let bytes: Vec<u8> = match &state.storage_service {
        Some(s) if s.has_s3_client() => s
            .download(&doc.file_key)
            .await
            .map(|(data, _content_type)| data)
            .unwrap_or_default(),
        _ => Vec::new(),
    };

    match state.id_document_ocr.extract(&bytes, &doc.mime).await {
        Ok(extraction) => {
            // Persist the extraction for audit / prefill, but do NOT mutate the
            // guest record — the manager confirms the values.
            if let Ok(json) = serde_json::to_value(&extraction) {
                let _ = state
                    .rental_repo
                    .set_guest_id_document_extraction(org_id, doc.id, json)
                    .await;
            }
            Ok(Json(extraction))
        }
        Err(IdOcrError::NotConfigured) => Err((
            StatusCode::NOT_IMPLEMENTED,
            Json(ErrorResponse::new(
                "OCR_NOT_CONFIGURED",
                "ID-document OCR extraction is not yet configured. \
                 Please enter the guest details manually until this feature is enabled.",
            )),
        )),
        Err(IdOcrError::InvalidDocument(msg)) => Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse::new("INVALID_DOCUMENT", msg)),
        )),
        Err(IdOcrError::Provider(msg)) => {
            tracing::error!(error = %msg, "ID-document OCR provider error");
            Err((
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse::new(
                    "OCR_PROVIDER_ERROR",
                    "OCR provider failed. Please try again later.",
                )),
            ))
        }
    }
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
    // SECURITY (#804): org-scoped register — cross-tenant ids return 404.
    let guest = state
        .rental_repo
        .register_guest_for_org(tenant.tenant_id, id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to register guest: {}", e),
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
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Report not found")
    )
)]
pub async fn get_report(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<RentalGuestReport>, (axum::http::StatusCode, String)> {
    // SECURITY (#804): scope to the authenticated tenant so a caller cannot
    // read another org's authority report by guessing a UUID.
    let report = state
        .rental_repo
        .find_report_for_org(tenant.tenant_id, id)
        .await
        .map_err(|e| {
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
    // SECURITY (#804): org-scoped submit — cross-tenant ids return 404.
    let report = state
        .rental_repo
        .submit_report_for_org(tenant.tenant_id, id, tenant.user_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to submit report: {}", e),
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
    // SECURITY (#804): org-scoped update — cross-tenant ids return 404.
    let feed = state
        .rental_repo
        .update_ical_feed_for_org(tenant.tenant_id, id, data)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update feed: {}", e),
            )
        })?;

    match feed {
        Some(f) => Ok(Json(f)),
        None => Err((
            axum::http::StatusCode::NOT_FOUND,
            "Feed not found".to_string(),
        )),
    }
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
    // SECURITY (#804): org-scoped delete — cross-tenant ids return 404.
    let deleted = state
        .rental_repo
        .delete_ical_feed_for_org(tenant.tenant_id, id)
        .await
        .map_err(|e| {
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
    // SECURITY (#804): scope to the authenticated tenant so a caller cannot
    // enumerate another org's feeds by guessing a unit UUID.
    let feeds = state
        .rental_repo
        .get_ical_feeds_for_unit_in_org(tenant.tenant_id, unit_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get feeds: {}", e),
            )
        })?;

    Ok(Json(feeds))
}
