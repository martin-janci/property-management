//! Owner/realtor listing CRUD routes (Epic 15.1/15.2).
//!
//! Uses `PortalPrincipal` — portal users have `principal_kind = 'public'` and
//! no `organization_members` row, so `RequestPrincipal` would 403 them.
//! IDOR is closed by the SECURITY DEFINER DB functions (migration 00186) which
//! gate all writes on `portal_owner_id = user_id OR created_by = user_id`.

use crate::state::AppState;
use api_core::extractors::PortalPrincipal;
use axum::{
    extract::{Path, State},
    routing::{get, patch, post},
    Json, Router,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Create portal listings router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_listing))
        .route("/{id}", get(get_my_listing))
        .route("/{id}", patch(update_listing))
}

// ============================================================================
// Enum allow-lists.
//
// These are the friendly `400` layer. The authoritative domains are also
// enforced at the database via CHECK constraints (migration 00194,
// `listings_*_check`) so an out-of-domain value cannot be persisted by any
// caller. Keep these lists in sync with migration 00194 and the inline domain
// comments in migration 00049_create_listings.
// ============================================================================

/// Allowed `property_type` values (`listings.property_type`).
/// Keep in sync with migration 00194 (`listings_property_type_check`) and 00049.
const ALLOWED_PROPERTY_TYPES: &[&str] = &[
    "apartment",
    "house",
    "commercial",
    "land",
    "parking",
    "storage",
    "other",
];

/// Allowed `transaction_type` values (`listings.transaction_type`).
/// Keep in sync with migration 00194 (`listings_transaction_type_check`) and 00049.
const ALLOWED_TRANSACTION_TYPES: &[&str] = &["sale", "rent"];

/// Allowed `currency` values (`listings.currency`).
/// Keep in sync with migration 00194 (`listings_currency_check`) and 00049.
const ALLOWED_CURRENCIES: &[&str] = &["EUR", "CZK"];

/// Statuses an owner may set directly via create/update.
///
/// Public visibility is gated by `is_published` (set only through the
/// moderation path — see migration 00186), so an owner must not be able to
/// flip a listing into the publicly-visible `active` state, nor into any
/// `published`/`approved`-style moderated state. They may only move a listing
/// between its own draft/paused/sold/rented/archived lifecycle states.
///
/// Narrower than the DB `listings_status_check` (migration 00194), which also
/// permits `active` — that moderation-only state is written by the moderation
/// path, never by an owner, so it is intentionally excluded here. Keep the
/// non-moderation values in sync with migration 00194 and 00049.
const ALLOWED_OWNER_STATUSES: &[&str] = &["draft", "paused", "sold", "rented", "archived"];

/// Validate an optional enum-like field against an allow-list.
///
/// `None` (field omitted) always passes. An unknown value yields a
/// `400 Bad Request` with a deterministic message.
fn validate_enum(
    field: &str,
    value: Option<&str>,
    allowed: &[&str],
) -> Result<(), (axum::http::StatusCode, String)> {
    if let Some(v) = value {
        if !allowed.contains(&v) {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                format!(
                    "Invalid {field}: '{v}'. Allowed values: {}",
                    allowed.join(", ")
                ),
            ));
        }
    }
    Ok(())
}

/// Request body for creating a portal listing.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatePortalListingRequest {
    pub title: String,
    pub description: Option<String>,
    pub property_type: String,
    pub transaction_type: String,
    pub price: Decimal,
    pub currency: Option<String>,
    pub street: String,
    pub city: String,
    pub postal_code: String,
    pub country: Option<String>,
    pub size_sqm: Option<Decimal>,
    pub rooms: Option<i32>,
    pub floor: Option<i32>,
    pub total_floors: Option<i32>,
}

/// Request body for patching a portal listing. All fields are optional.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePortalListingRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub property_type: Option<String>,
    pub transaction_type: Option<String>,
    pub price: Option<Decimal>,
    pub currency: Option<String>,
    pub street: Option<String>,
    pub city: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub size_sqm: Option<Decimal>,
    pub rooms: Option<i32>,
    pub floor: Option<i32>,
    pub total_floors: Option<i32>,
    pub status: Option<String>,
    pub is_negotiable: Option<bool>,
}

/// Listing response for portal owner/editor.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PortalListingResponse {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub property_type: String,
    pub transaction_type: String,
    pub price: Decimal,
    pub currency: String,
    pub street: String,
    pub city: String,
    pub postal_code: String,
    pub country: String,
    pub size_sqm: Option<Decimal>,
    pub rooms: Option<i32>,
    pub floor: Option<i32>,
    pub total_floors: Option<i32>,
    pub status: String,
    pub is_negotiable: bool,
    pub is_published: bool,
    pub slug: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

fn map_listing(l: db::models::Listing) -> PortalListingResponse {
    PortalListingResponse {
        id: l.id,
        title: l.title,
        description: l.description,
        property_type: l.property_type,
        transaction_type: l.transaction_type,
        price: l.price,
        currency: l.currency,
        street: l.street,
        city: l.city,
        postal_code: l.postal_code,
        country: l.country,
        size_sqm: l.size_sqm,
        rooms: l.rooms,
        floor: l.floor,
        total_floors: l.total_floors,
        status: l.status,
        is_negotiable: l.is_negotiable,
        is_published: l.is_published,
        slug: None,
        created_at: l.created_at,
        updated_at: l.updated_at,
    }
}

