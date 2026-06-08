//! Announcement statistics routes - org statistics, unread count.

use super::shared::*;
use crate::state::AppState;
use api_core::extractors::RlsConnection;
use api_core::{AuthUser, TenantExtractor};
use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use common::errors::ErrorResponse;

/// Statistics sub-router (statistics, unread-count).
pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/statistics", get(get_statistics))
        .route("/unread-count", get(get_unread_count))
}

/// Get announcement statistics.
///
/// Requires manager-level role.
#[utoipa::path(
    get,
    path = "/api/v1/announcements/statistics",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Announcement statistics", body = StatisticsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
pub(super) async fn get_statistics(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
) -> Result<Json<StatisticsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Authorization: require manager-level role (Task 3.7)
    if !tenant.role.is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can view statistics",
            )),
        ));
    }

    let org_id = tenant.tenant_id;

    match state
        .announcement_repo
        .get_statistics_rls(&mut **rls.conn(), org_id)
        .await
    {
        Ok(statistics) => {
            rls.release().await;
            Ok(Json(StatisticsResponse { statistics }))
        }
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to get statistics: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get statistics",
                )),
            ))
        }
    }
}

/// Get unread announcement count.
#[utoipa::path(
    get,
    path = "/api/v1/announcements/unread-count",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Unread count", body = UnreadCountResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
pub(super) async fn get_unread_count(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
) -> Result<Json<UnreadCountResponse>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = tenant.tenant_id;
    let user_id = auth.user_id;

    match state
        .announcement_repo
        .count_unread_rls(&mut **rls.conn(), org_id, user_id)
        .await
    {
        Ok(unread_count) => {
            rls.release().await;
            Ok(Json(UnreadCountResponse { unread_count }))
        }
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to get unread count: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get unread count",
                )),
            ))
        }
    }
}
