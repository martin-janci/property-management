//! Facility model (Epic 3, Story 3.7).

use chrono::{DateTime, NaiveTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Facility type enum values.
pub mod facility_type {
    pub const GYM: &str = "gym";
    pub const LAUNDRY: &str = "laundry";
    pub const MEETING_ROOM: &str = "meeting_room";
    pub const PARTY_ROOM: &str = "party_room";
    pub const SAUNA: &str = "sauna";
    pub const POOL: &str = "pool";
    pub const PLAYGROUND: &str = "playground";
    pub const PARKING: &str = "parking";
    pub const STORAGE: &str = "storage";
    pub const GARDEN: &str = "garden";
    pub const BBQ: &str = "bbq";
    pub const BIKE_STORAGE: &str = "bike_storage";
    pub const OTHER: &str = "other";
}

/// Booking status enum values.
pub mod booking_status {
    pub const PENDING: &str = "pending";
    pub const APPROVED: &str = "approved";
    pub const REJECTED: &str = "rejected";
    pub const CANCELLED: &str = "cancelled";
    pub const COMPLETED: &str = "completed";
    pub const NO_SHOW: &str = "no_show";
}

/// Facility entity from database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct Facility {
    pub id: Uuid,
    pub building_id: Uuid,
    pub name: String,
    pub facility_type: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub capacity: Option<i32>,
    pub is_bookable: bool,
    pub requires_approval: bool,
    pub max_booking_hours: Option<i32>,
    pub max_advance_days: Option<i32>,
    pub min_advance_hours: Option<i32>,
    pub available_from: Option<NaiveTime>,
    pub available_to: Option<NaiveTime>,
    pub available_days: Option<Vec<i32>>,
    pub rules: Option<String>,
    // Nullable NUMERIC: decode straight into `Option<Decimal>` (a
    // `try_from` adapter rejects NULL with UnexpectedNullError -> 500).
    pub hourly_fee: Option<Decimal>,
    // Nullable NUMERIC: decode straight into `Option<Decimal>` (a
    // `try_from` adapter rejects NULL with UnexpectedNullError -> 500).
    pub deposit_amount: Option<Decimal>,
    pub is_active: bool,
    pub photos: serde_json::Value,
    pub amenities: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Facility {
    /// Get facility type display name.
    pub fn facility_type_display(&self) -> &str {
        match self.facility_type.as_str() {
            "gym" => "Gym",
            "laundry" => "Laundry Room",
            "meeting_room" => "Meeting Room",
            "party_room" => "Party Room",
            "sauna" => "Sauna",
            "pool" => "Swimming Pool",
            "playground" => "Playground",
            "parking" => "Parking",
            "storage" => "Storage",
            "garden" => "Garden",
            "bbq" => "BBQ Area",
            "bike_storage" => "Bike Storage",
            "other" => "Other",
            _ => &self.facility_type,
        }
    }

    /// Check if available on a given day (0=Sunday, 1=Monday, etc.).
    pub fn is_available_on_day(&self, day: i32) -> bool {
        match &self.available_days {
            Some(days) => days.contains(&day),
            None => true, // Available all days if not specified
        }
    }
}

/// Summary view of a facility.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct FacilitySummary {
    pub id: Uuid,
    pub building_id: Uuid,
    pub name: String,
    pub facility_type: String,
    pub is_bookable: bool,
    pub is_active: bool,
}

/// Data for creating a new facility.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateFacility {
    pub building_id: Uuid,
    pub name: String,
    pub facility_type: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub capacity: Option<i32>,
    #[serde(default)]
    pub is_bookable: bool,
    #[serde(default)]
    pub requires_approval: bool,
    pub max_booking_hours: Option<i32>,
    pub max_advance_days: Option<i32>,
    pub min_advance_hours: Option<i32>,
    pub available_from: Option<NaiveTime>,
    pub available_to: Option<NaiveTime>,
    pub available_days: Option<Vec<i32>>,
    pub rules: Option<String>,
    pub hourly_fee: Option<Decimal>,
    pub deposit_amount: Option<Decimal>,
}

/// Data for updating a facility.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateFacility {
    pub name: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub capacity: Option<i32>,
    pub is_bookable: Option<bool>,
    pub requires_approval: Option<bool>,
    pub max_booking_hours: Option<i32>,
    pub max_advance_days: Option<i32>,
    pub min_advance_hours: Option<i32>,
    pub available_from: Option<NaiveTime>,
    pub available_to: Option<NaiveTime>,
    pub available_days: Option<Vec<i32>>,
    pub rules: Option<String>,
    pub hourly_fee: Option<Decimal>,
    pub deposit_amount: Option<Decimal>,
    pub is_active: Option<bool>,
    pub photos: Option<serde_json::Value>,
    pub amenities: Option<serde_json::Value>,
}

/// Facility booking entity from database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct FacilityBooking {
    pub id: Uuid,
    pub facility_id: Uuid,
    pub user_id: Uuid,
    pub unit_id: Option<Uuid>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub status: String,
    pub purpose: Option<String>,
    pub attendees: Option<i32>,
    pub notes: Option<String>,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub rejected_by: Option<Uuid>,
    pub rejected_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub cancellation_reason: Option<String>,
    // Nullable NUMERIC: decode straight into `Option<Decimal>` (a
    // `try_from` adapter rejects NULL with UnexpectedNullError -> 500).
    pub total_fee: Option<Decimal>,
    pub deposit_paid: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl FacilityBooking {
    /// Get status display name.
    pub fn status_display(&self) -> &str {
        match self.status.as_str() {
            "pending" => "Pending",
            "approved" => "Approved",
            "rejected" => "Rejected",
            "cancelled" => "Cancelled",
            "completed" => "Completed",
            "no_show" => "No Show",
            _ => &self.status,
        }
    }

    /// Calculate booking duration in hours.
    pub fn duration_hours(&self) -> i64 {
        (self.end_time - self.start_time).num_hours()
    }
}

/// Booking with facility and user info.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BookingWithDetails {
    #[serde(flatten)]
    pub booking: FacilityBooking,
    pub facility_name: String,
    pub user_name: String,
    pub user_email: String,
}

/// Data for creating a new booking.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateFacilityBooking {
    pub facility_id: Uuid,
    pub unit_id: Option<Uuid>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub purpose: Option<String>,
    pub attendees: Option<i32>,
    pub notes: Option<String>,
}

/// Data for updating a booking.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateFacilityBooking {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub purpose: Option<String>,
    pub attendees: Option<i32>,
    pub notes: Option<String>,
}

/// Request to approve a booking.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApproveBooking {
    pub notes: Option<String>,
}

/// Request to reject a booking.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RejectBooking {
    pub reason: String,
}

/// Request to cancel a booking.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CancelBooking {
    pub reason: Option<String>,
}

/// Available time slot.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AvailableSlot {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}
