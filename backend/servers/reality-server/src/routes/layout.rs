//! Public layout resolved endpoint — reality-server side.
//!
//! Serves `GET /api/v1/layout/resolved/{*screen}` with no auth and no tenant
//! layer. Only screens with a published config resolve; others 404.
//! Uses `acquire_public_conn()` to clear stale RLS context (required by
//! `scripts/check-rls-enforcement.sh`).
//!
//! Successful resolutions are cached in-process for 1 hour, mirroring the
//! sibling public read-only aggregation endpoint `routes::price_map`: both are
//! unauthenticated GETs that run several DB reads to assemble a mostly-static
//! response, so an uncached layout endpoint re-ran three queries
//! (`get_config` + `get_manifest` + `list_kills`) + a `resolve()` on every
//! portal page load. The cache uses `tokio::sync::RwLock` (NOT
//! `std::sync::RwLock`) so a contention spike never blocks the Tokio worker
//! thread, and is bounded + insertion-ordered so a caller rotating distinct
//! `{*screen}` / `platform` keys can never grow the heap without bound
//! (the same OOM-DoS defence `price_map` carries).

use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use db::repositories::LayoutRepository;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Cache TTL: 1 hour — matches `routes::price_map::CACHE_TTL`.
const CACHE_TTL: Duration = Duration::from_secs(3600);

/// Hard cap on the number of distinct entries held by [`RESOLVED_CACHE`].
///
/// The cache key embeds the caller-controlled `{*screen}` path and `platform`
/// query parameter off the public, unauthenticated resolved endpoint. Only
/// successful (published) resolutions are stored, but bounding the map inline
/// on every insert caps memory regardless of how many distinct keys are
/// presented — mirroring `price_map`'s `MAX_PRICE_MAP_CACHE_ENTRIES`.
const MAX_RESOLVED_CACHE_ENTRIES: usize = 10_000;

/// Resolved-layout cache entry.
#[derive(Clone, Debug)]
struct CacheEntry {
    data: layout_core::ResolvedScreen,
    expires_at: Instant,
    /// Monotonic insertion sequence — the eviction-order key into
    /// [`BoundedCache::order`]. Assigned by [`BoundedCache::insert`]; callers
    /// never set it.
    seq: u64,
}

/// Global resolved-layout cache (key = `"{screen}:{platform}"`).
static RESOLVED_CACHE: std::sync::OnceLock<Arc<RwLock<BoundedCache>>> = std::sync::OnceLock::new();

fn resolved_cache() -> &'static Arc<RwLock<BoundedCache>> {
    RESOLVED_CACHE.get_or_init(|| Arc::new(RwLock::new(BoundedCache::new())))
}

/// Bounded, insertion-ordered resolved-layout cache.
///
/// A lookup `HashMap` paired with a monotonic insertion-order index
/// (`BTreeMap<seq, key>`), so both TTL reclamation and the size cap run without
/// cloning the keyset or sorting on every insert — the same proven shape as
/// `routes::price_map::BoundedCache`.
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
    ///
    /// Same discipline as `routes::price_map::BoundedCache::insert`: reclaim
    /// TTL-expired entries from the front of the order index (amortised O(1)),
    /// re-sequence a replaced key to the back, then enforce the size cap by
    /// evicting the oldest-inserted entry — no full-keyset clone or sort.
    fn insert(&mut self, key: String, data: layout_core::ResolvedScreen, expires_at: Instant) {
        let now = Instant::now();
        // (1) Correctness: reclaim TTL-expired entries from the front (oldest
        // first), stopping at the first live entry.
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
        // order slot so the refreshed entry re-sequences to the back below.
        if let Some(prev) = self.entries.remove(&key) {
            self.order.remove(&prev.seq);
        }
        // (3) Memory bound: evict the oldest-inserted entry until there is room.
        while self.entries.len() >= MAX_RESOLVED_CACHE_ENTRIES {
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

/// Build the cache key from the (already-validated) screen id and platform.
fn cache_key(screen: &str, platform_key: &str) -> String {
    format!("{screen}:{platform_key}")
}

#[derive(Debug, Deserialize)]
pub struct ResolvedQuery {
    pub platform: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/resolved/{*screen}", get(get_resolved))
}

/// Namespace guard for the public, unauthenticated resolved endpoint: only
/// `reality/*` screens are servable here. Internal namespaces (e.g. `ppt/*`
/// management screens) must never leak through the public portal — they 404
/// before any DB access.
fn is_public_layout_screen(screen: &str) -> bool {
    screen.starts_with("reality/")
}

#[utoipa::path(get, path = "/api/v1/layout/resolved/{screen}", tag = "Layout",
    params(("screen" = String, Path, description = "Screen id, e.g. reality/listing-detail"),
           ("platform" = Option<String>, Query, description = "web|mobile, default web")),
    responses((status = 200, description = "Resolved section list (public, no tenant layer)"),
              (status = 404, description = "Screen not published or manifest missing")))]
