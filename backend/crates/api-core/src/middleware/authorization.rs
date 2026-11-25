//! Authorization middleware for role-based access control.
//!
//! Provides middleware for enforcing permission requirements on routes.
//! Works with the ValidatedTenantExtractor to ensure proper authorization.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use common::TenantRole;
use std::collections::HashSet;

/// Permission requirements for route access.
///
/// Defines what roles can access specific routes using either hierarchical levels
/// or explicit role allowlists.
///
/// # Authorization Modes
///
/// This struct supports two mutually exclusive authorization modes:
///
/// ## 1. Hierarchical Mode (default)
///
/// When `allowed_roles` is `None`, authorization uses `min_role_level`.
/// Any role with `TenantRole::level() >= min_role_level` is granted access.
///
/// ```text
/// SuperAdmin (100) -> PlatformAdmin (95) -> OrgAdmin (90) -> Manager (80)
///     -> TechnicalManager (75) -> Owner (60) -> OwnerDelegate (55)
///     -> PropertyManager (50) -> RealEstateAgent (45) -> Tenant (40)
///     -> Resident (30) -> Guest (10)
/// ```
///
/// ## 2. Allowlist Mode (override)
///
/// When `allowed_roles` is `Some(...)`, the role hierarchy is **completely ignored**.
/// Only the explicitly listed roles are granted access, regardless of their level.
///
/// # Precedence Rule
///
/// **`allowed_roles` takes precedence over `min_role_level` when specified.**
///
/// This allows for non-hierarchical permissions like "only Owner and Manager"
/// without including higher-level admins, or specific cross-cutting permissions.
///
/// # Examples
///
/// ```rust,ignore
/// // Hierarchical: Manager (level 80) and above
/// Permission::min_level(80, "Manager access")
///
/// // Allowlist: Only these specific roles, ignoring hierarchy
/// Permission {
///     min_role_level: 0, // ignored when allowed_roles is Some
///     allowed_roles: Some(HashSet::from([TenantRole::Owner, TenantRole::Manager])),
///     description: "Owner or Manager only",
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Permission {
    /// Minimum role level required (based on TenantRole hierarchy).
    ///
    /// Only used when `allowed_roles` is `None`. Higher values are more restrictive.
    /// See `TenantRole::level()` for the complete hierarchy.
    pub min_role_level: u8,
    /// Explicit set of allowed roles.
    ///
    /// **When `Some`: Completely overrides `min_role_level`.**
    /// Only roles in this set are granted access, regardless of their hierarchy level.
    /// When `None`: Falls back to hierarchical authorization using `min_role_level`.
    pub allowed_roles: Option<HashSet<TenantRole>>,
    /// Human-readable description for logging and documentation.
    pub description: &'static str,
}

impl Permission {
    /// Create a permission requiring minimum role level.
    pub const fn min_level(level: u8, description: &'static str) -> Self {
        Self {
            min_role_level: level,
            allowed_roles: None,
            description,
        }
    }

    /// Check if a role satisfies this permission.
    ///
    /// # Evaluation Order
    ///
    /// 1. If `allowed_roles` is `Some`, checks if `role` is in the allowlist.
    ///    The role hierarchy (`min_role_level`) is **not consulted**.
    /// 2. If `allowed_roles` is `None`, checks if `role.level() >= min_role_level`.
    pub fn is_satisfied_by(&self, role: &TenantRole) -> bool {
        if let Some(ref allowed) = self.allowed_roles {
            allowed.contains(role)
        } else {
            role.level() >= self.min_role_level
        }
    }
}

/// Common permission levels based on TenantRole hierarchy.
pub mod permissions {
    use super::Permission;

    /// Super admin only (level 100)
    pub const SUPER_ADMIN: Permission = Permission::min_level(100, "Super admin access");

    /// Platform admin and above (level 95+)
    pub const PLATFORM_ADMIN: Permission = Permission::min_level(95, "Platform admin access");

    /// Organization admin and above (level 90+)
    pub const ORG_ADMIN: Permission = Permission::min_level(90, "Organization admin access");

    /// Manager and above (level 80+)
    pub const MANAGER: Permission = Permission::min_level(80, "Manager access");

    /// Technical manager and above (level 75+)
    pub const TECHNICAL_MANAGER: Permission = Permission::min_level(75, "Technical manager access");

    /// Owner and above (level 60+)
    pub const OWNER: Permission = Permission::min_level(60, "Owner access");

