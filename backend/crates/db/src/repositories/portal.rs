//! Portal repository (Epic 16: Portal Search & Discovery).

use crate::models::portal::{
    CreatePortalUser, Favorite, FavoriteWithListing, FavoriteWithListingRow, PortalUser,
    PublicListingQuery, PublicListingSummary, SearchCriteria, UpdatePortalUser,
};
use crate::DbPool;
use chrono::Utc;
use sqlx::{Error as SqlxError, FromRow};
use uuid::Uuid;

/// Row for public listing summary.
#[derive(Debug, FromRow)]
struct PublicListingRow {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub price: rust_decimal::Decimal,
    pub currency: String,
    pub size_sqm: Option<rust_decimal::Decimal>,
    pub rooms: Option<i32>,
    pub city: String,
    pub property_type: String,
    pub transaction_type: String,
    pub photo_url: Option<String>,
    pub published_at: chrono::DateTime<Utc>,
}

/// Repository for portal operations.
#[derive(Clone)]
pub struct PortalRepository {
    pool: DbPool,
}

impl PortalRepository {
    /// Create a new PortalRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Borrow the underlying connection pool.
    ///
    /// N1: callers that need to construct a sibling repository (e.g.
    /// `UnifiedPortalUserRepo` for dual-write) can clone the pool from here
    /// instead of being passed a separate handle.
    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    // ========================================================================
    // Portal Users
    // ========================================================================

