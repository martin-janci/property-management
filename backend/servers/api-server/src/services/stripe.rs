//! Stripe Checkout integration (Story 11.5, [BIT-181]).
//!
//! We use Stripe **hosted Checkout** so raw card data never touches our
//! servers (PCI SAQ-A). The flow is:
//!
//! 1. The server creates a Checkout Session via the Stripe API
//!    ([`create_checkout_session`]) and returns the hosted `checkout_url`.
//! 2. The payer completes payment on Stripe's hosted page and is redirected
//!    back to `success_url`/`cancel_url`.
//! 3. Stripe POSTs a `checkout.session.completed` webhook, whose signature we
//!    verify ([`verify_signature`]) before settling the invoice.
//!
//! Secrets live on [`crate::state::StripeAppConfig`] (loaded from env at boot),
//! never in request bodies, and never logged.

use hmac::{Hmac, KeyInit, Mac};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::Deserialize;
use sha2::Sha256;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

/// Stripe's API base. Overridable in tests via `STRIPE_API_BASE` so the
/// outbound call can be pointed at a local mock; defaults to the live API.
fn api_base() -> String {
    std::env::var("STRIPE_API_BASE").unwrap_or_else(|_| "https://api.stripe.com".to_string())
}

/// Default tolerance (seconds) for the webhook timestamp, matching Stripe's
/// recommended replay-protection window of 5 minutes.
pub const DEFAULT_SIGNATURE_TOLERANCE_SECS: i64 = 300;

/// Error creating a Checkout Session against the Stripe API.
#[derive(Debug)]
pub enum CheckoutError {
    /// The configured secret key is empty — the integration is not configured.
    NotConfigured,
    /// Transport-level failure talking to Stripe.
    Transport(String),
    /// Stripe returned a non-2xx response (status + body excerpt).
    Api { status: u16, body: String },
    /// The Stripe response could not be parsed.
    Decode(String),
}

impl std::fmt::Display for CheckoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckoutError::NotConfigured => write!(f, "Stripe secret key is not configured"),
            CheckoutError::Transport(e) => write!(f, "Stripe transport error: {e}"),
            CheckoutError::Api { status, .. } => write!(f, "Stripe API error (status {status})"),
            CheckoutError::Decode(e) => write!(f, "Stripe response decode error: {e}"),
        }
    }
}

/// The fields we need from a created Checkout Session.
#[derive(Debug, Clone, Deserialize)]
pub struct CreatedCheckoutSession {
    /// Stripe Checkout Session id (`cs_...`). Stored as `session_id`.
    pub id: String,
    /// Hosted checkout page the payer is redirected to.
    pub url: String,
}

/// Create a Stripe Checkout Session for `amount` (in `currency`) tied to an
/// invoice. `amount` is the invoice currency amount (e.g. 12.34 EUR); Stripe
/// expects the smallest currency unit (cents), so we multiply by 100.
///
/// `client_reference_id`/`metadata.invoice_id` carry our invoice id so the
/// webhook (and any manual reconciliation) can correlate the session back to
/// the invoice without trusting client input.
#[allow(clippy::too_many_arguments)]
pub async fn create_checkout_session(
    secret_key: &str,
    success_url: &str,
    cancel_url: &str,
    invoice_id: &str,
    organization_id: &str,
    invoice_number: &str,
    amount: Decimal,
    currency: &str,
) -> Result<CreatedCheckoutSession, CheckoutError> {
    if secret_key.is_empty() {
        return Err(CheckoutError::NotConfigured);
    }

    // Smallest currency unit (cents). Round to avoid sub-cent fractions.
    let unit_amount = (amount * Decimal::from(100))
        .round()
        .to_i64()
        .ok_or_else(|| CheckoutError::Decode("amount out of range".to_string()))?;

    let product_name = format!("Invoice {invoice_number}");
    let form: Vec<(&str, String)> = vec![
        ("mode", "payment".to_string()),
        ("success_url", success_url.to_string()),
        ("cancel_url", cancel_url.to_string()),
        ("client_reference_id", invoice_id.to_string()),
        ("metadata[invoice_id]", invoice_id.to_string()),
        ("metadata[organization_id]", organization_id.to_string()),
        ("line_items[0][quantity]", "1".to_string()),
        (
            "line_items[0][price_data][currency]",
            currency.to_lowercase(),
        ),
        (
            "line_items[0][price_data][unit_amount]",
            unit_amount.to_string(),
        ),
        (
            "line_items[0][price_data][product_data][name]",
            product_name,
        ),
    ];

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/v1/checkout/sessions", api_base()))
        .bearer_auth(secret_key)
        .form(&form)
        .send()
        .await
        .map_err(|e| CheckoutError::Transport(e.to_string()))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| CheckoutError::Transport(e.to_string()))?;

    if !status.is_success() {
        // Truncate so a verbose Stripe error never floods logs / responses.
        let excerpt: String = body.chars().take(500).collect();
        return Err(CheckoutError::Api {
            status: status.as_u16(),
            body: excerpt,
        });
    }

    serde_json::from_str::<CreatedCheckoutSession>(&body)
        .map_err(|e| CheckoutError::Decode(e.to_string()))
}

