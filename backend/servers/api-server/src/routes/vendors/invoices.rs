//! Vendor invoice endpoints (Story 21.4).
//!
//! See [`super`] for the RLS contract that every handler upholds.

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use common::errors::ErrorResponse;
use db::models::{InvoiceSummary, UpdateVendorInvoice, VendorInvoice};
use uuid::Uuid;

use super::shared::{
    db_error, load_invoice_for_user, not_found, CreateInvoiceRequest, InvoiceSummaryQuery,
    ListInvoicesQuery, OrgQuery, RecordPaymentRequest, RejectRequest,
};
use crate::state::AppState;

pub(super) async fn create_invoice(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(payload): Json<CreateInvoiceRequest>,
) -> Result<(StatusCode, Json<VendorInvoice>), (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .vendor_repo
        .create_invoice(&mut **rls.conn(), org_id, user_id, payload.data)
        .await
        .map(|i| (StatusCode::CREATED, Json(i)))
        .map_err(|e| db_error("Failed to create invoice", e));
    rls.release().await;
    out
}

pub(super) async fn list_invoices(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListInvoicesQuery>,
) -> Result<Json<Vec<VendorInvoice>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .vendor_repo
        .list_invoices(&mut **rls.conn(), org_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to list invoices", e));
    rls.release().await;
    out
}

pub(super) async fn get_overdue_invoices(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(_query): Query<OrgQuery>,
) -> Result<Json<Vec<VendorInvoice>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .vendor_repo
        .get_overdue_invoices(&mut **rls.conn(), org_id)
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to get overdue invoices", e));
    rls.release().await;
    out
}

pub(super) async fn get_invoice_summary(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<InvoiceSummaryQuery>,
) -> Result<Json<Vec<InvoiceSummary>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .vendor_repo
        .get_invoice_summary(&mut **rls.conn(), org_id, query.start_date, query.end_date)
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to get invoice summary", e));
    rls.release().await;
    out
}

pub(super) async fn get_invoice(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<VendorInvoice>, (StatusCode, Json<ErrorResponse>)> {
    let out = load_invoice_for_user(&state, &mut rls, id).await.map(Json);
    rls.release().await;
    out
}

pub(super) async fn update_invoice(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateVendorInvoice>,
) -> Result<Json<VendorInvoice>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = async {
        load_invoice_for_user(&state, &mut rls, id).await?;
        state
            .vendor_repo
            .update_invoice(&mut **rls.conn(), id, org_id, data)
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to update invoice", e))
    }
    .await;
    rls.release().await;
    out
}

pub(super) async fn delete_invoice(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = async {
        load_invoice_for_user(&state, &mut rls, id).await?;
        let deleted = state
            .vendor_repo
            .delete_invoice(&mut **rls.conn(), id, org_id)
            .await
            .map_err(|e| db_error("Failed to delete invoice", e))?;
        if deleted {
            Ok(StatusCode::NO_CONTENT)
        } else {
            Err(not_found("Invoice not found"))
        }
    }
    .await;
    rls.release().await;
    out
}

pub(super) async fn approve_invoice(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<VendorInvoice>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = async {
        load_invoice_for_user(&state, &mut rls, id).await?;
        state
            .vendor_repo
            .approve_invoice(&mut **rls.conn(), id, org_id, user_id)
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to approve invoice", e))
    }
    .await;
    rls.release().await;
    out
}

pub(super) async fn reject_invoice(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<RejectRequest>,
) -> Result<Json<VendorInvoice>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = async {
        load_invoice_for_user(&state, &mut rls, id).await?;
        state
            .vendor_repo
            .reject_invoice(&mut **rls.conn(), id, org_id, user_id, &data.reason)
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to reject invoice", e))
    }
    .await;
    rls.release().await;
    out
}

pub(super) async fn record_payment(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<RecordPaymentRequest>,
) -> Result<Json<VendorInvoice>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = async {
        load_invoice_for_user(&state, &mut rls, id).await?;
        state
            .vendor_repo
            .record_payment(
                &mut **rls.conn(),
                id,
                org_id,
                data.amount,
                data.method.as_deref(),
                data.reference.as_deref(),
            )
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to record payment", e))
    }
    .await;
    rls.release().await;
    out
}
