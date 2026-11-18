//! Public listing routes - search and view (Story 16.1).

use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use db::models::user::Locale;
use db::models::{PublicListingQuery, PublicListingSummary};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Default page size for listing search when no `limit` is supplied.
const DEFAULT_LISTINGS_LIMIT: i32 = 20;
/// Maximum page size accepted for listing search. Larger requests are clamped
/// to this cap to avoid resource exhaustion once the table grows.
const MAX_LISTINGS_LIMIT: i32 = 100;

/// Clamp raw `page`/`limit` query params to safe bounds before they reach SQL.
///
/// A negative `limit` would otherwise surface a raw DB error
/// ("LIMIT must not be negative") as a 500, and an unbounded `limit`/`page` is a
/// latent resource-exhaustion vector. Returns `(page, limit)` with
/// `page >= 1` and `1 <= limit <= MAX_LISTINGS_LIMIT`.
fn clamp_pagination(page: Option<i32>, limit: Option<i32>) -> (i32, i32) {
    let page = page.unwrap_or(1).max(1);
    let limit = limit
        .unwrap_or(DEFAULT_LISTINGS_LIMIT)
        .clamp(1, MAX_LISTINGS_LIMIT);
    (page, limit)
}

/// Build the `PublicListingQuery` that reaches SQL from a raw search request,
/// clamping pagination first.
///
/// This is the single source of truth for the clamp-then-wire step used by the
/// `search` handler: it guarantees the clamped `(page, limit)` — not the raw
/// request values — are what end up in the query sent to the DB. Returning the
/// clamped pair as well lets the handler echo the effective values in the
/// response.
fn build_listing_query(req: ListingSearchRequest) -> (PublicListingQuery, i32, i32) {
    let (page, limit) = clamp_pagination(req.page, req.limit);
    let mut query: PublicListingQuery = req.into();
    query.page = Some(page);
    query.limit = Some(limit);
    (query, page, limit)
}

/// Create listings router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(search))
        // NOTE: Static segments (`/featured`, `/categories`, `/suggestions`) MUST be
        // declared before the `/{id}` capture so axum's matcher prefers them.
        // Otherwise these would be parsed as listing UUIDs and 400 on every request.
        .route("/featured", get(get_featured))
        .route("/categories", get(get_categories))
        .route("/suggestions", get(get_suggestions))
        .route("/{id}", get(get_listing))
        .route("/{id}/view", axum::routing::post(record_view))
}

/// Full listing row from database for detail view.
#[derive(Debug, FromRow)]
struct FullListingRow {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub price: i64,
    pub currency: String,
    pub size_sqm: Option<i32>,
    pub rooms: Option<i32>,
    pub bathrooms: Option<i32>,
    pub floor: Option<i32>,
    pub total_floors: Option<i32>,
    pub street: String,
    pub city: String,
    pub postal_code: String,
    pub country: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub property_type: String,
    pub transaction_type: String,
    pub features: serde_json::Value,
    pub published_at: Option<DateTime<Utc>>,
}

/// Listing search request (maps to PublicListingQuery).
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct ListingSearchRequest {
    /// Search query (address, city, description)
    pub q: Option<String>,
    /// Property type (apartment, house, land, commercial)
    pub property_type: Option<String>,
    /// Transaction type (sale, rent)
    pub transaction_type: Option<String>,
    /// Minimum price
    pub price_min: Option<i64>,
    /// Maximum price
    pub price_max: Option<i64>,
    /// Minimum area (m2)
    pub area_min: Option<i32>,
    /// Maximum area (m2)
    pub area_max: Option<i32>,
    /// Minimum rooms
    pub rooms_min: Option<i32>,
    /// Maximum rooms
    pub rooms_max: Option<i32>,
    /// City
    pub city: Option<String>,
    /// Country code (SK, CZ, etc.)
    pub country: Option<String>,
    /// Page number
    pub page: Option<i32>,
    /// Page size
    pub limit: Option<i32>,
    /// Sort by (price_asc, price_desc, date_desc, area_asc)
    pub sort: Option<String>,
}

impl From<ListingSearchRequest> for PublicListingQuery {
    fn from(req: ListingSearchRequest) -> Self {
        Self {
            q: req.q,
            property_type: req.property_type,
            transaction_type: req.transaction_type,
            price_min: req.price_min,
            price_max: req.price_max,
            area_min: req.area_min,
            area_max: req.area_max,
            rooms_min: req.rooms_min,
            rooms_max: req.rooms_max,
            city: req.city,
            country: req.country,
            page: req.page,
            limit: req.limit,
            sort: req.sort,
        }
    }
}

/// Listing search response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ListingSearchResponse {
    /// List of listings
    pub listings: Vec<ListingSummary>,
    /// Total count
    pub total: i64,
    /// Current page
    pub page: i32,
    /// Page size
    pub limit: i32,
}

