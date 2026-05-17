//! Outage model (UC-12: Utility Outages).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Outage status enum values.
pub mod outage_status {
    pub const PLANNED: &str = "planned";
    pub const ONGOING: &str = "ongoing";
    pub const RESOLVED: &str = "resolved";
    pub const CANCELLED: &str = "cancelled";

    pub const ALL: &[&str] = &[PLANNED, ONGOING, RESOLVED, CANCELLED];
}

/// Outage commodity type enum values.
pub mod outage_commodity {
    pub const WATER: &str = "water";
    pub const ELECTRICITY: &str = "electricity";
    pub const GAS: &str = "gas";
    pub const HEATING: &str = "heating";
    pub const INTERNET: &str = "internet";
    pub const OTHER: &str = "other";

    pub const ALL: &[&str] = &[WATER, ELECTRICITY, GAS, HEATING, INTERNET, OTHER];
}

/// Outage severity enum values.
pub mod outage_severity {
    pub const LOW: &str = "low";
    pub const MEDIUM: &str = "medium";
    pub const HIGH: &str = "high";
    pub const CRITICAL: &str = "critical";

    pub const ALL: &[&str] = &[LOW, MEDIUM, HIGH, CRITICAL];
}

// ============================================================================
// Outage
// ============================================================================

/// Outage entity from database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct Outage {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub created_by: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub commodity: String,
    pub severity: String,
    pub status: String,
    pub building_ids: serde_json::Value,
    pub scheduled_start: DateTime<Utc>,
    pub scheduled_end: Option<DateTime<Utc>>,
    pub actual_start: Option<DateTime<Utc>>,
    pub actual_end: Option<DateTime<Utc>>,
    pub external_reference: Option<String>,
    pub supplier_name: Option<String>,
    pub resolution_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Outage {
    /// Check if outage is planned.
    pub fn is_planned(&self) -> bool {
        self.status == outage_status::PLANNED
    }

    /// Check if outage is ongoing.
    pub fn is_ongoing(&self) -> bool {
        self.status == outage_status::ONGOING
    }

    /// Check if outage is resolved.
    pub fn is_resolved(&self) -> bool {
        self.status == outage_status::RESOLVED
    }

    /// Check if outage is cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.status == outage_status::CANCELLED
    }

    /// Check if outage is currently active (planned or ongoing).
    pub fn is_active(&self) -> bool {
        self.is_planned() || self.is_ongoing()
    }

    /// Check if outage can be edited.
    pub fn can_edit(&self) -> bool {
        self.is_planned() || self.is_ongoing()
    }
}

/// Summary view of an outage.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct OutageSummary {
    pub id: Uuid,
    pub title: String,
    pub commodity: String,
    pub severity: String,
    pub status: String,
    pub scheduled_start: DateTime<Utc>,
    pub scheduled_end: Option<DateTime<Utc>>,
    pub actual_start: Option<DateTime<Utc>>,
    pub actual_end: Option<DateTime<Utc>>,
}

/// Outage with additional details.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OutageWithDetails {
    #[serde(flatten)]
    pub outage: Outage,
    #[serde(rename = "creatorName")]
    pub creator_name: String,
    pub building_names: Vec<String>,
    pub notification_count: i64,
    pub read_count: i64,
}

/// Data for creating an outage.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateOutage {
    pub organization_id: Uuid,
    pub created_by: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub commodity: String,
    pub severity: String,
    pub building_ids: Vec<Uuid>,
    pub scheduled_start: DateTime<Utc>,
    pub scheduled_end: Option<DateTime<Utc>>,
    pub external_reference: Option<String>,
    pub supplier_name: Option<String>,
}

/// Data for updating an outage.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateOutage {
    pub title: Option<String>,
    pub description: Option<String>,
    pub commodity: Option<String>,
    pub severity: Option<String>,
    pub building_ids: Option<Vec<Uuid>>,
    pub scheduled_start: Option<DateTime<Utc>>,
    pub scheduled_end: Option<DateTime<Utc>>,
    pub external_reference: Option<String>,
    pub supplier_name: Option<String>,
}

/// Data for starting an outage (changing from planned to ongoing).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StartOutage {
    /// Actual start time. Defaults to now if not provided.
    pub actual_start: Option<DateTime<Utc>>,
}

/// Data for resolving an outage.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ResolveOutage {
    /// Actual end time. Defaults to now if not provided.
    pub actual_end: Option<DateTime<Utc>>,
    /// Resolution notes.
    pub resolution_notes: Option<String>,
}

/// Data for cancelling an outage.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CancelOutage {
    /// Reason for cancellation.
    pub reason: Option<String>,
}

// ============================================================================
// Outage Notification
// ============================================================================

/// Outage notification entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct OutageNotification {
    pub id: Uuid,
    pub outage_id: Uuid,
    pub user_id: Uuid,
    pub notified_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub notification_method: String,
}

/// Data for creating an outage notification.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateOutageNotification {
    pub outage_id: Uuid,
    pub user_id: Uuid,
    pub notification_method: String,
}

/// Data for marking an outage notification as read.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MarkOutageRead {
    pub outage_id: Uuid,
    pub user_id: Uuid,
}

// ============================================================================
// Query types
// ============================================================================

/// Query for listing outages.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct OutageListQuery {
    pub status: Option<Vec<String>>,
    pub commodity: Option<Vec<String>>,
    pub severity: Option<Vec<String>>,
    pub building_id: Option<Uuid>,
    pub from_date: Option<chrono::NaiveDate>,
    pub to_date: Option<chrono::NaiveDate>,
    pub active_only: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Statistics for outages.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct OutageStatistics {
    pub total: i64,
    pub planned: i64,
    pub ongoing: i64,
    pub resolved: i64,
    pub cancelled: i64,
}

/// Commodity statistics.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct CommodityCount {
    pub commodity: String,
    pub count: i64,
}

/// Active outages dashboard data.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OutageDashboard {
    pub active_count: i64,
    pub planned_count: i64,
    pub ongoing_count: i64,
    pub by_commodity: Vec<CommodityCount>,
    pub upcoming: Vec<OutageSummary>,
}
