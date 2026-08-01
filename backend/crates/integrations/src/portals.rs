//! Real estate portal integration client (Story 83.3).
//!
//! Implements webhook handling for real estate portals like Sreality, Bezrealitky, and Immowelt.
//! Provides a parser trait for extensible portal support.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

// ============================================
// Error Types
// ============================================

/// Errors that can occur during portal operations.
#[derive(Debug, Error)]
pub enum PortalError {
    /// API error from portal.
    #[error("API error: {0}")]
    Api(String),

    /// Network/HTTP error.
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Webhook signature verification failed.
    #[error("Signature verification failed")]
    InvalidSignature,

    /// Unknown portal type.
    #[error("Unknown portal type: {0}")]
    UnknownPortal(String),

    /// Parse error.
    #[error("Parse error: {0}")]
    Parse(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Webhook not found.
    #[error("Webhook connection not found")]
    NotFound,
}

/// Parse error for webhook payloads.
#[derive(Debug, Error)]
pub enum ParseError {
    /// JSON parsing failed.
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    /// Missing required field.
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Invalid field value.
    #[error("Invalid field value: {0}")]
    InvalidValue(String),

    /// Unsupported format.
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
}

// ============================================
// Portal Types
// ============================================

/// Supported portal types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PortalType {
    /// Sreality.cz (Czech Republic).
    Sreality,
    /// Bezrealitky.cz (Czech Republic).
    Bezrealitky,
    /// Immowelt.de (Germany).
    Immowelt,
    /// Custom/generic portal.
    Custom,
}

impl PortalType {
    /// Get portal type from string.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "sreality" => Some(Self::Sreality),
            "bezrealitky" => Some(Self::Bezrealitky),
            "immowelt" => Some(Self::Immowelt),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    /// Get the display name of the portal.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Sreality => "Sreality.cz",
            Self::Bezrealitky => "Bezrealitky.cz",
            Self::Immowelt => "Immowelt.de",
            Self::Custom => "Custom Portal",
        }
    }
}

impl std::fmt::Display for PortalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sreality => write!(f, "sreality"),
            Self::Bezrealitky => write!(f, "bezrealitky"),
            Self::Immowelt => write!(f, "immowelt"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

// ============================================
// Portal Connection
// ============================================

/// A portal webhook connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalConnection {
    /// Connection ID.
    pub id: Uuid,
    /// Organization ID.
    pub organization_id: Uuid,
    /// Portal type.
    pub portal_type: PortalType,
    /// Connection name for display.
    pub name: String,
    /// Webhook secret for signature verification.
    pub webhook_secret: String,
    /// Whether the connection is active.
    pub is_active: bool,
    /// Last webhook received timestamp.
    pub last_webhook_at: Option<DateTime<Utc>>,
    /// Created timestamp.
    pub created_at: DateTime<Utc>,
    /// Updated timestamp.
    pub updated_at: DateTime<Utc>,
}

impl PortalConnection {
    /// Generate the webhook URL for this connection.
    pub fn webhook_url(&self, base_url: &str) -> String {
        format!("{}/api/v1/webhooks/portal/{}", base_url, self.id)
    }
}

// ============================================
// Parsed Inquiry
// ============================================

/// A parsed inquiry from a portal webhook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedInquiry {
    /// External inquiry ID from the portal.
    pub external_id: String,
    /// External property/listing ID.
    pub property_id: String,
    /// Contact name.
    pub name: String,
    /// Contact email.
    pub email: String,
    /// Contact phone (optional).
    pub phone: Option<String>,
    /// Inquiry message.
    pub message: String,
    /// Additional metadata.
    pub metadata: Option<serde_json::Value>,
}

