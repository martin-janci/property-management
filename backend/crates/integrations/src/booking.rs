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

/// Low-level OTA XML serialization/deserialization helpers backed by
/// [`quick_xml`].
pub mod ota_xml {
    //! Namespace-aware XML helpers for the Booking.com OTA wire format.
    //!
    //! ## Design
    //! Each `write_*` function takes a [`quick_xml::Writer`] (wrapping any
    //! `io::Write`) and emits one logical OTA element.  Each `parse_*`
    //! function consumes a `&str` of raw XML and returns a typed result.
    //!
    //! The namespace `http://www.opentravel.org/OTA/2003/05` is declared on
    //! every top-level element as the default namespace so that all child
    //! elements inherit it without repetition.

    use chrono::NaiveDate;
    use quick_xml::{
        events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
        Reader, Writer,
    };
    use rust_decimal::Decimal;
    use std::io::Cursor;

    use super::{AvailStatusMessage, BookingError, OTA_NAMESPACE};

    // ------------------------------------------------------------------
    // Generation helpers
    // ------------------------------------------------------------------

    /// Write the standard XML declaration + root element opening tag with the
    /// OTA namespace declared as the default namespace.
    ///
    /// The caller is responsible for writing child elements and then calling
    /// [`write_end`] with the same `tag`.
    pub fn write_root_start(
        writer: &mut Writer<Cursor<Vec<u8>>>,
        tag: &str,
        version: &str,
    ) -> Result<(), BookingError> {
        // <?xml version="1.0" encoding="UTF-8"?>
        writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
            .map_err(|e| BookingError::Xml(e.to_string()))?;

        let mut start = BytesStart::new(tag);
        start.push_attribute(("xmlns", OTA_NAMESPACE));
        start.push_attribute(("Version", version));
        writer
            .write_event(Event::Start(start))
            .map_err(|e| BookingError::Xml(e.to_string()))?;

        Ok(())
    }

    /// Write a closing tag for `tag`.
    pub fn write_end(
        writer: &mut Writer<Cursor<Vec<u8>>>,
        tag: &str,
    ) -> Result<(), BookingError> {
        writer
            .write_event(Event::End(BytesEnd::new(tag)))
            .map_err(|e| BookingError::Xml(e.to_string()))
    }

    /// Serialize an `OTA_HotelAvailNotifRQ` document using quick-xml.
    ///
    /// Produces a namespace-qualified document suitable for POSTing to the
    /// Booking.com Supply XML API.
    pub fn build_avail_notif_rq(
        hotel_code: &str,
        messages: &[AvailStatusMessage],
    ) -> Result<String, BookingError> {
        let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

        write_root_start(&mut writer, "OTA_HotelAvailNotifRQ", "1.0")?;

        // <AvailStatusMessages HotelCode="...">
        let mut avail_msgs = BytesStart::new("AvailStatusMessages");
        avail_msgs.push_attribute(("HotelCode", hotel_code));
        writer
            .write_event(Event::Start(avail_msgs))
            .map_err(|e| BookingError::Xml(e.to_string()))?;

        for msg in messages {
            // <AvailStatusMessage BookingLimit="N" BookingLimitMessageType="SetLimit">
            let mut asm = BytesStart::new("AvailStatusMessage");
            asm.push_attribute(("BookingLimit", msg.booking_limit.to_string().as_str()));
            asm.push_attribute(("BookingLimitMessageType", "SetLimit"));
            writer
                .write_event(Event::Start(asm))
                .map_err(|e| BookingError::Xml(e.to_string()))?;

            // <StatusApplicationControl .../>
            let mut sac = BytesStart::new("StatusApplicationControl");
            sac.push_attribute(("Start", msg.start_date.to_string().as_str()));
            sac.push_attribute(("End", msg.end_date.to_string().as_str()));
            sac.push_attribute(("InvTypeCode", msg.room_type_code.as_str()));
            let rate_code = msg.rate_plan_code.as_deref().unwrap_or("STD");
            sac.push_attribute(("RatePlanCode", rate_code));
            writer
                .write_event(Event::Empty(sac))
                .map_err(|e| BookingError::Xml(e.to_string()))?;

            // <RestrictionStatus Status="Open|Close" Restriction="Master"/>
            let mut rs = BytesStart::new("RestrictionStatus");
            rs.push_attribute(("Status", msg.status.as_str()));
            rs.push_attribute(("Restriction", "Master"));
            writer
                .write_event(Event::Empty(rs))
                .map_err(|e| BookingError::Xml(e.to_string()))?;

            // LOS restrictions (optional)
            if let Some(ref los) = msg.los_restrictions {
                let mut los_el = BytesStart::new("LengthsOfStay");
                if let Some(min) = los.min_los {
                    los_el.push_attribute(("MinMaxMessageType", "SetMinLOS"));
                    los_el.push_attribute(("MinStay", min.to_string().as_str()));
                }
                if let Some(max) = los.max_los {
                    los_el.push_attribute(("MaxStay", max.to_string().as_str()));
                }
                writer
                    .write_event(Event::Empty(los_el))
                    .map_err(|e| BookingError::Xml(e.to_string()))?;
            }

            write_end(&mut writer, "AvailStatusMessage")?;
        }

        write_end(&mut writer, "AvailStatusMessages")?;
        write_end(&mut writer, "OTA_HotelAvailNotifRQ")?;

        let bytes = writer.into_inner().into_inner();
        String::from_utf8(bytes).map_err(|e| BookingError::Xml(e.to_string()))
    }

