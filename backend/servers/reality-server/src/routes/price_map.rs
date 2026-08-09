//! Price map routes (UC-31: Price Map Page).
//!
//! Returns city-level price aggregations from the listings table.
//! Results are cached in-process for 1 hour. The cache uses
//! `tokio::sync::RwLock` (NOT `std::sync::RwLock`) so a contention spike
//! never blocks the Tokio worker thread the request is running on.
//! No new migration needed (read-only aggregation over existing listings table).

use crate::state::AppState;
use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use utoipa::{IntoParams, ToSchema};

/// Price map cache entry.
#[derive(Clone, Debug)]
struct CacheEntry {
    data: Vec<DistrictPriceData>,
    expires_at: Instant,
}

/// Global price map cache (key = serialised query parameters).
static PRICE_MAP_CACHE: std::sync::OnceLock<Arc<RwLock<HashMap<String, CacheEntry>>>> =
    std::sync::OnceLock::new();

fn price_map_cache() -> &'static Arc<RwLock<HashMap<String, CacheEntry>>> {
    PRICE_MAP_CACHE.get_or_init(|| Arc::new(RwLock::new(HashMap::new())))
}

/// Cache TTL: 1 hour.
const CACHE_TTL: Duration = Duration::from_secs(3600);

/// Create price map router.
pub fn router() -> Router<AppState> {
    Router::new().route("/", get(get_price_map))
}

/// Price map query parameters.
#[derive(Debug, Deserialize, IntoParams)]
pub struct PriceMapQuery {
    /// Filter by city name (case-insensitive, optional).
    pub city: Option<String>,
    /// Filter by property type (apartment, house, land, commercial, …).
    pub property_type: Option<String>,
    /// sale or rent.
    pub mode: Option<String>,
    /// Time window: 1m | 3m | 12m | 5y (default 3m).
    pub time_window: Option<String>,
}

/// District/city price data point.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DistrictPriceData {
    /// City used as district grouping key.
    pub district_id: String,
    pub district_name: String,
    pub avg_price_per_m2: Option<f64>,
    pub listing_count: i64,
    /// Quarter-on-quarter trend percentage (positive = rising).
    pub trend_pct_qoq: Option<f64>,
}

/// Price map response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PriceMapResponse {
    pub districts: Vec<DistrictPriceData>,
    pub mode: String,
    pub property_type: Option<String>,
    pub city: Option<String>,
    pub time_window: String,
    pub cached: bool,
}

/// Get city-level price aggregations.
///
/// Aggregates avg price/m² from the `listings` table grouped by city.
/// Cached server-side for 1 hour per unique query combination.
#[utoipa::path(
    get,
    path = "/api/v1/price-map",
    tag = "PriceMap",
    params(PriceMapQuery),
    responses(
        (status = 200, description = "City price aggregations", body = PriceMapResponse)
    )
)]
pub async fn get_price_map(
    State(state): State<AppState>,
    Query(query): Query<PriceMapQuery>,
) -> Result<Json<PriceMapResponse>, (axum::http::StatusCode, String)> {
    let mode = query.mode.clone().unwrap_or_else(|| "sale".to_string());
    // Validate time_window — reject unsupported values up-front rather than
    // silently falling back. Otherwise the response echoed back an invalid
    // time_window even though the data was computed for a different interval.
    let time_window_str = query
        .time_window
        .clone()
        .unwrap_or_else(|| "3m".to_string());
    let interval = match time_window_str.as_str() {
        "1m" => "1 month",
        "3m" => "3 months",
        "12m" => "12 months",
        "5y" => "5 years",
        other => {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                format!(
                    "Unsupported time_window {:?} — expected one of 1m, 3m, 12m, 5y",
                    other
                ),
            ));
        }
    };

    let cache_key = format!(
        "{}:{}:{}:{}",
        mode,
        time_window_str,
        query.property_type.as_deref().unwrap_or(""),
        query.city.as_deref().unwrap_or("")
    );

    // Check cache (async read lock — never blocks Tokio worker)
    {
        let cache = price_map_cache().read().await;
        if let Some(entry) = cache.get(&cache_key) {
            if Instant::now() < entry.expires_at {
                return Ok(Json(PriceMapResponse {
                    districts: entry.data.clone(),
                    mode,
                    property_type: query.property_type,
                    city: query.city,
                    time_window: time_window_str,
                    cached: true,
                }));
            }
        }
    }

    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| crate::util::errors::db_error("acquire db connection", e))?;

    // Aggregate current-period avg price/m² by city.
    // size_sqm is the listings column; price/size_sqm gives €/m².
    // QoQ trend compares this period to the prior equal-length period.
    let sql = format!(
        r#"
        WITH current_period AS (
            SELECT
                city AS district_id,
                city AS district_name,
                AVG(CASE WHEN size_sqm > 0 THEN price / size_sqm END) AS avg_ppm2,
                COUNT(*) AS listing_count
            FROM listings
            WHERE transaction_type = $1
              AND ($2::text IS NULL OR property_type = $2)
              AND ($3::text IS NULL OR city ILIKE $3)
              AND created_at >= NOW() - INTERVAL '{interval}'
              AND status IN ('active', 'sold', 'rented')
            GROUP BY city
        ),
        prior_period AS (
            SELECT
                city AS district_id,
                AVG(CASE WHEN size_sqm > 0 THEN price / size_sqm END) AS avg_ppm2
            FROM listings
            WHERE transaction_type = $1
              AND ($2::text IS NULL OR property_type = $2)
              AND ($3::text IS NULL OR city ILIKE $3)
              AND created_at >= NOW() - (INTERVAL '{interval}' * 2)
              AND created_at <  NOW() - INTERVAL '{interval}'
              AND status IN ('active', 'sold', 'rented')
            GROUP BY city
        )
        SELECT
            c.district_id,
            c.district_name,
            c.avg_ppm2::float8 AS avg_price_per_m2,
            c.listing_count,
            CASE
                WHEN p.avg_ppm2 IS NOT NULL AND p.avg_ppm2 > 0
                THEN ROUND(((c.avg_ppm2 - p.avg_ppm2) / p.avg_ppm2 * 100)::numeric, 2)::float8
                ELSE NULL
            END AS trend_pct_qoq
        FROM current_period c
        LEFT JOIN prior_period p ON p.district_id = c.district_id
        ORDER BY c.listing_count DESC
        "#,
        interval = interval,
    );

    let city_pattern = query.city.as_ref().map(|c| format!("%{}%", c));

    let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(&mode)
        .bind(&query.property_type)
        .bind(&city_pattern)
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| crate::util::errors::db_error("aggregate price map data", e))?;

    let districts: Vec<DistrictPriceData> = rows
        .into_iter()
        .map(|r| DistrictPriceData {
            district_id: r.get::<String, _>("district_id"),
            district_name: r.get::<String, _>("district_name"),
            avg_price_per_m2: r.get("avg_price_per_m2"),
            listing_count: r.get::<i64, _>("listing_count"),
            trend_pct_qoq: r.get("trend_pct_qoq"),
        })
        .collect();

    // Store in cache (async write lock — never blocks Tokio worker)
    {
        let mut cache = price_map_cache().write().await;
        // Evict expired entries to avoid unbounded growth
        cache.retain(|_, v| Instant::now() < v.expires_at);
        cache.insert(
            cache_key,
            CacheEntry {
                data: districts.clone(),
                expires_at: Instant::now() + CACHE_TTL,
            },
        );
    }

    Ok(Json(PriceMapResponse {
        districts,
        mode,
        property_type: query.property_type,
        city: query.city,
        time_window: time_window_str,
        cached: false,
    }))
}
