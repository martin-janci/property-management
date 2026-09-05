//! Sales invoicing routes (EPIC-ACC-05 / UC-ACC-05). OWNED BY: BE-Invoicing.
//!
//! Resource-server handlers: `AuthUser` + `RlsConnection`, RBAC via
//! `accounting_core::authz`, money/VAT/rounding via `accounting_core::{money,vat,
//! rounding}`, numbering via `accounting_core::numbering` + BE-Config's atomic
//! allocator, persistence via the existing
//! `db::repositories::AccountingRepository` (base CRUD on the 00184 `invoice`
//! table) PLUS `db::repositories::acc_invoicing::AccInvoicingRepository`
//! (00196 extensions: issue, FX, credit notes, advance settlement, links,
//! attachments). `nest`ed under `/api/v1/invoices`.
//!
//! Mutations always: (1) gate with `authz::require_role(..MUTATE_MIN_ROLE)`,
//! (2) run inside one `rls.begin()` transaction so the document write and its
//! `acc_audit_log` entry (`audit::record` → `platform_repo.record_audit_rls`)
//! commit atomically (UC-ACC-16.7 append-only), (3) `rls.release().await` before
//! returning.

use accounting_core::numbering::{self, NumberParts};
use accounting_core::types::{LineInput, Money, RoundingMode};
use accounting_core::{audit, authz, money};
use api_core::extractors::{AuthUser, RlsConnection};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Datelike;
use db::models::acc_invoicing_ext::{AccDocumentLink, AccInvoiceExt};
use db::models::accounting::{
    CreateInvoice, CreateInvoiceItem, Invoice, InvoiceItem, UpdateInvoice,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::Connection;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::state::AppState;

/// Currencies accepted by the MVP document editor (mirrors the embedded MVP).
const VALID_CURRENCIES: &[&str] = &["CZK", "EUR"];

/// Request to issue a draft invoice (allocate a gapless number) (UC-ACC-05.1).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IssueInvoiceRequest {
    /// Numbering series to allocate from; if absent, the default for the
    /// invoice's doc_type/year is used.
    pub numbering_series_id: Option<Uuid>,
}

/// Request to create a credit note against an issued invoice (UC-ACC-05.7).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCreditNoteRequest {
    /// Full vs partial: lines to reverse; empty = full correction.
    pub items: Vec<db::models::accounting::CreateInvoiceItem>,
    pub issue_date: chrono::NaiveDate,
}

/// Request to set/override the FX rate on a foreign-currency invoice (UC-ACC-05.4).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SetExchangeRateRequest {
    pub exchange_rate: Decimal,
}

/// Request to create a typed link between two documents (UC-ACC-05.15).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateLinkRequest {
    pub to_invoice_id: Uuid,
    pub relation: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Map a company `rounding_mode` string to the engine's [`RoundingMode`].
/// Defaults to arithmetic if settings are absent/unknown.
fn rounding_mode_for(settings_mode: Option<&str>) -> RoundingMode {
    match settings_mode {
        Some("bankers") => RoundingMode::Bankers,
        Some("none") => RoundingMode::None,
        _ => RoundingMode::Arithmetic,
    }
}

/// Build a [`LineInput`] from a stored [`InvoiceItem`] (no per-line discount on
/// the 00184 schema, so `discount_pct = 0`).
fn line_input_from_item(item: &InvoiceItem) -> LineInput {
    LineInput {
        qty: item.qty,
        unit_price: item.unit_price,
        vat_rate: item.vat_rate,
        discount_pct: Decimal::ZERO,
    }
}

