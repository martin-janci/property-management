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
        Some("Indeterminate Booking.com response: no <Success> or <Errors> element".to_string()),
    )
}

/// Parse an `OTA_HotelResNotifRQ` body and return a list of
/// `(res_id, res_status, element_xml_fragment)` tuples.
///
/// Each fragment is the raw XML of one `<HotelReservation>` element so
/// that higher-level code can parse reservation details independently.
pub fn parse_res_notif_rq_raw(xml: &str) -> Result<Vec<(String, String, String)>, BookingError> {
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
        Some(s) => NaiveDate::parse_from_str(&s, "%Y-%m-%d")
            .map_err(|_| BookingError::InvalidMessage(format!("invalid OTA date in {attr}: {s}"))),
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
        let xml = r#"<OTA_HotelAvailNotifRS xmlns="http://www.opentravel.org/OTA/2003/05"><Succ"#;
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
        // Pin the concrete safe outcome (not just "no sentinel", which is
        // vacuously true for an empty/errored result): parsing must still
        // SUCCEED and the poisoned attribute must be dropped to empty — the
        // custom entity is never resolved/expanded. #1591
        let parsed =
            parse_avail_notif_rq(xml).expect("DTD input must still parse (entity dropped)");
        assert_eq!(
            parsed.hotel_code, "",
            "internal entity must be dropped to empty, not expanded into HotelCode"
        );
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
        let parsed = parse_avail_notif_rq(xml).expect("DTD input must still parse");
        assert_eq!(
            parsed.hotel_code, "",
            "external entity must yield no value (no file contents, no expansion)"
        );
    }

    #[test]
    fn test_parse_rate_amount_notif_rq_does_not_disclose_external_entity() {
        let xml = r#"<?xml version="1.0"?>
<!DOCTYPE OTA_HotelRateAmountNotifRQ [ <!ENTITY xxe SYSTEM "file:///etc/passwd"> ]>
<OTA_HotelRateAmountNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05">
  <RateAmountMessages HotelCode="&xxe;"/>
</OTA_HotelRateAmountNotifRQ>"#;
        let parsed = parse_rate_amount_notif_rq(xml).expect("DTD input must still parse");
        assert_eq!(
            parsed.hotel_code, "",
            "external entity must yield no value (no file contents, no expansion)"
        );
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
        let parsed = parse_avail_notif_rq(xml).expect("DTD input must still parse");
        assert_eq!(
            parsed.hotel_code, "",
            "billion-laughs entity must not amplify (or appear at all) in HotelCode"
        );
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

    // ----------------------------------------------------------------
    // Empty payloads (gap 83-2): generators must emit valid documents
    // for zero messages, and those documents must round-trip.
    // ----------------------------------------------------------------

    #[test]
    fn test_build_avail_notif_rq_empty_messages_round_trips() {
        let xml = build_avail_notif_rq("H-EMPTY", &[]).unwrap();
        assert!(xml.contains("OTA_HotelAvailNotifRQ"));
        assert!(xml.contains("HotelCode=\"H-EMPTY\""));
        let parsed = parse_avail_notif_rq(&xml).unwrap();
        assert_eq!(parsed.hotel_code, "H-EMPTY");
        assert!(parsed.messages.is_empty());
    }

    #[test]
    fn test_build_rate_amount_notif_rq_empty_updates_round_trips() {
        let xml = build_rate_amount_notif_rq("H-EMPTY2", &[]).unwrap();
        assert!(xml.contains("OTA_HotelRateAmountNotifRQ"));
        assert!(xml.contains("HotelCode=\"H-EMPTY2\""));
        let parsed = parse_rate_amount_notif_rq(&xml).unwrap();
        assert_eq!(parsed.hotel_code, "H-EMPTY2");
        assert!(parsed.rates.is_empty());
    }

    // ----------------------------------------------------------------
    // EchoToken (idempotency key) determinism
    // ----------------------------------------------------------------

    #[test]
    fn test_compute_echo_token_deterministic_and_content_sensitive() {
        let a = compute_echo_token("avail", "H1|DBL:STD");
        let b = compute_echo_token("avail", "H1|DBL:STD");
        let c = compute_echo_token("avail", "H1|SGL:STD");
        assert_eq!(a, b, "same payload must yield the same token");
        assert_ne!(a, c, "different payload must yield a different token");
        assert_eq!(a.len(), 32, "128-bit hex prefix");
        assert!(a.chars().all(|ch| ch.is_ascii_hexdigit()));
    }

    #[test]
    fn test_compute_echo_token_kind_namespacing() {
        // Equal canonical bodies must not collide across message kinds.
        let avail = compute_echo_token("avail", "same-body");
        let rate = compute_echo_token("rate", "same-body");
        assert_ne!(avail, rate, "kind must namespace the digest");
    }

    #[test]
    fn test_build_avail_notif_rq_stamps_stable_echo_token() {
        let msgs = vec![AvailStatusMessage {
            room_type_code: "DBL".to_string(),
            rate_plan_code: Some("STD".to_string()),
            start_date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2025, 6, 2).unwrap(),
            booking_limit: 2,
            status: "Open".to_string(),
            los_restrictions: None,
        }];
        let first = build_avail_notif_rq("H-ET", &msgs).unwrap();
        let second = build_avail_notif_rq("H-ET", &msgs).unwrap();
        assert!(first.contains("EchoToken=\""), "EchoToken missing");
        assert_eq!(
            first, second,
            "identical payload must serialize identically (retry dedup)"
        );
    }

    // ----------------------------------------------------------------
    // parse_res_notif_rq_raw: multi-reservation + fallback paths
    // ----------------------------------------------------------------

    #[test]
    fn test_parse_res_notif_rq_raw_multiple_reservations() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<OTA_HotelResNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05" Version="1.0">
  <HotelReservations>
