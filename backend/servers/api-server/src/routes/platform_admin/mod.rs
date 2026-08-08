//!
//! Routes for platform-wide administrative operations including
//! organization management, feature flags, system health, and announcements.
//!
//! This module is split by concern into submodules:
//! - [`tenants`] — organization management + platform statistics.
//! - [`features`] — feature flags (admin CRUD + public resolution).
//! - [`ops`] — system health monitoring, announcements, and maintenance.
//! - [`audit`] — support tooling (user diagnostics, support data) + onboarding
//!   config.
//!
//! The split is a pure structural move — all routes, handlers, and behaviour
//! are identical to the previous single-file `platform_admin.rs`. The public
//! surface (router constructors + [`extract_super_admin_token`]) is preserved.

pub mod audit;
pub mod features;
pub mod oauth;
pub mod ops;
pub mod settings;
pub mod tenants;

use admin_core::{require_capability, Capability};
use axum::{
    http::StatusCode,
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use uuid::Uuid;

use crate::state::AppState;

// Re-export the handlers so they stay reachable as
// `routes::platform_admin::<handler>` (e.g. for any external references) and so
// the router constructors below can refer to them unqualified.
pub use audit::{
    get_onboarding_config, get_support_data, get_user_activity, get_user_for_support,
    get_user_memberships, get_user_sessions, revoke_user_sessions, search_users_for_support,
};
pub use features::{
    create_feature_flag, create_feature_flag_override, delete_feature_flag,
    delete_feature_flag_override, get_feature_flag, get_resolved_feature_flags, list_feature_flags,
    toggle_feature_flag, update_feature_flag,
};
pub use oauth::get_oauth_token_usage;
pub use ops::{
    acknowledge_alert, acknowledge_announcement, create_system_announcement,
    delete_scheduled_maintenance, delete_system_announcement, get_active_announcements,
    get_health_alerts, get_health_dashboard, get_metric_history, get_system_announcement,
    get_thresholds, get_upcoming_maintenance, get_upcoming_maintenance_admin,
    list_system_announcements, schedule_maintenance, update_system_announcement, update_threshold,
};
pub use settings::{get_platform_settings, update_platform_settings};
pub use tenants::{
    get_organization, get_platform_stats, list_organizations, reactivate_organization,
    suspend_organization,
};

/// Create platform admin router.
///
/// Phase 5 addendum: each route carries a `RequireCapability` layer so the
/// platform-principal + MFA + active-grant triple is enforced on every call.
/// The pre-existing JWT-role checks (inside each handler) remain as defence
/// in depth — capabilities are additive.
pub fn router() -> Router<AppState> {
    Router::new()
        // Organization management (Story 10B.1)
        .route(
            "/organizations",
            get(list_organizations).layer(require_capability(Capability::AgenciesRead)),
        )
        .route(
            "/organizations/{id}",
            get(get_organization).layer(require_capability(Capability::AgenciesRead)),
        )
        .route(
            "/organizations/{id}/suspend",
            post(suspend_organization).layer(require_capability(Capability::AgenciesSuspend)),
        )
        .route(
            "/organizations/{id}/reactivate",
            post(reactivate_organization).layer(require_capability(Capability::AgenciesSuspend)),
        )
        .route(
            "/stats",
            get(get_platform_stats).layer(require_capability(Capability::AuditRead)),
        )
        // OAuth token-usage analytics (Epic 10A — data audit, follow-up #2628)
        .route(
            "/oauth/token-usage",
            get(get_oauth_token_usage).layer(require_capability(Capability::AuditRead)),
        )
        // Feature flag management (Story 10B.2)
        .route(
            "/feature-flags",
            get(list_feature_flags).layer(require_capability(Capability::FeatureFlagsWrite)),
        )
        .route(
            "/feature-flags",
            post(create_feature_flag).layer(require_capability(Capability::FeatureFlagsWrite)),
        )
        .route(
            "/feature-flags/{id}",
            get(get_feature_flag).layer(require_capability(Capability::FeatureFlagsWrite)),
        )
        .route(
            "/feature-flags/{id}",
            put(update_feature_flag).layer(require_capability(Capability::FeatureFlagsWrite)),
        )
        .route(
            "/feature-flags/{id}",
            delete(delete_feature_flag).layer(require_capability(Capability::FeatureFlagsWrite)),
        )
        .route(
            "/feature-flags/{id}/toggle",
            post(toggle_feature_flag).layer(require_capability(Capability::FeatureFlagsWrite)),
        )
        .route(
            "/feature-flags/{id}/overrides",
            post(create_feature_flag_override)
                .layer(require_capability(Capability::FeatureFlagsWrite)),
        )
        .route(
            "/feature-flags/{id}/overrides/{override_id}",
            delete(delete_feature_flag_override)
                .layer(require_capability(Capability::FeatureFlagsWrite)),
        )
        // Health monitoring (Story 10B.3)
        .route(
            "/health/dashboard",
            get(get_health_dashboard).layer(require_capability(Capability::AuditRead)),
        )
        .route(
            "/health/metrics/{name}/history",
            get(get_metric_history).layer(require_capability(Capability::AuditRead)),
        )
        .route(
            "/health/alerts",
            get(get_health_alerts).layer(require_capability(Capability::AuditRead)),
        )
        .route(
            "/health/alerts/{id}/acknowledge",
            post(acknowledge_alert).layer(require_capability(Capability::SiteSettingsWrite)),
        )
        .route(
            "/health/thresholds",
            get(get_thresholds).layer(require_capability(Capability::AuditRead)),
        )
        .route(
            "/health/thresholds/{name}",
            put(update_threshold).layer(require_capability(Capability::SiteSettingsWrite)),
        )
        // System announcements (Story 10B.4)
        .route(
            "/announcements",
            get(list_system_announcements).layer(require_capability(Capability::SiteSettingsRead)),
        )
        .route(
            "/announcements",
            post(create_system_announcement)
                .layer(require_capability(Capability::SiteSettingsWrite)),
        )
        .route(
            "/announcements/{id}",
            get(get_system_announcement).layer(require_capability(Capability::SiteSettingsRead)),
        )
        .route(
            "/announcements/{id}",
            put(update_system_announcement)
                .layer(require_capability(Capability::SiteSettingsWrite)),
        )
        .route(
            "/announcements/{id}",
            delete(delete_system_announcement)
                .layer(require_capability(Capability::SiteSettingsWrite)),
        )
        .route(
            "/maintenance",
            post(schedule_maintenance).layer(require_capability(Capability::SiteSettingsWrite)),
        )
        .route(
            "/maintenance",
            get(get_upcoming_maintenance_admin)
                .layer(require_capability(Capability::SiteSettingsRead)),
        )
        .route(
            "/maintenance/{id}",
            delete(delete_scheduled_maintenance)
                .layer(require_capability(Capability::SiteSettingsWrite)),
        )
        // Support data access (Story 10B.5)
        .route(
            "/support-data",
            get(get_support_data).layer(require_capability(Capability::AuditRead)),
        )
        .route(
            "/support/users",
            get(search_users_for_support).layer(require_capability(Capability::UsersRead)),
        )
        .route(
            "/support/users/{id}",
            get(get_user_for_support).layer(require_capability(Capability::UsersRead)),
        )
        .route(
            "/support/users/{id}/memberships",
            get(get_user_memberships).layer(require_capability(Capability::UsersRead)),
        )
        .route(
            "/support/users/{id}/sessions",
            get(get_user_sessions).layer(require_capability(Capability::UsersRead)),
        )
        .route(
            "/support/users/{id}/sessions/revoke",
            post(revoke_user_sessions).layer(require_capability(Capability::UsersWrite)),
        )
        .route(
            "/support/users/{id}/activity",
            get(get_user_activity).layer(require_capability(Capability::AuditRead)),
        )
        // Onboarding config (Story 10B.6)
        .route(
            "/onboarding-config",
            get(get_onboarding_config).layer(require_capability(Capability::SiteSettingsRead)),
        )
        // Platform settings (global operator settings — admin-web /admin/platform)
        .route(
            "/settings",
            get(get_platform_settings).layer(require_capability(Capability::SiteSettingsRead)),
        )
        .route(
            "/settings",
            patch(update_platform_settings)
                .layer(require_capability(Capability::SiteSettingsWrite)),
        )
        // Agency provisioning (Phase 1: Tenant Resolution).
        // Merged in from `agency_provisioning` so the new
        // `POST /api/v1/platform-admin/agencies` endpoint is picked up by the
        // existing `.nest("/api/v1/platform-admin", ...)` in `lib.rs`.
        .merge(super::agency_provisioning::router())
}

/// Create public announcements router (for regular users).
pub fn public_announcements_router() -> Router<AppState> {
    Router::new()
        .route("/active", get(get_active_announcements))
        .route("/{id}/acknowledge", post(acknowledge_announcement))
}

/// Create public maintenance router (for regular users).
pub fn public_maintenance_router() -> Router<AppState> {
    Router::new().route("/upcoming", get(get_upcoming_maintenance))
}

/// Create public feature flags router (for regular users).
pub fn public_feature_flags_router() -> Router<AppState> {
    Router::new().route("/", get(get_resolved_feature_flags))
}

// ==================== Helper Functions ====================

/// Super admin role names.
const SUPER_ADMIN_ROLES: &[&str] = &[
    "SuperAdministrator",
    "super_admin",
    "superadmin",
    "platform_admin",
];

/// Check if the user has super admin role.
fn has_super_admin_role(roles: &Option<Vec<String>>) -> bool {
    match roles {
        Some(user_roles) => user_roles.iter().any(|r| {
            SUPER_ADMIN_ROLES
                .iter()
                .any(|admin| r.eq_ignore_ascii_case(admin))
        }),
        None => false,
    }
}

/// Extract and validate super admin access token.
///
/// `pub(crate)` so sibling route modules (e.g. `agency_provisioning`) that are
/// merged into [`router`] can reuse the exact same platform-admin auth gate.
pub(crate) fn extract_super_admin_token(
    headers: &axum::http::HeaderMap,
    state: &AppState,
) -> Result<(Uuid, String), (StatusCode, Json<ErrorResponse>)> {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "MISSING_TOKEN",
                    "Authorization header required",
                )),
            )
        })?;

    if !auth_header.starts_with("Bearer ") {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Bearer token required")),
        ));
    }

    let token = &auth_header[7..];
    let claims = state
        .jwt_service
        .validate_access_token(token)
        .map_err(|e| {
            tracing::debug!(error = %e, "Invalid access token");
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "INVALID_TOKEN",
                    "Invalid or expired token",
                )),
            )
        })?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Check if user has super admin role
    if !has_super_admin_role(&claims.roles) {
        tracing::warn!(
            user_id = %user_id,
            email = %claims.email,
            roles = ?claims.roles,
            "Unauthorized platform admin access attempt"
        );
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "INSUFFICIENT_PERMISSIONS",
                "Super Admin role required to access platform admin endpoints",
            )),
        ));
    }

    Ok((user_id, claims.email))
}
