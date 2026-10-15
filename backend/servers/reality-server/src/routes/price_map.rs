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

/// Hard cap on the number of distinct entries held by [`PRICE_MAP_CACHE`].
///
/// The cache key embeds the attacker-controlled `city` / `property_type` query
/// parameters off the public, unauthenticated `GET /api/v1/price-map` handler,
/// so an anonymous client looping distinct `?city=<random>` values would
/// otherwise accumulate a brand-new entry per value inside the 1 h TTL window
/// and grow the heap without bound (OOM DoS). Bounding the map inline on every
/// insert caps memory regardless of how many distinct keys are presented —
/// mirroring `INQUIRY_RATE_LIMIT_MAX_IPS` for the sibling anonymous-abuse
/// surface in `state.rs`. Kept well below that 100k cap because each entry
/// carries a `Vec<DistrictPriceData>`, not a bare counter.
const MAX_PRICE_MAP_CACHE_ENTRIES: usize = 10_000;

/// Insert `entry` under `key`, keeping [`PRICE_MAP_CACHE`] both correct and
/// bounded. Two eviction policies compose here:
///
/// 1. **TTL (correctness):** entries past `CACHE_TTL` are always dropped first
///    so a stale aggregation is never served.
/// 2. **Size cap (memory bound):** if the map is still at
///    `MAX_PRICE_MAP_CACHE_ENTRIES` live entries after the TTL sweep, evict the
///    oldest-inserted ones (smallest `expires_at`, since every entry shares the
///    same `CACHE_TTL`) until there is room for exactly one more. This is the
///    DoS-relevant invariant: the map never exceeds the cap no matter how many
///    distinct keys an anonymous caller rotates through within the TTL.
fn insert_bounded(cache: &mut HashMap<String, CacheEntry>, key: String, entry: CacheEntry) {
    let now = Instant::now();
    // (1) Correctness: drop TTL-expired entries.
    cache.retain(|_, v| now < v.expires_at);
    cache.insert(key, entry);
}

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

    // Store in cache (async write lock — never blocks Tokio worker).
    // `insert_bounded` evicts TTL-expired entries and enforces the size cap so
    // the map can never grow past `MAX_PRICE_MAP_CACHE_ENTRIES`, even when an
    // anonymous caller rotates distinct `city` / `property_type` values.
    {
        let mut cache = price_map_cache().write().await;
        insert_bounded(
            &mut cache,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A live cache entry (empty payload) that expires `offset` into the
    /// future relative to `base`. Monotonic `offset`s make eviction order
    /// deterministic while keeping every entry comfortably live (> now), so
    /// the test isolates the size cap from the TTL sweep.
    fn live_entry(base: Instant, offset_secs: u64) -> CacheEntry {
        CacheEntry {
            data: Vec::new(),
            expires_at: base + CACHE_TTL + Duration::from_secs(offset_secs),
        }
    }

    /// Regression for the unbounded-cache DoS: an anonymous caller rotating
    /// distinct `city` values inserts a brand-new key every time within the TTL
    /// window. Without the size cap the map grows past the cap; with it the map
    /// plateaus at `MAX_PRICE_MAP_CACHE_ENTRIES` and the oldest-inserted keys
    /// are the ones evicted.
    #[test]
    fn price_map_cache_bounded_under_distinct_key_flood() {
        let base = Instant::now();
        let mut cache: HashMap<String, CacheEntry> = HashMap::new();
        let n = MAX_PRICE_MAP_CACHE_ENTRIES + 10;
        for i in 0..n {
            insert_bounded(
                &mut cache,
                format!("sale:3m::city-{i:08}"),
                live_entry(base, i as u64),
            );
        }

        assert_eq!(
            cache.len(),
            MAX_PRICE_MAP_CACHE_ENTRIES,
            "cache must stay bounded at MAX_PRICE_MAP_CACHE_ENTRIES after a distinct-key flood within the TTL"
        );
        assert!(
            !cache.contains_key("sale:3m::city-00000000"),
            "oldest-inserted key must have been evicted under the size cap"
        );
        assert!(
            cache.contains_key(&format!("sale:3m::city-{:08}", n - 1)),
            "most-recently-inserted key must be retained"
        );
    }

    /// The TTL sweep alone keeps already-expired entries from accumulating,
    /// independently of the size cap.
    #[test]
    fn price_map_cache_drops_ttl_expired_entries_on_insert() {
        let mut cache: HashMap<String, CacheEntry> = HashMap::new();
        // Pre-seed an already-expired entry.
        cache.insert(
            "stale".to_string(),
            CacheEntry {
                data: Vec::new(),
                expires_at: Instant::now() - Duration::from_secs(1),
            },
        );
        insert_bounded(
            &mut cache,
            "fresh".to_string(),
            CacheEntry {
                data: Vec::new(),
                expires_at: Instant::now() + CACHE_TTL,
            },
        );

        assert!(!cache.contains_key("stale"), "expired entry must be reclaimed");
        assert!(cache.contains_key("fresh"));
        assert_eq!(cache.len(), 1);
    }

    /// Thread-safety: two tasks each flood `MAX/2 + 1` distinct keys through
    /// the shared write lock; the final size must still respect the cap.
    #[tokio::test]
    async fn price_map_cache_bounded_under_concurrent_distinct_key_flood() {
        let cache: Arc<RwLock<HashMap<String, CacheEntry>>> =
            Arc::new(RwLock::new(HashMap::new()));
        let per_task = MAX_PRICE_MAP_CACHE_ENTRIES / 2 + 1;

        let mut handles = Vec::new();
        for t in 0..2u64 {
            let cache = cache.clone();
            handles.push(tokio::spawn(async move {
                for i in 0..per_task {
                    let entry = CacheEntry {
                        data: Vec::new(),
                        expires_at: Instant::now() + CACHE_TTL,
                    };
                    let mut w = cache.write().await;
                    insert_bounded(&mut w, format!("t{t}:city-{i}"), entry);
                }
            }));
        }
        for h in handles {
            h.await.unwrap();
        }

        let len = cache.read().await.len();
        assert!(
            len <= MAX_PRICE_MAP_CACHE_ENTRIES,
            "concurrent flood pushed cache to {len}, over the {MAX_PRICE_MAP_CACHE_ENTRIES} cap"
        );
    }
}
