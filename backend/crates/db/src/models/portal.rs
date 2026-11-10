//! Portal models (Epic 16: Portal Search & Discovery).
//!
//! Models for Reality Portal users, favorites, and saved searches.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ============================================
// Portal Users
// ============================================

/// Portal user entity (separate from PM users).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct PortalUser {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub password_hash: Option<String>, // None if SSO-only
    pub pm_user_id: Option<Uuid>,      // Link to Property Management user
    pub provider: String,              // local, google, facebook, pm_sso
    pub email_verified: bool,
    pub profile_image_url: Option<String>,
    pub locale: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create portal user request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatePortalUser {
    pub email: String,
    pub name: String,
    pub password: Option<String>,
    pub provider: String,
    pub pm_user_id: Option<Uuid>,
}

/// Update portal user request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdatePortalUser {
    pub name: Option<String>,
    pub profile_image_url: Option<String>,
    pub locale: Option<String>,
}

/// Portal user session.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct PortalSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

// ============================================
// Favorites (Story 16.2)
// ============================================

/// Favorite listing entity with price tracking (Story 84.6).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct Favorite {
    pub id: Uuid,
    pub user_id: Uuid,
    pub listing_id: Uuid,
    pub notes: Option<String>,
    /// Original price when the listing was favorited (for price change detection)
    pub original_price: Option<i64>,
    pub created_at: DateTime<Utc>,
}

/// Favorite with listing details.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FavoriteWithListing {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub title: String,
    pub price: i64,
    pub currency: String,
    pub city: String,
    pub property_type: String,
    pub transaction_type: String,
    pub photo_url: Option<String>,
    pub status: String,
    pub price_changed: bool, // True if price changed since favorited
    pub original_price: Option<i64>,
    pub created_at: DateTime<Utc>,
}

/// Row for favorite with listing query.
#[derive(Debug, Clone, FromRow)]
pub struct FavoriteWithListingRow {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub title: String,
    pub price: i64,
    pub currency: String,
    pub city: String,
    pub property_type: String,
    pub transaction_type: String,
    pub photo_url: Option<String>,
    pub status: String,
    pub original_price: Option<i64>,
    pub created_at: DateTime<Utc>,
}

/// Add favorite request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AddFavorite {
    pub notes: Option<String>,
}

/// Favorites list response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FavoritesResponse {
    pub favorites: Vec<FavoriteWithListing>,
    pub total: i64,
}

// ============================================
// Saved Searches (Story 16.3)
// ============================================

/// Search criteria (stored as JSON).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchCriteria {
    pub q: Option<String>,
    pub property_type: Option<String>,
    pub transaction_type: Option<String>,
    pub price_min: Option<i64>,
    pub price_max: Option<i64>,
    pub area_min: Option<i32>,
    pub area_max: Option<i32>,
    pub rooms_min: Option<i32>,
    pub rooms_max: Option<i32>,
    pub city: Option<String>,
    pub country: Option<String>,
}

/// Search alert notification.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchAlert {
    pub search_id: Uuid,
    pub search_name: String,
    pub new_listings: Vec<MatchedListing>,
}

/// Matched listing summary for alerts.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MatchedListing {
    pub id: Uuid,
    pub title: String,
    pub price: i64,
    pub currency: String,
    pub city: String,
    pub photo_url: Option<String>,
    pub published_at: DateTime<Utc>,
}

// ============================================
// Public Listing Search (Story 16.1)
// ============================================

/// Public listing search query parameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct PublicListingQuery {
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

/// Public listing summary for search results.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct PublicListingSummary {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub price: i64,
    pub currency: String,
    pub size_sqm: Option<i32>,
    pub rooms: Option<i32>,
    pub city: String,
    pub property_type: String,
    pub transaction_type: String,
    pub photo_url: Option<String>,
    pub published_at: DateTime<Utc>,
}

/// Public listing search response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublicListingSearchResponse {
    pub listings: Vec<PublicListingSummary>,
    pub total: i64,
    pub page: i32,
    pub limit: i32,
    pub suggestions: Option<SearchSuggestions>,
}

/// Search suggestions when no results found.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchSuggestions {
    pub expand_price_range: bool,
    pub nearby_cities: Vec<String>,
    pub alternative_types: Vec<String>,
}

/// Full public listing detail.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublicListingDetail {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub price: i64,
    pub currency: String,
    pub is_negotiable: bool,
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
    pub photos: Vec<String>,
    pub features: Vec<String>,
    pub published_at: DateTime<Utc>,
    pub view_count: i64,
}