/// A stored portal inquiry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalInquiry {
    /// Internal inquiry ID.
    pub id: Uuid,
    /// Portal connection ID.
    pub connection_id: Uuid,
    /// Portal type.
    pub portal_type: PortalType,
    /// External inquiry ID.
    pub external_id: Option<String>,
    /// Linked property ID (if matched).
    pub property_id: Option<Uuid>,
    /// External property ID from portal.
    pub property_external_id: Option<String>,
    /// Contact name.
    pub contact_name: String,
    /// Contact email.
    pub contact_email: String,
    /// Contact phone.
    pub contact_phone: Option<String>,
    /// Inquiry message.
    pub message: String,
    /// Inquiry status.
    pub status: InquiryStatus,
    /// Raw webhook payload.
    pub raw_payload: serde_json::Value,
    /// When the webhook was received.
    pub received_at: DateTime<Utc>,
    /// When the inquiry was read.
    pub read_at: Option<DateTime<Utc>>,
    /// When a reply was sent.
    pub replied_at: Option<DateTime<Utc>>,
    /// Created timestamp.
    pub created_at: DateTime<Utc>,
}

impl Default for PortalInquiry {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            connection_id: Uuid::nil(),
            portal_type: PortalType::Custom,
            external_id: None,
            property_id: None,
            property_external_id: None,
            contact_name: String::new(),
            contact_email: String::new(),
            contact_phone: None,
            message: String::new(),
            status: InquiryStatus::New,
            raw_payload: serde_json::Value::Null,
            received_at: Utc::now(),
            read_at: None,
            replied_at: None,
            created_at: Utc::now(),
        }
    }
}

/// Inquiry status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InquiryStatus {
    /// New, unread inquiry.
    #[default]
    New,
    /// Inquiry has been read.
    Read,
    /// A reply has been sent.
    Replied,
    /// Inquiry has been archived.
    Archived,
}

// ============================================
// Portal Parser Trait
// ============================================

/// Trait for parsing portal webhook payloads.
///
/// Implement this trait to add support for new portal types.
pub trait PortalParser: Send + Sync {
    /// Parse a webhook payload into a `ParsedInquiry`.
    ///
    /// # Arguments
    /// * `body` - The raw webhook body (usually JSON)
    ///
    /// # Returns
    /// Parsed inquiry or error.
    fn parse(&self, body: &str) -> Result<ParsedInquiry, ParseError>;

    /// Get the portal type this parser handles.
    fn portal_type(&self) -> PortalType;

    /// Get the expected signature header name.
    fn signature_header(&self) -> &'static str {
        "X-Webhook-Signature"
    }

    /// Verify the webhook signature.
    ///
    /// Default implementation uses HMAC-SHA256.
    fn verify_signature(&self, secret: &str, body: &str, signature: &str) -> bool {
        verify_webhook_signature(secret, body, signature)
    }
}

// ============================================
// Sreality Parser
// ============================================

/// Sreality.cz webhook payload.
#[derive(Debug, Clone, Deserialize)]
pub struct SrealityWebhook {
    /// Inquiry ID.
    pub inquiry_id: String,
    /// Estate/listing ID.
    pub estate_id: i64,
    /// Contact information.
    pub contact: SrealityContact,
    /// Message text.
    pub message: String,
    /// Event timestamp.
    pub timestamp: Option<String>,
}

/// Sreality contact information.
#[derive(Debug, Clone, Deserialize)]
pub struct SrealityContact {
    /// Contact name.
    pub name: String,
    /// Contact email.
    pub email: String,
    /// Contact phone.
    pub phone: Option<String>,
}

/// Sreality.cz webhook parser.
pub struct SrealityParser;

impl PortalParser for SrealityParser {
    fn parse(&self, body: &str) -> Result<ParsedInquiry, ParseError> {
        let data: SrealityWebhook = serde_json::from_str(body)?;

        Ok(ParsedInquiry {
            external_id: data.inquiry_id,
            property_id: data.estate_id.to_string(),
            name: data.contact.name,
            email: data.contact.email,
            phone: data.contact.phone,
            message: data.message,
            metadata: None,
        })
    }

