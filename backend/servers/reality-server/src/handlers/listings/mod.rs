//! Listing handlers - search and view logic.
//!
//! Implements full-text search, geo-spatial queries, filtering,
//! sorting, pagination, and view count tracking.

use db::models::{PublicListingDetail, PublicListingQuery, PublicListingSummary};
use db::repositories::PortalRepository;
use uuid::Uuid;

/// Listing search result.
#[derive(Debug)]
pub struct SearchResult {
    /// Matching listings
    pub listings: Vec<PublicListingSummary>,
    /// Total count of matching listings
    pub total: i64,
    /// Current page number
    pub page: i32,
    /// Page size
    pub limit: i32,
}

/// Listing handler for search and view operations.
#[derive(Clone)]
pub struct ListingHandler {
    repo: PortalRepository,
}

impl ListingHandler {
    /// Create a new ListingHandler.
    pub fn new(repo: PortalRepository) -> Self {
        Self { repo }
    }

    /// Search listings with full-text search and filters.
    ///
    /// Supports:
    /// - Full-text search on title, description, and address
    /// - Property type filtering (apartment, house, land, commercial)
    /// - Transaction type filtering (sale, rent)
    /// - Price range filtering
    /// - Area range filtering
    /// - Room count filtering
    /// - City and country filtering
    /// - Sorting by price, date, or area
    /// - Pagination
    pub async fn search(&self, query: &PublicListingQuery) -> Result<SearchResult, String> {
        let page = query.page.unwrap_or(1).max(1);
        let limit = query.limit.unwrap_or(20).clamp(1, 100);

        // Search listings
        let listings = self
            .repo
            .search_listings(query)
            .await
            .map_err(|e| format!("Search failed: {}", e))?;

        // Count total for pagination
        let total = self
            .repo
            .count_listings(query)
            .await
            .map_err(|e| format!("Count failed: {}", e))?;

        Ok(SearchResult {
            listings,
            total,
            page,
            limit,
        })
    }

    /// Get listing by ID with full details.
    pub async fn get_listing(&self, id: Uuid) -> Result<Option<PublicListingDetail>, String> {
        // Direct lookup by id. The previous implementation called
        // `search_listings(limit=1)` and then `.find(|l| l.id == id)`, which
        // returns the first active listing regardless of id (and `.find`
        // would then yield `None` whenever that arbitrary first row didn't
        // match) — a functional bug, not just a perf issue.
        let summary = self
            .repo
            .get_listing_by_id(id)
            .await
            .map_err(|e| format!("Failed to fetch listing: {}", e))?;

        match summary {
            Some(s) => Ok(Some(PublicListingDetail {
                id: s.id,
                title: s.title,
                description: s.description,
                price: s.price,
                currency: s.currency,
                is_negotiable: false, // Would need to be fetched from DB
                size_sqm: s.size_sqm,
                rooms: s.rooms,
                bathrooms: None,    // Would need to be fetched from DB
                floor: None,        // Would need to be fetched from DB
                total_floors: None, // Would need to be fetched from DB
                street: String::new(),
                city: s.city,
                postal_code: String::new(),
                country: String::new(),
                latitude: None,
                longitude: None,
                property_type: s.property_type,
                transaction_type: s.transaction_type,
                photos: s.photo_url.map(|url| vec![url]).unwrap_or_default(),
                features: vec![],
                published_at: s.published_at,
                view_count: 0,
            })),
            None => Ok(None),
        }
    }

    /// Get nearby cities for search suggestions.
    pub async fn get_nearby_cities(&self, city: &str, limit: i32) -> Result<Vec<String>, String> {
        self.repo
            .get_nearby_cities(city, limit)
            .await
            .map_err(|e| format!("Failed to get nearby cities: {}", e))
    }

    /// Track a listing view.
    ///
    /// Increments the daily analytics counters for the listing. Previously a
    /// no-op stub that never recorded anything; it now routes through the real
    /// write path — `RealityPortalRepository::track_view`, backed by the
    /// SECURITY DEFINER `portal_track_listing_view` function (migration 00214) —
    /// so views on portal listings actually land in `listing_analytics` despite
    /// its FORCE-RLS org-only policy (#2244). Best-effort at the call site.
    pub async fn track_view(&self, listing_id: Uuid, source: &str) -> Result<(), String> {
        tracing::debug!(%listing_id, %source, "Tracking listing view");
        db::repositories::RealityPortalRepository::new(self.repo.pool().clone())
            .track_view(listing_id, source)
            .await
            .map_err(|e| format!("Failed to track listing view: {}", e))
    }

