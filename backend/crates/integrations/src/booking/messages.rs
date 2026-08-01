//! OTA message request/response types for the Booking.com Connectivity API.
//!
//! Typed models mirroring the OpenTravel Alliance (OTA) XML messages exchanged
//! with Booking.com. Serialisation and parsing delegate to the low-level
//! [`super::ota_xml`] quick-xml helpers so the wire format stays in one place.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::BookingError;
use super::models::{map_reservation_status, BookingGuest, BookingReservation};
use super::ota_xml;

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
    pub(super) fn parse_single_reservation(xml: &str) -> Result<BookingReservation, BookingError> {
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