    fn portal_type(&self) -> PortalType {
        PortalType::Sreality
    }
}

// ============================================
// Bezrealitky Parser
// ============================================

/// Bezrealitky.cz webhook payload.
#[derive(Debug, Clone, Deserialize)]
pub struct BezrealitkyWebhook {
    /// Event type.
    pub event: String,
    /// Inquiry data.
    pub data: BezrealitkyInquiryData,
}

/// Bezrealitky inquiry data.
#[derive(Debug, Clone, Deserialize)]
pub struct BezrealitkyInquiryData {
    /// Inquiry ID.
    pub id: String,
    /// Property ID.
    pub property_id: String,
    /// Sender name.
    pub sender_name: String,
    /// Sender email.
    pub sender_email: String,
    /// Sender phone.
    pub sender_phone: Option<String>,
    /// Message text.
    pub text: String,
    /// Created timestamp.
    pub created_at: Option<String>,
}

/// Bezrealitky.cz webhook parser.
pub struct BezrealitkyParser;

impl PortalParser for BezrealitkyParser {
    fn parse(&self, body: &str) -> Result<ParsedInquiry, ParseError> {
        let data: BezrealitkyWebhook = serde_json::from_str(body)?;

        Ok(ParsedInquiry {
            external_id: data.data.id,
            property_id: data.data.property_id,
            name: data.data.sender_name,
            email: data.data.sender_email,
            phone: data.data.sender_phone,
            message: data.data.text,
            metadata: None,
        })
    }

    fn portal_type(&self) -> PortalType {
        PortalType::Bezrealitky
    }
}

// ============================================
// Immowelt Parser
// ============================================

/// Immowelt.de webhook payload.
#[derive(Debug, Clone, Deserialize)]
pub struct ImmoweltWebhook {
    /// Request ID.
    #[serde(rename = "requestId")]
    pub request_id: String,
    /// Object/property ID.
    #[serde(rename = "objectId")]
    pub object_id: String,
    /// Contact data.
    pub contact: ImmoweltContact,
    /// Message.
    pub message: String,
    /// Request type.
    #[serde(rename = "requestType")]
    pub request_type: Option<String>,
}

/// Immowelt contact information.
#[derive(Debug, Clone, Deserialize)]
pub struct ImmoweltContact {
    /// Salutation (Herr/Frau).
    pub salutation: Option<String>,
    /// First name.
    #[serde(rename = "firstName")]
    pub first_name: String,
    /// Last name.
    #[serde(rename = "lastName")]
    pub last_name: String,
    /// Email.
    pub email: String,
    /// Phone.
    pub phone: Option<String>,
}

/// Immowelt.de webhook parser.
pub struct ImmoweltParser;

impl PortalParser for ImmoweltParser {
    fn parse(&self, body: &str) -> Result<ParsedInquiry, ParseError> {
        let data: ImmoweltWebhook = serde_json::from_str(body)?;

        let full_name = format!("{} {}", data.contact.first_name, data.contact.last_name);

        Ok(ParsedInquiry {
            external_id: data.request_id,
            property_id: data.object_id,
            name: full_name,
            email: data.contact.email,
            phone: data.contact.phone,
            message: data.message,
            metadata: data
                .request_type
                .map(|rt| serde_json::json!({"request_type": rt})),
        })
    }

    fn portal_type(&self) -> PortalType {
        PortalType::Immowelt
    }
}

// ============================================
// Generic Parser
// ============================================

/// Generic webhook payload for custom portals.
#[derive(Debug, Clone, Deserialize)]
pub struct GenericWebhook {
    /// Inquiry/request ID.
    pub id: Option<String>,
    /// Property/listing ID.
    pub property_id: Option<String>,
    /// Contact name.
    pub name: Option<String>,
    /// Contact email.
    pub email: String,
    /// Contact phone.
    pub phone: Option<String>,
    /// Message.
    pub message: Option<String>,
}

/// Generic webhook parser for custom portals.
pub struct GenericParser;

