//! Subscription and billing repository (Epic 26).
//!
//! # RLS Integration (PAP-112 / PAP-80 / PAP-67)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on the seven org-scoped billing tables this
//! repo touches (`organization_subscriptions`, `payment_methods`,
//! `subscription_invoices`, `invoice_line_items`, `usage_records`,
//! `subscription_events`, `coupon_redemptions`). Under `FORCE` the
//! api-server's owner connection is no longer exempt, so a query issued on a
//! connection without `app.current_org_id` set collapses to deny-all:
//! own-org reads return empty and writes fail the policy `WITH CHECK`.
//!
//! Every method therefore takes an **executor whose connection already has
//! RLS context set** (org + user GUCs) — in handlers this comes from the
//! `RlsConnection` extractor via `&mut **rls.conn()`. The repository holds
//! **no pool**, so there is no way to issue a query that bypasses RLS. This
//! mirrors the `work_order.rs` / `vendor.rs` / `llm_document.rs` precedent.
//!
//! Single-statement methods take a generic [`Executor`](sqlx::Executor);
//! multi-statement methods (`create_subscription`, `create_invoice`,
//! `get_statistics`) and the transactional ones (`set_default_payment_method`,
//! `redeem_coupon`) take `&mut PgConnection` and reborrow. The transactions run
//! on the context-set connection: `set_request_context` sets session-level
//! GUCs, so the RLS context survives `BEGIN`/`COMMIT`.
//!
//! `subscription_plans` and `subscription_coupons` are **not** FORCE-bound
//! (they carry public read policies — plans/coupons are platform-global, not
//! tenant data), so callers without a tenant principal (the public
//! `/plans/public` endpoint) may run those read methods on a plain pool
//! executor.
//!
//! # Module layout
//!
//! The repository surface is large (plans, subscriptions, payment methods,
//! invoices, usage, events, coupons and statistics). To keep each area
//! readable and to reduce the churn surface of any single file, the
//! `impl SubscriptionRepository` block is split across cohesive sub-modules.
//! Every sub-module adds methods to the *same* [`SubscriptionRepository`]
//! struct defined here:
//!
//! - [`plans`]          — subscription plan CRUD (incl. public listing)
//! - [`subscriptions`]  — organization subscription CRUD & lifecycle
//! - [`payment_methods`]— payment method CRUD & default selection
//! - [`invoices`]       — invoice CRUD, listing & line items
//! - [`usage`]          — usage records & current-usage counts
//! - [`events`]         — subscription event log
//! - [`coupons`]        — coupon CRUD & redemption
//! - [`statistics`]     — aggregate subscription statistics

use sqlx::PgPool;

mod coupons;
mod events;
mod invoices;
mod payment_methods;
mod plans;
mod statistics;
mod subscriptions;
mod usage;

/// Repository for subscription and billing operations.
///
/// Stateless: every method receives an RLS-context-bearing executor. The repo
/// holds no pool so it cannot issue an un-scoped (deny-all under `FORCE`)
/// query.
#[derive(Clone)]
pub struct SubscriptionRepository;

impl SubscriptionRepository {
    /// Create a new SubscriptionRepository.
    ///
    /// The pool argument is retained for construction-site compatibility with
    /// the other repositories on `AppState`; this repo deliberately does not
    /// store it (see module docs — all queries run on a context-set
    /// connection supplied by the caller).
    pub fn new(_pool: PgPool) -> Self {
        Self
    }
}