/// Listing summary for search results.
#[derive(Debug, Serialize, ToSchema)]
pub struct ListingSummary {
    /// Listing ID
    pub id: Uuid,
    /// Title
    pub title: String,
    /// URL slug: `{slugified-title}-{uuid}`. The trailing UUID is
    /// authoritative (the detail handler resolves by it); the title prefix is
    /// cosmetic/SEO. A bare UUID is still accepted for backward-compatible
    /// deep links. Mirrors the `slug` field emitted by `/featured`
    /// (`RichListingSummary`) so search results link to SEO-friendly paths too.
    pub slug: String,
    /// Short description
    pub description: Option<String>,
    /// Price
    pub price: i64,
    /// Currency
    pub currency: String,
    /// Area in m2
    pub area: Option<i32>,
    /// Number of rooms
    pub rooms: Option<i32>,
    /// City
    pub city: String,
    /// Main photo URL
    pub photo_url: Option<String>,
    /// Property type
    pub property_type: String,
    /// Transaction type
    pub transaction_type: String,
    /// Published date
    pub published_at: String,
}

impl From<PublicListingSummary> for ListingSummary {
    fn from(summary: PublicListingSummary) -> Self {
        // Derive the SEO slug the same way `/featured` does (via
        // `RichListingSummary`): `{slugified-title}-{uuid}`, degrading to the
        // bare UUID when the title slugifies to nothing. Computed before `title`
        // is moved into the struct below.
        let slug = crate::util::slug::listing_slug(&summary.title, summary.id);
        Self {
            id: summary.id,
            title: summary.title,
            slug,
            description: summary.description,
            price: summary.price,
            currency: summary.currency,
            area: summary.size_sqm,
            rooms: summary.rooms,
            city: summary.city,
            photo_url: summary.photo_url,
            property_type: summary.property_type,
            transaction_type: summary.transaction_type,
            published_at: summary.published_at.to_rfc3339(),
        }
    }
}

/// Full listing detail.
#[derive(Debug, Serialize, ToSchema)]
pub struct ListingDetail {
    /// Listing ID
    pub id: Uuid,
    /// Title
    pub title: String,
    /// Full description
    pub description: Option<String>,
    /// Price
    pub price: i64,
    /// Currency
    pub currency: String,
    /// Area in m2
    pub area: Option<i32>,
    /// Number of rooms
    pub rooms: Option<i32>,
    /// Number of bathrooms
    pub bathrooms: Option<i32>,
    /// Floor number
    pub floor: Option<i32>,
    /// Total floors in building
    pub total_floors: Option<i32>,
    /// Address
    pub address: String,
    /// City
    pub city: String,
    /// Country
    pub country: String,
    /// Latitude
    pub latitude: Option<f64>,
    /// Longitude
    pub longitude: Option<f64>,
    /// Property type
    pub property_type: String,
    /// Transaction type
    pub transaction_type: String,
    /// Photo URLs
    pub photos: Vec<String>,
    /// Features (parking, balcony, etc.)
    pub features: Vec<String>,
    /// Published date
    pub published_at: String,
    /// View count
    pub view_count: i64,
}

/// Search suggestions response.
#[derive(Debug, Serialize, ToSchema)]
pub struct SuggestionsResponse {
    /// Nearby cities
    pub cities: Vec<String>,
    /// Popular searches
    pub popular_searches: Vec<String>,
}

/// Search listings.
#[utoipa::path(
    get,
    path = "/api/v1/listings",
    tag = "Listings",
    params(ListingSearchRequest),
    responses(
        (status = 200, description = "Search results", body = ListingSearchResponse)
    )
)]
pub async fn search(
    State(state): State<AppState>,
    Query(req): Query<ListingSearchRequest>,
) -> Result<Json<ListingSearchResponse>, (axum::http::StatusCode, String)> {
    // Clamp pagination at the handler before it reaches SQL (mirrors the
    // articles handler pattern). `build_listing_query` is the single source of
    // truth for the clamp-then-wire step.
    let (query, page, limit) = build_listing_query(req);

    // Search listings
    let listings = state
        .portal_repo
        .search_listings(&query)
        .await
        .map_err(|e| crate::util::errors::db_error("search listings", e))?;

    // Count total
    let total = state
        .portal_repo
        .count_listings(&query)
        .await
        .map_err(|e| crate::util::errors::db_error("count listings", e))?;

    // Convert to response types
    let listings: Vec<ListingSummary> = listings.into_iter().map(Into::into).collect();

    Ok(Json(ListingSearchResponse {
        listings,
        total,
        page,
        limit,
    }))
}

