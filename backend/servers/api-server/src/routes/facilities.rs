//! Facility routes (Epic 3, Story 3.7).
//!
//! Implements common area and facility management including booking system.

use api_core::extractors::{AuthUser, RlsConnection};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use common::errors::ErrorResponse;
use db::models::facility::{
    CreateFacility, CreateFacilityBooking, Facility, FacilityBooking, FacilitySummary,
    UpdateFacility, UpdateFacilityBooking,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::state::AppState;

/// Create facilities router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Facility management (by building)
        .route("/buildings/{building_id}/facilities", get(list_facilities))
        .route("/buildings/{building_id}/facilities", post(create_facility))
        .route("/buildings/{building_id}/facilities/{id}", get(get_facility))
        .route(
            "/buildings/:building_id/facilities/:id",
            put(update_facility),
        )
        .route(
            "/buildings/:building_id/facilities/:id",
            delete(delete_facility),
        )
        // Bookings
        .route(
            "/buildings/:building_id/facilities/:facility_id/bookings",
            get(list_facility_bookings),
        )
        .route(
            "/buildings/:building_id/facilities/:facility_id/bookings",
            post(create_booking),
        )
        .route(
            "/buildings/:building_id/facilities/:facility_id/availability",
            get(check_availability),
        )
        // Booking management
        .route("/bookings/my", get(list_my_bookings))
        .route("/bookings/{id}", get(get_booking))
        .route("/bookings/{id}", put(update_booking))
        .route("/bookings/{id}/cancel", post(cancel_booking))
        // Approval workflow
        .route(
            "/buildings/:building_id/bookings/pending",
            get(list_pending_bookings),
        )
        .route("/bookings/{id}/approve", post(approve_booking))
        .route("/bookings/{id}/reject", post(reject_booking))
}

// ==================== Request/Response Types ====================

/// Create facility request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateFacilityRequest {
    /// Facility name
    pub name: String,
    /// Facility type (gym, pool, meeting_room, bbq_area, playground, laundry, parking, other)
    pub facility_type: String,
    /// Description
    pub description: Option<String>,
    /// Location within building
    pub location: Option<String>,
    /// Maximum capacity
    pub capacity: Option<i32>,
    /// Whether the facility can be booked
    #[serde(default = "default_true")]
    pub is_bookable: bool,
    /// Whether bookings require approval
    #[serde(default)]
    pub requires_approval: bool,
    /// Maximum booking duration in hours
    pub max_booking_hours: Option<i32>,
    /// Maximum days in advance to book
    pub max_advance_days: Option<i32>,
    /// Minimum hours in advance to book
    pub min_advance_hours: Option<i32>,
    /// Available from time (HH:MM)
    pub available_from: Option<String>,
    /// Available to time (HH:MM)
    pub available_to: Option<String>,
    /// Available days (0=Sunday, 6=Saturday)
    pub available_days: Option<Vec<i32>>,
    /// Usage rules
    pub rules: Option<String>,
    /// Hourly fee
    pub hourly_fee: Option<Decimal>,
    /// Deposit amount
    pub deposit_amount: Option<Decimal>,
}

fn default_true() -> bool {
    true
}

/// Facility response.
#[derive(Debug, Serialize, ToSchema)]
pub struct FacilityResponse {
    pub id: Uuid,
    pub building_id: Uuid,
    pub name: String,
    pub facility_type: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub capacity: Option<i32>,
    pub is_bookable: bool,
    pub requires_approval: bool,
    pub max_booking_hours: Option<i32>,
    pub max_advance_days: Option<i32>,
    pub min_advance_hours: Option<i32>,
    pub available_from: Option<String>,
    pub available_to: Option<String>,
    pub available_days: Option<Vec<i32>>,
    pub rules: Option<String>,
    pub hourly_fee: Option<Decimal>,
    pub deposit_amount: Option<Decimal>,
    pub photos: serde_json::Value,
    pub amenities: serde_json::Value,
    pub is_active: bool,
    pub created_at: String,
}