impl PortalParser for GenericParser {
    fn parse(&self, body: &str) -> Result<ParsedInquiry, ParseError> {
        let data: GenericWebhook = serde_json::from_str(body)?;

        Ok(ParsedInquiry {
            external_id: data.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            property_id: data.property_id.unwrap_or_default(),
            name: data.name.unwrap_or_else(|| "Unknown".to_string()),
            email: data.email,
            phone: data.phone,
            message: data.message.unwrap_or_default(),
            metadata: None,
        })
    }

    fn portal_type(&self) -> PortalType {
        PortalType::Custom
    }
}

// ============================================
// Parser Factory
// ============================================

/// Get the appropriate parser for a portal type.
pub fn get_parser(portal_type: PortalType) -> Box<dyn PortalParser> {
    match portal_type {
        PortalType::Sreality => Box::new(SrealityParser),
        PortalType::Bezrealitky => Box::new(BezrealitkyParser),
        PortalType::Immowelt => Box::new(ImmoweltParser),
        PortalType::Custom => Box::new(GenericParser),
    }
}

/// Parse a webhook body using the appropriate parser.
pub fn parse_webhook(portal_type: PortalType, body: &str) -> Result<ParsedInquiry, ParseError> {
    let parser = get_parser(portal_type);
    parser.parse(body)
}

// ============================================
// Webhook Signature Verification
// ============================================

/// Verify a webhook signature using HMAC-SHA256.
///
/// # Arguments
/// * `secret` - The webhook secret
/// * `body` - The raw request body
/// * `signature` - The signature from the header
///
/// # Returns
/// True if the signature is valid.
pub fn verify_webhook_signature(secret: &str, body: &str, signature: &str) -> bool {
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        return false;
    };

    mac.update(body.as_bytes());

    let expected = hex::encode(mac.finalize().into_bytes());
    constant_time_compare(signature, &expected)
}

/// Constant-time string comparison to prevent timing attacks.
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

/// Whether an inbound webhook's signed unix-seconds `timestamp` is within
/// `tolerance_secs` of `now_unix` in either direction — the freshness half of
/// replay protection (gap 83-3 / issue #2330).
///
/// Shared by every webhook receiver that folds a timestamp into its signed
/// payload (the connection-scoped and per-portal receivers in
/// `api-server::routes`, and `services::stripe::verify_signature`) so the
/// arithmetic lives in exactly one place.
///
/// Overflow-proof: a naive `(now_unix - timestamp).abs() > tolerance_secs`
/// **panics** under debug/overflow-checks builds (a per-request panic aborts
/// the task) and silently **wraps** in release for an adversarial `timestamp`
/// near `i64::MIN`/`i64::MAX`, because the subtraction overflows before
/// `abs()` runs. We instead compute the skew with a checked subtraction and
/// take the *unsigned* absolute value; a subtraction that overflows is treated
/// as "out of tolerance" (reject), so the freshness gate is self-contained and
/// never panics on hostile input.
pub fn timestamp_within_tolerance(now_unix: i64, timestamp: i64, tolerance_secs: i64) -> bool {
    match now_unix.checked_sub(timestamp) {
        Some(skew) => skew.unsigned_abs() <= tolerance_secs.max(0) as u64,
        None => false,
    }
}

/// Why an inbound timestamped webhook signature failed to verify.
///
/// Both variants fail closed. Receivers map every variant to an opaque
/// `401 INVALID_SIGNATURE` (the discriminant is logged, never leaked to the
/// caller) so an attacker cannot distinguish "stale" from "bad key".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimestampedSignatureError {
    /// The signed timestamp is outside the freshness window — a stale capture
    /// being replayed, or an absurd future-dated delivery.
    StaleTimestamp,
    /// The HMAC over `"{timestamp}.{body}"` did not match, the signature was
    /// malformed, or the signing secret was empty.
    BadSignature,
}

