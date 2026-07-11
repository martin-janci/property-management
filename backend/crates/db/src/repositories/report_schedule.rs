//! Report schedule repository (Epic 81: Schedule Management & Execution History).
//!
//! Real SQLx implementation backed by the `report_schedules` and
//! `report_executions` tables added in migration
//! `00162_create_report_schedules_executions.sql`. The `cron_expression`
//! column is added by `00166_report_schedules_add_cron_expression.sql`.

use crate::models::report_schedule::{
    report_execution_status, report_schedule_status, ExecutionDownloadUrl, ExecutionHistoryQuery,
    ExecutionHistoryResponse, NewReportSchedule, ReportExecution, ReportSchedule,
    ReportScheduleRow,
};
use crate::DbPool;
use chrono::{Duration, Utc};
use common::errors::AppError;
use uuid::Uuid;

/// Canonical projection for `report_schedules` rows.
///
/// Every `SELECT` / `RETURNING` that maps into [`ReportScheduleRow`] must list
/// exactly these columns — `query_as` resolves `FromRow` fields by name at
/// runtime, so omitting one (historically `cron_expression`, issue #616) makes
/// the query fail with "no column found for name: …" only when a live DB is
/// hit. Defined with `concat!` so it is a compile-time `&'static str`
/// (required by sqlx 0.9's `SqlSafeStr` bound) and so every query below shares
/// the same column list. The tests at the bottom of this module guard the
/// projection without needing a database.
macro_rules! report_schedule_columns {
    () => {
        "id, report_id, organization_id, name, frequency, \
         day_of_week, day_of_month, time, timezone, format, \
         recipients, is_active, status, \
         last_run_at, next_run_at, created_at, updated_at, \
         cron_expression"
    };
}

