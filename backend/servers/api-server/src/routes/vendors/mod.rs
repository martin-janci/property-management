//! Vendor management routes (Epic 21).
//!
//! The handlers are split into cohesive sub-modules (one per resource group) to
//! keep each unit small; [`router`] below remains the single wiring point and
//! preserves the exact public route table. The request/response types and the
//! `shared` helper signatures are unchanged — they are re-exported from this
//! module so the public path (`routes::vendors::CreateVendorRequest`, …) is
//! identical to the pre-split monolith.
//!
//! # RLS (PAP-67 / PAP-70)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` on the vendor tables
//! (`vendors`, `vendor_contacts`, `vendor_contracts`, `vendor_invoices`,
//! `vendor_ratings`), so every query MUST run on a connection that has
//! `app.current_org_id` set or it collapses to deny-all. Each handler therefore
//! acquires an [`RlsConnection`](api_core::extractors::RlsConnection) (which
//! validates tenant membership and sets the org/user GUCs on a dedicated
//! connection) and passes `&mut **rls.conn()` to the repository. The
//! authoritative organization is `rls.tenant_id()` — the tenant the caller was
//! validated against — not a client-supplied `organization_id`, so the SQL org
//! filter and the RLS context can never disagree. Cross-tenant access is blocked
//! by RLS: a by-id read of another org's row returns no row (`404`), and a write
//! targeting another org fails the policy's `WITH CHECK`. `rls.release()` clears
//! the context before the connection returns to the pool.

use axum::{
    routing::{delete, get, patch, post},
    Router,
};

use crate::state::AppState;

mod contacts;
mod contracts;
mod core;
mod invoices;
mod shared;

// Re-export the request/response types so the public API path is unchanged
// from the pre-split monolith (`routes::vendors::<Type>`).
pub use shared::{
    CreateContractRequest, CreateInvoiceRequest, CreateVendorRequest, ExpiringQuery,
    InvoiceSummaryQuery, ListContractsQuery, ListInvoicesQuery, ListVendorsQuery, OrgQuery,
    PaginationQuery, PreferredRequest, RecordPaymentRequest, RejectRequest,
};

/// Create vendors router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Vendors
        .route("/", post(core::create_vendor))
        .route("/", get(core::list_vendors))
        .route("/with-details", get(core::list_vendors_with_details))
        .route("/statistics", get(core::get_statistics))
        .route("/{id}", get(core::get_vendor))
        .route("/{id}", patch(core::update_vendor))
        .route("/{id}", delete(core::delete_vendor))
        .route("/{id}/preferred", post(core::set_preferred))
        // Vendor Contacts
        .route("/{id}/contacts", post(contacts::add_contact))
        .route("/{id}/contacts", get(contacts::list_contacts))
        .route("/contacts/{contact_id}", delete(contacts::delete_contact))
        // Vendor Ratings
        .route("/{id}/ratings", post(contacts::add_rating))
        .route("/{id}/ratings", get(contacts::list_ratings))
        // Contracts
        .route("/contracts", post(contracts::create_contract))
        .route("/contracts", get(contracts::list_contracts))
        .route(
            "/contracts/expiring",
            get(contracts::get_expiring_contracts),
        )
        .route("/contracts/{id}", get(contracts::get_contract))
        .route("/contracts/{id}", patch(contracts::update_contract))
        .route("/contracts/{id}", delete(contracts::delete_contract))
        // Invoices
        .route("/invoices", post(invoices::create_invoice))
        .route("/invoices", get(invoices::list_invoices))
        .route("/invoices/overdue", get(invoices::get_overdue_invoices))
        .route("/invoices/summary", get(invoices::get_invoice_summary))
        .route("/invoices/{id}", get(invoices::get_invoice))
        .route("/invoices/{id}", patch(invoices::update_invoice))
        .route("/invoices/{id}", delete(invoices::delete_invoice))
        .route("/invoices/{id}/approve", post(invoices::approve_invoice))
        .route("/invoices/{id}/reject", post(invoices::reject_invoice))
        .route("/invoices/{id}/payment", post(invoices::record_payment))
}
