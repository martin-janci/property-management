//! Reality agencies: CRUD, branding, members and invitations (Story 32.x).

use super::RealityPortalRepository;
use crate::models::reality_portal::*;
use chrono::Utc;
use sqlx::Error as SqlxError;
use uuid::Uuid;

impl RealityPortalRepository {
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

    /// Get the agency the given portal user belongs to.
    ///
    /// Resolves the agency via the caller's `reality_agency_members` row,
    /// picking the earliest-joined membership when a user belongs to more
    /// than one. Returns `None` when the user is not a member of any agency
    /// so the caller can answer `404` (used by `GET /api/v1/agencies/me`).
    pub async fn get_agency_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Option<RealityAgency>, SqlxError> {
        sqlx::query_as::<_, RealityAgency>(
            r#"
            SELECT a.*
            FROM reality_agencies a
            JOIN reality_agency_members m ON m.agency_id = a.id
            WHERE m.user_id = $1
            ORDER BY m.joined_at ASC
            LIMIT 1
            "#,
        )
        .bind(user_id)
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
}
