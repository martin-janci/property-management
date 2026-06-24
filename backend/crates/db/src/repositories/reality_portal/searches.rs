//! Saved-search alert engine, favorites and saved searches (Stories 16.3, 31.1-31.4).

use super::RealityPortalRepository;
use crate::models::reality_portal::*;
use crate::models::PublicListingQuery;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{Error as SqlxError, Executor, Postgres, Row};
use uuid::Uuid;

impl RealityPortalRepository {
    // ========================================================================
    // Saved-search alert engine (Story 16.3, issue #983)
    //
    // These run on a caller-supplied connection so the background worker can
    // set the global-read RLS context (`set_global_read_context`) once and
    // read published listings across all orgs on the same connection.
    // `portal_saved_searches` and `search_alert_queue` are not RLS-gated.
    // ========================================================================

    /// All alert-enabled saved searches, oldest-checked first.
    pub async fn list_alertable_saved_searches<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<PortalSavedSearch>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, PortalSavedSearch>(
            r#"
            SELECT * FROM portal_saved_searches
            WHERE alerts_enabled = true
            ORDER BY last_matched_at ASC NULLS FIRST
            LIMIT 5000
            "#,
        )
        .fetch_all(executor)
        .await
    }

    /// IDs of published listings matching a saved search's criteria, optionally
    /// only those published after `since`. Mirrors the public `search_listings`
    /// filter set so on-demand and alert matching agree. Requires the executor's
    /// connection to be in global-read context to see cross-org published rows.
    pub async fn find_new_match_listing_ids<'e, E>(
        &self,
        executor: E,
        q: &PublicListingQuery,
        since: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<Uuid>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT l.id
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
                AND ($12::timestamptz IS NULL OR l.published_at > $12)
            ORDER BY l.published_at DESC
            LIMIT $13
            "#,
        )
        .bind(&q.q)
        .bind(&q.property_type)
        .bind(&q.transaction_type)
        .bind(q.price_min)
        .bind(q.price_max)
        .bind(q.area_min)
        .bind(q.area_max)
        .bind(q.rooms_min)
        .bind(q.rooms_max)
        .bind(&q.city)
        .bind(&q.country)
        .bind(since)
        .bind(limit)
        .fetch_all(executor)
        .await
    }

    /// Enqueue a pending alert for later delivery.
    pub async fn enqueue_search_alert<'e, E>(
        &self,
        executor: E,
        saved_search_id: Uuid,
        user_id: Uuid,
        listing_ids: &[Uuid],
        alert_type: &str,
    ) -> Result<(), SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query(
            r#"
            INSERT INTO search_alert_queue
                (saved_search_id, user_id, matching_listing_ids, alert_type, status)
            VALUES ($1, $2, $3, $4, 'pending')
            "#,
        )
        .bind(saved_search_id)
        .bind(user_id)
        .bind(listing_ids)
        .bind(alert_type)
        .execute(executor)
        .await?;
        Ok(())
    }

    /// Advance a saved search's match watermark (`last_matched_at = now()`) and
    /// add `new_matches` to its running `match_count`.
    pub async fn mark_saved_search_matched<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        new_matches: i64,
    ) -> Result<(), SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query(
            r#"
            UPDATE portal_saved_searches
            SET last_matched_at = now(),
                match_count = match_count + $2,
                updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(new_matches as i32)
        .execute(executor)
        .await?;
        Ok(())
    }

    // ------------------------------------------------------------------------
    // Saved-search alert delivery (in-app feed for the matching engine above).
    //
    // `search_alert_queue` is app-scoped by `user_id` (not RLS-gated), so these
    // run on the request-path pool and scope every statement by the owning user
    // — a portal user can only read/ack their own alerts (no IDOR).
    // ------------------------------------------------------------------------

    /// A portal user's saved-search alerts, newest first, each joined to the
    /// originating saved search's name.
    pub async fn get_search_alerts(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<SavedSearchAlert>, SqlxError> {
        sqlx::query_as::<_, SavedSearchAlert>(
            r#"
            SELECT
                q.id,
                q.saved_search_id,
                s.name AS saved_search_name,
                q.matching_listing_ids,
                q.alert_type,
                q.status,
                q.created_at,
                q.processed_at
            FROM search_alert_queue q
            JOIN portal_saved_searches s ON s.id = q.saved_search_id
            WHERE q.user_id = $1
            ORDER BY q.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Count a user's undelivered (`pending`) alerts — for an unread badge.
    pub async fn count_pending_search_alerts(&self, user_id: Uuid) -> Result<i64, SqlxError> {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM search_alert_queue WHERE user_id = $1 AND status = 'pending'",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Mark one alert delivered (`pending` → `sent`), scoped to the owner.
    /// Returns `true` when a row was updated (owned and still pending); `false`
    /// when not found / owned by another user — the route returns 404, so a
    /// cross-user id is indistinguishable from "not found" (no IDOR).
    pub async fn mark_search_alert_read(&self, id: Uuid, user_id: Uuid) -> Result<bool, SqlxError> {
        let res = sqlx::query(
            r#"
            UPDATE search_alert_queue
            SET status = 'sent', processed_at = NOW()
            WHERE id = $1 AND user_id = $2 AND status = 'pending'
            "#,
        )
        .bind(id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }

    /// Mark all of a user's pending alerts delivered. Returns the count updated.
    pub async fn mark_all_search_alerts_read(&self, user_id: Uuid) -> Result<u64, SqlxError> {
        let res = sqlx::query(
            r#"
            UPDATE search_alert_queue
            SET status = 'sent', processed_at = NOW()
            WHERE user_id = $1 AND status = 'pending'
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected())
    }

    // ========================================================================
    // Portal Favorites (Story 31.1, 31.4)
    // ========================================================================

    /// Add listing to favorites with price tracking.
    pub async fn add_favorite(
        &self,
        user_id: Uuid,
        listing_id: Uuid,
        notes: Option<String>,
    ) -> Result<PortalFavorite, SqlxError> {
        sqlx::query_as::<_, PortalFavorite>(
            r#"
            INSERT INTO portal_favorites (user_id, listing_id, notes, original_price)
            SELECT $1, $2, $3, l.price
            FROM listings l WHERE l.id = $2
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(listing_id)
        .bind(&notes)
        .fetch_one(&self.pool)
        .await
    }

    /// Get favorites with listing details and price change info.
    pub async fn get_favorites_with_listings(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<PortalFavoriteWithListing>, SqlxError> {
        let rows = sqlx::query(
            r#"
            SELECT
                pf.id,
                pf.listing_id,
                l.title,
                l.price as current_price,
                pf.original_price,
                l.currency,
                l.city,
                l.property_type,
                l.transaction_type,
                (SELECT url FROM listing_photos lp WHERE lp.listing_id = l.id ORDER BY display_order LIMIT 1) as photo_url,
                l.status,
                pf.price_alert_enabled,
                pf.created_at
            FROM portal_favorites pf
            JOIN listings l ON l.id = pf.listing_id
            WHERE pf.user_id = $1
            ORDER BY pf.created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let favorites = rows
            .iter()
            .map(|row| {
                let current_price: Decimal = row.get("current_price");
                let original_price: Option<Decimal> = row.get("original_price");
                let price_changed = original_price
                    .map(|op| op != current_price)
                    .unwrap_or(false);
                let price_change_percentage = original_price.and_then(|op| {
                    if op != Decimal::ZERO && price_changed {
                        Some(((current_price - op) / op * Decimal::from(100)).round_dp(2))
                    } else {
                        None
                    }
                });

                PortalFavoriteWithListing {
                    id: row.get("id"),
                    listing_id: row.get("listing_id"),
                    title: row.get("title"),
                    current_price,
                    original_price,
                    currency: row.get("currency"),
                    city: row.get("city"),
                    property_type: row.get("property_type"),
                    transaction_type: row.get("transaction_type"),
                    photo_url: row.get("photo_url"),
                    status: row.get("status"),
                    price_changed,
                    price_change_percentage,
                    price_alert_enabled: row.get("price_alert_enabled"),
                    created_at: row.get("created_at"),
                }
            })
            .collect();

        Ok(favorites)
    }

    /// Update favorite (notes, price alert settings).
    pub async fn update_favorite(
        &self,
        user_id: Uuid,
        listing_id: Uuid,
        data: UpdatePortalFavorite,
    ) -> Result<PortalFavorite, SqlxError> {
        sqlx::query_as::<_, PortalFavorite>(
            r#"
            UPDATE portal_favorites SET
                notes = COALESCE($3, notes),
                price_alert_enabled = COALESCE($4, price_alert_enabled)
            WHERE user_id = $1 AND listing_id = $2
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(listing_id)
        .bind(&data.notes)
        .bind(data.price_alert_enabled)
        .fetch_one(&self.pool)
        .await
    }

    /// Check whether a user has a given listing in their favorites.
    ///
    /// Single-row existence query — avoids fetching the entire favorites
    /// collection just to test membership (functional bug at large N where
    /// list endpoints cap results).
    pub async fn is_favorite(&self, user_id: Uuid, listing_id: Uuid) -> Result<bool, SqlxError> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM portal_favorites WHERE user_id = $1 AND listing_id = $2)",
        )
        .bind(user_id)
        .bind(listing_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(exists)
    }

    /// Remove listing from favorites.
    pub async fn remove_favorite(
        &self,
        user_id: Uuid,
        listing_id: Uuid,
    ) -> Result<bool, SqlxError> {
        let result =
            sqlx::query("DELETE FROM portal_favorites WHERE user_id = $1 AND listing_id = $2")
                .bind(user_id)
                .bind(listing_id)
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get price change alerts for favorites.
    pub async fn get_price_change_alerts(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<PriceChangeAlert>, SqlxError> {
        sqlx::query_as::<_, PriceChangeAlert>(
            r#"
            SELECT
                l.id as listing_id,
                l.title,
                lph.old_price,
                lph.new_price,
                lph.currency,
                lph.change_percentage,
                lph.changed_at
            FROM portal_favorites pf
            JOIN listings l ON l.id = pf.listing_id
            JOIN listing_price_history lph ON lph.listing_id = l.id
            WHERE pf.user_id = $1
              AND pf.price_alert_enabled = true
              AND lph.changed_at > COALESCE(pf.last_price_alert_at, pf.created_at)
            ORDER BY lph.changed_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    // ========================================================================
    // Portal Saved Searches (Story 31.2, 31.3)
    // ========================================================================

    /// Create a saved search.
    pub async fn create_saved_search(
        &self,
        user_id: Uuid,
        data: CreatePortalSavedSearch,
    ) -> Result<PortalSavedSearch, SqlxError> {
        sqlx::query_as::<_, PortalSavedSearch>(
            r#"
            INSERT INTO portal_saved_searches (user_id, name, criteria, alerts_enabled, alert_frequency)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(&data.name)
        .bind(&data.criteria)
        .bind(data.alerts_enabled)
        .bind(&data.alert_frequency)
        .fetch_one(&self.pool)
        .await
    }

    /// Get saved searches for a user.
    pub async fn get_saved_searches(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<PortalSavedSearch>, SqlxError> {
        sqlx::query_as::<_, PortalSavedSearch>(
            "SELECT * FROM portal_saved_searches WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Get a single saved search by id, scoped to the owning user.
    ///
    /// Single-row query — replaces the previous list-then-filter pattern in
    /// the handlers, which silently 404'd whenever a search wasn't in the
    /// first page of `get_saved_searches`.
    pub async fn get_saved_search_for_user(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<PortalSavedSearch>, SqlxError> {
        sqlx::query_as::<_, PortalSavedSearch>(
            "SELECT * FROM portal_saved_searches WHERE id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Update a saved search.
    pub async fn update_saved_search(
        &self,
        id: Uuid,
        user_id: Uuid,
        data: UpdatePortalSavedSearch,
    ) -> Result<PortalSavedSearch, SqlxError> {
        sqlx::query_as::<_, PortalSavedSearch>(
            r#"
            UPDATE portal_saved_searches SET
                name = COALESCE($3, name),
                criteria = COALESCE($4, criteria),
                alerts_enabled = COALESCE($5, alerts_enabled),
                alert_frequency = COALESCE($6, alert_frequency),
                updated_at = NOW()
            WHERE id = $1 AND user_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(&data.name)
        .bind(&data.criteria)
        .bind(data.alerts_enabled)
        .bind(&data.alert_frequency)
        .fetch_one(&self.pool)
        .await
    }

    /// Delete a saved search.
    pub async fn delete_saved_search(&self, id: Uuid, user_id: Uuid) -> Result<bool, SqlxError> {
        let result =
            sqlx::query("DELETE FROM portal_saved_searches WHERE id = $1 AND user_id = $2")
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected() > 0)
    }
}
