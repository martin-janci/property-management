//! Report schedule repository (Epic 81: Schedule Management & Execution History).
//!
//! Real SQLx implementation backed by the `report_schedules` and
//! `report_executions` tables added in migration
//! `00159_create_report_schedules_executions.sql`.

use crate::models::report_schedule::{
    report_execution_status, report_schedule_status, ExecutionDownloadUrl, ExecutionHistoryQuery,
    ExecutionHistoryResponse, ReportExecution, ReportSchedule, ReportScheduleRow,
};
use crate::DbPool;
use chrono::{Duration, Utc};
use common::errors::AppError;
use uuid::Uuid;

/// Repository for report schedule and execution operations.
#[derive(Clone)]
pub struct ReportScheduleRepository {
    pool: DbPool,
}

impl ReportScheduleRepository {
    /// Create a new ReportScheduleRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ============================================================================
    // Schedule operations
    // ============================================================================

    /// Get a schedule by ID (no tenant scoping).
    ///
    /// **Prefer [`get_by_id_in_org`] in route handlers** — this method exists
    /// only for internal callers that have already verified tenant scope (e.g.
    /// the scheduler worker iterating all schedules). Calling it directly from
    /// a request handler with a caller-supplied `id` re-introduces the IDOR
    /// class fixed in #624 / #646 / #647.
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<ReportSchedule>, AppError> {
        let row = sqlx::query_as::<_, ReportScheduleRow>(
            r#"
            SELECT id, report_id, organization_id, name, frequency,
                   day_of_week, day_of_month, time, timezone, format,
                   recipients, is_active, status,
                   last_run_at, next_run_at, created_at, updated_at
            FROM report_schedules
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, schedule_id = %id, "Failed to get report schedule");
            AppError::Database(e.to_string())
        })?;

        Ok(row.map(ReportSchedule::from))
    }

    /// Get a schedule by ID, scoped to the caller's organization.
    ///
    /// # Cross-tenant safety (closes #647)
    ///
    /// Returns `Ok(None)` both when the schedule does not exist and when it
    /// exists in a different organisation — the handler maps both to 404 so
    /// no cross-tenant existence information leaks.
    pub async fn get_by_id_in_org(
        &self,
        id: Uuid,
        caller_org_id: Uuid,
    ) -> Result<Option<ReportSchedule>, AppError> {
        let row = sqlx::query_as::<_, ReportScheduleRow>(
            r#"
            SELECT id, report_id, organization_id, name, frequency,
                   day_of_week, day_of_month, time, timezone, format,
                   recipients, is_active, status,
                   last_run_at, next_run_at, created_at, updated_at
            FROM report_schedules
            WHERE id              = $1
              AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(caller_org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                schedule_id = %id,
                org_id = %caller_org_id,
                "Failed to get report schedule"
            );
            AppError::Database(e.to_string())
        })?;

        Ok(row.map(ReportSchedule::from))
    }

    /// Pause a schedule (Story 81.1).
    ///
    /// Sets `is_active = false`, `status = 'paused'`.
    ///
    /// # Cross-tenant safety (closes #646)
    ///
    /// The WHERE clause includes `AND organization_id = $caller_org_id` so a
    /// principal in org A cannot pause a schedule that belongs to org B, even
    /// if they know the schedule's UUID. The UPDATE returns no row (→ 404)
    /// when the `id` exists but the `organization_id` does not match, giving
    /// the same opaque response as "not found" to avoid leaking existence
    /// information.
    pub async fn pause(
        &self,
        id: Uuid,
        caller_org_id: Uuid,
    ) -> Result<ReportSchedule, AppError> {
        let row = sqlx::query_as::<_, ReportScheduleRow>(
            r#"
            UPDATE report_schedules
            SET is_active  = false,
                status     = $1,
                updated_at = NOW()
            WHERE id              = $2
              AND organization_id = $3
            RETURNING id, report_id, organization_id, name, frequency,
                      day_of_week, day_of_month, time, timezone, format,
                      recipients, is_active, status,
                      last_run_at, next_run_at, created_at, updated_at
            "#,
        )
        .bind(report_schedule_status::PAUSED)
        .bind(id)
        .bind(caller_org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                schedule_id = %id,
                org_id = %caller_org_id,
                "Failed to pause report schedule"
            );
            AppError::Database(e.to_string())
        })?
        .ok_or_else(|| AppError::NotFound(format!("Report schedule {} not found", id)))?;

        Ok(ReportSchedule::from(row))
    }

    /// Resume a paused schedule (Story 81.1).
    ///
    /// Sets `is_active = true`, `status = 'active'`.
    ///
    /// # Cross-tenant safety (closes #646)
    ///
    /// Same tenant-scoping contract as [`pause`] — `AND organization_id = $3`
    /// in the UPDATE WHERE clause, 404 on zero-row update.
    pub async fn resume(
        &self,
        id: Uuid,
        caller_org_id: Uuid,
    ) -> Result<ReportSchedule, AppError> {
        let row = sqlx::query_as::<_, ReportScheduleRow>(
            r#"
            UPDATE report_schedules
            SET is_active  = true,
                status     = $1,
                updated_at = NOW()
            WHERE id              = $2
              AND organization_id = $3
            RETURNING id, report_id, organization_id, name, frequency,
                      day_of_week, day_of_month, time, timezone, format,
                      recipients, is_active, status,
                      last_run_at, next_run_at, created_at, updated_at
            "#,
        )
        .bind(report_schedule_status::ACTIVE)
        .bind(id)
        .bind(caller_org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                schedule_id = %id,
                org_id = %caller_org_id,
                "Failed to resume report schedule"
            );
            AppError::Database(e.to_string())
        })?
        .ok_or_else(|| AppError::NotFound(format!("Report schedule {} not found", id)))?;

        Ok(ReportSchedule::from(row))
    }

    // ============================================================================
    // Execution history (Story 81.2)
    // ============================================================================

    /// List execution history for a schedule, paginated.
    ///
    /// Optionally filters by status. Results are ordered by `started_at DESC`
    /// (newest execution first).
    pub async fn list_executions(
        &self,
        query: ExecutionHistoryQuery,
    ) -> Result<ExecutionHistoryResponse, AppError> {
        // Count total matching rows for pagination metadata.
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM report_executions
            WHERE schedule_id = $1
              AND ($2::text IS NULL OR status = $2)
              AND ($3::timestamptz IS NULL OR started_at >= $3)
              AND ($4::timestamptz IS NULL OR started_at <= $4)
            "#,
        )
        .bind(query.schedule_id)
        .bind(&query.status)
        .bind(query.date_from)
        .bind(query.date_to)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, schedule_id = %query.schedule_id, "Failed to count executions");
            AppError::Database(e.to_string())
        })?;

        let executions = sqlx::query_as::<_, ReportExecution>(
            r#"
            SELECT id, schedule_id, status,
                   started_at, completed_at, duration_ms,
                   file_key, file_name, file_size,
                   error_code, error_message, error_details,
                   created_at
            FROM report_executions
            WHERE schedule_id = $1
              AND ($2::text IS NULL OR status = $2)
              AND ($3::timestamptz IS NULL OR started_at >= $3)
              AND ($4::timestamptz IS NULL OR started_at <= $4)
            ORDER BY started_at DESC
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(query.schedule_id)
        .bind(&query.status)
        .bind(query.date_from)
        .bind(query.date_to)
        .bind(query.limit)
        .bind(query.offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, schedule_id = %query.schedule_id, "Failed to list executions");
            AppError::Database(e.to_string())
        })?;

        let has_more = (query.offset + query.limit) < total;

        Ok(ExecutionHistoryResponse {
            executions,
            total,
            has_more,
        })
    }

    /// Get a single execution by ID.
    pub async fn get_execution(&self, id: Uuid) -> Result<Option<ReportExecution>, AppError> {
        sqlx::query_as::<_, ReportExecution>(
            r#"
            SELECT id, schedule_id, status,
                   started_at, completed_at, duration_ms,
                   file_key, file_name, file_size,
                   error_code, error_message, error_details,
                   created_at
            FROM report_executions
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, execution_id = %id, "Failed to get execution");
            AppError::Database(e.to_string())
        })
    }

    /// Get presigned download URL for a completed execution.
    ///
    /// Generates a short-lived URL from the stored S3 `file_key`.
    /// When `file_key` is NULL (execution not yet complete) returns an error.
    pub async fn get_download_url(
        &self,
        execution_id: Uuid,
    ) -> Result<ExecutionDownloadUrl, AppError> {
        let execution = self
            .get_execution(execution_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Execution {} not found", execution_id)))?;

        let file_key = execution.file_key.ok_or_else(|| {
            AppError::BadRequest("Execution does not have a completed file yet".into())
        })?;

        let file_name = execution
            .file_name
            .unwrap_or_else(|| "report.pdf".to_string());

        // Derive MIME type from file extension.
        let content_type = if file_name.ends_with(".pdf") {
            "application/pdf"
        } else if file_name.ends_with(".xlsx") {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        } else {
            "text/csv"
        };

        // Build a presigned-style URL. In production the S3 client would
        // generate a short-lived signed URL; here we produce a stable
        // API-gateway proxy path that the file-serving layer resolves.
        let url = format!("/api/v1/reports/files/{}", file_key);
        let expires_at = Utc::now() + Duration::hours(1);

        Ok(ExecutionDownloadUrl {
            url,
            expires_at,
            file_name,
            content_type: content_type.to_string(),
        })
    }

    /// Retry a failed execution.
    ///
    /// Resets `status = 'pending'` and clears error fields so the
    /// scheduler picks the job up again. Only allowed when the current
    /// status is `'failed'`; all other statuses return a BadRequest error.
    pub async fn retry_execution(&self, id: Uuid) -> Result<ReportExecution, AppError> {
        let updated = sqlx::query_as::<_, ReportExecution>(
            r#"
            UPDATE report_executions
            SET status        = $1,
                completed_at  = NULL,
                duration_ms   = NULL,
                error_code    = NULL,
                error_message = NULL,
                error_details = NULL
            WHERE id = $2
              AND status = $3
            RETURNING id, schedule_id, status,
                      started_at, completed_at, duration_ms,
                      file_key, file_name, file_size,
                      error_code, error_message, error_details,
                      created_at
            "#,
        )
        .bind(report_execution_status::PENDING)
        .bind(id)
        .bind(report_execution_status::FAILED)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, execution_id = %id, "Failed to retry execution");
            AppError::Database(e.to_string())
        })?
        .ok_or_else(|| {
            AppError::BadRequest(
                "Execution not found or not in 'failed' state; only failed executions can be retried".into(),
            )
        })?;

        Ok(updated)
    }

    /// Update a schedule's cron expression, recipients, and/or enabled flag (gap-81-1).
    ///
    /// All parameters are optional; only non-`None` values are applied.
    ///
    /// TODO: UPDATE report_schedules
    ///       SET cron_expression = COALESCE($2, cron_expression),
    ///           recipients      = COALESCE($3, recipients),
    ///           is_active       = COALESCE($4, is_active),
    ///           status          = CASE WHEN $4 IS NOT NULL
    ///                               THEN CASE WHEN $4 THEN 'active' ELSE 'paused' END
    ///                               ELSE status END,
    ///           updated_at      = NOW()
    ///       WHERE id = $1
    ///       RETURNING *
    pub async fn update_schedule(
        &self,
        _id: Uuid,
        cron_expression: Option<String>,
        recipients: Option<Vec<String>>,
        enabled: Option<bool>,
        current: &mut ReportSchedule,
    ) -> Result<ReportSchedule, AppError> {
        if let Some(cron) = cron_expression {
            // Stored in `time` until the dedicated column is added via migration.
            current.time = cron;
        }
        if let Some(recips) = recipients {
            current.recipients = recips;
        }
        if let Some(is_enabled) = enabled {
            current.is_active = is_enabled;
            current.status = if is_enabled {
                report_schedule_status::ACTIVE.to_string()
            } else {
                report_schedule_status::PAUSED.to_string()
            };
        }
        current.updated_at = Utc::now();
        Ok(current.clone())
    }
}
