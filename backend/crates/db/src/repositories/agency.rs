//! Agency repository (Epic 17: Agency & Realtor Management).

use crate::models::agency::{
    agency_status, member_role, Agency, AgencyBranding, AgencyInvitation, AgencyListing,
    AgencyMember, AgencyMemberWithUser, AgencyMemberWithUserRow, AgencyProfile, AgencySummary,
    CreateAgency, CreateAgencyListing, InviteMember, ListingEditHistory, ListingImportJob,
    UpdateAgency, UpdateMemberRole,
};
use crate::DbPool;
use chrono::{Duration, Utc};
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Repository for agency operations.
#[derive(Clone)]
pub struct AgencyRepository {
    pool: DbPool,
}

impl AgencyRepository {
    /// Create a new AgencyRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // Agency CRUD (Story 17.1)
    // ========================================================================

    /// Create a new agency.
    pub async fn create_agency(
        &self,
        data: CreateAgency,
        owner_id: Uuid,
    ) -> Result<Agency, SqlxError> {
        // Start transaction
        let mut tx = self.pool.begin().await?;

        // Create agency
        let agency = sqlx::query_as::<_, Agency>(
            r#"
            INSERT INTO agencies (name, slug, email, address, city, postal_code, country, phone, website, status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(&data.name)
        .bind(&data.slug)
        .bind(&data.email)
        .bind(&data.address)
        .bind(&data.city)
        .bind(&data.postal_code)
        .bind(&data.country)
        .bind(&data.phone)
        .bind(&data.website)
        .bind(agency_status::PENDING)
        .fetch_one(&mut *tx)
        .await?;

        // Add owner as admin
        sqlx::query(
            r#"
            INSERT INTO agency_members (agency_id, user_id, role, is_active)
            VALUES ($1, $2, $3, true)
            "#,
        )
        .bind(agency.id)
        .bind(owner_id)
        .bind(member_role::ADMIN)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(agency)
    }

    /// Find agency by ID.
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Agency>, SqlxError> {
        let agency = sqlx::query_as::<_, Agency>(r#"SELECT * FROM agencies WHERE id = $1"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(agency)
    }

    /// Find agency by slug.
    pub async fn find_by_slug(&self, slug: &str) -> Result<Option<Agency>, SqlxError> {
        let agency = sqlx::query_as::<_, Agency>(r#"SELECT * FROM agencies WHERE slug = $1"#)
            .bind(slug)
            .fetch_optional(&self.pool)
            .await?;

        Ok(agency)
    }

    /// Update agency.
    pub async fn update_agency(&self, id: Uuid, data: UpdateAgency) -> Result<Agency, SqlxError> {
        let agency = sqlx::query_as::<_, Agency>(
            r#"
            UPDATE agencies SET
                name = COALESCE($2, name),
                address = COALESCE($3, address),
                city = COALESCE($4, city),
                postal_code = COALESCE($5, postal_code),
                country = COALESCE($6, country),
                phone = COALESCE($7, phone),
                email = COALESCE($8, email),
                website = COALESCE($9, website),
                logo_url = COALESCE($10, logo_url),
                primary_color = COALESCE($11, primary_color),
                description = COALESCE($12, description),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.name)
        .bind(&data.address)
        .bind(&data.city)
        .bind(&data.postal_code)
        .bind(&data.country)
        .bind(&data.phone)
        .bind(&data.email)
        .bind(&data.website)
        .bind(&data.logo_url)
        .bind(&data.primary_color)
        .bind(&data.description)
        .fetch_one(&self.pool)
        .await?;

        Ok(agency)
    }

    /// Update agency branding.
    pub async fn update_branding(
        &self,
        id: Uuid,
        branding: AgencyBranding,
    ) -> Result<Agency, SqlxError> {
        let agency = sqlx::query_as::<_, Agency>(
            r#"
            UPDATE agencies SET
                logo_url = COALESCE($2, logo_url),
                primary_color = COALESCE($3, primary_color),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&branding.logo_url)
        .bind(&branding.primary_color)
        .fetch_one(&self.pool)
        .await?;

        Ok(agency)
    }

    /// Get agency public profile.
    pub async fn get_profile(&self, slug: &str) -> Result<Option<AgencyProfile>, SqlxError> {
        let agency = self.find_by_slug(slug).await?;
        if agency.is_none() {
            return Ok(None);
        }
        let agency = agency.unwrap();

        // Get member count
        let member_count: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM agency_members WHERE agency_id = $1 AND is_active = true"#,
        )
        .bind(agency.id)
        .fetch_one(&self.pool)
        .await?;

        // Get active listing count
        let listing_count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM agency_listings al
            JOIN listings l ON l.id = al.listing_id
            WHERE al.agency_id = $1 AND l.status = 'active'
            "#,
        )
        .bind(agency.id)
        .fetch_one(&self.pool)
        .await?;

        Ok(Some(AgencyProfile {
            id: agency.id,
            name: agency.name,
            slug: agency.slug,
            address: agency.address,
            city: agency.city,
            country: agency.country,
            phone: agency.phone,
            email: agency.email,
            website: agency.website,
            logo_url: agency.logo_url,
            description: agency.description,
            verified: agency.status == agency_status::VERIFIED,
            member_count: member_count.0 as i32,
            active_listing_count: listing_count.0,
        }))
    }

    /// Get agency summary.
    pub async fn get_summary(&self, id: Uuid) -> Result<Option<AgencySummary>, SqlxError> {
        let summary = sqlx::query_as::<_, AgencySummary>(
            r#"SELECT id, name, slug, logo_url, city FROM agencies WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(summary)
    }

    // ========================================================================
    // Agency Members (Story 17.2)
    // ========================================================================

    /// Get agency members with user details.
    pub async fn get_members(
        &self,
        agency_id: Uuid,
    ) -> Result<Vec<AgencyMemberWithUser>, SqlxError> {
        let rows = sqlx::query_as::<_, AgencyMemberWithUserRow>(
            r#"
            SELECT
                am.id, am.agency_id, am.user_id,
                u.name as user_name, u.email as user_email, u.profile_image_url as user_avatar,
                am.role, am.is_active, am.joined_at,
                COALESCE((SELECT COUNT(*) FROM agency_listings WHERE realtor_id = am.user_id AND agency_id = am.agency_id), 0) as listing_count
            FROM agency_members am
            JOIN users u ON u.id = am.user_id
            WHERE am.agency_id = $1 AND am.is_active = true
            ORDER BY am.joined_at
            "#,
        )
        .bind(agency_id)
        .fetch_all(&self.pool)
        .await?;

        let members = rows
            .into_iter()
            .map(|r| AgencyMemberWithUser {
                id: r.id,
                agency_id: r.agency_id,
                user_id: r.user_id,
                user_name: r.user_name,
                user_email: r.user_email,
                user_avatar: r.user_avatar,
                role: r.role,
                is_active: r.is_active,
                joined_at: r.joined_at,
                listing_count: r.listing_count,
            })
            .collect();

        Ok(members)
    }

    /// Get member by user ID.
    pub async fn get_member(
        &self,
        agency_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<AgencyMember>, SqlxError> {
        let member = sqlx::query_as::<_, AgencyMember>(
            r#"SELECT * FROM agency_members WHERE agency_id = $1 AND user_id = $2"#,
        )
        .bind(agency_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(member)
    }

    /// Create invitation.
    pub async fn create_invitation(
        &self,
        agency_id: Uuid,
        data: InviteMember,
        invited_by: Uuid,
    ) -> Result<AgencyInvitation, SqlxError> {
        let token = Uuid::new_v4().to_string();
        let expires_at = Utc::now() + Duration::days(7);

        let invitation = sqlx::query_as::<_, AgencyInvitation>(
            r#"
            INSERT INTO agency_invitations (agency_id, email, role, invited_by, token, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(agency_id)
        .bind(&data.email)
        .bind(&data.role)
        .bind(invited_by)
        .bind(&token)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(invitation)
    }

    /// Accept invitation.
    pub async fn accept_invitation(
        &self,
        token: &str,
        user_id: Uuid,
    ) -> Result<AgencyMember, SqlxError> {
        let mut tx = self.pool.begin().await?;

        // Get invitation
        let invitation = sqlx::query_as::<_, AgencyInvitation>(
            r#"
            SELECT * FROM agency_invitations
            WHERE token = $1 AND accepted_at IS NULL AND expires_at > NOW()
            "#,
        )
        .bind(token)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| SqlxError::RowNotFound)?;

        // Mark invitation as accepted
        sqlx::query(r#"UPDATE agency_invitations SET accepted_at = NOW() WHERE id = $1"#)
            .bind(invitation.id)
            .execute(&mut *tx)
            .await?;

        // Create member
        let member = sqlx::query_as::<_, AgencyMember>(
            r#"
            INSERT INTO agency_members (agency_id, user_id, role, is_active)
            VALUES ($1, $2, $3, true)
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

    /// Update member role.
    pub async fn update_member_role(
        &self,
        agency_id: Uuid,
        user_id: Uuid,
        data: UpdateMemberRole,
    ) -> Result<AgencyMember, SqlxError> {
        let member = sqlx::query_as::<_, AgencyMember>(
            r#"
            UPDATE agency_members SET role = $3
            WHERE agency_id = $1 AND user_id = $2
            RETURNING *
            "#,
        )
        .bind(agency_id)
        .bind(user_id)
        .bind(&data.role)
        .fetch_one(&self.pool)
        .await?;

        Ok(member)
    }

    /// Remove member (set inactive).
    pub async fn remove_member(&self, agency_id: Uuid, user_id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE agency_members SET is_active = false, left_at = NOW()
            WHERE agency_id = $1 AND user_id = $2
            "#,
        )
        .bind(agency_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Reassign listings from one realtor to another.
    pub async fn reassign_listings(
        &self,
        agency_id: Uuid,
        from_user_id: Uuid,
        to_user_id: Uuid,
    ) -> Result<i64, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE agency_listings SET realtor_id = $3
            WHERE agency_id = $1 AND realtor_id = $2
            "#,
        )
        .bind(agency_id)
        .bind(from_user_id)
        .bind(to_user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    // ========================================================================
    // Agency Listings (Story 17.3)
    // ========================================================================

    /// Associate listing with agency.
    pub async fn create_agency_listing(
        &self,
        agency_id: Uuid,
        realtor_id: Uuid,
        data: CreateAgencyListing,
    ) -> Result<AgencyListing, SqlxError> {
        let listing = sqlx::query_as::<_, AgencyListing>(
            r#"
            INSERT INTO agency_listings (listing_id, agency_id, realtor_id, visibility, inquiry_assignment)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(data.listing_id)
        .bind(agency_id)
        .bind(realtor_id)
        .bind(&data.visibility)
        .bind(&data.inquiry_assignment)
        .fetch_one(&self.pool)
        .await?;

        Ok(listing)
    }

    /// Get agency listing by listing ID.
    pub async fn get_agency_listing(
        &self,
        listing_id: Uuid,
    ) -> Result<Option<AgencyListing>, SqlxError> {
        let listing = sqlx::query_as::<_, AgencyListing>(
            r#"SELECT * FROM agency_listings WHERE listing_id = $1"#,
        )
        .bind(listing_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(listing)
    }

    /// Update the visibility of a listing of `agency_id` (`None` when the
    /// listing is not one of that agency's listings).
    pub async fn update_listing_visibility(
        &self,
        agency_id: Uuid,
        listing_id: Uuid,
        visibility: &str,
    ) -> Result<Option<AgencyListing>, SqlxError> {
        let listing = sqlx::query_as::<_, AgencyListing>(
            r#"
            UPDATE agency_listings SET visibility = $2
            WHERE listing_id = $1 AND agency_id = $3
            RETURNING *
            "#,
        )
        .bind(listing_id)
        .bind(visibility)
        .bind(agency_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(listing)
    }

    /// Check if user can access listing (based on visibility and membership).
    pub async fn can_access_listing(
        &self,
        listing_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, SqlxError> {
        let row: (bool,) = sqlx::query_as(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM agency_listings al
                WHERE al.listing_id = $1
                AND (
                    al.realtor_id = $2
                    OR (al.visibility IN ('agency', 'public') AND EXISTS(
                        SELECT 1 FROM agency_members am
                        WHERE am.agency_id = al.agency_id AND am.user_id = $2 AND am.is_active = true
                    ))
                )
            )
            "#,
        )
        .bind(listing_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0)
    }

    /// Log listing edit.
    pub async fn log_listing_edit(
        &self,
        listing_id: Uuid,
        editor_id: Uuid,
        editor_name: String,
        field_changed: String,
        old_value: Option<String>,
        new_value: Option<String>,
    ) -> Result<ListingEditHistory, SqlxError> {
        let entry = sqlx::query_as::<_, ListingEditHistory>(
            r#"
            INSERT INTO listing_edit_history (listing_id, editor_id, editor_name, field_changed, old_value, new_value)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(listing_id)
        .bind(editor_id)
        .bind(&editor_name)
        .bind(&field_changed)
        .bind(&old_value)
        .bind(&new_value)
        .fetch_one(&self.pool)
        .await?;

        Ok(entry)
    }

    /// Get listing edit history.
    pub async fn get_listing_history(
        &self,
        listing_id: Uuid,
        limit: i32,
    ) -> Result<Vec<ListingEditHistory>, SqlxError> {
        let history = sqlx::query_as::<_, ListingEditHistory>(
            r#"
            SELECT * FROM listing_edit_history
            WHERE listing_id = $1
            ORDER BY edited_at DESC
            LIMIT $2
            "#,
        )
        .bind(listing_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(history)
    }

    // ========================================================================
    // Import Jobs (Story 17.4)
    // ========================================================================

    /// Create import job.
    pub async fn create_import_job(
        &self,
        agency_id: Uuid,
        user_id: Uuid,
        source: &str,
    ) -> Result<ListingImportJob, SqlxError> {
        let job = sqlx::query_as::<_, ListingImportJob>(
            r#"
            INSERT INTO listing_import_jobs (agency_id, user_id, source, status, total_records, processed_records, success_count, failure_count)
            VALUES ($1, $2, $3, 'pending', 0, 0, 0, 0)
            RETURNING *
            "#,
        )
        .bind(agency_id)
        .bind(user_id)
        .bind(source)
        .fetch_one(&self.pool)
        .await?;

        Ok(job)
    }

    /// Get import job.
    pub async fn get_import_job(&self, id: Uuid) -> Result<Option<ListingImportJob>, SqlxError> {
        let job = sqlx::query_as::<_, ListingImportJob>(
            r#"SELECT * FROM listing_import_jobs WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(job)
    }

    /// Update import job progress.
    pub async fn update_import_progress(
        &self,
        id: Uuid,
        status: &str,
        processed: i32,
        success: i32,
        failure: i32,
    ) -> Result<(), SqlxError> {
        sqlx::query(
            r#"
            UPDATE listing_import_jobs SET
                status = $2,
                processed_records = $3,
                success_count = $4,
                failure_count = $5,
                started_at = COALESCE(started_at, NOW())
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(processed)
        .bind(success)
        .bind(failure)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Complete import job.
    pub async fn complete_import_job(
        &self,
        id: Uuid,
        success: bool,
        error_log: Option<String>,
    ) -> Result<(), SqlxError> {
        let status = if success { "completed" } else { "failed" };

        sqlx::query(
            r#"
            UPDATE listing_import_jobs SET
                status = $2,
                error_log = $3,
                completed_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(&error_log)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get agency import jobs.
    pub async fn get_import_jobs(
        &self,
        agency_id: Uuid,
        limit: i32,
    ) -> Result<Vec<ListingImportJob>, SqlxError> {
        let jobs = sqlx::query_as::<_, ListingImportJob>(
            r#"
            SELECT * FROM listing_import_jobs
            WHERE agency_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(agency_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(jobs)
    }
}
