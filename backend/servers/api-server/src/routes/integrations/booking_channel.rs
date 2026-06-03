//! Booking.com channel sync routes (Gap 83-2).
//!
//! Adds the missing surface for the Booking.com channel manager integration:
//!
//! | Method | Path | Purpose |
//! |--------|------|---------|
//! | POST | `/organizations/{org_id}/booking/listing-push` | Push unit availability + rates to Booking.com via OTA XML |
//! | GET  | `/organizations/{org_id}/booking/conflicts`    | Cross-platform conflict detection for Booking.com reservations |
//!
//! The OAuth connect/disconnect and basic reservation pull already exist in
//! `install.rs`.  This module completes the channel-sync story by adding the
//! outbound push side and the conflict-detection surface.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::NaiveDate;
use db::models::BookingListQuery;
use integrations::{AvailabilityUpdate, BookingClient, RateUpdate};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::sync::OrgIdPath;
use crate::state::AppState;
use common::errors::ErrorResponse;
use common::TenantRole;

// ============================================================
// Types
// ============================================================

/// Room availability entry for a listing push.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RoomAvailabilityEntry {
    /// Booking.com room type ID.
    pub room_type_id: String,
    /// Date of the availability slot.
    pub date: NaiveDate,
    /// Number of rooms available on that date (0 = sold out).
    pub available_count: i32,
    /// If true, stop-sell is applied regardless of `available_count`.
    #[serde(default)]
    pub stop_sell: bool,
}

/// Rate entry for a listing push.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RoomRateEntry {
    /// Booking.com room type ID.
    pub room_type_id: String,
    /// Rate plan code (e.g. `"STD"`, `"NRF"`).
    pub rate_plan_code: String,
    /// Date the rate applies to.
    pub date: NaiveDate,
    /// Base rate amount.
    pub base_rate: rust_decimal::Decimal,
    /// Currency code (ISO 4217, e.g. `"EUR"`).
    #[serde(default = "default_currency")]
    pub currency: String,
}

fn default_currency() -> String {
    "EUR".to_string()
}

/// Request body for listing push to Booking.com.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ListingPushRequest {
    /// Internal unit ID to push on behalf of.
    pub unit_id: Uuid,
    /// Booking.com room type ID that maps to this unit.
    pub room_type_id: String,
    /// Availability slots to push (`OTA_HotelAvailNotifRQ`).
    #[serde(default)]
    pub availability: Vec<RoomAvailabilityEntry>,
    /// Rate updates to push (`OTA_HotelRateAmountNotifRQ`).
    #[serde(default)]
    pub rates: Vec<RoomRateEntry>,
}

/// Response returned after a successful listing push.
#[derive(Debug, Serialize, ToSchema)]
pub struct ListingPushResponse {
    pub success: bool,
    pub availability_pushed: i32,
    pub rates_pushed: i32,
    pub pushed_at: chrono::DateTime<chrono::Utc>,
    pub error: Option<String>,
}

/// A single detected booking conflict.
#[derive(Debug, Serialize, ToSchema)]
pub struct BookingConflict {
    /// Internal unit ID where the conflict occurs.
    pub unit_id: Uuid,
    /// Booking.com external reservation ID (falls back to internal UUID if not set).
    pub booking_reservation_id: String,
    /// Platform of the conflicting reservation.
    pub conflicting_platform: String,
    /// Internal booking ID that conflicts.
    pub conflicting_booking_id: Uuid,
    /// First date of the overlap.
    pub overlap_start: NaiveDate,
    /// Last date of the overlap.
    pub overlap_end: NaiveDate,
}

/// Response for the conflict-detection endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct ConflictCheckResponse {
    pub conflicts_found: i32,
    pub conflicts: Vec<BookingConflict>,
    pub checked_at: chrono::DateTime<chrono::Utc>,
    /// True if the query hit the 500-reservation cap — results may be incomplete.
    pub truncated: bool,
}

// ============================================================
// Router
// ============================================================