/// The bare column projection, used by the parity tests below.
#[cfg(test)]
const REPORT_SCHEDULE_COLUMNS: &str = report_schedule_columns!();

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

    /// Get a schedule by ID (unscoped — super-admin / internal use only).
    ///
    /// Prefer `get_by_id_scoped` in request handlers so the caller's
    /// `organization_id` is enforced in the WHERE clause.
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<ReportSchedule>, AppError> {
        let row = sqlx::query_as::<_, ReportScheduleRow>(concat!(
            "SELECT ",
            report_schedule_columns!(),
            " FROM report_schedules WHERE id = $1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, schedule_id = %id, "Failed to get report schedule");
            AppError::Database(e.to_string())
        })?;

        Ok(row.map(ReportSchedule::from))
    }

    /// Get a schedule by ID scoped to the caller's organisation (closes #646 / #647).
    ///
    /// Returns `NotFound` for both "does not exist" and "belongs to a different
    /// org" to avoid leaking cross-tenant existence information.
    pub async fn get_by_id_scoped(
        &self,
        id: Uuid,
        caller_org_id: Uuid,
    ) -> Result<Option<ReportSchedule>, AppError> {
        let row = sqlx::query_as::<_, ReportScheduleRow>(concat!(
            "SELECT ",
            report_schedule_columns!(),
            " FROM report_schedules WHERE id = $1 AND organization_id = $2"
        ))
        .bind(id)
        .bind(caller_org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                schedule_id = %id,
                org_id = %caller_org_id,
                "Failed to get report schedule (scoped)"
            );
            AppError::Database(e.to_string())
        })?;

        Ok(row.map(ReportSchedule::from))
    }

    /// Pause a schedule (Story 81.1), scoped to the caller's organisation (closes #646).
    ///
    /// Sets `is_active = false`, `status = 'paused'`.
    /// Returns `NotFound` when the `id` does not exist **or** belongs to a
    /// different organisation, giving the same opaque response in both cases.
    pub async fn pause(&self, id: Uuid, caller_org_id: Uuid) -> Result<ReportSchedule, AppError> {
        let row = sqlx::query_as::<_, ReportScheduleRow>(concat!(
            "UPDATE report_schedules \
             SET is_active = false, status = $1, updated_at = NOW() \
             WHERE id = $2 AND organization_id = $3 \
             RETURNING ",
            report_schedule_columns!()
        ))
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

    /// Resume a paused schedule (Story 81.1), scoped to the caller's organisation (closes #646).
    ///
    /// Sets `is_active = true`, `status = 'active'`.
    /// Returns `NotFound` when the `id` does not exist **or** belongs to a
    /// different organisation, giving the same opaque response in both cases.
    pub async fn resume(&self, id: Uuid, caller_org_id: Uuid) -> Result<ReportSchedule, AppError> {
        let row = sqlx::query_as::<_, ReportScheduleRow>(concat!(
            "UPDATE report_schedules \
             SET is_active = true, status = $1, updated_at = NOW() \
             WHERE id = $2 AND organization_id = $3 \
             RETURNING ",
            report_schedule_columns!()
        ))
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

    /// Create (persist) a new report schedule (gap-81-1).
    ///
    /// Inserts a row into `report_schedules` and returns the persisted record.
    /// The new schedule starts `is_active = true` / `status = 'active'` and has
    /// no `cron_expression` (callers use the legacy `time`/`frequency` fields at
    /// creation time and may later switch to a cron expression via
    /// `update_schedule`).
    ///
    /// # Cross-tenant safety
    ///
    /// `organization_id` comes from `input.organization_id`, which the handler
    /// derives from the authenticated tenant (`RlsConnection::tenant_id()`) —
    /// never from the request body — so a caller cannot create a schedule inside
    /// another organisation.
    pub async fn create(&self, input: NewReportSchedule) -> Result<ReportSchedule, AppError> {
        // Serialise the recipients Vec<String> into a JSON array for JSONB storage.
        let recipients_json = serde_json::Value::Array(
            input
                .recipients
                .into_iter()
                .map(serde_json::Value::String)
                .collect(),
        );

        let row = sqlx::query_as::<_, ReportScheduleRow>(concat!(
            "INSERT INTO report_schedules \
             (report_id, organization_id, name, frequency, day_of_week, day_of_month, \
              time, timezone, format, recipients, is_active, status, next_run_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, true, $11, $12) \
             RETURNING ",
            report_schedule_columns!()
        ))
        .bind(input.report_id)
        .bind(input.organization_id)
        .bind(input.name)
        .bind(input.frequency)
        .bind(input.day_of_week)
        .bind(input.day_of_month)
        .bind(input.time)
        .bind(input.timezone)
        .bind(input.format)
        .bind(recipients_json)
        .bind(report_schedule_status::ACTIVE)
        .bind(input.next_run_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                org_id = %input.organization_id,
                report_id = %input.report_id,
                "Failed to create report schedule"
            );
            AppError::Database(e.to_string())
        })?;

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

    /// Get a single execution by ID (unscoped — internal use only).
    ///
    /// Prefer `get_execution_scoped` in request handlers so the caller's
    /// `organization_id` is enforced via the parent schedule's tenant.
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

    /// Get a single execution scoped to the caller's organisation (closes #647).
    ///
    /// Joins `report_executions` → `report_schedules` and filters on
    /// `report_schedules.organization_id = caller_org_id` so a principal in
    /// org B cannot read an execution that belongs to org A's schedule.
    /// Returns `None` for both "not found" and "wrong org" to avoid leaking
    /// cross-tenant existence information.
    pub async fn get_execution_scoped(
        &self,
        id: Uuid,
        caller_org_id: Uuid,
    ) -> Result<Option<ReportExecution>, AppError> {
        sqlx::query_as::<_, ReportExecution>(
            r#"
            SELECT e.id, e.schedule_id, e.status,
                   e.started_at, e.completed_at, e.duration_ms,
                   e.file_key, e.file_name, e.file_size,
                   e.error_code, e.error_message, e.error_details,
                   e.created_at
            FROM report_executions e
            JOIN report_schedules  s ON s.id = e.schedule_id
            WHERE e.id              = $1
              AND s.organization_id = $2
            "#,
        )
        .bind(id)
        .bind(caller_org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                execution_id = %id,
                org_id = %caller_org_id,
                "Failed to get execution (scoped)"
            );
            AppError::Database(e.to_string())
        })
    }

    /// Retry a failed execution, scoped to the caller's organisation (closes #647).
    ///
    /// Like `get_execution_scoped` the UPDATE joins through `report_schedules`
    /// to enforce the tenant boundary.  Returns `BadRequest` when the execution
    /// is found but not in `failed` state, and `NotFound` when it does not exist
    /// or belongs to a different organisation.
    pub async fn retry_execution_scoped(
        &self,
        id: Uuid,
        caller_org_id: Uuid,
    ) -> Result<ReportExecution, AppError> {
        // First verify the execution exists and belongs to the caller's org.
        let execution = self
            .get_execution_scoped(id, caller_org_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Execution {} not found", id)))?;

        // Enforce "only failed executions may be retried" before touching the DB.
        if execution.status != report_execution_status::FAILED {
            return Err(AppError::BadRequest(
                "Execution not in 'failed' state; only failed executions can be retried".into(),
            ));
        }

        // Now reset the status.  Repeat the org check in the WHERE clause as a
        // defence-in-depth measure (covers TOCTOU between the SELECT above and
        // this UPDATE).
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
              AND schedule_id IN (
                  SELECT id FROM report_schedules WHERE organization_id = $4
              )
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
        .bind(caller_org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                execution_id = %id,
                org_id = %caller_org_id,
                "Failed to retry execution"
            );
            AppError::Database(e.to_string())
        })?
        .ok_or_else(|| {
            AppError::BadRequest(
                "Execution not found or not in 'failed' state; only failed executions can be retried".into(),
            )
        })?;

        Ok(updated)
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
    /// All parameters are optional; only non-`None` values are applied. The
    /// cron expression is persisted to the dedicated `cron_expression` column
    /// added by migration `00166_report_schedules_add_cron_expression.sql`
    /// (NOT the legacy `time` HH:MM column).
    ///
    /// # Cross-tenant safety (closes #624)
    ///
    /// The WHERE clause includes `AND organization_id = $caller_org_id` so a
    /// principal in org A cannot mutate a schedule that belongs to org B, even
    /// if they know the schedule's UUID. The UPDATE returns no row (→ 404) when
    /// the `id` exists but the `organization_id` does not match, giving the same
    /// opaque response as "not found" to avoid leaking existence information.
    pub async fn update_schedule(
        &self,
        id: Uuid,
        caller_org_id: Uuid,
        cron_expression: Option<String>,
        recipients: Option<Vec<String>>,
        enabled: Option<bool>,
    ) -> Result<ReportSchedule, AppError> {
        // Build the status string when `enabled` is being changed.
        let new_status: Option<String> = enabled.map(|is_active| {
            if is_active {
                report_schedule_status::ACTIVE.to_string()
            } else {
                report_schedule_status::PAUSED.to_string()
            }
        });

        // Convert the recipients Vec<String> into a JSON array for JSONB storage.
        let recipients_json: Option<serde_json::Value> = recipients.map(|v| {
            serde_json::Value::Array(v.into_iter().map(serde_json::Value::String).collect())
        });

        let row = sqlx::query_as::<_, ReportScheduleRow>(concat!(
            "UPDATE report_schedules \
             SET cron_expression = COALESCE($3, cron_expression), \
                 recipients      = COALESCE($4, recipients), \
                 is_active       = COALESCE($5, is_active), \
                 status          = COALESCE($6, status), \
                 updated_at      = NOW() \
             WHERE id = $1 AND organization_id = $2 \
             RETURNING ",
            report_schedule_columns!()
        ))
        .bind(id)
        .bind(caller_org_id)
        .bind(cron_expression.as_deref())
        .bind(recipients_json.as_ref())
        .bind(enabled)
        .bind(new_status.as_deref())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                schedule_id = %id,
                org_id = %caller_org_id,
                "Failed to update report schedule"
            );
            AppError::Database(e.to_string())
        })?
        .ok_or_else(|| {
            // Either the schedule doesn't exist or it belongs to a different
            // organisation.  Return the same 404 in both cases to avoid leaking
            // cross-tenant existence information.
            AppError::NotFound(format!("Report schedule {} not found", id))
        })?;

        Ok(ReportSchedule::from(row))
    }
}

