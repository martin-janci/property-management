// Epic 135: Enhanced Tenant Screening — Screening Queue
//
// See the module-level RLS note in `mod.rs` (PAP-67 / PAP-74).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use api_core::extractors::RlsConnection;
use db::models::enhanced_tenant_screening::*;

use crate::state::AppState;

/// Get pending queue items.
pub(super) async fn get_pending_queue(
    State(s): State<AppState>,
    mut rls: RlsConnection,
) -> impl IntoResponse {
    // SECURITY (audit4): manager-only; membership alone is not enough.
    if !super::is_screening_manager(&rls) {
        rls.release().await;
        return super::manager_required();
    }
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .get_pending_queue_items(&mut **rls.conn(), org_id, 50)
        .await;
    rls.release().await;

    match result {
        Ok(items) => Json(items).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Create queue item.
pub(super) async fn create_queue_item(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateScreeningQueueItem>,
) -> impl IntoResponse {
    // SECURITY (audit4): manager-only; membership alone is not enough.
    if !super::is_screening_manager(&rls) {
        rls.release().await;
        return super::manager_required();
    }
    if let Some(denied) =
        super::reject_foreign_refs(&s, &mut rls, req.screening_id, req.provider_config_id).await
    {
        rls.release().await;
        return denied;
    }
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .create_queue_item(&mut **rls.conn(), org_id, req)
        .await;
    rls.release().await;

    match result {
        Ok(item) => (StatusCode::CREATED, Json(item)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct UpdateQueueStatusRequest {
    status: String,
    error: Option<String>,
}

/// Update queue item status.
pub(super) async fn update_queue_status(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateQueueStatusRequest>,
) -> impl IntoResponse {
    // SECURITY (audit4): manager-only; membership alone is not enough.
    if !super::is_screening_manager(&rls) {
        rls.release().await;
        return super::manager_required();
    }
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .update_queue_item_status(
            &mut **rls.conn(),
            org_id,
            id,
            &req.status,
            req.error.as_deref(),
        )
        .await;
    rls.release().await;

    match result {
        Ok(true) => StatusCode::OK.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "Queue item not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
