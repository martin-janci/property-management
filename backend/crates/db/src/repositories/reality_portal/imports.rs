//! Property import jobs and feed subscriptions (Epic 34).

use super::RealityPortalRepository;
use crate::models::reality_portal::*;
use sqlx::Error as SqlxError;
use uuid::Uuid;

impl RealityPortalRepository {
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

    /// Update import job. Scoped to the owning `user_id` so a portal user
    /// cannot mutate another user's job by id (IDOR, PAP-142).
    pub async fn update_import_job(
        &self,
        id: Uuid,
        user_id: Uuid,
        data: UpdateImportJob,
    ) -> Result<PortalImportJob, SqlxError> {
        sqlx::query_as::<_, PortalImportJob>(
            r#"
            UPDATE portal_import_jobs SET
                source_url = COALESCE($3, source_url),
                source_filename = COALESCE($4, source_filename)
            WHERE id = $1 AND user_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(&data.source_url)
        .bind(&data.source_filename)
        .fetch_one(&self.pool)
        .await
    }

    /// Start import job. Scoped to the owning `user_id` (IDOR, PAP-142).
    pub async fn start_import_job(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<PortalImportJob, SqlxError> {
        sqlx::query_as::<_, PortalImportJob>(
            r#"
            UPDATE portal_import_jobs SET
                status = 'processing',
                started_at = NOW()
            WHERE id = $1 AND user_id = $2 AND status = 'pending'
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Cancel import job. Scoped to the owning `user_id` (IDOR, PAP-142).
    pub async fn cancel_import_job(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<PortalImportJob, SqlxError> {
        sqlx::query_as::<_, PortalImportJob>(
            r#"
            UPDATE portal_import_jobs SET
                status = 'cancelled',
                completed_at = NOW()
            WHERE id = $1 AND user_id = $2 AND status IN ('pending', 'processing')
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Get import job status. Scoped to the owning `user_id` so a portal user
    /// cannot read another user's job by id (IDOR, PAP-142).
    pub async fn get_import_job(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<PortalImportJob>, SqlxError> {
        sqlx::query_as::<_, PortalImportJob>(
            "SELECT * FROM portal_import_jobs WHERE id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(user_id)
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

    /// Resolve the agency a portal user belongs to (earliest active membership).
    ///
    /// Feed subscriptions are agency-scoped (#1584): a realtor's feeds belong to
    /// their agency and are shared with the agency's members, not keyed on the
    /// individual user. Returns `None` when the user has no active membership (the
    /// route then 403s — a user with no agency cannot own feeds). Multi-agency
    /// users resolve to their earliest-joined active agency.
    pub async fn get_active_agency_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Option<Uuid>, SqlxError> {
        sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT agency_id FROM reality_agency_members
            WHERE user_id = $1 AND is_active = TRUE
            ORDER BY joined_at ASC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

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

    /// Get feed subscription by ID. Scoped to the owning `agency_id` so an
    /// agency cannot read another agency's feed by id (IDOR, PAP-142).
    pub async fn get_feed_subscription(
        &self,
        id: Uuid,
        agency_id: Uuid,
    ) -> Result<Option<RealityFeedSubscription>, SqlxError> {
        sqlx::query_as::<_, RealityFeedSubscription>(
            "SELECT * FROM feed_subscriptions WHERE id = $1 AND agency_id = $2",
        )
        .bind(id)
        .bind(agency_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Update feed subscription. Scoped to the owning `agency_id` (IDOR, PAP-142).
    pub async fn update_feed_subscription(
        &self,
        id: Uuid,
        agency_id: Uuid,
        data: UpdateFeedSubscription,
    ) -> Result<RealityFeedSubscription, SqlxError> {
        sqlx::query_as::<_, RealityFeedSubscription>(
            r#"
            UPDATE feed_subscriptions SET
                name = COALESCE($3, name),
                feed_url = COALESCE($4, feed_url),
                feed_type = COALESCE($5, feed_type),
                sync_interval = COALESCE($6, sync_interval),
                is_active = COALESCE($7, is_active),
                updated_at = NOW()
            WHERE id = $1 AND agency_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(agency_id)
        .bind(&data.name)
        .bind(&data.feed_url)
        .bind(&data.feed_type)
        .bind(&data.sync_interval)
        .bind(data.is_active)
        .fetch_one(&self.pool)
        .await
    }

    /// Trigger immediate feed sync. Scoped to the owning `agency_id` (IDOR, PAP-142).
    pub async fn trigger_feed_sync(
        &self,
        id: Uuid,
        agency_id: Uuid,
    ) -> Result<RealityFeedSubscription, SqlxError> {
        // Mark as syncing and update last sync time
        sqlx::query_as::<_, RealityFeedSubscription>(
            r#"
            UPDATE feed_subscriptions SET
                last_sync_at = NOW()
            WHERE id = $1 AND agency_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(agency_id)
        .fetch_one(&self.pool)
        .await
    }
}
