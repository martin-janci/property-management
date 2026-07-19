//! Government Portal Integration repository (Epic 30).
//!
//! UC-22.3: Government Portal Integration

use chrono::{NaiveDate, Utc};
use sqlx::{Error as SqlxError, PgPool, Row};
use uuid::Uuid;

use crate::models::{
    AddSubmissionAttachment, CreatePortalConnection, CreateRegulatorySubmission,
    CreateSubmissionAudit, CreateSubmissionSchedule, GovernmentPortalConnection,
    GovernmentPortalStats, GovernmentPortalType, RegulatoryReportTemplate, RegulatorySubmission,
    RegulatorySubmissionAttachment, RegulatorySubmissionAudit, RegulatorySubmissionSchedule,
    SubmissionStatus, UpcomingDueDate,
};

/// Repository for government portal operations.
#[derive(Clone)]
pub struct GovernmentPortalRepository {
    pool: PgPool,
}

impl GovernmentPortalRepository {
    /// Create a new repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // Portal Connections
    // ========================================================================

    /// List portal connections for an organization.
    pub async fn list_connections(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<GovernmentPortalConnection>, SqlxError> {
        sqlx::query_as::<_, GovernmentPortalConnection>(
            r#"
            SELECT * FROM government_portal_connections
            WHERE organization_id = $1
            ORDER BY portal_name
            "#,
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Get a specific portal connection scoped to its owning organization.
    ///
    /// Tenant scoping is enforced at the query level (`organization_id = $2`) so
    /// that a connection `id` belonging to another org is treated as
    /// non-existent rather than leaking its credentials (cross-tenant IDOR, #2413).
    pub async fn get_connection(
        &self,
        id: Uuid,
        organization_id: Uuid,
    ) -> Result<Option<GovernmentPortalConnection>, SqlxError> {
        sqlx::query_as::<_, GovernmentPortalConnection>(
            "SELECT * FROM government_portal_connections WHERE id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(organization_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Create a portal connection.
    pub async fn create_connection(
        &self,
        organization_id: Uuid,
        request: CreatePortalConnection,
        created_by: Uuid,
    ) -> Result<GovernmentPortalConnection, SqlxError> {
        sqlx::query_as::<_, GovernmentPortalConnection>(
            r#"
            INSERT INTO government_portal_connections (
                organization_id, portal_type, portal_name, portal_code, country_code,
                api_endpoint, portal_username, oauth_client_id,
                auto_submit, test_mode, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(request.portal_type)
        .bind(&request.portal_name)
        .bind(&request.portal_code)
        .bind(&request.country_code)
        .bind(&request.api_endpoint)
        .bind(&request.portal_username)
        .bind(&request.oauth_client_id)
        .bind(request.auto_submit)
        .bind(request.test_mode)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await
    }

    /// Update a portal connection scoped to its owning organization.
    ///
    /// Returns `Ok(None)` when no connection with `id` exists **for the given
    /// organization**, so a cross-org `id` cannot overwrite another tenant's
    /// credentials (cross-tenant IDOR, #2413).
    #[allow(clippy::too_many_arguments)]
    pub async fn update_connection(
        &self,
        id: Uuid,
        organization_id: Uuid,
        portal_name: Option<&str>,
        api_endpoint: Option<&str>,
        portal_username: Option<&str>,
        oauth_client_id: Option<&str>,
        is_active: Option<bool>,
        auto_submit: Option<bool>,
        test_mode: Option<bool>,
    ) -> Result<Option<GovernmentPortalConnection>, SqlxError> {
        sqlx::query_as::<_, GovernmentPortalConnection>(
            r#"
            UPDATE government_portal_connections SET
                portal_name = COALESCE($3, portal_name),
                api_endpoint = COALESCE($4, api_endpoint),
                portal_username = COALESCE($5, portal_username),
                oauth_client_id = COALESCE($6, oauth_client_id),
                is_active = COALESCE($7, is_active),
                auto_submit = COALESCE($8, auto_submit),
                test_mode = COALESCE($9, test_mode),
                updated_at = now()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(organization_id)
        .bind(portal_name)
        .bind(api_endpoint)
        .bind(portal_username)
        .bind(oauth_client_id)
        .bind(is_active)
        .bind(auto_submit)
        .bind(test_mode)
        .fetch_optional(&self.pool)
        .await
    }

    /// Delete a portal connection scoped to its owning organization.
    ///
    /// Returns `false` when no connection with `id` exists for the given
    /// organization (cross-tenant IDOR guard, #2413).
    pub async fn delete_connection(
        &self,
        id: Uuid,
        organization_id: Uuid,
    ) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            "DELETE FROM government_portal_connections WHERE id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(organization_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Test connection to a portal (updates last_connection_test).
    ///
    /// Scoped to the owning organization; returns `false` when no matching
    /// connection exists for `organization_id`, so a cross-org `id` cannot be
    /// probed/mutated (cross-tenant IDOR guard, #2413).
    pub async fn record_connection_test(
        &self,
        id: Uuid,
        success: bool,
        organization_id: Uuid,
    ) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE government_portal_connections
            SET last_connection_test = now()
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(organization_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Ok(false);
        }

        if success {
            sqlx::query(
                r#"
                UPDATE government_portal_connections
                SET last_successful_submission = now()
                WHERE id = $1 AND organization_id = $2
                "#,
            )
            .bind(id)
            .bind(organization_id)
            .execute(&self.pool)
            .await?;
        }

        Ok(true)
    }

    // ========================================================================
    // Report Templates
    // ========================================================================

    /// List all active report templates.
    pub async fn list_templates(
        &self,
        portal_type: Option<GovernmentPortalType>,
        country_code: Option<&str>,
    ) -> Result<Vec<RegulatoryReportTemplate>, SqlxError> {
        sqlx::query_as::<_, RegulatoryReportTemplate>(
            r#"
            SELECT * FROM regulatory_report_templates
            WHERE is_active = true
              AND ($1::government_portal_type IS NULL OR portal_type = $1)
              AND ($2::text IS NULL OR country_code = $2)
            ORDER BY template_name
            "#,
        )
        .bind(portal_type)
        .bind(country_code)
        .fetch_all(&self.pool)
        .await
    }

    /// Get a specific template.
    pub async fn get_template(
        &self,
        id: Uuid,
    ) -> Result<Option<RegulatoryReportTemplate>, SqlxError> {
        sqlx::query_as::<_, RegulatoryReportTemplate>(
            "SELECT * FROM regulatory_report_templates WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Get template by code.
    pub async fn get_template_by_code(
        &self,
        template_code: &str,
    ) -> Result<Option<RegulatoryReportTemplate>, SqlxError> {
        sqlx::query_as::<_, RegulatoryReportTemplate>(
            "SELECT * FROM regulatory_report_templates WHERE template_code = $1",
        )
        .bind(template_code)
        .fetch_optional(&self.pool)
        .await
    }

    // ========================================================================
    // Regulatory Submissions
    // ========================================================================

    /// List submissions for an organization.
    #[allow(clippy::too_many_arguments)]
    pub async fn list_submissions(
        &self,
        organization_id: Uuid,
        status: Option<SubmissionStatus>,
        report_type: Option<&str>,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<RegulatorySubmission>, SqlxError> {
        sqlx::query_as::<_, RegulatorySubmission>(
            r#"
            SELECT * FROM regulatory_submissions
            WHERE organization_id = $1
              AND ($2::submission_status IS NULL OR status = $2)
              AND ($3::text IS NULL OR report_type = $3)
              AND ($4::date IS NULL OR report_period_start >= $4)
              AND ($5::date IS NULL OR report_period_end <= $5)
            ORDER BY created_at DESC
            LIMIT $6 OFFSET $7
            "#,
        )
        .bind(organization_id)
        .bind(status)
        .bind(report_type)
        .bind(from_date)
        .bind(to_date)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Get a specific submission.
    pub async fn get_submission(
        &self,
        id: Uuid,
    ) -> Result<Option<RegulatorySubmission>, SqlxError> {
        sqlx::query_as::<_, RegulatorySubmission>(
            "SELECT * FROM regulatory_submissions WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Create a submission.
    pub async fn create_submission(
        &self,
        organization_id: Uuid,
        request: CreateRegulatorySubmission,
        prepared_by: Uuid,
    ) -> Result<RegulatorySubmission, SqlxError> {
        // Generate reference
        let reference_row = sqlx::query("SELECT generate_submission_reference($1, $2) as ref")
            .bind(organization_id)
            .bind(&request.report_type)
            .fetch_one(&self.pool)
            .await?;

        let submission_reference: String = reference_row.get("ref");

        sqlx::query_as::<_, RegulatorySubmission>(
            r#"
            INSERT INTO regulatory_submissions (
                organization_id, portal_connection_id, template_id,
                submission_reference, report_type,
                report_period_start, report_period_end, report_data,
                prepared_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(request.portal_connection_id)
        .bind(request.template_id)
        .bind(&submission_reference)
        .bind(&request.report_type)
        .bind(request.report_period_start)
        .bind(request.report_period_end)
        .bind(&request.report_data)
        .bind(prepared_by)
        .fetch_one(&self.pool)
        .await
    }

    /// Update submission data.
    pub async fn update_submission(
        &self,
        id: Uuid,
        report_data: Option<serde_json::Value>,
        report_xml: Option<&str>,
    ) -> Result<RegulatorySubmission, SqlxError> {
        sqlx::query_as::<_, RegulatorySubmission>(
            r#"
            UPDATE regulatory_submissions SET
                report_data = COALESCE($2, report_data),
                report_xml = COALESCE($3, report_xml),
                updated_at = now()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&report_data)
        .bind(report_xml)
        .fetch_one(&self.pool)
        .await
    }

    /// Update submission status.
    pub async fn update_submission_status(
        &self,
        id: Uuid,
        status: SubmissionStatus,
        validation_result: Option<serde_json::Value>,
        submission_response: Option<serde_json::Value>,
        external_reference: Option<&str>,
    ) -> Result<RegulatorySubmission, SqlxError> {
        // Set appropriate timestamps based on status
        let now = Utc::now();

        let submission = sqlx::query_as::<_, RegulatorySubmission>(
            r#"
            UPDATE regulatory_submissions SET
                status = $2,
                validation_result = COALESCE($3, validation_result),
                submission_response = COALESCE($4, submission_response),
                external_reference = COALESCE($5, external_reference),
                validated_at = CASE WHEN $2 = 'validated' THEN $6 ELSE validated_at END,
                submitted_at = CASE WHEN $2 = 'submitted' THEN $6 ELSE submitted_at END,
                acknowledged_at = CASE WHEN $2 = 'acknowledged' THEN $6 ELSE acknowledged_at END,
                processed_at = CASE WHEN $2 IN ('accepted', 'rejected') THEN $6 ELSE processed_at END,
                updated_at = $6
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(&validation_result)
        .bind(&submission_response)
        .bind(external_reference)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(submission)
    }

    /// Submit a submission (set submitted_by and increment attempts).
    pub async fn submit_submission(
        &self,
        id: Uuid,
        submitted_by: Uuid,
    ) -> Result<RegulatorySubmission, SqlxError> {
        sqlx::query_as::<_, RegulatorySubmission>(
            r#"
            UPDATE regulatory_submissions SET
                status = 'submitted',
                submitted_by = $2,
                submitted_at = now(),
                submission_attempts = submission_attempts + 1,
                updated_at = now()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(submitted_by)
        .fetch_one(&self.pool)
        .await
    }

    /// Record submission error.
    pub async fn record_submission_error(
        &self,
        id: Uuid,
        error: &str,
        next_retry_at: Option<chrono::DateTime<Utc>>,
    ) -> Result<(), SqlxError> {
        sqlx::query(
            r#"
            UPDATE regulatory_submissions SET
                last_error = $2,
                next_retry_at = $3,
                updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(error)
        .bind(next_retry_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ========================================================================
    // Submission Audit
    // ========================================================================

    /// Create an audit entry.
    pub async fn create_audit(
        &self,
        audit: CreateSubmissionAudit,
    ) -> Result<RegulatorySubmissionAudit, SqlxError> {
        sqlx::query_as::<_, RegulatorySubmissionAudit>(
            r#"
            INSERT INTO regulatory_submission_audit (
                submission_id, action, previous_status, new_status,
                actor_id, actor_type, details, error_message
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(audit.submission_id)
        .bind(&audit.action)
        .bind(audit.previous_status)
        .bind(audit.new_status)
        .bind(audit.actor_id)
        .bind(&audit.actor_type)
        .bind(&audit.details)
        .bind(&audit.error_message)
        .fetch_one(&self.pool)
        .await
    }

    /// Get audit trail for a submission.
    pub async fn get_submission_audit(
        &self,
        submission_id: Uuid,
    ) -> Result<Vec<RegulatorySubmissionAudit>, SqlxError> {
        sqlx::query_as::<_, RegulatorySubmissionAudit>(
            r#"
            SELECT * FROM regulatory_submission_audit
            WHERE submission_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(submission_id)
        .fetch_all(&self.pool)
        .await
    }

    // ========================================================================
    // Submission Attachments
    // ========================================================================

    /// Add an attachment to a submission.
    pub async fn add_attachment(
        &self,
        submission_id: Uuid,
        attachment: AddSubmissionAttachment,
    ) -> Result<RegulatorySubmissionAttachment, SqlxError> {
        sqlx::query_as::<_, RegulatorySubmissionAttachment>(
            r#"
            INSERT INTO regulatory_submission_attachments (
                submission_id, file_name, file_type, file_size, file_url,
                checksum, attachment_type, description
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(submission_id)
        .bind(&attachment.file_name)
        .bind(&attachment.file_type)
        .bind(attachment.file_size)
        .bind(&attachment.file_url)
        .bind(&attachment.checksum)
        .bind(&attachment.attachment_type)
        .bind(&attachment.description)
        .fetch_one(&self.pool)
        .await
    }

    /// Get attachments for a submission.
    pub async fn get_submission_attachments(
        &self,
        submission_id: Uuid,
    ) -> Result<Vec<RegulatorySubmissionAttachment>, SqlxError> {
        sqlx::query_as::<_, RegulatorySubmissionAttachment>(
            r#"
            SELECT * FROM regulatory_submission_attachments
            WHERE submission_id = $1
            ORDER BY created_at
            "#,
        )
        .bind(submission_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Delete an attachment.
    pub async fn delete_attachment(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query("DELETE FROM regulatory_submission_attachments WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // Submission Schedules
    // ========================================================================

    /// List schedules for an organization.
    pub async fn list_schedules(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<RegulatorySubmissionSchedule>, SqlxError> {
        sqlx::query_as::<_, RegulatorySubmissionSchedule>(
            r#"
            SELECT * FROM regulatory_submission_schedules
            WHERE organization_id = $1
            ORDER BY next_due_date
            "#,
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Create a schedule.
    pub async fn create_schedule(
        &self,
        organization_id: Uuid,
        request: CreateSubmissionSchedule,
        created_by: Uuid,
    ) -> Result<RegulatorySubmissionSchedule, SqlxError> {
        // Calculate next due date from template
        let next_due_row = sqlx::query("SELECT calculate_next_due_date($1) as due_date")
            .bind(request.template_id)
            .fetch_one(&self.pool)
            .await?;

        let next_due_date: Option<NaiveDate> = next_due_row.get("due_date");

        sqlx::query_as::<_, RegulatorySubmissionSchedule>(
            r#"
            INSERT INTO regulatory_submission_schedules (
                organization_id, portal_connection_id, template_id,
                next_due_date, auto_generate, auto_submit, notify_before_days,
                notify_users, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(request.portal_connection_id)
        .bind(request.template_id)
        .bind(next_due_date)
        .bind(request.auto_generate)
        .bind(request.auto_submit)
        .bind(request.notify_before_days)
        .bind(serde_json::json!(request.notify_users))
        .bind(created_by)
        .fetch_one(&self.pool)
        .await
    }

    /// Update a schedule.
    pub async fn update_schedule(
        &self,
        id: Uuid,
        is_active: Option<bool>,
        auto_generate: Option<bool>,
        auto_submit: Option<bool>,
        notify_before_days: Option<i32>,
        notify_users: Option<Vec<Uuid>>,
    ) -> Result<RegulatorySubmissionSchedule, SqlxError> {
        let notify_users_json = notify_users.map(|u| serde_json::json!(u));

        sqlx::query_as::<_, RegulatorySubmissionSchedule>(
            r#"
            UPDATE regulatory_submission_schedules SET
                is_active = COALESCE($2, is_active),
                auto_generate = COALESCE($3, auto_generate),
                auto_submit = COALESCE($4, auto_submit),
                notify_before_days = COALESCE($5, notify_before_days),
                notify_users = COALESCE($6, notify_users),
                updated_at = now()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(is_active)
        .bind(auto_generate)
        .bind(auto_submit)
        .bind(notify_before_days)
        .bind(&notify_users_json)
        .fetch_one(&self.pool)
        .await
    }

    /// Delete a schedule.
    pub async fn delete_schedule(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query("DELETE FROM regulatory_submission_schedules WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // Statistics
    // ========================================================================

    /// Get portal statistics for an organization.
    pub async fn get_stats(
        &self,
        organization_id: Uuid,
    ) -> Result<GovernmentPortalStats, SqlxError> {
        // Get connection counts
        let connection_row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE is_active = true) as active
            FROM government_portal_connections
            WHERE organization_id = $1
            "#,
        )
        .bind(organization_id)
        .fetch_one(&self.pool)
        .await?;

        let total_connections: i64 = connection_row.get("total");
        let active_connections: i64 = connection_row.get("active");

        // Get submission counts
        let submission_row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE created_at >= DATE_TRUNC('month', CURRENT_DATE)) as this_month,
                COUNT(*) FILTER (WHERE status IN ('draft', 'pending_validation', 'validated', 'submitted', 'processing')) as pending,
                COUNT(*) FILTER (WHERE status = 'rejected') as rejected
            FROM regulatory_submissions
            WHERE organization_id = $1
            "#,
        )
        .bind(organization_id)
        .fetch_one(&self.pool)
        .await?;

        let total_submissions: i64 = submission_row.get("total");
        let submissions_this_month: i64 = submission_row.get("this_month");
        let pending_submissions: i64 = submission_row.get("pending");
        let rejected_submissions: i64 = submission_row.get("rejected");

        // Get upcoming due dates
        let upcoming_rows = sqlx::query(
            r#"
            SELECT
                s.id as schedule_id,
                t.template_name,
                t.portal_type,
                s.next_due_date,
                (s.next_due_date - CURRENT_DATE)::integer as days_until_due
            FROM regulatory_submission_schedules s
            JOIN regulatory_report_templates t ON t.id = s.template_id
            WHERE s.organization_id = $1
              AND s.is_active = true
              AND s.next_due_date IS NOT NULL
              AND s.next_due_date >= CURRENT_DATE
            ORDER BY s.next_due_date
            LIMIT 5
            "#,
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await?;

        let upcoming_due_dates: Vec<UpcomingDueDate> = upcoming_rows
            .into_iter()
            .map(|row| UpcomingDueDate {
                schedule_id: row.get("schedule_id"),
                template_name: row.get("template_name"),
                portal_type: row.get("portal_type"),
                due_date: row.get("next_due_date"),
                days_until_due: row.get("days_until_due"),
            })
            .collect();

        Ok(GovernmentPortalStats {
            total_connections,
            active_connections,
            total_submissions,
            submissions_this_month,
            pending_submissions,
            rejected_submissions,
            upcoming_due_dates,
        })
    }
}