/// Verify an inbound webhook that binds a unix-seconds `timestamp` into its
/// HMAC-SHA256 signature as `"{timestamp}.{body}"`, enforcing **authenticity**
/// (the shared `secret` produced `signature`) and **freshness** (`timestamp`
/// within `tolerance_secs` of `now_unix`).
///
/// This is the single reviewed implementation of the `"{ts}.{body}"` inbound
/// scheme shared by the hex-signature receivers (the connection-scoped portal,
/// Airbnb, and Booking.com push receivers in `api-server`) — the R4 follow-up
/// of the 2026-07 webhook-hardening audit. It mirrors, in one place, the
/// construction the Stripe receiver (`services::stripe::verify_signature`) and
/// the per-portal base64 receiver implement in their provider-specific
/// encodings (those keep bespoke verifiers because their signature encodings —
/// Stripe's compound `t=/v1=` header, per-portal base64 — differ from the plain
/// hex handled here).
///
/// Fail-closed properties:
/// * an empty `secret` is rejected (`verify_webhook_signature` would otherwise
///   compute a valid HMAC under the empty key, so an unset secret must never
///   verify);
/// * a `timestamp` outside the window is rejected before the HMAC is checked,
///   using the overflow-proof [`timestamp_within_tolerance`] (an adversarial
///   `i64::MIN`/`MAX` cannot panic or wrap the gate);
/// * folding the timestamp into the signed material means a captured signature
///   cannot be re-presented with a refreshed timestamp header — the swap
///   invalidates the HMAC.
///
/// An optional `sha256=` prefix on `signature` is tolerated. The HMAC compare
/// is constant-time (delegated to [`verify_webhook_signature`]).
///
/// `now_unix` / `tolerance_secs` are injected so the freshness window is
/// unit-testable without wall-clock dependence.
pub fn verify_timestamped_signature(
    secret: &str,
    timestamp: i64,
    body: &str,
    signature: &str,
    now_unix: i64,
    tolerance_secs: i64,
) -> Result<(), TimestampedSignatureError> {
    if secret.is_empty() {
        return Err(TimestampedSignatureError::BadSignature);
    }
    if !timestamp_within_tolerance(now_unix, timestamp, tolerance_secs) {
        return Err(TimestampedSignatureError::StaleTimestamp);
    }
    let signature = signature.trim_start_matches("sha256=");
    let signed_payload = format!("{timestamp}.{body}");
    if verify_webhook_signature(secret, &signed_payload, signature) {
        Ok(())
    } else {
        Err(TimestampedSignatureError::BadSignature)
    }
}

/// Compute HMAC-SHA256 signature for a payload.
///
/// # Arguments
/// * `secret` - The webhook secret
/// * `body` - The request body to sign
///
/// # Returns
/// Hex-encoded signature.
pub fn compute_hmac_sha256(secret: &str, body: &str) -> String {
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");

    mac.update(body.as_bytes());

    hex::encode(mac.finalize().into_bytes())
}

// ============================================
// Portal Client
// ============================================

/// Generic real estate portal client for publishing listings.
#[allow(dead_code)]
pub struct PortalClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl PortalClient {
    /// Create a new portal client.
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
            api_key,
        }
    }

    /// Publish a listing to the portal.
    pub async fn publish_listing(&self, _listing_id: &str) -> Result<String, PortalError> {
        tracing::info!("Publishing listing to portal");
        Ok("external-id".to_string())
    }

    /// Sync listing updates with the portal.
    pub async fn sync_listing(&self, _external_id: &str) -> Result<(), PortalError> {
        tracing::info!("Syncing listing with portal");
        Ok(())
    }

    /// Unpublish a listing from the portal.
    pub async fn unpublish_listing(&self, _external_id: &str) -> Result<(), PortalError> {
        tracing::info!("Unpublishing listing from portal");
        Ok(())
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get the API key.
    pub fn api_key(&self) -> &str {
        &self.api_key
    }
}