<HotelReservation ResStatus="Commit">
  <HotelReservationID ResID_Value="BK-100"/>
</HotelReservation>
<HotelReservation ResStatus="Modify">
  <HotelReservationID ResID_Value="BK-200"/>
</HotelReservation>
<HotelReservation ResStatus="Cancel">
  <HotelReservationID ResID_Value="BK-300"/>
</HotelReservation>
  </HotelReservations>
</OTA_HotelResNotifRQ>"#;
        let notifs = parse_res_notif_rq_raw(xml).unwrap();
        assert_eq!(notifs.len(), 3);
        assert_eq!(
            notifs
                .iter()
                .map(|(id, status, _)| (id.as_str(), status.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("BK-100", "Commit"),
                ("BK-200", "Modify"),
                ("BK-300", "Cancel")
            ]
        );
        // Each fragment is the standalone element for independent re-parsing.
        for (_, _, frag) in &notifs {
            assert!(frag.starts_with("<HotelReservation"));
            assert!(frag.ends_with("</HotelReservation>"));
        }
    }

    #[test]
    fn test_parse_res_notif_rq_raw_missing_res_status_defaults_to_commit() {
        let xml = r#"<OTA_HotelResNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05">
<HotelReservation>
  <HotelReservationID ResID_Value="BK-DEF"/>
</HotelReservation>
</OTA_HotelResNotifRQ>"#;
        let notifs = parse_res_notif_rq_raw(xml).unwrap();
        assert_eq!(notifs.len(), 1);
        assert_eq!(
            notifs[0].1, "Commit",
            "missing ResStatus defaults to Commit"
        );
        assert_eq!(notifs[0].0, "BK-DEF");
    }

    #[test]
    fn test_parse_res_notif_rq_raw_falls_back_to_id_attr() {
        let xml = r#"<OTA_HotelResNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05">
<HotelReservation ResStatus="Commit">
  <UniqueID ID="LEGACY-42"/>
</HotelReservation>
</OTA_HotelResNotifRQ>"#;
        let notifs = parse_res_notif_rq_raw(xml).unwrap();
        assert_eq!(notifs.len(), 1);
        assert_eq!(notifs[0].0, "LEGACY-42", "must fall back to ID attribute");
    }

    #[test]
    fn test_parse_res_notif_rq_raw_no_id_generates_uuid() {
        let xml = r#"<OTA_HotelResNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05">
<HotelReservation ResStatus="Commit"></HotelReservation>
</OTA_HotelResNotifRQ>"#;
        let notifs = parse_res_notif_rq_raw(xml).unwrap();
        assert_eq!(notifs.len(), 1);
        assert!(
            uuid::Uuid::parse_str(&notifs[0].0).is_ok(),
            "missing IDs must yield a synthetic UUID, got {}",
            notifs[0].0
        );
    }

    #[test]
    fn test_parse_res_notif_rq_raw_unclosed_element_stops_cleanly() {
        // A truncated document (open tag without matching close) must not
        // loop or panic; the dangling reservation is simply skipped.
        let xml = r#"<OTA_HotelResNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05">
<HotelReservation ResStatus="Commit">
  <HotelReservationID ResID_Value="BK-OK"/>
</HotelReservation>
<HotelReservation ResStatus="Cancel">
  <HotelReservationID ResID_Value="BK-TRUNCATED"/>"#;
        let notifs = parse_res_notif_rq_raw(xml).unwrap();
        assert_eq!(notifs.len(), 1, "only the complete element is returned");
        assert_eq!(notifs[0].0, "BK-OK");
    }

    #[test]
    fn test_parse_res_notif_rq_raw_empty_document() {
        let xml = r#"<OTA_HotelResNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05">
  <HotelReservations/>
</OTA_HotelResNotifRQ>"#;
        let notifs = parse_res_notif_rq_raw(xml).unwrap();
        assert!(notifs.is_empty());
    }

    // ----------------------------------------------------------------
    // Date boundaries
    // ----------------------------------------------------------------

    #[test]
    fn test_parse_avail_notif_rq_leap_day_ok_and_invalid_leap_day_errors() {
        let valid = r#"<OTA_HotelAvailNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05">
  <AvailStatusMessages HotelCode="H-LEAP">
<AvailStatusMessage BookingLimit="1">
  <StatusApplicationControl Start="2024-02-29" End="2024-03-01" InvTypeCode="DBL"/>
  <RestrictionStatus Status="Open"/>
</AvailStatusMessage>
  </AvailStatusMessages>
</OTA_HotelAvailNotifRQ>"#;
        let parsed = parse_avail_notif_rq(valid).unwrap();
        assert_eq!(
            parsed.messages[0].start_date,
            NaiveDate::from_ymd_opt(2024, 2, 29).unwrap()
        );

        // 2025 is not a leap year: Feb 29 must be a hard InvalidMessage.
        let invalid = valid.replace("2024-02-29", "2025-02-29");
        let err = parse_avail_notif_rq(&invalid).unwrap_err();
        assert!(matches!(err, BookingError::InvalidMessage(_)));
    }

    #[test]
    fn test_parse_avail_notif_rq_missing_dates_default_to_epoch() {
        // A missing Start/End attribute is tolerated (epoch), only a present
        // but malformed value is an error.
        let xml = r#"<OTA_HotelAvailNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05">
  <AvailStatusMessages HotelCode="H-NODATE">
<AvailStatusMessage BookingLimit="1">
  <StatusApplicationControl InvTypeCode="DBL"/>
  <RestrictionStatus Status="Open"/>
</AvailStatusMessage>
  </AvailStatusMessages>
</OTA_HotelAvailNotifRQ>"#;
        let parsed = parse_avail_notif_rq(xml).unwrap();
        let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
        assert_eq!(parsed.messages[0].start_date, epoch);
        assert_eq!(parsed.messages[0].end_date, epoch);
    }

    #[test]
    fn test_avail_round_trip_single_day_range() {
        // Start == End (single-night update) must round-trip unchanged.
        let day = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();
        let msgs = vec![AvailStatusMessage {
            room_type_code: "DBL".to_string(),
            rate_plan_code: Some("STD".to_string()),
            start_date: day,
            end_date: day,
            booking_limit: 1,
            status: "Open".to_string(),
            los_restrictions: None,
        }];
        let xml = build_avail_notif_rq("H-1D", &msgs).unwrap();
        let parsed = parse_avail_notif_rq(&xml).unwrap();
        assert_eq!(parsed.messages, msgs);
    }

    // ----------------------------------------------------------------
    // Malformed-XML error paths
    // ----------------------------------------------------------------

    #[test]
    fn test_parse_avail_notif_rq_mismatched_tags_is_xml_error() {
        let xml = r#"<OTA_HotelAvailNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05">
  <AvailStatusMessages HotelCode="H1">
</WrongClose>
</OTA_HotelAvailNotifRQ>"#;
        let err = parse_avail_notif_rq(xml).unwrap_err();
        assert!(
            matches!(err, BookingError::Xml(_)),
            "mismatched tags must be BookingError::Xml, got {err:?}"
        );
    }

    #[test]
    fn test_parse_rate_amount_notif_rq_mismatched_tags_is_xml_error() {
        let xml = r#"<OTA_HotelRateAmountNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05">
  <RateAmountMessages HotelCode="H1">
</Nope>
</OTA_HotelRateAmountNotifRQ>"#;
        let err = parse_rate_amount_notif_rq(xml).unwrap_err();
        assert!(matches!(err, BookingError::Xml(_)));
    }

    // ----------------------------------------------------------------
    // Rate parsing defaults + precedence
    // ----------------------------------------------------------------

    #[test]
    fn test_parse_rate_amount_notif_rq_first_base_amount_wins() {
        let xml = r#"<OTA_HotelRateAmountNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05">
  <RateAmountMessages HotelCode="H1">
<RateAmountMessage>
  <StatusApplicationControl Start="2025-08-01" End="2025-08-02" InvTypeCode="DBL" RatePlanCode="STD"/>
  <Rates><Rate><BaseByGuestAmts>
    <BaseByGuestAmt AmountAfterTax="100.00" CurrencyCode="EUR"/>
    <BaseByGuestAmt AmountAfterTax="999.99" CurrencyCode="USD"/>
  </BaseByGuestAmts></Rate></Rates>
</RateAmountMessage>
  </RateAmountMessages>
</OTA_HotelRateAmountNotifRQ>"#;
        let parsed = parse_rate_amount_notif_rq(xml).unwrap();
        assert_eq!(parsed.rates.len(), 1);
        assert_eq!(parsed.rates[0].amount, "100.00".parse::<Decimal>().unwrap());
        assert_eq!(parsed.rates[0].currency, "EUR", "first BaseByGuestAmt wins");
    }

    #[test]
    fn test_parse_rate_amount_notif_rq_before_tax_fallback() {
        // AmountAfterTax absent -> AmountBeforeTax is used.
        let xml = r#"<OTA_HotelRateAmountNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05">
  <RateAmountMessages HotelCode="H1">
<RateAmountMessage>
  <StatusApplicationControl Start="2025-08-01" End="2025-08-02" InvTypeCode="DBL"/>
  <Rates><Rate><BaseByGuestAmts>
    <BaseByGuestAmt AmountBeforeTax="80.00" CurrencyCode="CZK"/>
  </BaseByGuestAmts></Rate></Rates>
</RateAmountMessage>
  </RateAmountMessages>
</OTA_HotelRateAmountNotifRQ>"#;
        let parsed = parse_rate_amount_notif_rq(xml).unwrap();
        assert_eq!(parsed.rates[0].amount, "80.00".parse::<Decimal>().unwrap());
        assert_eq!(parsed.rates[0].currency, "CZK");
    }

    #[test]
    fn test_parse_rate_amount_notif_rq_missing_amount_defaults() {
        // No BaseByGuestAmt at all -> amount Decimal::ZERO, currency "EUR".
        let xml = r#"<OTA_HotelRateAmountNotifRQ xmlns="http://www.opentravel.org/OTA/2003/05">
  <RateAmountMessages HotelCode="H1">
<RateAmountMessage>
  <StatusApplicationControl Start="2025-08-01" End="2025-08-02" InvTypeCode="DBL"/>
</RateAmountMessage>
  </RateAmountMessages>
</OTA_HotelRateAmountNotifRQ>"#;
        let parsed = parse_rate_amount_notif_rq(xml).unwrap();
        assert_eq!(parsed.rates[0].amount, Decimal::ZERO);
        assert_eq!(parsed.rates[0].currency, "EUR");
    }

    // ----------------------------------------------------------------
    // Attribute escaping round-trip + res notif RS defaults
    // ----------------------------------------------------------------

    #[test]
    fn test_avail_round_trip_escapes_special_chars_in_attributes() {
        // quick-xml must escape `&`/`<` in attribute values on write and
        // unescape them on read — codes containing entities must round-trip.
        let msgs = vec![AvailStatusMessage {
            room_type_code: "A&B<C".to_string(),
            rate_plan_code: Some("R\"Q".to_string()),
            start_date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2025, 6, 2).unwrap(),
            booking_limit: 1,
            status: "Open".to_string(),
            los_restrictions: None,
        }];
        let xml = build_avail_notif_rq("H&M", &msgs).unwrap();
        assert!(
            !xml.contains("HotelCode=\"H&M\""),
            "raw ampersand must be escaped"
        );
        let parsed = parse_avail_notif_rq(&xml).unwrap();
        assert_eq!(parsed.hotel_code, "H&M");
        assert_eq!(parsed.messages, msgs);
    }

    #[test]
    fn test_build_res_notif_rs_error_default_text() {
        let xml = build_res_notif_rs(false, None).unwrap();
        assert!(
            xml.contains("Processing error"),
            "default error text missing"
        );
        let (ok, err) = parse_response_status(&xml);
        assert!(!ok);
        assert_eq!(err.as_deref(), Some("Processing error"));
    }

    #[test]
    fn test_build_res_notif_rs_round_trips_through_parse_response_status() {
        let xml = build_res_notif_rs(true, None).unwrap();
        let (ok, err) = parse_response_status(&xml);
        assert!(ok);
        assert!(err.is_none());
    }

    // ----------------------------------------------------------------
    // extract_xml_element
    // ----------------------------------------------------------------

    #[test]
    fn test_extract_xml_element_basic_missing_and_empty() {
        let frag = r#"<Root><GuestCount>2</GuestCount><Empty></Empty></Root>"#;
        assert_eq!(
            extract_xml_element(frag, "GuestCount").as_deref(),
            Some("2")
        );
        assert_eq!(extract_xml_element(frag, "Missing"), None);
        // Empty content is filtered to None (not Some("")).
        assert_eq!(extract_xml_element(frag, "Empty"), None);
    }
}
