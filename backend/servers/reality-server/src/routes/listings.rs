//! Public listing routes - search and view (Story 16.1).

use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use db::models::{PublicListingQuery, PublicListingSummary};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Create listings router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(search))
        .route("/{id}", get(get_listing))
        .route("/suggestions", get(get_suggestions))
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
        Self {
            id: summary.id,
            title: summary.title,
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
    let page = req.page.unwrap_or(1);
    let limit = req.limit.unwrap_or(20);
    let query: PublicListingQuery = req.into();

    // Search listings
    let listings = state
        .portal_repo
        .search_listings(&query)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to search listings: {}", e),
            )
        })?;

    // Count total
    let total = state
        .portal_repo
        .count_listings(&query)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to count listings: {}", e),
            )
        })?;

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
        ("id" = Uuid, Path, description = "Listing ID")
    ),
    responses(
        (status = 200, description = "Listing detail", body = ListingDetail),
        (status = 404, description = "Listing not found")
    )
)]
pub async fn get_listing(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ListingDetail>, (axum::http::StatusCode, String)> {
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
            l.id, l.title, l.description, l.price, l.currency,
            l.size_sqm, l.rooms, l.bathrooms, l.floor, l.total_floors,
            l.street, l.city, l.postal_code, l.country,
            l.latitude, l.longitude,
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
                view_count: 0, // Would need analytics query
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
) -> Result<Json<SuggestionsResponse>, (axum::http::StatusCode, String)> {
    let city = params
        .get("city")
        .map(|s| s.as_str())
        .unwrap_or("Bratislava");

    let cities = state
        .portal_repo
        .get_nearby_cities(city, 10)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get cities: {}", e),
            )
        })?;

    Ok(Json(SuggestionsResponse {
        cities,
        popular_searches: vec![
            "2-izbový byt Bratislava".to_string(),
            "Dom Košice".to_string(),
            "Pozemok Žilina".to_string(),
        ],
    }))
}