// ============================================
// Tests
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- timestamp_within_tolerance (issue #2330) ----

    #[test]
    fn timestamp_within_tolerance_accepts_inside_window() {
        assert!(timestamp_within_tolerance(
            1_700_000_000,
            1_700_000_000,
            300
        ));
        assert!(timestamp_within_tolerance(
            1_700_000_000,
            1_700_000_299,
            300
        ));
        assert!(timestamp_within_tolerance(
            1_700_000_000,
            1_699_999_701,
            300
        ));
    }

    #[test]
    fn timestamp_within_tolerance_accepts_exact_boundary() {
        // The gate is inclusive (`<=`): a delivery exactly `tolerance` away is
        // still fresh in either direction.
        assert!(timestamp_within_tolerance(
            1_700_000_000,
            1_700_000_300,
            300
        ));
        assert!(timestamp_within_tolerance(
            1_700_000_000,
            1_699_999_700,
            300
        ));
    }

    #[test]
    fn timestamp_within_tolerance_rejects_outside_window() {
        assert!(!timestamp_within_tolerance(
            1_700_000_000,
            1_700_000_301,
            300
        ));
        assert!(!timestamp_within_tolerance(
            1_700_000_000,
            1_699_999_699,
            300
        ));
    }

    #[test]
    fn timestamp_within_tolerance_is_overflow_proof() {
        // Adversarial timestamps that would overflow a naive
        // `(now - ts).abs()` must be rejected, not panic (debug) or wrap
        // (release). See the function docs.
        let now = 1_700_000_000;
        assert!(!timestamp_within_tolerance(now, i64::MIN, 300));
        assert!(!timestamp_within_tolerance(now, i64::MAX, 300));
        assert!(!timestamp_within_tolerance(i64::MIN, i64::MAX, 300));
        assert!(!timestamp_within_tolerance(i64::MAX, i64::MIN, 300));
    }

    #[test]
    fn test_portal_type_from_str() {
        assert_eq!(PortalType::from_str("sreality"), Some(PortalType::Sreality));
        assert_eq!(
            PortalType::from_str("Bezrealitky"),
            Some(PortalType::Bezrealitky)
        );
        assert_eq!(PortalType::from_str("IMMOWELT"), Some(PortalType::Immowelt));
        assert_eq!(PortalType::from_str("custom"), Some(PortalType::Custom));
        assert_eq!(PortalType::from_str("unknown"), None);
    }

    #[test]
    fn test_portal_type_display() {
        assert_eq!(PortalType::Sreality.to_string(), "sreality");
        assert_eq!(PortalType::Bezrealitky.to_string(), "bezrealitky");
        assert_eq!(PortalType::Immowelt.to_string(), "immowelt");
    }

    #[test]
    fn test_sreality_parser() {
        let body = r#"{
            "inquiry_id": "INQ123",
            "estate_id": 456789,
            "contact": {
                "name": "Jan Novak",
                "email": "jan@example.com",
                "phone": "+420123456789"
            },
            "message": "I am interested in this property."
        }"#;

        let parser = SrealityParser;
        let result = parser.parse(body).unwrap();

        assert_eq!(result.external_id, "INQ123");
        assert_eq!(result.property_id, "456789");
        assert_eq!(result.name, "Jan Novak");
        assert_eq!(result.email, "jan@example.com");
        assert_eq!(result.phone, Some("+420123456789".to_string()));
    }

    #[test]
    fn test_bezrealitky_parser() {
        let body = r#"{
            "event": "inquiry.created",
            "data": {
                "id": "BZR-123",
                "property_id": "PROP-456",
                "sender_name": "Petr Svoboda",
                "sender_email": "petr@example.com",
                "sender_phone": null,
                "text": "When can I schedule a viewing?"
            }
        }"#;

        let parser = BezrealitkyParser;
        let result = parser.parse(body).unwrap();

        assert_eq!(result.external_id, "BZR-123");
        assert_eq!(result.property_id, "PROP-456");
        assert_eq!(result.name, "Petr Svoboda");
        assert_eq!(result.email, "petr@example.com");
        assert!(result.phone.is_none());
    }

    #[test]
    fn test_immowelt_parser() {
        let body = r#"{
            "requestId": "REQ-789",
            "objectId": "OBJ-123",
            "contact": {
                "salutation": "Herr",
                "firstName": "Hans",
                "lastName": "Mueller",
                "email": "hans@example.de",
                "phone": "+491234567890"
            },
            "message": "Bitte kontaktieren Sie mich.",
            "requestType": "viewing"
        }"#;

        let parser = ImmoweltParser;
        let result = parser.parse(body).unwrap();

        assert_eq!(result.external_id, "REQ-789");
        assert_eq!(result.property_id, "OBJ-123");
        assert_eq!(result.name, "Hans Mueller");
        assert_eq!(result.email, "hans@example.de");
        assert_eq!(result.phone, Some("+491234567890".to_string()));
        assert!(result.metadata.is_some());
    }

    #[test]
    fn test_webhook_signature_verification() {
        let secret = "test_secret_key";
        let body = r#"{"test": "data"}"#;

        // Compute expected signature
        let expected_sig = compute_hmac_sha256(secret, body);

        // Verify should pass with correct signature
        assert!(verify_webhook_signature(secret, body, &expected_sig));

        // Verify should fail with wrong signature
        assert!(!verify_webhook_signature(secret, body, "wrong_signature"));

        // Verify should fail with wrong secret
        assert!(!verify_webhook_signature(
            "wrong_secret",
            body,
            &expected_sig
        ));
    }

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("hello", "hello"));
        assert!(!constant_time_compare("hello", "world"));
        assert!(!constant_time_compare("hello", "hell"));
        assert!(!constant_time_compare("a", "ab"));
    }

    #[test]
    fn test_get_parser() {
        let parser = get_parser(PortalType::Sreality);
        assert_eq!(parser.portal_type(), PortalType::Sreality);

        let parser = get_parser(PortalType::Bezrealitky);
        assert_eq!(parser.portal_type(), PortalType::Bezrealitky);

        let parser = get_parser(PortalType::Immowelt);
        assert_eq!(parser.portal_type(), PortalType::Immowelt);
    }

    #[test]
    fn test_inquiry_status_serialization() {
        let status = InquiryStatus::New;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"new\"");

        let parsed: InquiryStatus = serde_json::from_str("\"replied\"").unwrap();
        assert_eq!(parsed, InquiryStatus::Replied);
    }

    #[test]
    fn test_portal_connection_webhook_url() {
        let connection = PortalConnection {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            organization_id: Uuid::new_v4(),
            portal_type: PortalType::Sreality,
            name: "Test Connection".to_string(),
            webhook_secret: "secret".to_string(),
            is_active: true,
            last_webhook_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let url = connection.webhook_url("https://api.example.com");
        assert_eq!(
            url,
            "https://api.example.com/api/v1/webhooks/portal/550e8400-e29b-41d4-a716-446655440000"
        );
    }

    // ---- verify_timestamped_signature (audit R4 shared helper) ----

    const TS_SECRET: &str = "shared_ts_secret";
    const TS_BODY: &str = r#"{"event":"reservation.created","id":"r1"}"#;
    const TS_NOW: i64 = 1_700_000_000;
    const TS_TOL: i64 = 300;

    fn ts_sign(secret: &str, ts: i64, body: &str) -> String {
        compute_hmac_sha256(secret, &format!("{ts}.{body}"))
    }

    #[test]
    fn timestamped_sig_accepts_valid() {
        let sig = ts_sign(TS_SECRET, TS_NOW, TS_BODY);
        assert_eq!(
            verify_timestamped_signature(TS_SECRET, TS_NOW, TS_BODY, &sig, TS_NOW, TS_TOL),
            Ok(())
        );
    }

    #[test]
    fn timestamped_sig_accepts_sha256_prefix() {
        let sig = format!("sha256={}", ts_sign(TS_SECRET, TS_NOW, TS_BODY));
        assert_eq!(
            verify_timestamped_signature(TS_SECRET, TS_NOW, TS_BODY, &sig, TS_NOW, TS_TOL),
            Ok(())
        );
    }

    #[test]
    fn timestamped_sig_rejects_empty_secret() {
        // An unset secret must never verify, even under a syntactically valid
        // signature computed with the empty key.
        let sig = ts_sign("", TS_NOW, TS_BODY);
        assert_eq!(
            verify_timestamped_signature("", TS_NOW, TS_BODY, &sig, TS_NOW, TS_TOL),
            Err(TimestampedSignatureError::BadSignature)
        );
    }

    #[test]
    fn timestamped_sig_rejects_wrong_secret() {
        let sig = ts_sign("attacker", TS_NOW, TS_BODY);
        assert_eq!(
            verify_timestamped_signature(TS_SECRET, TS_NOW, TS_BODY, &sig, TS_NOW, TS_TOL),
            Err(TimestampedSignatureError::BadSignature)
        );
    }

    #[test]
    fn timestamped_sig_rejects_tampered_body() {
        let sig = ts_sign(TS_SECRET, TS_NOW, TS_BODY);
        let tampered = r#"{"event":"reservation.created","id":"evil"}"#;
        assert_eq!(
            verify_timestamped_signature(TS_SECRET, TS_NOW, tampered, &sig, TS_NOW, TS_TOL),
            Err(TimestampedSignatureError::BadSignature)
        );
    }

    #[test]
    fn timestamped_sig_rejects_stale_and_future() {
        for ts in [TS_NOW - TS_TOL - 1, TS_NOW + TS_TOL + 1] {
            let sig = ts_sign(TS_SECRET, ts, TS_BODY);
            assert_eq!(
                verify_timestamped_signature(TS_SECRET, ts, TS_BODY, &sig, TS_NOW, TS_TOL),
                Err(TimestampedSignatureError::StaleTimestamp),
                "timestamp {ts} must be rejected as stale"
            );
        }
    }

    #[test]
    fn timestamped_sig_accepts_exact_boundary() {
        for ts in [TS_NOW - TS_TOL, TS_NOW + TS_TOL] {
            let sig = ts_sign(TS_SECRET, ts, TS_BODY);
            assert_eq!(
                verify_timestamped_signature(TS_SECRET, ts, TS_BODY, &sig, TS_NOW, TS_TOL),
                Ok(())
            );
        }
    }

    #[test]
    fn timestamped_sig_rejects_adversarial_overflow_without_panic() {
        // Regression for issue #2330: an adversarial timestamp near i64::MIN/MAX
        // must reject as stale, never panic under overflow-checks builds.
        for ts in [i64::MIN, i64::MIN + 1, i64::MAX, i64::MAX - 1] {
            let sig = ts_sign(TS_SECRET, ts, TS_BODY);
            assert_eq!(
                verify_timestamped_signature(TS_SECRET, ts, TS_BODY, &sig, TS_NOW, TS_TOL),
                Err(TimestampedSignatureError::StaleTimestamp)
            );
        }
    }

    #[test]
    fn timestamped_sig_rejects_timestamp_swap_on_captured_signature() {
        // Core replay defense: a captured (old_ts, sig) pair re-presented with a
        // fresh timestamp is rejected because ts is folded into the HMAC.
        let old_ts = TS_NOW - 10_000;
        let sig = ts_sign(TS_SECRET, old_ts, TS_BODY);
        assert_eq!(
            verify_timestamped_signature(TS_SECRET, TS_NOW, TS_BODY, &sig, TS_NOW, TS_TOL),
            Err(TimestampedSignatureError::BadSignature)
        );
    }
}
