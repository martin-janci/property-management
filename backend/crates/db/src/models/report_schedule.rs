//! Report schedule models (Epic 81: Schedule Management & Execution History).
//!
//! Types for report schedule pause/resume and execution history.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Status constants
// ============================================================================

/// Status constants for report schedules.
pub mod report_schedule_status {
    pub const ACTIVE: &str = "active";
    pub const PAUSED: &str = "paused";
    pub const INACTIVE: &str = "inactive";
}

/// Status constants for report executions.
pub mod report_execution_status {
    pub const PENDING: &str = "pending";
    pub const RUNNING: &str = "running";
    pub const COMPLETED: &str = "completed";
    pub const FAILED: &str = "failed";
    pub const CANCELLED: &str = "cancelled";
    pub const SKIPPED: &str = "skipped";
}

// ============================================================================
// Models
// ============================================================================

/// A report schedule (in-memory representation).
///
/// When the `report_schedules` table is added via migration this struct maps
/// to that table row. Until then the repository returns stub data.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReportSchedule {
    pub id: Uuid,
    pub report_id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub frequency: String,
    pub day_of_week: Option<i32>,
    pub day_of_month: Option<i32>,
    pub time: String,
    pub timezone: String,
    pub format: String,
    pub recipients: Vec<String>,
    pub is_active: bool,
    /// "active", "paused", or "inactive"
    pub status: String,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A single report execution record.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReportExecution {
    pub id: Uuid,
    pub schedule_id: Uuid,
    /// "pending" | "running" | "completed" | "failed" | "cancelled" | "skipped"
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub file_key: Option<String>,
    pub file_name: Option<String>,
    pub file_size: Option<i64>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub error_details: Option<String>,
    pub created_at: DateTime<Utc>,
    /// Pre-computed download URL for completed executions with a generated file.
    ///
    /// Set to `Some("/api/v1/reports/executions/{id}/download")` when `file_key`
    /// is present; `None` for pending/running/failed executions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
}

/// Query parameters for listing execution history.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExecutionHistoryQuery {
    pub schedule_id: Uuid,
    pub status: Option<String>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub limit: i64,
    pub offset: i64,
}

/// Paginated execution history response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExecutionHistoryResponse {
    pub executions: Vec<ReportExecution>,
    pub total: i64,
    pub has_more: bool,
}

/// Download URL response for a completed execution.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExecutionDownloadUrl {
    pub url: String,
    pub expires_at: DateTime<Utc>,
    pub file_name: String,
    pub content_type: String,
}
