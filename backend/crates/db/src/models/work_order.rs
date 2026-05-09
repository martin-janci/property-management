//! Work orders and maintenance scheduling models (Epic 20).

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Work Orders (Story 20.2)
// ============================================================================

/// Work order for maintenance tasks.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct WorkOrder {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Uuid,
    pub equipment_id: Option<Uuid>,
    pub fault_id: Option<Uuid>,
    pub title: String,
    pub description: String,
    pub priority: String,
    pub work_type: String,
    pub assigned_to: Option<Uuid>,
    pub vendor_id: Option<Uuid>,
    pub scheduled_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub estimated_cost: Option<Decimal>,
    pub actual_cost: Option<Decimal>,
    pub status: String,
    pub resolution_notes: Option<String>,
    pub source: String,
    pub schedule_id: Option<Uuid>,
    pub attachments: Vec<Uuid>,
    pub tags: Vec<String>,
    #[schema(value_type = serde_json::Value)]
    pub metadata: sqlx::types::Json<serde_json::Value>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Work order activity update.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct WorkOrderUpdate {
    pub id: Uuid,
    pub work_order_id: Uuid,
    pub user_id: Uuid,
    pub update_type: String,
    pub content: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Work order priority constants.
pub mod work_order_priority {
    pub const LOW: &str = "low";
    pub const MEDIUM: &str = "medium";
    pub const HIGH: &str = "high";
    pub const URGENT: &str = "urgent";
    pub const ALL: &[&str] = &[LOW, MEDIUM, HIGH, URGENT];
}

/// Work order status constants.
pub mod work_order_status {
    pub const OPEN: &str = "open";
    pub const ASSIGNED: &str = "assigned";
    pub const IN_PROGRESS: &str = "in_progress";
    pub const ON_HOLD: &str = "on_hold";
    pub const COMPLETED: &str = "completed";
    pub const CANCELLED: &str = "cancelled";
    pub const ALL: &[&str] = &[OPEN, ASSIGNED, IN_PROGRESS, ON_HOLD, COMPLETED, CANCELLED];
}

/// Work order type constants.
pub mod work_order_type {
    pub const PREVENTIVE: &str = "preventive";
    pub const CORRECTIVE: &str = "corrective";
    pub const EMERGENCY: &str = "emergency";
    pub const INSPECTION: &str = "inspection";
    pub const ALL: &[&str] = &[PREVENTIVE, CORRECTIVE, EMERGENCY, INSPECTION];
}

/// Work order source constants.
pub mod work_order_source {
    pub const MANUAL: &str = "manual";
    pub const FAULT: &str = "fault";
    pub const SCHEDULE: &str = "schedule";
    pub const PREDICTION: &str = "prediction";
    pub const ALL: &[&str] = &[MANUAL, FAULT, SCHEDULE, PREDICTION];
}

/// Update type constants.
pub mod update_type {
    pub const COMMENT: &str = "comment";
    pub const STATUS_CHANGE: &str = "status_change";
    pub const ASSIGNMENT: &str = "assignment";
    pub const COST_UPDATE: &str = "cost_update";
    pub const ALL: &[&str] = &[COMMENT, STATUS_CHANGE, ASSIGNMENT, COST_UPDATE];
}

// ============================================================================
// Maintenance Schedules (Story 20.3)
// ============================================================================

/// Recurring maintenance schedule.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct MaintenanceSchedule {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub equipment_id: Option<Uuid>,
    pub name: String,
    pub description: String,
    pub work_type: String,
    pub frequency: String,
    pub day_of_week: Option<i32>,
    pub day_of_month: Option<i32>,
    pub month_of_year: Option<i32>,
    pub default_assignee: Option<Uuid>,
    pub default_vendor_id: Option<Uuid>,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub next_due_date: NaiveDate,
    pub last_run_date: Option<NaiveDate>,
    pub auto_create_work_order: bool,
    pub advance_days: Option<i32>,
    pub estimated_duration_hours: Option<Decimal>,
    pub estimated_cost: Option<Decimal>,
    pub is_active: bool,
    #[schema(value_type = serde_json::Value)]
    pub checklist: sqlx::types::Json<serde_json::Value>,
    #[schema(value_type = serde_json::Value)]
    pub metadata: sqlx::types::Json<serde_json::Value>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Schedule execution history.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ScheduleExecution {
    pub id: Uuid,
    pub schedule_id: Uuid,
    pub work_order_id: Option<Uuid>,
    pub due_date: NaiveDate,
    pub executed_at: Option<DateTime<Utc>>,
    pub status: String,
    pub skipped_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Schedule frequency constants.
pub mod schedule_frequency {
    pub const DAILY: &str = "daily";
    pub const WEEKLY: &str = "weekly";
    pub const BIWEEKLY: &str = "biweekly";
    pub const MONTHLY: &str = "monthly";
    pub const QUARTERLY: &str = "quarterly";
    pub const SEMIANNUAL: &str = "semiannual";
    pub const ANNUAL: &str = "annual";
    pub const ALL: &[&str] = &[
        DAILY, WEEKLY, BIWEEKLY, MONTHLY, QUARTERLY, SEMIANNUAL, ANNUAL,
    ];
}

/// Schedule execution status constants.
pub mod schedule_execution_status {
    pub const PENDING: &str = "pending";
    pub const CREATED: &str = "created";
    pub const SKIPPED: &str = "skipped";
    pub const COMPLETED: &str = "completed";
    pub const ALL: &[&str] = &[PENDING, CREATED, SKIPPED, COMPLETED];
}

// ============================================================================
// Request/Response DTOs
// ============================================================================

/// Request to create a work order.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateWorkOrder {
    pub building_id: Uuid,
    pub equipment_id: Option<Uuid>,
    pub fault_id: Option<Uuid>,
    pub title: String,
    pub description: String,
    pub priority: Option<String>,
    pub work_type: Option<String>,
    pub assigned_to: Option<Uuid>,
    pub vendor_id: Option<Uuid>,
    pub scheduled_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub estimated_cost: Option<Decimal>,
    pub tags: Option<Vec<String>>,
    pub metadata: Option<serde_json::Value>,
}

/// Request to update a work order.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateWorkOrder {
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub work_type: Option<String>,
    pub assigned_to: Option<Uuid>,
    pub vendor_id: Option<Uuid>,
    pub scheduled_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub estimated_cost: Option<Decimal>,
    pub actual_cost: Option<Decimal>,
    pub status: Option<String>,
    pub resolution_notes: Option<String>,
    pub tags: Option<Vec<String>>,
    pub metadata: Option<serde_json::Value>,
}