/// Why a webhook signature was rejected. All variants fail the request closed.
#[derive(Debug, PartialEq, Eq)]
pub enum SignatureError {
    /// The `Stripe-Signature` header was missing a `t=` or `v1=` component.
    Malformed,
    /// The `t=` timestamp was outside the tolerance window (replay defense).
    TimestampOutOfTolerance,
    /// No `v1=` signature matched the computed HMAC.
    NoMatch,
}

/// Verify a Stripe webhook signature over the **raw** request body.
///
/// Implements Stripe's scheme: the `Stripe-Signature` header is a list of
/// `key=value` pairs (e.g. `t=1690000000,v1=abc...,v1=def...`). The signed
/// payload is `"{t}.{raw_body}"`; the expected value is its HMAC-SHA256 keyed
/// by the webhook signing secret, hex-encoded. The header is valid if **any**
/// `v1` matches (constant-time compare) and `t` is within `tolerance_secs` of
/// `now_unix`.
///
/// `now_unix`/`tolerance_secs` are injected so the timestamp window is unit
/// testable without wall-clock dependence.
pub fn verify_signature(
    payload: &[u8],
    signature_header: &str,
    secret: &str,
    now_unix: i64,
    tolerance_secs: i64,
) -> Result<(), SignatureError> {
    let mut timestamp: Option<i64> = None;
    let mut v1_sigs: Vec<&str> = Vec::new();

    for part in signature_header.split(',') {
        let mut kv = part.splitn(2, '=');
        match (kv.next(), kv.next()) {
            (Some("t"), Some(t)) => timestamp = t.trim().parse::<i64>().ok(),
            (Some("v1"), Some(v)) => v1_sigs.push(v.trim()),
            _ => {}
        }
    }

    let timestamp = timestamp.ok_or(SignatureError::Malformed)?;
    if v1_sigs.is_empty() {
        return Err(SignatureError::Malformed);
    }

    // Replay defense: reject deliveries whose timestamp is too far from now.
    if (now_unix - timestamp).abs() > tolerance_secs {
        return Err(SignatureError::TimestampOutOfTolerance);
    }

    // signed_payload = "{timestamp}.{raw_body}"
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).map_err(|_| SignatureError::Malformed)?;
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(payload);
    let expected = mac.finalize().into_bytes();
    let expected_hex = hex::encode(expected);
    let expected_bytes = expected_hex.as_bytes();

    let matched = v1_sigs.iter().any(|candidate| {
        let candidate = candidate.as_bytes();
        candidate.len() == expected_bytes.len() && candidate.ct_eq(expected_bytes).unwrap_u8() == 1
    });

    if matched {
        Ok(())
    } else {
        Err(SignatureError::NoMatch)
    }
}

