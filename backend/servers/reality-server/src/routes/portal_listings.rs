//! Owner/realtor listing CRUD routes (Epic 15.1/15.2).
//!
//! Uses `PortalPrincipal` — portal users have `principal_kind = 'public'` and
//! no `organization_members` row, so `RequestPrincipal` would 403 them.
//! IDOR is closed by the SECURITY DEFINER DB functions (migration 00186) which
//! gate all writes on `portal_owner_id = user_id OR created_by = user_id`.

use crate::state::AppState;
use api_core::extractors::PortalPrincipal;
use axum::{
    extract::{Path, Query, State},
    routing::{get, patch, post},
    Json, Router,
};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Create portal listings router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_listing))
        .route("/", get(list_my_listings))
        .route("/{id}", get(get_my_listing))
        .route("/{id}", patch(update_listing))
        .route("/{id}/analytics", get(get_my_listing_analytics))
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

// ============================================================================
// Realtor "my listings" LIST + per-listing analytics (Epic 15 / Story 33.4).
//
// These close the backend gap that left the mobile MyListings / ListingAnalytics
// screens and the web realtor dashboard rendering permanent empty stubs: there
// was no LIST endpoint over a portal user's own listings and no HTTP surface for
// the existing per-listing `get_listing_analytics` repo method. Both are gated by
// `PortalPrincipal` (portal owners have no org membership) and scoped to the
// caller's own rows via the portal-owner RLS context / SECURITY DEFINER ownership
// check — never cross-user.
// ============================================================================

/// Query parameters for listing a portal user's own listings.
#[derive(Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct MyListingsQuery {
    /// Optional status filter (e.g. `draft`, `active`, `paused`, `sold`,
    /// `rented`, `archived`). Omit to return all of the caller's listings.
    pub status: Option<String>,
    /// Page size (default 20, clamped to 1..=100).
    pub limit: Option<i32>,
    /// Row offset (default 0).
    pub offset: Option<i32>,
}

/// Paginated response for a portal user's own listings.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MyListingsResponse {
    pub listings: Vec<PortalListingResponse>,
    pub total: i64,
}

/// List the authenticated portal user's own listings.
///
/// GET /api/v1/my/listings
#[utoipa::path(
    get,
    path = "/api/v1/my/listings",
    tag = "PortalListings",
    params(MyListingsQuery),
    responses(
        (status = 200, description = "The caller's listings", body = MyListingsResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_my_listings(
    State(state): State<AppState>,
    principal: PortalPrincipal,
    Query(query): Query<MyListingsQuery>,
) -> Result<Json<MyListingsResponse>, (axum::http::StatusCode, String)> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = crate::util::clamp_offset_i32(query.offset);
    let status = query.status.as_deref();

    let listings = state
        .reality_portal_repo
        .list_portal_listings(principal.user_id, status, limit, offset)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, user_id = %principal.user_id, "Failed to list portal listings");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to list listings".to_string(),
            )
        })?;

    let total = state
        .reality_portal_repo
        .count_portal_listings(principal.user_id, status)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, user_id = %principal.user_id, "Failed to count portal listings");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to count listings".to_string(),
            )
        })?;

    Ok(Json(MyListingsResponse {
        listings: listings.into_iter().map(map_listing).collect(),
        total,
    }))
}

/// Query parameters for per-listing analytics.
#[derive(Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ListingAnalyticsQuery {
    /// Inclusive lower bound on the analytics day (ISO-8601 date). Omit for no
    /// lower bound.
    pub from_date: Option<NaiveDate>,
    /// Inclusive upper bound on the analytics day (ISO-8601 date). Omit for no
    /// upper bound.
    pub to_date: Option<NaiveDate>,
}

/// One day's analytics counters — camelCase wire mirror of
/// `db::models::ListingAnalytics` (the model is snake_case; the rest of the
/// portal API is camelCase, so the endpoint serves a camelCase DTO for a
/// consistent, wireable contract).
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DailyListingAnalytics {
    pub date: NaiveDate,
    pub views: i32,
    pub unique_views: i32,
    pub favorites_added: i32,
    pub favorites_removed: i32,
    pub inquiries: i32,
    pub phone_clicks: i32,
    pub share_clicks: i32,
    pub source_website: i32,
    pub source_mobile: i32,
    pub source_search: i32,
    pub source_direct: i32,
}

