use crate::state::AppState;
use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use common::TenantRole;
use db::models::accounting::PaymentMatch;
use uuid::Uuid;

/// List suggested payment matches for a specific statement line.
pub async fn list_matches(
    mut rls: RlsConnection,
    State(state): State<AppState>,
    Path(line_id): Path<Uuid>,
) -> Result<Json<Vec<PaymentMatch>>, StatusCode> {
    // Role check: Accounting is manager-only
    if !rls.is_super_admin() && !rls.has_role(TenantRole::Manager) {
        return Err(StatusCode::FORBIDDEN);
    }

    let matches = state
        .accounting_repo
        .list_payment_matches_by_line_rls(&mut **rls, line_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    rls.release().await;
    Ok(Json(matches))
}

/// Confirm a payment match.
pub async fn confirm_match(
    mut rls: RlsConnection,
    State(state): State<AppState>,
    Path(match_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    // Role check: Accounting is manager-only
    if !rls.is_super_admin() && !rls.has_role(TenantRole::Manager) {
        return Err(StatusCode::FORBIDDEN);
    }

    let tenant_id = rls.tenant_id();
    let user_id = rls.user_id();

    state
        .accounting_service
        .confirm_match(&mut **rls, tenant_id, match_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to confirm match: {}", e);
            match e {
                common::errors::AppError::NotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    rls.release().await;
    Ok(StatusCode::OK)
}

/// Reject a payment match.
pub async fn reject_match(
    mut rls: RlsConnection,
    State(state): State<AppState>,
    Path(match_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    // Role check: Accounting is manager-only
    if !rls.is_super_admin() && !rls.has_role(TenantRole::Manager) {
        return Err(StatusCode::FORBIDDEN);
    }

    let user_id = rls.user_id();

    state
        .accounting_service
        .reject_match(&mut **rls, match_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to reject match: {}", e);
            match e {
                common::errors::AppError::NotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    rls.release().await;
    Ok(StatusCode::OK)
}