/// Assemble the Booking.com channel-sync sub-router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/organizations/{org_id}/booking/listing-push",
            post(push_booking_listing),
        )
        .route(
            "/organizations/{org_id}/booking/conflicts",
            get(get_booking_conflicts),
        )
}

// ============================================================
// Handlers
// ============================================================

/// Push listing availability and rates to Booking.com.
///
/// Sends `OTA_HotelAvailNotifRQ` for the availability slots and
/// `OTA_HotelRateAmountNotifRQ` for the rate entries supplied in the
/// request body.  Both push calls are made against the authenticated
/// Booking.com connection for the given organization.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/organizations/{org_id}/booking/listing-push",
    params(OrgIdPath),
    request_body = ListingPushRequest,
    responses(
        (status = 200, description = "Listing pushed to Booking.com", body = ListingPushResponse),
        (status = 400, description = "Invalid request or upstream push error"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "No Booking.com connection found for organization"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Booking.com"
)]
pub async fn push_booking_listing(
    State(state): State<AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
    Json(request): Json<ListingPushRequest>,
) -> Result<Json<ListingPushResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        unit_id = %request.unit_id,
        room_type_id = %request.room_type_id,
        "Pushing listing to Booking.com"
    );

    // BLOCKING-1: Ensure caller belongs to this org (defense-in-depth alongside RLS).
    if auth.tenant_id != Some(path.org_id) && !auth.is_platform_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "You do not have access to this organization",
            )),
        ));
    }

    // BLOCKING-2: Require an administrative role to manage integrations.
    let role = auth.role.unwrap_or(TenantRole::Guest);
    let allowed = matches!(
        role,
        TenantRole::SuperAdmin
            | TenantRole::PlatformAdmin
            | TenantRole::OrgAdmin
            | TenantRole::Manager
    );
    if !allowed {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Insufficient permissions to manage integrations",
            )),
        ));
    }

    let rental_repo = &state.rental_repo;

    // Resolve the org's Booking.com connection.
    let connection = rental_repo
        .find_booking_connection_by_org(path.org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "DB error looking up Booking.com connection");
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
                    "No Booking.com connection found for this organization",
                )),
            )
        })?;

    let hotel_id = connection.external_property_id.clone().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "NOT_CONFIGURED",
                "Hotel ID is not configured on this connection",
            )),
        )
    })?;

    // HIGH: Validate credentials explicitly — empty strings silently break OTA API calls.
    let username = connection.access_token.clone().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "NOT_CONFIGURED",
                "Booking.com credentials are incomplete (missing access_token)",
            )),
        )
    })?;
    let password = connection.refresh_token.clone().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "NOT_CONFIGURED",
                "Booking.com credentials are incomplete (missing refresh_token)",
            )),
        )
    })?;
    if username.is_empty() || password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "NOT_CONFIGURED",
                "Booking.com credentials are incomplete (empty)",
            )),
        ));
    }

    let credentials = integrations::BookingCredentials::new(hotel_id.clone(), username, password);
    let client = BookingClient::new(credentials);

    let mut avail_pushed = 0i32;
    let mut rates_pushed = 0i32;

    // ---- Push availability (OTA_HotelAvailNotifRQ) ----
    if !request.availability.is_empty() {
        let avail_updates: Vec<AvailabilityUpdate> = request
            .availability
            .iter()
            .map(|a| AvailabilityUpdate {
                room_type_id: a.room_type_id.clone(),
                date: a.date,
                available_count: a.available_count,
                stop_sell: a.stop_sell,
                cta: false,
                ctd: false,
                min_los: None,
                max_los: None,
            })
            .collect();

        let count = avail_updates.len() as i32;
        client
            .push_availability(&hotel_id, &avail_updates)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to push availability to Booking.com");
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new(
                        "PUSH_ERROR",
                        format!("Availability push failed: {}", e),
                    )),
                )
            })?;
        avail_pushed = count;
    }

    // ---- Push rates (OTA_HotelRateAmountNotifRQ) ----
    if !request.rates.is_empty() {
        let rate_updates: Vec<RateUpdate> = request
            .rates
            .iter()
            .map(|r| RateUpdate {
                room_type_id: r.room_type_id.clone(),
                rate_plan_code: r.rate_plan_code.clone(),
                date: r.date,
                base_rate: r.base_rate,
                currency: r.currency.clone(),
                extra_person_rate: None,
                extra_child_rate: None,
            })
            .collect();

        rates_pushed = rate_updates.len() as i32;
        client
            .push_rates(&hotel_id, &rate_updates)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to push rates to Booking.com");
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new(
                        "PUSH_ERROR",
                        format!("Rate push failed: {}", e),
                    )),
                )
            })?;
    }

    tracing::info!(
        org_id = %path.org_id,
        hotel_id = %hotel_id,
        avail_pushed = avail_pushed,
        rates_pushed = rates_pushed,
        "Booking.com listing push completed"
    );

    Ok(Json(ListingPushResponse {
        success: true,
        availability_pushed: avail_pushed,
        rates_pushed,
        pushed_at: chrono::Utc::now(),
        error: None,
    }))
}

