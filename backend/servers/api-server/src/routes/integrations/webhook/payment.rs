//! Inbound payment-gateway (Stripe) confirmation webhook (Story 11.5, BIT-181).
//!
//! Split out of the former monolithic `webhook.rs` (behaviour preserved).

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};

use common::errors::ErrorResponse;

use crate::state::AppState;

// ==================== Payment-gateway webhook (Story 11.5, BIT-181) ====================

/// Inbound payment-gateway confirmation webhook.
///
/// `POST /api/v1/integrations/webhooks/payments/{provider}` (currently only
/// `stripe`). Mirrors the other inbound receivers: load the signing secret
/// from `AppState` (never the request), **fail closed** when it is unset,
/// verify the signature over the raw body before parsing, and acknowledge
/// idempotently.
///
/// On a verified `checkout.session.completed` with `payment_status == "paid"`,
/// the originating `online_payment_sessions` row is looked up by its Stripe
/// session id, the invoice is settled via the gateway-settlement path (records
/// a `payments` row + allocation; the DB trigger marks the invoice `paid`), and
/// the session is marked `completed`. Already-completed sessions are a no-op so
/// Stripe's at-least-once retries cannot double-settle.
///
/// Tenant isolation: the session id is the only attacker-influenced input and
/// it is resolved server-side to its owning organization/invoice — no org id is
/// trusted from the payload, and a forged session id that matches nothing is
/// acknowledged without side effects.
pub async fn handle_payment_webhook(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    use crate::services::stripe;

    // Only Stripe is wired today; reject unknown providers explicitly.
    if provider != "stripe" {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "UNKNOWN_PROVIDER",
                "Unsupported payment provider",
            )),
        ));
    }

    // Fail closed when the signing secret is not configured.
    let secret = state.stripe_config.webhook_secret.as_str();
    if secret.is_empty() {
        tracing::error!("STRIPE_WEBHOOK_SECRET is not configured — refusing webhook");
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new(
                "NOT_CONFIGURED",
                "Payment webhook secret is not configured",
            )),
        ));
    }

    let signature = headers
        .get("Stripe-Signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    let now_unix = chrono::Utc::now().timestamp();
    if let Err(e) = stripe::verify_signature(
        &body,
        signature,
        secret,
        now_unix,
        stripe::DEFAULT_SIGNATURE_TOLERANCE_SECS,
    ) {
        tracing::warn!(?e, "Stripe webhook signature verification failed");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "INVALID_SIGNATURE",
                "Webhook signature verification failed",
            )),
        ));
    }

    let event = stripe::parse_event(&body).map_err(|e| {
        tracing::warn!(error = %e, "Failed to parse Stripe webhook event");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("PARSE_ERROR", "Invalid webhook payload")),
        )
    })?;

    // Only settlement events are actionable; acknowledge everything else so
    // Stripe does not retry events we deliberately ignore.
    if event.event_type != "checkout.session.completed" {
        return Ok(StatusCode::OK);
    }
    if event.data.object.payment_status.as_deref() != Some("paid") {
        return Ok(StatusCode::OK);
    }

    let provider_session_id = &event.data.object.id;
    let session = match state
        .financial_repo
        .get_payment_session_by_provider_id(provider_session_id)
        .await
    {
        Ok(Some(s)) => s,
        Ok(None) => {
            // Unknown session — ack so Stripe stops retrying; nothing to do.
            tracing::warn!(session_id = %provider_session_id, "Stripe webhook for unknown session");
            return Ok(StatusCode::OK);
        }
        Err(e) => {
            tracing::error!("Failed to load payment session: {:?}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to load payment session",
                )),
            ));
        }
    };

    // Idempotency: a session already settled is a no-op (Stripe retries).
    if session.status == "completed" {
        return Ok(StatusCode::OK);
    }

    // Defense-in-depth: settlement uses our server-stored `session.amount`/
    // `currency` (never a client-supplied amount), but if the amount Stripe
    // reports collecting disagrees with what we recorded, something is wrong —
    // refuse to settle and ack (200) so Stripe stops retrying a mismatched
    // event. Older payloads may omit these fields; when absent we keep the
    // prior behaviour and settle on the stored amount.
    {
        use rust_decimal::prelude::ToPrimitive;
        use rust_decimal::Decimal;

        let factor = stripe::minor_unit_factor(&session.currency);
        let expected_minor = (session.amount * Decimal::from(factor)).round().to_i64();

        if let (Some(expected), Some(reported)) = (expected_minor, event.data.object.amount_total) {
            if expected != reported {
                tracing::warn!(
                    session_id = %provider_session_id,
                    expected_minor = expected,
                    reported_minor = reported,
                    "Stripe webhook amount_total disagrees with stored session amount — not settling"
                );
                return Ok(StatusCode::OK);
            }
        }

        if let Some(reported_currency) = event.data.object.currency.as_deref() {
            if !reported_currency.eq_ignore_ascii_case(&session.currency) {
                tracing::warn!(
                    session_id = %provider_session_id,
                    expected_currency = %session.currency,
                    reported_currency = %reported_currency,
                    "Stripe webhook currency disagrees with stored session currency — not settling"
                );
                return Ok(StatusCode::OK);
            }
        }
    }

    let invoice = match state.financial_repo.get_invoice(session.invoice_id).await {
        Ok(Some(inv)) => inv,
        Ok(None) => {
            tracing::error!(invoice_id = %session.invoice_id, "Webhook session references a missing invoice");
            return Ok(StatusCode::OK);
        }
        Err(e) => {
            tracing::error!("Failed to load invoice for webhook: {:?}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to load invoice")),
            ));
        }
    };

    // Prefer the PaymentIntent id as the gateway reference; fall back to the
    // Checkout Session id.
    let gateway_ref = event
        .data
        .object
        .payment_intent
        .clone()
        .unwrap_or_else(|| session.session_id.clone());

    let settled = state
        .financial_repo
        .settle_invoice_from_gateway(&session, &invoice, &gateway_ref)
        .await
        .map_err(|e| {
            tracing::error!("Failed to settle invoice from gateway webhook: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to settle invoice")),
            )
        })?;

    match settled {
        Some(_) => tracing::info!(
            invoice_id = %invoice.id,
            session_id = %session.session_id,
            "Invoice settled from Stripe Checkout webhook"
        ),
        // The atomic idempotency claim found the session already settled by a
        // concurrent delivery. Ack so Stripe stops retrying; no double-credit.
        None => tracing::info!(
            session_id = %session.session_id,
            "Stripe webhook duplicate: session already settled, no-op"
        ),
    }

    Ok(StatusCode::OK)
}
