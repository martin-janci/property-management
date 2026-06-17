//! Issued invoices routes for accounting MVP (N3).

use api_core::extractors::{AuthUser, RlsConnection};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use common::TenantRole;
use db::models::accounting::{CreateInvoice, Invoice, InvoiceItem, UpdateInvoice};
use rust_decimal::Decimal;
use sqlx::Connection;
use uuid::Uuid;

use crate::state::AppState;

fn validate_create_invoice(data: &CreateInvoice) -> Result<(), StatusCode> {
    if data.number.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    if data.number.len() > 50 {
        return Err(StatusCode::BAD_REQUEST);
    }
    if data.due_date < data.issue_date {
        return Err(StatusCode::BAD_REQUEST);
    }
    if let Some(ref vs) = data.variable_symbol {
        if vs.len() > 20 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    let valid_currencies = ["CZK", "EUR"];
    if !valid_currencies.contains(&data.currency.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    for item in &data.items {
        if item.description.trim().is_empty() {
            return Err(StatusCode::BAD_REQUEST);
        }
        if item.description.len() > 255 {
            return Err(StatusCode::BAD_REQUEST);
        }
        if item.qty < Decimal::ZERO {
            return Err(StatusCode::BAD_REQUEST);
        }
        if item.unit_price < Decimal::ZERO {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    Ok(())
}

fn validate_update_invoice(data: &UpdateInvoice) -> Result<(), StatusCode> {
    if let Some(ref number) = data.number {
        if number.trim().is_empty() {
            return Err(StatusCode::BAD_REQUEST);
        }
        if number.len() > 50 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    if let Some(Some(ref vs)) = data.variable_symbol {
        if vs.len() > 20 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    if let Some(ref currency) = data.currency {
        let valid_currencies = ["CZK", "EUR"];
        if !valid_currencies.contains(&currency.as_str()) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    if let Some(paid_amount) = data.paid_amount {
        if paid_amount < Decimal::ZERO {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    // Note: due_date >= issue_date check is harder for partial updates without fetching current state,
    // but we can check if both are provided in the update.
    if let (Some(issue), Some(due)) = (data.issue_date, data.due_date) {
        if due < issue {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    Ok(())
}

/// List all invoices for the current tenant.
#[utoipa::path(
    get,
    path = "/api/v1/accounting/invoices",
    responses(
        (status = 200, description = "List of invoices", body = [Invoice]),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "accounting"
)]
pub async fn list_invoices(
    mut rls: RlsConnection,
    State(state): State<AppState>,
    AuthUser { user_id: _user, .. }: AuthUser,
) -> Result<Json<Vec<Invoice>>, StatusCode> {
    // Role check: Accounting is manager-only
    if !rls.is_super_admin() && !rls.has_role(TenantRole::Manager) {
        return Err(StatusCode::FORBIDDEN);
    }

    let invoices = state
        .accounting_repo
        .list_invoices_rls(&mut **rls)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list invoices: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    rls.release().await;
    Ok(Json(invoices))
}

/// Get a single invoice by ID.
#[utoipa::path(
    get,
    path = "/api/v1/accounting/invoices/{id}",
    params(
        ("id" = Uuid, Path, description = "Invoice ID")
    ),
    responses(
        (status = 200, description = "Invoice details", body = Invoice),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Invoice not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "accounting"
)]
pub async fn get_invoice(
    Path(id): Path<Uuid>,
    mut rls: RlsConnection,
    State(state): State<AppState>,
    AuthUser { user_id: _user, .. }: AuthUser,
) -> Result<Json<Invoice>, StatusCode> {
    // Role check: Accounting is manager-only
    if !rls.is_super_admin() && !rls.has_role(TenantRole::Manager) {
        return Err(StatusCode::FORBIDDEN);
    }

    let invoice = state
        .accounting_repo
        .find_invoice_rls(&mut **rls, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to find invoice {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    rls.release().await;
    Ok(Json(invoice))
}

/// List items for an invoice.
#[utoipa::path(
    get,
    path = "/api/v1/accounting/invoices/{id}/items",
    params(
        ("id" = Uuid, Path, description = "Invoice ID")
    ),
    responses(
        (status = 200, description = "List of invoice items", body = [InvoiceItem]),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "accounting"
)]
pub async fn list_invoice_items(
    Path(id): Path<Uuid>,
    mut rls: RlsConnection,
    State(state): State<AppState>,
    AuthUser { user_id: _user, .. }: AuthUser,
) -> Result<Json<Vec<InvoiceItem>>, StatusCode> {
    // Role check: Accounting is manager-only
    if !rls.is_super_admin() && !rls.has_role(TenantRole::Manager) {
        return Err(StatusCode::FORBIDDEN);
    }

    let items = state
        .accounting_repo
        .find_invoice_items_rls(&mut **rls, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list invoice items for {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    rls.release().await;
    Ok(Json(items))
}

/// Create a new invoice with items.
#[utoipa::path(
    post,
    path = "/api/v1/accounting/invoices",
    request_body = CreateInvoice,
    responses(
        (status = 201, description = "Invoice created", body = Invoice),
        (status = 400, description = "Bad request (validation)"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (no tenant or role)"),
        (status = 500, description = "Internal server error")
    ),
    tag = "accounting"
)]
pub async fn create_invoice(
    mut rls: RlsConnection,
    State(state): State<AppState>,
    AuthUser {
        user_id: _user_id,
        tenant_id,
        ..
    }: AuthUser,
    Json(mut data): Json<CreateInvoice>,
) -> Result<(StatusCode, Json<Invoice>), StatusCode> {
    let tenant_id = tenant_id.ok_or(StatusCode::FORBIDDEN)?;

    // Role check: Accounting is manager-only
    if !rls.is_super_admin() && !rls.has_role(TenantRole::Manager) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Force tenant_id from session
    data.tenant_id = tenant_id;

    // Semantic validation
    validate_create_invoice(&data)?;

    // Use a transaction for multiple queries
    let mut tx = rls.begin().await.map_err(|e| {
        tracing::error!("Failed to start transaction: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Validate contact_id ownership
    let contact = state
        .accounting_repo
        .find_contact_rls(&mut *tx, data.contact_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to find contact: {}", e);
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
            tracing::error!("Failed to create invoice: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit transaction: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    rls.release().await;
    Ok((StatusCode::CREATED, Json(invoice)))
}

/// Update an existing invoice.
#[utoipa::path(
    patch,
    path = "/api/v1/accounting/invoices/{id}",
    params(
        ("id" = Uuid, Path, description = "Invoice ID")
    ),
    request_body = UpdateInvoice,
    responses(
        (status = 200, description = "Invoice updated", body = Invoice),
        (status = 400, description = "Bad request (validation)"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (role)"),
        (status = 404, description = "Invoice not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "accounting"
)]
pub async fn update_invoice(
    Path(id): Path<Uuid>,
    mut rls: RlsConnection,
    State(state): State<AppState>,
    AuthUser { user_id: _user, .. }: AuthUser,
    Json(data): Json<UpdateInvoice>,
) -> Result<Json<Invoice>, StatusCode> {
    // Role check: Accounting is manager-only
    if !rls.is_super_admin() && !rls.has_role(TenantRole::Manager) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Semantic validation
    validate_update_invoice(&data)?;

    if let Some(contact_id) = data.contact_id {
        let contact = state
            .accounting_repo
            .find_contact_rls(&mut **rls, contact_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to find contact: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        if contact.is_none() {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let invoice = state
        .accounting_repo
        .update_invoice_rls(&mut **rls, id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update invoice {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    rls.release().await;
    Ok(Json(invoice))
}

/// Delete an invoice.
#[utoipa::path(
    delete,
    path = "/api/v1/accounting/invoices/{id}",
    params(
        ("id" = Uuid, Path, description = "Invoice ID")
    ),
    responses(
        (status = 204, description = "Invoice deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (role)"),
        (status = 404, description = "Invoice not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "accounting"
)]
pub async fn delete_invoice(
    Path(id): Path<Uuid>,
    mut rls: RlsConnection,
    State(state): State<AppState>,
    AuthUser { user_id: _user, .. }: AuthUser,
) -> Result<StatusCode, StatusCode> {
    // Role check: Accounting is manager-only
    if !rls.is_super_admin() && !rls.has_role(TenantRole::Manager) {
        return Err(StatusCode::FORBIDDEN);
    }

    state
        .accounting_repo
        .delete_invoice_rls(&mut **rls, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete invoice {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    rls.release().await;
    Ok(StatusCode::NO_CONTENT)
}
