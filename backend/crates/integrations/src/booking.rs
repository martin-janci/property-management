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

pub mod ota_xml {
    //! Low-level OTA XML serialization/deserialization helpers backed by
    //! [`quick_xml`].
    //!
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
        write_root_start_with_echo_token(writer, tag, version, None)
    }

    /// Like [`write_root_start`] but additionally stamps an OTA `EchoToken`
    /// attribute on the root element when `echo_token` is `Some`.
    ///
    /// The `EchoToken` is the OTA-standard idempotency / correlation key: the
    /// supplier (Booking.com) echoes it back on the response and uses it to
    /// deduplicate retried requests, so the same logical push that is re-sent
    /// (e.g. by [`BookingClient::post_ota_with_retry`] after a transient 5xx)
    /// must carry the *same* token to be recognised as a duplicate rather than
    /// applied twice.
    pub fn write_root_start_with_echo_token(
        writer: &mut Writer<Cursor<Vec<u8>>>,
        tag: &str,
        version: &str,
        echo_token: Option<&str>,
    ) -> Result<(), BookingError> {
        // <?xml version="1.0" encoding="UTF-8"?>
        writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
            .map_err(|e| BookingError::Xml(e.to_string()))?;

        let mut start = BytesStart::new(tag);
        start.push_attribute(("xmlns", OTA_NAMESPACE));
        start.push_attribute(("Version", version));
        if let Some(token) = echo_token {
            start.push_attribute(("EchoToken", token));
        }
        writer
            .write_event(Event::Start(start))
            .map_err(|e| BookingError::Xml(e.to_string()))?;

        Ok(())
    }

    /// Compute a deterministic OTA `EchoToken` (idempotency key) for a push.
    ///
    /// The token is a stable, content-derived digest: identical logical
    /// payloads (same hotel + same ordered set of update lines) always produce
    /// the same token, while any change to the payload produces a different
    /// one. This gives the upstream a key to deduplicate idempotent retries on
    /// while still distinguishing genuinely different pushes.
    ///
    /// `kind` namespaces the digest per message type so an availability push and
    /// a rate push with coincidentally-equal canonical bodies never collide.
    pub fn compute_echo_token(kind: &str, canonical: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(kind.as_bytes());
        hasher.update(b"\x1f");
        hasher.update(canonical.as_bytes());
        let digest = hasher.finalize();
        // 128-bit hex prefix is ample for dedup keying and keeps the attribute
        // short in the on-wire XML.
        let mut out = String::with_capacity(32);
        for byte in &digest[..16] {
            use std::fmt::Write as _;
            let _ = write!(out, "{byte:02x}");
        }
        out
    }

    /// Write a closing tag for `tag`.
    pub fn write_end(writer: &mut Writer<Cursor<Vec<u8>>>, tag: &str) -> Result<(), BookingError> {
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

        // Deterministic idempotency key: stamp an OTA EchoToken derived from the
        // canonical (hotel + ordered messages) payload so retries of the same
        // push carry the same token and the upstream can deduplicate them.
        let mut canonical = String::new();
        canonical.push_str(hotel_code);
        for msg in messages {
            use std::fmt::Write as _;
            let _ = write!(
                canonical,
                "|{}:{}:{}:{}:{}:{}:{:?}:{:?}",
                msg.room_type_code,
                msg.rate_plan_code.as_deref().unwrap_or("STD"),
                msg.start_date,
                msg.end_date,
                msg.booking_limit,
                msg.status,
                msg.los_restrictions.as_ref().and_then(|l| l.min_los),
                msg.los_restrictions.as_ref().and_then(|l| l.max_los),
            );
        }
        let echo_token = compute_echo_token("avail", &canonical);

        write_root_start_with_echo_token(
            &mut writer,
            "OTA_HotelAvailNotifRQ",
            "1.0",
            Some(&echo_token),
        )?;

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

        // Deterministic idempotency key (see build_avail_notif_rq): stamp an OTA
        // EchoToken derived from the canonical payload so retries are dedupable.
        let mut canonical = String::new();
        canonical.push_str(hotel_id);
        for (start, end, room_type, rate_plan, rate, currency) in updates {
            use std::fmt::Write as _;
            let _ = write!(
                canonical,
                "|{start}:{end}:{room_type}:{rate_plan}:{rate}:{currency}"
            );
        }
        let echo_token = compute_echo_token("rate", &canonical);

        write_root_start_with_echo_token(
            &mut writer,
            "OTA_HotelRateAmountNotifRQ",
            "1.0",
            Some(&echo_token),
        )?;

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
    pub fn build_res_notif_rs(
        success: bool,
        error_text: Option<&str>,
    ) -> Result<String, BookingError> {
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
                    // quick-xml 0.40 removed `BytesText::unescape`; decode the
                    // charset then resolve XML entities via the free function
                    // (same decode+unescape semantics the old method provided).
                    let decoded = t.decode().unwrap_or_default();
                    let text = quick_xml::escape::unescape(&decoded)
                        .map(|c| c.into_owned())
                        .unwrap_or_else(|_| decoded.into_owned());
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
            return (
                false,
                error_msg.or_else(|| Some("Unknown error from Booking.com".to_string())),
            );
        }
        if has_success {
            return (true, None);
        }
        // Indeterminate: neither <Success> nor <Errors> was seen (malformed,
        // truncated, or an unexpected response shape). Do NOT silently treat
        // this as success -- that would mask a failed push. Surface it as a
        // non-success indeterminate state so callers can react.
        (
            false,
            Some(
                "Indeterminate Booking.com response: no <Success> or <Errors> element".to_string(),
            ),
        )
    }

    /// Parse an `OTA_HotelResNotifRQ` body and return a list of
    /// `(res_id, res_status, element_xml_fragment)` tuples.
    ///
    /// Each fragment is the raw XML of one `<HotelReservation>` element so
    /// that higher-level code can parse reservation details independently.
    pub fn parse_res_notif_rq_raw(
        xml: &str,
    ) -> Result<Vec<(String, String, String)>, BookingError> {
        // Element boundaries are located with a namespace-safe scan that only
        // matches the *element name* (so a hypothetical `<HotelReservationFoo>`
        // can never be mistaken for `<HotelReservation>`). The `ResStatus`
        // attribute is then read from the `<HotelReservation>` *start-tag only*
        // via the streaming reader, so a nested child element carrying a
        // similarly named attribute can never shadow it.
        let mut results = Vec::new();
        let mut pos = 0;

        while let Some(start_rel) = find_element_start(&xml[pos..], "HotelReservation") {
            let abs_start = pos + start_rel;
            // Locate matching close tag -- naive scan safe here since
            // HotelReservation is not self-nested in OTA.
            if let Some(end_rel) = xml[abs_start..].find("</HotelReservation>") {
                let abs_end = abs_start + end_rel + "</HotelReservation>".len();
                let fragment = &xml[abs_start..abs_end];

                // ResStatus lives on the HotelReservation start-tag itself.
                // Parse the start-tag attributes precisely rather than scanning
                // the whole fragment for `ResStatus="`.
                let res_status =
                    start_tag_attr(fragment, "ResStatus").unwrap_or_else(|| "Commit".to_string());
                // ResID_Value lives on a nested <HotelReservationID> element;
                // a word-boundary-aware fragment scan is correct here.
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

    /// Find the byte offset of the start-tag of an element named exactly
    /// `name` (namespace prefixes allowed), ensuring the match ends on a tag
    /// boundary (whitespace, `>` or `/`) so `<HotelReservationFoo>` is not
    /// mistaken for `<HotelReservation>`.
    fn find_element_start(xml: &str, name: &str) -> Option<usize> {
        let mut search_from = 0;
        while let Some(rel) = xml[search_from..].find('<') {
            let lt = search_from + rel;
            let after = &xml[lt + 1..];
            // Skip closing tags / comments / declarations.
            let after = after.strip_prefix('/').map(|_| "").unwrap_or(after);
            // Allow an optional `prefix:` before the element name.
            let candidate = after.rsplit(':').next().unwrap_or(after);
            if let Some(rest) = candidate.strip_prefix(name) {
                let boundary = rest
                    .chars()
                    .next()
                    .map(|c| c.is_whitespace() || c == '>' || c == '/')
                    .unwrap_or(false);
                // Ensure the `prefix:` (if any) sits immediately after `<`.
                let prefix_ok = after == candidate || after.ends_with(candidate);
                if boundary && prefix_ok {
                    return Some(lt);
                }
            }
            search_from = lt + 1;
        }
        None
    }

    /// Read an attribute from the *start-tag* of an XML fragment via the
    /// streaming reader, so nested elements cannot shadow the value.
    fn start_tag_attr(fragment: &str, attr: &str) -> Option<String> {
        let mut reader = Reader::from_str(fragment);
        reader.config_mut().trim_text(true);
        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    return attr_value(e, attr);
                }
                Ok(Event::Eof) => return None,
                Err(_) => return None,
                _ => {}
            }
        }
    }

    /// A single availability instruction parsed from an inbound
    /// `OTA_HotelAvailNotifRQ` document.
    ///
    /// This is the inbound counterpart to [`build_avail_notif_rq`]: it reads a
    /// channel-manager / partner push and reconstructs the typed
    /// [`AvailStatusMessage`] values so the application can apply them. The
    /// `hotel_code` is read from the enclosing `<AvailStatusMessages>` element.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ParsedAvailNotif {
        /// Hotel code from the `<AvailStatusMessages HotelCode="...">` wrapper.
        pub hotel_code: String,
        /// The availability instructions in document order.
        pub messages: Vec<AvailStatusMessage>,
    }

    /// A single rate instruction parsed from an inbound
    /// `OTA_HotelRateAmountNotifRQ` document.
    ///
    /// Inbound counterpart to [`build_rate_amount_notif_rq`].
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ParsedRateAmount {
        /// Room type / inventory code (`InvTypeCode`).
        pub room_type_code: String,
        /// Rate plan code (`RatePlanCode`); `None` when the tag omits it.
        pub rate_plan_code: Option<String>,
        /// Effective start date (`Start`).
        pub start_date: NaiveDate,
        /// Effective end date (`End`).
        pub end_date: NaiveDate,
        /// Amount after tax (`AmountAfterTax`).
        pub amount: Decimal,
        /// ISO-4217 currency code (`CurrencyCode`).
        pub currency: String,
    }

    /// A `<RateAmountMessage>` list parsed from an inbound
    /// `OTA_HotelRateAmountNotifRQ` document.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ParsedRateNotif {
        /// Hotel code from the `<RateAmountMessages HotelCode="...">` wrapper.
        pub hotel_code: String,
        /// The rate instructions in document order.
        pub rates: Vec<ParsedRateAmount>,
    }

    /// Parse an inbound `OTA_HotelAvailNotifRQ` document into typed
    /// [`AvailStatusMessage`] values.
    ///
    /// This is the streaming, namespace-aware inverse of
    /// [`build_avail_notif_rq`]. Each `<AvailStatusMessage>` contributes one
    /// [`AvailStatusMessage`]:
    /// - `BookingLimit` attribute → `booking_limit` (defaults to `0`).
    /// - `<StatusApplicationControl>` → `start_date`/`end_date`/`room_type_code`/
    ///   `rate_plan_code`. A missing/blank `RatePlanCode` parses to `None`.
    /// - `<RestrictionStatus Status="...">` → `status` (defaults to `"Open"`).
    /// - `<LengthsOfStay MinStay=".." MaxStay="..">` → `los_restrictions`
    ///   (only present when at least one of the bounds is given).
    ///
    /// Returns [`BookingError::InvalidMessage`] when a `Start`/`End` date is
    /// present but not a valid `YYYY-MM-DD` value.
    pub fn parse_avail_notif_rq(xml: &str) -> Result<ParsedAvailNotif, BookingError> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut hotel_code = String::new();
        let mut messages = Vec::new();

        // Fields accumulated for the message currently being built.
        let mut cur: Option<PartialAvail> = None;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let name = e.name().into_inner().to_vec();
                    match local_name(&name) {
                        "AvailStatusMessages" => {
                            if let Some(v) = attr_value(e, "HotelCode") {
                                hotel_code = v;
                            }
                        }
                        "AvailStatusMessage" => {
                            cur = Some(PartialAvail {
                                booking_limit: attr_value(e, "BookingLimit")
                                    .and_then(|s| s.parse().ok())
                                    .unwrap_or(0),
                                ..PartialAvail::default()
                            });
                        }
                        "StatusApplicationControl" => {
                            if let Some(c) = cur.as_mut() {
                                c.start_date = Some(parse_ota_date(e, "Start")?);
                                c.end_date = Some(parse_ota_date(e, "End")?);
                                c.room_type_code = attr_value(e, "InvTypeCode").unwrap_or_default();
                                c.rate_plan_code =
                                    attr_value(e, "RatePlanCode").filter(|s| !s.is_empty());
                            }
                        }
                        "RestrictionStatus" => {
                            if let Some(c) = cur.as_mut() {
                                if let Some(s) = attr_value(e, "Status") {
                                    c.status = s;
                                }
                            }
                        }
                        "LengthsOfStay" => {
                            if let Some(c) = cur.as_mut() {
                                let min = attr_value(e, "MinStay").and_then(|s| s.parse().ok());
                                let max = attr_value(e, "MaxStay").and_then(|s| s.parse().ok());
                                if min.is_some() || max.is_some() {
                                    c.los = Some(super::LosRestrictions {
                                        min_los: min,
                                        max_los: max,
                                    });
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    let name = e.name().into_inner().to_vec();
                    if local_name(&name) == "AvailStatusMessage" {
                        if let Some(c) = cur.take() {
                            messages.push(c.into_message());
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(BookingError::Xml(e.to_string())),
                _ => {}
            }
        }

        Ok(ParsedAvailNotif {
            hotel_code,
            messages,
        })
    }

    /// Parse an inbound `OTA_HotelRateAmountNotifRQ` document into typed
    /// [`ParsedRateAmount`] values.
    ///
    /// Streaming, namespace-aware inverse of [`build_rate_amount_notif_rq`].
    /// Each `<RateAmountMessage>` contributes one [`ParsedRateAmount`] built
    /// from its `<StatusApplicationControl>` (dates + codes) and the first
    /// `<BaseByGuestAmt>` (`AmountAfterTax` + `CurrencyCode`).
    ///
    /// Returns [`BookingError::InvalidMessage`] when a `Start`/`End` date is
    /// malformed, or when `AmountAfterTax` is present but not a valid decimal.
    pub fn parse_rate_amount_notif_rq(xml: &str) -> Result<ParsedRateNotif, BookingError> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut hotel_code = String::new();
        let mut rates = Vec::new();
        let mut cur: Option<PartialRate> = None;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let name = e.name().into_inner().to_vec();
                    match local_name(&name) {
                        "RateAmountMessages" => {
                            if let Some(v) = attr_value(e, "HotelCode") {
                                hotel_code = v;
                            }
                        }
                        "RateAmountMessage" => {
                            cur = Some(PartialRate::default());
                        }
                        "StatusApplicationControl" => {
                            if let Some(c) = cur.as_mut() {
                                c.start_date = Some(parse_ota_date(e, "Start")?);
                                c.end_date = Some(parse_ota_date(e, "End")?);
                                c.room_type_code = attr_value(e, "InvTypeCode").unwrap_or_default();
                                c.rate_plan_code =
                                    attr_value(e, "RatePlanCode").filter(|s| !s.is_empty());
                            }
                        }
                        "BaseByGuestAmt" => {
                            if let Some(c) = cur.as_mut() {
                                // Only the first BaseByGuestAmt seeds the amount.
                                if c.amount.is_none() {
                                    if let Some(raw) = attr_value(e, "AmountAfterTax")
                                        .or_else(|| attr_value(e, "AmountBeforeTax"))
                                    {
                                        let amt = raw.parse::<Decimal>().map_err(|_| {
                                            BookingError::InvalidMessage(format!(
                                                "invalid rate amount: {raw}"
                                            ))
                                        })?;
                                        c.amount = Some(amt);
                                    }
                                    if let Some(cc) = attr_value(e, "CurrencyCode") {
                                        c.currency = cc;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    let name = e.name().into_inner().to_vec();
                    if local_name(&name) == "RateAmountMessage" {
                        if let Some(c) = cur.take() {
                            rates.push(c.into_rate());
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(BookingError::Xml(e.to_string())),
                _ => {}
            }
        }

        Ok(ParsedRateNotif { hotel_code, rates })
    }

    /// Read an OTA date attribute (`YYYY-MM-DD`) from a start/empty tag.
    ///
    /// A missing attribute yields the Unix epoch (`1970-01-01`) so partial
    /// documents still parse; a *present but malformed* value is a hard error.
    fn parse_ota_date(
        e: &quick_xml::events::BytesStart<'_>,
        attr: &str,
    ) -> Result<NaiveDate, BookingError> {
        match attr_value(e, attr) {
            Some(s) => NaiveDate::parse_from_str(&s, "%Y-%m-%d").map_err(|_| {
                BookingError::InvalidMessage(format!("invalid OTA date in {attr}: {s}"))
            }),
            None => Ok(NaiveDate::from_ymd_opt(1970, 1, 1).expect("epoch is a valid date")),
        }
    }

    /// Mutable accumulator for one `<AvailStatusMessage>` while streaming.
    #[derive(Default)]
    struct PartialAvail {
        booking_limit: i32,
        room_type_code: String,
        rate_plan_code: Option<String>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        status: String,
        los: Option<super::LosRestrictions>,
    }

    impl PartialAvail {
        fn into_message(self) -> AvailStatusMessage {
            let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).expect("epoch is a valid date");
            AvailStatusMessage {
                room_type_code: self.room_type_code,
                rate_plan_code: self.rate_plan_code,
                start_date: self.start_date.unwrap_or(epoch),
                end_date: self.end_date.unwrap_or(epoch),
                booking_limit: self.booking_limit,
                status: if self.status.is_empty() {
                    "Open".to_string()
                } else {
                    self.status
                },
                los_restrictions: self.los,
            }
        }
    }

    /// Mutable accumulator for one `<RateAmountMessage>` while streaming.
    #[derive(Default)]
    struct PartialRate {
        room_type_code: String,
        rate_plan_code: Option<String>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        amount: Option<Decimal>,
        currency: String,
    }

    impl PartialRate {
        fn into_rate(self) -> ParsedRateAmount {
            let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).expect("epoch is a valid date");
            ParsedRateAmount {
                room_type_code: self.room_type_code,
                rate_plan_code: self.rate_plan_code,
                start_date: self.start_date.unwrap_or(epoch),
                end_date: self.end_date.unwrap_or(epoch),
                amount: self.amount.unwrap_or(Decimal::ZERO),
                currency: if self.currency.is_empty() {
                    "EUR".to_string()
                } else {
                    self.currency
                },
            }
        }
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
            // quick-xml 0.40 deprecated `unescape_value`; `normalized_value`
            // performs XML attribute-value normalization (entity resolution)
            // on the UTF-8 value, matching the old decode+unescape behavior.
            .and_then(|a| {
                a.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                    .ok()
                    .map(|v| v.into_owned())
            })
    }

    /// Word-boundary-aware attribute extractor for a raw XML fragment.
    ///
    /// Matches `attr_name="..."` only when the attribute name is not a suffix
    /// of a longer name -- i.e. the character preceding the match must be a
    /// tag/attribute boundary (whitespace, `<`, `/` or `:`). This prevents
    /// `XResStatus="..."` from being mis-read as `ResStatus="..."`.
    pub fn extract_xml_attr(xml: &str, attr_name: &str) -> Option<String> {
        let pattern = format!("{}=\"", attr_name);
        let mut from = 0;
        while let Some(rel) = xml[from..].find(&pattern) {
            let start = from + rel;
            let prev_ok = start == 0
                || xml[..start]
                    .chars()
                    .next_back()
                    .map(|c| c.is_whitespace() || c == '<' || c == '/' || c == ':')
                    .unwrap_or(true);
            if prev_ok {
                let val_start = start + pattern.len();
                return xml[val_start..]
                    .find('"')
                    .map(|end| xml[val_start..val_start + end].to_string());
            }
            from = start + pattern.len();
        }
        None
    }

    /// Naive element-content extractor that works on a raw XML fragment.
    pub fn extract_xml_element(xml: &str, element_name: &str) -> Option<String> {
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

            assert!(
                xml.contains("OTA_HotelAvailNotifRQ"),
                "root element missing"
            );
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

            assert!(
                xml.contains("OTA_HotelRateAmountNotifRQ"),
                "root element missing"
            );
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

        #[test]
        fn test_parse_response_status_indeterminate_is_not_success() {
            // No <Success> and no <Errors> -- must NOT be reported as success.
            let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<OTA_HotelAvailNotifRS xmlns="http://www.opentravel.org/OTA/2003/05" Version="1.0">
</OTA_HotelAvailNotifRS>"#;
            let (ok, err) = parse_response_status(xml);
            assert!(!ok, "indeterminate response must not be success");
            assert!(
                err.is_some(),
                "indeterminate response should carry a message"
            );
        }

        #[test]
        fn test_parse_response_status_truncated_is_not_success() {
            // Truncated / malformed XML must surface as non-success, not OK.
            let xml =
                r#"<OTA_HotelAvailNotifRS xmlns="http://www.opentravel.org/OTA/2003/05"><Succ"#;
            let (ok, _err) = parse_response_status(xml);
            assert!(!ok, "truncated response must not be success");
        }

        #[test]
        fn test_extract_xml_attr_word_boundary() {
            // A longer attribute name ending in the target must not match.
            let frag = r#"<HotelReservation XResStatus="Bogus" ResStatus="Commit"/>"#;
            assert_eq!(
                extract_xml_attr(frag, "ResStatus").as_deref(),
                Some("Commit")
            );
        }

        #[test]
        fn test_res_status_not_shadowed_by_nested_element() {
            // ResStatus on the start-tag must win over any nested element that
            // happens to carry a ResStatus attribute.
            let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<OTA_HotelResNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05" Version="1.0">
  <HotelReservations>
    <HotelReservation ResStatus="Cancel">
      <ResGlobalInfo>
        <SomeNested ResStatus="Commit"/>
        <HotelReservationIDs>
          <HotelReservationID ResID_Value="BK-003"/>
        </HotelReservationIDs>
      </ResGlobalInfo>
    </HotelReservation>
  </HotelReservations>
</OTA_HotelResNotifRQ>"#;
            let notifs = parse_res_notif_rq_raw(xml).unwrap();
            assert_eq!(notifs.len(), 1);
            assert_eq!(notifs[0].1, "Cancel", "start-tag ResStatus must win");
            assert_eq!(notifs[0].0, "BK-003");
        }

        #[test]
        fn test_find_element_start_no_prefix_false_match() {
            // `<HotelReservationFoo>` must not be detected as HotelReservation.
            let xml = r#"<HotelReservationFoo ResStatus="Cancel"></HotelReservationFoo>"#;
            let notifs = parse_res_notif_rq_raw(xml).unwrap();
            assert_eq!(notifs.len(), 0, "must not match the longer element name");
        }

        // ----------------------------------------------------------------
        // Inbound OTA_HotelAvailNotifRQ parsing
        // ----------------------------------------------------------------

        const SAMPLE_AVAIL_RQ: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<OTA_HotelAvailNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05" Version="1.0">
  <AvailStatusMessages HotelCode="H-12345">
    <AvailStatusMessage BookingLimit="3" BookingLimitMessageType="SetLimit">
      <StatusApplicationControl Start="2025-06-01" End="2025-06-05" InvTypeCode="DBL" RatePlanCode="STD"/>
      <RestrictionStatus Status="Open" Restriction="Master"/>
    </AvailStatusMessage>
    <AvailStatusMessage BookingLimit="0" BookingLimitMessageType="SetLimit">
      <StatusApplicationControl Start="2025-07-01" End="2025-07-07" InvTypeCode="SGL"/>
      <RestrictionStatus Status="Close" Restriction="Master"/>
      <LengthsOfStay MinMaxMessageType="SetMinLOS" MinStay="2" MaxStay="14"/>
    </AvailStatusMessage>
  </AvailStatusMessages>
</OTA_HotelAvailNotifRQ>"#;

        #[test]
        fn test_parse_avail_notif_rq_two_messages() {
            let parsed = parse_avail_notif_rq(SAMPLE_AVAIL_RQ).unwrap();
            assert_eq!(parsed.hotel_code, "H-12345");
            assert_eq!(parsed.messages.len(), 2);

            let first = &parsed.messages[0];
            assert_eq!(first.room_type_code, "DBL");
            assert_eq!(first.rate_plan_code.as_deref(), Some("STD"));
            assert_eq!(
                first.start_date,
                NaiveDate::from_ymd_opt(2025, 6, 1).unwrap()
            );
            assert_eq!(first.end_date, NaiveDate::from_ymd_opt(2025, 6, 5).unwrap());
            assert_eq!(first.booking_limit, 3);
            assert_eq!(first.status, "Open");
            assert!(first.los_restrictions.is_none());

            let second = &parsed.messages[1];
            assert_eq!(second.room_type_code, "SGL");
            // No RatePlanCode on the second message -> None.
            assert_eq!(second.rate_plan_code, None);
            assert_eq!(second.status, "Close");
            assert_eq!(second.booking_limit, 0);
            let los = second.los_restrictions.as_ref().expect("LOS present");
            assert_eq!(los.min_los, Some(2));
            assert_eq!(los.max_los, Some(14));
        }

        #[test]
        fn test_avail_round_trip() {
            // Build with the generator, parse with the new parser, expect the
            // same logical content back.
            let msgs = vec![
                AvailStatusMessage {
                    room_type_code: "DBL".to_string(),
                    rate_plan_code: Some("STD".to_string()),
                    start_date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
                    end_date: NaiveDate::from_ymd_opt(2025, 6, 10).unwrap(),
                    booking_limit: 5,
                    status: "Open".to_string(),
                    los_restrictions: None,
                },
                AvailStatusMessage {
                    room_type_code: "SUITE".to_string(),
                    // Use an explicit rate plan: build_avail_notif_rq defaults a
                    // `None` rate_plan_code to "STD" on the wire, so only an
                    // explicit code can round-trip losslessly. (The lossy-None
                    // case is asserted separately below.)
                    rate_plan_code: Some("NONREF".to_string()),
                    start_date: NaiveDate::from_ymd_opt(2025, 8, 1).unwrap(),
                    end_date: NaiveDate::from_ymd_opt(2025, 8, 3).unwrap(),
                    booking_limit: 0,
                    status: "Close".to_string(),
                    los_restrictions: Some(LosRestrictions {
                        min_los: Some(3),
                        max_los: None,
                    }),
                },
            ];
            let xml = build_avail_notif_rq("H-RT", &msgs).unwrap();
            let parsed = parse_avail_notif_rq(&xml).unwrap();
            assert_eq!(parsed.hotel_code, "H-RT");
            assert_eq!(parsed.messages, msgs, "round-trip must preserve messages");
        }

        #[test]
        fn test_avail_none_rate_plan_defaults_to_std_on_wire() {
            // Documents the generator's lossy default: a `None` rate_plan_code
            // is serialized as RatePlanCode="STD", so it parses back as
            // Some("STD") rather than None.
            let msgs = vec![AvailStatusMessage {
                room_type_code: "DBL".to_string(),
                rate_plan_code: None,
                start_date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2025, 6, 2).unwrap(),
                booking_limit: 1,
                status: "Open".to_string(),
                los_restrictions: None,
            }];
            let xml = build_avail_notif_rq("H-DEF", &msgs).unwrap();
            let parsed = parse_avail_notif_rq(&xml).unwrap();
            assert_eq!(parsed.messages[0].rate_plan_code.as_deref(), Some("STD"));
        }

        #[test]
        fn test_parse_avail_notif_rq_malformed_date_errors() {
            let xml = r#"<OTA_HotelAvailNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05">
  <AvailStatusMessages HotelCode="H1">
    <AvailStatusMessage BookingLimit="1">
      <StatusApplicationControl Start="not-a-date" End="2025-06-05" InvTypeCode="DBL"/>
      <RestrictionStatus Status="Open"/>
    </AvailStatusMessage>
  </AvailStatusMessages>
</OTA_HotelAvailNotifRQ>"#;
            let err = parse_avail_notif_rq(xml).unwrap_err();
            assert!(
                matches!(err, BookingError::InvalidMessage(_)),
                "malformed Start date must be InvalidMessage, got {err:?}"
            );
        }

        #[test]
        fn test_parse_avail_notif_rq_empty_is_ok() {
            let xml = r#"<OTA_HotelAvailNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05">
  <AvailStatusMessages HotelCode="H-EMPTY"/>
</OTA_HotelAvailNotifRQ>"#;
            let parsed = parse_avail_notif_rq(xml).unwrap();
            assert_eq!(parsed.hotel_code, "H-EMPTY");
            assert!(parsed.messages.is_empty());
        }

        // ----------------------------------------------------------------
        // Inbound OTA_HotelRateAmountNotifRQ parsing
        // ----------------------------------------------------------------

        const SAMPLE_RATE_RQ: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<OTA_HotelRateAmountNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05" Version="1.0">
  <RateAmountMessages HotelCode="H-999">
    <RateAmountMessage>
      <StatusApplicationControl Start="2025-08-01" End="2025-08-31" InvTypeCode="DBL" RatePlanCode="STD"/>
      <Rates>
        <Rate>
          <BaseByGuestAmts>
            <BaseByGuestAmt AmountAfterTax="120.50" CurrencyCode="EUR"/>
          </BaseByGuestAmts>
        </Rate>
      </Rates>
    </RateAmountMessage>
    <RateAmountMessage>
      <StatusApplicationControl Start="2025-09-01" End="2025-09-30" InvTypeCode="SGL" RatePlanCode="NONREF"/>
      <Rates>
        <Rate>
          <BaseByGuestAmts>
            <BaseByGuestAmt AmountAfterTax="89.00" CurrencyCode="USD"/>
          </BaseByGuestAmts>
        </Rate>
      </Rates>
    </RateAmountMessage>
  </RateAmountMessages>
</OTA_HotelRateAmountNotifRQ>"#;

        #[test]
        fn test_parse_rate_amount_notif_rq_two_messages() {
            let parsed = parse_rate_amount_notif_rq(SAMPLE_RATE_RQ).unwrap();
            assert_eq!(parsed.hotel_code, "H-999");
            assert_eq!(parsed.rates.len(), 2);

            let first = &parsed.rates[0];
            assert_eq!(first.room_type_code, "DBL");
            assert_eq!(first.rate_plan_code.as_deref(), Some("STD"));
            assert_eq!(
                first.start_date,
                NaiveDate::from_ymd_opt(2025, 8, 1).unwrap()
            );
            assert_eq!(
                first.end_date,
                NaiveDate::from_ymd_opt(2025, 8, 31).unwrap()
            );
            assert_eq!(first.amount, "120.50".parse::<Decimal>().unwrap());
            assert_eq!(first.currency, "EUR");

            let second = &parsed.rates[1];
            assert_eq!(second.room_type_code, "SGL");
            assert_eq!(second.amount, "89.00".parse::<Decimal>().unwrap());
            assert_eq!(second.currency, "USD");
        }

        #[test]
        fn test_rate_round_trip() {
            let d1 = NaiveDate::from_ymd_opt(2025, 8, 1).unwrap();
            let d2 = NaiveDate::from_ymd_opt(2025, 8, 31).unwrap();
            let rate: Decimal = "199.99".parse().unwrap();
            let updates = vec![(d1, d2, "DBL", "STD", &rate, "EUR")];
            let xml = build_rate_amount_notif_rq("H-RT2", &updates).unwrap();

            let parsed = parse_rate_amount_notif_rq(&xml).unwrap();
            assert_eq!(parsed.hotel_code, "H-RT2");
            assert_eq!(parsed.rates.len(), 1);
            let r = &parsed.rates[0];
            assert_eq!(r.room_type_code, "DBL");
            assert_eq!(r.rate_plan_code.as_deref(), Some("STD"));
            assert_eq!(r.start_date, d1);
            assert_eq!(r.end_date, d2);
            assert_eq!(r.amount, rate);
            assert_eq!(r.currency, "EUR");
        }

        #[test]
        fn test_parse_rate_amount_notif_rq_bad_amount_errors() {
            let xml = r#"<OTA_HotelRateAmountNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05">
  <RateAmountMessages HotelCode="H1">
    <RateAmountMessage>
      <StatusApplicationControl Start="2025-08-01" End="2025-08-02" InvTypeCode="DBL" RatePlanCode="STD"/>
      <Rates><Rate><BaseByGuestAmts>
        <BaseByGuestAmt AmountAfterTax="not-a-number" CurrencyCode="EUR"/>
      </BaseByGuestAmts></Rate></Rates>
    </RateAmountMessage>
  </RateAmountMessages>
</OTA_HotelRateAmountNotifRQ>"#;
            let err = parse_rate_amount_notif_rq(xml).unwrap_err();
            assert!(
                matches!(err, BookingError::InvalidMessage(_)),
                "bad amount must be InvalidMessage, got {err:?}"
            );
        }

        // ----------------------------------------------------------------
        // XXE / DTD / entity-expansion regression guards (#1519)
        //
        // Every inbound field is read via `attr_value`, which calls quick_xml's
        // `normalized_value`. quick_xml (0.40) resolves only the five predefined
        // XML entities; a custom `<!ENTITY>` from an internal DTD subset is NOT
        // resolved, so `normalized_value` returns `Err` and `attr_value` yields
        // `None` — the poisoned attribute is dropped, never expanded or fetched.
        // `<!DOCTYPE>` itself is an inert `Event::DocType`.
        //
        // These tests lock that safe posture: a quick_xml config change, a switch
        // to another XML library, or enabling custom-entity resolution would flip
        // one of these assertions.
        // ----------------------------------------------------------------

        #[test]
        fn test_parse_avail_notif_rq_does_not_expand_internal_entity() {
            // If internal general entities were expanded, HotelCode would become
            // "INJECTED_SENTINEL". They must not be.
            let xml = r#"<?xml version="1.0"?>
<!DOCTYPE OTA_HotelAvailNotifRQ [ <!ENTITY inj "INJECTED_SENTINEL"> ]>
<OTA_HotelAvailNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05">
  <AvailStatusMessages HotelCode="&inj;"/>
</OTA_HotelAvailNotifRQ>"#;
            // Whether the parser errors or drops the field, the sentinel must
            // never appear — the entity is not expanded.
            if let Ok(parsed) = parse_avail_notif_rq(xml) {
                assert!(
                    !parsed.hotel_code.contains("INJECTED_SENTINEL"),
                    "internal entity was expanded into HotelCode: {:?}",
                    parsed.hotel_code
                );
            }
        }

        #[test]
        fn test_parse_avail_notif_rq_does_not_disclose_external_entity() {
            // Classic XXE file-disclosure probe. quick_xml never fetches external
            // entities, so /etc/passwd contents must never surface.
            let xml = r#"<?xml version="1.0"?>
<!DOCTYPE OTA_HotelAvailNotifRQ [ <!ENTITY xxe SYSTEM "file:///etc/passwd"> ]>
<OTA_HotelAvailNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05">
  <AvailStatusMessages HotelCode="&xxe;"/>
</OTA_HotelAvailNotifRQ>"#;
            if let Ok(parsed) = parse_avail_notif_rq(xml) {
                assert!(
                    !parsed.hotel_code.contains("root:") && !parsed.hotel_code.contains('/'),
                    "external entity disclosed file contents into HotelCode: {:?}",
                    parsed.hotel_code
                );
            }
        }

        #[test]
        fn test_parse_rate_amount_notif_rq_does_not_disclose_external_entity() {
            let xml = r#"<?xml version="1.0"?>
<!DOCTYPE OTA_HotelRateAmountNotifRQ [ <!ENTITY xxe SYSTEM "file:///etc/passwd"> ]>
<OTA_HotelRateAmountNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05">
  <RateAmountMessages HotelCode="&xxe;"/>
</OTA_HotelRateAmountNotifRQ>"#;
            if let Ok(parsed) = parse_rate_amount_notif_rq(xml) {
                assert!(
                    !parsed.hotel_code.contains("root:") && !parsed.hotel_code.contains('/'),
                    "external entity disclosed file contents into HotelCode: {:?}",
                    parsed.hotel_code
                );
            }
        }

        #[test]
        fn test_parse_avail_notif_rq_billion_laughs_does_not_amplify() {
            // Nested-entity amplification ("billion laughs"). quick_xml does not
            // expand custom entities, so this returns promptly without allocating
            // an amplified string — never a multi-KB HotelCode.
            let xml = r#"<?xml version="1.0"?>
<!DOCTYPE OTA_HotelAvailNotifRQ [
  <!ENTITY lol "lol">
  <!ENTITY lol2 "&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;">
  <!ENTITY lol3 "&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;">
  <!ENTITY lol4 "&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;">
]>
<OTA_HotelAvailNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05">
  <AvailStatusMessages HotelCode="&lol4;"/>
</OTA_HotelAvailNotifRQ>"#;
            if let Ok(parsed) = parse_avail_notif_rq(xml) {
                assert!(
                    parsed.hotel_code.len() < 64,
                    "billion-laughs amplified HotelCode to {} bytes",
                    parsed.hotel_code.len()
                );
                assert!(
                    !parsed.hotel_code.contains("lollol"),
                    "billion-laughs entity expanded: {:?}",
                    parsed.hotel_code
                );
            }
        }

        #[test]
        fn test_parse_avail_notif_rq_prefixed_namespace_parity() {
            // A partner sending `ota:`-prefixed elements must parse identically
            // to the default-namespace form (`local_name` strips the prefix).
            let prefixed = r#"<?xml version="1.0"?>
<ota:OTA_HotelAvailNotifRQ xmlns:ota="http://www.opentravel.org/OTA/2003/05">
  <ota:AvailStatusMessages HotelCode="H-NS">
    <ota:AvailStatusMessage BookingLimit="2">
      <ota:StatusApplicationControl Start="2025-06-01" End="2025-06-05" InvTypeCode="DBL" RatePlanCode="STD"/>
      <ota:RestrictionStatus Status="Open"/>
    </ota:AvailStatusMessage>
  </ota:AvailStatusMessages>
</ota:OTA_HotelAvailNotifRQ>"#;
            let parsed = parse_avail_notif_rq(prefixed).unwrap();
            assert_eq!(parsed.hotel_code, "H-NS");
            assert_eq!(parsed.messages.len(), 1);
            let m = &parsed.messages[0];
            assert_eq!(m.room_type_code, "DBL");
            assert_eq!(m.rate_plan_code.as_deref(), Some("STD"));
            assert_eq!(m.booking_limit, 2);
            assert_eq!(m.status, "Open");
            assert_eq!(m.start_date, NaiveDate::from_ymd_opt(2025, 6, 1).unwrap());
            assert_eq!(m.end_date, NaiveDate::from_ymd_opt(2025, 6, 5).unwrap());
        }

        #[test]
        fn test_parse_rate_amount_notif_rq_prefixed_namespace_parity() {
            let prefixed = r#"<?xml version="1.0"?>
<ota:OTA_HotelRateAmountNotifRQ xmlns:ota="http://www.opentravel.org/OTA/2003/05">
  <ota:RateAmountMessages HotelCode="H-NS2">
    <ota:RateAmountMessage>
      <ota:StatusApplicationControl Start="2025-08-01" End="2025-08-31" InvTypeCode="DBL" RatePlanCode="STD"/>
      <ota:Rates><ota:Rate><ota:BaseByGuestAmts>
        <ota:BaseByGuestAmt AmountAfterTax="120.50" CurrencyCode="EUR"/>
      </ota:BaseByGuestAmts></ota:Rate></ota:Rates>
    </ota:RateAmountMessage>
  </ota:RateAmountMessages>
</ota:OTA_HotelRateAmountNotifRQ>"#;
            let parsed = parse_rate_amount_notif_rq(prefixed).unwrap();
            assert_eq!(parsed.hotel_code, "H-NS2");
            assert_eq!(parsed.rates.len(), 1);
            let r = &parsed.rates[0];
            assert_eq!(r.room_type_code, "DBL");
            assert_eq!(r.rate_plan_code.as_deref(), Some("STD"));
            assert_eq!(r.amount, "120.50".parse::<Decimal>().unwrap());
            assert_eq!(r.currency, "EUR");
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
// Retry Configuration (AC-5 rate/availability push)
// ============================================

/// Retry policy for OTA rate & availability *push* operations.
///
/// Booking.com's Supply XML endpoint is occasionally transient-fail under
/// load (HTTP 429/5xx) and the OTA push messages are idempotent (each carries
/// an absolute `Start`/`End`/`InvTypeCode` application control rather than a
/// delta), so re-sending the same document is safe. This struct controls the
/// bounded exponential-backoff retry loop used by [`BookingClient::push_rates`]
/// and [`BookingClient::push_availability`].
///
/// This is intentionally a small, push-specific policy distinct from
/// [`crate::connector::RetryConfig`], which drives the generic [`crate::connector::Connector`]
/// request/response abstraction. The Booking client talks to `reqwest`
/// directly (it must hand-roll OTA XML bodies + Basic auth), so it carries its
/// own lightweight policy rather than routing through the generic connector.
#[derive(Debug, Clone)]
pub struct BookingRetryConfig {
    /// Total number of attempts (1 = no retry). Must be >= 1.
    pub max_attempts: u32,
    /// Delay before the first retry, in milliseconds.
    pub initial_delay_ms: u64,
    /// Upper bound on any single backoff delay, in milliseconds.
    pub max_delay_ms: u64,
    /// Multiplier applied to the delay after each failed attempt.
    pub backoff_multiplier: u32,
}

impl Default for BookingRetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 500,
            max_delay_ms: 8_000,
            backoff_multiplier: 2,
        }
    }
}

impl BookingRetryConfig {
    /// A no-retry policy (single attempt) — useful in tests to keep them fast.
    pub fn no_retry() -> Self {
        Self {
            max_attempts: 1,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            backoff_multiplier: 1,
        }
    }

    /// Return a copy of this policy with `max_attempts` overridden, keeping the
    /// other (delay/backoff) parameters. Handy for zero-delay multi-attempt
    /// retry configs in tests (`no_retry().clone_with_attempts(3)`).
    pub fn clone_with_attempts(&self, max_attempts: u32) -> Self {
        Self {
            max_attempts: max_attempts.max(1),
            ..self.clone()
        }
    }

    /// Backoff delay (ms) before the retry that follows `attempt` (0-based:
    /// `attempt = 0` is the delay after the first attempt failed). Capped at
    /// [`Self::max_delay_ms`].
    pub fn delay_for_attempt(&self, attempt: u32) -> u64 {
        let factor = self.backoff_multiplier.saturating_pow(attempt);
        self.initial_delay_ms
            .saturating_mul(factor as u64)
            .min(self.max_delay_ms)
    }
}

/// Whether an HTTP status code returned by the Booking.com endpoint is worth
/// retrying. Transient server-side / throttling failures are retryable;
/// client errors (4xx other than 408/429) are not — they indicate a bad
/// message and will fail identically on retry.
fn is_retryable_status(status: u16) -> bool {
    matches!(status, 408 | 425 | 429 | 500 | 502 | 503 | 504)
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
                let base_delay = self.retry.delay_for_attempt(attempt - 1);
                let delay = retry_after_ms
                    .map(|ra| ra.max(base_delay))
                    .unwrap_or(base_delay);
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
// OAuth 2.0 (authorization-code) client
// ============================================
//
// Booking.com's newer Connectivity APIs use an OAuth 2.0 authorization-code
// grant in addition to the legacy Basic-auth Supply XML credentials handled by
// [`BookingClient`].  This section mirrors the Airbnb OAuth client
// (`integrations::airbnb`) so the api-server can host a parallel
// `POST /organizations/{org_id}/booking/token/exchange` route (Coverage 83-2).

/// Booking.com OAuth token endpoint (authorization-code + refresh grants).
pub const BOOKING_OAUTH_TOKEN_URL: &str = "https://account.booking.com/oauth2/token";

/// Booking.com OAuth authorization endpoint (front-channel redirect target).
pub const BOOKING_OAUTH_AUTH_URL: &str = "https://account.booking.com/oauth2/authorize";

/// OAuth configuration for Booking.com.
#[derive(Debug, Clone)]
pub struct BookingOAuthConfig {
    /// Booking.com OAuth client ID.
    pub client_id: String,
    /// Booking.com OAuth client secret.
    pub client_secret: String,
    /// OAuth redirect URI for the callback.
    pub redirect_uri: String,
}

/// OAuth tokens received from Booking.com.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookingOAuthTokens {
    /// Access token for API calls.
    pub access_token: String,
    /// Refresh token for obtaining new access tokens.
    pub refresh_token: Option<String>,
    /// When the access token expires.
    pub expires_at: Option<DateTime<Utc>>,
    /// Token type (usually "Bearer").
    pub token_type: String,
    /// OAuth scopes granted.
    pub scope: Option<String>,
}

/// Booking.com OAuth client (authorization-code grant).
///
/// Distinct from [`BookingClient`], which speaks the legacy Basic-auth Supply
/// XML protocol.  This client only handles the OAuth token lifecycle.
pub struct BookingOAuthClient {
    client: reqwest::Client,
    config: BookingOAuthConfig,
}

impl BookingOAuthClient {
    /// Create a new Booking.com OAuth client.
    pub fn new(config: BookingOAuthConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    /// Create a new client from individual OAuth parameters.
    pub fn with_credentials(
        client_id: String,
        client_secret: String,
        redirect_uri: String,
    ) -> Self {
        Self::new(BookingOAuthConfig {
            client_id,
            client_secret,
            redirect_uri,
        })
    }

    /// Generate the OAuth authorization URL the user is redirected to.
    pub fn generate_auth_url(&self, state: &str) -> String {
        let scopes = "reservations:read availability:write rates:write";
        format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&state={}&scope={}",
            BOOKING_OAUTH_AUTH_URL,
            urlencoding::encode(&self.config.client_id),
            urlencoding::encode(&self.config.redirect_uri),
            urlencoding::encode(state),
            urlencoding::encode(scopes),
        )
    }

    /// Exchange an authorization code for access and refresh tokens.
    pub async fn exchange_code(&self, code: &str) -> Result<BookingOAuthTokens, BookingError> {
        #[derive(Serialize)]
        struct TokenRequest<'a> {
            grant_type: &'a str,
            client_id: &'a str,
            client_secret: &'a str,
            code: &'a str,
            redirect_uri: &'a str,
        }

        let response = self
            .client
            .post(BOOKING_OAUTH_TOKEN_URL)
            .form(&TokenRequest {
                grant_type: "authorization_code",
                client_id: &self.config.client_id,
                client_secret: &self.config.client_secret,
                code,
                redirect_uri: &self.config.redirect_uri,
            })
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(BookingError::Auth(error));
        }

        let token_response: OAuthTokenResponse = response
            .json()
            .await
            .map_err(|e| BookingError::Api(e.to_string()))?;

        Ok(token_response.into_tokens(None))
    }

    /// Refresh an expired access token using the stored refresh token.
    pub async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<BookingOAuthTokens, BookingError> {
        #[derive(Serialize)]
        struct RefreshRequest<'a> {
            grant_type: &'a str,
            client_id: &'a str,
            client_secret: &'a str,
            refresh_token: &'a str,
        }

        let response = self
            .client
            .post(BOOKING_OAUTH_TOKEN_URL)
            .form(&RefreshRequest {
                grant_type: "refresh_token",
                client_id: &self.config.client_id,
                client_secret: &self.config.client_secret,
                refresh_token,
            })
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(BookingError::Auth(error));
        }

        let token_response: OAuthTokenResponse = response
            .json()
            .await
            .map_err(|e| BookingError::Api(e.to_string()))?;

        Ok(token_response.into_tokens(Some(refresh_token)))
    }
}