fn map_daily_analytics(a: db::models::ListingAnalytics) -> DailyListingAnalytics {
    DailyListingAnalytics {
        date: a.date,
        views: a.views,
        unique_views: a.unique_views,
        favorites_added: a.favorites_added,
        favorites_removed: a.favorites_removed,
        inquiries: a.inquiries,
        phone_clicks: a.phone_clicks,
        share_clicks: a.share_clicks,
        source_website: a.source_website,
        source_mobile: a.source_mobile,
        source_search: a.source_search,
        source_direct: a.source_direct,
    }
}

/// Analytics summary + daily series for one of the caller's listings.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListingAnalyticsResponse {
    pub listing_id: Uuid,
    pub total_views: i64,
    pub total_inquiries: i64,
    pub total_favorites: i64,
    pub days_on_market: i32,
    pub daily_analytics: Vec<DailyListingAnalytics>,
}

/// Get analytics for one of the authenticated portal user's own listings.
///
/// GET /api/v1/my/listings/{id}/analytics
///
/// Ownership is enforced up front via `portal_get_listing` (SECURITY DEFINER,
/// migration 00186): a listing the caller does not own yields 404, so a realtor
/// can never read another realtor's analytics.
#[utoipa::path(
    get,
    path = "/api/v1/my/listings/{id}/analytics",
    tag = "PortalListings",
    params(
        ("id" = Uuid, Path, description = "Listing ID"),
        ListingAnalyticsQuery
    ),
    responses(
        (status = 200, description = "Analytics summary + daily series", body = ListingAnalyticsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found or not owned")
    )
)]
pub async fn get_my_listing_analytics(
    State(state): State<AppState>,
    principal: PortalPrincipal,
    Path(id): Path<Uuid>,
    Query(query): Query<ListingAnalyticsQuery>,
) -> Result<Json<ListingAnalyticsResponse>, (axum::http::StatusCode, String)> {
    // Ownership gate: only the portal owner may read analytics for their listing.
    let listing = state
        .reality_portal_repo
        .get_portal_listing(id, principal.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, listing_id = %id, user_id = %principal.user_id, "Failed to load listing for analytics");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load listing".to_string(),
            )
        })?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Listing not found or not owned".to_string(),
            )
        })?;

    // Read via the ownership-gated SECURITY DEFINER path: `listing_analytics`
    // is org-scoped under FORCE RLS with no portal-owner branch, so a plain
    // RLS-subject SELECT returns an empty series for portal-owned listings
    // (#2199). `get_portal_listing_analytics` bypasses that policy and gates on
    // `portal_owner_id = user_id OR created_by = user_id`.
    let daily = state
        .reality_portal_repo
        .get_portal_listing_analytics(id, principal.user_id, query.from_date, query.to_date)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, listing_id = %id, "Failed to load listing analytics");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load analytics".to_string(),
            )
        })?;

    let total_views: i64 = daily.iter().map(|d| i64::from(d.views)).sum();
    let total_inquiries: i64 = daily.iter().map(|d| i64::from(d.inquiries)).sum();
    // Net favorites = added − removed, floored at 0 (a listing can't have a
    // negative favorite count even if a day removed more than it added).
    let total_favorites: i64 = daily
        .iter()
        .map(|d| i64::from(d.favorites_added) - i64::from(d.favorites_removed))
        .sum::<i64>()
        .max(0);
    let days_on_market = i32::try_from((chrono::Utc::now() - listing.created_at).num_days().max(0))
        .unwrap_or(i32::MAX);

    Ok(Json(ListingAnalyticsResponse {
        listing_id: id,
        total_views,
        total_inquiries,
        total_favorites,
        days_on_market,
        daily_analytics: daily.into_iter().map(map_daily_analytics).collect(),
    }))
}