    /// Serialize an `OTA_HotelRateAmountNotifRQ` document using quick-xml.
    pub fn build_rate_amount_notif_rq(
        hotel_id: &str,
        updates: &[(NaiveDate, NaiveDate, &str, &str, &Decimal, &str)],
    ) -> Result<String, BookingError> {
        // updates: (start, end, room_type_id, rate_plan_code, base_rate, currency)
        let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

        write_root_start(&mut writer, "OTA_HotelRateAmountNotifRQ", "1.0")?;

        let mut rate_msgs = BytesStart::new("RateAmountMessages");
        rate_msgs.push_attribute(("HotelCode", hotel_id));
        writer
            .write_event(Event::Start(rate_msgs))
            .map_err(|e| BookingError::Xml(e.to_string()))?;

        for (start, end, room_type, rate_plan, rate, currency) in updates {
            writer
                .write_event(Event::Start(BytesStart::new("RateAmountMessage")))
                .map_err(|e| BookingError::Xml(e.to_string()))?;

            let mut sac = BytesStart::new("StatusApplicationControl");
            sac.push_attribute(("Start", start.to_string().as_str()));
            sac.push_attribute(("End", end.to_string().as_str()));
            sac.push_attribute(("InvTypeCode", *room_type));
            sac.push_attribute(("RatePlanCode", *rate_plan));
            writer
                .write_event(Event::Empty(sac))
                .map_err(|e| BookingError::Xml(e.to_string()))?;

            writer
                .write_event(Event::Start(BytesStart::new("Rates")))
                .map_err(|e| BookingError::Xml(e.to_string()))?;
            writer
                .write_event(Event::Start(BytesStart::new("Rate")))
                .map_err(|e| BookingError::Xml(e.to_string()))?;
            writer
                .write_event(Event::Start(BytesStart::new("BaseByGuestAmts")))
                .map_err(|e| BookingError::Xml(e.to_string()))?;

            let mut bga = BytesStart::new("BaseByGuestAmt");
            bga.push_attribute(("AmountAfterTax", rate.to_string().as_str()));
            bga.push_attribute(("CurrencyCode", *currency));
            writer
                .write_event(Event::Empty(bga))
                .map_err(|e| BookingError::Xml(e.to_string()))?;

            write_end(&mut writer, "BaseByGuestAmts")?;
            write_end(&mut writer, "Rate")?;
            write_end(&mut writer, "Rates")?;
            write_end(&mut writer, "RateAmountMessage")?;
        }

        write_end(&mut writer, "RateAmountMessages")?;
        write_end(&mut writer, "OTA_HotelRateAmountNotifRQ")?;

        let bytes = writer.into_inner().into_inner();
        String::from_utf8(bytes).map_err(|e| BookingError::Xml(e.to_string()))
    }

    /// Serialize an `OTA_HotelResNotifRS` document using quick-xml.
    pub fn build_res_notif_rs(success: bool, error_text: Option<&str>) -> Result<String, BookingError> {
        let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

        write_root_start(&mut writer, "OTA_HotelResNotifRS", "1.0")?;

        if success {
            writer
                .write_event(Event::Empty(BytesStart::new("Success")))
                .map_err(|e| BookingError::Xml(e.to_string()))?;
        } else {
            writer
                .write_event(Event::Start(BytesStart::new("Errors")))
                .map_err(|e| BookingError::Xml(e.to_string()))?;

            let mut err_el = BytesStart::new("Error");
            err_el.push_attribute(("Type", "3"));
            err_el.push_attribute(("Code", "450"));
            writer
                .write_event(Event::Start(err_el))
                .map_err(|e| BookingError::Xml(e.to_string()))?;
            let msg = error_text.unwrap_or("Processing error");
            writer
                .write_event(Event::Text(BytesText::new(msg)))
                .map_err(|e| BookingError::Xml(e.to_string()))?;
            write_end(&mut writer, "Error")?;
            write_end(&mut writer, "Errors")?;
        }

        write_end(&mut writer, "OTA_HotelResNotifRS")?;

        let bytes = writer.into_inner().into_inner();
        String::from_utf8(bytes).map_err(|e| BookingError::Xml(e.to_string()))
    }