/// Raw token-endpoint response shape (shared by exchange + refresh).
#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    #[serde(default = "default_token_type")]
    token_type: String,
    scope: Option<String>,
}

fn default_token_type() -> String {
    "Bearer".to_string()
}

impl OAuthTokenResponse {
    /// Convert the wire response into [`BookingOAuthTokens`], computing
    /// `expires_at` from `expires_in` and falling back to `fallback_refresh`
    /// when the endpoint did not rotate the refresh token.
    fn into_tokens(self, fallback_refresh: Option<&str>) -> BookingOAuthTokens {
        let expires_at = self
            .expires_in
            .map(|secs| Utc::now() + Duration::seconds(secs));
        BookingOAuthTokens {
            access_token: self.access_token,
            refresh_token: self
                .refresh_token
                .or_else(|| fallback_refresh.map(|s| s.to_string())),
            expires_at,
            token_type: self.token_type,
            scope: self.scope,
        }
    }
}

// ============================================
// Tests
// ============================================

#[cfg(test)]
mod oauth_tests {
    use super::*;

    #[test]
    fn test_generate_auth_url() {
        let client = BookingOAuthClient::new(BookingOAuthConfig {
            client_id: "test_client_id".to_string(),
            client_secret: "test_secret".to_string(),
            redirect_uri: "http://localhost:8080/callback".to_string(),
        });
        let url = client.generate_auth_url("test_state");
        assert!(url.contains("account.booking.com"));
        assert!(url.contains("test_client_id"));
        assert!(url.contains("test_state"));
        assert!(url.contains("response_type=code"));
        assert!(!url.contains("test_secret"));
    }