pub async fn get_resolved(
    State(state): State<AppState>,
    Path(screen): Path<String>,
    Query(q): Query<ResolvedQuery>,
) -> Result<Json<layout_core::ResolvedScreen>, (StatusCode, String)> {
    // Public namespace restriction — before any DB access.
    if !is_public_layout_screen(&screen) {
        return Err((StatusCode::NOT_FOUND, "unknown screen".to_string()));
    }
    let platform = match q.platform.as_deref().unwrap_or("web") {
        "web" => layout_core::Platform::Web,
        "mobile" => layout_core::Platform::Mobile,
        other => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("unknown platform {other:?}"),
            ))
        }
    };
    let platform_key = match platform {
        layout_core::Platform::Web => "web",
        layout_core::Platform::Mobile => "mobile",
    };
    let key = cache_key(&screen, platform_key);

    // Serve from the in-process cache when a live entry exists — avoids the
    // three DB reads + resolve on every portal page load (mirrors price_map).
    {
        let cache = resolved_cache().read().await;
        if let Some(entry) = cache.get(&key) {
            if Instant::now() < entry.expires_at {
                tracing::debug!(%screen, platform_key, "resolved layout cache hit");
                return Ok(Json(entry.data.clone()));
            }
        }
    }

    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| crate::util::errors::db_error("acquire db connection", e))?;
    let repo = LayoutRepository::new();

    let cfg = repo
        .get_config(&mut *conn, &screen)
        .await
        .map_err(|e| crate::util::errors::db_error("get layout config", e))?
        .ok_or((StatusCode::NOT_FOUND, "unknown screen".to_string()))?;
    let published = cfg.published.ok_or((
        StatusCode::NOT_FOUND,
        "screen has no published config".to_string(),
    ))?;
    let base: layout_core::ScreenConfig = serde_json::from_value(published)
        .map_err(|e| crate::util::errors::db_error("parse stored layout config", e))?;

    let manifest_row = repo
        .get_manifest(&mut *conn, platform_key)
        .await
        .map_err(|e| crate::util::errors::db_error("get layout manifest", e))?
        .ok_or((
            StatusCode::NOT_FOUND,
            "no registry manifest for platform".to_string(),
        ))?;
    let manifest: layout_core::RegistryManifest = serde_json::from_value(manifest_row.manifest)
        .map_err(|e| crate::util::errors::db_error("parse stored layout manifest", e))?;

    let kills: BTreeSet<layout_core::SectionType> = repo
        .list_kills(&mut *conn, &screen)
        .await
        .map_err(|e| crate::util::errors::db_error("list layout kills", e))?
        .into_iter()
        .map(|k| layout_core::SectionType(k.section_type))
        .collect();

    // Public portal: no tenant layer (spec §3.2 — the tenant layer simply
    // doesn't contribute on public reality screens).
    let resolved = layout_core::resolve(&base, platform, None, &kills, &manifest);

    // Store the resolution for the TTL window. `BoundedCache::insert` reclaims
    // expired entries and enforces the size cap via an insertion-order index,
    // so the map can never grow past `MAX_RESOLVED_CACHE_ENTRIES` even if a
    // caller rotates distinct `{*screen}` / `platform` keys.
    {
        let mut cache = resolved_cache().write().await;
        cache.insert(key, resolved.clone(), Instant::now() + CACHE_TTL);
    }

    Ok(Json(resolved))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal `ResolvedScreen` for cache tests — contents are irrelevant to
    /// the cache mechanics (TTL, round-trip, size bound), only identity is.
    fn sample_resolved() -> layout_core::ResolvedScreen {
        layout_core::ResolvedScreen {
            screen: "reality/listing-detail".to_string(),
            version: 1,
            sections: Vec::new(),
        }
    }

    #[test]
    fn only_reality_namespace_is_public() {
        assert!(is_public_layout_screen("reality/listing-detail"));
        assert!(is_public_layout_screen("reality/home"));
        // internal namespaces must not be servable on the public portal
        assert!(!is_public_layout_screen("ppt/dashboard"));
        assert!(!is_public_layout_screen("admin/anything"));
        assert!(!is_public_layout_screen("reality")); // no trailing segment
        assert!(!is_public_layout_screen(""));
        assert!(!is_public_layout_screen("Reality/home")); // case-sensitive
    }

    #[test]
    fn cache_key_combines_screen_and_platform() {
        assert_eq!(
            cache_key("reality/listing-detail", "web"),
            "reality/listing-detail:web"
        );
        // Platform is part of the key so web/mobile resolutions never collide.
        assert_ne!(
            cache_key("reality/home", "web"),
            cache_key("reality/home", "mobile")
        );
    }

    /// Characterization: a stored resolution is served back on the next lookup
    /// (the whole point of the cache). Fails against the no-op `insert` stub.
    #[test]
    fn cache_round_trips_a_live_entry() {
        let mut cache = BoundedCache::new();
        let key = cache_key("reality/listing-detail", "web");
        cache.insert(key.clone(), sample_resolved(), Instant::now() + CACHE_TTL);
        let entry = cache.get(&key).expect("stored entry must be retrievable");
        assert!(
            Instant::now() < entry.expires_at,
            "entry must still be live"
        );
    }

    /// Characterization: an already-expired entry is reclaimed on the next
    /// insert and never accumulates.
    #[test]
    fn cache_drops_ttl_expired_entries_on_insert() {
        let mut cache = BoundedCache::new();
        cache.insert(
            "stale".to_string(),
            sample_resolved(),
            Instant::now() - Duration::from_secs(1),
        );
        cache.insert(
            "fresh".to_string(),
            sample_resolved(),
            Instant::now() + CACHE_TTL,
        );
        assert!(
            !cache.contains_key("stale"),
            "expired entry must be reclaimed"
        );
        assert!(cache.contains_key("fresh"));
        assert_eq!(cache.len(), 1);
    }

    /// Characterization / DoS bound: a caller rotating distinct keys cannot
    /// grow the cache past `MAX_RESOLVED_CACHE_ENTRIES`; the oldest-inserted
    /// keys are the ones evicted.
    #[test]
    fn cache_is_bounded_under_distinct_key_flood() {
        let base = Instant::now();
        let mut cache = BoundedCache::new();
        let n = MAX_RESOLVED_CACHE_ENTRIES + 10;
        for i in 0..n {
            cache.insert(
                format!("reality/screen-{i:08}:web"),
                sample_resolved(),
                base + CACHE_TTL + Duration::from_secs(i as u64),
            );
        }
        assert_eq!(
            cache.len(),
            MAX_RESOLVED_CACHE_ENTRIES,
            "cache must stay bounded after a distinct-key flood within the TTL"
        );
        assert!(
            !cache.contains_key("reality/screen-00000000:web"),
            "oldest-inserted key must have been evicted under the size cap"
        );
        assert!(
            cache.contains_key(&format!("reality/screen-{:08}:web", n - 1)),
            "most-recently-inserted key must be retained"
        );
    }

    /// Re-inserting an existing key refreshes it in place without growing the
    /// map, and moves it to the youngest eviction slot.
    #[test]
    fn cache_replace_refreshes_without_growing() {
        let tied = Instant::now() + CACHE_TTL;
        let mut cache = BoundedCache::new();
        for i in 0..MAX_RESOLVED_CACHE_ENTRIES {
            cache.insert(format!("screen-{i:08}"), sample_resolved(), tied);
        }
        assert_eq!(cache.len(), MAX_RESOLVED_CACHE_ENTRIES);

        // Refresh the oldest key — must not grow, must survive the next eviction.
        cache.insert("screen-00000000".to_string(), sample_resolved(), tied);
        assert_eq!(cache.len(), MAX_RESOLVED_CACHE_ENTRIES);

        cache.insert("screen-new".to_string(), sample_resolved(), tied);
        assert_eq!(cache.len(), MAX_RESOLVED_CACHE_ENTRIES);
        assert!(
            cache.contains_key("screen-00000000"),
            "refreshed key must survive — it is no longer the oldest"
        );
        assert!(
            !cache.contains_key("screen-00000001"),
            "the new oldest key must be the one evicted"
        );
        assert!(cache.contains_key("screen-new"));
    }
}
