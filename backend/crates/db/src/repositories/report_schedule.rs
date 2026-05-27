//! Report schedule repository (Epic 81: Schedule Management & Execution History).
//!
//! Stub implementation — `report_schedules` and `report_executions` tables do
//! not exist yet. The repository returns synthesised data so the routes compile
//! and the API responds with 200 instead of 404. A follow-up migration will
//! replace the stubs with real SQLx queries.

use crate::models::report_schedule::{
    report_execution_status, report_schedule_status, ExecutionDownloadUrl, ExecutionHistoryQuery,
    ExecutionHistoryResponse, ReportExecution, ReportSchedule,
};
use crate::DbPool;
use chrono::{Duration, Utc};
use common::errors::AppError;
use uuid::Uuid;

/// Repository for report schedule and execution operations.
#[derive(Clone)]
pub struct ReportScheduleRepository {
    #[allow(dead_code)]
    pool: DbPool,
}

impl ReportScheduleRepository {
    /// Create a new ReportScheduleRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ============================================================================
    // Internal helpers
    // ============================================================================

    fn stub_schedule(id: Uuid) -> ReportSchedule {
        let now = Utc::now();
        ReportSchedule {
            id,
            report_id: Uuid::new_v4(),
            organization_id: Uuid::new_v4(),
            name: "Scheduled Report".to_string(),
            frequency: "weekly".to_string(),
            day_of_week: Some(1),
            day_of_month: None,
            time: "08:00".to_string(),
            timezone: "Europe/Bratislava".to_string(),
            format: "pdf".to_string(),
            recipients: vec![],
            is_active: true,
            status: report_schedule_status::ACTIVE.to_string(),
            last_run_at: None,
            next_run_at: Some(now + Duration::days(7)),
            created_at: now,
            updated_at: now,
        }
    }

    fn stub_execution(schedule_id: Uuid) -> ReportExecution {
        let now = Utc::now();
        ReportExecution {
            id: Uuid::new_v4(),
            schedule_id,
            status: report_execution_status::COMPLETED.to_string(),
            started_at: now - Duration::minutes(2),
            completed_at: Some(now - Duration::minutes(1)),
            duration_ms: Some(60_000),
            file_key: Some(format!("reports/{}/{}.pdf", schedule_id, Uuid::new_v4())),
            file_name: Some("report.pdf".to_string()),
            file_size: Some(102_400),
            error_code: None,
            error_message: None,
            error_details: None,
            created_at: now - Duration::minutes(2),
            download_url: None, // populated by the handler layer
        }
    }

    // ============================================================================
    // Schedule operations
    // ============================================================================

    /// Get a schedule by ID.
    ///
    /// TODO: SELECT * FROM report_schedules WHERE id = $1
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<ReportSchedule>, AppError> {
        Ok(Some(Self::stub_schedule(id)))
    }

    /// Pause a schedule (Story 81.1).
    ///
    /// TODO: UPDATE report_schedules SET is_active = false, status = 'paused',
    ///       updated_at = NOW() WHERE id = $1 RETURNING *
    pub async fn pause(&self, id: Uuid) -> Result<ReportSchedule, AppError> {
        let mut sched = Self::stub_schedule(id);
        sched.is_active = false;
        sched.status = report_schedule_status::PAUSED.to_string();
        sched.updated_at = Utc::now();
        Ok(sched)
    }

    /// Resume a paused schedule (Story 81.1).
    ///
    /// TODO: UPDATE report_schedules SET is_active = true, status = 'active',
    ///       updated_at = NOW() WHERE id = $1 RETURNING *
    pub async fn resume(&self, id: Uuid) -> Result<ReportSchedule, AppError> {
        let mut sched = Self::stub_schedule(id);
        sched.is_active = true;
        sched.status = report_schedule_status::ACTIVE.to_string();
        sched.updated_at = Utc::now();
        Ok(sched)
    }

    // ============================================================================
    // Execution history
    // ============================================================================

    /// List execution history for a schedule (Story 81.2).
    ///
    /// TODO: SELECT * FROM report_executions WHERE schedule_id = $1
    ///       AND ($2::text IS NULL OR status = $2) ... ORDER BY started_at DESC
    pub async fn list_executions(
        &self,
        query: ExecutionHistoryQuery,
    ) -> Result<ExecutionHistoryResponse, AppError> {
        let exec = Self::stub_execution(query.schedule_id);
        let executions = vec![exec];
        let total = executions.len() as i64;
        let has_more = (query.offset + query.limit) < total;
        Ok(ExecutionHistoryResponse {
            executions,
            total,
            has_more,
        })
    }

    /// Get a single execution by ID (Story 81.2).
    ///
    /// TODO: SELECT * FROM report_executions WHERE id = $1
    pub async fn get_execution(&self, id: Uuid) -> Result<Option<ReportExecution>, AppError> {
        let now = Utc::now();
        let exec = ReportExecution {
            id,
            schedule_id: Uuid::new_v4(),
            status: report_execution_status::COMPLETED.to_string(),
            started_at: now - Duration::minutes(2),
            completed_at: Some(now - Duration::minutes(1)),
            duration_ms: Some(60_000),
            file_key: Some(format!("reports/executions/{}.pdf", id)),
            file_name: Some("report.pdf".to_string()),
            file_size: Some(102_400),
            error_code: None,
            error_message: None,
            error_details: None,
            created_at: now - Duration::minutes(2),
            download_url: None, // populated by the handler layer
        };
        Ok(Some(exec))
    }

    /// Get presigned download URL for a completed execution (Story 81.2).
    ///
    /// TODO: look up file_key, generate S3 presigned URL
    pub async fn get_download_url(
        &self,
        execution_id: Uuid,
    ) -> Result<ExecutionDownloadUrl, AppError> {
        Ok(ExecutionDownloadUrl {
            url: format!(
                "/api/v1/reports/executions/{}/download/file.pdf",
                execution_id
            ),
            expires_at: Utc::now() + Duration::hours(1),
            file_name: "report.pdf".to_string(),
            content_type: "application/pdf".to_string(),
        })
    }

    /// Retry a failed execution (Story 81.2).
    ///
    /// TODO: UPDATE report_executions SET status = 'pending', error_message = NULL,
    ///       completed_at = NULL WHERE id = $1 AND status = 'failed' RETURNING *
    pub async fn retry_execution(&self, id: Uuid) -> Result<ReportExecution, AppError> {
        let now = Utc::now();
        let exec = ReportExecution {
            id,
            schedule_id: Uuid::new_v4(),
            status: report_execution_status::PENDING.to_string(),
            started_at: now,
            completed_at: None,
            duration_ms: None,
            file_key: None,
            file_name: None,
            file_size: None,
            error_code: None,
            error_message: None,
            error_details: None,
            created_at: now,
            download_url: None,
        };
        Ok(exec)
    }

    /// Update a schedule's cron expression, recipients, and/or enabled flag (gap-81-1).
    ///
    /// All parameters are optional; only non-`None` values are applied.
    ///
    /// TODO: UPDATE report_schedules
    ///       SET time        = COALESCE($2, time),
    ///           recipients  = COALESCE($3, recipients),
    ///           is_active   = COALESCE($4, is_active),
    ///           status      = COALESCE($5, status),
    ///           updated_at  = NOW()
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
            // Stored in `time` until a dedicated column is added via migration.
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
