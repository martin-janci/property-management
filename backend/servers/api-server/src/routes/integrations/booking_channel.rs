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
use integrations::{AvailabilityUpdate, BookingClient, PushOutcome, RateUpdate};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::sync::OrgIdPath;
use crate::state::AppState;
use common::errors::ErrorResponse;
use common::TenantRole;

/// Maximum number of availability/rate updates accepted in a single
/// `listing-push` request.
///
/// Each entry becomes one `AvailStatusMessage` / `RateAmountMessage` in the
/// generated OTA XML. Without a cap, an arbitrarily large list lets a caller
/// build an unbounded XML body, risking OOM locally and timeouts upstream.
/// Mirrors the `MAX_BATCH_SIZE` guard PR #607 added to the legacy
/// `install.rs` push endpoints (issue #572) — applied here to the unified
/// gap-83-2 OTA push surface, which had no such guard.
pub const MAX_BATCH_SIZE: usize = 500;

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
// Request validation
// ============================================================

/// Validate a [`ListingPushRequest`] before it is turned into OTA XML and
/// pushed to Booking.com.
///
/// These guards protect the OTA XML transport from producing malformed or
/// unbounded payloads:
///
/// * **Batch size** — the combined availability + rate update count must not
///   exceed [`MAX_BATCH_SIZE`]. Each entry maps to one OTA message element;
///   an unbounded list could OOM the encoder or time out upstream.
/// * **Non-negative availability** — a negative `available_count` serialises
///   to an invalid `BookingLimit` attribute in `OTA_HotelAvailNotifRQ`, which
///   Booking.com rejects. This mirrors the `available_count >= 0` guard from
///   PR #607.
/// * **Non-negative rate** — a negative `base_rate` serialises to an invalid
///   `AmountAfterTax` attribute in `OTA_HotelRateAmountNotifRQ` and is a
///   money-correctness footgun, so it is rejected fast with a 400 (symmetric
///   to the availability guard).
/// * **Well-formed currency** — `currency` is carried verbatim into the
///   `CurrencyCode` XML attribute, so a structurally invalid code (not a
///   3-letter ASCII-uppercase ISO-4217-shaped code) is rejected fast with a
///   400 instead of producing a document Booking.com rejects.
///
/// On failure, returns `(error_code, human_message)` so the caller can build
/// a `400` [`ErrorResponse`]. On success, returns `Ok(())`.
fn validate_listing_push(request: &ListingPushRequest) -> Result<(), (&'static str, &'static str)> {
    let total = request.availability.len() + request.rates.len();
    if total > MAX_BATCH_SIZE {
        return Err((
            "BATCH_TOO_LARGE",
            "A maximum of 500 availability + rate updates per request is allowed",
        ));
    }

    if request.availability.iter().any(|a| a.available_count < 0) {
        return Err((
            "INVALID_AVAILABLE_COUNT",
            "available_count must be non-negative",
        ));
    }

    if request
        .rates
        .iter()
        .any(|r| r.base_rate < rust_decimal::Decimal::ZERO)
    {
        return Err(("INVALID_RATE", "base_rate must be non-negative"));
    }

    if request
        .rates
        .iter()
        .any(|r| r.currency.len() != 3 || !r.currency.chars().all(|ch| ch.is_ascii_uppercase()))
    {
        return Err((
            "INVALID_CURRENCY",
            "currency must be a 3-letter ASCII-uppercase code",
        ));
    }

    Ok(())
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

    // Validate the payload before any DB lookup or OTA XML generation so a
    // malformed batch fails fast with a 400 instead of producing invalid XML.
    if let Err((code, message)) = validate_listing_push(&request) {
        tracing::warn!(
            org_id = %path.org_id,
            error_code = code,
            "Rejecting Booking.com listing push: {}",
            message
        );
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(code, message)),
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

    // BIT-99: credentials are stored encrypted; decrypt for OTA API use.
    // decrypt_if_available tolerates legacy plaintext rows (no "enc:" prefix).
    let crypto = integrations::IntegrationCrypto::try_from_env();
    let username = integrations::decrypt_if_available(crypto.as_ref(), &username);
    let password = integrations::decrypt_if_available(crypto.as_ref(), &password);

    let credentials = integrations::BookingCredentials::new(hotel_id.clone(), username, password);
    let client = BookingClient::new(credentials);

    // Map the request into the OTA push payloads. The combined push skips an
    // empty stream internally, so we build both unconditionally.
    let avail_updates = map_availability_updates(&request.availability);
    let rate_updates = map_rate_updates(&request.rates);

    // Push both OTA streams (OTA_HotelAvailNotifRQ + OTA_HotelRateAmountNotifRQ)
    // in one call so a partial failure is surfaced instead of silently losing
    // the half that already applied (AC-5).
    let outcome = client
        .push_availability_and_rates(&hotel_id, &avail_updates, &rate_updates)
        .await;

    let avail_pushed = outcome.availability_pushed as i32;
    let rates_pushed = outcome.rates_pushed as i32;

    match classify_listing_push(&outcome) {
        ListingPushResult::Success => {
            tracing::info!(
                org_id = %path.org_id,
                hotel_id = %hotel_id,
                avail_pushed,
                rates_pushed,
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
        // Total failure — nothing applied — keeps the loud 4xx contract callers
        // already rely on (no half-synced state to report).
        ListingPushResult::TotalFailure(message) => {
            tracing::error!(
                org_id = %path.org_id,
                hotel_id = %hotel_id,
                error = %message,
                "Booking.com listing push failed (nothing applied)"
            );
            Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "PUSH_ERROR",
                    format!("Listing push failed: {message}"),
                )),
            ))
        }
        // Genuine partial sync — one stream landed, the other failed. Report 200
        // with success=false + accurate per-stream counts so the caller can
        // reconcile the half-synced channel instead of seeing a bare error that
        // hides the applied half.
        ListingPushResult::Partial(message) => {
            tracing::warn!(
                org_id = %path.org_id,
                hotel_id = %hotel_id,
                avail_pushed,
                rates_pushed,
                error = %message,
                "Booking.com listing push partially applied — channel is half-synced"
            );

            Ok(Json(ListingPushResponse {
                success: false,
                availability_pushed: avail_pushed,
                rates_pushed,
                pushed_at: chrono::Utc::now(),
                error: Some(format!("partial sync: {message}")),
            }))
        }
    }
}

