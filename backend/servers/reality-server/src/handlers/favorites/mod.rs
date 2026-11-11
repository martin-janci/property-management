//! Favorites handlers - save and manage favorite listings.
//!
//! Implements favorites CRUD operations with price tracking.

use db::models::{PortalFavorite, PortalFavoriteWithListing, UpdatePortalFavorite};
use db::repositories::{FavoriteError, RealityPortalRepository};
use uuid::Uuid;

/// Maximum favorites allowed per user.
pub const MAX_FAVORITES_PER_USER: i32 = 100;

/// Favorites operation result.
#[derive(Debug)]
pub enum FavoriteResult {
    /// Operation successful
    Success,
    /// Listing not found
    ListingNotFound,
    /// Favorite not found
    FavoriteNotFound,
    /// Maximum favorites limit reached
    LimitReached,
    /// Already favorited
    AlreadyFavorited,
    /// Database error
    DatabaseError(String),
}

/// Favorites handler for managing user favorites.
#[derive(Clone)]
pub struct FavoritesHandler {
    repo: RealityPortalRepository,
}

impl FavoritesHandler {
    /// Create a new FavoritesHandler.
    pub fn new(repo: RealityPortalRepository) -> Self {
        Self { repo }
    }

    /// Add a listing to favorites.
    pub async fn add_favorite(
        &self,
        user_id: Uuid,
        listing_id: Uuid,
        notes: Option<String>,
    ) -> Result<PortalFavorite, FavoriteResult> {
        // Already favorited?
        let existing = self
            .repo
            .get_favorites_with_listings(user_id)
            .await
            .map_err(|e| FavoriteResult::DatabaseError(e.to_string()))?;
        if existing.iter().any(|f| f.listing_id == listing_id) {
            return Err(FavoriteResult::AlreadyFavorited);
        }

        // Cap check + insert run atomically inside the repository (TOCTOU-safe).
        self.repo
            .add_favorite_capped(user_id, listing_id, notes, MAX_FAVORITES_PER_USER as i64)
            .await
            .map_err(|e| match e {
                FavoriteError::LimitReached { .. } => FavoriteResult::LimitReached,
                FavoriteError::Db(e) => {
                    // A foreign-key violation means the referenced listing is gone.
                    if crate::util::errors::classify_db_error(&e)
                        == crate::util::errors::DbErrorClass::ForeignKeyViolation
                    {
                        FavoriteResult::ListingNotFound
                    } else {
                        FavoriteResult::DatabaseError(e.to_string())
                    }
                }
            })
    }

    /// Remove a listing from favorites.
    pub async fn remove_favorite(
        &self,
        user_id: Uuid,
        listing_id: Uuid,
    ) -> Result<(), FavoriteResult> {
        let removed = self
            .repo
            .remove_favorite(user_id, listing_id)
            .await
            .map_err(|e| FavoriteResult::DatabaseError(e.to_string()))?;

        if removed {
            Ok(())
        } else {
            Err(FavoriteResult::FavoriteNotFound)
        }
    }

    /// Get all favorites for a user.
    pub async fn get_favorites(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<PortalFavoriteWithListing>, String> {
        self.repo
            .get_favorites_with_listings(user_id)
            .await
            .map_err(|e| e.to_string())
    }

    /// Check if a listing is favorited by a user.
    pub async fn is_favorited(&self, user_id: Uuid, listing_id: Uuid) -> Result<bool, String> {
        let favorites = self
            .repo
            .get_favorites_with_listings(user_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(favorites.iter().any(|f| f.listing_id == listing_id))
    }

    /// Update favorite notes or price alert settings.
    pub async fn update_favorite(
        &self,
        user_id: Uuid,
        listing_id: Uuid,
        notes: Option<String>,
        price_alert_enabled: Option<bool>,
    ) -> Result<PortalFavorite, FavoriteResult> {
        let update = UpdatePortalFavorite {
            notes,
            price_alert_enabled,
        };

        self.repo
            .update_favorite(user_id, listing_id, update)
            .await
            .map_err(|e| {
                // `fetch_one` on the UPDATE yields `RowNotFound` when the user
                // has no such favorite. Classify by the sqlx variant, not the
                // "no rows" message text.
                if crate::util::errors::classify_db_error(&e)
                    == crate::util::errors::DbErrorClass::NotFound
                {
                    FavoriteResult::FavoriteNotFound
                } else {
                    FavoriteResult::DatabaseError(e.to_string())
                }
            })
    }

    /// Get favorites count for a user.
    pub async fn get_favorites_count(&self, user_id: Uuid) -> Result<i32, String> {
        let favorites = self
            .repo
            .get_favorites_with_listings(user_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(favorites.len() as i32)
    }

    /// Get favorites with price changes.
    pub async fn get_favorites_with_price_changes(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<PortalFavoriteWithListing>, String> {
        let favorites = self
            .repo
            .get_favorites_with_listings(user_id)
            .await
            .map_err(|e| e.to_string())?;

        // Filter to only those with price changes
        Ok(favorites.into_iter().filter(|f| f.price_changed).collect())
    }

    /// Get price change alerts for a user.
    pub async fn get_price_alerts(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<db::models::PriceChangeAlert>, String> {
        self.repo
            .get_price_change_alerts(user_id)
            .await
            .map_err(|e| e.to_string())
    }
}