    /// Owner delegate and above (level 55+)
    pub const OWNER_DELEGATE: Permission = Permission::min_level(55, "Owner delegate access");

    /// Property manager and above (level 50+)
    pub const PROPERTY_MANAGER: Permission = Permission::min_level(50, "Property manager access");

    /// Tenant and above (level 40+)
    pub const TENANT: Permission = Permission::min_level(40, "Tenant access");

    /// Resident and above (level 30+)
    pub const RESIDENT: Permission = Permission::min_level(30, "Resident access");

    /// Any authenticated user (level 1+)
    pub const AUTHENTICATED: Permission = Permission::min_level(1, "Any authenticated user");
}

/// Authorization error response.
#[derive(Debug)]
pub struct AuthorizationError {
    pub status: StatusCode,
    pub message: &'static str,
}

impl IntoResponse for AuthorizationError {
    fn into_response(self) -> Response {
        (self.status, self.message).into_response()
    }
}

/// Middleware layer that requires a specific permission.
///
/// This is designed to be used with `axum::middleware::from_fn_with_state`
/// for routes that need permission checks.
///
/// # Example
/// ```ignore
/// use api_core::middleware::authorization::{require_permission, permissions};
///
/// let protected_routes = Router::new()
///     .route("/admin", get(admin_handler))
///     .layer(middleware::from_fn(|req, next| {
///         require_permission(permissions::MANAGER, req, next)
///     }));
/// ```
pub async fn require_permission(
    required: Permission,
    request: Request<Body>,
    next: Next,
) -> Result<Response, AuthorizationError> {
    // Get role from request extensions (set by TenantExtractor or
    // ValidatedTenantExtractor after the JWT has been authenticated).
    //
    // A MISSING role means the request was never authenticated / no tenant
    // role was resolved upstream. We MUST fail closed here: previously this
    // defaulted to `TenantRole::Guest`, but `Guest` (level 10) satisfies
    // `AUTHENTICATED` (min_level 1) and every permission at or below its
    // level, so an unauthenticated request silently cleared `require_*`
    // guards — a fail-open auth bypass. Reject with 401 instead. (A role
    // that is *explicitly* resolved to `Guest` — e.g. an authenticated
    // non-member via `TenantExtractor` — is still present in extensions and
    // flows through the normal permission check below; only the absent-role
    // fall-through is denied here.)
    let Some(role) = request.extensions().get::<TenantRole>().cloned() else {
        tracing::warn!(
            permission = required.description,
            "Authorization denied - no authenticated role in request context (fail-closed)"
        );
        return Err(AuthorizationError {
            status: StatusCode::UNAUTHORIZED,
            message: "Authentication required",
        });
    };

    if required.is_satisfied_by(&role) {
        tracing::debug!(
            role = ?role,
            permission = required.description,
            "Authorization granted"
        );
        Ok(next.run(request).await)
    } else {
        tracing::warn!(
            role = ?role,
            required_level = required.min_role_level,
            permission = required.description,
            "Authorization denied - insufficient permissions"
        );
        Err(AuthorizationError {
            status: StatusCode::FORBIDDEN,
            message: "Insufficient permissions for this operation",
        })
    }
}

/// Convenience middleware factory for manager-level access.
pub async fn require_manager(
    request: Request<Body>,
    next: Next,
) -> Result<Response, AuthorizationError> {
    require_permission(permissions::MANAGER, request, next).await
}

/// Convenience middleware factory for owner-level access.
pub async fn require_owner(
    request: Request<Body>,
    next: Next,
) -> Result<Response, AuthorizationError> {
    require_permission(permissions::OWNER, request, next).await
}

/// Convenience middleware factory for admin-level access.
pub async fn require_admin(
    request: Request<Body>,
    next: Next,
) -> Result<Response, AuthorizationError> {
    require_permission(permissions::ORG_ADMIN, request, next).await
}

/// Convenience middleware factory for platform admin access.
pub async fn require_platform_admin(
    request: Request<Body>,
    next: Next,
) -> Result<Response, AuthorizationError> {
    require_permission(permissions::PLATFORM_ADMIN, request, next).await
}