impl From<Facility> for FacilityResponse {
    fn from(f: Facility) -> Self {
        Self {
            id: f.id,
            building_id: f.building_id,
            name: f.name,
            facility_type: f.facility_type,
            description: f.description,
            location: f.location,
            capacity: f.capacity,
            is_bookable: f.is_bookable,
            requires_approval: f.requires_approval,
            max_booking_hours: f.max_booking_hours,
            max_advance_days: f.max_advance_days,
            min_advance_hours: f.min_advance_hours,
            available_from: f.available_from.map(|t| t.to_string()),
            available_to: f.available_to.map(|t| t.to_string()),
            available_days: f.available_days,
            rules: f.rules,
            hourly_fee: f.hourly_fee,
            deposit_amount: f.deposit_amount,
            photos: f.photos,
            amenities: f.amenities,
            is_active: f.is_active,
            created_at: f.created_at.to_rfc3339(),
        }
    }
}

/// Update facility request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateFacilityRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub capacity: Option<i32>,
    pub is_bookable: Option<bool>,
    pub requires_approval: Option<bool>,
    pub max_booking_hours: Option<i32>,
    pub max_advance_days: Option<i32>,
    pub min_advance_hours: Option<i32>,
    pub available_from: Option<String>,
    pub available_to: Option<String>,
    pub available_days: Option<Vec<i32>>,
    pub rules: Option<String>,
    pub hourly_fee: Option<Decimal>,
    pub deposit_amount: Option<Decimal>,
    pub photos: Option<serde_json::Value>,
    pub amenities: Option<serde_json::Value>,
    pub is_active: Option<bool>,
}

/// Create booking request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateBookingRequest {
    /// Unit ID (for resident bookings)
    pub unit_id: Option<Uuid>,
    /// Start time
    pub start_time: DateTime<Utc>,
    /// End time
    pub end_time: DateTime<Utc>,
    /// Purpose of booking
    pub purpose: Option<String>,
    /// Expected number of attendees
    pub attendees: Option<i32>,
    /// Additional notes
    pub notes: Option<String>,
}

/// Booking response.
#[derive(Debug, Serialize, ToSchema)]
pub struct BookingResponse {
    pub id: Uuid,
    pub facility_id: Uuid,
    pub user_id: Uuid,
    pub unit_id: Option<Uuid>,
    pub start_time: String,
    pub end_time: String,
    pub status: String,
    pub purpose: Option<String>,
    pub attendees: Option<i32>,
    pub notes: Option<String>,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<String>,
    pub rejection_reason: Option<String>,
    pub cancelled_at: Option<String>,
    pub cancellation_reason: Option<String>,
    pub created_at: String,
}

impl From<FacilityBooking> for BookingResponse {
    fn from(b: FacilityBooking) -> Self {
        Self {
            id: b.id,
            facility_id: b.facility_id,
            user_id: b.user_id,
            unit_id: b.unit_id,
            start_time: b.start_time.to_rfc3339(),
            end_time: b.end_time.to_rfc3339(),
            status: b.status,
            purpose: b.purpose,
            attendees: b.attendees,
            notes: b.notes,
            approved_by: b.approved_by,
            approved_at: b.approved_at.map(|dt| dt.to_rfc3339()),
            rejection_reason: b.rejection_reason,
            cancelled_at: b.cancelled_at.map(|dt| dt.to_rfc3339()),
            cancellation_reason: b.cancellation_reason,
            created_at: b.created_at.to_rfc3339(),
        }
    }
}

/// Update booking request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateBookingRequest {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub purpose: Option<String>,
    pub attendees: Option<i32>,
    pub notes: Option<String>,
}

/// Cancel booking request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CancelBookingRequest {
    pub reason: Option<String>,
}

/// Reject booking request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RejectBookingRequest {
    pub reason: String,
}

/// Availability query parameters.
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct AvailabilityQuery {
    /// Start of time range
    pub from: DateTime<Utc>,
    /// End of time range
    pub to: DateTime<Utc>,
}

/// Bookings query parameters.
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct BookingsQuery {
    /// Start of time range
    pub from: Option<DateTime<Utc>>,
    /// End of time range
    pub to: Option<DateTime<Utc>>,
}

/// Availability slot.
#[derive(Debug, Serialize, ToSchema)]
pub struct AvailabilitySlot {
    pub start_time: String,
    pub end_time: String,
    pub is_available: bool,
    pub booking_id: Option<Uuid>,
}

// ==================== Facility Handlers ====================