/// HTTP-mapping classification of a combined [`PushOutcome`].
///
/// Distinguishes a *genuine* partial sync (one stream applied, the other
/// failed → caller must reconcile a half-synced channel) from a total failure
/// (nothing applied). [`PushOutcome::is_partial`] alone is insufficient: it is
/// also true when one stream was *skipped* (empty) and the only attempted
/// stream failed — which is a total failure, not a half-sync.
#[derive(Debug)]
enum ListingPushResult {
    Success,
    Partial(String),
    TotalFailure(String),
}

/// Build a `"availability: …; rates: …"` summary of whichever stream(s) failed.
fn push_stream_errors(outcome: &PushOutcome) -> String {
    let mut parts = Vec::new();
    if let Some(e) = &outcome.availability_error {
        parts.push(format!("availability: {e}"));
    }
    if let Some(e) = &outcome.rates_error {
        parts.push(format!("rates: {e}"));
    }
    parts.join("; ")
}

/// Map the request's availability entries to OTA [`AvailabilityUpdate`]s
/// (the input to `OTA_HotelAvailNotifRQ`).
///
/// Pure request→OTA translation for the rate/availability push flow (AC-5),
/// extracted from the handler so the mapping — and the OTA XML it ultimately
/// produces — can be unit-tested without an HTTP round-trip. The route handler
/// does not currently set the CTA/CTD/LOS restriction fields, so they are
/// defaulted here; keeping the mapping in one place means a future restriction
/// surface only changes this function.
fn map_availability_updates(entries: &[RoomAvailabilityEntry]) -> Vec<AvailabilityUpdate> {
    entries
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
        .collect()
}

/// Map the request's rate entries to OTA [`RateUpdate`]s (the input to
/// `OTA_HotelRateAmountNotifRQ`).
///
/// Companion of [`map_availability_updates`] for the rate stream. The handler
/// does not expose extra-person/child pricing, so those are defaulted to `None`.
fn map_rate_updates(entries: &[RoomRateEntry]) -> Vec<RateUpdate> {
    entries
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
        .collect()
}

