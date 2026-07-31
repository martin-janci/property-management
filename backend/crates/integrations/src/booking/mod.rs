//! Booking.com integration client (Story 83.2).
//!
//! Implements Booking.com Connectivity API integration,
//! including OTA XML message handling for reservations and availability.
//!
//! # XML handling
//!
//! All OTA XML serialisation and deserialisation is performed via [`quick_xml`]
//! (see the [`ota_xml`] submodule).  The helpers produce and consume
//! namespace-qualified XML that complies with the OpenTravel Alliance schema
//! (`xmlns="http://www.opentravel.org/OTA/2003/05"`).  The legacy
//! string-search helpers are retained only as fallback paths for unusual
//! response shapes that the streaming parser cannot reach.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

// ============================================
// OTA XML namespace + quick-xml helpers
// ============================================

/// Namespace URI used by all OTA messages exchanged with Booking.com.
pub const OTA_NAMESPACE: &str = "http://www.opentravel.org/OTA/2003/05";

/// Low-level OTA XML serialization/deserialization helpers for the
/// Booking.com OTA wire format. See [`ota_xml`] for details.
pub mod ota_xml;

/// OAuth 2.0 (authorization-code) client for Booking.com's newer Connectivity
/// APIs. See [`oauth`] for details.
pub mod oauth;

/// Retry policy + push-outcome types for OTA rate/availability push (AC-5).
/// See [`retry`] for details.
pub mod retry;

// Re-export the moved public items so existing `booking::<Item>` paths (and the
// crate-level `pub use booking::{...}` in `lib.rs`) keep resolving unchanged
// after the module split.
pub use oauth::{
    BookingOAuthClient, BookingOAuthConfig, BookingOAuthTokens, BOOKING_OAUTH_AUTH_URL,
    BOOKING_OAUTH_TOKEN_URL,
};
pub use retry::{BookingRetryConfig, PushOutcome};
// `is_retryable_status` is crate-private (pub(super)); bring it into module
// scope so the `BookingClient` impl and the in-module tests keep calling it
// unqualified.
use retry::is_retryable_status;

// ============================================
// Error Types
// ============================================

/// Errors that can occur during Booking.com API operations.
#[derive(Debug, Error)]
pub enum BookingError {
    /// OAuth/authentication error.
    #[error("Authentication error: {0}")]
    Auth(String),

    /// API error from Booking.com.
    #[error("API error: {0}")]
    Api(String),

    /// Network/HTTP error.
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// XML parsing error.
    #[error("XML error: {0}")]
    Xml(String),

    /// Rate limit exceeded.
    #[error("Rate limit exceeded, retry after {0} seconds")]
    RateLimited(u64),

    /// Push operation failed.
    #[error("Push failed: {0}")]
    PushFailed(String),

    /// Invalid OTA message.
    #[error("Invalid OTA message: {0}")]
    InvalidMessage(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),
}

// ============================================
// Authentication Types
// ============================================

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
// OTA Message Types
// ============================================

/// OTA Read Request for fetching reservations.
#[derive(Debug, Clone)]
pub struct OtaReadRQ {
    /// Hotel code.
    pub hotel_code: String,
    /// Start date for reservation search.
    pub start_date: NaiveDate,
    /// End date for reservation search.
    pub end_date: NaiveDate,
    /// Reservation status filter.
    pub status_filter: Option<String>,
}

impl OtaReadRQ {
    /// Convert to OTA XML format.
    pub fn to_xml(&self) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<OTA_ReadRQ xmlns="http://www.opentravel.org/OTA/2003/05" Version="1.0">
  <ReadRequests>
    <HotelReadRequest HotelCode="{}">
      <SelectionCriteria Start="{}" End="{}" DateType="Arrival"/>
    </HotelReadRequest>
  </ReadRequests>
</OTA_ReadRQ>"#,
            self.hotel_code, self.start_date, self.end_date
        )
    }
}

/// OTA Read Response containing reservations.
#[derive(Debug, Clone)]
pub struct OtaReadRS {
    /// Whether the request was successful.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
    /// Reservations returned.
    pub reservations: Vec<BookingReservation>,
}

impl OtaReadRS {
    /// Parse from OTA XML format.
    ///
    /// # Arguments
    /// * `xml` - The XML response body
    ///
    /// # Returns
    /// Parsed response or error.
    pub fn from_xml(xml: &str) -> Result<Self, BookingError> {
        tracing::info!("Parsing OTA_ReadRS XML");

        // Use quick-xml based status parser for error/success detection.
        let (status_success, status_error) = ota_xml::parse_response_status(xml);

        // Some Booking.com OTA_ReadRS variants omit <Success/> and instead
        // signal success implicitly with a <ReservationsList> element. Accept
        // that as success even though parse_response_status returns the
        // indeterminate (false, _) state for a response with no <Success>.
        let implicit_success = xml.contains("<ReservationsList");

        if !status_success && !implicit_success {
            return Ok(Self {
                success: false,
                error: status_error,
                reservations: Vec::new(),
            });
        }

        // Require an explicit success indicator (either <Success/> or the
        // implicit <ReservationsList> element) before attempting to parse.
        if !xml.contains("<Success") && !implicit_success {
            return Ok(Self {
                success: false,
                error: Some("Unknown response format".to_string()),
                reservations: Vec::new(),
            });
        }

        // Parse reservations from XML
        let reservations = Self::parse_reservations(xml)?;

        Ok(Self {
            success: true,
            error: None,
            reservations,
        })
    }

    /// Parse reservations from OTA XML.
    fn parse_reservations(xml: &str) -> Result<Vec<BookingReservation>, BookingError> {
        let mut reservations = Vec::new();

        // Find all HotelReservation elements
        let mut search_pos = 0;
        while let Some(start) = xml[search_pos..].find("<HotelReservation") {
            let abs_start = search_pos + start;
            if let Some(end) = xml[abs_start..].find("</HotelReservation>") {
                let res_xml = &xml[abs_start..abs_start + end + 19];
                if let Ok(reservation) = Self::parse_single_reservation(res_xml) {
                    reservations.push(reservation);
                }
                search_pos = abs_start + end + 19;
            } else {
                break;
            }
        }

        Ok(reservations)
    }

    /// Parse a single HotelReservation element.
    fn parse_single_reservation(xml: &str) -> Result<BookingReservation, BookingError> {
        // Extract reservation ID
        let res_id = Self::extract_attr(xml, "ResID_Value")
            .or_else(|| Self::extract_attr(xml, "ID"))
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        // Extract booking number
        let booking_number =
            Self::extract_attr(xml, "ResID_Value").unwrap_or_else(|| res_id.clone());

        // Extract hotel ID
        let hotel_id = Self::extract_attr(xml, "HotelCode").unwrap_or_default();

        // Extract room type ID
        let room_type_id = Self::extract_attr(xml, "InvTypeCode")
            .or_else(|| Self::extract_attr(xml, "RoomTypeCode"))
            .unwrap_or_default();

        // Extract dates
        let check_in = Self::extract_attr(xml, "Start")
            .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
            .unwrap_or_else(|| Utc::now().date_naive());

        let check_out = Self::extract_attr(xml, "End")
            .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
            .unwrap_or_else(|| check_in + Duration::days(1));

        let nights = (check_out - check_in).num_days() as i32;

        // Extract status
        let status_str =
            Self::extract_attr(xml, "ResStatus").unwrap_or_else(|| "Confirmed".to_string());
        let status = map_reservation_status(&status_str);

        // Extract guest info
        let guest = BookingGuest {
            first_name: Self::extract_element(xml, "GivenName")
                .unwrap_or_else(|| "Guest".to_string()),
            last_name: Self::extract_element(xml, "Surname").unwrap_or_default(),
            email: Self::extract_element(xml, "Email"),
            phone: Self::extract_element(xml, "PhoneNumber")
                .or_else(|| Self::extract_attr(xml, "PhoneNumber")),
            country: Self::extract_attr(xml, "CountryCode"),
            address: Self::extract_element(xml, "AddressLine"),
            city: Self::extract_element(xml, "CityName"),
            postal_code: Self::extract_element(xml, "PostalCode"),
            notes: Self::extract_element(xml, "SpecialRequests")
                .or_else(|| Self::extract_element(xml, "Text")),
        };

        // Extract counts
        let rooms: i32 = Self::extract_attr(xml, "NumberOfUnits")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        let adults: i32 = Self::extract_attr(xml, "AdultCount")
            .or_else(|| {
                Self::extract_attr(xml, "AgeQualifyingCode")
                    .filter(|s| s == "10")
                    .and(Self::extract_attr(xml, "Count"))
            })
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        let children: i32 = Self::extract_attr(xml, "ChildCount")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        // Extract pricing
        let total_price = Self::extract_attr(xml, "AmountAfterTax")
            .or_else(|| Self::extract_attr(xml, "Amount"))
            .and_then(|s| s.parse().ok())
            .unwrap_or(Decimal::ZERO);
        let commission = Self::extract_attr(xml, "CommissionAmount")
            .and_then(|s| s.parse().ok())
            .unwrap_or(Decimal::ZERO);
        let currency = Self::extract_attr(xml, "CurrencyCode").unwrap_or_else(|| "EUR".to_string());

        // Extract rate and meal plan
        let rate_plan = Self::extract_attr(xml, "RatePlanCode");
        let meal_plan = Self::extract_attr(xml, "MealPlanCode");

        // Extract special requests
        let special_requests = Self::extract_element(xml, "SpecialRequests")
            .or_else(|| Self::extract_element(xml, "Text"));

        // Extract timestamps
        let booked_at = Self::extract_attr(xml, "CreateDateTime")
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);
        let modified_at = Self::extract_attr(xml, "LastModifyDateTime")
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or(booked_at);

        // Extract cancellation deadline
        let cancel_deadline = Self::extract_attr(xml, "AbsoluteDeadline")
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        // Check if non-refundable
        let is_non_refundable =
            xml.contains("NonRefundable=\"true\"") || xml.contains("NonRefundable=\"1\"");

        Ok(BookingReservation {
            reservation_id: res_id,
            booking_number,
            hotel_id,
            room_type_id,
            guest,
            check_in,
            check_out,
            nights,
            status,
            rooms,
            adults,
            children,
            total_price,
            commission,
            currency,
            rate_plan,
            meal_plan,
            special_requests,
            booked_at,
            modified_at,
            cancel_deadline,
            is_non_refundable,
        })
    }

    /// Extract an attribute value from XML.
    fn extract_attr(xml: &str, attr_name: &str) -> Option<String> {
        let pattern = format!("{}=\"", attr_name);
        if let Some(start) = xml.find(&pattern) {
            let start = start + pattern.len();
            if let Some(end) = xml[start..].find('"') {
                return Some(xml[start..start + end].to_string());
            }
        }
        None
    }

    /// Extract element content from XML.
    fn extract_element(xml: &str, element_name: &str) -> Option<String> {
        let open_tag = format!("<{}", element_name);
        if let Some(tag_start) = xml.find(&open_tag) {
            // Find the end of the opening tag
            if let Some(content_start) = xml[tag_start..].find('>') {
                let content_start = tag_start + content_start + 1;
                let close_tag = format!("</{}>", element_name);
                if let Some(end) = xml[content_start..].find(&close_tag) {
                    let content = xml[content_start..content_start + end].trim();
                    if !content.is_empty() {
                        return Some(content.to_string());
                    }
                }
            }
        }
        None
    }
}