/// List facilities for a building.
#[utoipa::path(
    get,
    path = "/api/v1/buildings/{building_id}/facilities",
    tag = "Facilities",
    params(("building_id" = Uuid, Path, description = "Building ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of facilities", body = Vec<FacilitySummary>),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Building not found", body = ErrorResponse)
    )
)]
pub async fn list_facilities(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(building_id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Verify building exists and user has access
    let building = state
        .building_repo
        .find_by_id_rls(&mut **rls.conn(), building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Building not found")),
            )
        })?;

    let is_member = state
        .org_member_repo
        .is_member(building.organization_id, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check org membership");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !is_member {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    let facilities = state
        .facility_repo
        .find_by_building(building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list facilities");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list facilities")),
            )
        })?;

    Ok(Json(facilities))
}

/// Create a new facility (Story 3.7.1).
#[utoipa::path(
    post,
    path = "/api/v1/buildings/{building_id}/facilities",
    tag = "Facilities",
    params(("building_id" = Uuid, Path, description = "Building ID")),
    request_body = CreateFacilityRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Facility created", body = FacilityResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse)
    )
)]
pub async fn create_facility(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(building_id): Path<Uuid>,
    Json(req): Json<CreateFacilityRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Verify building exists and user has access
    let building = state
        .building_repo
        .find_by_id_rls(&mut **rls.conn(), building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Building not found")),
            )
        })?;

    let is_member = state
        .org_member_repo
        .is_member(building.organization_id, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check org membership");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !is_member {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    // Validate name
    if req.name.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_NAME",
                "Facility name is required",
            )),
        ));
    }

    // Parse time strings
    let available_from = req
        .available_from
        .as_ref()
        .and_then(|s| chrono::NaiveTime::parse_from_str(s, "%H:%M").ok());
    let available_to = req
        .available_to
        .as_ref()
        .and_then(|s| chrono::NaiveTime::parse_from_str(s, "%H:%M").ok());

    let create_data = CreateFacility {
        building_id,
        name: req.name,
        facility_type: req.facility_type,
        description: req.description,
        location: req.location,
        capacity: req.capacity,
        is_bookable: req.is_bookable,
        requires_approval: req.requires_approval,
        max_booking_hours: req.max_booking_hours,
        max_advance_days: req.max_advance_days,
        min_advance_hours: req.min_advance_hours,
        available_from,
        available_to,
        available_days: req.available_days,
        rules: req.rules,
        hourly_fee: req.hourly_fee,
        deposit_amount: req.deposit_amount,
    };

    let facility = state.facility_repo.create(create_data).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to create facility");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DB_ERROR", "Failed to create facility")),
        )
    })?;

    tracing::info!(
        facility_id = %facility.id,
        building_id = %building_id,
        user_id = %auth.user_id,
        "Facility created"
    );

    Ok((StatusCode::CREATED, Json(FacilityResponse::from(facility))))
}

/// Get facility by ID.
#[utoipa::path(
    get,
    path = "/api/v1/buildings/{building_id}/facilities/{id}",
    tag = "Facilities",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("id" = Uuid, Path, description = "Facility ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Facility found", body = FacilityResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Facility not found", body = ErrorResponse)
    )
)]
pub async fn get_facility(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path((building_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Verify building exists and user has access
    let building = state
        .building_repo
        .find_by_id_rls(&mut **rls.conn(), building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Building not found")),
            )
        })?;

    let is_member = state
        .org_member_repo
        .is_member(building.organization_id, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check org membership");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !is_member {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    let facility = state
        .facility_repo
        .find_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get facility");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Facility not found")),
            )
        })?;

    if facility.building_id != building_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Facility not found in this building",
            )),
        ));
    }

    Ok(Json(FacilityResponse::from(facility)))
}

