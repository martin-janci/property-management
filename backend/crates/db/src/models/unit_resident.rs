//! Unit resident model (Epic 3, Story 3.3).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Resident type enum values.
pub mod resident_type {
    pub const OWNER: &str = "owner";
    pub const TENANT: &str = "tenant";
    pub const FAMILY_MEMBER: &str = "family_member";
    pub const SUBTENANT: &str = "subtenant";
}

/// Unit resident entity from database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct UnitResident {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub user_id: Uuid,
    pub resident_type: String,
    pub is_primary: bool,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub receives_notifications: bool,
    pub receives_mail: bool,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
}

impl UnitResident {
    /// Check if resident is currently active.
    pub fn is_active(&self) -> bool {
        self.end_date.is_none()
    }

    /// Get resident type display name.
    pub fn resident_type_display(&self) -> &str {
        match self.resident_type.as_str() {
            "owner" => "Owner",
            "tenant" => "Tenant",
            "family_member" => "Family Member",
            "subtenant" => "Subtenant",
            _ => &self.resident_type,
        }
    }
}

/// Summary view of a unit resident.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct UnitResidentSummary {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub user_id: Uuid,
    pub resident_type: String,
    pub is_primary: bool,
    pub is_active: bool,
}

/// Resident with user info for display.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct UnitResidentWithUser {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub user_id: Uuid,
    pub user_name: String,
    pub user_email: String,
    pub resident_type: String,
    pub is_primary: bool,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
}

/// Data for adding a resident to a unit.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateUnitResident {
    pub unit_id: Uuid,
    pub user_id: Uuid,
    pub resident_type: String,
    #[serde(default)]
    pub is_primary: bool,
    pub start_date: Option<NaiveDate>,
    #[serde(default = "default_true")]
    pub receives_notifications: bool,
    #[serde(default = "default_true")]
    pub receives_mail: bool,
    pub notes: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Data for updating a unit resident.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateUnitResident {
    pub resident_type: Option<String>,
    pub is_primary: Option<bool>,
    pub end_date: Option<NaiveDate>,
    pub receives_notifications: Option<bool>,
    pub receives_mail: Option<bool>,
    pub notes: Option<String>,
}

/// Request to end a residency.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EndResidency {
    pub end_date: NaiveDate,
}

/// Resident-facing "My Unit" row (Epic 3, Story 3.6).
///
/// One row per active `unit_residents` association for the authenticated user,
/// flattened with the unit's own details and the building's public address.
///
/// Privacy: this view is resolved strictly by `user_id = <caller>` and selects
/// only the caller's own association plus the unit/building columns. It never
/// includes other residents' or owners' identities — that PII filtering is
/// enforced here at the query layer, not in the UI.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MyUnitRow {
    // Caller's own association with the unit.
    pub resident_id: Uuid,
    pub resident_type: String,
    pub is_primary: bool,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub receives_notifications: bool,
    pub receives_mail: bool,
    // Unit details (no manager-internal `notes`).
    pub unit_id: Uuid,
    pub building_id: Uuid,
    pub entrance: Option<String>,
    pub designation: String,
    pub floor: i32,
    pub unit_type: String,
    pub size_sqm: Option<rust_decimal::Decimal>,
    pub rooms: Option<i32>,
    pub ownership_share: rust_decimal::Decimal,
    pub occupancy_status: String,
    pub description: Option<String>,
    pub unit_status: String,
    // Building address context only — no owner/resident PII.
    pub building_name: Option<String>,
    pub building_street: String,
    pub building_city: String,
    pub building_postal_code: String,
}
