//! Integration webhook-surface routes.
//!
//! Covers two concerns:
//! 1. **Outbound webhook management** — CRUD for webhook subscriptions,
//!    test delivery, and delivery logs (Story 61.5). UNMOUNTED (PAP-122):
//!    the backing schema exists in no migration; see `router()` for the
//!    remount conditions. The live subscription surface is the
//!    enhanced-webhook CRUD in `routes/api_ecosystem.rs`.
//! 2. **Inbound webhook receivers** — endpoints that external systems POST to:
//!    - E-signature providers (DocuSign, Adobe Sign, HelloSign) — UNMOUNTED
//!      (PAP-122), writes the migration-less `esignature_workflows` table
//!    - Booking.com OTA push notifications
//!    - Portal/listing-site webhooks (public, no auth)
//!
//! # RLS routing (PAP-105 / PAP-80)
//!
//! `webhook_subscriptions` runs under `FORCE ROW LEVEL SECURITY` (migration
//! `00179`), so every outbound-subscription handler acquires an
//! [`RlsConnection`] and runs its queries on that context-set connection; the
//! `{org_id}` path segment must equal `rls.tenant_id()` so the SQL org filter
//! and the policy can never disagree, and by-id reads of another tenant's
//! subscription resolve to `None` → `404` via RLS. The inbound receivers have
//! no request principal: the e-signature receiver writes the non-FORCE
//! `esignature_workflows` table on the pool (scoped by the provider-signed
//! envelope id), and must NEVER touch `webhook_subscriptions` that way.
//! Every authenticated path calls `rls.release().await` before returning.

mod airbnb;
mod booking;
mod esignature;
mod payment;
mod portal;
mod subscriptions;

pub use airbnb::*;
pub use booking::*;
pub use esignature::*;
pub use payment::*;
pub use portal::*;
pub use subscriptions::*;

use axum::{routing::post, Router};

use crate::state::AppState;

// ==================== Router ====================

/// Create webhook-surface router.
pub fn router() -> Router<AppState> {
    Router::new()
        // ROADMAP(PAP-122): outbound webhook-subscription CRUD (Story 61.5)
        // unmounted — the `IntegrationRepository` SQL behind it expects
        // `status` / `retry_policy` columns and a `webhook_delivery_logs`
        // table that exist in no migration, and it duplicates the live,
        // schema-aligned enhanced-webhook surface in `routes/api_ecosystem.rs`
        // (which is what `/organizations/{org_id}/webhooks` should keep
        // serving). Remount only after the Epic-61 migrations land AND the
        // two surfaces are reconciled onto one column convention.
        //
        // ROADMAP(PAP-122): the e-signature inbound receiver is unmounted with
        // the rest of the e-signature surface — its handler writes
        // `esignature_workflows`, which exists in no migration.
        //
        // Inbound webhook receivers (live)
        .route("/booking/push", post(booking_push_notification))
        .route(
            "/webhooks/portal/{connection_id}",
            post(handle_portal_webhook),
        )
        // Gap 83-1: Airbnb inbound webhook
        .route("/airbnb/webhook", post(handle_airbnb_webhook))
        // Story 11.5 (BIT-181): payment-gateway confirmation webhook
        .route(
            "/webhooks/payments/{provider}",
            post(handle_payment_webhook),
        )
}
