//! Domain model types for the Booking.com integration.
//!
//! Plain data structures for credentials, properties, reservations, guests and
//! internal<->Booking.com id mappings. These carry no wire-format logic; the OTA
//! XML request/response models live in [`super::messages`].

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Booking.com API credentials.
#[derive(Debug, Clone)]
pub struct BookingCredentials {
    /// Hotel ID assigned by Booking.com.
    pub hotel_id: String,
    /// API username.
    pub username: String,
    /// API password.
    pub password: String,
    /// API endpoint URL.
    pub api_url: String,
}

impl BookingCredentials {
    /// Create credentials with the standard production API URL.
    pub fn new(hotel_id: String, username: String, password: String) -> Self {
        Self {
            hotel_id,
            username,
            password,
            api_url: "https://supply-xml.booking.com/hotels/xml".to_string(),
        }
    }

    /// Create credentials with a custom API URL (for testing).
    pub fn with_url(hotel_id: String, username: String, password: String, api_url: String) -> Self {
        Self {
            hotel_id,
            username,
            password,
            api_url,
        }
    }
}

// ============================================
// Property Types
// ============================================

/// A Booking.com property (hotel).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookingProperty {
    /// Booking.com hotel ID.
    pub hotel_id: String,
    /// Property name.
    pub name: String,
    /// Property description.
    pub description: Option<String>,
    /// Star rating (0-5).
    pub star_rating: Option<i32>,
    /// Property type.
    pub property_type: String,
    /// Address.
    pub address: BookingAddress,
    /// Contact information.
    pub contact: BookingContact,
    /// Room types.
    pub room_types: Vec<BookingRoomType>,
    /// Facilities/amenities.
    pub facilities: Vec<String>,
    /// Check-in time.
    pub check_in_time: Option<String>,
    /// Check-out time.
    pub check_out_time: Option<String>,
    /// Last sync timestamp.
    pub synced_at: Option<DateTime<Utc>>,
}

/// Property address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookingAddress {
    /// Street address.
    pub street: String,
    /// City.
    pub city: String,
    /// State/Province.
    pub state: Option<String>,
    /// Postal code.
    pub postal_code: String,
    /// Country code (ISO 3166-1 alpha-2).
    pub country_code: String,
    /// Latitude.
    pub latitude: Option<f64>,
    /// Longitude.
    pub longitude: Option<f64>,
}

/// Property contact information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookingContact {
    /// Contact email.
    pub email: Option<String>,
    /// Contact phone.
    pub phone: Option<String>,
    /// Website URL.
    pub website: Option<String>,
}

/// A room type at a property.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookingRoomType {
    /// Room type ID.
    pub id: String,
    /// Room type name.
    pub name: String,
    /// Room description.
    pub description: Option<String>,
    /// Maximum occupancy.
    pub max_occupancy: i32,
    /// Number of beds.
    pub bed_count: i32,
    /// Bed type description.
    pub bed_type: Option<String>,
    /// Room size in square meters.
    pub room_size: Option<f32>,
    /// Room amenities.
    pub amenities: Vec<String>,
    /// Total rooms of this type.
    pub total_rooms: i32,
}

// ============================================
// Reservation Types
// ============================================

/// A Booking.com reservation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookingReservation {
    /// Reservation ID.
    pub reservation_id: String,
    /// Booking.com reservation number.
    pub booking_number: String,
    /// Hotel ID.
    pub hotel_id: String,
    /// Room type ID.
    pub room_type_id: String,
    /// Guest information.
    pub guest: BookingGuest,
    /// Check-in date.
    pub check_in: NaiveDate,
    /// Check-out date.
    pub check_out: NaiveDate,
    /// Number of nights.
    pub nights: i32,
    /// Reservation status.
    pub status: BookingReservationStatus,
    /// Number of rooms booked.
    pub rooms: i32,
    /// Number of adults.
    pub adults: i32,
    /// Number of children.
    pub children: i32,
    /// Total price.
    pub total_price: Decimal,
    /// Commission amount.
    pub commission: Decimal,
    /// Currency code.
    pub currency: String,
    /// Rate plan code.
    pub rate_plan: Option<String>,
    /// Meal plan code.
    pub meal_plan: Option<String>,
    /// Special requests.
    pub special_requests: Option<String>,
    /// Booking timestamp.
    pub booked_at: DateTime<Utc>,
    /// Last modification timestamp.
    pub modified_at: DateTime<Utc>,
    /// Cancellation deadline.
    pub cancel_deadline: Option<DateTime<Utc>>,
    /// Is non-refundable.
    pub is_non_refundable: bool,
}

/// Booking.com reservation status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BookingReservationStatus {
    /// New reservation, pending confirmation.
    New,
    /// Confirmed reservation.
    Confirmed,
    /// Modified reservation.
    Modified,
    /// Cancelled reservation.
    Cancelled,
    /// No-show.
    NoShow,
}

/// Map an OTA `ResStatus` value to the internal [`BookingReservationStatus`].
///
/// Recognised OTA values: `Commit`/`Confirmed`, `Cancel`/`Cancelled`,
/// `Modify`/`Modified`, `NoShow`/`No_Show` (case-insensitive). Anything
/// unrecognised maps to [`BookingReservationStatus::New`].
pub fn map_reservation_status(res_status: &str) -> BookingReservationStatus {
    match res_status.to_lowercase().as_str() {
        "commit" | "confirmed" => BookingReservationStatus::Confirmed,
        "cancel" | "cancelled" => BookingReservationStatus::Cancelled,
        "modify" | "modified" => BookingReservationStatus::Modified,
        "noshow" | "no_show" => BookingReservationStatus::NoShow,
        _ => BookingReservationStatus::New,
    }
}

/// Booking.com guest information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookingGuest {
    /// Guest first name.
    pub first_name: String,
    /// Guest last name.
    pub last_name: String,
    /// Guest email.
    pub email: Option<String>,
    /// Guest phone.
    pub phone: Option<String>,
    /// Guest country.
    pub country: Option<String>,
    /// Guest address.
    pub address: Option<String>,
    /// Guest city.
    pub city: Option<String>,
    /// Guest postal code.
    pub postal_code: Option<String>,
    /// Special notes about guest.
    pub notes: Option<String>,
}

// ============================================
// Property Mapping
// ============================================

/// Mapping between internal property and Booking.com property.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyMapping {
    /// Internal property ID.
    pub internal_property_id: Uuid,
    /// Booking.com hotel ID.
    pub external_property_id: String,
    /// External property name for reference.
    pub external_property_name: Option<String>,
    /// Room type mappings.
    pub room_mappings: Vec<RoomTypeMapping>,
    /// Whether sync is enabled.
    pub sync_enabled: bool,
    /// Last sync timestamp.
    pub last_sync_at: Option<DateTime<Utc>>,
}

/// Mapping between internal unit and Booking.com room type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomTypeMapping {
    /// Internal unit ID.
    pub internal_unit_id: Uuid,
    /// Booking.com room type ID.
    pub external_room_type_id: String,
    /// Room type name for reference.
    pub external_room_type_name: Option<String>,
}