/// Create a new portal listing owned by the authenticated user.
///
/// POST /api/v1/my/listings
#[utoipa::path(
    post,
    path = "/api/v1/my/listings",
    tag = "PortalListings",
    request_body = CreatePortalListingRequest,
    responses(
        (status = 201, description = "Listing created", body = PortalListingResponse),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Validation error")
    )
)]
pub async fn create_listing(
    State(state): State<AppState>,
    principal: PortalPrincipal,
    Json(body): Json<CreatePortalListingRequest>,
) -> Result<(axum::http::StatusCode, Json<PortalListingResponse>), (axum::http::StatusCode, String)>
{
    // Reject unknown enum-like values before hitting the DB (free-text columns
    // have no CHECK constraint — see migration 00049).
    validate_enum(
        "propertyType",
        Some(body.property_type.as_str()),
        ALLOWED_PROPERTY_TYPES,
    )?;
    validate_enum(
        "transactionType",
        Some(body.transaction_type.as_str()),
        ALLOWED_TRANSACTION_TYPES,
    )?;
    validate_enum("currency", body.currency.as_deref(), ALLOWED_CURRENCIES)?;
    // NOTE: `status` is intentionally NOT accepted on create — `portal_create_listing`
    // (migration 00186) hard-codes it to 'draft'. If a `status` field is ever added to
    // `CreatePortalListingRequest`, it must also be guarded here via
    // `validate_enum("status", ..., ALLOWED_OWNER_STATUSES)` so create matches update.

    let listing = state
        .reality_portal_repo
        .create_portal_listing(
            principal.user_id,
            &body.title,
            body.description.as_deref(),
            &body.property_type,
            &body.transaction_type,
            body.price,
            body.currency.as_deref().unwrap_or("EUR"),
            &body.street,
            &body.city,
            &body.postal_code,
            body.country.as_deref().unwrap_or("SK"),
            body.size_sqm,
            body.rooms,
            body.floor,
            body.total_floors,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, user_id = %principal.user_id, "Failed to create portal listing");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create listing".to_string(),
            )
        })?;

    Ok((axum::http::StatusCode::CREATED, Json(map_listing(listing))))
}

/// Get a portal listing owned by the authenticated user (for editing).
///
/// GET /api/v1/my/listings/{id}
#[utoipa::path(
    get,
    path = "/api/v1/my/listings/{id}",
    tag = "PortalListings",
    params(("id" = Uuid, Path, description = "Listing ID")),
    responses(
        (status = 200, description = "Listing detail", body = PortalListingResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found or not owned")
    )
)]
pub async fn get_my_listing(
    State(state): State<AppState>,
    principal: PortalPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<PortalListingResponse>, (axum::http::StatusCode, String)> {
    let listing = state
        .reality_portal_repo
        .get_portal_listing(id, principal.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, listing_id = %id, user_id = %principal.user_id, "Failed to get portal listing");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get listing".to_string(),
            )
        })?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Listing not found".to_string(),
            )
        })?;

    Ok(Json(map_listing(listing)))
}

/// Update a portal listing owned by the authenticated user.
///
/// PATCH /api/v1/my/listings/{id}
#[utoipa::path(
    patch,
    path = "/api/v1/my/listings/{id}",
    tag = "PortalListings",
    params(("id" = Uuid, Path, description = "Listing ID")),
    request_body = UpdatePortalListingRequest,
    responses(
        (status = 200, description = "Updated listing", body = PortalListingResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found or not owned")
    )
)]
pub async fn update_listing(
    State(state): State<AppState>,
    principal: PortalPrincipal,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdatePortalListingRequest>,
) -> Result<Json<PortalListingResponse>, (axum::http::StatusCode, String)> {
    // Reject unknown enum-like values (free-text columns, no DB CHECK).
    validate_enum(
        "propertyType",
        body.property_type.as_deref(),
        ALLOWED_PROPERTY_TYPES,
    )?;
    validate_enum(
        "transactionType",
        body.transaction_type.as_deref(),
        ALLOWED_TRANSACTION_TYPES,
    )?;
    validate_enum("currency", body.currency.as_deref(), ALLOWED_CURRENCIES)?;
    // Owners may only move a listing between its own lifecycle states; flipping
    // into the publicly-visible `active`/`published`/`approved` states is a
    // privileged transition reserved for the moderation path.
    validate_enum("status", body.status.as_deref(), ALLOWED_OWNER_STATUSES)?;

    let listing = state
        .reality_portal_repo
        .update_portal_listing(
            id,
            principal.user_id,
            body.title.as_deref(),
            body.description.as_deref(),
            body.property_type.as_deref(),
            body.transaction_type.as_deref(),
            body.price,
            body.currency.as_deref(),
            body.street.as_deref(),
            body.city.as_deref(),
            body.postal_code.as_deref(),
            body.country.as_deref(),
            body.size_sqm,
            body.rooms,
            body.floor,
            body.total_floors,
            body.status.as_deref(),
            body.is_negotiable,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, listing_id = %id, user_id = %principal.user_id, "Failed to update portal listing");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update listing".to_string(),
            )
        })?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Listing not found or not owned".to_string(),
            )
        })?;

    Ok(Json(map_listing(listing)))
}
