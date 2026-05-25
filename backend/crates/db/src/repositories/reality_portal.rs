//! Reality Portal Professional repository (Epics 31-34).
//!
//! Repository for agencies, realtors, inquiries, and property import.

use crate::models::reality_portal::*;
use crate::DbPool;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::{Error as SqlxError, Row};
use uuid::Uuid;

/// Repository for Reality Portal Professional operations.
#[derive(Clone)]
pub struct RealityPortalRepository {
    pool: DbPool,
}

impl RealityPortalRepository {
    /// Create a new RealityPortalRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
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

    // ========================================================================
    // Reality Agencies (Story 32.1, 32.4)
    // ========================================================================

    /// Create a new agency.
    pub async fn create_agency(
        &self,
        owner_user_id: Uuid,
        data: CreateRealityAgency,
    ) -> Result<RealityAgency, SqlxError> {
        let mut tx = self.pool.begin().await?;

        // Generate slug
        let slug: String = sqlx::query_scalar("SELECT generate_agency_slug($1)")
            .bind(&data.name)
            .fetch_one(&mut *tx)
            .await?;

        // Create agency
        let agency = sqlx::query_as::<_, RealityAgency>(
            r#"
            INSERT INTO reality_agencies (
                name, slug, email, phone, website,
                street, city, postal_code, country,
                description, tagline
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, COALESCE($9, 'SK'), $10, $11)
            RETURNING *
            "#,
        )
        .bind(&data.name)
        .bind(&slug)
        .bind(&data.email)
        .bind(&data.phone)
        .bind(&data.website)
        .bind(&data.street)
        .bind(&data.city)
        .bind(&data.postal_code)
        .bind(&data.country)
        .bind(&data.description)
        .bind(&data.tagline)
        .fetch_one(&mut *tx)
        .await?;

        // Add owner as member
        sqlx::query(
            r#"
            INSERT INTO reality_agency_members (agency_id, user_id, role)
            VALUES ($1, $2, 'owner')
            "#,
        )
        .bind(agency.id)
        .bind(owner_user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(agency)
    }

    /// Get agency by ID.
    pub async fn get_agency(&self, id: Uuid) -> Result<Option<RealityAgency>, SqlxError> {
        sqlx::query_as::<_, RealityAgency>("SELECT * FROM reality_agencies WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// List public agencies (verified status only). Used by the public
    /// directory surface in `reality-web` and the KMP mobile clients.
    pub async fn list_public_agencies(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<RealityAgency>, i64), SqlxError> {
        let agencies = sqlx::query_as::<_, RealityAgency>(
            r#"
            SELECT * FROM reality_agencies
            WHERE status = 'verified'
            ORDER BY name ASC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM reality_agencies WHERE status = 'verified'")
                .fetch_one(&self.pool)
                .await?;

        Ok((agencies, total))
    }

    /// Get agency by slug.
    pub async fn get_agency_by_slug(&self, slug: &str) -> Result<Option<RealityAgency>, SqlxError> {
        sqlx::query_as::<_, RealityAgency>("SELECT * FROM reality_agencies WHERE slug = $1")
            .bind(slug)
            .fetch_optional(&self.pool)
            .await
    }

    /// Update agency details.
    pub async fn update_agency(
        &self,
        id: Uuid,
        data: UpdateRealityAgency,
    ) -> Result<RealityAgency, SqlxError> {
        sqlx::query_as::<_, RealityAgency>(
            r#"
            UPDATE reality_agencies SET
                name = COALESCE($2, name),
                email = COALESCE($3, email),
                phone = COALESCE($4, phone),
                website = COALESCE($5, website),
                street = COALESCE($6, street),
                city = COALESCE($7, city),
                postal_code = COALESCE($8, postal_code),
                description = COALESCE($9, description),
                tagline = COALESCE($10, tagline),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.name)
        .bind(&data.email)
        .bind(&data.phone)
        .bind(&data.website)
        .bind(&data.street)
        .bind(&data.city)
        .bind(&data.postal_code)
        .bind(&data.description)
        .bind(&data.tagline)
        .fetch_one(&self.pool)
        .await
    }

    /// Update agency branding.
    pub async fn update_agency_branding(
        &self,
        id: Uuid,
        data: UpdateAgencyBranding,
    ) -> Result<RealityAgency, SqlxError> {
        sqlx::query_as::<_, RealityAgency>(
            r#"
            UPDATE reality_agencies SET
                logo_url = COALESCE($2, logo_url),
                banner_url = COALESCE($3, banner_url),
                primary_color = COALESCE($4, primary_color),
                secondary_color = COALESCE($5, secondary_color),
                logo_watermark_position = COALESCE($6, logo_watermark_position),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.logo_url)
        .bind(&data.banner_url)
        .bind(&data.primary_color)
        .bind(&data.secondary_color)
        .bind(&data.logo_watermark_position)
        .fetch_one(&self.pool)
        .await
    }

    // ========================================================================
    // Agency Members (Story 32.2)
    // ========================================================================

    /// Get agency members.
    pub async fn get_agency_members(
        &self,
        agency_id: Uuid,
    ) -> Result<Vec<RealityAgencyMember>, SqlxError> {
        sqlx::query_as::<_, RealityAgencyMember>(
            "SELECT * FROM reality_agency_members WHERE agency_id = $1 ORDER BY joined_at",
        )
        .bind(agency_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Create agency invitation.
    pub async fn create_invitation(
        &self,
        agency_id: Uuid,
        invited_by: Uuid,
        data: CreateAgencyInvitation,
    ) -> Result<RealityAgencyInvitation, SqlxError> {
        let token = Uuid::new_v4().to_string();
        let expires_at = Utc::now() + chrono::Duration::days(7);

        sqlx::query_as::<_, RealityAgencyInvitation>(
            r#"
            INSERT INTO reality_agency_invitations (agency_id, email, role, invited_by, token, message, expires_at)
            VALUES ($1, $2, COALESCE($3, 'realtor'), $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(agency_id)
        .bind(&data.email)
        .bind(&data.role)
        .bind(invited_by)
        .bind(&token)
        .bind(&data.message)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await
    }

    /// Accept invitation.
    pub async fn accept_invitation(
        &self,
        token: &str,
        user_id: Uuid,
    ) -> Result<RealityAgencyMember, SqlxError> {
        let mut tx = self.pool.begin().await?;

        // Get invitation
        let invitation = sqlx::query_as::<_, RealityAgencyInvitation>(
            "SELECT * FROM reality_agency_invitations WHERE token = $1 AND accepted_at IS NULL AND expires_at > NOW()",
        )
        .bind(token)
        .fetch_one(&mut *tx)
        .await?;

        // Mark invitation as accepted
        sqlx::query("UPDATE reality_agency_invitations SET accepted_at = NOW() WHERE id = $1")
            .bind(invitation.id)
            .execute(&mut *tx)
            .await?;

        // Add member
        let member = sqlx::query_as::<_, RealityAgencyMember>(
            r#"
            INSERT INTO reality_agency_members (agency_id, user_id, role)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(invitation.agency_id)
        .bind(user_id)
        .bind(&invitation.role)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(member)
    }

    // ========================================================================
    // Realtor Profiles (Story 33.1)
    // ========================================================================

    /// Create or update realtor profile.
    pub async fn upsert_realtor_profile(
        &self,
        user_id: Uuid,
        data: CreateRealtorProfile,
    ) -> Result<RealtorProfile, SqlxError> {
        sqlx::query_as::<_, RealtorProfile>(
            r#"
            INSERT INTO realtor_profiles (
                user_id, bio, tagline, specializations, experience_years,
                languages, license_number, phone, whatsapp, email_public
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (user_id) DO UPDATE SET
                bio = COALESCE($2, realtor_profiles.bio),
                tagline = COALESCE($3, realtor_profiles.tagline),
                specializations = COALESCE($4, realtor_profiles.specializations),
                experience_years = COALESCE($5, realtor_profiles.experience_years),
                languages = COALESCE($6, realtor_profiles.languages),
                license_number = COALESCE($7, realtor_profiles.license_number),
                phone = COALESCE($8, realtor_profiles.phone),
                whatsapp = COALESCE($9, realtor_profiles.whatsapp),
                email_public = COALESCE($10, realtor_profiles.email_public),
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(&data.bio)
        .bind(&data.tagline)
        .bind(&data.specializations)
        .bind(data.experience_years)
        .bind(&data.languages)
        .bind(&data.license_number)
        .bind(&data.phone)
        .bind(&data.whatsapp)
        .bind(&data.email_public)
        .fetch_one(&self.pool)
        .await
    }

    /// Get realtor profile.
    pub async fn get_realtor_profile(
        &self,
        user_id: Uuid,
    ) -> Result<Option<RealtorProfile>, SqlxError> {
        sqlx::query_as::<_, RealtorProfile>("SELECT * FROM realtor_profiles WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Update realtor profile.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_realtor_profile(
        &self,
        user_id: Uuid,
        data: UpdateRealtorProfile,
    ) -> Result<RealtorProfile, SqlxError> {
        sqlx::query_as::<_, RealtorProfile>(
            r#"
            UPDATE realtor_profiles SET
                photo_url = COALESCE($2, photo_url),
                bio = COALESCE($3, bio),
                tagline = COALESCE($4, tagline),
                specializations = COALESCE($5, specializations),
                experience_years = COALESCE($6, experience_years),
                languages = COALESCE($7, languages),
                license_number = COALESCE($8, license_number),
                phone = COALESCE($9, phone),
                whatsapp = COALESCE($10, whatsapp),
                email_public = COALESCE($11, email_public),
                linkedin_url = COALESCE($12, linkedin_url),
                facebook_url = COALESCE($13, facebook_url),
                instagram_url = COALESCE($14, instagram_url),
                show_phone = COALESCE($15, show_phone),
                show_email = COALESCE($16, show_email),
                accept_inquiries = COALESCE($17, accept_inquiries),
                updated_at = NOW()
            WHERE user_id = $1
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(&data.photo_url)
        .bind(&data.bio)
        .bind(&data.tagline)
        .bind(&data.specializations)
        .bind(data.experience_years)
        .bind(&data.languages)
        .bind(&data.license_number)
        .bind(&data.phone)
        .bind(&data.whatsapp)
        .bind(&data.email_public)
        .bind(&data.linkedin_url)
        .bind(&data.facebook_url)
        .bind(&data.instagram_url)
        .bind(data.show_phone)
        .bind(data.show_email)
        .bind(data.accept_inquiries)
        .fetch_one(&self.pool)
        .await
    }

    // ========================================================================
    // Listing Inquiries (Story 33.3)
    // ========================================================================

    /// Create listing inquiry.
    pub async fn create_inquiry(
        &self,
        listing_id: Uuid,
        realtor_id: Uuid,
        user_id: Option<Uuid>,
        data: CreateListingInquiry,
    ) -> Result<ListingInquiry, SqlxError> {
        sqlx::query_as::<_, ListingInquiry>(
            r#"
            INSERT INTO listing_inquiries (
                listing_id, realtor_id, user_id, name, email, phone,
                message, inquiry_type, preferred_contact, preferred_time
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, COALESCE($8, 'info'), COALESCE($9, 'email'), $10)
            RETURNING *
            "#,
        )
        .bind(listing_id)
        .bind(realtor_id)
        .bind(user_id)
        .bind(&data.name)
        .bind(&data.email)
        .bind(&data.phone)
        .bind(&data.message)
        .bind(&data.inquiry_type)
        .bind(&data.preferred_contact)
        .bind(&data.preferred_time)
        .fetch_one(&self.pool)
        .await
    }

    /// Get inquiries for a realtor.
    pub async fn get_realtor_inquiries(
        &self,
        realtor_id: Uuid,
        status: Option<String>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ListingInquiry>, SqlxError> {
        sqlx::query_as::<_, ListingInquiry>(
            r#"
            SELECT * FROM listing_inquiries
            WHERE realtor_id = $1 AND ($2::text IS NULL OR status = $2)
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(realtor_id)
        .bind(&status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Count inquiries for a realtor (matching the same filter as
    /// `get_realtor_inquiries`). Used by paginated routes to expose the
    /// true `total` instead of returning `len()` of the current page.
    pub async fn count_realtor_inquiries(
        &self,
        realtor_id: Uuid,
        status: Option<String>,
    ) -> Result<i64, SqlxError> {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM listing_inquiries
            WHERE realtor_id = $1 AND ($2::text IS NULL OR status = $2)
            "#,
        )
        .bind(realtor_id)
        .bind(&status)
        .fetch_one(&self.pool)
        .await
    }

    /// Get a single inquiry by id, scoped to the owning realtor.
    ///
    /// Single-row query — replaces the previous list-then-filter pattern in
    /// the handler, which silently 404'd any inquiry beyond the first 100
    /// for power users.
    pub async fn get_inquiry_for_realtor(
        &self,
        inquiry_id: Uuid,
        realtor_id: Uuid,
    ) -> Result<Option<ListingInquiry>, SqlxError> {
        sqlx::query_as::<_, ListingInquiry>(
            "SELECT * FROM listing_inquiries WHERE id = $1 AND realtor_id = $2",
        )
        .bind(inquiry_id)
        .bind(realtor_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Mark inquiry as read.
    pub async fn mark_inquiry_read(&self, id: Uuid) -> Result<(), SqlxError> {
        sqlx::query("UPDATE listing_inquiries SET status = 'read', read_at = NOW() WHERE id = $1 AND read_at IS NULL")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Mark an inquiry as read, scoped to the calling realtor.
    ///
    /// Returns `true` when the inquiry belongs to `realtor_id` (idempotent 204);
    /// `false` when not found or owned by another realtor (caller returns 404).
    pub async fn mark_inquiry_read_for_realtor(
        &self,
        id: Uuid,
        realtor_id: Uuid,
    ) -> Result<bool, SqlxError> {
        let owned: bool = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM listing_inquiries WHERE id = $1 AND realtor_id = $2)",
        )
        .bind(id)
        .bind(realtor_id)
        .fetch_one(&self.pool)
        .await?;

        if !owned {
            return Ok(false);
        }

        sqlx::query(
            "UPDATE listing_inquiries SET status = 'read', read_at = NOW() WHERE id = $1 AND read_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(true)
    }

    /// Respond to inquiry.
    ///
    /// Enforces realtor ownership: the calling realtor (`realtor_id`) MUST own
    /// the inquiry (`listing_inquiries.realtor_id = realtor_id`). Returns
    /// `Ok(None)` when the inquiry does not exist or belongs to a different
    /// realtor so the route layer can return 404 (indistinguishable from
    /// "not found" to the caller — no information leakage).
    pub async fn respond_to_inquiry(
        &self,
        id: Uuid,
        realtor_id: Uuid,
        message: &str,
    ) -> Result<Option<InquiryMessage>, SqlxError> {
        let mut tx = self.pool.begin().await?;

        // Ownership check: only the realtor who owns the inquiry may respond.
        // This mirrors the pattern used by `get_inquiry_for_realtor` and
        // `mark_inquiry_read_for_realtor`. Without this guard, realtor B could
        // insert a message on realtor A's inquiry (IDOR).
        let owned: bool = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM listing_inquiries WHERE id = $1 AND realtor_id = $2)",
        )
        .bind(id)
        .bind(realtor_id)
        .fetch_one(&mut *tx)
        .await?;

        if !owned {
            // Rollback (nothing written yet) and signal caller to 404.
            tx.rollback().await?;
            return Ok(None);
        }

        // Create message
        let msg = sqlx::query_as::<_, InquiryMessage>(
            r#"
            INSERT INTO inquiry_messages (inquiry_id, sender_type, sender_id, message)
            VALUES ($1, 'realtor', $2, $3)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(realtor_id)
        .bind(message)
        .fetch_one(&mut *tx)
        .await?;

        // Update inquiry status (ownership already verified above)
        sqlx::query(
            "UPDATE listing_inquiries SET status = 'responded', responded_at = NOW() WHERE id = $1 AND realtor_id = $2",
        )
        .bind(id)
        .bind(realtor_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(Some(msg))
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

    // ========================================================================
    // Import Jobs (Story 34.1)
    // ========================================================================

    /// Create import job.
    pub async fn create_import_job(
        &self,
        user_id: Uuid,
        data: CreateImportJob,
    ) -> Result<PortalImportJob, SqlxError> {
        sqlx::query_as::<_, PortalImportJob>(
            r#"
            INSERT INTO portal_import_jobs (user_id, agency_id, source_type, source_url, source_filename)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(data.agency_id)
        .bind(&data.source_type)
        .bind(&data.source_url)
        .bind(&data.source_filename)
        .fetch_one(&self.pool)
        .await
    }

    /// List import jobs for a user.
    pub async fn list_import_jobs(
        &self,
        user_id: Uuid,
        status: Option<String>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<PortalImportJobWithStats>, SqlxError> {
        sqlx::query_as::<_, PortalImportJobWithStats>(
            r#"
            SELECT
                j.id,
                j.source_type,
                j.source_url,
                j.source_filename,
                j.status,
                j.total_records,
                j.processed_records,
                j.success_count,
                j.failure_count,
                j.started_at,
                j.completed_at,
                j.created_at
            FROM portal_import_jobs j
            WHERE j.user_id = $1 AND ($2::text IS NULL OR j.status = $2)
            ORDER BY j.created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(user_id)
        .bind(&status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Update import job.
    pub async fn update_import_job(
        &self,
        id: Uuid,
        data: UpdateImportJob,
    ) -> Result<PortalImportJob, SqlxError> {
        sqlx::query_as::<_, PortalImportJob>(
            r#"
            UPDATE portal_import_jobs SET
                source_url = COALESCE($2, source_url),
                source_filename = COALESCE($3, source_filename)
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.source_url)
        .bind(&data.source_filename)
        .fetch_one(&self.pool)
        .await
    }

    /// Start import job.
    pub async fn start_import_job(&self, id: Uuid) -> Result<PortalImportJob, SqlxError> {
        sqlx::query_as::<_, PortalImportJob>(
            r#"
            UPDATE portal_import_jobs SET
                status = 'processing',
                started_at = NOW()
            WHERE id = $1 AND status = 'pending'
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
    }

    /// Cancel import job.
    pub async fn cancel_import_job(&self, id: Uuid) -> Result<PortalImportJob, SqlxError> {
        sqlx::query_as::<_, PortalImportJob>(
            r#"
            UPDATE portal_import_jobs SET
                status = 'cancelled',
                completed_at = NOW()
            WHERE id = $1 AND status IN ('pending', 'processing')
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
    }

    /// Get import job status.
    pub async fn get_import_job(&self, id: Uuid) -> Result<Option<PortalImportJob>, SqlxError> {
        sqlx::query_as::<_, PortalImportJob>("SELECT * FROM portal_import_jobs WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Update import job progress.
    pub async fn update_import_progress(
        &self,
        id: Uuid,
        processed: i32,
        success: i32,
        failure: i32,
    ) -> Result<(), SqlxError> {
        sqlx::query(
            r#"
            UPDATE portal_import_jobs SET
                processed_records = $2,
                success_count = $3,
                failure_count = $4
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(processed)
        .bind(success)
        .bind(failure)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ========================================================================
    // Feed Subscriptions (Story 34.2)
    // ========================================================================

    /// List feed subscriptions for an agency.
    pub async fn list_feed_subscriptions(
        &self,
        agency_id: Uuid,
    ) -> Result<Vec<RealityFeedSubscription>, SqlxError> {
        sqlx::query_as::<_, RealityFeedSubscription>(
            "SELECT * FROM feed_subscriptions WHERE agency_id = $1 ORDER BY created_at DESC",
        )
        .bind(agency_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Create feed subscription.
    pub async fn create_feed_subscription(
        &self,
        agency_id: Uuid,
        data: CreateFeedSubscription,
    ) -> Result<RealityFeedSubscription, SqlxError> {
        sqlx::query_as::<_, RealityFeedSubscription>(
            r#"
            INSERT INTO feed_subscriptions (agency_id, name, feed_url, feed_type, sync_interval)
            VALUES ($1, $2, $3, COALESCE($4, 'xml'), COALESCE($5, 'daily'))
            RETURNING *
            "#,
        )
        .bind(agency_id)
        .bind(&data.name)
        .bind(&data.feed_url)
        .bind(&data.feed_type)
        .bind(&data.sync_interval)
        .fetch_one(&self.pool)
        .await
    }

    /// Get feed subscription by ID.
    pub async fn get_feed_subscription(
        &self,
        id: Uuid,
    ) -> Result<Option<RealityFeedSubscription>, SqlxError> {
        sqlx::query_as::<_, RealityFeedSubscription>(
            "SELECT * FROM feed_subscriptions WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Update feed subscription.
    pub async fn update_feed_subscription(
        &self,
        id: Uuid,
        data: UpdateFeedSubscription,
    ) -> Result<RealityFeedSubscription, SqlxError> {
        sqlx::query_as::<_, RealityFeedSubscription>(
            r#"
            UPDATE feed_subscriptions SET
                name = COALESCE($2, name),
                feed_url = COALESCE($3, feed_url),
                feed_type = COALESCE($4, feed_type),
                sync_interval = COALESCE($5, sync_interval),
                is_active = COALESCE($6, is_active),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.name)
        .bind(&data.feed_url)
        .bind(&data.feed_type)
        .bind(&data.sync_interval)
        .bind(data.is_active)
        .fetch_one(&self.pool)
        .await
    }

    /// Trigger immediate feed sync.
    pub async fn trigger_feed_sync(&self, id: Uuid) -> Result<RealityFeedSubscription, SqlxError> {
        // Mark as syncing and update last sync time
        sqlx::query_as::<_, RealityFeedSubscription>(
            r#"
            UPDATE feed_subscriptions SET
                last_sync_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
    }
}