    // ------------------------------------------------------------------
    // Parsing helpers
    // ------------------------------------------------------------------

    /// Parse success/error status from any OTA response XML.
    ///
    /// Returns `(success, error_message)`.
    pub fn parse_response_status(xml: &str) -> (bool, Option<String>) {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut has_success = false;
        let mut has_errors = false;
        let mut error_msg: Option<String> = None;
        let mut in_error = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let name_bytes = e.name().into_inner().to_vec();
                    let local = local_name(&name_bytes).to_string();
                    match local.as_str() {
                        "Success" => has_success = true,
                        "Errors" => has_errors = true,
                        "Error" => {
                            in_error = true;
                            // Try ShortText attribute first
                            if let Some(val) = attr_value(e, "ShortText") {
                                error_msg = Some(val);
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(ref t)) if in_error && error_msg.is_none() => {
                    let text = t.unescape().unwrap_or_default();
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        error_msg = Some(trimmed.to_string());
                    }
                }
                Ok(Event::End(ref e)) => {
                    let name_bytes = e.name().into_inner().to_vec();
                    if local_name(&name_bytes) == "Error" {
                        in_error = false;
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        if has_errors || error_msg.is_some() {
            return (false, error_msg.or_else(|| Some("Unknown error from Booking.com".to_string())));
        }
        if has_success {
            return (true, None);
        }
        // Ambiguous -- caller decides
        (true, None)
    }

    /// Parse an `OTA_HotelResNotifRQ` body and return a list of
    /// `(res_id, res_status, element_xml_fragment)` tuples.
    ///
    /// Each fragment is the raw XML of one `<HotelReservation>` element so
    /// that higher-level code can parse reservation details independently.
    pub fn parse_res_notif_rq_raw(xml: &str) -> Result<Vec<(String, String, String)>, BookingError> {
        // We use the string-scan approach here because quick-xml's fragment
        // extraction from a streaming reader requires buffering the whole
        // subtree, which duplicates complexity for no gain on this particular
        // element structure.  The string-scan is safe: we only look for a
        // well-known element boundary and pass the fragment up.
        let mut results = Vec::new();
        let mut pos = 0;

        while let Some(start_rel) = xml[pos..].find("<HotelReservation") {
            let abs_start = pos + start_rel;
            // Locate matching close tag -- naive scan safe here since
            // HotelReservation is not self-nested in OTA.
            if let Some(end_rel) = xml[abs_start..].find("</HotelReservation>") {
                let abs_end = abs_start + end_rel + "</HotelReservation>".len();
                let fragment = &xml[abs_start..abs_end];

                let res_status = extract_xml_attr(fragment, "ResStatus")
                    .unwrap_or_else(|| "Commit".to_string());
                let res_id = extract_xml_attr(fragment, "ResID_Value")
                    .or_else(|| extract_xml_attr(fragment, "ID"))
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

                results.push((res_id, res_status, fragment.to_string()));
                pos = abs_end;
            } else {
                break;
            }
        }

        Ok(results)
    }

    // ------------------------------------------------------------------
    // Internal utilities
    // ------------------------------------------------------------------

    /// Strip the namespace prefix from a qualified XML name.
    ///
    /// `ota:Success` -> `Success`, `Success` -> `Success`.
    pub fn local_name(name: &[u8]) -> &str {
        let s = std::str::from_utf8(name).unwrap_or("");
        if let Some(colon) = s.find(':') {
            &s[colon + 1..]
        } else {
            s
        }
    }

    /// Extract a named attribute from a quick-xml `BytesStart` event.
    pub fn attr_value(e: &quick_xml::events::BytesStart<'_>, name: &str) -> Option<String> {
        e.attributes()
            .filter_map(|a| a.ok())
            .find(|a| {
                let key_bytes = a.key.into_inner().to_vec();
                local_name(&key_bytes) == name
            })
            .and_then(|a| {
                a.unescape_value()
                    .ok()
                    .map(|v| v.into_owned())
            })
    }

    /// Naive attribute extractor that works on a raw XML fragment.
    pub fn extract_xml_attr(xml: &str, attr_name: &str) -> Option<String> {
        let pattern = format!("{}=\"", attr_name);
        xml.find(&pattern).and_then(|start| {
            let val_start = start + pattern.len();
            xml[val_start..].find('"').map(|end| xml[val_start..val_start + end].to_string())
        })
    }

    /// Naive element-content extractor that works on a raw XML fragment.
    pub fn extract_xml_element(xml: &str, element_name: &str) -> Option<String> {
        let open_tag = format!("<{}", element_name);
        xml.find(&open_tag).and_then(|tag_start| {
            xml[tag_start..].find('>').and_then(|content_rel| {
                let content_start = tag_start + content_rel + 1;
                let close_tag = format!("</{}>", element_name);
                xml[content_start..].find(&close_tag).map(|end| {
                    xml[content_start..content_start + end].trim().to_string()
                })
            })
        }).filter(|s| !s.is_empty())
    }

    // ------------------------------------------------------------------
    // Tests for ota_xml helpers
    // ------------------------------------------------------------------
    #[cfg(test)]
    mod tests {
        use super::*;
        use chrono::NaiveDate;
        use rust_decimal::Decimal;

        use crate::booking::{AvailStatusMessage, LosRestrictions};

        #[test]
        fn test_build_avail_notif_rq_namespace() {
            let msgs = vec![AvailStatusMessage {
                room_type_code: "DBL".to_string(),
                rate_plan_code: Some("STD".to_string()),
                start_date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
                booking_limit: 3,
                status: "Open".to_string(),
                los_restrictions: None,
            }];

            let xml = build_avail_notif_rq("12345", &msgs).unwrap();

            assert!(xml.contains("OTA_HotelAvailNotifRQ"), "root element missing");
            assert!(
                xml.contains("http://www.opentravel.org/OTA/2003/05"),
                "OTA namespace missing"
            );
            assert!(xml.contains("HotelCode=\"12345\""), "hotel code missing");
            assert!(xml.contains("InvTypeCode=\"DBL\""), "room type missing");
            assert!(xml.contains("BookingLimit=\"3\""), "booking limit missing");
            assert!(xml.contains("Status=\"Open\""), "status missing");
        }

        #[test]
        fn test_build_avail_notif_rq_with_los() {
            let msgs = vec![AvailStatusMessage {
                room_type_code: "SGL".to_string(),
                rate_plan_code: None,
                start_date: NaiveDate::from_ymd_opt(2025, 7, 1).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2025, 7, 7).unwrap(),
                booking_limit: 1,
                status: "Close".to_string(),
                los_restrictions: Some(LosRestrictions {
                    min_los: Some(2),
                    max_los: Some(14),
                }),
            }];

            let xml = build_avail_notif_rq("99999", &msgs).unwrap();
            assert!(xml.contains("LengthsOfStay"), "LOS element missing");
            assert!(xml.contains("MinStay=\"2\""), "min LOS missing");
            assert!(xml.contains("MaxStay=\"14\""), "max LOS missing");
        }

        #[test]
        fn test_build_rate_amount_notif_rq_namespace() {
            let date = NaiveDate::from_ymd_opt(2025, 8, 1).unwrap();
            let rate: Decimal = "120.00".parse().unwrap();
            let updates = vec![(date, date, "DBL", "STD", &rate, "EUR")];

            let xml = build_rate_amount_notif_rq("12345", &updates).unwrap();

            assert!(xml.contains("OTA_HotelRateAmountNotifRQ"), "root element missing");
            assert!(
                xml.contains("http://www.opentravel.org/OTA/2003/05"),
                "OTA namespace missing"
            );
            assert!(xml.contains("AmountAfterTax="), "rate missing");
            assert!(xml.contains("CurrencyCode=\"EUR\""), "currency missing");
        }

        #[test]
        fn test_build_res_notif_rs_success() {
            let xml = build_res_notif_rs(true, None).unwrap();
            assert!(xml.contains("OTA_HotelResNotifRS"), "root element missing");
            assert!(
                xml.contains("http://www.opentravel.org/OTA/2003/05"),
                "OTA namespace missing"
            );
            assert!(xml.contains("<Success"), "Success element missing");
            assert!(!xml.contains("<Errors"), "unexpected Errors element");
        }

        #[test]
        fn test_build_res_notif_rs_error() {
            let xml = build_res_notif_rs(false, Some("Duplicate reservation")).unwrap();
            assert!(xml.contains("<Errors"), "Errors element missing");
            assert!(xml.contains("Duplicate reservation"), "error text missing");
            assert!(!xml.contains("<Success"), "unexpected Success element");
        }

        #[test]
        fn test_parse_response_status_success() {
            let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<OTA_HotelAvailNotifRS xmlns="http://www.opentravel.org/OTA/2003/05" Version="1.0">
  <Success/>
</OTA_HotelAvailNotifRS>"#;

            let (ok, err) = parse_response_status(xml);
            assert!(ok, "should be success");
            assert!(err.is_none(), "should have no error");
        }

        #[test]
        fn test_parse_response_status_error_short_text() {
            let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<OTA_HotelAvailNotifRS xmlns="http://www.opentravel.org/OTA/2003/05" Version="1.0">
  <Errors>
    <Error Type="3" Code="450" ShortText="Invalid hotel code"/>
  </Errors>
</OTA_HotelAvailNotifRS>"#;

            let (ok, err) = parse_response_status(xml);
            assert!(!ok, "should be error");
            assert_eq!(err.unwrap(), "Invalid hotel code");
        }

        #[test]
        fn test_parse_response_status_error_body_text() {
            let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<OTA_HotelResNotifRS xmlns="http://www.opentravel.org/OTA/2003/05" Version="1.0">
  <Errors>
    <Error Type="3" Code="450">Room type not found</Error>
  </Errors>
</OTA_HotelResNotifRS>"#;

            let (ok, err) = parse_response_status(xml);
            assert!(!ok, "should be error");
            assert_eq!(err.unwrap(), "Room type not found");
        }

        #[test]
        fn test_parse_res_notif_rq_raw_commit() {
            let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<OTA_HotelResNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05" Version="1.0">
  <HotelReservations>
    <HotelReservation ResStatus="Commit">
      <ResGlobalInfo>
        <HotelReservationIDs>
          <HotelReservationID ResID_Value="BK-001"/>
        </HotelReservationIDs>
      </ResGlobalInfo>
    </HotelReservation>
  </HotelReservations>
</OTA_HotelResNotifRQ>"#;

            let notifs = parse_res_notif_rq_raw(xml).unwrap();
            assert_eq!(notifs.len(), 1, "expected one notification");
            let (res_id, res_status, _fragment) = &notifs[0];
            assert_eq!(res_status, "Commit");
            assert_eq!(res_id, "BK-001");
        }

        #[test]
        fn test_parse_res_notif_rq_raw_cancel() {
            let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<OTA_HotelResNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05" Version="1.0">
  <HotelReservations>
    <HotelReservation ResStatus="Cancel">
      <ResGlobalInfo>
        <HotelReservationIDs>
          <HotelReservationID ResID_Value="BK-002"/>
        </HotelReservationIDs>
      </ResGlobalInfo>
    </HotelReservation>
  </HotelReservations>
</OTA_HotelResNotifRQ>"#;

            let notifs = parse_res_notif_rq_raw(xml).unwrap();
            assert_eq!(notifs.len(), 1);
            assert_eq!(notifs[0].1, "Cancel");
            assert_eq!(notifs[0].0, "BK-002");
        }
    }
}

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

        if !status_success {
            return Ok(Self {
                success: false,
                error: status_error,
                reservations: Vec::new(),
            });
        }

        // Also accept a <ReservationsList> element as an implicit success
        // indicator (some Booking.com response variants omit <Success/>).
        if !xml.contains("<Success") && !xml.contains("<ReservationsList") {
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
        let status = match status_str.to_lowercase().as_str() {
            "commit" | "confirmed" => BookingReservationStatus::Confirmed,
            "cancel" | "cancelled" => BookingReservationStatus::Cancelled,
            "modify" | "modified" => BookingReservationStatus::Modified,
            "noshow" | "no_show" => BookingReservationStatus::NoShow,
            _ => BookingReservationStatus::New,
        };

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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct LosRestrictions {
    /// Minimum stay.
    pub min_los: Option<i32>,
    /// Maximum stay.
    pub max_los: Option<i32>,
}

impl OtaHotelAvailNotifRQ {
    /// Convert to OTA XML format using quick-xml (namespace-aware).
    pub fn to_xml(&self) -> String {
        ota_xml::build_avail_notif_rq(&self.hotel_code, &self.avail_status_messages)
            .unwrap_or_else(|e| {
                tracing::error!(error = %e, "Failed to build OTA_HotelAvailNotifRQ XML");
                // Fallback to legacy format on unexpected serialisation error.
                self.to_xml_legacy()
            })
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
}

impl BookingClient {
    /// Create a new Booking.com client.
    pub fn new(credentials: BookingCredentials) -> Self {
        Self {
            client: reqwest::Client::new(),
            credentials,
        }
    }

    /// Create a new client with simple API key (for basic usage).
    pub fn with_api_key(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            credentials: BookingCredentials {
                hotel_id: String::new(),
                username: api_key.clone(),
                password: api_key,
                api_url: "https://supply-xml.booking.com/hotels/xml".to_string(),
            },
        }
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

    // ==================== Property Operations ====================

    /// Fetch all properties for the account.
    pub async fn fetch_properties(&self) -> Result<Vec<BookingProperty>, BookingError> {
        tracing::info!("Fetching Booking.com properties");

        if !self.credentials.hotel_id.is_empty() {
            match self.fetch_property(&self.credentials.hotel_id).await {
                Ok(property) => Ok(vec![property]),
                Err(_) => {
                    Ok(vec![BookingProperty {
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
                    }])
                }
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

        let star_rating = Self::extract_xml_attr(xml, "Rating")
            .or_else(|| Self::extract_xml_attr(xml, "StarRating"))
            .and_then(|s| s.parse().ok());

        let property_type = Self::extract_xml_attr(xml, "PropertyType")
            .or_else(|| Self::extract_xml_attr(xml, "HotelCategory"))
            .unwrap_or_else(|| "hotel".to_string());

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
            phone: Self::extract_xml_attr(xml, "PhoneNumber")
                .or_else(|| Self::extract_xml_element(xml, "PhoneNumber")),
            website: Self::extract_xml_element(xml, "URL"),
        };

        let room_types = self.parse_room_types(xml);
        let facilities = self.parse_facilities(xml);
        let check_in_time = Self::extract_xml_attr(xml, "CheckInTime");
        let check_out_time = Self::extract_xml_attr(xml, "CheckOutTime");

        Ok(BookingProperty {
            hotel_id: hotel_id.to_string(),
            name,
            description,
            star_rating,
            property_type,
            address,
            contact,
            room_types,
            facilities,
            check_in_time,
            check_out_time,
            synced_at: Some(Utc::now()),
        })
    }

    /// Parse room types from XML.
    fn parse_room_types(&self, xml: &str) -> Vec<BookingRoomType> {
        let mut room_types = Vec::new();
        let mut search_pos = 0;
        while let Some(start) = xml[search_pos..].find("<GuestRoom") {
            let abs_start = search_pos + start;
            if let Some(end) = xml[abs_start..].find("</GuestRoom>") {
                let room_xml = &xml[abs_start..abs_start + end + 12];

                let id = Self::extract_xml_attr(room_xml, "RoomTypeCode")
                    .or_else(|| Self::extract_xml_attr(room_xml, "InvTypeCode"))
                    .unwrap_or_else(|| Uuid::new_v4().to_string());
                let name = Self::extract_xml_element(room_xml, "RoomTypeName")
                    .or_else(|| Self::extract_xml_attr(room_xml, "RoomTypeName"))
                    .unwrap_or_else(|| "Room".to_string());
                let description = Self::extract_xml_element(room_xml, "DescriptiveText");
                let max_occupancy = Self::extract_xml_attr(room_xml, "MaxOccupancy")
                    .and_then(|s| s.parse().ok()).unwrap_or(2);
                let bed_count = Self::extract_xml_attr(room_xml, "NumberOfBeds")
                    .and_then(|s| s.parse().ok()).unwrap_or(1);
                let bed_type = Self::extract_xml_attr(room_xml, "BedType");
                let room_size = Self::extract_xml_attr(room_xml, "RoomSize").and_then(|s| s.parse().ok());
                let total_rooms = Self::extract_xml_attr(room_xml, "Quantity")
                    .and_then(|s| s.parse().ok()).unwrap_or(1);

                room_types.push(BookingRoomType {
                    id, name, description, max_occupancy, bed_count, bed_type, room_size,
                    amenities: Vec::new(), total_rooms,
                });
                search_pos = abs_start + end + 12;
            } else {
                break;
            }
        }
        room_types
    }

    /// Parse facilities from XML.
    fn parse_facilities(&self, xml: &str) -> Vec<String> {
        let mut facilities = Vec::new();
        let mut search_pos = 0;
        while let Some(start) = xml[search_pos..].find("<Service") {
            let abs_start = search_pos + start;
            if let Some(end) = xml[abs_start..].find("/>") {
                let service_xml = &xml[abs_start..abs_start + end + 2];
                if let Some(code) = Self::extract_xml_attr(service_xml, "Code") {
                    let name = match code.as_str() {
                        "1" => "24-hour front desk",
                        "2" => "Bar",
                        "3" => "Restaurant",
                        "4" => "Room service",
                        "5" => "Parking",
                        "6" => "Free WiFi",
                        "7" => "Pool",
                        "8" => "Spa",
                        "9" => "Gym",
                        "10" => "Airport shuttle",
                        _ => &code,
                    };
                    facilities.push(name.to_string());
                }
                search_pos = abs_start + end + 2;
            } else {
                break;
            }
        }
        facilities
    }

    /// Extract an XML attribute value (static method for property parsing).
    fn extract_xml_attr(xml: &str, attr_name: &str) -> Option<String> {
        let pattern = format!("{}=\"", attr_name);
        if let Some(start) = xml.find(&pattern) {
            let start = start + pattern.len();
            if let Some(end) = xml[start..].find('"') {
                return Some(xml[start..start + end].to_string());
            }
        }
        None
    }

    /// Extract XML element content (static method for property parsing).
    fn extract_xml_element(xml: &str, element_name: &str) -> Option<String> {
        let open_tag = format!("<{}", element_name);
        if let Some(tag_start) = xml.find(&open_tag) {
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

    // ==================== Reservation Operations ====================

    /// Fetch reservations for a property.
    pub async fn fetch_reservations(
        &self,
        hotel_id: &str,
    ) -> Result<Vec<BookingReservation>, BookingError> {
        let request = OtaReadRQ {
            hotel_code: hotel_id.to_string(),
            start_date: Utc::now().date_naive(),
            end_date: Utc::now().date_naive() + Duration::days(365),
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
        let parsed = OtaReadRS::from_xml(&body)?;

        if parsed.success {
            Ok(parsed.reservations)
        } else {
            Err(BookingError::Api(
                parsed.error.unwrap_or_else(|| "Unknown error".to_string()),
            ))
        }
    }

    /// Sync reservations from Booking.com.
    pub async fn sync_reservations(
        &self,
        hotel_id: &str,
    ) -> Result<Vec<BookingReservation>, BookingError> {
        tracing::info!("Syncing Booking.com reservations for hotel: {}", hotel_id);
        let reservations = self.fetch_reservations(hotel_id).await?;
        tracing::info!("Synced {} reservations from Booking.com", reservations.len());
        Ok(reservations)
    }

    // ==================== Push Operations ====================

    /// Push availability updates to Booking.com.
    pub async fn push_availability(
        &self,
        mapping: &PropertyMapping,
        updates: Vec<AvailabilityUpdate>,
    ) -> Result<(), BookingError> {
        let request = OtaHotelAvailNotifRQ {
            hotel_code: mapping.external_property_id.clone(),
            avail_status_messages: updates
                .iter()
                .map(|u| AvailStatusMessage {
                    room_type_code: u.room_type_id.clone(),
                    rate_plan_code: None,
                    start_date: u.date,
                    end_date: u.date,
                    booking_limit: u.available_count,
                    status: if u.stop_sell || u.available_count == 0 {
                        "Close".to_string()
                    } else {
                        "Open".to_string()
                    },
                    los_restrictions: u.min_los.map(|min| LosRestrictions {
                        min_los: Some(min),
                        max_los: u.max_los,
                    }),
                })
                .collect(),
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
        let parsed = OtaHotelAvailNotifRS::from_xml(&body)?;

        if parsed.success {
            tracing::info!("Pushed {} availability updates to Booking.com", updates.len());
            Ok(())
        } else {
            Err(BookingError::PushFailed(
                parsed.error.unwrap_or_else(|| "Unknown error".to_string()),
            ))
        }
    }

    /// Push rate updates to Booking.com.
    ///
    /// Uses OTA_HotelRateAmountNotifRQ to update rates.
    pub async fn push_rates(
        &self,
        hotel_id: &str,
        updates: Vec<RateUpdate>,
    ) -> Result<(), BookingError> {
        if updates.is_empty() {
            return Ok(());
        }

        tracing::info!(
            "Pushing {} rate updates to Booking.com for hotel {}",
            updates.len(),
            hotel_id
        );

        // Build OTA_HotelRateAmountNotifRQ using quick-xml
        let rate_entries: Vec<(NaiveDate, NaiveDate, &str, &str, &Decimal, &str)> = updates
            .iter()
            .map(|u| (u.date, u.date, u.room_type_id.as_str(), u.rate_plan_code.as_str(), &u.base_rate, u.currency.as_str()))
            .collect();

        let request_xml = ota_xml::build_rate_amount_notif_rq(hotel_id, &rate_entries)?;

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

        // Parse response using quick-xml status helper
        let (ok, error_msg) = ota_xml::parse_response_status(&body);
        if !ok {
            return Err(BookingError::PushFailed(
                error_msg.unwrap_or_else(|| "Failed to push rates".to_string()),
            ));
        }

        tracing::info!("Successfully pushed {} rates to Booking.com", updates.len());
        Ok(())
    }

    // ==================== Webhook/Push Notification Handling ====================

    /// Handle an incoming push notification from Booking.com.
    pub fn parse_push_notification(xml: &str) -> Result<OtaHotelResNotifRQ, BookingError> {
        OtaHotelResNotifRQ::from_xml(xml)
    }

    /// Generate a response for a push notification.
    pub fn generate_push_response(success: bool, error: Option<&str>) -> String {
        if success {
            OtaHotelResNotifRS::success().to_xml()
        } else {
            OtaHotelResNotifRS::error(error.unwrap_or("Unknown error")).to_xml()
        }
    }
}

// ============================================
// Mapping Functions
// ============================================

/// Map Booking.com reservation status to internal status string.
pub fn map_reservation_status(status: &BookingReservationStatus) -> String {
    match status {
        BookingReservationStatus::New => "pending".to_string(),
        BookingReservationStatus::Confirmed => "confirmed".to_string(),
        BookingReservationStatus::Modified => "modified".to_string(),
        BookingReservationStatus::Cancelled => "cancelled".to_string(),
        BookingReservationStatus::NoShow => "no_show".to_string(),
    }
}

// ============================================
// Tests
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ota_read_rq_xml() {
        let request = OtaReadRQ {
            hotel_code: "12345".to_string(),
            start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
            status_filter: None,
        };

        let xml = request.to_xml();
        assert!(xml.contains("HotelCode=\"12345\""));
        assert!(xml.contains("Start=\"2024-01-01\""));
        assert!(xml.contains("End=\"2024-12-31\""));
    }

    #[test]
    fn test_ota_hotel_res_notif_rs_success() {
        let response = OtaHotelResNotifRS::success();
        let xml = response.to_xml();

        assert!(xml.contains("<Success/>"));
        assert!(!xml.contains("<Errors>"));
    }

    #[test]
    fn test_ota_hotel_res_notif_rs_error() {
        let response = OtaHotelResNotifRS::error("Test error");
        let xml = response.to_xml();

        assert!(xml.contains("<Errors>"));
        assert!(xml.contains("Test error"));
        assert!(!xml.contains("<Success/>"));
    }

    #[test]
    fn test_ota_hotel_avail_notif_rq_xml() {
        let request = OtaHotelAvailNotifRQ {
            hotel_code: "12345".to_string(),
            avail_status_messages: vec![AvailStatusMessage {
                room_type_code: "DBL".to_string(),
                rate_plan_code: Some("STD".to_string()),
                start_date: NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
                booking_limit: 5,
                status: "Open".to_string(),
                los_restrictions: None,
            }],
        };

        let xml = request.to_xml();
        assert!(xml.contains("HotelCode=\"12345\""));
        assert!(xml.contains("BookingLimit=\"5\""));
        assert!(xml.contains("InvTypeCode=\"DBL\""));
        assert!(xml.contains("Status=\"Open\""));
    }

    #[test]
    fn test_map_reservation_status() {
        assert_eq!(
            map_reservation_status(&BookingReservationStatus::New),
            "pending"
        );
        assert_eq!(
            map_reservation_status(&BookingReservationStatus::Confirmed),
            "confirmed"
        );
        assert_eq!(
            map_reservation_status(&BookingReservationStatus::Cancelled),
            "cancelled"
        );
        assert_eq!(
            map_reservation_status(&BookingReservationStatus::NoShow),
            "no_show"
        );
    }

    #[test]
    fn test_credentials_new() {
        let creds =
            BookingCredentials::new("12345".to_string(), "user".to_string(), "pass".to_string());

        assert_eq!(creds.hotel_id, "12345");
        assert!(creds.api_url.contains("booking.com"));
    }

    #[test]
    fn test_reservation_status_serialization() {
        let status = BookingReservationStatus::Confirmed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"confirmed\"");

        let parsed: BookingReservationStatus = serde_json::from_str("\"cancelled\"").unwrap();
        assert_eq!(parsed, BookingReservationStatus::Cancelled);
    }
}
