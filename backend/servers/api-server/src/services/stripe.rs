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

/// The number of minor units in one major unit of `currency`, per Stripe's
/// currency table. Most currencies are two-decimal (×100), but a handful are
/// zero-decimal (charged as whole units, ×1) or three-decimal (×1000). Using a
/// flat ×100 would charge a JPY invoice 100× and a KWD invoice 0.1×.
///
/// The match is on the uppercase ISO-4217 code; unknown codes default to the
/// two-decimal majority. Lists mirror Stripe's documented zero-decimal and
/// three-decimal currency sets.
pub fn minor_unit_factor(currency: &str) -> i64 {
    match currency.to_uppercase().as_str() {
        // Zero-decimal currencies — the amount is already in the smallest unit.
        "BIF" | "CLP" | "DJF" | "GNF" | "JPY" | "KMF" | "KRW" | "MGA" | "PYG" | "RWF" | "UGX"
        | "VND" | "VUV" | "XAF" | "XOF" | "XPF" => 1,
        // Three-decimal currencies.
        "BHD" | "IQD" | "JOD" | "KWD" | "LYD" | "OMR" | "TND" => 1000,
        // Two-decimal majority (EUR, USD, GBP, …).
        _ => 100,
    }
}

/// Create a Stripe Checkout Session for `amount` (in `currency`) tied to an
/// invoice. `amount` is the invoice currency amount (e.g. 12.34 EUR); Stripe
/// expects the smallest currency unit, so we scale by the per-currency factor
/// from [`minor_unit_factor`] (×100 for two-decimal currencies, ×1 for
/// zero-decimal, ×1000 for three-decimal).
///
/// `client_reference_id`/`metadata.invoice_id` carry our invoice id so the
/// webhook (and any manual reconciliation) can correlate the session back to
/// the invoice without trusting client input.
///
/// The request carries an `Idempotency-Key` derived from `invoice_id` so a
/// retried/double-submitted checkout for the same invoice does not mint
/// duplicate Stripe sessions.
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

    // Smallest currency unit. Scale by the per-currency factor (×100 for most,
    // ×1 zero-decimal, ×1000 three-decimal) and round to avoid sub-unit
    // fractions.
    let factor = minor_unit_factor(currency);
    let unit_amount = (amount * Decimal::from(factor))
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
        // Idempotency-Key so a retried/double-submitted checkout for the same
        // invoice reuses the existing Stripe session instead of minting a new
        // one (Stripe replays the original response for a matching key).
        .header("Idempotency-Key", format!("checkout-{invoice_id}"))
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
    // Overflow-proof (issue #2330): a naive `(now_unix - timestamp).abs()`
    // panics (debug) / wraps (release) for an adversarial `t` near i64::MIN;
    // the shared helper computes the skew with a checked subtraction. Shared
    // with the portal receivers so all three gates behave identically.
    if !integrations::portals::timestamp_within_tolerance(now_unix, timestamp, tolerance_secs) {
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
    /// Total Stripe actually collected, in the smallest currency unit. Used as
    /// a defensive cross-check against our stored session amount before
    /// settling. Absent on older/partial payloads, so it is optional.
    #[serde(default)]
    pub amount_total: Option<i64>,
    /// Lowercase ISO-4217 currency Stripe collected in. Cross-checked against
    /// the stored session currency. Optional for back-compat.
    #[serde(default)]
    pub currency: Option<String>,
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
    fn adversarial_timestamp_does_not_panic_and_is_rejected() {
        // Regression for issue #2330: a parseable but adversarial `t` near
        // i64::MIN/MAX must be rejected by the freshness gate WITHOUT panicking
        // under overflow-checks builds (the old `(now - t).abs()` overflowed).
        let body = br#"{"a":1}"#;
        for t in [i64::MIN, i64::MIN + 1, i64::MAX, i64::MAX - 1] {
            let header = format!("t={t},v1=deadbeef");
            assert_eq!(
                verify_signature(body, &header, "whsec_test", 1_000_000, 300),
                Err(SignatureError::TimestampOutOfTolerance),
                "adversarial timestamp {t} must be rejected as out-of-tolerance, not panic"
            );
        }
    }

    #[test]
    fn timestamp_at_exact_tolerance_boundary_passes() {
        // The freshness gate is inclusive; pin it so an off-by-one refactor
        // (`<=` vs `<`) is caught.
        let body = br#"{"a":1}"#;
        let header = sign(body, "whsec_test", 1_000_000);
        assert_eq!(
            verify_signature(body, &header, "whsec_test", 1_000_300, 300),
            Ok(())
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
            "data":{"object":{"id":"cs_test_123","payment_status":"paid","payment_intent":"pi_abc","amount_total":10000,"currency":"eur"}}
        }"#;
        let ev = parse_event(body).unwrap();
        assert_eq!(ev.event_type, "checkout.session.completed");
        assert_eq!(ev.data.object.id, "cs_test_123");
        assert_eq!(ev.data.object.payment_status.as_deref(), Some("paid"));
        assert_eq!(ev.data.object.payment_intent.as_deref(), Some("pi_abc"));
        assert_eq!(ev.data.object.amount_total, Some(10000));
        assert_eq!(ev.data.object.currency.as_deref(), Some("eur"));
    }

    #[test]
    fn parse_event_without_amount_fields_is_back_compat() {
        // Payloads that omit amount_total/currency must still parse (the fields
        // are optional and default to None) so settlement stays unchanged.
        let body = br#"{
            "type":"checkout.session.completed",
            "data":{"object":{"id":"cs_test_456","payment_status":"paid"}}
        }"#;
        let ev = parse_event(body).unwrap();
        assert_eq!(ev.data.object.amount_total, None);
        assert_eq!(ev.data.object.currency, None);
    }

    #[test]
    fn minor_unit_factor_two_decimal_default() {
        // The two-decimal majority and unknown codes both scale by 100.
        assert_eq!(minor_unit_factor("EUR"), 100);
        assert_eq!(minor_unit_factor("USD"), 100);
        assert_eq!(minor_unit_factor("GBP"), 100);
        assert_eq!(minor_unit_factor("ZZZ"), 100);
    }

    #[test]
    fn minor_unit_factor_zero_decimal() {
        // Zero-decimal currencies are charged as whole units (×1) — a flat ×100
        // would overcharge a JPY invoice 100×.
        assert_eq!(minor_unit_factor("JPY"), 1);
        assert_eq!(minor_unit_factor("KRW"), 1);
        assert_eq!(minor_unit_factor("HUF"), 100); // HUF is two-decimal on Stripe.
        assert_eq!(minor_unit_factor("VND"), 1);
    }

    #[test]
    fn minor_unit_factor_three_decimal() {
        assert_eq!(minor_unit_factor("KWD"), 1000);
        assert_eq!(minor_unit_factor("BHD"), 1000);
        assert_eq!(minor_unit_factor("TND"), 1000);
    }

    #[test]
    fn minor_unit_factor_is_case_insensitive() {
        assert_eq!(minor_unit_factor("jpy"), 1);
        assert_eq!(minor_unit_factor("kwd"), 1000);
        assert_eq!(minor_unit_factor("eur"), 100);
    }
}