/// Update facility (Story 3.7.2).
#[utoipa::path(
    put,
    path = "/api/v1/buildings/{building_id}/facilities/{id}",
    tag = "Facilities",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("id" = Uuid, Path, description = "Facility ID")
    ),
    request_body = UpdateFacilityRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Facility updated", body = FacilityResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Facility not found", body = ErrorResponse)
    )
)]
pub async fn update_facility(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path((building_id, id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateFacilityRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Verify building exists and user has access
    let building = state
        .building_repo
        .find_by_id_rls(&mut **rls.conn(), building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Building not found")),
            )
        })?;

    let is_member = state
        .org_member_repo
        .is_member(building.organization_id, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check org membership");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !is_member {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    // Parse time strings
    let available_from = req
        .available_from
        .as_ref()
        .and_then(|s| chrono::NaiveTime::parse_from_str(s, "%H:%M").ok());
    let available_to = req
        .available_to
        .as_ref()
        .and_then(|s| chrono::NaiveTime::parse_from_str(s, "%H:%M").ok());

    let update_data = UpdateFacility {
        name: req.name,
        description: req.description,
        location: req.location,
        capacity: req.capacity,
        is_bookable: req.is_bookable,
        requires_approval: req.requires_approval,
        max_booking_hours: req.max_booking_hours,
        max_advance_days: req.max_advance_days,
        min_advance_hours: req.min_advance_hours,
        available_from,
        available_to,
        available_days: req.available_days,
        rules: req.rules,
        hourly_fee: req.hourly_fee,
        deposit_amount: req.deposit_amount,
        photos: req.photos,
        amenities: req.amenities,
        is_active: req.is_active,
    };

    let facility = state
        .facility_repo
        .update(id, update_data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update facility");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to update facility")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Facility not found")),
            )
        })?;

    tracing::info!(facility_id = %id, user_id = %auth.user_id, "Facility updated");

    Ok(Json(FacilityResponse::from(facility)))
}

/// Delete facility.
#[utoipa::path(
    delete,
    path = "/api/v1/buildings/{building_id}/facilities/{id}",
    tag = "Facilities",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("id" = Uuid, Path, description = "Facility ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Facility deleted"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Facility not found", body = ErrorResponse)
    )
)]
pub async fn delete_facility(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path((building_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Verify building exists and user has access
    let building = state
        .building_repo
        .find_by_id_rls(&mut **rls.conn(), building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Building not found")),
            )
        })?;

    let is_member = state
        .org_member_repo
        .is_member(building.organization_id, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check org membership");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !is_member {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    let deleted = state.facility_repo.delete(id).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to delete facility");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DB_ERROR", "Failed to delete facility")),
        )
    })?;

    if deleted {
        tracing::info!(facility_id = %id, user_id = %auth.user_id, "Facility deleted");
        Ok(StatusCode::OK)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Facility not found")),
        ))
    }
}

// ==================== Booking Handlers ====================

/// List bookings for a facility.
#[utoipa::path(
    get,
    path = "/api/v1/buildings/{building_id}/facilities/{facility_id}/bookings",
    tag = "Facility Bookings",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("facility_id" = Uuid, Path, description = "Facility ID"),
        BookingsQuery
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of bookings", body = Vec<BookingResponse>),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse)
    )
)]
pub async fn list_facility_bookings(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path((building_id, facility_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<BookingsQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Verify building and access
    let building = state
        .building_repo
        .find_by_id_rls(&mut **rls.conn(), building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Building not found")),
            )
        })?;

    let is_member = state
        .org_member_repo
        .is_member(building.organization_id, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check org membership");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !is_member {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    let from = query.from.unwrap_or_else(Utc::now);
    let to = query
        .to
        .unwrap_or_else(|| from + chrono::Duration::days(30));

    let bookings = state
        .facility_repo
        .find_bookings_by_facility(facility_id, from, to)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list bookings");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list bookings")),
            )
        })?;

    let response: Vec<BookingResponse> = bookings.into_iter().map(BookingResponse::from).collect();

    Ok(Json(response))
}