/// OTA Hotel Reservation Notification Request (push from Booking.com).
#[derive(Debug, Clone)]
pub struct OtaHotelResNotifRQ {
    /// Reservations in the notification.
    pub reservations: Vec<OtaReservationNotification>,
}

/// A single reservation notification.
#[derive(Debug, Clone)]
pub struct OtaReservationNotification {
    /// Reservation ID.
    pub res_id: String,
    /// Reservation status (Commit, Cancel, Modify).
    pub res_status: String,
    /// Full reservation data.
    pub reservation: Option<BookingReservation>,
}

impl OtaHotelResNotifRQ {
    /// Parse from OTA XML format using quick-xml helpers.
    ///
    /// Uses [`ota_xml::parse_res_notif_rq_raw`] for namespace-safe element
    /// boundary detection, then falls back to the existing field-extraction
    /// logic for reservation details.
    pub fn from_xml(xml: &str) -> Result<Self, BookingError> {
        tracing::info!("Parsing OTA_HotelResNotifRQ XML");

        let raw_notifs = ota_xml::parse_res_notif_rq_raw(xml)?;
        let mut notifications = Vec::new();

        for (res_id, res_status, fragment) in raw_notifs {
            // Parse full reservation details if this is not a cancellation.
            let reservation = if res_status.to_lowercase() != "cancel" {
                OtaReadRS::parse_single_reservation(&fragment).ok()
            } else {
                None
            };

            notifications.push(OtaReservationNotification {
                res_id,
                res_status,
                reservation,
            });
        }

        Ok(Self {
            reservations: notifications,
        })
    }
}

/// OTA Hotel Reservation Notification Response.
#[derive(Debug, Clone)]
pub struct OtaHotelResNotifRS {
    /// Whether processing was successful.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

impl OtaHotelResNotifRS {
    /// Create a success response.
    pub fn success() -> Self {
        Self {
            success: true,
            error: None,
        }
    }

    /// Create an error response.
    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            error: Some(message.to_string()),
        }
    }

    /// Convert to OTA XML format using quick-xml (namespace-aware).
    pub fn to_xml(&self) -> String {
        ota_xml::build_res_notif_rs(self.success, self.error.as_deref())
            .unwrap_or_else(|e| {
                tracing::error!(error = %e, "Failed to build OTA_HotelResNotifRS XML");
                // Minimal valid fallback.
                if self.success {
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<OTA_HotelResNotifRS xmlns=\"http://www.opentravel.org/OTA/2003/05\" Version=\"1.0\"><Success/></OTA_HotelResNotifRS>".to_string()
                } else {
                    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<OTA_HotelResNotifRS xmlns=\"http://www.opentravel.org/OTA/2003/05\" Version=\"1.0\"><Errors><Error Type=\"3\" Code=\"450\">{}</Error></Errors></OTA_HotelResNotifRS>",
                        self.error.as_deref().unwrap_or("Unknown error"))
                }
            })
    }
}

// ============================================
// Availability Types
// ============================================

/// Availability update for a room type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityUpdate {
    /// Room type ID.
    pub room_type_id: String,
    /// Date for the update.
    pub date: NaiveDate,
    /// Number of available rooms.
    pub available_count: i32,
    /// Stop sell flag.
    pub stop_sell: bool,
    /// Closed to arrival.
    pub cta: bool,
    /// Closed to departure.
    pub ctd: bool,
    /// Minimum length of stay.
    pub min_los: Option<i32>,
    /// Maximum length of stay.
    pub max_los: Option<i32>,
}

/// OTA Hotel Availability Notification Request.
#[derive(Debug, Clone)]
pub struct OtaHotelAvailNotifRQ {
    /// Hotel code.
    pub hotel_code: String,
    /// Availability status messages.
    pub avail_status_messages: Vec<AvailStatusMessage>,
}

/// Availability status message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailStatusMessage {
    /// Room type code.
    pub room_type_code: String,
    /// Rate plan code.
    pub rate_plan_code: Option<String>,
    /// Start date.
    pub start_date: NaiveDate,
    /// End date.
    pub end_date: NaiveDate,
    /// Booking limit (available rooms).
    pub booking_limit: i32,
    /// Status (Open/Close).
    pub status: String,
    /// Length of stay restrictions.
    pub los_restrictions: Option<LosRestrictions>,
}

/// Length of stay restrictions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LosRestrictions {
    /// Minimum stay.
    pub min_los: Option<i32>,
    /// Maximum stay.
    pub max_los: Option<i32>,
}

impl OtaHotelAvailNotifRQ {
    /// Convert to OTA XML format using quick-xml (namespace-aware).
    pub fn to_xml(&self) -> String {
        ota_xml::build_avail_notif_rq(&self.hotel_code, &self.avail_status_messages).unwrap_or_else(
            |e| {
                tracing::error!(error = %e, "Failed to build OTA_HotelAvailNotifRQ XML");
                // Fallback to legacy format on unexpected serialisation error.
                self.to_xml_legacy()
            },
        )
    }

    /// Legacy string-template fallback (kept for emergency use only).
    fn to_xml_legacy(&self) -> String {
        let mut messages = String::new();
        for msg in &self.avail_status_messages {
            messages.push_str(&format!(
                "    <AvailStatusMessage BookingLimit=\"{}\" BookingLimitMessageType=\"SetLimit\">\n      <StatusApplicationControl Start=\"{}\" End=\"{}\" InvTypeCode=\"{}\" RatePlanCode=\"{}\"/>\n      <RestrictionStatus Status=\"{}\" Restriction=\"Master\"/>\n    </AvailStatusMessage>\n",
                msg.booking_limit,
                msg.start_date,
                msg.end_date,
                msg.room_type_code,
                msg.rate_plan_code.as_deref().unwrap_or("STD"),
                msg.status
            ));
        }
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<OTA_HotelAvailNotifRQ xmlns=\"http://www.opentravel.org/OTA/2003/05\" Version=\"1.0\">\n  <AvailStatusMessages HotelCode=\"{}\">\n{}  </AvailStatusMessages>\n</OTA_HotelAvailNotifRQ>",
            self.hotel_code, messages
        )
    }
}

/// OTA Hotel Availability Notification Response.
#[derive(Debug, Clone)]
pub struct OtaHotelAvailNotifRS {
    /// Whether the update was successful.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

impl OtaHotelAvailNotifRS {
    /// Parse from OTA XML format using quick-xml (namespace-aware).
    pub fn from_xml(xml: &str) -> Result<Self, BookingError> {
        tracing::info!("Parsing OTA_HotelAvailNotifRS XML");
        let (success, error) = ota_xml::parse_response_status(xml);
        Ok(Self { success, error })
    }
}

// ============================================
// Rate Types
// ============================================

/// Rate update for a room type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateUpdate {
    /// Room type ID.
    pub room_type_id: String,
    /// Rate plan code.
    pub rate_plan_code: String,
    /// Date for the rate.
    pub date: NaiveDate,
    /// Base rate amount.
    pub base_rate: Decimal,
    /// Currency code.
    pub currency: String,
    /// Extra person rate.
    pub extra_person_rate: Option<Decimal>,
    /// Extra child rate.
    pub extra_child_rate: Option<Decimal>,
}

/// OTA Hotel Rate Amount Notification Request (`OTA_HotelRateAmountNotifRQ`).
///
/// Typed request model for outbound rate messages, mirroring
/// [`OtaHotelAvailNotifRQ`] on the availability side. Serialisation delegates
/// to [`ota_xml::build_rate_amount_notif_rq`] so the wire format stays in one
/// place; [`from_xml`](OtaHotelRateAmountNotifRQ::from_xml) is the inbound
/// inverse built on [`ota_xml::parse_rate_amount_notif_rq`].
#[derive(Debug, Clone)]
pub struct OtaHotelRateAmountNotifRQ {
    /// Hotel code (`<RateAmountMessages HotelCode="...">`).
    pub hotel_code: String,
    /// Rate updates, one per `<RateAmountMessage>`.
    pub rate_amount_messages: Vec<RateUpdate>,
}

impl OtaHotelRateAmountNotifRQ {
    /// Serialise to an `OTA_HotelRateAmountNotifRQ` document.
    ///
    /// Each [`RateUpdate`] applies to a single day (`date` used for both the
    /// `Start` and `End` of the `<StatusApplicationControl>`), matching the
    /// push flow's grouping in `BookingClient::push_rates`.
    pub fn to_xml(&self) -> Result<String, BookingError> {
        let update_tuples: Vec<(NaiveDate, NaiveDate, &str, &str, &Decimal, &str)> = self
            .rate_amount_messages
            .iter()
            .map(|u| {
                (
                    u.date,
                    u.date,
                    u.room_type_id.as_str(),
                    u.rate_plan_code.as_str(),
                    &u.base_rate,
                    u.currency.as_str(),
                )
            })
            .collect();
        ota_xml::build_rate_amount_notif_rq(&self.hotel_code, &update_tuples)
    }

    /// Parse an inbound `OTA_HotelRateAmountNotifRQ` document into the typed
    /// model. Wraps [`ota_xml::parse_rate_amount_notif_rq`], mapping each
    /// parsed `<RateAmountMessage>` to a [`RateUpdate`]. Per-message rates are
    /// single-day, so `date` is taken from the parsed `start_date`. Optional
    /// extra-person/child rates are not carried on the OTA wire and default to
    /// `None`.
    pub fn from_xml(xml: &str) -> Result<Self, BookingError> {
        tracing::info!("Parsing OTA_HotelRateAmountNotifRQ XML");
        let parsed = ota_xml::parse_rate_amount_notif_rq(xml)?;
        let rate_amount_messages = parsed
            .rates
            .into_iter()
            .map(|r| RateUpdate {
                room_type_id: r.room_type_code,
                rate_plan_code: r.rate_plan_code.unwrap_or_default(),
                date: r.start_date,
                base_rate: r.amount,
                currency: r.currency,
                extra_person_rate: None,
                extra_child_rate: None,
            })
            .collect();
        Ok(Self {
            hotel_code: parsed.hotel_code,
            rate_amount_messages,
        })
    }
}