#[cfg(test)]
mod tests {
    use super::REPORT_SCHEDULE_COLUMNS;

    /// Every column that [`super::ReportScheduleRow`] maps from `FromRow` must
    /// appear in the canonical projection. `query_as` resolves fields by name
    /// at runtime, so a missing column only surfaces as a DB error in
    /// production. This test is the no-DB guard for issue #616, where the
    /// `cron_expression` column was added to the row struct but dropped from
    /// the `get_by_id_scoped` SELECT — making cron edits silently fail to
    /// round-trip through the dedicated column.
    #[test]
    fn column_projection_covers_every_row_field() {
        // Field list mirrors `ReportScheduleRow`. If a field is added there,
        // add it here and to REPORT_SCHEDULE_COLUMNS — the test fails loudly
        // until both are in sync.
        let expected = [
            "id",
            "report_id",
            "organization_id",
            "name",
            "frequency",
            "day_of_week",
            "day_of_month",
            "time",
            "timezone",
            "format",
            "recipients",
            "is_active",
            "status",
            "last_run_at",
            "next_run_at",
            "created_at",
            "updated_at",
            "cron_expression",
        ];

        let actual: Vec<&str> = REPORT_SCHEDULE_COLUMNS
            .split(',')
            .map(|c| c.trim())
            .collect();

        assert_eq!(
            actual, expected,
            "REPORT_SCHEDULE_COLUMNS must list every ReportScheduleRow field, \
             in order, including cron_expression (issue #616)"
        );
    }

    /// Explicit guard for the exact regression in issue #616.
    #[test]
    fn projection_includes_cron_expression() {
        assert!(
            REPORT_SCHEDULE_COLUMNS
                .split(',')
                .any(|c| c.trim() == "cron_expression"),
            "cron_expression must be selected so report-schedule edits round-trip \
             through the dedicated column, not the legacy `time` field (issue #616)"
        );
    }
}
