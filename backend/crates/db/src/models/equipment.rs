//! Equipment and predictive maintenance models (Epic 13, Story 13.3).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Equipment tracked for maintenance.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Equipment {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Uuid,
    pub facility_id: Option<Uuid>,
    pub name: String,
    pub category: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub installation_date: Option<NaiveDate>,
    pub warranty_expires: Option<NaiveDate>,
    pub expected_lifespan_years: Option<i32>,
    pub maintenance_interval_days: Option<i32>,
    pub last_maintenance_date: Option<NaiveDate>,
    pub next_maintenance_due: Option<NaiveDate>,
    pub status: String,
    pub notes: Option<String>,
    pub metadata: sqlx::types::Json<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Equipment maintenance record.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EquipmentMaintenance {
    pub id: Uuid,
    pub equipment_id: Uuid,
    pub maintenance_type: String,
    pub description: String,
    pub performed_by: Option<Uuid>,
    pub external_vendor: Option<String>,
    pub cost: Option<rust_decimal::Decimal>,
    pub parts_replaced: Vec<String>,
    pub fault_id: Option<Uuid>,
    pub scheduled_date: Option<NaiveDate>,
    pub completed_date: Option<NaiveDate>,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Predictive maintenance prediction.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MaintenancePrediction {
    pub id: Uuid,
    pub equipment_id: Uuid,
    pub risk_score: f64,
    pub predicted_failure_date: Option<NaiveDate>,
    pub confidence: f64,
    pub recommendation: String,
    pub factors: sqlx::types::Json<serde_json::Value>,
    pub acknowledged: bool,
    pub acknowledged_by: Option<Uuid>,
    pub action_taken: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Equipment status constants.
pub mod equipment_status {
    pub const OPERATIONAL: &str = "operational";
    pub const NEEDS_MAINTENANCE: &str = "needs_maintenance";
    pub const UNDER_REPAIR: &str = "under_repair";
    pub const DECOMMISSIONED: &str = "decommissioned";
    pub const ALL: &[&str] = &[OPERATIONAL, NEEDS_MAINTENANCE, UNDER_REPAIR, DECOMMISSIONED];
}

/// Maintenance type constants.
pub mod maintenance_type {
    pub const PREVENTIVE: &str = "preventive";
    pub const CORRECTIVE: &str = "corrective";
    pub const EMERGENCY: &str = "emergency";
    pub const INSPECTION: &str = "inspection";
    pub const ALL: &[&str] = &[PREVENTIVE, CORRECTIVE, EMERGENCY, INSPECTION];
}

/// Maintenance status constants.
pub mod maintenance_status {
    pub const SCHEDULED: &str = "scheduled";
    pub const IN_PROGRESS: &str = "in_progress";
    pub const COMPLETED: &str = "completed";
    pub const CANCELLED: &str = "cancelled";
    pub const ALL: &[&str] = &[SCHEDULED, IN_PROGRESS, COMPLETED, CANCELLED];
}

/// Request to create equipment.
///
/// `organization_id` is intentionally NOT part of this struct: it is always
/// derived from the verified `RequestPrincipal` server-side, never trusted
/// from client input (prevents IDOR).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEquipment {
    pub building_id: Uuid,
    pub facility_id: Option<Uuid>,
    pub name: String,
    pub category: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub installation_date: Option<NaiveDate>,
    pub warranty_expires: Option<NaiveDate>,
    pub expected_lifespan_years: Option<i32>,
    pub maintenance_interval_days: Option<i32>,
    pub notes: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Request to update equipment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEquipment {
    pub name: Option<String>,
    pub category: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub installation_date: Option<NaiveDate>,
    pub warranty_expires: Option<NaiveDate>,
    pub expected_lifespan_years: Option<i32>,
    pub maintenance_interval_days: Option<i32>,
    pub status: Option<String>,
    pub notes: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Request to create maintenance record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMaintenance {
    pub equipment_id: Uuid,
    pub maintenance_type: String,
    pub description: String,
    pub performed_by: Option<Uuid>,
    pub external_vendor: Option<String>,
    pub cost: Option<rust_decimal::Decimal>,
    pub parts_replaced: Option<Vec<String>>,
    pub fault_id: Option<Uuid>,
    pub scheduled_date: Option<NaiveDate>,
    pub notes: Option<String>,
}

/// Request to update maintenance record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMaintenance {
    pub maintenance_type: Option<String>,
    pub description: Option<String>,
    pub performed_by: Option<Uuid>,
    pub external_vendor: Option<String>,
    pub cost: Option<rust_decimal::Decimal>,
    pub parts_replaced: Option<Vec<String>>,
    pub completed_date: Option<NaiveDate>,
    pub status: Option<String>,
    pub notes: Option<String>,
}

/// Equipment with maintenance summary.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EquipmentWithSummary {
    pub id: Uuid,
    pub name: String,
    pub category: String,
    pub building_name: String,
    pub status: String,
    pub next_maintenance_due: Option<NaiveDate>,
    pub maintenance_count: i64,
    pub last_maintenance_date: Option<NaiveDate>,
    pub risk_score: Option<f64>,
}

/// Query parameters for equipment.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EquipmentQuery {
    pub building_id: Option<Uuid>,
    pub facility_id: Option<Uuid>,
    pub category: Option<String>,
    pub status: Option<String>,
    pub needs_maintenance: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
