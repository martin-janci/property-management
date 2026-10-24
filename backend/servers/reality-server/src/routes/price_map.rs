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
    collections::{BTreeMap, HashMap},
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
    /// Monotonic insertion sequence — the eviction-order key into
    /// [`BoundedCache::order`]. Assigned by [`BoundedCache::insert`]; callers
    /// never set it.
    seq: u64,
}

/// Global price map cache (key = serialised query parameters).
static PRICE_MAP_CACHE: std::sync::OnceLock<Arc<RwLock<BoundedCache>>> = std::sync::OnceLock::new();

fn price_map_cache() -> &'static Arc<RwLock<BoundedCache>> {
    PRICE_MAP_CACHE.get_or_init(|| Arc::new(RwLock::new(BoundedCache::new())))
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

/// Bounded, insertion-ordered price-map cache.
///
/// A lookup `HashMap` paired with a monotonic insertion-order index
/// (`BTreeMap<seq, key>`). Both eviction policies run **without cloning the
/// keyset or sorting on every insert** — the fix for the "eviction amplifies
/// the flood" DoS follow-up (#3132, on PR #3092):
///
/// 1. **TTL (correctness):** expired entries are reclaimed by popping the front
///    of the order index while it is expired — amortised O(1) per reclaimed
///    entry instead of an O(n) `retain` scan on every insert. A stale
///    aggregation is never served (the read path re-checks `expires_at` too).
/// 2. **Size cap (memory bound):** a genuinely new key that would exceed
///    `MAX_PRICE_MAP_CACHE_ENTRIES` evicts the single oldest-inserted entry
///    (front of the index) — O(log n), no full-keyset clone, no O(n log n)
///    sort. Under the distinct-key flood this defends against, each at-capacity
///    insert is now one BTree pop + one map insert while holding the write
///    lock, rather than cloning and sorting ~10k keys. OOM stays fixed and the
///    CPU/lock-contention amplifier is gone.
///
/// **Invariant:** `expires_at` is monotonic non-decreasing in `seq` (every
/// entry shares the constant `CACHE_TTL`, the `Instant` clock is monotonic, and
/// a replaced key is re-sequenced to the back with a fresh `expires_at`), so
/// the front of `order` is always the oldest-inserted and soonest-to-expire
/// live entry — making the FIFO order exact for both policies.
#[derive(Default)]
struct BoundedCache {
    /// key -> entry (the entry carries its own `seq`).
    entries: HashMap<String, CacheEntry>,
    /// insertion sequence -> key, ascending (front == oldest-inserted).
    order: BTreeMap<u64, String>,
    /// Next insertion sequence to hand out (strictly monotonic).
    next_seq: u64,
}

impl BoundedCache {
    fn new() -> Self {
        Self::default()
    }

    /// Look up an entry by key. The caller re-checks `expires_at` before
    /// serving, so a not-yet-reclaimed stale entry is never returned as live.
    fn get(&self, key: &str) -> Option<&CacheEntry> {
        self.entries.get(key)
    }

    /// Evict the oldest-inserted (front) entry, keeping `entries` and `order`
    /// in sync. Returns the evicted key, or `None` when empty.
    fn evict_oldest(&mut self) -> Option<String> {
        let (seq, key) = self.order.first_key_value().map(|(&s, k)| (s, k.clone()))?;
        self.order.remove(&seq);
        self.entries.remove(&key);
        Some(key)
    }

    /// Insert `data` under `key`, keeping the cache correct and bounded.
    fn insert(&mut self, key: String, data: Vec<DistrictPriceData>, expires_at: Instant) {
        let now = Instant::now();
        // (1) Correctness: reclaim TTL-expired entries from the front (oldest
        // first). Stops at the first live entry — O(1) under the flood, where
        // nothing has expired yet, instead of an O(n) scan of the whole map.
        while let Some((seq, front_key)) =
            self.order.first_key_value().map(|(&s, k)| (s, k.clone()))
        {
            let expired = self
                .entries
                .get(&front_key)
                .map(|e| e.expires_at <= now)
                .unwrap_or(true);
            if !expired {
                break;
            }
            self.order.remove(&seq);
            self.entries.remove(&front_key);
        }
        // (2) Replacing an existing key must not grow the map: drop its old
        // order slot so the refreshed entry re-sequences to the back (youngest)
        // below, and the size-cap check sees the correct post-removal length.
        if let Some(prev) = self.entries.remove(&key) {
            self.order.remove(&prev.seq);
        }
        // (3) Memory bound: evict the oldest-inserted entry until there is room
        // for exactly one more. Normally runs zero or one time (the map grows
        // one key at a time), so no sort or full-keyset clone is ever needed.
        while self.entries.len() >= MAX_PRICE_MAP_CACHE_ENTRIES {
            if self.evict_oldest().is_none() {
                break;
            }
        }
        // (4) Insert at the back with a fresh monotonic sequence.
        let seq = self.next_seq;
        self.next_seq += 1;
        self.order.insert(seq, key.clone());
        self.entries.insert(
            key,
            CacheEntry {
                data,
                expires_at,
                seq,
            },
        );
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }

