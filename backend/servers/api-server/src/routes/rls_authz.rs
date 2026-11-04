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

/// Optional platform-operator override for owner-or-operator routes.
///
/// Some routes serve both the resource owner and platform operators (e.g. a
/// developer reading their own API keys vs. an operator reviewing anyone's).
/// A hard [`admin_core::RequireCapability`] gate would lock the owner out, so
/// this extractor runs the *same* capability check — platform
/// `principal_kind`, MFA enrollment, active grant, recent MFA, audit row — only
/// for platform principals, and yields `PlatformOverride(true)` only when it
/// passes. Everyone else (including a platform principal without the grant or
/// without fresh MFA) gets `PlatformOverride(false)` and must pass the
/// handler's own ownership check; the denied attempt is audited by
/// `RequireCapability`.
///
/// The required capability comes from the route's
/// [`admin_core::require_capability`] layer, exactly as for
/// `RequireCapability`. A missing layer/`AdminDeps` is a wiring bug and
/// surfaces as `500` rather than silently disabling the override.
///
/// JWT `role` / `roles` claims are never consulted: platform status comes
/// from `users.principal_kind` only (the admin-core contract).
#[derive(Debug, Clone, Copy)]
pub struct PlatformOverride(pub bool);

impl From<admin_core::RequireCapability> for PlatformOverride {
    /// A satisfied hard capability gate is, by construction, an operator.
    fn from(_: admin_core::RequireCapability) -> Self {
        Self(true)
    }
}

impl axum::extract::FromRequestParts<crate::state::AppState> for PlatformOverride {
    type Rejection = axum::response::Response;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &crate::state::AppState,
    ) -> Result<Self, Self::Rejection> {
        use axum::response::IntoResponse;

        let auth = api_core::AuthUser::from_request_parts(parts, state)
            .await
            .map_err(IntoResponse::into_response)?;
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
                    .into_response()
            })?;
        let is_platform =
            user.is_some_and(|u| u.principal_kind_enum() == db::models::PrincipalKind::Platform);
        if !is_platform {
            return Ok(Self(false));
        }
        match admin_core::RequireCapability::from_request_parts(parts, state).await {
            Ok(_) => Ok(Self(true)),
            Err(e @ admin_core::AdminError::Internal(_)) => Err(e.into_response()),
            Err(_) => Ok(Self(false)),
        }
    }
}