    #[test]
    fn test_token_response_deserialization_full() {
        let json = r#"{"access_token":"at-123","refresh_token":"rt-456","expires_in":3600,"token_type":"Bearer","scope":"reservations:read"}"#;
        let parsed: OAuthTokenResponse = serde_json::from_str(json).unwrap();
        let tokens = parsed.into_tokens(None);
        assert_eq!(tokens.access_token, "at-123");
        assert_eq!(tokens.refresh_token.as_deref(), Some("rt-456"));
        assert_eq!(tokens.token_type, "Bearer");
        assert_eq!(tokens.scope.as_deref(), Some("reservations:read"));
        assert!(tokens.expires_at.is_some());
    }

    #[test]
    fn test_token_response_defaults_token_type() {
        let json = r#"{"access_token":"at-789","expires_in":7200}"#;
        let parsed: OAuthTokenResponse = serde_json::from_str(json).unwrap();
        let tokens = parsed.into_tokens(None);
        assert_eq!(tokens.token_type, "Bearer");
        assert!(tokens.refresh_token.is_none());
        assert!(tokens.scope.is_none());
    }

    #[test]
    fn test_refresh_falls_back_to_existing_refresh_token() {
        let json = r#"{"access_token":"at-new","expires_in":3600,"token_type":"Bearer"}"#;
        let parsed: OAuthTokenResponse = serde_json::from_str(json).unwrap();
        let tokens = parsed.into_tokens(Some("rt-original"));
        assert_eq!(tokens.access_token, "at-new");
        assert_eq!(tokens.refresh_token.as_deref(), Some("rt-original"));
    }