/// Get listing detail.
#[utoipa::path(
    get,
    path = "/api/v1/listings/{id}",
    tag = "Listings",
    params(
        ("id" = String, Path, description = "Listing slug (`title-<uuid>`) or bare UUID")
    ),
    responses(
        (status = 200, description = "Listing detail", body = ListingDetail),
        (status = 404, description = "Listing not found")
    )
)]
pub async fn get_listing(
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
) -> Result<Json<ListingDetail>, (axum::http::StatusCode, String)> {
    // Accept both the SEO slug (`title-<uuid>`) and a bare UUID. An
    // unresolvable segment is a 404 (the resource can't exist), not a 400.
    let id = crate::util::slug::parse_listing_id(&id_or_slug).ok_or((
        axum::http::StatusCode::NOT_FOUND,
        "Listing not found".to_string(),
    ))?;
    tracing::info!(%id, "Get listing detail");

    // Generic public-facing error message for any DB-side failure. The detailed
    // error is logged server-side so ops can investigate, but the client only
    // sees "Internal server error" — raw sqlx::Error can expose pool state,
    // connection strings fragments, or migration details.
    fn db_error(
        context: &'static str,
        id: Uuid,
        e: sqlx::Error,
    ) -> (axum::http::StatusCode, String) {
        tracing::error!(%id, error = %e, "{context}");
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        )
    }

    // Acquire a dedicated connection and clear any stale RLS context before
    // running these public queries (defense-in-depth against context bleeding).
    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| db_error("Failed to acquire db connection", id, e))?;

    // Query the full listing with address and coordinates
    let listing = sqlx::query_as::<_, FullListingRow>(
        r#"
        SELECT
            l.id, l.title, l.description, l.price::bigint AS price, l.currency,
            l.size_sqm::int AS size_sqm, l.rooms, l.bathrooms, l.floor, l.total_floors,
            l.street, l.city, l.postal_code, l.country,
            l.latitude::float8 AS latitude, l.longitude::float8 AS longitude,
            l.property_type, l.transaction_type, l.features,
            l.published_at
        FROM listings l
        WHERE l.id = $1 AND l.status = 'active'
        "#,
    )
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| db_error("Failed to fetch listing", id, e))?;

    match listing {
        Some(l) => {
            // Track the view
            let _ = state.reality_portal_repo.track_view(id, "website").await;

            // Read back the aggregate view total via the SECURITY DEFINER path
            // (a direct SELECT on this RLS-subject public pool reads zero for
            // portal-owned listings — see get_view_count / migration 00220).
            // Best-effort: degrade to 0 rather than failing the public detail
            // page if analytics are momentarily unavailable.
            let view_count = match state.reality_portal_repo.get_view_count(id).await {
                Ok(count) => count,
                Err(e) => {
                    tracing::warn!(%id, error = %e, "Failed to read listing view count; defaulting to 0");
                    0
                }
            };

            // Get photos for the listing
            let photos: Vec<String> = sqlx::query_scalar(
                "SELECT url FROM listing_photos WHERE listing_id = $1 ORDER BY display_order",
            )
            .bind(id)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| {
                tracing::error!(%id, error = %e, "Failed to fetch listing photos");
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            })?;

            // Build full address
            let address = format!("{}, {} {}", l.street, l.postal_code, l.city);

            // Parse features from JSON
            let features: Vec<String> = l
                .features
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            Ok(Json(ListingDetail {
                id: l.id,
                title: l.title,
                description: l.description,
                price: l.price,
                currency: l.currency,
                area: l.size_sqm,
                rooms: l.rooms,
                bathrooms: l.bathrooms,
                floor: l.floor,
                total_floors: l.total_floors,
                address,
                city: l.city,
                country: l.country,
                latitude: l.latitude,
                longitude: l.longitude,
                property_type: l.property_type,
                transaction_type: l.transaction_type,
                photos,
                features,
                published_at: l.published_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
                view_count,
            }))
        }
        None => Err((
            axum::http::StatusCode::NOT_FOUND,
            "Listing not found".to_string(),
        )),
    }
}

/// Get search suggestions.
#[utoipa::path(
    get,
    path = "/api/v1/listings/suggestions",
    tag = "Listings",
    params(
        ("city" = Option<String>, Query, description = "Current city for nearby suggestions")
    ),
    responses(
        (status = 200, description = "Search suggestions", body = SuggestionsResponse)
    )
)]
pub async fn get_suggestions(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Json<SuggestionsResponse>, (axum::http::StatusCode, String)> {
    let locale = resolve_locale(params.get("locale").map(|s| s.as_str()), &headers);
    let city = params
        .get("city")
        .map(|s| s.as_str())
        .unwrap_or("Bratislava");

    let cities = state
        .portal_repo
        .get_nearby_cities(city, 10)
        .await
        .map_err(|e| crate::util::errors::db_error("get nearby cities", e))?;

    // Derive popular searches from real inventory: the (property_type, city)
    // pairs with the most active listings. Public search queries are not logged,
    // so this is a best-effort proxy for what visitors are likely to search.
    let popular_searches = state
        .portal_repo
        .get_popular_search_pairs(5)
        .await
        .map_err(|e| crate::util::errors::db_error("get popular search pairs", e))?
        .into_iter()
        .map(|(property_type, city)| popular_search_phrase(&property_type, &city, &locale))
        .collect();

    Ok(Json(SuggestionsResponse {
        cities,
        popular_searches,
    }))
}

/// Build a localized popular-search phrase from a `(property_type, city)` pair,
/// e.g. `("apartment", "Bratislava", &Locale::Slovak) -> "Byty Bratislava"`.
/// Reuses the same `label_for_property_type` mapping as the categories endpoint
/// so the two surfaces stay consistent.
fn popular_search_phrase(property_type: &str, city: &str, locale: &Locale) -> String {
    format!(
        "{} {}",
        label_for_property_type(property_type, locale),
        city
    )
}

// ============================================================================
// Rich listing summary types — match frontend `ListingSummary` shape
// (camelCase, nested address + primaryPhoto). Used by /featured.
// ============================================================================

/// Address sub-object on a rich listing summary.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RichListingAddress {
    pub city: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub district: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    pub country: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
}

/// Primary photo descriptor on a rich listing summary.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RichListingPhoto {
    pub id: Uuid,
    pub url: String,
    pub thumbnail_url: String,
    pub is_primary: bool,
    pub order: i32,
}

