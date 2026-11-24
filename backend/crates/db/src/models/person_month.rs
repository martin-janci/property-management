//! Person month model (Epic 3, Story 3.5).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Person month source enum values.
pub mod person_month_source {
    pub const MANUAL: &str = "manual";
    pub const CALCULATED: &str = "calculated";
    pub const IMPORTED: &str = "imported";
}

/// Person month entity from database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct PersonMonth {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub year: i32,
    pub month: i32,
    pub count: i32,
    pub source: String,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
    pub updated_by: Option<Uuid>,
}

impl PersonMonth {
    /// Check if this is manually entered.
    pub fn is_manual(&self) -> bool {
        self.source == person_month_source::MANUAL
    }

    /// Get source display name.
    pub fn source_display(&self) -> &str {
        match self.source.as_str() {
            "manual" => "Manual Entry",
            "calculated" => "Auto-calculated",
            "imported" => "Imported",
            _ => &self.source,
        }
    }

    /// Get period as string (e.g., "2024-03").
    pub fn period_string(&self) -> String {
        format!("{:04}-{:02}", self.year, self.month)
    }
}

/// Summary view with unit info.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct PersonMonthWithUnit {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub unit_designation: String,
    pub year: i32,
    pub month: i32,
    pub count: i32,
    pub source: String,
}

/// Data for creating/updating person months.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatePersonMonth {
    pub unit_id: Uuid,
    pub year: i32,
    pub month: i32,
    pub count: i32,
    #[serde(default = "default_source")]
    pub source: String,
    pub notes: Option<String>,
}

fn default_source() -> String {
    "manual".to_string()
}

/// Bulk upsert request for person months.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BulkPersonMonthEntry {
    pub unit_id: Uuid,
    pub count: i32,
}

/// Bulk upsert request for a specific month.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BulkUpsertPersonMonths {
    pub year: i32,
    pub month: i32,
    pub entries: Vec<BulkPersonMonthEntry>,
}

/// Data for updating a person month.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdatePersonMonth {
    pub count: Option<i32>,
    pub source: Option<String>,
    pub notes: Option<String>,
}

/// Yearly person month summary for a unit.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct YearlyPersonMonthSummary {
    pub unit_id: Uuid,
    pub year: i32,
    pub months: Vec<MonthlyCount>,
    pub total: i32,
}

/// Monthly count entry.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MonthlyCount {
    pub month: i32,
    pub count: i32,
    pub source: String,
}

/// Building-level person month aggregate.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct BuildingPersonMonthSummary {
    pub building_id: Uuid,
    pub year: i32,
    pub month: i32,
    pub total_count: i64,
    pub unit_count: i64,
}

/// Status values of a resident person-month submission.
pub mod person_month_submission_status {
    pub const PENDING: &str = "pending";
    pub const APPROVED: &str = "approved";
    pub const REJECTED: &str = "rejected";
}

/// A resident-proposed person-month count awaiting (or after) manager review.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct PersonMonthSubmission {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub year: i32,
    pub month: i32,
    pub count: i32,
    pub note: Option<String>,
    pub status: String,
    pub submitted_by: Uuid,
    pub submitted_at: DateTime<Utc>,
    pub decided_by: Option<Uuid>,
    pub decided_at: Option<DateTime<Utc>>,
    pub decision_note: Option<String>,
}

/// A submission joined with its unit designation and submitter email, for
/// the manager review list.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct PersonMonthSubmissionWithUnit {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub unit_designation: String,
    pub year: i32,
    pub month: i32,
    pub count: i32,
    pub current_count: Option<i32>,
    pub note: Option<String>,
    pub status: String,
    pub submitted_by: Uuid,
    pub submitter_email: Option<String>,
    pub submitted_at: DateTime<Utc>,
}