    /// Get popular searches for suggestions.
    pub fn get_popular_searches(&self, locale: &str) -> Vec<String> {
        // Return locale-specific popular searches
        match locale {
            "sk" => vec![
                "2-izbovy byt Bratislava".to_string(),
                "Dom s pozemkom Kosice".to_string(),
                "Pozemok Zilina".to_string(),
                "3-izbovy byt Nitra".to_string(),
                "Rodinny dom Trnava".to_string(),
            ],
            "cs" => vec![
                "2+kk Praha".to_string(),
                "Rodinny dum Brno".to_string(),
                "Pozemek Ostrava".to_string(),
                "3+1 Plzen".to_string(),
                "Byt k pronajmu Praha".to_string(),
            ],
            "de" => vec![
                "2-Zimmer-Wohnung Wien".to_string(),
                "Haus Graz".to_string(),
                "Grundstueck Salzburg".to_string(),
                "3-Zimmer-Wohnung Linz".to_string(),
                "Wohnung mieten Wien".to_string(),
            ],
            _ => vec![
                "2-bedroom apartment".to_string(),
                "House for sale".to_string(),
                "Land plot".to_string(),
                "3-bedroom house".to_string(),
                "Apartment for rent".to_string(),
            ],
        }
    }

    /// Validate search query parameters.
    pub fn validate_query(query: &PublicListingQuery) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Validate price range
        if let (Some(min), Some(max)) = (query.price_min, query.price_max) {
            if min > max {
                errors.push("Minimum price cannot be greater than maximum price".to_string());
            }
        }

        // Validate area range
        if let (Some(min), Some(max)) = (query.area_min, query.area_max) {
            if min > max {
                errors.push("Minimum area cannot be greater than maximum area".to_string());
            }
        }

        // Validate room range
        if let (Some(min), Some(max)) = (query.rooms_min, query.rooms_max) {
            if min > max {
                errors.push("Minimum rooms cannot be greater than maximum rooms".to_string());
            }
        }

        // Validate property type
        if let Some(ref property_type) = query.property_type {
            let valid_types = ["apartment", "house", "land", "commercial"];
            if !valid_types.contains(&property_type.as_str()) {
                errors.push(format!(
                    "Invalid property type. Must be one of: {}",
                    valid_types.join(", ")
                ));
            }
        }

        // Validate transaction type
        if let Some(ref transaction_type) = query.transaction_type {
            let valid_types = ["sale", "rent"];
            if !valid_types.contains(&transaction_type.as_str()) {
                errors.push(format!(
                    "Invalid transaction type. Must be one of: {}",
                    valid_types.join(", ")
                ));
            }
        }

        // Validate sort
        if let Some(ref sort) = query.sort {
            let valid_sorts = ["price_asc", "price_desc", "date_desc", "area_asc"];
            if !valid_sorts.contains(&sort.as_str()) {
                errors.push(format!(
                    "Invalid sort option. Must be one of: {}",
                    valid_sorts.join(", ")
                ));
            }
        }

        // Validate pagination
        if let Some(page) = query.page {
            if page < 1 {
                errors.push("Page must be at least 1".to_string());
            }
        }

        if let Some(limit) = query.limit {
            if !(1..=100).contains(&limit) {
                errors.push("Limit must be between 1 and 100".to_string());
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Geo-spatial search utilities.
pub mod geo {
    /// Calculate distance between two points in kilometers using Haversine formula.
    pub fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
        const EARTH_RADIUS_KM: f64 = 6371.0;

        let lat1_rad = lat1.to_radians();
        let lat2_rad = lat2.to_radians();
        let delta_lat = (lat2 - lat1).to_radians();
        let delta_lon = (lon2 - lon1).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);

        let c = 2.0 * a.sqrt().asin();

        EARTH_RADIUS_KM * c
    }

    /// Calculate bounding box for radius search.
    pub fn bounding_box(lat: f64, lon: f64, radius_km: f64) -> (f64, f64, f64, f64) {
        const EARTH_RADIUS_KM: f64 = 6371.0;

        let lat_delta = (radius_km / EARTH_RADIUS_KM).to_degrees();
        let lon_delta = (radius_km / (EARTH_RADIUS_KM * lat.to_radians().cos())).to_degrees();

        let min_lat = lat - lat_delta;
        let max_lat = lat + lat_delta;
        let min_lon = lon - lon_delta;
        let max_lon = lon + lon_delta;

        (min_lat, max_lat, min_lon, max_lon)
    }
}
