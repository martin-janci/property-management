//! Org-role gates for handlers that run on an [`RlsConnection`].
//!
//! `RlsConnection` is built on `ValidatedTenantExtractor`, so `rls.role()` is
//! the caller's role from their active `organization_members` row (never the
//! JWT). It proves membership but enforces no role, so manager-only mutations
//! must check it explicitly.

use api_core::extractors::RlsConnection;
use axum::{http::StatusCode, Json};
use common::errors::ErrorResponse;

/// Require a manager-class org role (manager, technical manager, org admin or
/// a platform admin). On rejection the RLS connection is released and `403`
/// is returned.
pub async fn require_manager_rls(
    rls: &mut RlsConnection,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if rls.role().is_manager() {
        return Ok(());
    }
    rls.release().await;
    Err((
        StatusCode::FORBIDDEN,
        Json(ErrorResponse::new(
            "FORBIDDEN",
            "Manager-level access required",
        )),
    ))
}

/// [`require_manager_rls`] for handlers whose error type is
/// `(StatusCode, String)`.
pub async fn require_manager_rls_plain(
    rls: &mut RlsConnection,
) -> Result<(), (StatusCode, String)> {
    require_manager_rls(rls)
        .await
        .map_err(|(status, _)| (status, "Manager-level access required".to_string()))
}

/// Whether the caller is a platform operator.
///
/// Login access tokens carry `roles`/`principal_kind`, not the singular
/// `role` claim that `AuthUser::is_platform_admin()` reads, so that check
/// alone rejects every real platform admin. The authoritative source is the
/// `users.principal_kind` column (the same one `RequestPrincipal` uses);
/// `AuthUser::is_platform_admin()` is kept for signed service tokens.
pub async fn is_platform_operator(
    state: &crate::state::AppState,
    auth: &api_core::AuthUser,
) -> Result<bool, (StatusCode, Json<ErrorResponse>)> {
    if auth.is_platform_admin() {
        return Ok(true);
    }
    let user = state
        .user_repo
        .find_by_id(auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to resolve principal kind");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
            )
        })?;
    Ok(user.is_some_and(|u| u.principal_kind_enum() == db::models::PrincipalKind::Platform))
}