/// Rich listing summary returned by /featured. Camel-cased to match the
/// `ListingSummary` TypeScript type in @ppt/reality-api-client.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RichListingSummary {
    pub id: Uuid,
    pub title: String,
    /// URL slug: `{slugified-title}-{uuid}`. The trailing UUID is
    /// authoritative (the detail handler resolves by it); the title prefix is
    /// cosmetic/SEO. A bare UUID is still accepted for backward-compatible
    /// deep links.
    pub slug: String,
    pub property_type: String,
    pub transaction_type: String,
    pub status: String,
    pub price: i64,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_per_sqm: Option<i64>,
    pub area: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rooms: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bedrooms: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bathrooms: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub floor: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_floors: Option<i32>,
    pub address: RichListingAddress,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_photo: Option<RichListingPhoto>,
    pub is_featured: bool,
    pub is_favorite: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Featured listings response. Three buckets, matches `FeaturedListingsResponse`
/// in @ppt/reality-api-client.
#[derive(Debug, Serialize, ToSchema)]
pub struct FeaturedListingsResponse {
    pub sale: Vec<RichListingSummary>,
    pub rent: Vec<RichListingSummary>,
    pub new: Vec<RichListingSummary>,
}

/// Category count entry. Matches `CategoryCount` in @ppt/reality-api-client.
#[derive(Debug, Serialize, ToSchema)]
pub struct CategoryCount {
    /// Property type key — e.g. "apartment", "house", "land", "commercial".
    #[serde(rename = "type")]
    pub property_type: String,
    /// Count of currently active listings of this type.
    pub count: i64,
    /// Display label localized to the request locale (resolved from the
    /// `locale` query param, then the `Accept-Language` header, falling back
    /// to English). Supported languages: sk, cs, de, en.
    pub label: String,
    /// Icon hint for the frontend; client-side `categoryIcons` lookup falls
    /// back to the property_type key.
    pub icon: String,
}

/// Row shape for the rich listing query.
#[derive(Debug, FromRow)]
struct RichListingRow {
    id: Uuid,
    title: String,
    property_type: String,
    transaction_type: String,
    status: String,
    price: i64,
    currency: String,
    size_sqm: Option<i32>,
    rooms: Option<i32>,
    bathrooms: Option<i32>,
    floor: Option<i32>,
    total_floors: Option<i32>,
    street: String,
    city: String,
    postal_code: String,
    country: String,
    latitude: Option<f64>,
    longitude: Option<f64>,
    photo_id: Option<Uuid>,
    photo_url: Option<String>,
    photo_thumbnail_url: Option<String>,
    photo_display_order: Option<i32>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    published_at: Option<DateTime<Utc>>,
}

impl From<RichListingRow> for RichListingSummary {
    fn from(r: RichListingRow) -> Self {
        let area = r.size_sqm.unwrap_or(0);
        let price_per_sqm = if area > 0 {
            Some(r.price / area as i64)
        } else {
            None
        };
        let primary_photo = match (r.photo_id, r.photo_url.clone()) {
            (Some(id), Some(url)) => Some(RichListingPhoto {
                id,
                url: url.clone(),
                // Fall back to the original URL when no thumbnail variant
                // was generated (older imports without resized variants).
                thumbnail_url: r.photo_thumbnail_url.unwrap_or(url),
                is_primary: true,
                order: r.photo_display_order.unwrap_or(0),
            }),
            _ => None,
        };
        // Derive an SEO-friendly, human-readable slug from the title with the
        // UUID appended as a stable suffix (`beautiful-apartment-<uuid>`).
        // Listings have no `slug` column, so the trailing UUID is what keeps
        // the path resolvable: `get_listing` extracts it via
        // `util::slug::parse_listing_id`, which also still accepts a bare UUID,
        // so existing deep links keep working.
        let slug = crate::util::slug::listing_slug(&r.title, r.id);
        Self {
            id: r.id,
            title: r.title,
            slug,
            property_type: r.property_type,
            transaction_type: r.transaction_type,
            status: r.status,
            price: r.price,
            currency: r.currency,
            price_per_sqm,
            area,
            rooms: r.rooms,
            // No `bedrooms` column in listings; report null.
            bedrooms: None,
            bathrooms: r.bathrooms,
            floor: r.floor,
            total_floors: r.total_floors,
            address: RichListingAddress {
                city: r.city,
                district: None,
                street: Some(r.street),
                postal_code: Some(r.postal_code),
                country: r.country,
                latitude: r.latitude,
                longitude: r.longitude,
            },
            primary_photo,
            // No `is_featured` column — surfaced via `/featured` ranking.
            is_featured: false,
            // Anonymous request — favorite state is hydrated client-side
            // from /api/v1/favorites/ids when the user is authenticated.
            is_favorite: false,
            created_at: r.created_at.to_rfc3339(),
            updated_at: r.updated_at.to_rfc3339(),
        }
    }
}

/// Shared SELECT clause for the rich listing summary query.
const RICH_LISTING_SELECT: &str = r#"
    SELECT
        l.id, l.title, l.property_type, l.transaction_type, l.status,
        l.price::bigint AS price, l.currency,
        l.size_sqm::int AS size_sqm,
        l.rooms, l.bathrooms, l.floor, l.total_floors,
        l.street, l.city, l.postal_code, l.country,
        l.latitude::float8 AS latitude,
        l.longitude::float8 AS longitude,
        p.id AS photo_id,
        p.url AS photo_url,
        p.thumbnail_url AS photo_thumbnail_url,
        p.display_order AS photo_display_order,
        l.created_at, l.updated_at, l.published_at
    FROM listings l
    LEFT JOIN LATERAL (
        SELECT id, url, thumbnail_url, display_order
        FROM listing_photos
        WHERE listing_id = l.id
        ORDER BY display_order ASC
        LIMIT 1
    ) p ON TRUE