/// Create a booking (Story 3.7.3).
#[utoipa::path(
    post,
    path = "/api/v1/buildings/{building_id}/facilities/{facility_id}/bookings",
    tag = "Facility Bookings",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("facility_id" = Uuid, Path, description = "Facility ID")
    ),
    request_body = CreateBookingRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Booking created", body = BookingResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 409, description = "Time slot not available", body = ErrorResponse)
    )
)]
pub async fn create_booking(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path((building_id, facility_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<CreateBookingRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Verify building and access
    let building = state
        .building_repo
        .find_by_id_rls(&mut **rls.conn(), building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Building not found")),
            )
        })?;

    let is_member = state
        .org_member_repo
        .is_member(building.organization_id, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check org membership");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !is_member {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    // Validate time range
    if req.end_time <= req.start_time {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_TIME",
                "End time must be after start time",
            )),
        ));
    }

    if req.start_time < Utc::now() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_TIME",
                "Cannot book in the past",
            )),
        ));
    }

    // Check availability
    let available = state
        .facility_repo
        .check_availability(facility_id, req.start_time, req.end_time, None)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check availability");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !available {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse::new(
                "NOT_AVAILABLE",
                "The requested time slot is not available",
            )),
        ));
    }

    let create_data = CreateFacilityBooking {
        facility_id,
        unit_id: req.unit_id,
        start_time: req.start_time,
        end_time: req.end_time,
        purpose: req.purpose,
        attendees: req.attendees,
        notes: req.notes,
    };

    let booking = state
        .facility_repo
        .create_booking(auth.user_id, create_data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create booking");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to create booking")),
            )
        })?;

    tracing::info!(
        booking_id = %booking.id,
        facility_id = %facility_id,
        user_id = %auth.user_id,
        "Booking created"
    );

    Ok((StatusCode::CREATED, Json(BookingResponse::from(booking))))
}

/// Check facility availability.
#[utoipa::path(
    get,
    path = "/api/v1/buildings/{building_id}/facilities/{facility_id}/availability",
    tag = "Facility Bookings",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("facility_id" = Uuid, Path, description = "Facility ID"),
        AvailabilityQuery
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Availability info", body = Vec<AvailabilitySlot>),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse)
    )
)]
pub async fn check_availability(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path((building_id, facility_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<AvailabilityQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Verify building and access
    let building = state
        .building_repo
        .find_by_id_rls(&mut **rls.conn(), building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Building not found")),
            )
        })?;

    let is_member = state
        .org_member_repo
        .is_member(building.organization_id, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check org membership");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !is_member {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    // Get existing bookings in range
    let bookings = state
        .facility_repo
        .find_bookings_by_facility(facility_id, query.from, query.to)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get bookings");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    // Convert to availability slots
    let slots: Vec<AvailabilitySlot> = bookings
        .into_iter()
        .map(|b| AvailabilitySlot {
            start_time: b.start_time.to_rfc3339(),
            end_time: b.end_time.to_rfc3339(),
            is_available: false,
            booking_id: Some(b.id),
        })
        .collect();

    Ok(Json(slots))
}

/// List my bookings.
#[utoipa::path(
    get,
    path = "/api/v1/bookings/my",
    tag = "Facility Bookings",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of my bookings", body = Vec<BookingResponse>),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn list_my_bookings(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let bookings = state
        .facility_repo
        .find_bookings_by_user(auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list bookings");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list bookings")),
            )
        })?;

    let response: Vec<BookingResponse> = bookings.into_iter().map(BookingResponse::from).collect();

    Ok(Json(response))
}

/// Get booking by ID.
#[utoipa::path(
    get,
    path = "/api/v1/bookings/{id}",
    tag = "Facility Bookings",
    params(("id" = Uuid, Path, description = "Booking ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Booking found", body = BookingResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 404, description = "Booking not found", body = ErrorResponse)
    )
)]
pub async fn get_booking(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let booking = state
        .facility_repo
        .find_booking_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get booking");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Booking not found")),
            )
        })?;

    Ok(Json(BookingResponse::from(booking)))
}

/// Update booking.
#[utoipa::path(
    put,
    path = "/api/v1/bookings/{id}",
    tag = "Facility Bookings",
    params(("id" = Uuid, Path, description = "Booking ID")),
    request_body = UpdateBookingRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Booking updated", body = BookingResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Booking not found", body = ErrorResponse)
    )
)]
pub async fn update_booking(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateBookingRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Get existing booking
    let existing = state
        .facility_repo
        .find_booking_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get booking");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Booking not found")),
            )
        })?;

    // Check ownership
    if existing.user_id != auth.user_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You can only update your own bookings",
            )),
        ));
    }

    // Check if time is being changed and validate availability
    if req.start_time.is_some() || req.end_time.is_some() {
        let new_start = req.start_time.unwrap_or(existing.start_time);
        let new_end = req.end_time.unwrap_or(existing.end_time);

        if new_end <= new_start {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_TIME",
                    "End time must be after start time",
                )),
            ));
        }

        let available = state
            .facility_repo
            .check_availability(existing.facility_id, new_start, new_end, Some(id))
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to check availability");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("DB_ERROR", "Database error")),
                )
            })?;

        if !available {
            return Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse::new(
                    "NOT_AVAILABLE",
                    "The requested time slot is not available",
                )),
            ));
        }
    }

    let update_data = UpdateFacilityBooking {
        start_time: req.start_time,
        end_time: req.end_time,
        purpose: req.purpose,
        attendees: req.attendees,
        notes: req.notes,
    };

    let booking = state
        .facility_repo
        .update_booking(id, update_data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update booking");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to update booking")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Booking not found")),
            )
        })?;

    tracing::info!(booking_id = %id, user_id = %auth.user_id, "Booking updated");

    Ok(Json(BookingResponse::from(booking)))
}

