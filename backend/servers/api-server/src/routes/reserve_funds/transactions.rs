//! Transaction listing/recording and inter-account transfer route surface.

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use db::models::reserve_funds::{
    FundTransaction, FundTransferRequest, RecordFundTransaction, TransactionQuery,
};
use uuid::Uuid;

use super::shared::{insert_child_error, internal_error, ApiResult};
use crate::state::AppState;

/// Transaction / transfer sub-router.
pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/{fund_id}/transactions",
            get(list_transactions).post(record_transaction),
        )
        .route("/transfers", post(transfer_funds))
}

/// List transactions.
async fn list_transactions(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
    Query(mut query): Query<TransactionQuery>,
) -> ApiResult<Json<Vec<FundTransaction>>> {
    let org_id = rls.tenant_id();
    query.fund_id = Some(fund_id);

    let out = state
        .reserve_fund_repo
        .list_transactions(&mut **rls.conn(), org_id, query)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to list transactions: {}", e)));
    rls.release().await;
    out
}

/// Record a transaction.
async fn record_transaction(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
    Json(req): Json<RecordFundTransaction>,
) -> ApiResult<Json<FundTransaction>> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .reserve_fund_repo
        .record_transaction(rls.conn(), org_id, fund_id, req, user_id)
        .await
        .map(Json)
        .map_err(|e| insert_child_error("record transaction", "Fund not found", e));
    rls.release().await;
    out
}

/// Transfer funds between accounts.
async fn transfer_funds(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<FundTransferRequest>,
) -> ApiResult<Json<(FundTransaction, FundTransaction)>> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .reserve_fund_repo
        .transfer_funds(rls.conn(), org_id, req, user_id)
        .await
        .map(Json)
        .map_err(|e| insert_child_error("transfer funds", "Fund not found", e));
    rls.release().await;
    out
}