"#;

/// Fetch the most recent N active listings filtered by transaction type.
async fn fetch_top_listings(
    state: &AppState,
    transaction_type: Option<&str>,
    limit: i32,
) -> Result<Vec<RichListingSummary>, sqlx::Error> {
    let mut conn = state.acquire_public_conn().await?;

    let sql = format!(
        "{} WHERE l.status = 'active' AND ($1::text IS NULL OR l.transaction_type = $1) \
         ORDER BY COALESCE(l.published_at, l.created_at) DESC LIMIT $2",
        RICH_LISTING_SELECT
    );

    // SAFETY (sqlx::AssertSqlSafe): `sql` is built by `format!` from two static
    // string literals only — the `RICH_LISTING_SELECT` const and the fixed
    // WHERE/ORDER/LIMIT suffix. No request input is interpolated into the query
    // text; `transaction_type` and `limit` are passed as bound parameters
    // ($1/$2). Opting out of the SQL-injection guard here is sound.
    let rows = sqlx::query_as::<_, RichListingRow>(sqlx::AssertSqlSafe(sql))
        .bind(transaction_type)
        .bind(limit)
        .fetch_all(&mut *conn)
        .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Get featured listings for the homepage.
///
/// Returns three curated buckets — `sale`, `rent`, `new` — used by the
/// reality-web homepage. Selection is "top 4 most-recently published per
/// bucket" since there's no `is_featured` column on `listings` yet.
#[utoipa::path(
    get,
    path = "/api/v1/listings/featured",
    tag = "Listings",
    responses(
        (status = 200, description = "Featured listings grouped by bucket", body = FeaturedListingsResponse)
    )
)]
pub async fn get_featured(
    State(state): State<AppState>,
) -> Result<Json<FeaturedListingsResponse>, (axum::http::StatusCode, String)> {
    // Frontend renders `slice(0, 4)` per bucket; 12 is plenty of headroom
    // and lets a curation pass be added later without changing the contract.
    const PER_BUCKET: i32 = 12;

    fn db_err(
        context: &'static str,
    ) -> impl FnOnce(sqlx::Error) -> (axum::http::StatusCode, String) {
        move |e| {
            tracing::error!(error = %e, "{context}");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            )
        }
    }

    let sale = fetch_top_listings(&state, Some("sale"), PER_BUCKET)
        .await
        .map_err(db_err("Failed to fetch featured sale listings"))?;
    let rent = fetch_top_listings(&state, Some("rent"), PER_BUCKET)
        .await
        .map_err(db_err("Failed to fetch featured rent listings"))?;
    // "New" bucket is just the most recently published, regardless of
    // transaction type. Two of these will overlap with sale/rent above on
    // small datasets — that's expected and fine for a homepage rail.
    let new = fetch_top_listings(&state, None, PER_BUCKET)
        .await
        .map_err(db_err("Failed to fetch new listings"))?;

    Ok(Json(FeaturedListingsResponse { sale, rent, new }))
}

/// Category listing query parameters.
#[derive(Debug, Default, Deserialize, IntoParams)]
pub struct CategoriesQuery {
    /// Preferred display language for category labels (`sk` | `cs` | `de` |
    /// `en`). When omitted, the `Accept-Language` header is consulted; if that
    /// is also absent or unrecognized, labels default to English.
    pub locale: Option<String>,
}

/// Get listing counts grouped by property type.
///
/// Drives the homepage "Browse by category" tiles. Only counts listings with
/// `status = 'active'`. Labels are localized to the resolved request locale
/// (`locale` query param > `Accept-Language` header > English fallback); the
/// frontend has its own i18n fallback if a label is missing.
#[utoipa::path(
    get,
    path = "/api/v1/listings/categories",
    tag = "Listings",
    params(CategoriesQuery),
    responses(
        (status = 200, description = "Listing counts by property type", body = [CategoryCount])
    )
)]
pub async fn get_categories(
    State(state): State<AppState>,
    Query(query): Query<CategoriesQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<CategoryCount>>, (axum::http::StatusCode, String)> {
    let locale = resolve_locale(query.locale.as_deref(), &headers);
    let mut conn = state.acquire_public_conn().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to acquire db connection");
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        )
    })?;

    let rows: Vec<(String, i64)> = sqlx::query_as(
        r#"
        SELECT property_type, COUNT(*)::bigint AS count
        FROM listings
        WHERE status = 'active'
        GROUP BY property_type
        ORDER BY count DESC
        "#,
    )
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to count listings by property_type");
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        )
    })?;

    let result = rows
        .into_iter()
        .map(|(property_type, count)| {
            let label = label_for_property_type(&property_type, &locale);
            CategoryCount {
                icon: property_type.clone(),
                property_type,
                count,
                label,
            }
        })
        .collect();

    Ok(Json(result))
}