/// OTA Hotel Rate Amount Notification Response (`OTA_HotelRateAmountNotifRS`).
///
/// Typed response model mirroring [`OtaHotelAvailNotifRS`]; parsing delegates
/// to [`ota_xml::parse_response_status`].
#[derive(Debug, Clone)]
pub struct OtaHotelRateAmountNotifRS {
    /// Whether the rate update was accepted.
    pub success: bool,
    /// Error message when `success` is `false`.
    pub error: Option<String>,
}

impl OtaHotelRateAmountNotifRS {
    /// Parse from an `OTA_HotelRateAmountNotifRS` document.
    pub fn from_xml(xml: &str) -> Result<Self, BookingError> {
        tracing::info!("Parsing OTA_HotelRateAmountNotifRS XML");
        let (success, error) = ota_xml::parse_response_status(xml);
        Ok(Self { success, error })
    }
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

// ============================================
// Client Implementation
// ============================================

/// Booking.com API client.
pub struct BookingClient {
    client: reqwest::Client,
    credentials: BookingCredentials,
    retry: BookingRetryConfig,
}

impl BookingClient {
    fn build_http_client() -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default()
    }

    /// Create a new Booking.com client.
    pub fn new(credentials: BookingCredentials) -> Self {
        Self {
            client: Self::build_http_client(),
            credentials,
            retry: BookingRetryConfig::default(),
        }
    }

    /// Create a new client with simple API key (for basic usage).
    pub fn with_api_key(api_key: String) -> Self {
        Self {
            client: Self::build_http_client(),
            credentials: BookingCredentials {
                hotel_id: String::new(),
                username: api_key.clone(),
                password: api_key,
                api_url: "https://supply-xml.booking.com/hotels/xml".to_string(),
            },
            retry: BookingRetryConfig::default(),
        }
    }

    /// Override the retry policy used by the OTA push operations.
    pub fn with_retry(mut self, retry: BookingRetryConfig) -> Self {
        self.retry = retry;
        self
    }

    /// Generate the authorization header.
    fn generate_auth_header(&self) -> String {
        use base64::{engine::general_purpose::STANDARD, Engine};
        let credentials = format!(
            "{}:{}",
            self.credentials.username, self.credentials.password
        );
        format!("Basic {}", STANDARD.encode(credentials.as_bytes()))
    }

    /// POST an OTA XML document to the Booking.com endpoint with bounded
    /// exponential-backoff retry on transient failures (AC-5).
    ///
    /// Retries are attempted on:
    /// * network/transport errors (timeouts, connection resets), and
    /// * retryable HTTP status codes (see [`is_retryable_status`]).
    ///
    /// Non-retryable HTTP errors (e.g. 400/401/403/404) fail immediately with
    /// [`BookingError::Api`]. When all attempts are exhausted the last error is
    /// returned. On HTTP success the raw response body is returned for the
    /// caller to parse the OTA-level `<Success>` / `<Errors>` status.
    ///
    /// `op` is a short label used only for tracing.
    async fn post_ota_with_retry(&self, op: &str, body: String) -> Result<String, BookingError> {
        let attempts = self.retry.max_attempts.max(1);
        let mut last_err: Option<BookingError> = None;
        // Retry-After hint (ms) from the last 429/503 response.
        let mut retry_after_ms: Option<u64> = None;

        for attempt in 0..attempts {
            if attempt > 0 {
                let delay = self.retry.effective_delay_ms(attempt - 1, retry_after_ms);
                retry_after_ms = None;
                tracing::warn!(
                    op,
                    attempt = attempt + 1,
                    max = attempts,
                    delay_ms = delay,
                    "Retrying Booking.com OTA push after transient failure"
                );
                if delay > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                }
            }

            let send_result = self
                .client
                .post(&self.credentials.api_url)
                .header("Authorization", self.generate_auth_header())
                .header("Content-Type", "application/xml")
                .body(body.clone())
                .send()
                .await;

            let response = match send_result {
                Ok(resp) => resp,
                Err(e) => {
                    // Transport-level failure — always treated as transient.
                    tracing::warn!(op, error = %e, "Booking.com OTA push transport error");
                    last_err = Some(BookingError::Network(e));
                    continue;
                }
            };

            let status = response.status();
            if status.is_success() {
                return response.text().await.map_err(BookingError::Network);
            }

            let code = status.as_u16();

            // Parse Retry-After before consuming the body (response is moved by .text()).
            let ra_hint_secs = if code == 429 || code == 503 {
                response
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.trim().parse::<u64>().ok())
            } else {
                None
            };

            let error_body = response.text().await.unwrap_or_default();

            if is_retryable_status(code) {
                tracing::warn!(
                    op,
                    status = code,
                    retry_after_secs = ra_hint_secs,
                    "Booking.com OTA push returned retryable HTTP status"
                );
                if let Some(secs) = ra_hint_secs {
                    retry_after_ms = Some(secs.saturating_mul(1000).min(self.retry.max_delay_ms));
                }
                // 429 always maps to RateLimited (with or without Retry-After);
                // other retryable codes map to Api unless they carried Retry-After.
                last_err = if code == 429 {
                    Some(BookingError::RateLimited(ra_hint_secs.unwrap_or(0)))
                } else {
                    Some(BookingError::Api(format!("HTTP {code}: {error_body}")))
                };
                continue;
            }