/// Cancel a booking (Story 3.7.5).
#[utoipa::path(
    post,
    path = "/api/v1/bookings/{id}/cancel",
    tag = "Facility Bookings",
    params(("id" = Uuid, Path, description = "Booking ID")),
    request_body = CancelBookingRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Booking cancelled", body = BookingResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Booking not found", body = ErrorResponse)
    )
)]
pub async fn cancel_booking(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CancelBookingRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Get existing booking
    let existing = state
        .facility_repo
        .find_booking_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get booking");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Booking not found")),
            )
        })?;

    // Check ownership
    if existing.user_id != auth.user_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You can only cancel your own bookings",
            )),
        ));
    }

    let booking = state
        .facility_repo
        .cancel_booking(id, req.reason.as_deref())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to cancel booking");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to cancel booking")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Booking not found or already cancelled",
                )),
            )
        })?;

    tracing::info!(booking_id = %id, user_id = %auth.user_id, "Booking cancelled");

    Ok(Json(BookingResponse::from(booking)))
}

/// List pending bookings for approval.
#[utoipa::path(
    get,
    path = "/api/v1/buildings/{building_id}/bookings/pending",
    tag = "Facility Bookings",
    params(("building_id" = Uuid, Path, description = "Building ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of pending bookings", body = Vec<BookingResponse>),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse)
    )
)]
pub async fn list_pending_bookings(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(building_id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Verify building and access
    let building = state
        .building_repo
        .find_by_id_rls(&mut **rls.conn(), building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Building not found")),
            )
        })?;

    let is_member = state
        .org_member_repo
        .is_member(building.organization_id, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check org membership");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !is_member {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    let bookings = state
        .facility_repo
        .find_pending_bookings(building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list pending bookings");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list bookings")),
            )
        })?;

    let response: Vec<BookingResponse> = bookings.into_iter().map(BookingResponse::from).collect();

    Ok(Json(response))
}

/// Approve a booking (Story 3.7.4).
#[utoipa::path(
    post,
    path = "/api/v1/bookings/{id}/approve",
    tag = "Facility Bookings",
    params(("id" = Uuid, Path, description = "Booking ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Booking approved", body = BookingResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Booking not found", body = ErrorResponse)
    )
)]
pub async fn approve_booking(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let booking = state
        .facility_repo
        .approve_booking(id, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to approve booking");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to approve booking")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Booking not found or not pending",
                )),
            )
        })?;

    tracing::info!(
        booking_id = %id,
        approved_by = %auth.user_id,
        "Booking approved"
    );

    Ok(Json(BookingResponse::from(booking)))
}

/// Reject a booking.
#[utoipa::path(
    post,
    path = "/api/v1/bookings/{id}/reject",
    tag = "Facility Bookings",
    params(("id" = Uuid, Path, description = "Booking ID")),
    request_body = RejectBookingRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Booking rejected", body = BookingResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Booking not found", body = ErrorResponse)
    )
)]
pub async fn reject_booking(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<RejectBookingRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let booking = state
        .facility_repo
        .reject_booking(id, auth.user_id, &req.reason)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to reject booking");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to reject booking")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Booking not found or not pending",
                )),
            )
        })?;

    tracing::info!(
        booking_id = %id,
        rejected_by = %auth.user_id,
        "Booking rejected"
    );

    Ok(Json(BookingResponse::from(booking)))
}