/// Resolve the display locale for category labels.
///
/// Precedence: explicit `locale` query param, then the first tag of the
/// `Accept-Language` header, then the [`Locale`] default (English). Both inputs
/// are normalized through [`Locale::parse`], which falls back to English for an
/// unrecognized value — so an unsupported language never leaks through.
fn resolve_locale(query_locale: Option<&str>, headers: &HeaderMap) -> Locale {
    if let Some(loc) = query_locale.map(str::trim).filter(|s| !s.is_empty()) {
        return Locale::parse(loc);
    }
    if let Some(tag) = headers
        .get(axum::http::header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .and_then(first_language_tag)
    {
        return Locale::parse(tag);
    }
    Locale::default()
}

/// Extract the first (highest-priority) language tag from an `Accept-Language`
/// header value, dropping any `;q=` weight. Returns `None` for an empty header.
fn first_language_tag(header: &str) -> Option<&str> {
    header
        .split(',')
        .next()
        .map(|tag| tag.split(';').next().unwrap_or(tag).trim())
        .filter(|tag| !tag.is_empty())
}

/// Localized label for a `property_type` enum value, in the given [`Locale`].
fn label_for_property_type(t: &str, locale: &Locale) -> String {
    let label = match (t, locale) {
        ("apartment", Locale::Slovak) => "Byty",
        ("apartment", Locale::Czech) => "Byty",
        ("apartment", Locale::German) => "Wohnungen",
        ("apartment", Locale::English) => "Apartments",
        ("house", Locale::Slovak) => "Domy",
        ("house", Locale::Czech) => "Domy",
        ("house", Locale::German) => "Häuser",
        ("house", Locale::English) => "Houses",
        ("land", Locale::Slovak) => "Pozemky",
        ("land", Locale::Czech) => "Pozemky",
        ("land", Locale::German) => "Grundstücke",
        ("land", Locale::English) => "Land",
        ("commercial", Locale::Slovak) => "Komerčné priestory",
        ("commercial", Locale::Czech) => "Komerční prostory",
        ("commercial", Locale::German) => "Gewerbeflächen",
        ("commercial", Locale::English) => "Commercial",
        ("office", Locale::Slovak) => "Kancelárie",
        ("office", Locale::Czech) => "Kanceláře",
        ("office", Locale::German) => "Büros",
        ("office", Locale::English) => "Offices",
        ("garage", Locale::Slovak) => "Garáže",
        ("garage", Locale::Czech) => "Garáže",
        ("garage", Locale::German) => "Garagen",
        ("garage", Locale::English) => "Garages",
        ("parking", Locale::Slovak) => "Parkovanie",
        ("parking", Locale::Czech) => "Parkování",
        ("parking", Locale::German) => "Parkplätze",
        ("parking", Locale::English) => "Parking",
        ("storage", Locale::Slovak) => "Sklady",
        ("storage", Locale::Czech) => "Sklady",
        ("storage", Locale::German) => "Lager",
        ("storage", Locale::English) => "Storage",
        // Fallback: capitalize the type so unknown values still render.
        (other, _) => return capitalize(other),
    };
    label.to_string()
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Record a listing view (analytics).
///
/// POST `/api/v1/listings/{id}/view`. Increments the view counter for the
/// listing. Anonymous endpoint — no auth required so SSR pages can fire it.
#[utoipa::path(
    post,
    path = "/api/v1/listings/{id}/view",
    tag = "Listings",
    params(("id" = String, Path, description = "Listing slug (`title-<uuid>`) or bare UUID")),
    responses(
        (status = 204, description = "View recorded"),
        (status = 404, description = "Listing not found")
    )
)]
pub async fn record_view(
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    // Accept both the SEO slug (`title-<uuid>`) and a bare UUID, matching the
    // detail handler so the view beacon fires from the same path segment.
    let id = crate::util::slug::parse_listing_id(&id_or_slug).ok_or((
        axum::http::StatusCode::NOT_FOUND,
        "Listing not found".to_string(),
    ))?;
    // `track_view` is best-effort — analytics shouldn't break the page on
    // a transient db hiccup. Log and return 204 either way; only return a
    // hard error when the listing genuinely doesn't exist.
    if let Err(e) = state.reality_portal_repo.track_view(id, "website").await {
        tracing::warn!(%id, error = %e, "Failed to track listing view (non-fatal)");
    }
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Regression for #953: pagination params must be clamped before they reach
    // SQL. A negative `limit` previously surfaced a raw DB error
    // ("LIMIT must not be negative") as a 500; an unbounded limit/page was a
    // latent resource-exhaustion vector.

    #[test]
    fn defaults_when_unset() {
        assert_eq!(
            clamp_pagination(None, None),
            (1, DEFAULT_LISTINGS_LIMIT),
            "missing params should default to page 1 and the default limit"
        );
    }

    #[test]
    fn negative_limit_is_floored_to_one() {
        let (_, limit) = clamp_pagination(None, Some(-1));
        assert_eq!(limit, 1, "negative limit must never reach SQL");
    }

    #[test]
    fn zero_limit_is_floored_to_one() {
        let (_, limit) = clamp_pagination(None, Some(0));
        assert_eq!(limit, 1, "zero limit must be raised to the minimum of 1");
    }

    #[test]
    fn oversized_limit_is_capped() {
        let (_, limit) = clamp_pagination(None, Some(100_000));
        assert_eq!(limit, MAX_LISTINGS_LIMIT, "limit must be capped at the max");
    }

    #[test]
    fn negative_page_is_floored_to_one() {
        let (page, _) = clamp_pagination(Some(-5), None);
        assert_eq!(page, 1, "negative page must be raised to 1");
    }

    #[test]
    fn valid_values_pass_through() {
        assert_eq!(clamp_pagination(Some(3), Some(50)), (3, 50));
    }

    // --- ListingSummary SEO slug (search `/listings` path) ---
    //
    // Regression: the search-results `ListingSummary` previously carried only a
    // bare UUID `id` and no `slug`, so reality-web had to fall back to the raw
    // UUID when building listing links — while `/featured` (`RichListingSummary`)
    // already emitted a title-prefixed SEO slug. These assert the search path
    // now populates the same `{slugified-title}-{uuid}` slug. They fail against
    // the old struct, which had no `slug` field to populate.

    fn public_summary(id: Uuid, title: &str) -> PublicListingSummary {
        PublicListingSummary {
            id,
            title: title.to_string(),
            description: None,
            price: 100_000,
            currency: "EUR".to_string(),
            size_sqm: Some(60),
            rooms: Some(3),
            city: "Bratislava".to_string(),
            property_type: "apartment".to_string(),
            transaction_type: "sale".to_string(),
            photo_url: None,
            published_at: Utc::now(),
        }
    }

    #[test]
    fn summary_slug_is_title_prefixed_not_bare_uuid() {
        let id = Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap();
        let summary: ListingSummary = public_summary(id, "Beautiful Apartment").into();
        assert_eq!(
            summary.slug, "beautiful-apartment-11111111-2222-3333-4444-555555555555",
            "search results must emit the SEO slug, not a bare UUID"
        );
        assert_ne!(
            summary.slug,
            id.to_string(),
            "a title-bearing listing must not degrade to a bare-UUID slug"
        );
    }

    #[test]
    fn summary_slug_matches_featured_slug_for_same_listing() {
        // The search path and `/featured` must agree on the slug for a given
        // (title, id) so a listing resolves to the same canonical URL whichever
        // surface linked to it.
        let id = Uuid::parse_str("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee").unwrap();
        let title = "3-izbový byt, Bratislava!";
        let summary: ListingSummary = public_summary(id, title).into();
        assert_eq!(summary.slug, crate::util::slug::listing_slug(title, id));
    }

    #[test]
    fn summary_slug_degrades_to_bare_uuid_when_title_empty() {
        let id = Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap();
        let summary: ListingSummary = public_summary(id, "---").into();
        assert_eq!(
            summary.slug,
            id.to_string(),
            "a title that slugifies to nothing must still yield a resolvable path"
        );
    }

    // --- popular_search_phrase (popular-search suggestions) ---
    //
    // Placed here, among the clamp-helper tests, rather than at the end of the
    // module: the localization tests appended on `dev` land at the tail, so
    // keeping these away from that boundary avoids a spurious add/add merge
    // conflict when the branch is brought up to date.

    #[test]
    fn popular_search_phrase_uses_localized_label_and_city() {
        assert_eq!(
            popular_search_phrase("apartment", "Bratislava", &Locale::Slovak),
            "Byty Bratislava"
        );
        assert_eq!(
            popular_search_phrase("house", "Košice", &Locale::Slovak),
            "Domy Košice"
        );
        assert_eq!(
            popular_search_phrase("land", "Žilina", &Locale::Slovak),
            "Pozemky Žilina"
        );
    }

    #[test]
    fn popular_search_phrase_falls_back_to_capitalized_type_for_unknown() {
        // Unknown/future enum values must still yield a usable phrase rather
        // than an empty label (mirrors label_for_property_type's fallback).
        assert_eq!(
            popular_search_phrase("villa", "Nitra", &Locale::Slovak),
            "Villa Nitra"
        );
    }

    // --- Wiring regression for #953 / #959 ---
    //
    // The pure `clamp_pagination` tests above only prove the helper is correct.
    // They would still pass if someone dropped the override that copies the
    // clamped values into the `PublicListingQuery` that actually reaches SQL —
    // re-opening the original 500 / resource-exhaustion bug. These tests assert
    // the end-to-end clamp-then-wire step (`build_listing_query`), i.e. that the
    // *query sent to the DB* carries the clamped values, not the raw request.

    /// A search request with everything unset except the fields under test.
    fn req_with_pagination(page: Option<i32>, limit: Option<i32>) -> ListingSearchRequest {
        ListingSearchRequest {
            q: None,
            property_type: None,
            transaction_type: None,
            price_min: None,
            price_max: None,
            area_min: None,
            area_max: None,
            rooms_min: None,
            rooms_max: None,
            city: None,
            country: None,
            page,
            limit,
            sort: None,
        }
    }

    #[test]
    fn query_for_negative_limit_never_carries_negative_to_sql() {
        let (query, _, echoed_limit) = build_listing_query(req_with_pagination(None, Some(-1)));
        assert_eq!(
            query.limit,
            Some(1),
            "negative limit must be clamped in the query handed to SQL"
        );
        assert_eq!(echoed_limit, 1, "response must echo the clamped limit");
    }

    #[test]
    fn query_for_oversized_limit_is_capped_in_sql_query() {
        let (query, _, echoed_limit) =
            build_listing_query(req_with_pagination(None, Some(100_000)));
        assert_eq!(
            query.limit,
            Some(MAX_LISTINGS_LIMIT),
            "oversized limit must be capped in the query handed to SQL"
        );
        assert_eq!(echoed_limit, MAX_LISTINGS_LIMIT);
    }

    #[test]
    fn query_for_negative_page_never_carries_negative_to_sql() {
        let (query, echoed_page, _) = build_listing_query(req_with_pagination(Some(-5), None));
        assert_eq!(
            query.page,
            Some(1),
            "negative page must be clamped in the query handed to SQL"
        );
        assert_eq!(echoed_page, 1, "response must echo the clamped page");
    }

    #[test]
    fn query_preserves_non_pagination_filters() {
        let mut req = req_with_pagination(Some(2), Some(25));
        req.city = Some("Bratislava".to_string());
        req.transaction_type = Some("rent".to_string());
        let (query, page, limit) = build_listing_query(req);
        assert_eq!((page, limit), (2, 25), "valid pagination passes through");
        assert_eq!(query.city.as_deref(), Some("Bratislava"));
        assert_eq!(query.transaction_type.as_deref(), Some("rent"));
    }

    // --- Category label localization (de-hardcoding of sk labels) ---
    //
    // Regression: `label_for_property_type` previously returned Slovak strings
    // unconditionally, ignoring the request locale. These tests assert every
    // supported language renders its own label — they fail against the old
    // sk-only implementation.

    fn accept_language(value: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(
            axum::http::header::ACCEPT_LANGUAGE,
            value.parse().expect("valid header value"),
        );
        h
    }

    #[test]
    fn labels_are_localized_per_locale() {
        assert_eq!(
            label_for_property_type("apartment", &Locale::Slovak),
            "Byty"
        );
        assert_eq!(
            label_for_property_type("apartment", &Locale::German),
            "Wohnungen"
        );
        assert_eq!(
            label_for_property_type("apartment", &Locale::English),
            "Apartments"
        );
        assert_eq!(
            label_for_property_type("commercial", &Locale::Czech),
            "Komerční prostory"
        );
        assert_eq!(
            label_for_property_type("office", &Locale::English),
            "Offices"
        );
    }

    #[test]
    fn unknown_property_type_falls_back_to_capitalized_key() {
        // Fallback is locale-independent: capitalize the raw key.
        assert_eq!(
            label_for_property_type("penthouse", &Locale::German),
            "Penthouse"
        );
        assert_eq!(
            label_for_property_type("penthouse", &Locale::Slovak),
            "Penthouse"
        );
    }

    #[test]
    fn locale_query_param_takes_precedence_over_header() {
        let headers = accept_language("sk");
        assert_eq!(
            resolve_locale(Some("de"), &headers),
            Locale::German,
            "explicit query param must win over Accept-Language"
        );
    }

    #[test]
    fn locale_falls_back_to_accept_language_header() {
        let headers = accept_language("cs-CZ,cs;q=0.9,en;q=0.8");
        assert_eq!(
            resolve_locale(None, &headers),
            Locale::Czech,
            "first Accept-Language tag (minus weight) should resolve"
        );
    }

    #[test]
    fn locale_defaults_to_english_when_no_signal() {
        assert_eq!(
            resolve_locale(None, &HeaderMap::new()),
            Locale::English,
            "no query param and no header should default to English"
        );
        // Blank query param is ignored, not parsed to English-via-unknown.
        assert_eq!(
            resolve_locale(Some("  "), &accept_language("sk")),
            Locale::Slovak,
            "blank query param must defer to the header"
        );
    }

    #[test]
    fn first_language_tag_strips_quality_weight() {
        assert_eq!(first_language_tag("de-DE,de;q=0.9"), Some("de-DE"));
        assert_eq!(first_language_tag("sk"), Some("sk"));
        assert_eq!(first_language_tag(""), None);
    }

    // --- region resolution for suggestions seed city (derive, don't hardcode sk) ---

    #[test]
    fn region_query_param_takes_precedence_and_is_normalized() {
        // Explicit region wins over the env fallback and is lower-cased/trimmed.
        assert_eq!(resolve_region(Some("CZ")), "cz");
        assert_eq!(resolve_region(Some("  de ")), "de");
    }

    #[test]
    fn region_defaults_to_sk_when_param_blank_and_env_unset() {
        // Blank/whitespace param is ignored; with REGION unset the default is
        // the Slovak origin deployment, preserving historical behavior.
        // (Run in a process where REGION is not exported.)
        if std::env::var_os("REGION").is_none() {
            assert_eq!(resolve_region(None), DEFAULT_REGION);
            assert_eq!(resolve_region(Some("   ")), DEFAULT_REGION);
        }
    }

    #[test]
    fn default_city_follows_region_and_falls_back_to_bratislava() {
        // The previously hardcoded seed city is now region-derived; sk and any
        // unknown/EU/local region still map to Bratislava (existing default).
        assert_eq!(default_city_for_region("sk"), "Bratislava");
        assert_eq!(default_city_for_region("cz"), "Praha");
        assert_eq!(default_city_for_region("de"), "Wien");
        assert_eq!(default_city_for_region("eu"), "Bratislava");
        assert_eq!(default_city_for_region("local"), "Bratislava");
        assert_eq!(default_city_for_region("zz-unknown"), "Bratislava");
    }
}
