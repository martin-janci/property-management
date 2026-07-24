//! Unit model (Epic 2B, UC-15).

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Unit entity from database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct Unit {
    pub id: Uuid,
    pub building_id: Uuid,

    // Location within building
    pub entrance: Option<String>,
    pub designation: String,
    pub floor: i32,

    // Unit details
    pub unit_type: String,
    pub size_sqm: Option<Decimal>,
    pub rooms: Option<i32>,
    #[sqlx(try_from = "rust_decimal::Decimal")]
    pub ownership_share: Decimal,

    // Occupancy
    pub occupancy_status: String,

    // Additional info
    pub description: Option<String>,
    pub notes: Option<String>,
    pub settings: serde_json::Value,

    // Status
    pub status: String,

    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Unit {
    /// Check if unit is active.
    pub fn is_active(&self) -> bool {
        self.status == "active"
    }

    /// Get floor display string (handles negative floors for basement).
    pub fn floor_display(&self) -> String {
        match self.floor {
            f if f < 0 => format!("B{}", f.abs()),
            0 => "G".to_string(),
            f => format!("{}", f),
        }
    }

    /// Get unit type display name.
    pub fn unit_type_display(&self) -> &str {
        match self.unit_type.as_str() {
            "apartment" => "Apartment",
            "commercial" => "Commercial",
            "parking" => "Parking",
            "storage" => "Storage",
            "other" => "Other",
            _ => &self.unit_type,
        }
    }
}

/// Summary view of a unit (for list views).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct UnitSummary {
    pub id: Uuid,
    pub building_id: Uuid,
    pub designation: String,
    pub floor: i32,
    pub unit_type: String,
    pub occupancy_status: String,
    pub status: String,
}

/// Unit with owner information for display.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UnitWithOwners {
    #[serde(flatten)]
    pub unit: Unit,
    pub owners: Vec<UnitOwnerInfo>,
}

/// Owner information for a unit.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct UnitOwnerInfo {
    pub user_id: Uuid,
    pub user_name: String,
    pub user_email: String,
    #[sqlx(try_from = "rust_decimal::Decimal")]
    pub ownership_percentage: Decimal,
    pub is_primary: bool,
}

/// Data for creating a new unit.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateUnit {
    pub building_id: Uuid,
    pub entrance: Option<String>,
    pub designation: String,
    #[serde(default)]
    pub floor: i32,
    #[serde(default = "default_unit_type")]
    pub unit_type: String,
    pub size_sqm: Option<Decimal>,
    pub rooms: Option<i32>,
    #[serde(default = "default_ownership_share")]
    pub ownership_share: Decimal,
    pub description: Option<String>,
}

fn default_unit_type() -> String {
    "apartment".to_string()
}

fn default_ownership_share() -> Decimal {
    Decimal::new(10000, 2) // 100.00
}

/// Data for updating a unit.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateUnit {
    pub entrance: Option<String>,
    pub designation: Option<String>,
    pub floor: Option<i32>,
    pub unit_type: Option<String>,
    pub size_sqm: Option<Decimal>,
    pub rooms: Option<i32>,
    pub ownership_share: Option<Decimal>,
    pub occupancy_status: Option<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub settings: Option<serde_json::Value>,
}

/// Unit owner assignment from database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct UnitOwner {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub user_id: Uuid,
    #[sqlx(try_from = "rust_decimal::Decimal")]
    pub ownership_percentage: Decimal,
    pub is_primary: bool,
    pub valid_from: NaiveDate,
    pub valid_until: Option<NaiveDate>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Data for assigning an owner to a unit.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AssignUnitOwner {
    pub unit_id: Uuid,
    pub user_id: Uuid,
    #[serde(default = "default_ownership_share")]
    pub ownership_percentage: Decimal,
    #[serde(default = "default_true")]
    pub is_primary: bool,
    pub valid_from: Option<NaiveDate>,
}

fn default_true() -> bool {
    true
}

/// Unit type enum.
pub mod unit_type {
    pub const APARTMENT: &str = "apartment";
    pub const COMMERCIAL: &str = "commercial";
    pub const PARKING: &str = "parking";
    pub const STORAGE: &str = "storage";
    pub const OTHER: &str = "other";
}

/// Occupancy status enum.
pub mod occupancy_status {
    pub const OWNER_OCCUPIED: &str = "owner_occupied";
    pub const RENTED: &str = "rented";
    pub const VACANT: &str = "vacant";
    pub const UNKNOWN: &str = "unknown";
}

/// Unit status enum.
pub mod unit_status {
    pub const ACTIVE: &str = "active";
    pub const ARCHIVED: &str = "archived";
}