/// Validate the create-invoice payload (UC-ACC-05.1/.2). Mirrors the embedded
/// MVP's field rules.
fn validate_create_invoice(data: &CreateInvoice) -> Result<(), StatusCode> {
    if data.due_date < data.issue_date {
        return Err(StatusCode::BAD_REQUEST);
    }
    if !VALID_CURRENCIES.contains(&data.currency.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    for item in &data.items {
        if item.description.trim().is_empty() || item.description.len() > 255 {
            return Err(StatusCode::BAD_REQUEST);
        }
        if item.qty < Decimal::ZERO || item.unit_price < Decimal::ZERO {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Reads
// ---------------------------------------------------------------------------

/// List invoices for the active company (UC-ACC-05.1).
#[utoipa::path(
    get,
    path = "/api/v1/invoices",
    responses(
        (status = 200, description = "Invoices", body = [AccInvoiceExt]),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Invoices"
)]
pub async fn list_invoices(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<Vec<AccInvoiceExt>>, StatusCode> {
    // UC-ACC-05.1: read gate (manager+). RLS scopes rows to the active company.
    authz::require_role(rls.role(), authz::READ_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let invoices = state
        .invoicing_repo
        .list_invoices_ext_rls(&mut **rls, None)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list invoices: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    rls.release().await;
    Ok(Json(invoices))
}

/// Get a single invoice (UC-ACC-05.1).
#[utoipa::path(
    get,
    path = "/api/v1/invoices/{id}",
    params(("id" = Uuid, Path, description = "Invoice id")),
    responses(
        (status = 200, description = "Invoice", body = AccInvoiceExt),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found")
    ),
    tag = "Invoices"
)]
pub async fn get_invoice(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<AccInvoiceExt>, StatusCode> {
    authz::require_role(rls.role(), authz::READ_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let invoice = state
        .invoicing_repo
        .find_invoice_ext_rls(&mut **rls, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to find invoice {id}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    rls.release().await;
    Ok(Json(invoice))
}

/// List items for an invoice (UC-ACC-05.2).
#[utoipa::path(
    get,
    path = "/api/v1/invoices/{id}/items",
    params(("id" = Uuid, Path, description = "Invoice id")),
    responses(
        (status = 200, description = "Invoice items", body = [InvoiceItem]),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Invoices"
)]
pub async fn list_invoice_items(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<Vec<InvoiceItem>>, StatusCode> {
    authz::require_role(rls.role(), authz::READ_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let items = state
        .accounting_repo
        .find_invoice_items_rls(&mut **rls, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list invoice items for {id}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    rls.release().await;
    Ok(Json(items))
}

// ---------------------------------------------------------------------------
// Mutations
// ---------------------------------------------------------------------------

/// Create a draft invoice with line items (UC-ACC-05.1/.2). Money/VAT computed +
/// rounded by `AccountingRepository::create_invoice_rls` (the #1522 invariant:
/// header == Σ rounded line rows), which `accounting_core::money` mirrors.
#[utoipa::path(
    post,
    path = "/api/v1/invoices",
    request_body = CreateInvoice,
    responses(
        (status = 201, description = "Created draft", body = Invoice),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    tag = "Invoices"
)]
pub async fn create_invoice(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    auth: AuthUser,
    Json(mut data): Json<CreateInvoice>,
) -> Result<(StatusCode, Json<Invoice>), StatusCode> {
    // UC-ACC-16.2: mutate gate (manager+).
    authz::require_role(rls.role(), authz::MUTATE_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let tenant_id = rls.tenant_id();
    // Force tenant_id from the trusted RLS context, never the request body.
    data.tenant_id = tenant_id;
    // A freshly created document is always a draft (UC-ACC-05.1); the number is
    // allocated on issue, so callers cannot pre-set an issued status here.
    data.status = None;

    validate_create_invoice(&data)?;

    let mut tx = rls.begin().await.map_err(|e| {
        tracing::error!("Failed to start transaction: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Validate contact ownership (RLS already scopes to tenant).
    let contact = state
        .accounting_repo
        .find_contact_rls(&mut *tx, data.contact_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to find contact: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    if contact.is_none() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let invoice = state
        .accounting_repo
        .create_invoice_rls(&mut tx, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create invoice: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // UC-ACC-16.7: append-only audit entry in the SAME transaction.
    let entry = audit::record(
        tenant_id,
        Some(auth.user_id),
        "create",
        "invoice",
        Some(invoice.id),
        serde_json::json!({ "number": invoice.number, "status": "draft" }),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    state
        .platform_repo
        .record_audit_rls(&mut *tx, entry)
        .await
        .map_err(|e| {
            tracing::error!("Failed to record audit: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    rls.release().await;
    Ok((StatusCode::CREATED, Json(invoice)))
}

/// Update a draft invoice (UC-ACC-05.1). Issued invoices are immutable except via
/// a corrective document, so the update is rejected unless status is `draft`.
#[utoipa::path(
    patch,
    path = "/api/v1/invoices/{id}",
    params(("id" = Uuid, Path, description = "Invoice id")),
    request_body = UpdateInvoice,
    responses(
        (status = 200, description = "Updated", body = Invoice),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden / issued is immutable"),
        (status = 404, description = "Not found")
    ),
    tag = "Invoices"
)]
pub async fn update_invoice(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    mut rls: RlsConnection,
    auth: AuthUser,
    Json(data): Json<UpdateInvoice>,
) -> Result<Json<Invoice>, StatusCode> {
    authz::require_role(rls.role(), authz::MUTATE_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let tenant_id = rls.tenant_id();

    let mut tx = rls.begin().await.map_err(|e| {
        tracing::error!("Failed to start transaction: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // UC-ACC-05.1: issued documents are immutable. Reject any edit to a
    // non-draft.
    let existing = state
        .invoicing_repo
        .find_invoice_ext_rls(&mut *tx, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load invoice {id}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;
    if existing.status != "draft" {
        return Err(StatusCode::FORBIDDEN);
    }

    let invoice = state
        .accounting_repo
        .update_invoice_rls(&mut *tx, id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update invoice {id}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let entry = audit::record(
        tenant_id,
        Some(auth.user_id),
        "update",
        "invoice",
        Some(id),
        serde_json::json!({ "status": "draft" }),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    state
        .platform_repo
        .record_audit_rls(&mut *tx, entry)
        .await
        .map_err(|e| {
            tracing::error!("Failed to record audit: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    rls.release().await;
    Ok(Json(invoice))
}

/// Delete a draft invoice (UC-ACC-05.1). Issued invoices cannot be deleted.
#[utoipa::path(
    delete,
    path = "/api/v1/invoices/{id}",
    params(("id" = Uuid, Path, description = "Invoice id")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden / issued cannot be deleted")
    ),
    tag = "Invoices"
)]
pub async fn delete_invoice(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    mut rls: RlsConnection,
    auth: AuthUser,
) -> Result<StatusCode, StatusCode> {
    authz::require_role(rls.role(), authz::MUTATE_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let tenant_id = rls.tenant_id();

    let mut tx = rls.begin().await.map_err(|e| {
        tracing::error!("Failed to start transaction: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // UC-ACC-05.1: only drafts are deletable; issued docs are kept for audit.
    let existing = state
        .invoicing_repo
        .find_invoice_ext_rls(&mut *tx, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load invoice {id}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;
    if existing.status != "draft" {
        return Err(StatusCode::FORBIDDEN);
    }

    // Audit BEFORE delete so the entry references the still-existing row.
    let entry = audit::record(
        tenant_id,
        Some(auth.user_id),
        "delete",
        "invoice",
        Some(id),
        serde_json::json!({ "number": existing.number }),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    state
        .platform_repo
        .record_audit_rls(&mut *tx, entry)
        .await
        .map_err(|e| {
            tracing::error!("Failed to record audit: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    state
        .accounting_repo
        .delete_invoice_rls(&mut *tx, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete invoice {id}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    rls.release().await;
    Ok(StatusCode::NO_CONTENT)
}

/// Issue a draft: allocate the next gapless number and lock it (UC-ACC-05.1).
///
/// Issuance preconditions (blocked with 400 otherwise): the draft has a contact
/// and at least one line. Number allocation is atomic + gapless: inside ONE
/// transaction we (1) resolve the doc_type's numbering series for the issue
/// year, (2) `allocate_next_number_rls` (single `UPDATE ... RETURNING`), (3)
/// `numbering::format_number`, (4) `issue_invoice_rls` (status draft→issued),
/// (5) audit. Concurrent issuance therefore can never skip or duplicate a
/// number (legal requirement).
#[utoipa::path(
    post,
    path = "/api/v1/invoices/{id}/issue",
    params(("id" = Uuid, Path, description = "Invoice id")),
    request_body = IssueInvoiceRequest,
    responses(
        (status = 200, description = "Issued", body = AccInvoiceExt),
        (status = 400, description = "Draft incomplete"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    tag = "Invoices"
)]
pub async fn issue_invoice(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    mut rls: RlsConnection,
    auth: AuthUser,
    Json(_req): Json<IssueInvoiceRequest>,
) -> Result<Json<AccInvoiceExt>, StatusCode> {
    authz::require_role(rls.role(), authz::MUTATE_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let tenant_id = rls.tenant_id();

    let mut tx = rls.begin().await.map_err(|e| {
        tracing::error!("Failed to start transaction: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Load the draft (RLS-scoped).
    let draft = state
        .invoicing_repo
        .find_invoice_ext_rls(&mut *tx, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load invoice {id}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // UC-ACC-05.1: issuance preconditions — must be a draft, have a contact, and
    // have at least one line.
    if draft.status != "draft" {
        return Err(StatusCode::BAD_REQUEST);
    }
    let items = state
        .accounting_repo
        .find_invoice_items_rls(&mut *tx, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load items for {id}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    if items.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // UC-ACC-05.4: a foreign-currency document cannot be issued without an FX
    // rate (so the base-currency equivalent for VAT/reporting is recorded).
    let company = state
        .config_repo
        .get_company_settings_rls(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load company settings: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    let base_currency = company
        .as_ref()
        .map(|c| c.base_currency.clone())
        .unwrap_or_else(|| "CZK".to_string());
    if draft.currency != base_currency && draft.exchange_rate.is_none() {
        // Block issue without a rate (UC-ACC-05.4).
        return Err(StatusCode::BAD_REQUEST);
    }

    // Resolve the numbering series for this doc_type + issue year (UC-ACC-02.3).
    let year = draft.issue_date.year();
    let series_list = state
        .config_repo
        .list_numbering_series_rls(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load numbering series: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    let series = series_list
        .into_iter()
        .find(|s| s.doc_type == draft.doc_type && s.year == year)
        // Without a configured series we cannot issue a compliant number.
        .ok_or(StatusCode::BAD_REQUEST)?;

    // Atomic gapless allocation, then pure format.
    let seq = state
        .config_repo
        .allocate_next_number_rls(&mut *tx, &draft.doc_type, year)
        .await
        .map_err(|e| {
            tracing::error!("Failed to allocate number: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    let number = numbering::format_number(
        &series.format,
        &NumberParts {
            prefix: &series.prefix,
            year,
            seq,
        },
    )
    .map_err(|e| {
        tracing::error!("Failed to format number: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let issued = state
        .invoicing_repo
        .issue_invoice_rls(&mut *tx, id, &number, series.id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to issue invoice {id}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        // None means the draft guard in the UPDATE did not match (concurrent
        // issue / status changed) — surface as a conflict-class 400.
        .ok_or(StatusCode::BAD_REQUEST)?;

    let entry = audit::record(
        tenant_id,
        Some(auth.user_id),
        "issue",
        "invoice",
        Some(id),
        serde_json::json!({ "number": number, "status": "issued" }),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    state
        .platform_repo
        .record_audit_rls(&mut *tx, entry)
        .await
        .map_err(|e| {
            tracing::error!("Failed to record audit: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    rls.release().await;
    Ok(Json(issued))
}

/// Set/override the exchange rate on a foreign-currency invoice (UC-ACC-05.4).
/// Persists the rate AND the base-currency equivalent (`base_currency_amount`)
/// computed from the document total via `money::to_base_currency`, which drives
/// VAT/reporting.
#[utoipa::path(
    post,
    path = "/api/v1/invoices/{id}/exchange-rate",
    params(("id" = Uuid, Path, description = "Invoice id")),
    request_body = SetExchangeRateRequest,
    responses(
        (status = 200, description = "Updated", body = AccInvoiceExt),
        (status = 400, description = "Invalid rate"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    tag = "Invoices"
)]
pub async fn set_exchange_rate(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    mut rls: RlsConnection,
    auth: AuthUser,
    Json(req): Json<SetExchangeRateRequest>,
) -> Result<Json<AccInvoiceExt>, StatusCode> {
    authz::require_role(rls.role(), authz::MUTATE_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    // Rate must be strictly positive (UC-ACC-05.4).
    if req.exchange_rate <= Decimal::ZERO {
        return Err(StatusCode::BAD_REQUEST);
    }

    let tenant_id = rls.tenant_id();

    let mut tx = rls.begin().await.map_err(|e| {
        tracing::error!("Failed to start transaction: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let invoice = state
        .invoicing_repo
        .find_invoice_ext_rls(&mut *tx, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load invoice {id}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Compute the base-currency equivalent of the document total.
    let doc_total = Money {
        base: invoice.base_amount,
        vat: invoice.vat_amount,
        total: invoice.total_amount,
    };
    let base_currency_amount =
        money::to_base_currency(&doc_total, req.exchange_rate, RoundingMode::Arithmetic).total;

    let updated = state
        .invoicing_repo
        .set_exchange_rate_rls(&mut *tx, id, req.exchange_rate, base_currency_amount)
        .await
        .map_err(|e| {
            tracing::error!("Failed to set exchange rate for {id}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let entry = audit::record(
        tenant_id,
        Some(auth.user_id),
        "set_exchange_rate",
        "invoice",
        Some(id),
        serde_json::json!({ "exchange_rate": req.exchange_rate, "base_currency_amount": base_currency_amount }),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    state
        .platform_repo
        .record_audit_rls(&mut *tx, entry)
        .await
        .map_err(|e| {
            tracing::error!("Failed to record audit: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    rls.release().await;
    Ok(Json(updated))
}

/// Create a credit note against an issued invoice (UC-ACC-05.7).
///
/// The original stays immutable. A new `doc_type='credit_note'` document is
/// inserted with `corrected_invoice_id` set and SIGNED (negative) amounts: full
/// correction (empty `items`) reverses the whole original; partial correction
/// reverses only the supplied lines. VAT is reversed via the negated totals
/// (`vat::reverse_groups` semantics) so the correction lands in the same VAT
/// buckets. A `corrects` document link is recorded (UC-ACC-05.15).
#[utoipa::path(
    post,
    path = "/api/v1/invoices/{id}/credit-note",
    params(("id" = Uuid, Path, description = "Corrected invoice id")),
    request_body = CreateCreditNoteRequest,
    responses(
        (status = 201, description = "Credit note created", body = AccInvoiceExt),
        (status = 400, description = "Original not issued"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    tag = "Invoices"
)]
pub async fn create_credit_note(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    mut rls: RlsConnection,
    auth: AuthUser,
    Json(req): Json<CreateCreditNoteRequest>,
) -> Result<(StatusCode, Json<AccInvoiceExt>), StatusCode> {
    authz::require_role(rls.role(), authz::MUTATE_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let tenant_id = rls.tenant_id();

    let mut tx = rls.begin().await.map_err(|e| {
        tracing::error!("Failed to start transaction: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Load the original; only an issued (or later-lifecycle) document can be
    // corrected — a draft has no legal number to correct (UC-ACC-05.7).
    let original = state
        .invoicing_repo
        .find_invoice_ext_rls(&mut *tx, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load original invoice {id}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;
    if original.status == "draft" {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Determine which lines to reverse. Empty request items = FULL correction
    // (reverse every original line); otherwise PARTIAL (reverse the supplied
    // lines). Resolve named VAT rates the same way the base create path does.
    let company = state
        .config_repo
        .get_company_settings_rls(&mut *tx)
        .await
        .ok()
        .flatten();
    let mode = rounding_mode_for(company.as_ref().map(|c| c.rounding_mode.as_str()));

    let mut credit_lines: Vec<(String, LineInput)> = Vec::new();
    if req.items.is_empty() {
        // Full correction: reverse the persisted original line rows verbatim.
        let original_items = state
            .accounting_repo
            .find_invoice_items_rls(&mut *tx, id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to load original items: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        for item in &original_items {
            credit_lines.push((item.description.clone(), line_input_from_item(item)));
        }
    } else {
        // Partial correction: reverse the supplied lines.
        for item in &req.items {
            credit_lines.push((
                item.description.clone(),
                LineInput {
                    qty: item.qty,
                    unit_price: item.unit_price,
                    vat_rate: item.vat_rate,
                    discount_pct: Decimal::ZERO,
                },
            ));
        }
    }

    // Compute the positive totals, then negate to reverse the VAT and base
    // (UC-ACC-05.7). Building a signed credit-note header keeps the receivables
    // recompute (EPIC-ACC-09) and VAT records (EPIC-ACC-10) consistent.
    let line_inputs: Vec<LineInput> = credit_lines.iter().map(|(_, li)| li.clone()).collect();
    let positive =
        money::compute_document_total(&line_inputs, mode).map_err(|_| StatusCode::BAD_REQUEST)?;

    let credit_header = AccInvoiceExt {
        // ids/timestamps are DB-defaulted on insert; placeholders here.
        id: Uuid::nil(),
        tenant_id,
        contact_id: original.contact_id,
        number: String::new(), // credit notes are numbered on their own issue
        issue_date: req.issue_date,
        due_date: req.issue_date,
        taxable_supply_date: original.taxable_supply_date,
        currency: original.currency.clone(),
        total_amount: -positive.total,
        base_amount: -positive.base,
        vat_amount: -positive.vat,
        variable_symbol: original.variable_symbol.clone(),
        status: "draft".to_string(),
        paid_amount: Decimal::ZERO,
        created_at: original.created_at,
        updated_at: original.updated_at,
        doc_type: "credit_note".to_string(),
        corrected_invoice_id: Some(id),
        advance_settlement: Decimal::ZERO,
        settled_advance_id: None,
        exchange_rate: original.exchange_rate,
        base_currency_amount: None,
        numbering_series_id: None,
        bank_account_id: original.bank_account_id,
    };

    let credit_note = state
        .invoicing_repo
        .create_credit_note_rls(&mut *tx, credit_header)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create credit note: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Insert signed line rows (negated per-line money) so the credit note's
    // items reconcile to its negative header.
    for (description, li) in &credit_lines {
        let m = money::compute_line(li, mode).map_err(|_| StatusCode::BAD_REQUEST)?;
        state
            .invoicing_repo
            .insert_invoice_item_signed_rls(
                &mut *tx,
                credit_note.id,
                tenant_id,
                description,
                -li.qty,
                li.unit_price,
                li.vat_rate,
                -m.base,
                -m.vat,
                -m.total,
            )
            .await
            .map_err(|e| {
                tracing::error!("Failed to insert credit-note line: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    // UC-ACC-05.15: typed `corrects` link from the credit note to the original.
    let link = AccDocumentLink {
        id: Uuid::nil(),
        tenant_id,
        from_invoice_id: credit_note.id,
        to_invoice_id: id,
        relation: "corrects".to_string(),
        created_at: credit_note.created_at,
    };
    state
        .invoicing_repo
        .create_link_rls(&mut *tx, link)
        .await
        .map_err(|e| {
            tracing::error!("Failed to link credit note: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let entry = audit::record(
        tenant_id,
        Some(auth.user_id),
        "create_credit_note",
        "invoice",
        Some(credit_note.id),
        serde_json::json!({ "corrected_invoice_id": id, "total_amount": credit_note.total_amount }),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    state
        .platform_repo
        .record_audit_rls(&mut *tx, entry)
        .await
        .map_err(|e| {
            tracing::error!("Failed to record audit: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    rls.release().await;
    Ok((StatusCode::CREATED, Json(credit_note)))
}

/// Duplicate an invoice into a fresh draft (UC-ACC-05.16).
///
/// Deep-copies the line items; resets dates to today, status to `draft`; carries
/// over NO number, NO payments, NO links and NO doc-type extensions — exactly a
/// fresh draft the user can edit and issue independently.
#[utoipa::path(
    post,
    path = "/api/v1/invoices/{id}/duplicate",
    params(("id" = Uuid, Path, description = "Invoice id")),
    responses(
        (status = 201, description = "Duplicated draft", body = Invoice),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    tag = "Invoices"
)]
pub async fn duplicate_invoice(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    mut rls: RlsConnection,
    auth: AuthUser,
) -> Result<(StatusCode, Json<Invoice>), StatusCode> {
    authz::require_role(rls.role(), authz::MUTATE_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let tenant_id = rls.tenant_id();

    let mut tx = rls.begin().await.map_err(|e| {
        tracing::error!("Failed to start transaction: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let source = state
        .invoicing_repo
        .find_invoice_ext_rls(&mut *tx, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load invoice {id}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let source_items = state
        .accounting_repo
        .find_invoice_items_rls(&mut *tx, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load items for {id}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Fresh dates: issue = today, due = today + (default terms or original span).
    let today = chrono::Utc::now().date_naive();
    let span = (source.due_date - source.issue_date).num_days().max(0);
    let due = today + chrono::Duration::days(span);

    let new_draft = CreateInvoice {
        tenant_id,
        contact_id: source.contact_id,
        // No number — allocated only on issue (UC-ACC-05.16).
        number: String::new(),
        issue_date: today,
        due_date: due,
        taxable_supply_date: None,
        currency: source.currency.clone(),
        // No variable symbol / payments carried over.
        variable_symbol: None,
        status: None, // forced to draft by create_invoice_rls default
        items: source_items
            .iter()
            .map(|it| CreateInvoiceItem {
                description: it.description.clone(),
                qty: it.qty,
                unit_price: it.unit_price,
                vat_rate: it.vat_rate,
                vat_rate_type: None,
            })
            .collect(),
        country: None,
    };

    let invoice = state
        .accounting_repo
        .create_invoice_rls(&mut tx, new_draft)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create duplicate: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let entry = audit::record(
        tenant_id,
        Some(auth.user_id),
        "duplicate",
        "invoice",
        Some(invoice.id),
        serde_json::json!({ "source_invoice_id": id }),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    state
        .platform_repo
        .record_audit_rls(&mut *tx, entry)
        .await
        .map_err(|e| {
            tracing::error!("Failed to record audit: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    rls.release().await;
    Ok((StatusCode::CREATED, Json(invoice)))
}

/// List typed document links from/to an invoice (UC-ACC-05.15).
#[utoipa::path(
    get,
    path = "/api/v1/invoices/{id}/links",
    params(("id" = Uuid, Path, description = "Invoice id")),
    responses(
        (status = 200, description = "Document links", body = [AccDocumentLink]),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Invoices"
)]
pub async fn list_links(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<Vec<AccDocumentLink>>, StatusCode> {
    authz::require_role(rls.role(), authz::READ_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let links = state
        .invoicing_repo
        .list_links_rls(&mut **rls, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list links for {id}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    rls.release().await;
    Ok(Json(links))
}

/// Render the document as a print-ready PDF (UC-ACC-05.9).
///
/// Works for every lifecycle state — drafts render with a visible
/// "DRAFT — not a tax document" line so previews cannot be passed off as tax
/// documents. All reads happen inside one RLS transaction so the PDF is a
/// consistent snapshot (invoice + items + company + contact + bank account).
#[utoipa::path(
    get,
    path = "/api/v1/invoices/{id}/pdf",
    params(("id" = Uuid, Path, description = "Invoice id")),
    responses(
        (status = 200, description = "PDF document", content_type = "application/pdf"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Render failed (e.g. fonts missing on host)")
    ),
    tag = "Invoices"
)]
pub async fn get_invoice_pdf(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<axum::response::Response, StatusCode> {
    authz::require_role(rls.role(), authz::READ_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let mut tx = rls.begin().await.map_err(|e| {
        tracing::error!("Failed to start transaction: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let invoice = state
        .invoicing_repo
        .find_invoice_ext_rls(&mut *tx, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load invoice {id}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;
    let items = state
        .accounting_repo
        .find_invoice_items_rls(&mut *tx, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load items for {id}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    let company = state
        .config_repo
        .get_company_settings_rls(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load company settings: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    let contact = state
        .accounting_repo
        .find_contact_rls(&mut *tx, invoice.contact_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load contact: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    // The document's pinned bank account wins; otherwise the default/first
    // active one (UC-ACC-02.10).
    let banks = state
        .config_repo
        .list_bank_accounts_rls(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load bank accounts: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    // Reads only, but the tx borrows the RLS connection — close it before
    // rendering so `rls.release()` below can re-borrow.
    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let bank_account = match invoice.bank_account_id {
        Some(bid) => banks.iter().find(|b| b.id == bid),
        None => banks.iter().find(|b| b.is_default).or(banks.first()),
    };

    let ctx = crate::pdf::InvoiceRenderContext {
        invoice: &invoice,
        items: &items,
        company: company.as_ref(),
        contact: contact.as_ref(),
        bank_account,
    };
    let bytes = crate::pdf::render_invoice_pdf(&ctx).map_err(|e| {
        tracing::error!("PDF render for {id} failed: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    rls.release().await;

    use axum::response::IntoResponse;
    let disposition = format!(
        "inline; filename=\"{}\"",
        crate::pdf::pdf_filename(&invoice.number)
    );
    Ok((
        StatusCode::OK,
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/pdf".to_string(),
            ),
            (axum::http::header::CONTENT_DISPOSITION, disposition),
        ],
        bytes,
    )
        .into_response())
}

// ---------------------------------------------------------------------------
// Rendering / delivery / export HOOKS (UC-ACC-05.8/.9/.10/.11/.12)
// ---------------------------------------------------------------------------
//
// EPIC-ACC-05 specifies PDF preview, email send, shareable links, payment QR,
// and ISDOC/XML export. The full renderers are out of scope for the MVP build
// (flagged in contract-notes/be-invoicing.md) — but the SEAMS are defined here
// as traits with minimal/no-op implementations so the issue/send/export flows
// have stable injection points and the deterministic pieces (the SPAYD payment
// string for the QR) are unit-testable now. The wired send/export/share routes
// live in the platform router (owned by BE-Platform); these hooks are the
// domain contract those routes will call. Foundation/Phase 2 can register real
// implementations against these traits without changing handler code.
pub mod hooks {
    use super::*;

    /// Render an issued document to a print-ready artifact (UC-ACC-05.1/.9).
    /// The MVP ships [`NoopRenderer`]; a real renderer produces branded PDF
    /// bytes from the invoice + company template.
    pub trait DocumentRenderer: Send + Sync {
        /// Produce PDF bytes for `invoice` (full correction stubbed for MVP).
        fn render_pdf(&self, invoice: &AccInvoiceExt) -> Result<Vec<u8>, RenderError>;
    }

    /// Export an issued document to a structured electronic format
    /// (ISDOC/XML, UC-ACC-05.12). The MVP ships a minimal ISDOC-shaped XML so the
    /// export endpoint and the public API (EPIC-ACC-14) share one code path.
    pub trait DocumentExporter: Send + Sync {
        /// Produce a structured (ISDOC/XML) representation of `invoice`.
        fn export(&self, invoice: &AccInvoiceExt) -> Result<String, RenderError>;
    }

    /// Deliver an issued document by email from a template (UC-ACC-05.10).
    /// The MVP ships [`NoopMailer`]; a real mailer sends async with a
    /// delivery-status callback and advances status to `sent` only on success.
    pub trait DocumentMailer: Send + Sync {
        /// Send `invoice` (rendered PDF + link) to `recipient`. Returns Ok on a
        /// successfully queued send.
        fn send(&self, invoice: &AccInvoiceExt, recipient: &str) -> Result<(), RenderError>;
    }

    /// Rendering/delivery failure surfaced by the hook traits.
    #[derive(Debug)]
    pub enum RenderError {
        /// The feature is intentionally not implemented in the MVP build.
        NotImplemented,
        /// A real implementation failed (transport, template, schema, ...).
        Failed(String),
    }

    /// No-op renderer for the MVP: returns `NotImplemented` so callers degrade
    /// gracefully (the document is still issued; only the artifact is absent).
    pub struct NoopRenderer;
    impl DocumentRenderer for NoopRenderer {
        fn render_pdf(&self, _invoice: &AccInvoiceExt) -> Result<Vec<u8>, RenderError> {
            Err(RenderError::NotImplemented)
        }
    }

    /// No-op mailer for the MVP.
    pub struct NoopMailer;
    impl DocumentMailer for NoopMailer {
        fn send(&self, _invoice: &AccInvoiceExt, _recipient: &str) -> Result<(), RenderError> {
            Err(RenderError::NotImplemented)
        }
    }

    /// Build the SPAYD ("Short Payment Descriptor", the QR-platba standard used
    /// in CZ/SK) string for a payment QR code (UC-ACC-05.8). This IS deterministic
    /// and implemented: the QR image generation itself is a downstream concern,
    /// but the encoded payment string — amount, currency, IBAN, and the
    /// structured payment reference that `auto-match` (EPIC-ACC-08.6) keys on —
    /// is domain logic and unit-tested here.
    ///
    /// Returns `None` when nothing is owed (zero/negative balance) so the QR is
    /// omitted (UC-ACC-05.8 "omitted when paid").
    pub fn payment_spayd(
        iban: &str,
        amount_due: Decimal,
        currency: &str,
        variable_symbol: Option<&str>,
    ) -> Option<String> {
        if amount_due <= Decimal::ZERO {
            return None;
        }
        // SPAYD core fields: ACC (IBAN), AM (amount, 2dp), CC (currency). The
        // variable symbol is carried in X-VS so it matches the auto-match key.
        let mut s = format!(
            "SPD*1.0*ACC:{}*AM:{}*CC:{}",
            iban,
            amount_due.round_dp(2),
            currency
        );
        if let Some(vs) = variable_symbol {
            if !vs.is_empty() {
                s.push_str(&format!("*X-VS:{vs}"));
            }
        }
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn spayd_encodes_amount_currency_iban_and_vs() {
        let s = super::hooks::payment_spayd(
            "SK3112000000198742637541",
            d("123.45"),
            "EUR",
            Some("2026001"),
        )
        .unwrap();
        assert!(s.starts_with("SPD*1.0*"));
        assert!(s.contains("ACC:SK3112000000198742637541"));
        assert!(s.contains("AM:123.45"));
        assert!(s.contains("CC:EUR"));
        assert!(s.contains("X-VS:2026001"));
    }

    #[test]
    fn spayd_omitted_when_nothing_owed() {
        // Zero/negative balance → QR omitted (UC-ACC-05.8).
        assert!(super::hooks::payment_spayd("SK...", Decimal::ZERO, "EUR", None).is_none());
        assert!(super::hooks::payment_spayd("SK...", d("-5.00"), "EUR", None).is_none());
    }

    #[test]
    fn rounding_mode_mapping() {
        assert_eq!(rounding_mode_for(Some("bankers")), RoundingMode::Bankers);
        assert_eq!(rounding_mode_for(Some("none")), RoundingMode::None);
        assert_eq!(
            rounding_mode_for(Some("arithmetic")),
            RoundingMode::Arithmetic
        );
        // Unknown / absent → arithmetic default.
        assert_eq!(rounding_mode_for(None), RoundingMode::Arithmetic);
        assert_eq!(rounding_mode_for(Some("weird")), RoundingMode::Arithmetic);
    }

    #[test]
    fn validate_rejects_due_before_issue() {
        let data = CreateInvoice {
            tenant_id: Uuid::nil(),
            contact_id: Uuid::nil(),
            number: String::new(),
            issue_date: chrono::NaiveDate::from_ymd_opt(2026, 6, 10).unwrap(),
            due_date: chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            taxable_supply_date: None,
            currency: "EUR".to_string(),
            variable_symbol: None,
            status: None,
            items: vec![],
            country: None,
        };
        assert_eq!(validate_create_invoice(&data), Err(StatusCode::BAD_REQUEST));
    }

    #[test]
    fn validate_rejects_unknown_currency() {
        let data = CreateInvoice {
            tenant_id: Uuid::nil(),
            contact_id: Uuid::nil(),
            number: String::new(),
            issue_date: chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            due_date: chrono::NaiveDate::from_ymd_opt(2026, 6, 10).unwrap(),
            taxable_supply_date: None,
            currency: "USD".to_string(),
            variable_symbol: None,
            status: None,
            items: vec![],
            country: None,
        };
        assert_eq!(validate_create_invoice(&data), Err(StatusCode::BAD_REQUEST));
    }

    #[test]
    fn validate_accepts_valid_draft() {
        let data = CreateInvoice {
            tenant_id: Uuid::nil(),
            contact_id: Uuid::nil(),
            number: String::new(),
            issue_date: chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            due_date: chrono::NaiveDate::from_ymd_opt(2026, 6, 14).unwrap(),
            taxable_supply_date: None,
            currency: "CZK".to_string(),
            variable_symbol: None,
            status: None,
            items: vec![CreateInvoiceItem {
                description: "Consulting".to_string(),
                qty: d("1"),
                unit_price: d("100.00"),
                vat_rate: d("21"),
                vat_rate_type: None,
            }],
            country: None,
        };
        assert_eq!(validate_create_invoice(&data), Ok(()));
    }

    #[test]
    fn line_input_from_item_zero_discount() {
        let item = InvoiceItem {
            id: Uuid::nil(),
            invoice_id: Uuid::nil(),
            tenant_id: Uuid::nil(),
            description: "x".to_string(),
            qty: d("2"),
            unit_price: d("50.00"),
            vat_rate: d("20"),
            total_amount: d("120.00"),
            base_amount: d("100.00"),
            vat_amount: d("20.00"),
        };
        let li = line_input_from_item(&item);
        assert_eq!(li.qty, d("2"));
        assert_eq!(li.discount_pct, Decimal::ZERO);
    }
}

/// HTTP-level tests for `get_invoice_pdf` (`GET /api/v1/invoices/{id}/pdf`,
/// UC-ACC-05.9). These drive the real router through the auth/RLS extractors,
/// mirroring the api-server accounting suites' harness (mint an `api_core`
/// bearer + `X-Tenant-ID`, seed org/user/membership on the pool).
///
/// NOTE on cross-tenant isolation: `#[sqlx::test]` connects as the `postgres`
/// superuser (see `backend.yml` `DATABASE_URL`), which BYPASSES row-level
/// security even under `FORCE ROW LEVEL SECURITY`. Cross-org IDOR is therefore
/// verified by the dedicated `rls-smoke-test` CI job (non-superuser
/// `rls_test_runner`), exactly as the api-server `accounting_invoices_tests`
/// suite documents. Here the 404 path is exercised with an unknown id, which is
/// role-agnostic and RLS-agnostic.
#[cfg(test)]
mod http_tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{header, Request};
    use db::models::accounting::{CreateInvoice, CreateInvoiceItem, InvoiceStatus};
    use sqlx::PgPool;
    use tower::ServiceExt;

    const TEST_JWT_SECRET: &str =
        "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

    /// `AuthUser` caches its `DecodingKey` from `JWT_SECRET` on first use
    /// (`OnceLock`), so every test in this binary must agree on the secret. Set
    /// it before building any request.
    fn init_jwt_secret() {
        if std::env::var("JWT_SECRET").is_err() {
            std::env::set_var("JWT_SECRET", TEST_JWT_SECRET);
        }
    }

    /// Liberation Sans must be on disk for the 200 path to actually render a
    /// PDF; CI installs `fonts-liberation`. Absent (bare dev host), skip the
    /// render assertion rather than fail — matching the renderer smoke tests.
    fn fonts_available() -> bool {
        std::path::Path::new(&crate::pdf::font_dir())
            .join("LiberationSans-Regular.ttf")
            .exists()
    }

    fn mint(user_id: Uuid, org_id: Uuid, role: common::tenant::TenantRole) -> String {
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
        let now = chrono::Utc::now().timestamp();
        let claims = api_core::Claims {
            sub: user_id,
            exp: now + 3600,
            iat: now,
            token_type: "access".to_string(),
            tenant_id: Some(org_id),
            role: Some(role),
            email: "acc-pdf@test.internal".to_string(),
            name: "Acc PDF Test".to_string(),
        };
        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(TEST_JWT_SECRET.as_bytes()),
        )
        .expect("mint token")
    }

    async fn seed_user(pool: &PgPool) -> Uuid {
        sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO users (email, password_hash, name, status) \
             VALUES ($1, 'x', 'Test User', 'active') RETURNING id",
        )
        .bind(format!("u-{}@test.internal", Uuid::new_v4()))
        .fetch_one(pool)
        .await
        .expect("seed user")
    }

    async fn seed_org(pool: &PgPool) -> Uuid {
        sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO organizations (name, slug, contact_email, status) \
             VALUES ('Acc PDF Org', $1, $2, 'active') RETURNING id",
        )
        .bind(format!("acc-pdf-{}", Uuid::new_v4()))
        .bind(format!("{}@test-org.internal", Uuid::new_v4()))
        .fetch_one(pool)
        .await
        .expect("seed org")
    }

    async fn seed_membership(pool: &PgPool, org_id: Uuid, user_id: Uuid, role_type: &str) {
        sqlx::query(
            "INSERT INTO organization_members (organization_id, user_id, role_type, status) \
             VALUES ($1, $2, $3, 'active') ON CONFLICT DO NOTHING",
        )
        .bind(org_id)
        .bind(user_id)
        .bind(role_type)
        .execute(pool)
        .await
        .expect("seed membership");
    }

    async fn seed_contact(pool: &PgPool, tenant_id: Uuid) -> Uuid {
        sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO contact (tenant_id, name) VALUES ($1, 'Customer a.s.') RETURNING id",
        )
        .bind(tenant_id)
        .fetch_one(pool)
        .await
        .expect("seed contact")
    }

    /// Seed an issued invoice (with one line item) via the same repository the
    /// route uses, so the stored rows match production shape. Returns its id.
    async fn seed_invoice(state: &AppState, tenant_id: Uuid, contact_id: Uuid) -> Uuid {
        let data = CreateInvoice {
            tenant_id,
            contact_id,
            number: "2026/001".to_string(),
            issue_date: chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            due_date: chrono::NaiveDate::from_ymd_opt(2026, 6, 15).unwrap(),
            taxable_supply_date: Some(chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()),
            currency: "EUR".to_string(),
            variable_symbol: Some("2026001".to_string()),
            status: Some(InvoiceStatus::Issued),
            items: vec![CreateInvoiceItem {
                description: "Consulting".to_string(),
                qty: Decimal::from(1),
                unit_price: Decimal::from(100),
                vat_rate: Decimal::from(23),
                vat_rate_type: None,
            }],
            country: Some("SK".to_string()),
        };
        let mut conn = state.db.acquire().await.expect("acquire conn");
        let invoice = state
            .accounting_repo
            .create_invoice_rls(&mut conn, data)
            .await
            .expect("seed invoice");
        invoice.id
    }

    fn app(pool: PgPool) -> axum::Router {
        axum::Router::new()
            .nest("/api/v1", crate::routes::api_router())
            .with_state(AppState::new(pool))
    }

    async fn get_pdf(
        pool: PgPool,
        id: Uuid,
        org_id: Uuid,
        token: &str,
    ) -> axum::response::Response {
        app(pool)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/invoices/{id}/pdf"))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header("X-Tenant-ID", org_id.to_string())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("router response")
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn get_invoice_pdf_returns_pdf_for_manager(pool: PgPool) {
        init_jwt_secret();
        let state = AppState::new(pool.clone());

        let org_id = seed_org(&pool).await;
        let user_id = seed_user(&pool).await;
        seed_membership(&pool, org_id, user_id, "manager").await;
        let contact_id = seed_contact(&pool, org_id).await;
        let invoice_id = seed_invoice(&state, org_id, contact_id).await;

        if !fonts_available() {
            eprintln!(
                "skipping PDF-body assertions: LiberationSans not found in {}",
                crate::pdf::font_dir()
            );
            return;
        }

        let token = mint(user_id, org_id, common::tenant::TenantRole::Manager);
        let resp = get_pdf(pool, invoice_id, org_id, &token).await;

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("application/pdf")
        );
        let disposition = resp
            .headers()
            .get(header::CONTENT_DISPOSITION)
            .and_then(|v| v.to_str().ok())
            .expect("content-disposition present");
        // Sanitized filename: `2026/001` -> `2026-001.pdf`.
        assert!(
            disposition.contains("filename=\"2026-001.pdf\""),
            "unexpected disposition: {disposition}"
        );

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("read body");
        assert!(body.len() > 100, "PDF should be non-trivial");
        assert_eq!(&body[..5], b"%PDF-", "output must be a PDF");
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn get_invoice_pdf_404_for_unknown_id(pool: PgPool) {
        init_jwt_secret();

        let org_id = seed_org(&pool).await;
        let user_id = seed_user(&pool).await;
        seed_membership(&pool, org_id, user_id, "manager").await;

        let token = mint(user_id, org_id, common::tenant::TenantRole::Manager);
        let resp = get_pdf(pool, Uuid::new_v4(), org_id, &token).await;

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn get_invoice_pdf_403_below_read_min_role(pool: PgPool) {
        init_jwt_secret();
        let state = AppState::new(pool.clone());

        let org_id = seed_org(&pool).await;
        let user_id = seed_user(&pool).await;
        // Tenant is below READ_MIN_ROLE (Manager) — the DB-resolved role governs
        // authorization, not the JWT claim.
        seed_membership(&pool, org_id, user_id, "tenant").await;
        let contact_id = seed_contact(&pool, org_id).await;
        let invoice_id = seed_invoice(&state, org_id, contact_id).await;

        let token = mint(user_id, org_id, common::tenant::TenantRole::Tenant);
        let resp = get_pdf(pool, invoice_id, org_id, &token).await;

        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }
}