            // Non-retryable HTTP error: fail fast.
            return Err(BookingError::Api(format!("HTTP {code}: {error_body}")));
        }

        Err(last_err.unwrap_or_else(|| {
            BookingError::PushFailed("push failed with no recorded error".to_string())
        }))
    }

    // ==================== Property Operations ====================

    /// Fetch all properties for the account.
    pub async fn fetch_properties(&self) -> Result<Vec<BookingProperty>, BookingError> {
        tracing::info!("Fetching Booking.com properties");

        if !self.credentials.hotel_id.is_empty() {
            match self.fetch_property(&self.credentials.hotel_id).await {
                Ok(property) => Ok(vec![property]),
                Err(_) => Ok(vec![BookingProperty {
                    hotel_id: self.credentials.hotel_id.clone(),
                    name: format!("Property {}", self.credentials.hotel_id),
                    description: None,
                    star_rating: None,
                    property_type: "hotel".to_string(),
                    address: BookingAddress {
                        street: String::new(),
                        city: String::new(),
                        state: None,
                        postal_code: String::new(),
                        country_code: String::new(),
                        latitude: None,
                        longitude: None,
                    },
                    contact: BookingContact {
                        email: None,
                        phone: None,
                        website: None,
                    },
                    room_types: Vec::new(),
                    facilities: Vec::new(),
                    check_in_time: None,
                    check_out_time: None,
                    synced_at: Some(Utc::now()),
                }]),
            }
        } else {
            Ok(Vec::new())
        }
    }

    /// Fetch a single property by ID.
    pub async fn fetch_property(&self, hotel_id: &str) -> Result<BookingProperty, BookingError> {
        tracing::info!("Fetching Booking.com property: {}", hotel_id);

        let request_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<OTA_HotelDescriptiveInfoRQ xmlns="http://www.opentravel.org/OTA/2003/05" Version="1.0">
  <HotelDescriptiveInfos>
    <HotelDescriptiveInfo HotelCode="{}" />
  </HotelDescriptiveInfos>
</OTA_HotelDescriptiveInfoRQ>"#,
            hotel_id
        );

        let response = self
            .client
            .post(&self.credentials.api_url)
            .header("Authorization", self.generate_auth_header())
            .header("Content-Type", "application/xml")
            .body(request_xml)
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(BookingError::Api(error));
        }

        let body = response.text().await?;
        self.parse_property_response(&body, hotel_id)
    }

    /// Parse property details from OTA_HotelDescriptiveInfoRS.
    fn parse_property_response(
        &self,
        xml: &str,
        hotel_id: &str,
    ) -> Result<BookingProperty, BookingError> {
        if xml.contains("<Errors>") || xml.contains("<Error") {
            return Err(BookingError::Api(
                "Failed to fetch property details".to_string(),
            ));
        }

        let name = Self::extract_xml_attr(xml, "HotelName")
            .or_else(|| Self::extract_xml_element(xml, "HotelName"))
            .unwrap_or_else(|| format!("Property {}", hotel_id));

        let description = Self::extract_xml_element(xml, "DescriptiveText");

        let star_rating = Self::extract_xml_attr(xml, "Rating").and_then(|s| s.parse::<i32>().ok());

        let address = BookingAddress {
            street: Self::extract_xml_element(xml, "AddressLine").unwrap_or_default(),
            city: Self::extract_xml_element(xml, "CityName").unwrap_or_default(),
            state: Self::extract_xml_element(xml, "StateProv"),
            postal_code: Self::extract_xml_element(xml, "PostalCode").unwrap_or_default(),
            country_code: Self::extract_xml_attr(xml, "CountryCode").unwrap_or_default(),
            latitude: Self::extract_xml_attr(xml, "Latitude").and_then(|s| s.parse().ok()),
            longitude: Self::extract_xml_attr(xml, "Longitude").and_then(|s| s.parse().ok()),
        };

        let contact = BookingContact {
            email: Self::extract_xml_element(xml, "Email"),
            phone: Self::extract_xml_attr(xml, "PhoneNumber"),
            website: Self::extract_xml_element(xml, "URL"),
        };

        Ok(BookingProperty {
            hotel_id: hotel_id.to_string(),
            name,
            description,
            star_rating,
            property_type: "hotel".to_string(),
            address,
            contact,
            room_types: Vec::new(),
            facilities: Vec::new(),
            check_in_time: Self::extract_xml_attr(xml, "CheckInTime"),
            check_out_time: Self::extract_xml_attr(xml, "CheckOutTime"),
            synced_at: Some(Utc::now()),
        })
    }

    // ==================== Reservation Operations ====================

    /// Fetch reservations from Booking.com.
    pub async fn fetch_reservations(
        &self,
        hotel_id: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<BookingReservation>, BookingError> {
        tracing::info!("Fetching Booking.com reservations for hotel {}", hotel_id);

        let request = OtaReadRQ {
            hotel_code: hotel_id.to_string(),
            start_date,
            end_date,
            status_filter: None,
        };

        let response = self
            .client
            .post(&self.credentials.api_url)
            .header("Authorization", self.generate_auth_header())
            .header("Content-Type", "application/xml")
            .body(request.to_xml())
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(BookingError::Api(error));
        }

        let body = response.text().await?;
        let ota_response = OtaReadRS::from_xml(&body)?;

        if !ota_response.success {
            return Err(BookingError::Api(
                ota_response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        Ok(ota_response.reservations)
    }

    // ==================== Availability Operations ====================

    /// Push availability updates to Booking.com.
    pub async fn push_availability(
        &self,
        hotel_id: &str,
        updates: &[AvailabilityUpdate],
    ) -> Result<(), BookingError> {
        tracing::info!("Pushing availability to Booking.com for hotel {}", hotel_id);

        let messages: Vec<AvailStatusMessage> = updates
            .iter()
            .map(|u| AvailStatusMessage {
                room_type_code: u.room_type_id.clone(),
                rate_plan_code: None,
                start_date: u.date,
                end_date: u.date,
                booking_limit: u.available_count,
                status: if u.stop_sell {
                    "Close".to_string()
                } else {
                    "Open".to_string()
                },
                los_restrictions: if u.min_los.is_some() || u.max_los.is_some() {
                    Some(LosRestrictions {
                        min_los: u.min_los,
                        max_los: u.max_los,
                    })
                } else {
                    None
                },
            })
            .collect();

        let request = OtaHotelAvailNotifRQ {
            hotel_code: hotel_id.to_string(),
            avail_status_messages: messages,
        };

        let body = self
            .post_ota_with_retry("push_availability", request.to_xml())
            .await?;
        let ota_response = OtaHotelAvailNotifRS::from_xml(&body)?;

        if !ota_response.success {
            return Err(BookingError::PushFailed(
                ota_response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        Ok(())
    }

    // ==================== Rate Operations ====================

    /// Push rate updates to Booking.com.
    pub async fn push_rates(
        &self,
        hotel_id: &str,
        updates: &[RateUpdate],
    ) -> Result<(), BookingError> {
        tracing::info!("Pushing rates to Booking.com for hotel {}", hotel_id);

        // Group updates for the quick-xml builder: (start, end, room_type, rate_plan, rate, currency)
        let update_tuples: Vec<(NaiveDate, NaiveDate, &str, &str, &Decimal, &str)> = updates
            .iter()
            .map(|u| {
                (
                    u.date,
                    u.date,
                    u.room_type_id.as_str(),
                    u.rate_plan_code.as_str(),
                    &u.base_rate,
                    u.currency.as_str(),
                )
            })
            .collect();

        let xml = ota_xml::build_rate_amount_notif_rq(hotel_id, &update_tuples)?;

        let body = self.post_ota_with_retry("push_rates", xml).await?;
        let (success, error) = ota_xml::parse_response_status(&body);

        if !success {
            return Err(BookingError::PushFailed(
                error.unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        Ok(())
    }

    /// Push availability *and* rates to Booking.com in one outbound sync,
    /// reporting a per-stream [`PushOutcome`] (AC-5).
    ///
    /// This is the full rate/availability outbound sync entry point: it sends
    /// the availability stream (`OTA_HotelAvailNotifRQ`) and the rate stream
    /// (`OTA_HotelRateAmountNotifRQ`) for `hotel_id`, each through the bounded
    /// retry / error-handling path of [`Self::post_ota_with_retry`].
    ///
    /// Unlike the single-stream [`Self::push_availability`] /
    /// [`Self::push_rates`], a failure in one stream does **not** abort the
    /// other: both are always attempted (when their input is non-empty) so a
    /// rate rejection cannot silently discard a pending availability update.
    /// The returned [`PushOutcome`] records which stream applied, how many
    /// messages each sent, and the per-stream error text — letting the caller
    /// distinguish a clean sync ([`PushOutcome::is_success`]) from a
    /// half-applied one ([`PushOutcome::is_partial`]) that needs reconciling.
    ///
    /// Empty slices are treated as "nothing to push" for that stream (counted
    /// as 0, no error). Pushing two empty slices yields a successful no-op
    /// outcome.
    pub async fn push_availability_and_rates(
        &self,
        hotel_id: &str,
        availability: &[AvailabilityUpdate],
        rates: &[RateUpdate],
    ) -> PushOutcome {
        tracing::info!(
            hotel_id,
            availability = availability.len(),
            rates = rates.len(),
            "Booking.com combined availability + rate push"
        );

        let mut outcome = PushOutcome::default();

        if !availability.is_empty() {
            match self.push_availability(hotel_id, availability).await {
                Ok(()) => outcome.availability_pushed = availability.len(),
                Err(e) => {
                    tracing::error!(error = %e, "availability push failed in combined sync");
                    outcome.availability_error = Some(e.to_string());
                }
            }
        }

        if !rates.is_empty() {
            match self.push_rates(hotel_id, rates).await {
                Ok(()) => outcome.rates_pushed = rates.len(),
                Err(e) => {
                    tracing::error!(error = %e, "rate push failed in combined sync");
                    outcome.rates_error = Some(e.to_string());
                }
            }
        }

        outcome
    }

    // ==================== Webhook Handling ====================

    /// Process an incoming reservation notification from Booking.com.
    pub async fn process_reservation_notification(
        &self,
        xml_body: &str,
    ) -> Result<OtaHotelResNotifRS, BookingError> {
        tracing::info!("Processing reservation notification from Booking.com");

        let notification = OtaHotelResNotifRQ::from_xml(xml_body)?;

        // Process each reservation
        for notif in &notification.reservations {
            tracing::info!(
                "Processing reservation {} with status {}",
                notif.res_id,
                notif.res_status
            );
        }

        Ok(OtaHotelResNotifRS::success())
    }

    // ==================== Helper Methods ====================

    fn extract_xml_attr(xml: &str, attr_name: &str) -> Option<String> {
        let pattern = format!("{}=\"", attr_name);
        xml.find(&pattern).and_then(|start| {
            let val_start = start + pattern.len();
            xml[val_start..]
                .find('"')
                .map(|end| xml[val_start..val_start + end].to_string())
        })
    }

    fn extract_xml_element(xml: &str, element_name: &str) -> Option<String> {
        let open_tag = format!("<{}", element_name);
        xml.find(&open_tag)
            .and_then(|tag_start| {
                xml[tag_start..].find('>').and_then(|content_rel| {
                    let content_start = tag_start + content_rel + 1;
                    let close_tag = format!("</{}>", element_name);
                    xml[content_start..]
                        .find(&close_tag)
                        .map(|end| xml[content_start..content_start + end].trim().to_string())
                })
            })
            .filter(|s| !s.is_empty())
    }
}

// ============================================
// Tests
// ============================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    };

    /// A single scripted HTTP response for [`MockOtaServer`].
    struct MockResponse {
        status: u16,
        body: String,
        extra_headers: Vec<(String, String)>,
        /// When `true` the server reads the request and then drops the
        /// connection without writing any response, simulating a
        /// transport-level failure (connection reset / incomplete message)
        /// that the retry loop must treat as transient.
        drop_conn: bool,
    }

    impl MockResponse {
        fn status(status: u16, body: &str) -> Self {
            Self {
                status,
                body: body.to_string(),
                extra_headers: vec![],
                drop_conn: false,
            }
        }

        /// A response that abandons the connection mid-flight (no bytes
        /// written), forcing a `reqwest` transport error on the client.
        fn drop_connection() -> Self {
            Self {
                status: 0,
                body: String::new(),
                extra_headers: vec![],
                drop_conn: true,
            }
        }

        fn with_header(mut self, name: &str, value: &str) -> Self {
            self.extra_headers
                .push((name.to_string(), value.to_string()));
            self
        }
    }

    /// Minimal one-connection-per-request HTTP/1.1 mock server backed by a
    /// raw `tokio` TCP listener (no extra test deps). Each incoming request is
    /// answered with the next scripted [`MockResponse`]; once the script is
    /// exhausted it replies 500. Used to drive the retry/error-handling paths
    /// of the OTA push without touching the network.
    struct MockOtaServer {
        addr: std::net::SocketAddr,
        hits: Arc<AtomicUsize>,
        bodies: Arc<Mutex<Vec<String>>>,
    }

    impl MockOtaServer {
        async fn spawn(responses: Vec<MockResponse>) -> Self {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let hits = Arc::new(AtomicUsize::new(0));
            let hits_task = hits.clone();
            let bodies: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
            let bodies_task = bodies.clone();
            let script = Arc::new(Mutex::new(responses.into_iter()));

            tokio::spawn(async move {
                loop {
                    let (mut socket, _) = match listener.accept().await {
                        Ok(pair) => pair,
                        Err(_) => break,
                    };
                    let hits_inner = hits_task.clone();
                    let bodies_inner = bodies_task.clone();
                    let script_inner = script.clone();
                    tokio::spawn(async move {
                        use tokio::io::{AsyncReadExt, AsyncWriteExt};
                        // Read the full request and capture the body so tests can
                        // assert on idempotency tokens / OTA payload shape.
                        let mut buf = [0u8; 8192];
                        let n = socket.read(&mut buf).await.unwrap_or(0);
                        let raw = String::from_utf8_lossy(&buf[..n]).to_string();
                        let body = raw
                            .split_once("\r\n\r\n")
                            .map(|(_, b)| b.to_string())
                            .unwrap_or_default();
                        bodies_inner.lock().unwrap().push(body);

                        hits_inner.fetch_add(1, Ordering::SeqCst);
                        let resp = {
                            let mut it = script_inner.lock().unwrap();
                            it.next()
                        };
                        // Simulate a transport failure: drop the socket without
                        // writing a response so the client observes a reset /
                        // incomplete message.
                        if resp.as_ref().is_some_and(|r| r.drop_conn) {
                            return;
                        }
                        let (status, body, extra_headers) = match resp {
                            Some(r) => (r.status, r.body, r.extra_headers),
                            None => (500, "<exhausted/>".to_string(), vec![]),
                        };
                        let reason = match status {
                            200 => "OK",
                            400 => "Bad Request",
                            429 => "Too Many Requests",
                            503 => "Service Unavailable",
                            _ => "Status",
                        };
                        let extra = extra_headers
                            .iter()
                            .map(|(k, v)| format!("{k}: {v}\r\n"))
                            .collect::<String>();
                        let response = format!(
                            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/xml\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n{body}",
                            body.len()
                        );
                        let _ = socket.write_all(response.as_bytes()).await;
                        let _ = socket.flush().await;
                    });
                }
            });

            Self { addr, hits, bodies }
        }

        fn url(&self) -> String {
            format!("http://{}/hotels/xml", self.addr)
        }

        fn hits(&self) -> usize {
            self.hits.load(Ordering::SeqCst)
        }

        /// Snapshot of the request bodies captured so far, in arrival order.
        fn bodies(&self) -> Vec<String> {
            self.bodies.lock().unwrap().clone()
        }
    }

    /// Extract the value of the `EchoToken` attribute from an OTA RQ root, if
    /// present. Test-only helper.
    fn echo_token_of(xml: &str) -> Option<String> {
        let marker = "EchoToken=\"";
        let start = xml.find(marker)? + marker.len();
        let rest = &xml[start..];
        let end = rest.find('"')?;
        Some(rest[..end].to_string())
    }

    #[test]
    fn test_credentials_new() {
        let creds = BookingCredentials::new(
            "hotel123".to_string(),
            "user".to_string(),
            "pass".to_string(),
        );
        assert_eq!(creds.hotel_id, "hotel123");
        assert_eq!(creds.username, "user");
        assert_eq!(creds.password, "pass");
        assert!(creds.api_url.contains("booking.com"));
    }

    #[test]
    fn test_ota_read_rq_xml() {
        let rq = OtaReadRQ {
            hotel_code: "H123".to_string(),
            start_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2025, 1, 31).unwrap(),
            status_filter: None,
        };
        let xml = rq.to_xml();
        assert!(xml.contains("OTA_ReadRQ"));
        assert!(xml.contains("H123"));
        assert!(xml.contains("2025-01-01"));
    }

    #[test]
    fn test_ota_hotel_avail_notif_rq_xml() {
        let rq = OtaHotelAvailNotifRQ {
            hotel_code: "H456".to_string(),
            avail_status_messages: vec![AvailStatusMessage {
                room_type_code: "DBL".to_string(),
                rate_plan_code: Some("STD".to_string()),
                start_date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2025, 6, 30).unwrap(),
                booking_limit: 5,
                status: "Open".to_string(),
                los_restrictions: None,
            }],
        };
        let xml = rq.to_xml();
        assert!(xml.contains("OTA_HotelAvailNotifRQ"));
        assert!(xml.contains("H456"));
        assert!(xml.contains("DBL"));
    }

    #[test]
    fn test_ota_hotel_res_notif_rs_success() {
        let rs = OtaHotelResNotifRS::success();
        let xml = rs.to_xml();
        assert!(xml.contains("OTA_HotelResNotifRS"));
        assert!(xml.contains("Success"));
    }

    #[test]
    fn test_ota_hotel_res_notif_rs_error() {
        let rs = OtaHotelResNotifRS::error("Test error");
        let xml = rs.to_xml();
        assert!(xml.contains("Errors"));
        assert!(xml.contains("Test error"));
    }

    #[test]
    fn test_reservation_status_serialization() {
        let status = BookingReservationStatus::Confirmed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"confirmed\"");

        let status = BookingReservationStatus::Cancelled;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"cancelled\"");
    }

    #[test]
    fn test_retryable_status_classification() {
        // Transient server/throttle codes are retryable.
        for code in [408, 425, 429, 500, 502, 503, 504] {
            assert!(is_retryable_status(code), "{code} should be retryable");
        }
        // Client errors (bad message / auth) are NOT retryable.
        for code in [400, 401, 403, 404, 409, 422] {
            assert!(!is_retryable_status(code), "{code} should not be retryable");
        }
    }

    #[test]
    fn test_retry_backoff_is_exponential_and_capped() {
        let cfg = BookingRetryConfig {
            max_attempts: 5,
            initial_delay_ms: 100,
            max_delay_ms: 1_000,
            backoff_multiplier: 2,
        };
        assert_eq!(cfg.delay_for_attempt(0), 100); // 100 * 2^0
        assert_eq!(cfg.delay_for_attempt(1), 200); // 100 * 2^1
        assert_eq!(cfg.delay_for_attempt(2), 400); // 100 * 2^2
        assert_eq!(cfg.delay_for_attempt(3), 800); // 100 * 2^3
        assert_eq!(cfg.delay_for_attempt(4), 1_000); // 1600 capped to 1000
                                                     // Large attempt counts must not overflow.
        assert_eq!(cfg.delay_for_attempt(64), 1_000);
    }

    #[test]
    fn test_retry_config_defaults_and_no_retry() {
        let def = BookingRetryConfig::default();
        assert!(def.max_attempts >= 1);
        let none = BookingRetryConfig::no_retry();
        assert_eq!(none.max_attempts, 1);
        assert_eq!(none.delay_for_attempt(0), 0);
    }

    #[test]
    fn test_effective_delay_uses_backoff_when_no_retry_after() {
        // Without a Retry-After hint, effective_delay_ms is exactly the
        // exponential backoff for the given attempt.
        let cfg = BookingRetryConfig {
            max_attempts: 5,
            initial_delay_ms: 100,
            max_delay_ms: 10_000,
            backoff_multiplier: 2,
        };
        assert_eq!(cfg.effective_delay_ms(0, None), 100);
        assert_eq!(cfg.effective_delay_ms(1, None), 200);
        assert_eq!(cfg.effective_delay_ms(2, None), 400);
        // Matches delay_for_attempt for every attempt when no hint is present.
        for attempt in 0..6 {
            assert_eq!(
                cfg.effective_delay_ms(attempt, None),
                cfg.delay_for_attempt(attempt)
            );
        }
    }

    #[test]
    fn test_effective_delay_takes_max_of_hint_and_backoff() {
        let cfg = BookingRetryConfig {
            max_attempts: 5,
            initial_delay_ms: 100,
            max_delay_ms: 10_000,
            backoff_multiplier: 2,
        };
        // Hint larger than backoff -> hint wins (never retry sooner than the
        // server asked).
        assert_eq!(cfg.effective_delay_ms(0, Some(5_000)), 5_000);
        // Backoff larger than hint -> backoff wins (never retry sooner than our
        // own schedule).
        assert_eq!(cfg.effective_delay_ms(3, Some(100)), 800);
        // Equal -> that value.
        assert_eq!(cfg.effective_delay_ms(1, Some(200)), 200);
        // A zero hint never shortens the backoff below its scheduled value.
        assert_eq!(cfg.effective_delay_ms(2, Some(0)), 400);
    }

    #[tokio::test]
    async fn test_push_availability_retries_then_succeeds_on_transient_5xx() {
        // First call returns 503 (retryable), second returns an OTA success
        // body. The push must retry and ultimately succeed.
        let server = MockOtaServer::spawn(vec![
            MockResponse::status(503, "<busy/>"),
            MockResponse::status(
                200,
                "<OTA_HotelAvailNotifRS xmlns=\"http://www.opentravel.org/OTA/2003/05\"><Success/></OTA_HotelAvailNotifRS>",
            ),
        ])
        .await;

        let creds = BookingCredentials::with_url(
            "H1".to_string(),
            "u".to_string(),
            "p".to_string(),
            server.url(),
        );
        let client = BookingClient::new(creds).with_retry(BookingRetryConfig {
            max_attempts: 3,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            backoff_multiplier: 1,
        });

        let updates = vec![AvailabilityUpdate {
            room_type_id: "DBL".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            available_count: 4,
            stop_sell: false,
            cta: false,
            ctd: false,
            min_los: None,
            max_los: None,
        }];

        let result = client.push_availability("H1", &updates).await;
        assert!(result.is_ok(), "expected success after retry: {result:?}");
        assert_eq!(server.hits(), 2, "expected exactly one retry");
    }

    #[tokio::test]
    async fn test_push_rates_does_not_retry_on_client_error() {
        // 400 is non-retryable: the push must fail after a single attempt.
        let server = MockOtaServer::spawn(vec![
            MockResponse::status(400, "<bad-request/>"),
            // Second response should never be consumed.
            MockResponse::status(200, "<Success/>"),
        ])
        .await;

        let creds = BookingCredentials::with_url(
            "H1".to_string(),
            "u".to_string(),
            "p".to_string(),
            server.url(),
        );
        let client = BookingClient::new(creds).with_retry(BookingRetryConfig {
            max_attempts: 4,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            backoff_multiplier: 1,
        });

        let updates = vec![RateUpdate {
            room_type_id: "DBL".to_string(),
            rate_plan_code: "STD".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            base_rate: "120.00".parse().unwrap(),
            currency: "EUR".to_string(),
            extra_person_rate: None,
            extra_child_rate: None,
        }];

        let result = client.push_rates("H1", &updates).await;
        assert!(result.is_err(), "400 must not be retried into success");
        assert_eq!(server.hits(), 1, "client error must not be retried");
    }

    #[tokio::test]
    async fn test_push_rates_exhausts_retries_on_persistent_5xx() {
        let server = MockOtaServer::spawn(vec![
            MockResponse::status(503, "<busy/>"),
            MockResponse::status(503, "<busy/>"),
        ])
        .await;

        let creds = BookingCredentials::with_url(
            "H1".to_string(),
            "u".to_string(),
            "p".to_string(),
            server.url(),
        );
        let client = BookingClient::new(creds).with_retry(BookingRetryConfig {
            max_attempts: 2,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            backoff_multiplier: 1,
        });

        let updates = vec![RateUpdate {
            room_type_id: "DBL".to_string(),
            rate_plan_code: "STD".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            base_rate: "120.00".parse().unwrap(),
            currency: "EUR".to_string(),
            extra_person_rate: None,
            extra_child_rate: None,
        }];

        let result = client.push_rates("H1", &updates).await;
        assert!(result.is_err(), "persistent 5xx must fail");
        assert_eq!(server.hits(), 2, "should exhaust exactly max_attempts");
    }

    #[tokio::test]
    async fn test_push_availability_retries_after_transport_error() {
        // First attempt: the server drops the connection without replying,
        // producing a reqwest transport error. That must be treated as
        // transient and retried; the second attempt returns an OTA success.
        let server = MockOtaServer::spawn(vec![
            MockResponse::drop_connection(),
            MockResponse::status(
                200,
                "<OTA_HotelAvailNotifRS xmlns=\"http://www.opentravel.org/OTA/2003/05\"><Success/></OTA_HotelAvailNotifRS>",
            ),
        ])
        .await;

        let creds = BookingCredentials::with_url(
            "H1".to_string(),
            "u".to_string(),
            "p".to_string(),
            server.url(),
        );
        let client = BookingClient::new(creds)
            .with_retry(BookingRetryConfig::no_retry().clone_with_attempts(3));

        let result = client.push_availability("H1", &[avail_update(4)]).await;
        assert!(
            result.is_ok(),
            "transport error on first attempt must be retried into success: {result:?}"
        );
        assert_eq!(
            server.hits(),
            2,
            "expected exactly one retry after the reset"
        );
    }

    #[tokio::test]
    async fn test_push_rates_exhausts_retries_on_persistent_transport_error() {
        // Every attempt drops the connection: the push must exhaust its
        // attempts and surface a Network error rather than looping forever.
        let server = MockOtaServer::spawn(vec![
            MockResponse::drop_connection(),
            MockResponse::drop_connection(),
        ])
        .await;

        let creds = BookingCredentials::with_url(
            "H1".to_string(),
            "u".to_string(),
            "p".to_string(),
            server.url(),
        );
        let client = BookingClient::new(creds)
            .with_retry(BookingRetryConfig::no_retry().clone_with_attempts(2));

        let result = client.push_rates("H1", &[rate_update("120.00")]).await;
        assert!(
            matches!(result, Err(BookingError::Network(_))),
            "persistent transport failure must surface as Network error: {result:?}"
        );
        assert_eq!(server.hits(), 2, "should exhaust exactly max_attempts");
    }

    #[tokio::test]
    async fn test_push_429_with_retry_after_returns_rate_limited() {
        // Both attempts return 429 with Retry-After: 0 so the test stays fast
        // (delay capped to 0 via max_delay_ms=0). Verify:
        //   a) all attempts are consumed, and
        //   b) the final error is RateLimited, not a generic Api error.
        let server = MockOtaServer::spawn(vec![
            MockResponse::status(429, "<rate-limited/>").with_header("Retry-After", "0"),
            MockResponse::status(429, "<rate-limited/>").with_header("Retry-After", "0"),
        ])
        .await;

        let creds = BookingCredentials::with_url(
            "H1".to_string(),
            "u".to_string(),
            "p".to_string(),
            server.url(),
        );
        let client = BookingClient::new(creds).with_retry(BookingRetryConfig {
            max_attempts: 2,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            backoff_multiplier: 1,
        });

        let updates = vec![AvailabilityUpdate {
            room_type_id: "DBL".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            available_count: 2,
            stop_sell: false,
            cta: false,
            ctd: false,
            min_los: None,
            max_los: None,
        }];

        let result = client.push_availability("H1", &updates).await;
        assert!(
            matches!(result, Err(BookingError::RateLimited(_))),
            "429 with Retry-After must surface as RateLimited, got: {result:?}"
        );
        assert_eq!(server.hits(), 2, "should exhaust exactly max_attempts");
    }

    #[tokio::test]
    async fn test_push_429_without_retry_after_returns_rate_limited_zero() {
        // Naked 429 with no Retry-After header must also surface as
        // RateLimited(0), not Api("HTTP 429: …").
        let server = MockOtaServer::spawn(vec![
            MockResponse::status(429, "<too-many/>"),
            MockResponse::status(429, "<too-many/>"),
        ])
        .await;

        let creds = BookingCredentials::with_url(
            "H1".to_string(),
            "u".to_string(),
            "p".to_string(),
            server.url(),
        );
        let client = BookingClient::new(creds).with_retry(BookingRetryConfig {
            max_attempts: 2,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            backoff_multiplier: 1,
        });

        let updates = vec![AvailabilityUpdate {
            room_type_id: "DBL".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            available_count: 2,
            stop_sell: false,
            cta: false,
            ctd: false,
            min_los: None,
            max_los: None,
        }];

        let result = client.push_availability("H1", &updates).await;
        assert_eq!(
            server.hits(),
            2,
            "should exhaust max_attempts on persistent 429"
        );
        match result {
            Err(BookingError::RateLimited(secs)) => {
                assert_eq!(secs, 0, "no Retry-After header → RateLimited(0)");
            }
            other => panic!("expected RateLimited(0), got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_push_429_retry_after_secs_propagated_to_error() {
        // Retry-After: 5 must propagate into RateLimited(5).
        let server = MockOtaServer::spawn(vec![
            MockResponse::status(429, "<too-many/>").with_header("Retry-After", "5"),
            MockResponse::status(429, "<too-many/>").with_header("Retry-After", "5"),
        ])
        .await;

        let creds = BookingCredentials::with_url(
            "H1".to_string(),
            "u".to_string(),
            "p".to_string(),
            server.url(),
        );
        // max_delay_ms = 0 clamps the Retry-After sleep to 0 so the test
        // stays fast; the important assertion is the error value, not the timing.
        let client = BookingClient::new(creds).with_retry(BookingRetryConfig {
            max_attempts: 2,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            backoff_multiplier: 1,
        });

        let updates = vec![AvailabilityUpdate {
            room_type_id: "DBL".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            available_count: 2,
            stop_sell: false,
            cta: false,
            ctd: false,
            min_los: None,
            max_los: None,
        }];

        let result = client.push_availability("H1", &updates).await;
        assert_eq!(server.hits(), 2, "should exhaust max_attempts");
        match result {
            Err(BookingError::RateLimited(secs)) => {
                assert_eq!(secs, 5, "Retry-After: 5 → RateLimited(5)");
            }
            other => panic!("expected RateLimited(5), got: {other:?}"),
        }
    }

    // ==================== Idempotency (AC-5) ====================

    fn avail_update(count: i32) -> AvailabilityUpdate {
        AvailabilityUpdate {
            room_type_id: "DBL".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            available_count: count,
            stop_sell: false,
            cta: false,
            ctd: false,
            min_los: None,
            max_los: None,
        }
    }

    fn rate_update(rate: &str) -> RateUpdate {
        RateUpdate {
            room_type_id: "DBL".to_string(),
            rate_plan_code: "STD".to_string(),
            date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            base_rate: rate.parse().unwrap(),
            currency: "EUR".to_string(),
            extra_person_rate: None,
            extra_child_rate: None,
        }
    }

    #[test]
    fn test_echo_token_is_deterministic_for_identical_payload() {
        let a = ota_xml::compute_echo_token("avail", "H1|DBL:STD:2025-06-01");
        let b = ota_xml::compute_echo_token("avail", "H1|DBL:STD:2025-06-01");
        assert_eq!(
            a, b,
            "identical payload must yield identical idempotency key"
        );
        assert_eq!(a.len(), 32, "token is a 128-bit hex digest");
    }

    #[test]
    fn test_echo_token_differs_for_different_payload_and_kind() {
        let base = ota_xml::compute_echo_token("avail", "H1|DBL:STD:2025-06-01:4");
        // Different content -> different token.
        assert_ne!(
            base,
            ota_xml::compute_echo_token("avail", "H1|DBL:STD:2025-06-01:5")
        );
        // Same content, different message kind -> different token (no collision
        // between an availability push and a rate push).
        assert_ne!(
            base,
            ota_xml::compute_echo_token("rate", "H1|DBL:STD:2025-06-01:4")
        );
    }

    #[test]
    fn test_avail_xml_stamps_deterministic_echo_token() {
        let rq = OtaHotelAvailNotifRQ {
            hotel_code: "H1".to_string(),
            avail_status_messages: vec![AvailStatusMessage {
                room_type_code: "DBL".to_string(),
                rate_plan_code: Some("STD".to_string()),
                start_date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
                booking_limit: 4,
                status: "Open".to_string(),
                los_restrictions: None,
            }],
        };
        let first = rq.to_xml();
        let second = rq.to_xml();
        let t1 = echo_token_of(&first).expect("avail RQ must carry an EchoToken");
        let t2 = echo_token_of(&second).expect("avail RQ must carry an EchoToken");
        assert_eq!(t1, t2, "rebuilding the same push must reuse the same token");
    }

    #[test]
    fn test_rate_xml_stamps_echo_token_and_changes_with_content() {
        let token_for = |rate: &str| {
            let xml = ota_xml::build_rate_amount_notif_rq(
                "H1",
                &[(
                    NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
                    NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
                    "DBL",
                    "STD",
                    &rate.parse::<Decimal>().unwrap(),
                    "EUR",
                )],
            )
            .unwrap();
            echo_token_of(&xml).expect("rate RQ must carry an EchoToken")
        };
        assert_eq!(token_for("120.00"), token_for("120.00"));
        assert_ne!(token_for("120.00"), token_for("130.00"));
    }

    #[tokio::test]
    async fn test_push_availability_reuses_echo_token_across_retries() {
        // 503 then 200: the push retries. Both requests must carry the SAME
        // EchoToken so the upstream can deduplicate the retried delivery.
        let server = MockOtaServer::spawn(vec![
            MockResponse::status(503, "<busy/>"),
            MockResponse::status(
                200,
                "<OTA_HotelAvailNotifRS xmlns=\"http://www.opentravel.org/OTA/2003/05\"><Success/></OTA_HotelAvailNotifRS>",
            ),
        ])
        .await;

        let creds = BookingCredentials::with_url(
            "H1".to_string(),
            "u".to_string(),
            "p".to_string(),
            server.url(),
        );
        let client = BookingClient::new(creds)
            .with_retry(BookingRetryConfig::no_retry().clone_with_attempts(3));

        let result = client.push_availability("H1", &[avail_update(4)]).await;
        assert!(result.is_ok(), "expected success after retry: {result:?}");
        assert_eq!(server.hits(), 2, "expected exactly one retry");

        let bodies = server.bodies();
        assert_eq!(bodies.len(), 2, "both delivery attempts captured");
        let t0 = echo_token_of(&bodies[0]).expect("attempt 0 must carry a token");
        let t1 = echo_token_of(&bodies[1]).expect("attempt 1 must carry a token");
        assert_eq!(
            t0, t1,
            "retried delivery must reuse the same idempotency key"
        );
    }

    #[tokio::test]
    async fn test_push_rates_reuses_echo_token_across_retries() {
        let server = MockOtaServer::spawn(vec![
            MockResponse::status(503, "<busy/>"),
            MockResponse::status(
                200,
                "<OTA_HotelRateAmountNotifRS xmlns=\"http://www.opentravel.org/OTA/2003/05\"><Success/></OTA_HotelRateAmountNotifRS>",
            ),
        ])
        .await;

        let creds = BookingCredentials::with_url(
            "H1".to_string(),
            "u".to_string(),
            "p".to_string(),
            server.url(),
        );
        let client = BookingClient::new(creds)
            .with_retry(BookingRetryConfig::no_retry().clone_with_attempts(3));

        let result = client.push_rates("H1", &[rate_update("120.00")]).await;
        assert!(result.is_ok(), "expected success after retry: {result:?}");
        assert_eq!(server.hits(), 2);

        let bodies = server.bodies();
        let t0 = echo_token_of(&bodies[0]).expect("attempt 0 token");
        let t1 = echo_token_of(&bodies[1]).expect("attempt 1 token");
        assert_eq!(t0, t1, "rate retry must reuse the same idempotency key");
    }

    // ==================== Combined push (PushOutcome, AC-5) ====================

    const AVAIL_OK_BODY: &str = "<OTA_HotelAvailNotifRS xmlns=\"http://www.opentravel.org/OTA/2003/05\"><Success/></OTA_HotelAvailNotifRS>";
    const RATE_OK_BODY: &str = "<OTA_HotelRateAmountNotifRS xmlns=\"http://www.opentravel.org/OTA/2003/05\"><Success/></OTA_HotelRateAmountNotifRS>";
    const ERR_BODY: &str = "<OTA_HotelRateAmountNotifRS xmlns=\"http://www.opentravel.org/OTA/2003/05\"><Errors><Error Type=\"3\" Code=\"450\" ShortText=\"Rate rejected\"/></Errors></OTA_HotelRateAmountNotifRS>";

    fn client_for(server: &MockOtaServer) -> BookingClient {
        let creds = BookingCredentials::with_url(
            "H1".to_string(),
            "u".to_string(),
            "p".to_string(),
            server.url(),
        );
        BookingClient::new(creds).with_retry(BookingRetryConfig::no_retry())
    }

    #[test]
    fn test_push_outcome_success_and_partial_helpers() {
        let clean = PushOutcome {
            availability_pushed: 2,
            rates_pushed: 1,
            availability_error: None,
            rates_error: None,
        };
        assert!(clean.is_success());
        assert!(!clean.is_partial());

        let half = PushOutcome {
            availability_pushed: 2,
            rates_pushed: 0,
            availability_error: None,
            rates_error: Some("Rate rejected".to_string()),
        };
        assert!(!half.is_success());
        assert!(half.is_partial());

        let both = PushOutcome {
            availability_pushed: 0,
            rates_pushed: 0,
            availability_error: Some("a".to_string()),
            rates_error: Some("b".to_string()),
        };
        assert!(!both.is_success());
        // Both failed -> not partial (it's a total failure, nothing landed).
        assert!(!both.is_partial());
    }

    #[tokio::test]
    async fn test_combined_push_both_streams_succeed() {
        // Availability push (1 HTTP call) then rate push (1 HTTP call).
        let server = MockOtaServer::spawn(vec![
            MockResponse::status(200, AVAIL_OK_BODY),
            MockResponse::status(200, RATE_OK_BODY),
        ])
        .await;
        let client = client_for(&server);

        let outcome = client
            .push_availability_and_rates("H1", &[avail_update(4)], &[rate_update("120.00")])
            .await;

        assert!(
            outcome.is_success(),
            "both streams should apply: {outcome:?}"
        );
        assert!(!outcome.is_partial());
        assert_eq!(outcome.availability_pushed, 1);
        assert_eq!(outcome.rates_pushed, 1);
        assert_eq!(server.hits(), 2, "one HTTP call per stream");
    }

    #[tokio::test]
    async fn test_combined_push_availability_ok_rates_fail_is_partial() {
        // Availability succeeds; rates come back with an OTA <Errors> body.
        // The availability success must NOT be lost just because rates failed.
        let server = MockOtaServer::spawn(vec![
            MockResponse::status(200, AVAIL_OK_BODY),
            MockResponse::status(200, ERR_BODY),
        ])
        .await;
        let client = client_for(&server);

        let outcome = client
            .push_availability_and_rates("H1", &[avail_update(4)], &[rate_update("120.00")])
            .await;

        assert!(!outcome.is_success());
        assert!(
            outcome.is_partial(),
            "exactly one stream failed: {outcome:?}"
        );
        assert_eq!(outcome.availability_pushed, 1, "availability still applied");
        assert_eq!(outcome.rates_pushed, 0, "rates did not apply");
        assert!(outcome.availability_error.is_none());
        assert!(
            outcome
                .rates_error
                .as_deref()
                .is_some_and(|e| e.contains("Rate rejected")),
            "rate error text must be surfaced: {outcome:?}"
        );
    }

    #[tokio::test]
    async fn test_combined_push_no_op_when_both_empty() {
        // No availability and no rates: a successful no-op that makes zero
        // HTTP calls.
        let server = MockOtaServer::spawn(vec![]).await;
        let client = client_for(&server);

        let outcome = client.push_availability_and_rates("H1", &[], &[]).await;

        assert!(outcome.is_success());
        assert!(!outcome.is_partial());
        assert_eq!(outcome.availability_pushed, 0);
        assert_eq!(outcome.rates_pushed, 0);
        assert_eq!(server.hits(), 0, "empty sync must not hit the network");
    }

    #[tokio::test]
    async fn test_combined_push_continues_rates_after_availability_failure() {
        // Availability fails (non-retryable 400) but rates must still be
        // attempted — the failure of one stream cannot abort the other.
        let server = MockOtaServer::spawn(vec![
            MockResponse::status(400, "<bad-request/>"),
            MockResponse::status(200, RATE_OK_BODY),
        ])
        .await;
        let client = client_for(&server);

        let outcome = client
            .push_availability_and_rates("H1", &[avail_update(4)], &[rate_update("99.00")])
            .await;

        assert!(outcome.is_partial(), "one stream failed: {outcome:?}");
        assert_eq!(outcome.availability_pushed, 0);
        assert_eq!(
            outcome.rates_pushed, 1,
            "rates attempted despite avail failure"
        );
        assert!(outcome.availability_error.is_some());
        assert!(outcome.rates_error.is_none());
        assert_eq!(server.hits(), 2, "both streams hit the network");
    }

    #[test]
    fn test_map_reservation_status() {
        // Commit -> Confirmed
        let xml = r#"<HotelReservation ResStatus="Commit"><ResGlobalInfo><HotelReservationIDs><HotelReservationID ResID_Value="BK-001"/></HotelReservationIDs></ResGlobalInfo></HotelReservation>"#;
        let result = OtaReadRS::parse_single_reservation(xml);
        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.status, BookingReservationStatus::Confirmed);

        // Cancel -> Cancelled
        let xml = r#"<HotelReservation ResStatus="Cancel"><ResGlobalInfo><HotelReservationIDs><HotelReservationID ResID_Value="BK-002"/></HotelReservationIDs></ResGlobalInfo></HotelReservation>"#;
        let result = OtaReadRS::parse_single_reservation(xml);
        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.status, BookingReservationStatus::Cancelled);
    }

    // ----------------------------------------------------------------------
    // OtaReadRS::from_xml — response-envelope classification.
    //
    // parse_single_reservation is covered above; these lock in the four
    // envelope branches of from_xml (explicit <Success/>, implicit
    // <ReservationsList>, an <Errors> response, and the indeterminate
    // no-marker case) plus the multi-reservation fan-out. Behaviour-only
    // characterisation — no production code changes.
    // ----------------------------------------------------------------------

    #[test]
    fn test_from_xml_explicit_success_parses_reservation() {
        // <Success/> present, no <ReservationsList> wrapper: the explicit
        // success branch must still parse the embedded reservation.
        let xml = r#"<OTA_ReadRS><Success/><HotelReservation ResStatus="Commit"><ResGlobalInfo><HotelReservationIDs><HotelReservationID ResID_Value="BK-EXPL-1"/></HotelReservationIDs></ResGlobalInfo></HotelReservation></OTA_ReadRS>"#;
        let rs = OtaReadRS::from_xml(xml).expect("parse must not error");
        assert!(rs.success, "explicit <Success/> is a success envelope");
        assert!(rs.error.is_none());
        assert_eq!(rs.reservations.len(), 1);
        assert_eq!(rs.reservations[0].reservation_id, "BK-EXPL-1");
        assert_eq!(
            rs.reservations[0].status,
            BookingReservationStatus::Confirmed
        );
    }

    #[test]
    fn test_from_xml_implicit_success_via_reservations_list() {
        // No <Success/>, but a <ReservationsList> element implies success
        // for the Booking.com variants that omit the explicit marker.
        let xml = r#"<OTA_ReadRS><ReservationsList><HotelReservation ResStatus="Commit"><ResGlobalInfo><HotelReservationIDs><HotelReservationID ResID_Value="BK-IMPL-1"/></HotelReservationIDs></ResGlobalInfo></HotelReservation></ReservationsList></OTA_ReadRS>"#;
        let rs = OtaReadRS::from_xml(xml).expect("parse must not error");
        assert!(rs.success, "<ReservationsList> is an implicit success");
        assert!(rs.error.is_none());
        assert_eq!(rs.reservations.len(), 1);
        assert_eq!(rs.reservations[0].reservation_id, "BK-IMPL-1");
    }

    #[test]
    fn test_from_xml_error_response_is_unsuccessful() {
        // An <Errors> envelope must surface success=false with the
        // ShortText carried through as the error message, and no
        // reservations parsed.
        let xml = r#"<OTA_ReadRS><Errors><Error Type="1" ShortText="Authentication failed"/></Errors></OTA_ReadRS>"#;
        let rs = OtaReadRS::from_xml(xml).expect("parse must not error");
        assert!(!rs.success, "<Errors> envelope is not a success");
        assert_eq!(rs.error.as_deref(), Some("Authentication failed"));
        assert!(rs.reservations.is_empty());
    }

    #[test]
    fn test_from_xml_indeterminate_response_is_unsuccessful() {
        // No <Success>, no <Errors>, no <ReservationsList>: the parser must
        // NOT silently treat this as success (that would mask a failed
        // pull). success=false with a non-empty error and no reservations.
        let xml = r#"<OTA_ReadRS></OTA_ReadRS>"#;
        let rs = OtaReadRS::from_xml(xml).expect("parse must not error");
        assert!(!rs.success, "indeterminate envelope must not be success");
        assert!(
            rs.error.is_some(),
            "indeterminate response must carry an error"
        );
        assert!(rs.reservations.is_empty());
    }

    #[test]
    fn test_from_xml_parses_multiple_reservations() {
        // The parse_reservations fan-out must return one entry per
        // <HotelReservation> element, in document order.
        let xml = r#"<OTA_ReadRS><Success/><ReservationsList><HotelReservation ResStatus="Commit"><ResGlobalInfo><HotelReservationIDs><HotelReservationID ResID_Value="BK-A"/></HotelReservationIDs></ResGlobalInfo></HotelReservation><HotelReservation ResStatus="Cancel"><ResGlobalInfo><HotelReservationIDs><HotelReservationID ResID_Value="BK-B"/></HotelReservationIDs></ResGlobalInfo></HotelReservation></ReservationsList></OTA_ReadRS>"#;
        let rs = OtaReadRS::from_xml(xml).expect("parse must not error");
        assert!(rs.success);
        assert_eq!(rs.reservations.len(), 2);
        assert_eq!(rs.reservations[0].reservation_id, "BK-A");
        assert_eq!(
            rs.reservations[0].status,
            BookingReservationStatus::Confirmed
        );
        assert_eq!(rs.reservations[1].reservation_id, "BK-B");
        assert_eq!(
            rs.reservations[1].status,
            BookingReservationStatus::Cancelled
        );
    }

    // ----------------------------------------------------------------------
    // OtaHotelRateAmountNotifRQ / RS typed request/response models (Story 83.2)
    // ----------------------------------------------------------------------

    fn rate_update_on(room: &str, rate: &str, date: NaiveDate) -> RateUpdate {
        RateUpdate {
            room_type_id: room.to_string(),
            rate_plan_code: "STD".to_string(),
            date,
            base_rate: rate.parse::<Decimal>().unwrap(),
            currency: "EUR".to_string(),
            extra_person_rate: None,
            extra_child_rate: None,
        }
    }

    #[test]
    fn test_rate_amount_notif_rq_to_xml_shape() {
        let rq = OtaHotelRateAmountNotifRQ {
            hotel_code: "H-RT-99".to_string(),
            rate_amount_messages: vec![rate_update_on(
                "DBL",
                "120.50",
                NaiveDate::from_ymd_opt(2025, 7, 1).unwrap(),
            )],
        };
        let xml = rq.to_xml().expect("serialisation must succeed");
        assert!(xml.contains("OTA_HotelRateAmountNotifRQ"));
        assert!(xml.contains(&format!("xmlns=\"{OTA_NAMESPACE}\"")));
        assert!(xml.contains("HotelCode=\"H-RT-99\""));
        assert!(xml.contains("InvTypeCode=\"DBL\""));
        assert!(xml.contains("RatePlanCode=\"STD\""));
        assert!(xml.contains("AmountAfterTax=\"120.50\""));
        assert!(xml.contains("CurrencyCode=\"EUR\""));
        // Single-day update: Start == End.
        assert!(xml.contains("Start=\"2025-07-01\""));
        assert!(xml.contains("End=\"2025-07-01\""));
    }

    #[test]
    fn test_rate_amount_notif_rq_from_xml_parses_typed_model() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<OTA_HotelRateAmountNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05" Version="1.0">
  <RateAmountMessages HotelCode="H-IN-7">
    <RateAmountMessage>
      <StatusApplicationControl Start="2025-08-10" End="2025-08-10" InvTypeCode="STE" RatePlanCode="FLEX"/>
      <Rates><Rate><BaseByGuestAmts>
        <BaseByGuestAmt AmountAfterTax="250.00" CurrencyCode="EUR"/>
      </BaseByGuestAmts></Rate></Rates>
    </RateAmountMessage>
  </RateAmountMessages>
</OTA_HotelRateAmountNotifRQ>"#;
        let rq = OtaHotelRateAmountNotifRQ::from_xml(xml).expect("parse must succeed");
        assert_eq!(rq.hotel_code, "H-IN-7");
        assert_eq!(rq.rate_amount_messages.len(), 1);
        let m = &rq.rate_amount_messages[0];
        assert_eq!(m.room_type_id, "STE");
        assert_eq!(m.rate_plan_code, "FLEX");
        assert_eq!(m.date, NaiveDate::from_ymd_opt(2025, 8, 10).unwrap());
        assert_eq!(m.base_rate, "250.00".parse::<Decimal>().unwrap());
        assert_eq!(m.currency, "EUR");
        assert_eq!(m.extra_person_rate, None);
    }

    #[test]
    fn test_rate_amount_notif_rq_round_trip() {
        let original = OtaHotelRateAmountNotifRQ {
            hotel_code: "H-RT-RT".to_string(),
            rate_amount_messages: vec![
                rate_update_on("DBL", "99.00", NaiveDate::from_ymd_opt(2025, 9, 1).unwrap()),
                rate_update_on(
                    "STE",
                    "180.00",
                    NaiveDate::from_ymd_opt(2025, 9, 2).unwrap(),
                ),
            ],
        };
        let xml = original.to_xml().unwrap();
        let parsed = OtaHotelRateAmountNotifRQ::from_xml(&xml).unwrap();
        assert_eq!(parsed.hotel_code, original.hotel_code);
        assert_eq!(parsed.rate_amount_messages.len(), 2);
        for (got, want) in parsed
            .rate_amount_messages
            .iter()
            .zip(original.rate_amount_messages.iter())
        {
            assert_eq!(got.room_type_id, want.room_type_id);
            assert_eq!(got.rate_plan_code, want.rate_plan_code);
            assert_eq!(got.date, want.date);
            assert_eq!(got.base_rate, want.base_rate);
            assert_eq!(got.currency, want.currency);
        }
    }

    #[test]
    fn test_rate_amount_notif_rq_from_xml_bad_amount_errors() {
        let xml = r#"<OTA_HotelRateAmountNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05">
  <RateAmountMessages HotelCode="H">
    <RateAmountMessage>
      <StatusApplicationControl Start="2025-08-10" End="2025-08-10" InvTypeCode="STE" RatePlanCode="FLEX"/>
      <Rates><Rate><BaseByGuestAmts>
        <BaseByGuestAmt AmountAfterTax="not-a-number" CurrencyCode="EUR"/>
      </BaseByGuestAmts></Rate></Rates>
    </RateAmountMessage>
  </RateAmountMessages>
</OTA_HotelRateAmountNotifRQ>"#;
        assert!(OtaHotelRateAmountNotifRQ::from_xml(xml).is_err());
    }

    #[test]
    fn test_rate_amount_notif_rs_success_and_error() {
        let ok = OtaHotelRateAmountNotifRS::from_xml(
            "<OTA_HotelRateAmountNotifRS xmlns=\"http://www.opentravel.org/OTA/2003/05\"><Success/></OTA_HotelRateAmountNotifRS>",
        )
        .unwrap();
        assert!(ok.success);
        assert!(ok.error.is_none());

        let err = OtaHotelRateAmountNotifRS::from_xml(
            "<OTA_HotelRateAmountNotifRS xmlns=\"http://www.opentravel.org/OTA/2003/05\"><Errors><Error ShortText=\"rate rejected\"/></Errors></OTA_HotelRateAmountNotifRS>",
        )
        .unwrap();
        assert!(!err.success);
        assert_eq!(err.error.as_deref(), Some("rate rejected"));
    }

    // ==================== fetch_property outbound happy path ====================

    /// A well-formed `OTA_HotelDescriptiveInfoRS` success body carrying the
    /// property fields `parse_property_response` extracts. Deliberately free of
    /// any `<Errors>` / `<Error` marker so the parser takes the success branch.
    const HOTEL_DESCRIPTIVE_INFO_RS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<OTA_HotelDescriptiveInfoRS xmlns="http://www.opentravel.org/OTA/2003/05" Version="1.0">
  <Success/>
  <HotelDescriptiveContents>
    <HotelDescriptiveContent HotelCode="H42" HotelName="Grand Test Hotel">
      <HotelInfo Rating="4">
        <Descriptions>
          <DescriptiveText>A lovely seaside hotel.</DescriptiveText>
        </Descriptions>
        <Position Latitude="48.1486" Longitude="17.1077"/>
      </HotelInfo>
      <ContactInfos>
        <ContactInfo>
          <Address CountryCode="SK">
            <AddressLine>123 Ocean Drive</AddressLine>
            <CityName>Bratislava</CityName>
            <StateProv>BL</StateProv>
            <PostalCode>81101</PostalCode>
          </Address>
          <Email>info@grandtest.example</Email>
          <Phone PhoneNumber="+421900000000"/>
          <URL>https://grandtest.example</URL>
        </ContactInfo>
      </ContactInfos>
      <Policies>
        <Policy>
          <PolicyInfo CheckInTime="14:00" CheckOutTime="11:00"/>
        </Policy>
      </Policies>
    </HotelDescriptiveContent>
  </HotelDescriptiveContents>
</OTA_HotelDescriptiveInfoRS>"#;

    /// The connect / sync "happy path" hinges on `fetch_property` issuing a real
    /// `OTA_HotelDescriptiveInfoRQ` POST and parsing the descriptive-info
    /// response — a path that was previously only reachable against the live
    /// Booking.com endpoint (documented limitation). The base-URL seam
    /// (`BookingCredentials::with_url`, mirroring Airbnb's `with_base_url`
    /// from #2240) lets us point the client at [`MockOtaServer`] and exercise
    /// the outbound call + response parsing deterministically.
    #[tokio::test]
    async fn test_fetch_property_happy_path_issues_request_and_parses_response() {
        let server =
            MockOtaServer::spawn(vec![MockResponse::status(200, HOTEL_DESCRIPTIVE_INFO_RS)]).await;

        let creds = BookingCredentials::with_url(
            "H42".to_string(),
            "user".to_string(),
            "pass".to_string(),
            server.url(),
        );
        let client = BookingClient::new(creds);

        let property = client
            .fetch_property("H42")
            .await
            .expect("fetch_property should succeed on a well-formed OTA response");

        // Exactly one outbound call was made.
        assert_eq!(server.hits(), 1, "fetch_property should POST once");

        // The outbound request is a namespaced OTA_HotelDescriptiveInfoRQ that
        // carries the requested hotel code.
        let sent = server.bodies();
        assert_eq!(sent.len(), 1);
        assert!(
            sent[0].contains("OTA_HotelDescriptiveInfoRQ"),
            "outbound body should be an OTA_HotelDescriptiveInfoRQ, got: {}",
            sent[0]
        );
        assert!(
            sent[0].contains("HotelCode=\"H42\""),
            "outbound body should target the requested hotel code, got: {}",
            sent[0]
        );

        // The response was parsed into the typed property.
        assert_eq!(property.hotel_id, "H42");
        assert_eq!(property.name, "Grand Test Hotel");
        assert_eq!(
            property.description.as_deref(),
            Some("A lovely seaside hotel.")
        );
        assert_eq!(property.star_rating, Some(4));
        assert_eq!(property.address.street, "123 Ocean Drive");
        assert_eq!(property.address.city, "Bratislava");
        assert_eq!(property.address.state.as_deref(), Some("BL"));
        assert_eq!(property.address.postal_code, "81101");
        assert_eq!(property.address.country_code, "SK");
        assert_eq!(
            property.contact.email.as_deref(),
            Some("info@grandtest.example")
        );
        assert_eq!(property.contact.phone.as_deref(), Some("+421900000000"));
        assert_eq!(
            property.contact.website.as_deref(),
            Some("https://grandtest.example")
        );
        assert_eq!(property.check_in_time.as_deref(), Some("14:00"));
        assert_eq!(property.check_out_time.as_deref(), Some("11:00"));
        assert!(property.synced_at.is_some());
    }

    /// `fetch_properties` fans out to `fetch_property` for the configured hotel
    /// id; on the happy path it returns the single parsed property (rather than
    /// the placeholder fallback used when the descriptive-info call fails).
    #[tokio::test]
    async fn test_fetch_properties_happy_path_returns_parsed_property() {
        let server =
            MockOtaServer::spawn(vec![MockResponse::status(200, HOTEL_DESCRIPTIVE_INFO_RS)]).await;

        let creds = BookingCredentials::with_url(
            "H42".to_string(),
            "user".to_string(),
            "pass".to_string(),
            server.url(),
        );
        let client = BookingClient::new(creds);

        let properties = client
            .fetch_properties()
            .await
            .expect("fetch_properties should succeed");

        assert_eq!(properties.len(), 1);
        assert_eq!(properties[0].hotel_id, "H42");
        // The real parsed name proves we took the success path, not the
        // "Property {hotel_id}" placeholder built on fetch failure.
        assert_eq!(properties[0].name, "Grand Test Hotel");
    }
}