    /// SQL fragment that projects `users` columns to the `PortalUser` shape.
    ///
    /// Phase 6: `portal_users` has been dropped (migration 00148). All portal
    /// user records now live in `users` with `principal_kind = 'public'`. This
    /// helper allows `query_as::<_, PortalUser>` calls to work against `users`
    /// without changing the `PortalUser` struct (which is still referenced by
    /// handler return types).
    ///
    /// Column mapping:
    ///  - `pm_user_id`   → NULL (back-pointer is now `users.portal_origin_id`)
    ///  - `provider`     → 'local' (the unified schema no longer tracks the
    ///    auth provider; SSO accounts are identified by the
    ///    `!sso-only-no-password` sentinel in `password_hash`, NOT by
    ///    `principal_kind` which is the user-kind discriminator
    ///    public/staff/platform)
    ///  - `email_verified` → derived from `email_verified_at IS NOT NULL`
    ///  - `password_hash` → NULL when the sentinel value is stored (SSO-only)
    fn portal_user_projection() -> &'static str {
        r#"
        SELECT
            u.id,
            u.email,
            u.name,
            CASE WHEN u.password_hash = '!sso-only-no-password'
                 THEN NULL
                 ELSE u.password_hash
            END AS password_hash,
            NULL::uuid               AS pm_user_id,
            'local'                  AS provider,
            (u.email_verified_at IS NOT NULL) AS email_verified,
            u.profile_image_url,
            u.locale,
            u.created_at,
            u.updated_at
        FROM users u
        WHERE u.principal_kind = 'public'
          AND u.status != 'deleted'
        "#
    }

    /// Create a new portal user.
    ///
    /// Phase 6: writes directly to `users` (principal_kind='public').
    /// `portal_users` has been dropped by migration 00148.
    pub async fn create_user(&self, data: CreatePortalUser) -> Result<PortalUser, SqlxError> {
        let user = sqlx::query_as::<_, PortalUser>(
            r#"
            INSERT INTO users (
                email, password_hash, name, locale, status,
                principal_kind, created_at, updated_at
            )
            VALUES ($1, COALESCE($2, '!sso-only-no-password'), $3, 'en', 'active', 'public', NOW(), NOW())
            RETURNING
                id,
                email,
                name,
                CASE WHEN password_hash = '!sso-only-no-password' THEN NULL ELSE password_hash END AS password_hash,
                NULL::uuid AS pm_user_id,
                'local' AS provider,
                (email_verified_at IS NOT NULL) AS email_verified,
                profile_image_url,
                locale,
                created_at,
                updated_at
            "#,
        )
        .bind(&data.email)
        .bind(&data.password)
        .bind(&data.name)
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    /// Find portal user by ID.
    ///
    /// Phase 6: reads from `users` (principal_kind='public').
    pub async fn find_user_by_id(&self, id: Uuid) -> Result<Option<PortalUser>, SqlxError> {
        let sql = format!("{} AND u.id = $1", Self::portal_user_projection());
        let user = sqlx::query_as::<_, PortalUser>(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(user)
    }

    /// Find portal user by email.
    ///
    /// Phase 6: reads from `users` (principal_kind='public').
    pub async fn find_user_by_email(&self, email: &str) -> Result<Option<PortalUser>, SqlxError> {
        let sql = format!(
            "{} AND LOWER(u.email) = LOWER($1)",
            Self::portal_user_projection()
        );
        let user = sqlx::query_as::<_, PortalUser>(sqlx::AssertSqlSafe(sql))
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;

        Ok(user)
    }

    /// Find portal user by PM user ID (for SSO).
    ///
    /// Phase 6: `pm_user_id` was `portal_users.pm_user_id`; after unification
    /// the equivalent concept is `users.portal_origin_id` pointing at the old
    /// portal_users.id, but for SSO the PM user link is expressed as the PM
    /// users.id stored in `users` with `principal_kind = 'staff'`. The
    /// original `portal_users.pm_user_id` column was dropped along with
    /// the table (migration 00148), and the unified `users` schema does
    /// NOT carry a `pm_user_id` back-pointer (only `portal_origin_id`
    /// which points to the LEGACY portal_users.id, not a PM user.id).
    ///
    /// Returning `Ok(None)` keeps the method's signature compatible for
    /// the few legacy callers but encourages them to switch to
    /// `find_user_by_email`. Reality-server's SSO upsert already keys on
    /// email, so the "was_existing" classification should be derived from
    /// an email lookup, not from this method — see
    /// `handlers/users::UserHandler::upsert_sso_user`.
    pub async fn find_user_by_pm_id(
        &self,
        _pm_user_id: Uuid,
    ) -> Result<Option<PortalUser>, SqlxError> {
        Ok(None)
    }

    /// Update portal user profile fields.
    ///
    /// Phase 6: updates `users` directly (principal_kind='public').
    pub async fn update_user(
        &self,
        id: Uuid,
        data: UpdatePortalUser,
    ) -> Result<PortalUser, SqlxError> {
        let user = sqlx::query_as::<_, PortalUser>(
            r#"
            UPDATE users SET
                name              = COALESCE($2, name),
                profile_image_url = COALESCE($3, profile_image_url),
                locale            = COALESCE($4, locale),
                updated_at        = NOW()
            WHERE id = $1
              AND principal_kind = 'public'
              AND status != 'deleted'
            RETURNING
                id,
                email,
                name,
                CASE WHEN password_hash = '!sso-only-no-password' THEN NULL ELSE password_hash END AS password_hash,
                NULL::uuid AS pm_user_id,
                'local' AS provider,
                (email_verified_at IS NOT NULL) AS email_verified,
                profile_image_url,
                locale,
                created_at,
                updated_at
            "#,
        )
        .bind(id)
        .bind(&data.name)
        .bind(&data.profile_image_url)
        .bind(&data.locale)
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    /// Replace a portal user's password hash.
    ///
    /// Phase 6: updates `users` directly (principal_kind='public').
    pub async fn update_password_hash(
        &self,
        id: Uuid,
        password_hash: &str,
    ) -> Result<PortalUser, SqlxError> {
        let user = sqlx::query_as::<_, PortalUser>(
            r#"
            UPDATE users SET
                password_hash = $2,
                updated_at    = NOW()
            WHERE id = $1
              AND principal_kind = 'public'
              AND status != 'deleted'
            RETURNING
                id,
                email,
                name,
                CASE WHEN password_hash = '!sso-only-no-password' THEN NULL ELSE password_hash END AS password_hash,
                NULL::uuid AS pm_user_id,
                'local' AS provider,
                (email_verified_at IS NOT NULL) AS email_verified,
                profile_image_url,
                locale,
                created_at,
                updated_at
            "#,
        )
        .bind(id)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    // ========================================================================
    // Public Listing Search (Story 16.1)
    // ========================================================================

    /// Search public listings.
    pub async fn search_listings(
        &self,
        query: &PublicListingQuery,
    ) -> Result<Vec<PublicListingSummary>, SqlxError> {
        let page = query.page.unwrap_or(1).max(1);
        let limit = query.limit.unwrap_or(20).min(100);
        let offset = (page - 1) * limit;

        // Sort order
        let order_by = match query.sort.as_deref() {
            Some("price_asc") => "l.price ASC",
            Some("price_desc") => "l.price DESC",
            Some("area_asc") => "l.size_sqm ASC",
            Some("date_desc") => "l.published_at DESC",
            _ => "l.published_at DESC",
        };

        let sql = format!(
            r#"
            SELECT
                l.id, l.title, l.description, l.price, l.currency,
                l.size_sqm, l.rooms, l.city, l.property_type, l.transaction_type,
                (SELECT url FROM listing_photos WHERE listing_id = l.id ORDER BY display_order LIMIT 1) as photo_url,
                l.published_at
            FROM listings l
            WHERE l.status = 'active'
                AND ($1::text IS NULL OR l.title ILIKE '%' || $1 || '%' OR l.description ILIKE '%' || $1 || '%' OR l.city ILIKE '%' || $1 || '%')
                AND ($2::text IS NULL OR l.property_type = $2)
                AND ($3::text IS NULL OR l.transaction_type = $3)
                AND ($4::bigint IS NULL OR l.price >= $4)
                AND ($5::bigint IS NULL OR l.price <= $5)
                AND ($6::int IS NULL OR l.size_sqm >= $6)
                AND ($7::int IS NULL OR l.size_sqm <= $7)
                AND ($8::int IS NULL OR l.rooms >= $8)
                AND ($9::int IS NULL OR l.rooms <= $9)
                AND ($10::text IS NULL OR l.city ILIKE '%' || $10 || '%')
                AND ($11::text IS NULL OR l.country = $11)
            ORDER BY {}
            LIMIT $12 OFFSET $13
            "#,
            order_by
        );
        let rows = sqlx::query_as::<_, PublicListingRow>(sqlx::AssertSqlSafe(sql))
            .bind(&query.q)
            .bind(&query.property_type)
            .bind(&query.transaction_type)
            .bind(query.price_min)
            .bind(query.price_max)
            .bind(query.area_min)
            .bind(query.area_max)
            .bind(query.rooms_min)
            .bind(query.rooms_max)
            .bind(&query.city)
            .bind(&query.country)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        // Convert to PublicListingSummary
        let listings = rows
            .into_iter()
            .map(|r| PublicListingSummary {
                id: r.id,
                title: r.title,
                description: r.description,
                price: r.price.try_into().unwrap_or(0),
                currency: r.currency,
                size_sqm: r.size_sqm.map(|d| d.try_into().unwrap_or(0)),
                rooms: r.rooms,
                city: r.city,
                property_type: r.property_type,
                transaction_type: r.transaction_type,
                photo_url: r.photo_url,
                published_at: r.published_at,
            })
            .collect();

        Ok(listings)
    }

    /// Count public listings matching query.
    pub async fn count_listings(&self, query: &PublicListingQuery) -> Result<i64, SqlxError> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM listings l
            WHERE l.status = 'active'
                AND ($1::text IS NULL OR l.title ILIKE '%' || $1 || '%' OR l.description ILIKE '%' || $1 || '%' OR l.city ILIKE '%' || $1 || '%')
                AND ($2::text IS NULL OR l.property_type = $2)
                AND ($3::text IS NULL OR l.transaction_type = $3)
                AND ($4::bigint IS NULL OR l.price >= $4)
                AND ($5::bigint IS NULL OR l.price <= $5)
                AND ($6::int IS NULL OR l.size_sqm >= $6)
                AND ($7::int IS NULL OR l.size_sqm <= $7)
                AND ($8::int IS NULL OR l.rooms >= $8)
                AND ($9::int IS NULL OR l.rooms <= $9)
                AND ($10::text IS NULL OR l.city ILIKE '%' || $10 || '%')
                AND ($11::text IS NULL OR l.country = $11)
            "#,
        )
        .bind(&query.q)
        .bind(&query.property_type)
        .bind(&query.transaction_type)
        .bind(query.price_min)
        .bind(query.price_max)
        .bind(query.area_min)
        .bind(query.area_max)
        .bind(query.rooms_min)
        .bind(query.rooms_max)
        .bind(&query.city)
        .bind(&query.country)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0)
    }

    /// Get a single listing by ID.
    pub async fn get_listing_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<PublicListingSummary>, SqlxError> {
        let row = sqlx::query_as::<_, PublicListingRow>(
            r#"
            SELECT
                l.id, l.title, l.description, l.price, l.currency,
                l.size_sqm, l.rooms, l.city, l.property_type, l.transaction_type,
                (SELECT url FROM listing_photos WHERE listing_id = l.id ORDER BY display_order LIMIT 1) as photo_url,
                l.published_at
            FROM listings l
            WHERE l.id = $1 AND l.status = 'active'
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| PublicListingSummary {
            id: r.id,
            title: r.title,
            description: r.description,
            price: r.price.try_into().unwrap_or(0),
            currency: r.currency,
            size_sqm: r.size_sqm.map(|d| d.try_into().unwrap_or(0)),
            rooms: r.rooms,
            city: r.city,
            property_type: r.property_type,
            transaction_type: r.transaction_type,
            photo_url: r.photo_url,
            published_at: r.published_at,
        }))
    }

    /// Get nearby cities for suggestions.
    pub async fn get_nearby_cities(
        &self,
        city: &str,
        limit: i32,
    ) -> Result<Vec<String>, SqlxError> {
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT city
            FROM listings
            WHERE status = 'active' AND city != $1
            ORDER BY city
            LIMIT $2
            "#,
        )
        .bind(city)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|(c,)| c).collect())
    }

    // ========================================================================
    // Favorites (Story 16.2)
    // ========================================================================

    /// Add listing to favorites with price tracking (Story 84.6).
    ///
    /// When a listing is added to favorites, the current price is stored as
    /// original_price. This enables price change notifications when the listing
    /// price changes.
    pub async fn add_favorite(
        &self,
        user_id: Uuid,
        listing_id: Uuid,
        notes: Option<String>,
    ) -> Result<Favorite, SqlxError> {
        // Store original price from the listing at the time of favoriting
        // This enables price change detection later
        let favorite = sqlx::query_as::<_, Favorite>(
            r#"
            INSERT INTO favorites (user_id, listing_id, notes, original_price)
            SELECT $1, $2, $3, l.price
            FROM listings l
            WHERE l.id = $2
            ON CONFLICT (user_id, listing_id) DO UPDATE SET
                notes = COALESCE($3, favorites.notes),
                created_at = favorites.created_at
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(listing_id)
        .bind(&notes)
        .fetch_one(&self.pool)
        .await?;

        Ok(favorite)
    }

    /// Remove listing from favorites.
    pub async fn remove_favorite(
        &self,
        user_id: Uuid,
        listing_id: Uuid,
    ) -> Result<bool, SqlxError> {
        let result = sqlx::query(r#"DELETE FROM favorites WHERE user_id = $1 AND listing_id = $2"#)
            .bind(user_id)
            .bind(listing_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get user's favorites with listing details and price change tracking (Story 84.6).
    ///
    /// Returns favorites with price change detection. The `price_changed` flag indicates
    /// when the listing's current price differs from the original price captured at the
    /// time the favorite was added. This enables users to be notified when a property
    /// they're interested in has a price change (either increase or decrease).
    ///
    /// Price change detection logic:
    /// - `original_price`: The listing price when the favorite was created
    /// - `price`: The current listing price
    /// - `price_changed`: True if original_price != current price
    pub async fn get_favorites(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<FavoriteWithListing>, SqlxError> {
        // Query favorites with original_price for price change tracking
        // The original_price column stores the price at the time the favorite was added
        // Note: original_price column should be added to favorites table via migration
        let rows = sqlx::query_as::<_, FavoriteWithListingRow>(
            r#"
            SELECT
                f.id, f.listing_id, l.title, l.price, l.currency, l.city,
                l.property_type, l.transaction_type,
                (SELECT url FROM listing_photos WHERE listing_id = l.id ORDER BY display_order LIMIT 1) as photo_url,
                l.status,
                COALESCE(f.original_price, l.price) as original_price,
                f.created_at
            FROM favorites f
            JOIN listings l ON l.id = f.listing_id
            WHERE f.user_id = $1
            ORDER BY f.created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        // Convert to FavoriteWithListing with price change detection
        let favorites = rows
            .into_iter()
            .map(|r| {
                // Detect price change by comparing current price with original price
                // Price changed if original_price exists and differs from current price
                let price_changed = r.original_price.is_some_and(|original| r.price != original);

                FavoriteWithListing {
                    id: r.id,
                    listing_id: r.listing_id,
                    title: r.title,
                    price: r.price,
                    currency: r.currency,
                    city: r.city,
                    property_type: r.property_type,
                    transaction_type: r.transaction_type,
                    photo_url: r.photo_url,
                    status: r.status,
                    price_changed,
                    original_price: r.original_price,
                    created_at: r.created_at,
                }
            })
            .collect();

        Ok(favorites)
    }

    /// Count user's favorites.
    pub async fn count_favorites(&self, user_id: Uuid) -> Result<i64, SqlxError> {
        let row: (i64,) = sqlx::query_as(r#"SELECT COUNT(*) FROM favorites WHERE user_id = $1"#)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(row.0)
    }

    /// Check if listing is favorited by user.
    pub async fn is_favorited(&self, user_id: Uuid, listing_id: Uuid) -> Result<bool, SqlxError> {
        let row: (bool,) = sqlx::query_as(
            r#"SELECT EXISTS(SELECT 1 FROM favorites WHERE user_id = $1 AND listing_id = $2)"#,
        )
        .bind(user_id)
        .bind(listing_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0)
    }

    // ========================================================================
    // Saved Searches (Story 16.3)
    // ========================================================================

    /// Find matching listings for a saved search (for alerts).
    pub async fn find_matching_listings(
        &self,
        criteria: &SearchCriteria,
        since: chrono::DateTime<Utc>,
    ) -> Result<Vec<PublicListingSummary>, SqlxError> {
        let query = PublicListingQuery {
            q: criteria.q.clone(),
            property_type: criteria.property_type.clone(),
            transaction_type: criteria.transaction_type.clone(),
            price_min: criteria.price_min,
            price_max: criteria.price_max,
            area_min: criteria.area_min,
            area_max: criteria.area_max,
            rooms_min: criteria.rooms_min,
            rooms_max: criteria.rooms_max,
            city: criteria.city.clone(),
            country: criteria.country.clone(),
            page: Some(1),
            limit: Some(50), // Max 50 matches per alert
            sort: Some("date_desc".to_string()),
        };

        // Search with additional filter for new listings
        let rows = sqlx::query_as::<_, PublicListingRow>(
            r#"
            SELECT
                l.id, l.title, l.description, l.price, l.currency,
                l.size_sqm, l.rooms, l.city, l.property_type, l.transaction_type,
                (SELECT url FROM listing_photos WHERE listing_id = l.id ORDER BY display_order LIMIT 1) as photo_url,
                l.published_at
            FROM listings l
            WHERE l.status = 'active'
                AND l.published_at > $12
                AND ($1::text IS NULL OR l.title ILIKE '%' || $1 || '%' OR l.description ILIKE '%' || $1 || '%')
                AND ($2::text IS NULL OR l.property_type = $2)
                AND ($3::text IS NULL OR l.transaction_type = $3)
                AND ($4::bigint IS NULL OR l.price >= $4)
                AND ($5::bigint IS NULL OR l.price <= $5)
                AND ($6::int IS NULL OR l.size_sqm >= $6)
                AND ($7::int IS NULL OR l.size_sqm <= $7)
                AND ($8::int IS NULL OR l.rooms >= $8)
                AND ($9::int IS NULL OR l.rooms <= $9)
                AND ($10::text IS NULL OR l.city ILIKE '%' || $10 || '%')
                AND ($11::text IS NULL OR l.country = $11)
            ORDER BY l.published_at DESC
            LIMIT 50
            "#,
        )
        .bind(&query.q)
        .bind(&query.property_type)
        .bind(&query.transaction_type)
        .bind(query.price_min)
        .bind(query.price_max)
        .bind(query.area_min)
        .bind(query.area_max)
        .bind(query.rooms_min)
        .bind(query.rooms_max)
        .bind(&query.city)
        .bind(&query.country)
        .bind(since)
        .fetch_all(&self.pool)
        .await?;

        let listings = rows
            .into_iter()
            .map(|r| PublicListingSummary {
                id: r.id,
                title: r.title,
                description: r.description,
                price: r.price.try_into().unwrap_or(0),
                currency: r.currency,
                size_sqm: r.size_sqm.map(|d| d.try_into().unwrap_or(0)),
                rooms: r.rooms,
                city: r.city,
                property_type: r.property_type,
                transaction_type: r.transaction_type,
                photo_url: r.photo_url,
                published_at: r.published_at,
            })
            .collect();

        Ok(listings)
    }

    // ========================================================================
    // Portal Sessions (Security Fix: DB-backed sessions)
    // ========================================================================

    /// Create a new portal session.
    ///
    /// The token_hash should be a SHA-256 hash of the actual session token.
    /// This allows token validation without storing the raw token.
    pub async fn create_session(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: chrono::DateTime<Utc>,
    ) -> Result<crate::models::portal::PortalSession, SqlxError> {
        let session = sqlx::query_as::<_, crate::models::portal::PortalSession>(
            r#"
            INSERT INTO portal_sessions (user_id, token_hash, expires_at)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(session)
    }

    /// Find a session by token hash.
    pub async fn find_session_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<crate::models::portal::PortalSession>, SqlxError> {
        let session = sqlx::query_as::<_, crate::models::portal::PortalSession>(
            r#"SELECT * FROM portal_sessions WHERE token_hash = $1 AND expires_at > NOW()"#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(session)
    }

    /// Get user for a session by token hash.
    ///
    /// Phase 6: reads from `users` instead of `portal_users` (dropped in 00148).
    /// `portal_sessions.user_id` now references `users(id)` after migration 00148.
    pub async fn get_session_user(
        &self,
        token_hash: &str,
    ) -> Result<Option<PortalUser>, SqlxError> {
        let user = sqlx::query_as::<_, PortalUser>(
            r#"
            SELECT
                u.id,
                u.email,
                u.name,
                CASE WHEN u.password_hash = '!sso-only-no-password'
                     THEN NULL
                     ELSE u.password_hash
                END AS password_hash,
                NULL::uuid AS pm_user_id,
                'local' AS provider,
                (u.email_verified_at IS NOT NULL) AS email_verified,
                u.profile_image_url,
                u.locale,
                u.created_at,
                u.updated_at
            FROM users u
            JOIN portal_sessions s ON s.user_id = u.id
            WHERE s.token_hash = $1 AND s.expires_at > NOW()
              AND u.principal_kind = 'public'
              AND u.status != 'deleted'
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// Refresh a session by extending its expiration time.
    pub async fn refresh_session(
        &self,
        token_hash: &str,
        new_expires_at: chrono::DateTime<Utc>,
    ) -> Result<Option<crate::models::portal::PortalSession>, SqlxError> {
        let session = sqlx::query_as::<_, crate::models::portal::PortalSession>(
            r#"
            UPDATE portal_sessions
            SET expires_at = $2
            WHERE token_hash = $1 AND expires_at > NOW()
            RETURNING *
            "#,
        )
        .bind(token_hash)
        .bind(new_expires_at)
        .fetch_optional(&self.pool)
        .await?;

        Ok(session)
    }

    /// Delete a session by token hash.
    pub async fn delete_session(&self, token_hash: &str) -> Result<bool, SqlxError> {
        let result = sqlx::query(r#"DELETE FROM portal_sessions WHERE token_hash = $1"#)
            .bind(token_hash)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete all sessions for a user.
    pub async fn delete_user_sessions(&self, user_id: Uuid) -> Result<u64, SqlxError> {
        let result = sqlx::query(r#"DELETE FROM portal_sessions WHERE user_id = $1"#)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Clean up expired sessions.
    pub async fn cleanup_expired_sessions(&self) -> Result<u64, SqlxError> {
        let result = sqlx::query(r#"DELETE FROM portal_sessions WHERE expires_at < NOW()"#)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Upsert a user from SSO provider (create if not exists, update if exists).
    ///
    /// Phase 6: writes to `users` (principal_kind='public'). The `pm_user_id`
    /// parameter is no longer stored — SSO linkage is now via the PM-side
    /// `users.id` being the same user with `principal_kind='staff'`; the
    /// reality-server `UnifiedPortalUserRepo::sso_upsert` is the authoritative
    /// path. This method is kept for call-site compatibility.
    pub async fn upsert_sso_user(
        &self,
        _pm_user_id: Uuid,
        email: &str,
        name: &str,
        avatar_url: Option<&str>,
    ) -> Result<PortalUser, SqlxError> {
        let user = sqlx::query_as::<_, PortalUser>(
            r#"
            INSERT INTO users (
                email, name, password_hash, locale, status,
                email_verified_at, principal_kind, profile_image_url,
                created_at, updated_at
            )
            VALUES ($1, $2, '!sso-only-no-password', 'en', 'active', NOW(), 'public', $3, NOW(), NOW())
            -- Guard against overwriting a staff/platform principal that
            -- happens to share this email — only update when the existing
            -- row is also `public`. If it's another kind, the CONFLICT
            -- branch silently does no-op (DO UPDATE WHERE false), the
            -- RETURNING is empty, and fetch_one fails — the caller should
            -- treat that as a collision and route to manual review (the
            -- same invariant the merge migration enforces). (Copilot
            -- 3252731628)
            ON CONFLICT (email) DO UPDATE SET
                name              = EXCLUDED.name,
                profile_image_url = COALESCE(EXCLUDED.profile_image_url, users.profile_image_url),
                updated_at        = NOW()
            WHERE users.principal_kind = 'public'
            RETURNING
                id,
                email,
                name,
                NULL::text AS password_hash,
                NULL::uuid AS pm_user_id,
                'local' AS provider,
                -- Derive email_verified from the timestamp the same way the
                -- rest of the repository does, so SSO upserts don't claim
                -- "verified" for rows whose email_verified_at was cleared
                -- elsewhere (review R-followup / Copilot comment 3252615108).
                (email_verified_at IS NOT NULL) AS email_verified,
                profile_image_url,
                locale,
                created_at,
                updated_at
            "#,
        )
        .bind(email)
        .bind(name)
        .bind(avatar_url)
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }
}
