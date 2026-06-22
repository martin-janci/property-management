//! Building model (Epic 2B, UC-15).

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Building entity from database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct Building {
    pub id: Uuid,
    pub organization_id: Uuid,

    // Address
    pub street: String,
    pub city: String,
    pub postal_code: String,
    pub country: String,

    // Basic info
    pub name: Option<String>,
    pub description: Option<String>,

    // Building details
    pub year_built: Option<i32>,
    pub total_floors: i32,
    pub total_entrances: i32,

    // Geocoded coordinates (Story 3.1 AC3). NULL when geocoding is
    // unconfigured or the address could not be resolved.
    pub latitude: Option<Decimal>,
    pub longitude: Option<Decimal>,

    // Flexible JSONB fields
    pub amenities: serde_json::Value,
    pub contacts: serde_json::Value,
    pub settings: serde_json::Value,

    // Status
    pub status: String,

    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Building {
    /// Check if building is active.
    pub fn is_active(&self) -> bool {
        self.status == "active"
    }

    /// Whether the building has resolved map coordinates.
    pub fn has_coordinates(&self) -> bool {
        self.latitude.is_some() && self.longitude.is_some()
    }

    /// Get full address as a single string.
    pub fn full_address(&self) -> String {
        format!(
            "{}, {} {}, {}",
            self.street, self.postal_code, self.city, self.country
        )
    }

    /// Get amenities as a list of strings.
    pub fn amenity_list(&self) -> Vec<String> {
        self.amenities
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Summary view of a building (for list views).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct BuildingSummary {
    pub id: Uuid,
    pub name: Option<String>,
    pub street: String,
    pub city: String,
    pub postal_code: String,
    pub total_floors: i32,
    pub status: String,
    /// Geocoded latitude (Story 3.1 AC3); null when unresolved.
    #[sqlx(default)]
    pub latitude: Option<Decimal>,
    /// Geocoded longitude (Story 3.1 AC3); null when unresolved.
    #[sqlx(default)]
    pub longitude: Option<Decimal>,
    #[sqlx(default)]
    pub unit_count: Option<i64>,
}

/// Data for creating a new building.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateBuilding {
    pub organization_id: Uuid,
    pub street: String,
    pub city: String,
    pub postal_code: String,
    #[serde(default = "default_country")]
    pub country: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub year_built: Option<i32>,
    #[serde(default = "default_one")]
    pub total_floors: i32,
    #[serde(default = "default_one")]
    pub total_entrances: i32,
    #[serde(default)]
    pub amenities: Vec<String>,
    /// Latitude (e.g. from geocoding). Optional; defaults to NULL.
    #[serde(default)]
    pub latitude: Option<Decimal>,
    /// Longitude (e.g. from geocoding). Optional; defaults to NULL.
    #[serde(default)]
    pub longitude: Option<Decimal>,
}

fn default_country() -> String {
    "Slovakia".to_string()
}

fn default_one() -> i32 {
    1
}

/// Data for updating a building.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateBuilding {
    pub street: Option<String>,
    pub city: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub year_built: Option<i32>,
    pub total_floors: Option<i32>,
    pub total_entrances: Option<i32>,
    pub amenities: Option<Vec<String>>,
    pub contacts: Option<serde_json::Value>,
    pub settings: Option<serde_json::Value>,
    /// New latitude (e.g. re-geocoded after an address change).
    pub latitude: Option<Decimal>,
    /// New longitude (e.g. re-geocoded after an address change).
    pub longitude: Option<Decimal>,
}

/// Building contact information.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BuildingContact {
    pub name: String,
    pub role: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

/// Building statistics for dashboard/reporting.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BuildingStatistics {
    pub building_id: Uuid,
    pub total_units: i64,
    pub occupied_units: i64,
    pub vacant_units: i64,
    pub total_owners: i64,
    pub ownership_coverage: f64, // Percentage of units with assigned owners
}

/// Building status enum.
pub mod building_status {
    pub const ACTIVE: &str = "active";
    pub const ARCHIVED: &str = "archived";
}
