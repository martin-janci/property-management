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

    // ========================================================================
    // Listing Analytics (Story 33.4)
    // ========================================================================

    /// Track listing view.
    pub async fn track_view(&self, listing_id: Uuid, source: &str) -> Result<(), SqlxError> {
        sqlx::query("SELECT track_listing_view($1, $2)")
            .bind(listing_id)
            .bind(source)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get listing analytics.
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
}