    #[test]
    fn test_rotated_refresh_token_wins_over_fallback() {
        let json = r#"{"access_token":"at-new","refresh_token":"rt-rotated","expires_in":3600}"#;
        let parsed: OAuthTokenResponse = serde_json::from_str(json).unwrap();
        let tokens = parsed.into_tokens(Some("rt-original"));
        assert_eq!(tokens.refresh_token.as_deref(), Some("rt-rotated"));
    }

    #[test]
    fn test_oauth_tokens_roundtrip_serde() {
        let tokens = BookingOAuthTokens {
            access_token: "at".to_string(),
            refresh_token: Some("rt".to_string()),
            expires_at: None,
            token_type: "Bearer".to_string(),
            scope: Some("rates:write".to_string()),
        };
        let json = serde_json::to_string(&tokens).unwrap();
        let back: BookingOAuthTokens = serde_json::from_str(&json).unwrap();
        assert_eq!(back.access_token, "at");
        assert_eq!(back.refresh_token.as_deref(), Some("rt"));
        assert_eq!(back.scope.as_deref(), Some("rates:write"));
    }
}

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
    }

    impl MockResponse {
        fn status(status: u16, body: &str) -> Self {
            Self {
                status,
                body: body.to_string(),
                extra_headers: vec![],
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
}