    #[cfg(test)]
    fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }
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

    // SAFETY (sqlx::AssertSqlSafe): the only value interpolated into `sql` via
    // `format!` is `interval`, a `&'static str` chosen from the closed match on
    // `time_window_str` above (any other value already returned 400). It is
    // never derived from request input. All request-controlled values — mode,
    // property_type and the city pattern — are passed as bound parameters
    // ($1/$2/$3), so no untrusted data reaches the query text. Opting out of the
    // SQL-injection guard here is sound.
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
    // `BoundedCache::insert` reclaims TTL-expired entries and enforces the size
    // cap via an insertion-order index, so the map can never grow past
    // `MAX_PRICE_MAP_CACHE_ENTRIES` — and each insert stays O(log n) rather than
    // cloning + sorting the whole keyset — even when an anonymous caller rotates
    // distinct `city` / `property_type` values.
    {
        let mut cache = price_map_cache().write().await;
        cache.insert(cache_key, districts.clone(), Instant::now() + CACHE_TTL);
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

    /// Regression for the unbounded-cache DoS: an anonymous caller rotating
    /// distinct `city` values inserts a brand-new key every time within the TTL
    /// window. Without the size cap the map grows past the cap; with it the map
    /// plateaus at `MAX_PRICE_MAP_CACHE_ENTRIES` and the oldest-inserted keys
    /// are the ones evicted.
    #[test]
    fn price_map_cache_bounded_under_distinct_key_flood() {
        let base = Instant::now();
        let mut cache = BoundedCache::new();
        let n = MAX_PRICE_MAP_CACHE_ENTRIES + 10;
        for i in 0..n {
            // Monotonic, comfortably-live expiries keep every entry past `now`
            // so this test isolates the size cap from the TTL sweep.
            cache.insert(
                format!("sale:3m::city-{i:08}"),
                Vec::new(),
                base + CACHE_TTL + Duration::from_secs(i as u64),
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

    /// Follow-up regression (#3132): under a burst where every entry lands in
    /// the same clock tick (identical `expires_at`), size-cap eviction must
    /// still be deterministic oldest-inserted-first. The previous
    /// sort-by-`expires_at` eviction broke ties by `HashMap` iteration order
    /// (nondeterministic); the insertion-order index makes it exact. The first
    /// `EVICTED` inserted keys are dropped; the last `MAX` are retained.
    #[test]
    fn price_map_cache_evicts_in_insertion_order_under_tied_expiry() {
        // All entries share one expiry — the realistic coarse-clock / burst
        // case — so eviction order is decided purely by insertion order.
        let tied = Instant::now() + CACHE_TTL;
        let mut cache = BoundedCache::new();
        const EVICTED: usize = 256;
        let n = MAX_PRICE_MAP_CACHE_ENTRIES + EVICTED;
        for i in 0..n {
            cache.insert(format!("city-{i:08}"), Vec::new(), tied);
        }

        assert_eq!(cache.len(), MAX_PRICE_MAP_CACHE_ENTRIES);
        for i in 0..EVICTED {
            assert!(
                !cache.contains_key(&format!("city-{i:08}")),
                "oldest-inserted key city-{i:08} must be evicted first under tied expiry"
            );
        }
        for i in EVICTED..n {
            assert!(
                cache.contains_key(&format!("city-{i:08}")),
                "recently-inserted key city-{i:08} must be retained"
            );
        }
    }

    /// Re-inserting an existing key refreshes it in place: the map does not
    /// grow, and the refreshed key moves to the youngest slot so it survives
    /// a subsequent size-cap eviction (it is no longer the oldest).
    #[test]
    fn price_map_cache_replace_refreshes_without_growing() {
        let tied = Instant::now() + CACHE_TTL;
        let mut cache = BoundedCache::new();
        for i in 0..MAX_PRICE_MAP_CACHE_ENTRIES {
            cache.insert(format!("city-{i:08}"), Vec::new(), tied);
        }
        assert_eq!(cache.len(), MAX_PRICE_MAP_CACHE_ENTRIES);

        // Refresh the oldest key — must not grow the map and must move it to
        // the back of the eviction order.
        cache.insert("city-00000000".to_string(), Vec::new(), tied);
        assert_eq!(
            cache.len(),
            MAX_PRICE_MAP_CACHE_ENTRIES,
            "replacing an existing key must not grow the map"
        );

        // One genuinely new key now evicts the *new* oldest (city-00000001),
        // not the just-refreshed city-00000000.
        cache.insert("city-new".to_string(), Vec::new(), tied);
        assert_eq!(cache.len(), MAX_PRICE_MAP_CACHE_ENTRIES);
        assert!(
            cache.contains_key("city-00000000"),
            "refreshed key must survive — it is no longer the oldest"
        );
        assert!(
            !cache.contains_key("city-00000001"),
            "the new oldest key must be the one evicted"
        );
        assert!(cache.contains_key("city-new"));
    }

    /// The TTL sweep alone keeps already-expired entries from accumulating,
    /// independently of the size cap.
    #[test]
    fn price_map_cache_drops_ttl_expired_entries_on_insert() {
        let mut cache = BoundedCache::new();
        // Pre-seed an already-expired entry.
        cache.insert(
            "stale".to_string(),
            Vec::new(),
            Instant::now() - Duration::from_secs(1),
        );
        cache.insert("fresh".to_string(), Vec::new(), Instant::now() + CACHE_TTL);

        assert!(
            !cache.contains_key("stale"),
            "expired entry must be reclaimed"
        );
        assert!(cache.contains_key("fresh"));
        assert_eq!(cache.len(), 1);
    }

    /// Thread-safety: two tasks each flood `MAX/2 + 1` distinct keys through
    /// the shared write lock; the final size must still respect the cap.
    #[tokio::test]
    async fn price_map_cache_bounded_under_concurrent_distinct_key_flood() {
        let cache: Arc<RwLock<BoundedCache>> = Arc::new(RwLock::new(BoundedCache::new()));
        let per_task = MAX_PRICE_MAP_CACHE_ENTRIES / 2 + 1;

        let mut handles = Vec::new();
        for t in 0..2u64 {
            let cache = cache.clone();
            handles.push(tokio::spawn(async move {
                for i in 0..per_task {
                    let mut w = cache.write().await;
                    w.insert(
                        format!("t{t}:city-{i}"),
                        Vec::new(),
                        Instant::now() + CACHE_TTL,
                    );
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