/// The subset of a Stripe event we act on. Stripe sends a large envelope; we
/// only deserialize the fields we use so unknown fields are ignored.
#[derive(Debug, Clone, Deserialize)]
pub struct StripeEvent {
    /// e.g. `"checkout.session.completed"`.
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: StripeEventData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StripeEventData {
    pub object: StripeCheckoutObject,
}

/// The Checkout Session object carried by `checkout.session.*` events.
#[derive(Debug, Clone, Deserialize)]
pub struct StripeCheckoutObject {
    /// Checkout Session id (`cs_...`) — matches our stored `session_id`.
    pub id: String,
    /// `"paid"`, `"unpaid"`, or `"no_payment_required"`.
    #[serde(default)]
    pub payment_status: Option<String>,
    /// The PaymentIntent id (`pi_...`), used as the gateway reference.
    #[serde(default)]
    pub payment_intent: Option<String>,
}

/// Parse a Stripe event from a raw (already signature-verified) body.
pub fn parse_event(payload: &[u8]) -> Result<StripeEvent, String> {
    serde_json::from_slice::<StripeEvent>(payload).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a valid `Stripe-Signature` header for `payload` at `ts`.
    fn sign(payload: &[u8], secret: &str, ts: i64) -> String {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(ts.to_string().as_bytes());
        mac.update(b".");
        mac.update(payload);
        let sig = hex::encode(mac.finalize().into_bytes());
        format!("t={ts},v1={sig}")
    }

    #[test]
    fn valid_signature_passes() {
        let body = br#"{"type":"checkout.session.completed"}"#;
        let header = sign(body, "whsec_test", 1_000_000);
        assert_eq!(
            verify_signature(body, &header, "whsec_test", 1_000_000, 300),
            Ok(())
        );
    }

    #[test]
    fn tampered_body_fails() {
        let body = br#"{"type":"checkout.session.completed"}"#;
        let header = sign(body, "whsec_test", 1_000_000);
        let tampered = br#"{"type":"checkout.session.completed","x":1}"#;
        assert_eq!(
            verify_signature(tampered, &header, "whsec_test", 1_000_000, 300),
            Err(SignatureError::NoMatch)
        );
    }

    #[test]
    fn wrong_secret_fails() {
        let body = br#"{"a":1}"#;
        let header = sign(body, "whsec_real", 1_000_000);
        assert_eq!(
            verify_signature(body, &header, "whsec_attacker", 1_000_000, 300),
            Err(SignatureError::NoMatch)
        );
    }

    #[test]
    fn stale_timestamp_fails() {
        let body = br#"{"a":1}"#;
        let header = sign(body, "whsec_test", 1_000_000);
        // now is 10 minutes after the signed timestamp, tolerance 5 minutes.
        assert_eq!(
            verify_signature(body, &header, "whsec_test", 1_000_600, 300),
            Err(SignatureError::TimestampOutOfTolerance)
        );
    }

    #[test]
    fn malformed_header_fails() {
        let body = br#"{"a":1}"#;
        assert_eq!(
            verify_signature(body, "garbage", "whsec_test", 1_000_000, 300),
            Err(SignatureError::Malformed)
        );
        assert_eq!(
            verify_signature(body, "t=1000000", "whsec_test", 1_000_000, 300),
            Err(SignatureError::Malformed)
        );
    }

    #[test]
    fn multiple_v1_one_matching_passes() {
        let body = br#"{"a":1}"#;
        let good = sign(body, "whsec_test", 1_000_000);
        // Prepend a bogus v1; Stripe sends several during secret rotation.
        let header = format!("{good},v1=deadbeef");
        assert_eq!(
            verify_signature(body, &header, "whsec_test", 1_000_000, 300),
            Ok(())
        );
    }

    #[test]
    fn parse_checkout_completed_event() {
        let body = br#"{
            "type":"checkout.session.completed",
            "data":{"object":{"id":"cs_test_123","payment_status":"paid","payment_intent":"pi_abc"}}
        }"#;
        let ev = parse_event(body).unwrap();
        assert_eq!(ev.event_type, "checkout.session.completed");
        assert_eq!(ev.data.object.id, "cs_test_123");
        assert_eq!(ev.data.object.payment_status.as_deref(), Some("paid"));
        assert_eq!(ev.data.object.payment_intent.as_deref(), Some("pi_abc"));
    }
}
