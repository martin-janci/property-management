//! Route modules for the accounting server.
//!
//! `health` (liveness/readiness) is Foundation/wiring. The per-epic modules
//! below are each owned by exactly ONE Phase 1 coder (see CONTRACT.md):
//!   * `accounts` — EPIC-ACC-01 (BE-Accounts)
//!   * `config`   — EPIC-ACC-02 (BE-Config)
//!   * `contacts` — EPIC-ACC-03 (BE-Contacts+Catalog)
//!   * `catalog`  — EPIC-ACC-04 (BE-Contacts+Catalog)
//!   * `invoices` — EPIC-ACC-05 (BE-Invoicing)
//!   * `platform` — EPIC-ACC-16 (BE-Platform)
//!
//! Each handler is a `todo!()` stub in Phase 0b; coders fill the bodies in their
//! OWN route file only. The `router()` wiring + nesting below is Foundation-owned.

pub mod accounts;
pub mod catalog;
pub mod config;
pub mod contacts;
pub mod health;
pub mod invoices;
pub mod platform;

use crate::state::AppState;
use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};

/// Build the full `/api/v1` router for the accounting server. Foundation owns
/// this wiring; coders never edit it — they only fill their handler bodies.
pub fn api_router() -> Router<AppState> {
    Router::new()
        // EPIC-ACC-01 — Accounts & access
        .nest(
            "/accounts",
            Router::new()
                .route(
                    "/companies",
                    get(accounts::list_my_companies).post(accounts::create_company),
                )
                .route("/companies/{id}", get(accounts::get_company))
                .route("/switch-company", post(accounts::switch_company))
                .route("/members/invite", post(accounts::invite_user))
                .route("/members/role", post(accounts::set_member_role))
                .route("/members/{user_id}", delete(accounts::deactivate_member))
                .route(
                    "/accountant-access",
                    post(accounts::grant_accountant_access),
                )
                .route(
                    "/accountant-access/{user_id}",
                    delete(accounts::revoke_accountant_access),
                )
                .route("/billing", get(accounts::get_billing)),
        )
        // EPIC-ACC-02 — Company & document configuration
        .nest(
            "/config",
            Router::new()
                .route(
                    "/company",
                    get(config::get_company_settings).put(config::upsert_company_settings),
                )
                .route(
                    "/numbering",
                    get(config::list_numbering_series).post(config::create_numbering_series),
                )
                .route("/units", get(config::list_units))
                .route("/vat-rates", get(config::list_vat_rates))
                .route("/bank-accounts", get(config::list_bank_accounts)),
        )
        // EPIC-ACC-03 — Contacts & CRM
        .nest(
            "/contacts",
            Router::new()
                .route(
                    "/",
                    get(contacts::list_contacts).post(contacts::create_contact),
                )
                .route(
                    "/{id}",
                    get(contacts::get_contact)
                        .patch(contacts::update_contact)
                        .delete(contacts::deactivate_contact),
                )
                .route("/{id}/merge", post(contacts::merge_contact))
                .route("/{id}/addresses", get(contacts::list_addresses)),
        )
        // EPIC-ACC-04 — Catalog
        .nest(
            "/catalog",
            Router::new()
                .route(
                    "/items",
                    get(catalog::list_items).post(catalog::create_item),
                )
                .route(
                    "/items/{id}",
                    get(catalog::get_item).patch(catalog::update_item),
                )
                .route("/items/{id}/price-levels", get(catalog::list_price_levels))
                .route("/categories", get(catalog::list_categories)),
        )
        // EPIC-ACC-05 — Sales invoicing
        .nest(
            "/invoices",
            Router::new()
                .route(
                    "/",
                    get(invoices::list_invoices).post(invoices::create_invoice),
                )
                .route(
                    "/{id}",
                    get(invoices::get_invoice)
                        .patch(invoices::update_invoice)
                        .delete(invoices::delete_invoice),
                )
                .route("/{id}/items", get(invoices::list_invoice_items))
                .route("/{id}/pdf", get(invoices::get_invoice_pdf))
                .route("/{id}/issue", post(invoices::issue_invoice))
                .route("/{id}/exchange-rate", post(invoices::set_exchange_rate))
                .route("/{id}/credit-note", post(invoices::create_credit_note))
                .route("/{id}/duplicate", post(invoices::duplicate_invoice))
                .route("/{id}/links", get(invoices::list_links)),
        )
        // EPIC-ACC-16 — Platform/security
        .nest(
            "/platform",
            Router::new()
                .route("/audit-log", get(platform::list_audit_log))
                .route("/tags", post(platform::add_tag))
                .route("/share-links", post(platform::create_share_link))
                .route("/share-links/{id}", delete(platform::revoke_share_link))
                .route("/2fa/enroll", post(platform::enroll_two_factor))
                .route("/2fa/confirm", post(platform::confirm_two_factor)),
        )
        // EPIC-ACC-05.11 — PUBLIC capability-token invoice view (no auth)
        .nest(
            "/share",
            Router::new().route("/{token}", get(platform::view_shared_invoice)),
        )
}