fn classify_listing_push(outcome: &PushOutcome) -> ListingPushResult {
    if outcome.is_success() {
        return ListingPushResult::Success;
    }
    let message = push_stream_errors(outcome);
    if outcome.availability_pushed == 0 && outcome.rates_pushed == 0 {
        ListingPushResult::TotalFailure(message)
    } else {
        ListingPushResult::Partial(message)
    }
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

    // ---- classify_listing_push (PushOutcome → HTTP mapping) ----

    #[test]
    fn classify_success_when_no_stream_errored() {
        let outcome = PushOutcome {
            availability_pushed: 3,
            rates_pushed: 2,
            availability_error: None,
            rates_error: None,
        };
        assert!(matches!(
            classify_listing_push(&outcome),
            ListingPushResult::Success
        ));
    }

    #[test]
    fn classify_partial_when_one_stream_landed_and_other_failed() {
        // Availability applied, rates rejected → half-synced channel.
        let outcome = PushOutcome {
            availability_pushed: 3,
            rates_pushed: 0,
            availability_error: None,
            rates_error: Some("rate plan not found".to_string()),
        };
        match classify_listing_push(&outcome) {
            ListingPushResult::Partial(msg) => assert!(msg.contains("rates:")),
            other => panic!("expected Partial, got {other:?}"),
        }
    }

    #[test]
    fn classify_total_failure_when_only_attempted_stream_failed() {
        // Rates were skipped (empty); availability was the only stream and it
        // failed → nothing applied. is_partial() is true here, so this guards
        // against treating a single-stream failure as a half-sync.
        let outcome = PushOutcome {
            availability_pushed: 0,
            rates_pushed: 0,
            availability_error: Some("auth rejected".to_string()),
            rates_error: None,
        };
        match classify_listing_push(&outcome) {
            ListingPushResult::TotalFailure(msg) => assert!(msg.contains("availability:")),
            other => panic!("expected TotalFailure, got {other:?}"),
        }
    }

    #[test]
    fn classify_total_failure_when_both_streams_failed() {
        let outcome = PushOutcome {
            availability_pushed: 0,
            rates_pushed: 0,
            availability_error: Some("a".to_string()),
            rates_error: Some("b".to_string()),
        };
        match classify_listing_push(&outcome) {
            ListingPushResult::TotalFailure(msg) => {
                assert!(msg.contains("availability:") && msg.contains("rates:"));
            }
            other => panic!("expected TotalFailure, got {other:?}"),
        }
    }

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

    /// Build a minimal valid `ListingPushRequest` with the given counts of
    /// availability and rate entries (all entries non-negative).
    fn make_request(avail: usize, rates: usize) -> ListingPushRequest {
        ListingPushRequest {
            unit_id: Uuid::new_v4(),
            room_type_id: "DBL".to_string(),
            availability: (0..avail)
                .map(|i| RoomAvailabilityEntry {
                    room_type_id: "DBL".to_string(),
                    date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap()
                        + chrono::Duration::days(i as i64),
                    available_count: 1,
                    stop_sell: false,
                })
                .collect(),
            rates: (0..rates)
                .map(|i| RoomRateEntry {
                    room_type_id: "DBL".to_string(),
                    rate_plan_code: "STD".to_string(),
                    date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap()
                        + chrono::Duration::days(i as i64),
                    base_rate: rust_decimal::Decimal::new(10000, 2),
                    currency: "EUR".to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn test_validate_listing_push_accepts_valid_batch() {
        let req = make_request(10, 10);
        assert!(validate_listing_push(&req).is_ok());
    }

    #[test]
    fn test_validate_listing_push_accepts_max_batch_boundary() {
        // Exactly MAX_BATCH_SIZE combined entries is allowed.
        let req = make_request(MAX_BATCH_SIZE, 0);
        assert!(validate_listing_push(&req).is_ok());
        let req = make_request(MAX_BATCH_SIZE / 2, MAX_BATCH_SIZE - MAX_BATCH_SIZE / 2);
        assert!(validate_listing_push(&req).is_ok());
    }

    #[test]
    fn test_validate_listing_push_rejects_oversized_batch() {
        // availability + rates combined exceeding the cap -> BATCH_TOO_LARGE.
        let req = make_request(MAX_BATCH_SIZE, 1);
        let err = validate_listing_push(&req).unwrap_err();
        assert_eq!(err.0, "BATCH_TOO_LARGE");
    }

    #[test]
    fn test_validate_listing_push_rejects_negative_available_count() {
        let mut req = make_request(2, 0);
        req.availability[1].available_count = -1;
        let err = validate_listing_push(&req).unwrap_err();
        assert_eq!(err.0, "INVALID_AVAILABLE_COUNT");
    }

    #[test]
    fn test_validate_listing_push_rejects_negative_base_rate() {
        let mut req = make_request(0, 2);
        req.rates[1].base_rate = rust_decimal::Decimal::new(-1, 0);
        let err = validate_listing_push(&req).unwrap_err();
        assert_eq!(err.0, "INVALID_RATE");
    }

    #[test]
    fn test_validate_listing_push_rejects_invalid_currency() {
        let mut req = make_request(0, 2);
        req.rates[1].currency = "eur".to_string();
        let err = validate_listing_push(&req).unwrap_err();
        assert_eq!(err.0, "INVALID_CURRENCY");
    }

    #[test]
    fn test_validate_listing_push_zero_available_count_is_valid() {
        // 0 means sold out — a valid OTA `BookingLimit`, not a rejection.
        let mut req = make_request(1, 0);
        req.availability[0].available_count = 0;
        assert!(validate_listing_push(&req).is_ok());
    }

    #[test]
    fn test_validate_listing_push_empty_request_is_valid() {
        // No availability and no rates is a no-op push, not an error here;
        // the handler simply pushes nothing.
        let req = make_request(0, 0);
        assert!(validate_listing_push(&req).is_ok());
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

    // ---- request → OTA-update mapping + end-to-end XML (AC-5 push flow) ----

    #[test]
    fn test_map_availability_updates_carries_fields_and_defaults_restrictions() {
        let req = make_request(2, 0);
        let updates = map_availability_updates(&req.availability);

        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].room_type_id, "DBL");
        assert_eq!(updates[0].available_count, 1);
        assert!(!updates[0].stop_sell);
        // Restriction fields are not exposed by the route → defaulted.
        assert!(!updates[0].cta);
        assert!(!updates[0].ctd);
        assert!(updates[0].min_los.is_none());
        assert!(updates[0].max_los.is_none());
    }

    #[test]
    fn test_map_availability_updates_preserves_stop_sell() {
        let mut req = make_request(1, 0);
        req.availability[0].stop_sell = true;
        let updates = map_availability_updates(&req.availability);
        assert!(updates[0].stop_sell);
    }

    #[test]
    fn test_map_rate_updates_carries_fields_and_defaults_extras() {
        let req = make_request(0, 2);
        let updates = map_rate_updates(&req.rates);

        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].room_type_id, "DBL");
        assert_eq!(updates[0].rate_plan_code, "STD");
        assert_eq!(updates[0].currency, "EUR");
        assert_eq!(updates[0].base_rate, rust_decimal::Decimal::new(10000, 2));
        // Extra-person / extra-child pricing is not exposed by the route.
        assert!(updates[0].extra_person_rate.is_none());
        assert!(updates[0].extra_child_rate.is_none());
    }

    #[test]
    fn test_empty_request_maps_to_empty_update_streams() {
        let req = make_request(0, 0);
        assert!(map_availability_updates(&req.availability).is_empty());
        assert!(map_rate_updates(&req.rates).is_empty());
    }

    /// End-to-end push-flow shape (AC-5): a route `ListingPushRequest` maps to
    /// OTA updates that serialise to a valid `OTA_HotelAvailNotifRQ` carrying the
    /// requested availability. This is the route-boundary half of the push flow;
    /// the transport/retry half is covered in `integrations::booking` tests.
    #[test]
    fn test_request_availability_maps_to_valid_ota_avail_xml() {
        let mut req = make_request(1, 0);
        req.availability[0].available_count = 5;

        let updates = map_availability_updates(&req.availability);
        let rq = integrations::OtaHotelAvailNotifRQ {
            hotel_code: "HOTEL-1".to_string(),
            avail_status_messages: updates
                .iter()
                .map(|u| integrations::AvailStatusMessage {
                    room_type_code: u.room_type_id.clone(),
                    rate_plan_code: None,
                    start_date: u.date,
                    end_date: u.date,
                    booking_limit: u.available_count,
                    status: if u.stop_sell { "Close" } else { "Open" }.to_string(),
                    los_restrictions: None,
                })
                .collect(),
        };

        let xml = rq.to_xml();
        assert!(xml.contains("OTA_HotelAvailNotifRQ"), "xml: {xml}");
        assert!(xml.contains("HOTEL-1"), "xml: {xml}");
        assert!(xml.contains("DBL"), "xml: {xml}");
        // available_count = 5 surfaces as the OTA BookingLimit.
        assert!(xml.contains("BookingLimit=\"5\""), "xml: {xml}");
    }

    /// End-to-end push-flow shape (AC-5): a route `ListingPushRequest` maps to
    /// OTA updates that serialise to a valid `OTA_HotelRateAmountNotifRQ`
    /// carrying the requested rate.
    #[test]
    fn test_request_rates_map_to_valid_ota_rate_xml() {
        let req = make_request(0, 1);
        let updates = map_rate_updates(&req.rates);
        let rq = integrations::OtaHotelRateAmountNotifRQ {
            hotel_code: "HOTEL-1".to_string(),
            rate_amount_messages: updates,
        };

        let xml = rq.to_xml().expect("rate xml builds");
        assert!(xml.contains("OTA_HotelRateAmountNotifRQ"), "xml: {xml}");
        assert!(xml.contains("HOTEL-1"), "xml: {xml}");
        assert!(xml.contains("STD"), "xml: {xml}");
        // base_rate 100.00 EUR surfaces in the amount/currency.
        assert!(xml.contains("100.00"), "xml: {xml}");
        assert!(xml.contains("EUR"), "xml: {xml}");
    }
}