/// Detect conflicts between Booking.com reservations and other-platform bookings.
///
/// Fetches all active `platform = 'booking'` bookings for the org, then checks
/// each one for date-range overlap with bookings from other platforms on the same
/// unit.  Cancelled reservations are skipped on both sides.
///
/// This is a read-only endpoint — it reports conflicts but does not resolve them.
/// Operators should cancel or modify the conflicting reservation on the appropriate
/// platform.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/organizations/{org_id}/booking/conflicts",
    params(OrgIdPath),
    responses(
        (status = 200, description = "Conflict check results", body = ConflictCheckResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "No Booking.com connection found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Booking.com"
)]
pub async fn get_booking_conflicts(
    State(state): State<AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
) -> Result<Json<ConflictCheckResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Checking Booking.com reservation conflicts"
    );

    // BLOCKING-1: Ensure caller belongs to this org (defense-in-depth alongside RLS).
    if auth.tenant_id != Some(path.org_id) && !auth.is_platform_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "You do not have access to this organization",
            )),
        ));
    }

    // BLOCKING-2: Require an administrative role to manage integrations.
    let role = auth.role.unwrap_or(TenantRole::Guest);
    let allowed = matches!(
        role,
        TenantRole::SuperAdmin
            | TenantRole::PlatformAdmin
            | TenantRole::OrgAdmin
            | TenantRole::Manager
    );
    if !allowed {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Insufficient permissions to manage integrations",
            )),
        ));
    }

    let rental_repo = &state.rental_repo;

    // Guard: a Booking.com connection must exist for this org.
    rental_repo
        .find_booking_connection_by_org(path.org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "DB error looking up Booking.com connection");
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
                    "No Booking.com connection found for this organization",
                )),
            )
        })?;

    // Fetch all Booking.com bookings for this org (up to 500 — enough for a single
    // property; background jobs handle bulk sync for large portfolios).
    let booking_query = BookingListQuery {
        unit_id: None,
        building_id: None,
        platform: Some("booking".to_string()),
        status: None,
        from_date: None,
        to_date: None,
        guest_name: None,
        page: Some(1),
        limit: Some(500),
    };

    let (booking_reservations, _) = rental_repo
        .list_bookings(path.org_id, booking_query)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list Booking.com bookings");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to list bookings",
                )),
            )
        })?;

    let truncated = booking_reservations.len() >= 500;
    if truncated {
        tracing::warn!(
            org_id = %path.org_id,
            "Booking.com conflict check hit the 500-reservation cap — results may be incomplete"
        );
    }

    let mut conflicts: Vec<BookingConflict> = Vec::new();

    for bk in &booking_reservations {
        // Skip cancelled — they don't block availability.
        if bk.status == "cancelled" {
            continue;
        }

        // Fetch other-platform bookings for the same unit in the overlapping window.
        let other_query = BookingListQuery {
            unit_id: Some(bk.unit_id),
            building_id: None,
            platform: None,
            status: None,
            from_date: Some(bk.check_in),
            to_date: Some(bk.check_out),
            guest_name: None,
            page: Some(1),
            limit: Some(100),
        };

        let candidates = rental_repo
            .list_bookings(path.org_id, other_query)
            .await
            .map(|(v, _)| v)
            .unwrap_or_default();

        for other in &candidates {
            // Skip same-platform and cancelled.
            if other.platform == "booking" || other.status == "cancelled" {
                continue;
            }
            // Date-range overlap: [A.start, A.end) overlaps [B.start, B.end) iff
            //   A.start < B.end  AND  A.end > B.start
            if bk.check_in < other.check_out && bk.check_out > other.check_in {
                let overlap_start = bk.check_in.max(other.check_in);
                let overlap_end = bk.check_out.min(other.check_out);

                tracing::warn!(
                    booking_id = %bk.id,
                    other_id   = %other.id,
                    platform   = %other.platform,
                    unit_id    = %bk.unit_id,
                    %overlap_start,
                    %overlap_end,
                    "Cross-platform conflict detected"
                );

                conflicts.push(BookingConflict {
                    unit_id: bk.unit_id,
                    booking_reservation_id: bk
                        .external_booking_id
                        .clone()
                        .unwrap_or_else(|| bk.id.to_string()),
                    conflicting_platform: other.platform.clone(),
                    conflicting_booking_id: other.id,
                    overlap_start,
                    overlap_end,
                });
            }
        }
    }

    let conflicts_found = conflicts.len() as i32;

    tracing::info!(
        org_id = %path.org_id,
        conflicts_found = conflicts_found,
        "Booking.com conflict check completed"
    );

    Ok(Json(ConflictCheckResponse {
        conflicts_found,
        conflicts,
        checked_at: chrono::Utc::now(),
        truncated,
    }))
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_listing_push_request_deserializes() {
        let json = serde_json::json!({
            "unit_id": "00000000-0000-0000-0000-000000000001",
            "room_type_id": "DBL",
            "availability": [
                {
                    "room_type_id": "DBL",
                    "date": "2025-06-01",
                    "available_count": 2,
                    "stop_sell": false
                }
            ],
            "rates": [
                {
                    "room_type_id": "DBL",
                    "rate_plan_code": "STD",
                    "date": "2025-06-01",
                    "base_rate": "120.00",
                    "currency": "EUR"
                }
            ]
        });

        let req: ListingPushRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.room_type_id, "DBL");
        assert_eq!(req.availability.len(), 1);
        assert_eq!(req.rates.len(), 1);
        assert_eq!(req.rates[0].rate_plan_code, "STD");
    }

    #[test]
    fn test_room_rate_entry_default_currency() {
        let json = serde_json::json!({
            "room_type_id": "SGL",
            "rate_plan_code": "NRF",
            "date": "2025-07-15",
            "base_rate": "95.00"
        });

        let entry: RoomRateEntry = serde_json::from_value(json).unwrap();
        assert_eq!(entry.currency, "EUR");
    }

    #[test]
    fn test_conflict_check_response_serializes() {
        let response = ConflictCheckResponse {
            conflicts_found: 1,
            conflicts: vec![BookingConflict {
                unit_id: Uuid::new_v4(),
                booking_reservation_id: "BK12345".to_string(),
                conflicting_platform: "airbnb".to_string(),
                conflicting_booking_id: Uuid::new_v4(),
                overlap_start: NaiveDate::from_ymd_opt(2025, 6, 10).unwrap(),
                overlap_end: NaiveDate::from_ymd_opt(2025, 6, 12).unwrap(),
            }],
            checked_at: chrono::Utc::now(),
            truncated: false,
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["conflicts_found"], 1);
        assert_eq!(json["conflicts"][0]["conflicting_platform"], "airbnb");
    }

    #[test]
    fn test_availability_entry_default_stop_sell() {
        let json = serde_json::json!({
            "room_type_id": "DBL",
            "date": "2025-08-01",
            "available_count": 3
        });

        let entry: RoomAvailabilityEntry = serde_json::from_value(json).unwrap();
        assert!(!entry.stop_sell);
        assert_eq!(entry.available_count, 3);
    }
}