/// Convenience middleware factory for any authenticated user.
pub async fn require_authenticated(
    request: Request<Body>,
    next: Next,
) -> Result<Response, AuthorizationError> {
    require_permission(permissions::AUTHENTICATED, request, next).await
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::{body::Body, http::Request, routing::get, Router};
    use tower::ServiceExt; // for `oneshot`

    /// Minimal downstream handler — only reached if authorization GRANTS access.
    async fn ok_handler() -> &'static str {
        "reached-handler"
    }

    /// Router guarded by `require_authenticated`, with NO upstream extractor
    /// inserting a `TenantRole` into the request extensions. This models an
    /// unauthenticated request (or one where the auth/tenant extractor never
    /// ran or failed to resolve a role): the `TenantRole` extension is absent.
    fn unauthenticated_router() -> Router {
        Router::new()
            .route("/guarded", get(ok_handler))
            .layer(axum::middleware::from_fn(require_authenticated))
    }

    /// Router that injects an explicit `TenantRole` before the authorization
    /// layer runs, modelling a fully-resolved authenticated principal.
    fn router_with_role(role: TenantRole) -> Router {
        Router::new()
            .route("/guarded", get(ok_handler))
            .layer(axum::middleware::from_fn(require_authenticated))
            // `from_fn` layers run bottom-to-top, so this role-injector runs
            // BEFORE `require_authenticated` and populates the extension.
            .layer(axum::middleware::from_fn(
                move |mut req: Request<Body>, next: axum::middleware::Next| async move {
                    req.extensions_mut().insert(role);
                    next.run(req).await
                },
            ))
    }

    async fn status_of(app: Router) -> StatusCode {
        app.oneshot(
            Request::builder()
                .uri("/guarded")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
    }

    /// Regression (fail-open): a request with NO `TenantRole` in its extensions
    /// — i.e. never authenticated / role never resolved — must be REJECTED by
    /// `require_authenticated`, not silently treated as `Guest`.
    ///
    /// Before the fix, `require_permission` did `.unwrap_or(TenantRole::Guest)`,
    /// and `Guest` (level 10) satisfies `AUTHENTICATED` (min_level 1), so an
    /// unauthenticated request was GRANTED access (HTTP 200). The default must
    /// DENY: a missing role is "unauthenticated" → 401, never a fall-through to
    /// a guest identity.
    #[tokio::test]
    async fn missing_role_is_denied_not_treated_as_guest() {
        let status = status_of(unauthenticated_router()).await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "a request with no resolved TenantRole must be rejected (401), \
             NOT defaulted to Guest and admitted — that is a fail-open auth bypass"
        );
    }

    /// The fix must not break the legitimate authenticated path: a request that
    /// carries a sufficient role still reaches the handler.
    #[tokio::test]
    async fn present_sufficient_role_is_allowed() {
        let status = status_of(router_with_role(TenantRole::Manager)).await;
        assert_eq!(status, StatusCode::OK);
    }

    /// A request whose role IS resolved but is insufficient for the required
    /// permission is forbidden (403) — distinct from the unauthenticated 401.
    #[tokio::test]
    async fn present_insufficient_role_is_forbidden() {
        // `OWNER` requires level 60; `Resident` (30) is below it.
        let status = Router::new()
            .route("/guarded", get(ok_handler))
            .layer(axum::middleware::from_fn(require_owner))
            .layer(axum::middleware::from_fn(
                move |mut req: Request<Body>, next: axum::middleware::Next| async move {
                    req.extensions_mut().insert(TenantRole::Resident);
                    next.run(req).await
                },
            ));
        assert_eq!(status_of(status).await, StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_permission_levels() {
        // Super admin can access everything
        assert!(permissions::SUPER_ADMIN.is_satisfied_by(&TenantRole::SuperAdmin));
        assert!(!permissions::SUPER_ADMIN.is_satisfied_by(&TenantRole::Manager));

        // Manager can access manager-level and below
        assert!(permissions::MANAGER.is_satisfied_by(&TenantRole::Manager));
        assert!(permissions::MANAGER.is_satisfied_by(&TenantRole::SuperAdmin));
        assert!(!permissions::MANAGER.is_satisfied_by(&TenantRole::Owner));

        // Owner can access owner-level and below
        assert!(permissions::OWNER.is_satisfied_by(&TenantRole::Owner));
        assert!(permissions::OWNER.is_satisfied_by(&TenantRole::Manager));
        assert!(!permissions::OWNER.is_satisfied_by(&TenantRole::Tenant));

        // Any authenticated user
        assert!(permissions::AUTHENTICATED.is_satisfied_by(&TenantRole::Guest));
        assert!(permissions::AUTHENTICATED.is_satisfied_by(&TenantRole::Tenant));
    }
}
