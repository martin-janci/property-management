//! Portal listing CRUD, view tracking and analytics (Epic 15, Story 33.4).

use super::RealityPortalRepository;
use crate::models::reality_portal::*;
use crate::models::Listing;
use chrono::NaiveDate;
use sqlx::Error as SqlxError;
use uuid::Uuid;

impl RealityPortalRepository {
    // ========================================================================
    // Portal Listing CRUD (Epic 15.1/15.2 — owner/realtor edit flow)
    // ========================================================================

    /// Create a portal-user-owned listing (no PPT org required).
    ///
    /// Uses the SECURITY DEFINER `portal_create_listing` function (migration 00186)
    /// so the operation bypasses the org-scoped RLS on the `listings` table.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_portal_listing(
        &self,
        user_id: Uuid,
        title: &str,
        description: Option<&str>,
        property_type: &str,
        transaction_type: &str,
        price: rust_decimal::Decimal,
        currency: &str,
        street: &str,
        city: &str,
        postal_code: &str,
        country: &str,
        size_sqm: Option<rust_decimal::Decimal>,
        rooms: Option<i32>,
        floor: Option<i32>,
        total_floors: Option<i32>,
    ) -> Result<Listing, SqlxError> {
        sqlx::query_as::<_, Listing>(
            r#"SELECT * FROM portal_create_listing($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)"#,
        )
        .bind(user_id)
        .bind(title)
        .bind(description)
        .bind(property_type)
        .bind(transaction_type)
        .bind(price)
        .bind(currency)
        .bind(street)
        .bind(city)
        .bind(postal_code)
        .bind(country)
        .bind(size_sqm)
        .bind(rooms)
        .bind(floor)
        .bind(total_floors)
        .fetch_one(&self.pool)
        .await
    }

    /// Get a portal-user-owned listing for editing.
    ///
    /// Uses SECURITY DEFINER `portal_get_listing` (migration 00186); ownership
    /// checked via `portal_owner_id = user_id OR created_by = user_id`.
    pub async fn get_portal_listing(
        &self,
        listing_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Listing>, SqlxError> {
        sqlx::query_as::<_, Listing>(r#"SELECT * FROM portal_get_listing($1, $2)"#)
            .bind(listing_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Patch a portal-user-owned listing. None fields are left unchanged.
    ///
    /// Uses SECURITY DEFINER `portal_update_listing` (migration 00186); ownership
    /// is enforced inside the function. Returns None when not found / not owned.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_portal_listing(
        &self,
        listing_id: Uuid,
        user_id: Uuid,
        title: Option<&str>,
        description: Option<&str>,
        property_type: Option<&str>,
        transaction_type: Option<&str>,
        price: Option<rust_decimal::Decimal>,
        currency: Option<&str>,
        street: Option<&str>,
        city: Option<&str>,
        postal_code: Option<&str>,
        country: Option<&str>,
        size_sqm: Option<rust_decimal::Decimal>,
        rooms: Option<i32>,
        floor: Option<i32>,
        total_floors: Option<i32>,
        status: Option<&str>,
        is_negotiable: Option<bool>,
    ) -> Result<Option<Listing>, SqlxError> {
        sqlx::query_as::<_, Listing>(
            r#"SELECT * FROM portal_update_listing($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)"#,
        )
        .bind(listing_id)
        .bind(user_id)
        .bind(title)
        .bind(description)
        .bind(property_type)
        .bind(transaction_type)
        .bind(price)
        .bind(currency)
        .bind(street)
        .bind(city)
        .bind(postal_code)
        .bind(country)
        .bind(size_sqm)
        .bind(rooms)
        .bind(floor)
        .bind(total_floors)
        .bind(status)
        .bind(is_negotiable)
        .fetch_optional(&self.pool)
        .await
    }

    /// List a portal user's own listings (Epic 15 — realtor "my listings").
    ///
    /// The `listings` table is `FORCE ROW LEVEL SECURITY` (migration 00110), so
    /// unlike the sibling CRUD helpers — which use `SECURITY DEFINER` functions
    /// to bypass RLS — this method establishes the **portal-owner** RLS context
    /// (context 5 of `listings_five_context`, migration 00186:
    /// `portal_owner_id = get_current_portal_user_id()`) before selecting.
    ///
    /// The context is set with `set_config(..., is_local => true)` so it is
    /// scoped to the transaction and auto-cleared on commit — no risk of RLS
    /// context bleeding onto the pooled connection (the leak class the
    /// `tenant_context` helpers guard against). The explicit
    /// `portal_owner_id = $1` predicate is defense-in-depth so the query stays
    /// correct even for a DB role that bypasses RLS.
    pub async fn list_portal_listings(
        &self,
        user_id: Uuid,
        status: Option<&str>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<Listing>, SqlxError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT set_config('app.current_user_id', $1, true)")
            .bind(user_id.to_string())
            .execute(&mut *tx)
            .await?;
        let listings = sqlx::query_as::<_, Listing>(
            r#"
            SELECT * FROM listings
            WHERE portal_owner_id = $1
              AND ($2::text IS NULL OR status = $2)
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(user_id)
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(listings)
    }

    /// Count a portal user's own listings, matching the same filter as
    /// [`list_portal_listings`](Self::list_portal_listings). Lets paginated
    /// routes surface the true `total` instead of the current page's `len()`.
    pub async fn count_portal_listings(
        &self,
        user_id: Uuid,
        status: Option<&str>,
    ) -> Result<i64, SqlxError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT set_config('app.current_user_id', $1, true)")
            .bind(user_id.to_string())
            .execute(&mut *tx)
            .await?;
        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM listings
            WHERE portal_owner_id = $1
              AND ($2::text IS NULL OR status = $2)
            "#,
        )
        .bind(user_id)
        .bind(status)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(total)
    }

    // ========================================================================
    // Listing Analytics (Story 33.4)
    // ========================================================================

    /// Track a listing view (increment daily analytics counters).
    ///
    /// Routes through the SECURITY DEFINER `portal_track_listing_view` function
    /// (migration 00214), **not** the plain `track_listing_view` (00063). The
    /// latter is SECURITY INVOKER and its `INSERT … ON CONFLICT` into
    /// `listing_analytics` — a `FORCE ROW LEVEL SECURITY` table with an org-only
    /// policy and no portal-owner branch — fails the `WITH CHECK` for portal
    /// listings (`organization_id IS NULL`) on the RLS-subject reality-server
    /// pool, so no view was ever recorded (#2244). The definer function bypasses
    /// that policy while only ever touching the viewed listing's aggregate
    /// counters. View tracking is public, so there is no ownership gate.
    pub async fn track_view(&self, listing_id: Uuid, source: &str) -> Result<(), SqlxError> {
        sqlx::query("SELECT portal_track_listing_view($1, $2)")
            .bind(listing_id)
            .bind(source)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get the aggregate view total for a listing (public read).
    ///
    /// Routes through the SECURITY DEFINER `portal_get_listing_view_count`
    /// function (migration 00220), **not** a direct `SELECT SUM(views)`. A plain
    /// select on the RLS-subject reality-server pool reads back zero for
    /// portal-owned listings (`organization_id IS NULL`): `listing_analytics` is
    /// `FORCE ROW LEVEL SECURITY` (migration 00113) with an org-only policy and
    /// no portal-owner branch, and the public pool has no org context — the same
    /// RLS trap that broke the write path (#2244) and the per-owner read (#2199).
    /// The definer function bypasses that policy and only ever aggregates the
    /// requested listing's `views` counter. There is no ownership gate — a public
    /// listing's total view count is itself public (read-side counterpart of
    /// [`track_view`](Self::track_view)).
    pub async fn get_view_count(&self, listing_id: Uuid) -> Result<i64, SqlxError> {
        sqlx::query_scalar::<_, i64>("SELECT portal_get_listing_view_count($1)")
            .bind(listing_id)
            .fetch_one(&self.pool)
            .await
    }

    /// Get listing analytics (org-scoped path).
    ///
    /// This runs a direct `SELECT` on the RLS-subject pool, so it only returns
    /// rows the current org RLS context admits. It is **not** suitable for
    /// portal-owned listings (`organization_id IS NULL`): `listing_analytics`
    /// is `FORCE ROW LEVEL SECURITY` (migration 00113) with an org-only policy
    /// and no portal-owner branch, so this method reads back an empty series for
    /// them. Portal routes must use
    /// [`get_portal_listing_analytics`](Self::get_portal_listing_analytics)
    /// instead (see #2199).
    pub async fn get_listing_analytics(
        &self,
        listing_id: Uuid,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
    ) -> Result<Vec<ListingAnalytics>, SqlxError> {
        sqlx::query_as::<_, ListingAnalytics>(
            r#"
            SELECT * FROM listing_analytics
            WHERE listing_id = $1
              AND ($2::date IS NULL OR date >= $2)
              AND ($3::date IS NULL OR date <= $3)
            ORDER BY date DESC
            "#,
        )
        .bind(listing_id)
        .bind(from_date)
        .bind(to_date)
        .fetch_all(&self.pool)
        .await
    }

    /// Get analytics for a **portal-user-owned** listing.
    ///
    /// Uses the SECURITY DEFINER `portal_get_listing_analytics` function
    /// (migration 00213), ownership-gated on
    /// `portal_owner_id = user_id OR created_by = user_id` — mirroring
    /// [`get_portal_listing`](Self::get_portal_listing). Unlike
    /// [`get_listing_analytics`](Self::get_listing_analytics), it bypasses the
    /// org-scoped `listing_analytics` RLS policy (which has no portal-owner
    /// branch), so portal listings return their real recorded analytics instead
    /// of an empty series (#2199). Returns an empty vec when the listing is not
    /// owned by `user_id`.
    pub async fn get_portal_listing_analytics(
        &self,
        listing_id: Uuid,
        user_id: Uuid,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
    ) -> Result<Vec<ListingAnalytics>, SqlxError> {
        sqlx::query_as::<_, ListingAnalytics>(
            r#"SELECT * FROM portal_get_listing_analytics($1, $2, $3, $4)"#,
        )
        .bind(listing_id)
        .bind(user_id)
        .bind(from_date)
        .bind(to_date)
        .fetch_all(&self.pool)
        .await
    }
}