/// Request to add work order update/comment.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AddWorkOrderUpdate {
    pub update_type: Option<String>,
    pub content: String,
}

/// Request to create a maintenance schedule.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateMaintenanceSchedule {
    pub building_id: Option<Uuid>,
    pub equipment_id: Option<Uuid>,
    pub name: String,
    pub description: String,
    pub work_type: Option<String>,
    pub frequency: String,
    pub day_of_week: Option<i32>,
    pub day_of_month: Option<i32>,
    pub month_of_year: Option<i32>,
    pub default_assignee: Option<Uuid>,
    pub default_vendor_id: Option<Uuid>,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub auto_create_work_order: Option<bool>,
    pub advance_days: Option<i32>,
    pub estimated_duration_hours: Option<Decimal>,
    pub estimated_cost: Option<Decimal>,
    pub checklist: Option<Vec<String>>,
    pub metadata: Option<serde_json::Value>,
}

/// Request to update a maintenance schedule.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateMaintenanceSchedule {
    pub name: Option<String>,
    pub description: Option<String>,
    pub work_type: Option<String>,
    pub frequency: Option<String>,
    pub day_of_week: Option<i32>,
    pub day_of_month: Option<i32>,
    pub month_of_year: Option<i32>,
    pub default_assignee: Option<Uuid>,
    pub default_vendor_id: Option<Uuid>,
    pub end_date: Option<NaiveDate>,
    pub auto_create_work_order: Option<bool>,
    pub advance_days: Option<i32>,
    pub estimated_duration_hours: Option<Decimal>,
    pub estimated_cost: Option<Decimal>,
    pub is_active: Option<bool>,
    pub checklist: Option<Vec<String>>,
    pub metadata: Option<serde_json::Value>,
}

// ============================================================================
// Query/Summary DTOs
// ============================================================================

/// Query parameters for work orders.
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct WorkOrderQuery {
    pub building_id: Option<Uuid>,
    pub equipment_id: Option<Uuid>,
    pub fault_id: Option<Uuid>,
    pub assigned_to: Option<Uuid>,
    pub vendor_id: Option<Uuid>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub work_type: Option<String>,
    pub source: Option<String>,
    pub due_before: Option<NaiveDate>,
    pub due_after: Option<NaiveDate>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

/// Query parameters for maintenance schedules.
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct ScheduleQuery {
    pub building_id: Option<Uuid>,
    pub equipment_id: Option<Uuid>,
    pub frequency: Option<String>,
    pub is_active: Option<bool>,
    pub due_before: Option<NaiveDate>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

/// Work order with related entity names.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct WorkOrderWithDetails {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub priority: String,
    pub work_type: String,
    pub status: String,
    pub building_name: String,
    pub equipment_name: Option<String>,
    pub assigned_to_name: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
}

/// Work order statistics summary.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct WorkOrderStatistics {
    pub total: i64,
    pub open: i64,
    pub in_progress: i64,
    pub completed: i64,
    pub overdue: i64,
    pub avg_completion_days: Option<f64>,
    pub total_cost: Option<Decimal>,
}

/// Upcoming schedule summary.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct UpcomingSchedule {
    pub id: Uuid,
    pub name: String,
    pub frequency: String,
    pub next_due_date: NaiveDate,
    pub equipment_name: Option<String>,
    pub building_name: String,
    pub days_until_due: i32,
}

/// Service history report entry (Story 20.4).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ServiceHistoryEntry {
    pub id: Uuid,
    pub work_order_id: Uuid,
    pub title: String,
    pub work_type: String,
    pub completed_at: Option<DateTime<Utc>>,
    pub actual_cost: Option<Decimal>,
    pub resolution_notes: Option<String>,
    pub equipment_name: Option<String>,
}

/// Maintenance cost summary by category.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct MaintenanceCostSummary {
    pub work_type: String,
    pub work_order_count: i64,
    pub total_cost: Option<Decimal>,
    pub avg_cost: Option<Decimal>,
}
