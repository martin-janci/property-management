//! Platform Admin routes (Epic 10B).
//!
//! Routes for platform-wide administrative operations including
//! organization management, feature flags, system health, and announcements.

use admin_core::{require_capability, Capability, RequireCapability};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{
    AdminOrganizationDetail, AdminOrganizationSummary, CreateFeatureFlagOverrideRequest,
    CreateFeatureFlagRequest, FeatureFlag, ReactivateOrganizationRequest,
    SuspendOrganizationRequest,
};
use db::models::{
    CreateMaintenanceRequest, CreateSystemAnnouncementRequest, MetricThreshold,
    ScheduledMaintenance, SystemAnnouncement, SystemAnnouncementAcknowledgment,
};
use db::repositories::{
    ActiveAnnouncement, FaultStatusCount, FeatureFlagWithCount, FeatureFlagWithOverrides,
    HealthDashboard, MetricHistory, OnboardingTour, PlatformStats, ResolvedFeatureFlag,
    SupportActivityLog, SupportData, SupportUserInfo, SupportUserMembership, SupportUserSession,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::state::AppState;

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

// ==================== Types ====================

/// Query parameters for listing organizations.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ListOrganizationsQuery {
    /// Page number (1-based)
    #[serde(default = "default_page")]
    pub page: u32,
    /// Page size (max 100)
    #[serde(default = "default_page_size")]
    pub page_size: u32,
    /// Filter by status (active, suspended)
    pub status: Option<String>,
    /// Search by name or slug
    pub search: Option<String>,
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    20
}

/// Response for list organizations endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct ListOrganizationsResponse {
    /// Organizations on this page
    pub organizations: Vec<AdminOrganizationSummary>,
    /// Total number of organizations matching criteria
    pub total: i64,
    /// Current page number
    pub page: u32,
    /// Page size
    pub page_size: u32,
}

/// Response for organization actions.
#[derive(Debug, Serialize, ToSchema)]
pub struct OrganizationActionResponse {
    /// Success message
    pub message: String,
    /// Updated organization details
    pub organization: AdminOrganizationDetail,
}

/// Platform stats response.
#[derive(Debug, Serialize, ToSchema)]
pub struct PlatformStatsResponse {
    pub stats: PlatformStats,
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

// ==================== Endpoints ====================

/// List all organizations with metrics (platform admin view).
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/organizations",
    tag = "Platform Admin",
    security(("bearer_auth" = [])),
    params(
        ("page" = Option<u32>, Query, description = "Page number (1-based)"),
        ("page_size" = Option<u32>, Query, description = "Page size (max 100)"),
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("search" = Option<String>, Query, description = "Search by name or slug")
    ),
    responses(
        (status = 200, description = "Organizations retrieved", body = ListOrganizationsResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse)
    )
)]
pub async fn list_organizations(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<ListOrganizationsQuery>,
) -> Result<Json<ListOrganizationsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (_admin_id, _admin_email) = extract_super_admin_token(&headers, &state)?;

    // Validate page size
    let page_size = query.page_size.clamp(1, 100);
    let page = query.page.max(1);
    let offset = ((page - 1) * page_size) as i64;

    // Get organizations with metrics
    let (orgs, total) = match state
        .platform_admin_repo
        .list_organizations_with_metrics(
            offset,
            page_size as i64,
            query.status.as_deref(),
            query.search.as_deref(),
        )
        .await
    {
        Ok(result) => result,
        Err(e) => {
            tracing::error!(error = %e, "Failed to list organizations");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to list organizations",
                )),
            ));
        }
    };

    let summaries: Vec<AdminOrganizationSummary> = orgs
        .into_iter()
        .map(AdminOrganizationSummary::from)
        .collect();

    Ok(Json(ListOrganizationsResponse {
        organizations: summaries,
        total,
        page,
        page_size,
    }))
}

/// Get organization details with metrics.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/organizations/{id}",
    tag = "Platform Admin",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Organization ID")
    ),
    responses(
        (status = 200, description = "Organization retrieved", body = AdminOrganizationDetail),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn get_organization(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<AdminOrganizationDetail>, (StatusCode, Json<ErrorResponse>)> {
    let (_admin_id, _admin_email) = extract_super_admin_token(&headers, &state)?;

    let org_id: Uuid = id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_ID",
                "Invalid organization ID format",
            )),
        )
    })?;

    let org = match state
        .platform_admin_repo
        .get_organization_detail(org_id)
        .await
    {
        Ok(Some(org)) => org,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "ORG_NOT_FOUND",
                    "Organization not found",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get organization");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get organization",
                )),
            ));
        }
    };

    Ok(Json(org))
}

/// Suspend an organization.
#[utoipa::path(
    post,
    path = "/api/v1/platform-admin/organizations/{id}/suspend",
    tag = "Platform Admin",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Organization ID to suspend")
    ),
    request_body = SuspendOrganizationRequest,
    responses(
        (status = 200, description = "Organization suspended", body = OrganizationActionResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse),
        (status = 409, description = "Organization already suspended", body = ErrorResponse)
    )
)]
pub async fn suspend_organization(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<SuspendOrganizationRequest>,
) -> Result<Json<OrganizationActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (admin_id, admin_email) = extract_super_admin_token(&headers, &state)?;

    let org_id: Uuid = id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_ID",
                "Invalid organization ID format",
            )),
        )
    })?;

    // Suspend the organization
    let _org = match state
        .platform_admin_repo
        .suspend_organization(org_id, admin_id, &req.reason)
        .await
    {
        Ok(Some(org)) => org,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "ORG_NOT_FOUND",
                    "Organization not found or already suspended",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to suspend organization");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to suspend organization",
                )),
            ));
        }
    };

    // Get all org member user IDs for session invalidation
    let user_ids = state
        .platform_admin_repo
        .get_org_member_user_ids(org_id)
        .await
        .unwrap_or_default();

    // Revoke all sessions for org members
    for user_id in &user_ids {
        if let Err(e) = state
            .session_repo
            .revoke_all_user_tokens(*user_id, None)
            .await
        {
            tracing::warn!(
                user_id = %user_id,
                error = %e,
                "Failed to revoke sessions for org member"
            );
        }
    }

    tracing::info!(
        org_id = %org_id,
        admin_id = %admin_id,
        admin_email = %admin_email,
        reason = %req.reason,
        affected_users = user_ids.len(),
        "Organization suspended"
    );

    // Get updated organization details
    let org_detail = state
        .platform_admin_repo
        .get_organization_detail(org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get org details after suspension");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Organization suspended but failed to retrieve updated details",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Organization suspended but not found",
                )),
            )
        })?;

    Ok(Json(OrganizationActionResponse {
        message: format!(
            "Organization suspended successfully. {} user sessions revoked.",
            user_ids.len()
        ),
        organization: org_detail,
    }))
}

/// Reactivate a suspended organization.
#[utoipa::path(
    post,
    path = "/api/v1/platform-admin/organizations/{id}/reactivate",
    tag = "Platform Admin",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Organization ID to reactivate")
    ),
    request_body = ReactivateOrganizationRequest,
    responses(
        (status = 200, description = "Organization reactivated", body = OrganizationActionResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse),
        (status = 409, description = "Organization not suspended", body = ErrorResponse)
    )
)]
pub async fn reactivate_organization(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<ReactivateOrganizationRequest>,
) -> Result<Json<OrganizationActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (admin_id, admin_email) = extract_super_admin_token(&headers, &state)?;

    let org_id: Uuid = id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_ID",
                "Invalid organization ID format",
            )),
        )
    })?;

    // Reactivate the organization
    let _org = match state
        .platform_admin_repo
        .reactivate_organization(org_id)
        .await
    {
        Ok(Some(org)) => org,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "ORG_NOT_FOUND",
                    "Organization not found or not suspended",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to reactivate organization");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to reactivate organization",
                )),
            ));
        }
    };

    tracing::info!(
        org_id = %org_id,
        admin_id = %admin_id,
        admin_email = %admin_email,
        note = ?req.note,
        "Organization reactivated"
    );

    // Get updated organization details
    let org_detail = state
        .platform_admin_repo
        .get_organization_detail(org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get org details after reactivation");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Organization reactivated but failed to retrieve updated details",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Organization reactivated but not found",
                )),
            )
        })?;

    Ok(Json(OrganizationActionResponse {
        message: "Organization reactivated successfully".to_string(),
        organization: org_detail,
    }))
}

/// Get platform-wide statistics.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/stats",
    tag = "Platform Admin",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Platform stats retrieved", body = PlatformStatsResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse)
    )
)]
pub async fn get_platform_stats(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<PlatformStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (_admin_id, _admin_email) = extract_super_admin_token(&headers, &state)?;

    let stats = state
        .platform_admin_repo
        .get_platform_stats()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get platform stats");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get platform stats",
                )),
            )
        })?;

    Ok(Json(PlatformStatsResponse { stats }))
}

// ==================== Feature Flag Endpoints (Story 10B.2) ====================

/// Feature flag response types.
#[derive(Debug, Serialize, ToSchema)]
pub struct ListFeatureFlagsResponse {
    pub flags: Vec<FeatureFlagWithCount>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FeatureFlagResponse {
    pub flag: FeatureFlag,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FeatureFlagDetailResponse {
    pub flag: FeatureFlagWithOverrides,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ResolvedFlagsResponse {
    pub flags: Vec<ResolvedFeatureFlag>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateFeatureFlagRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_enabled: Option<bool>,
}

/// List all feature flags.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/feature-flags",
    tag = "Platform Admin - Feature Flags",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Feature flags retrieved", body = ListFeatureFlagsResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse)
    )
)]
pub async fn list_feature_flags(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ListFeatureFlagsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (_admin_id, _admin_email) = extract_super_admin_token(&headers, &state)?;

    let flags = state.feature_flag_repo.list_all().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to list feature flags");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to list feature flags",
            )),
        )
    })?;

    Ok(Json(ListFeatureFlagsResponse { flags }))
}

/// Create a new feature flag.
#[utoipa::path(
    post,
    path = "/api/v1/platform-admin/feature-flags",
    tag = "Platform Admin - Feature Flags",
    security(("bearer_auth" = [])),
    request_body = CreateFeatureFlagRequest,
    responses(
        (status = 201, description = "Feature flag created", body = FeatureFlagResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 409, description = "Flag key already exists", body = ErrorResponse)
    )
)]
pub async fn create_feature_flag(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateFeatureFlagRequest>,
) -> Result<(StatusCode, Json<FeatureFlagResponse>), (StatusCode, Json<ErrorResponse>)> {
    let (admin_id, admin_email) = extract_super_admin_token(&headers, &state)?;

    let flag = state
        .feature_flag_repo
        .create(
            &req.key,
            &req.name,
            req.description.as_deref(),
            req.is_enabled,
        )
        .await
        .map_err(|e| {
            if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
                (
                    StatusCode::CONFLICT,
                    Json(ErrorResponse::new(
                        "DUPLICATE_KEY",
                        "Feature flag key already exists",
                    )),
                )
            } else {
                tracing::error!(error = %e, "Failed to create feature flag");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "DATABASE_ERROR",
                        "Failed to create feature flag",
                    )),
                )
            }
        })?;

    tracing::info!(
        flag_key = %req.key,
        admin_id = %admin_id,
        admin_email = %admin_email,
        "Feature flag created"
    );

    Ok((StatusCode::CREATED, Json(FeatureFlagResponse { flag })))
}

/// Get a feature flag with all overrides.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/feature-flags/{id}",
    tag = "Platform Admin - Feature Flags",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Feature flag ID")
    ),
    responses(
        (status = 200, description = "Feature flag retrieved", body = FeatureFlagDetailResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Feature flag not found", body = ErrorResponse)
    )
)]
pub async fn get_feature_flag(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<FeatureFlagDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (_admin_id, _admin_email) = extract_super_admin_token(&headers, &state)?;

    let flag_id: Uuid = id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_ID",
                "Invalid feature flag ID format",
            )),
        )
    })?;

    let flag = state
        .feature_flag_repo
        .get_by_id(flag_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get feature flag");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get feature flag",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "FLAG_NOT_FOUND",
                    "Feature flag not found",
                )),
            )
        })?;

    Ok(Json(FeatureFlagDetailResponse { flag }))
}

/// Update a feature flag.
#[utoipa::path(
    put,
    path = "/api/v1/platform-admin/feature-flags/{id}",
    tag = "Platform Admin - Feature Flags",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Feature flag ID")
    ),
    request_body = UpdateFeatureFlagRequest,
    responses(
        (status = 200, description = "Feature flag updated", body = FeatureFlagResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Feature flag not found", body = ErrorResponse)
    )
)]
pub async fn update_feature_flag(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UpdateFeatureFlagRequest>,
) -> Result<Json<FeatureFlagResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (admin_id, admin_email) = extract_super_admin_token(&headers, &state)?;

    let flag_id: Uuid = id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_ID",
                "Invalid feature flag ID format",
            )),
        )
    })?;

    let flag = state
        .feature_flag_repo
        .update(
            flag_id,
            req.name.as_deref(),
            req.description.as_deref(),
            req.is_enabled,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update feature flag");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to update feature flag",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "FLAG_NOT_FOUND",
                    "Feature flag not found",
                )),
            )
        })?;

    tracing::info!(
        flag_id = %flag_id,
        admin_id = %admin_id,
        admin_email = %admin_email,
        "Feature flag updated"
    );

    Ok(Json(FeatureFlagResponse { flag }))
}

/// Delete a feature flag.
#[utoipa::path(
    delete,
    path = "/api/v1/platform-admin/feature-flags/{id}",
    tag = "Platform Admin - Feature Flags",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Feature flag ID")
    ),
    responses(
        (status = 204, description = "Feature flag deleted"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Feature flag not found", body = ErrorResponse)
    )
)]
pub async fn delete_feature_flag(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let (admin_id, admin_email) = extract_super_admin_token(&headers, &state)?;

    let flag_id: Uuid = id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_ID",
                "Invalid feature flag ID format",
            )),
        )
    })?;

    let deleted = state.feature_flag_repo.delete(flag_id).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to delete feature flag");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to delete feature flag",
            )),
        )
    })?;

    if !deleted {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "FLAG_NOT_FOUND",
                "Feature flag not found",
            )),
        ));
    }

    tracing::info!(
        flag_id = %flag_id,
        admin_id = %admin_id,
        admin_email = %admin_email,
        "Feature flag deleted"
    );

    Ok(StatusCode::NO_CONTENT)
}

/// Toggle a feature flag's global enabled state.
#[utoipa::path(
    post,
    path = "/api/v1/platform-admin/feature-flags/{id}/toggle",
    tag = "Platform Admin - Feature Flags",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Feature flag ID")
    ),
    responses(
        (status = 200, description = "Feature flag toggled", body = FeatureFlagResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Feature flag not found", body = ErrorResponse)
    )
)]
pub async fn toggle_feature_flag(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<FeatureFlagResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (admin_id, admin_email) = extract_super_admin_token(&headers, &state)?;

    let flag_id: Uuid = id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_ID",
                "Invalid feature flag ID format",
            )),
        )
    })?;

    let flag = state
        .feature_flag_repo
        .toggle(flag_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to toggle feature flag");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to toggle feature flag",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "FLAG_NOT_FOUND",
                    "Feature flag not found",
                )),
            )
        })?;

    tracing::info!(
        flag_id = %flag_id,
        new_state = flag.is_enabled,
        admin_id = %admin_id,
        admin_email = %admin_email,
        "Feature flag toggled"
    );

    Ok(Json(FeatureFlagResponse { flag }))
}

/// Create a feature flag override.
#[utoipa::path(
    post,
    path = "/api/v1/platform-admin/feature-flags/{id}/overrides",
    tag = "Platform Admin - Feature Flags",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Feature flag ID")
    ),
    request_body = CreateFeatureFlagOverrideRequest,
    responses(
        (status = 201, description = "Override created", body = db::models::FeatureFlagOverride),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Feature flag not found", body = ErrorResponse)
    )
)]
pub async fn create_feature_flag_override(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<CreateFeatureFlagOverrideRequest>,
) -> Result<(StatusCode, Json<db::models::FeatureFlagOverride>), (StatusCode, Json<ErrorResponse>)>
{
    let (admin_id, admin_email) = extract_super_admin_token(&headers, &state)?;

    let flag_id: Uuid = id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_ID",
                "Invalid feature flag ID format",
            )),
        )
    })?;

    // Verify flag exists
    if state
        .feature_flag_repo
        .get_by_id(flag_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check feature flag");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check feature flag",
                )),
            )
        })?
        .is_none()
    {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "FLAG_NOT_FOUND",
                "Feature flag not found",
            )),
        ));
    }

    let override_record = state
        .feature_flag_repo
        .create_override(
            flag_id,
            req.scope_type.clone(),
            req.scope_id,
            req.is_enabled,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create override");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to create override",
                )),
            )
        })?;

    tracing::info!(
        flag_id = %flag_id,
        scope_type = %req.scope_type,
        scope_id = %req.scope_id,
        is_enabled = req.is_enabled,
        admin_id = %admin_id,
        admin_email = %admin_email,
        "Feature flag override created"
    );

    Ok((StatusCode::CREATED, Json(override_record)))
}

/// Delete a feature flag override.
#[utoipa::path(
    delete,
    path = "/api/v1/platform-admin/feature-flags/{id}/overrides/{override_id}",
    tag = "Platform Admin - Feature Flags",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Feature flag ID"),
        ("override_id" = String, Path, description = "Override ID")
    ),
    responses(
        (status = 204, description = "Override deleted"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Override not found", body = ErrorResponse)
    )
)]
pub async fn delete_feature_flag_override(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path((id, override_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let (admin_id, admin_email) = extract_super_admin_token(&headers, &state)?;

    let _flag_id: Uuid = id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_ID",
                "Invalid feature flag ID format",
            )),
        )
    })?;

    let override_uuid: Uuid = override_id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_ID",
                "Invalid override ID format",
            )),
        )
    })?;

    let deleted = state
        .feature_flag_repo
        .delete_override(override_uuid)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to delete override");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to delete override",
                )),
            )
        })?;

    if !deleted {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "OVERRIDE_NOT_FOUND",
                "Override not found",
            )),
        ));
    }

    tracing::info!(
        override_id = %override_uuid,
        admin_id = %admin_id,
        admin_email = %admin_email,
        "Feature flag override deleted"
    );

    Ok(StatusCode::NO_CONTENT)
}

/// Get resolved feature flags for current user context (public endpoint).
#[utoipa::path(
    get,
    path = "/api/v1/feature-flags",
    tag = "Feature Flags",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Resolved flags retrieved", body = ResolvedFlagsResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn get_resolved_feature_flags(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ResolvedFlagsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Extract user context from token (any authenticated user)
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

    // Get user's org context from token if available
    let org_id = claims.org_id.and_then(|t| t.parse().ok());

    let flags = state
        .feature_flag_repo
        .resolve_all_for_context(Some(user_id), org_id, None)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to resolve feature flags");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to resolve feature flags",
                )),
            )
        })?;

    Ok(Json(ResolvedFlagsResponse { flags }))
}

// ==================== Health Monitoring Handlers (Story 10B.3) ====================

/// Query parameters for metric history.
#[derive(Debug, Deserialize, utoipa::IntoParams, ToSchema)]
pub struct MetricHistoryQuery {
    /// Time range preset: 1h, 6h, 24h, 7d, 30d
    #[serde(default = "default_time_range")]
    pub range: String,
}

fn default_time_range() -> String {
    "24h".to_string()
}

/// Query parameters for alerts.
#[derive(Debug, Deserialize, utoipa::IntoParams, ToSchema)]
pub struct AlertsQuery {
    /// Only show unacknowledged alerts
    #[serde(default)]
    pub active_only: bool,
    /// Page number (1-based)
    #[serde(default = "default_page")]
    pub page: u32,
    /// Page size (max 100)
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

/// Request to update a threshold.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateThresholdRequest {
    pub warning_threshold: Option<f64>,
    pub critical_threshold: Option<f64>,
}

/// Response for alert acknowledgment.
#[derive(Debug, Serialize, ToSchema)]
pub struct AlertAcknowledgeResponse {
    pub message: String,
    pub alert: db::models::MetricAlert,
}

/// Get health dashboard with current metrics and alerts.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/health/dashboard",
    responses(
        (status = 200, description = "Health dashboard retrieved", body = HealthDashboard),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Health"
)]
pub async fn get_health_dashboard(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<HealthDashboard>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let dashboard = state
        .health_monitoring_repo
        .get_dashboard()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get health dashboard");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get health dashboard",
                )),
            )
        })?;

    Ok(Json(dashboard))
}

/// Get historical metrics for a specific metric.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/health/metrics/{name}/history",
    params(
        ("name" = String, Path, description = "Metric name"),
        MetricHistoryQuery
    ),
    responses(
        (status = 200, description = "Metric history retrieved", body = MetricHistory),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Health"
)]
pub async fn get_metric_history(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(name): Path<String>,
    Query(query): Query<MetricHistoryQuery>,
) -> Result<Json<MetricHistory>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let now = chrono::Utc::now();
    let start_time = match query.range.as_str() {
        "1h" => now - chrono::Duration::hours(1),
        "6h" => now - chrono::Duration::hours(6),
        "24h" => now - chrono::Duration::hours(24),
        "7d" => now - chrono::Duration::days(7),
        "30d" => now - chrono::Duration::days(30),
        _ => now - chrono::Duration::hours(24), // default to 24h
    };

    let history = state
        .health_monitoring_repo
        .get_metric_history(&name, start_time, now)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, metric_name = %name, "Failed to get metric history");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get metric history",
                )),
            )
        })?;

    Ok(Json(history))
}

/// Get health alerts.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/health/alerts",
    params(AlertsQuery),
    responses(
        (status = 200, description = "Alerts retrieved", body = Vec<db::models::MetricAlert>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Health"
)]
pub async fn get_health_alerts(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<AlertsQuery>,
) -> Result<Json<Vec<db::models::MetricAlert>>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let alerts = if query.active_only {
        state.health_monitoring_repo.get_active_alerts().await
    } else {
        let offset = ((query.page.saturating_sub(1)) as i64) * (query.page_size as i64);
        state
            .health_monitoring_repo
            .get_alerts(query.page_size as i64, offset)
            .await
    }
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to get alerts");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", "Failed to get alerts")),
        )
    })?;

    Ok(Json(alerts))
}

/// Acknowledge a health alert.
#[utoipa::path(
    post,
    path = "/api/v1/platform-admin/health/alerts/{id}/acknowledge",
    params(
        ("id" = Uuid, Path, description = "Alert ID")
    ),
    responses(
        (status = 200, description = "Alert acknowledged", body = AlertAcknowledgeResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 404, description = "Alert not found or already acknowledged"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Health"
)]
pub async fn acknowledge_alert(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<AlertAcknowledgeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (admin_user_id, _) = extract_super_admin_token(&headers, &state)?;

    let alert = state
        .health_monitoring_repo
        .acknowledge_alert(id, admin_user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, alert_id = %id, "Failed to acknowledge alert");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to acknowledge alert",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "ALERT_NOT_FOUND",
                    "Alert not found or already acknowledged",
                )),
            )
        })?;

    Ok(Json(AlertAcknowledgeResponse {
        message: "Alert acknowledged".to_string(),
        alert,
    }))
}

/// Get all metric thresholds.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/health/thresholds",
    responses(
        (status = 200, description = "Thresholds retrieved", body = Vec<MetricThreshold>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Health"
)]
pub async fn get_thresholds(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Vec<MetricThreshold>>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let thresholds = state
        .health_monitoring_repo
        .get_thresholds()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get thresholds");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get thresholds",
                )),
            )
        })?;

    Ok(Json(thresholds))
}

/// Update a metric threshold.
#[utoipa::path(
    put,
    path = "/api/v1/platform-admin/health/thresholds/{name}",
    params(
        ("name" = String, Path, description = "Metric name")
    ),
    request_body = UpdateThresholdRequest,
    responses(
        (status = 200, description = "Threshold updated", body = MetricThreshold),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 404, description = "Threshold not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Health"
)]
pub async fn update_threshold(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(name): Path<String>,
    Json(request): Json<UpdateThresholdRequest>,
) -> Result<Json<MetricThreshold>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let threshold = state
        .health_monitoring_repo
        .update_threshold(&name, request.warning_threshold, request.critical_threshold)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, metric_name = %name, "Failed to update threshold");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to update threshold",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "THRESHOLD_NOT_FOUND",
                    "Metric threshold not found",
                )),
            )
        })?;

    Ok(Json(threshold))
}

// ==================== System Announcement Handlers (Story 10B.4) ====================

/// Query parameters for listing announcements.
#[derive(Debug, Deserialize, utoipa::IntoParams, ToSchema)]
pub struct ListAnnouncementsAdminQuery {
    /// Include deleted announcements
    #[serde(default)]
    pub include_deleted: bool,
}

/// Request to update an announcement.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateAnnouncementRequest {
    pub title: Option<String>,
    pub message: Option<String>,
    pub severity: Option<String>,
    pub start_at: Option<chrono::DateTime<chrono::Utc>>,
    pub end_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
    pub is_dismissible: Option<bool>,
    pub requires_acknowledgment: Option<bool>,
}

/// Response for announcement creation.
#[derive(Debug, Serialize, ToSchema)]
pub struct AnnouncementResponse {
    pub message: String,
    pub announcement: SystemAnnouncement,
}

/// Response for maintenance scheduling.
#[derive(Debug, Serialize, ToSchema)]
pub struct MaintenanceResponse {
    pub message: String,
    pub maintenance: ScheduledMaintenance,
    pub announcement: Option<SystemAnnouncement>,
}

/// Response for acknowledgment.
#[derive(Debug, Serialize, ToSchema)]
pub struct AcknowledgmentResponse {
    pub message: String,
    pub acknowledgment: SystemAnnouncementAcknowledgment,
}

/// List all system announcements (admin view).
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/announcements",
    params(ListAnnouncementsAdminQuery),
    responses(
        (status = 200, description = "Announcements retrieved", body = Vec<SystemAnnouncement>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Announcements"
)]
pub async fn list_system_announcements(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<ListAnnouncementsAdminQuery>,
) -> Result<Json<Vec<SystemAnnouncement>>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let announcements = state
        .system_announcement_repo
        .list_all(query.include_deleted)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list announcements");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to list announcements",
                )),
            )
        })?;

    Ok(Json(announcements))
}

/// Create a system announcement.
#[utoipa::path(
    post,
    path = "/api/v1/platform-admin/announcements",
    request_body = CreateSystemAnnouncementRequest,
    responses(
        (status = 201, description = "Announcement created", body = AnnouncementResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Announcements"
)]
pub async fn create_system_announcement(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(request): Json<CreateSystemAnnouncementRequest>,
) -> Result<(StatusCode, Json<AnnouncementResponse>), (StatusCode, Json<ErrorResponse>)> {
    let (admin_user_id, _) = extract_super_admin_token(&headers, &state)?;

    let announcement = state
        .system_announcement_repo
        .create_announcement(
            &request.title,
            &request.message,
            request.severity.as_str(),
            request.start_at,
            request.end_at,
            request.is_dismissible,
            request.requires_acknowledgment,
            admin_user_id,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create announcement");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to create announcement",
                )),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(AnnouncementResponse {
            message: "Announcement created".to_string(),
            announcement,
        }),
    ))
}

/// Get a specific announcement.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/announcements/{id}",
    params(
        ("id" = Uuid, Path, description = "Announcement ID")
    ),
    responses(
        (status = 200, description = "Announcement retrieved", body = SystemAnnouncement),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 404, description = "Announcement not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Announcements"
)]
pub async fn get_system_announcement(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<SystemAnnouncement>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let announcement = state
        .system_announcement_repo
        .get_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, announcement_id = %id, "Failed to get announcement");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get announcement",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "ANNOUNCEMENT_NOT_FOUND",
                    "Announcement not found",
                )),
            )
        })?;

    Ok(Json(announcement))
}

/// Update a system announcement.
#[utoipa::path(
    put,
    path = "/api/v1/platform-admin/announcements/{id}",
    params(
        ("id" = Uuid, Path, description = "Announcement ID")
    ),
    request_body = UpdateAnnouncementRequest,
    responses(
        (status = 200, description = "Announcement updated", body = SystemAnnouncement),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 404, description = "Announcement not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Announcements"
)]
pub async fn update_system_announcement(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateAnnouncementRequest>,
) -> Result<Json<SystemAnnouncement>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let announcement = state
        .system_announcement_repo
        .update_announcement(
            id,
            request.title.as_deref(),
            request.message.as_deref(),
            request.severity.as_deref(),
            request.start_at,
            request.end_at,
            request.is_dismissible,
            request.requires_acknowledgment,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, announcement_id = %id, "Failed to update announcement");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to update announcement",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "ANNOUNCEMENT_NOT_FOUND",
                    "Announcement not found",
                )),
            )
        })?;

    Ok(Json(announcement))
}

/// Delete a system announcement (soft delete).
#[utoipa::path(
    delete,
    path = "/api/v1/platform-admin/announcements/{id}",
    params(
        ("id" = Uuid, Path, description = "Announcement ID")
    ),
    responses(
        (status = 204, description = "Announcement deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 404, description = "Announcement not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Announcements"
)]
pub async fn delete_system_announcement(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let deleted = state
        .system_announcement_repo
        .delete_announcement(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, announcement_id = %id, "Failed to delete announcement");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to delete announcement",
                )),
            )
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "ANNOUNCEMENT_NOT_FOUND",
                "Announcement not found",
            )),
        ))
    }
}

/// Schedule a maintenance window.
#[utoipa::path(
    post,
    path = "/api/v1/platform-admin/maintenance",
    request_body = CreateMaintenanceRequest,
    responses(
        (status = 201, description = "Maintenance scheduled", body = MaintenanceResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Maintenance"
)]
pub async fn schedule_maintenance(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(request): Json<CreateMaintenanceRequest>,
) -> Result<(StatusCode, Json<MaintenanceResponse>), (StatusCode, Json<ErrorResponse>)> {
    let (admin_user_id, _) = extract_super_admin_token(&headers, &state)?;

    // Create announcement if requested
    let announcement = if request.create_announcement {
        let ann = state
            .system_announcement_repo
            .create_announcement(
                &format!("Scheduled Maintenance: {}", request.title),
                &format!(
                    "Maintenance is scheduled from {} to {}. {}",
                    request.start_at,
                    request.end_at,
                    request.description.as_deref().unwrap_or("")
                ),
                "warning",
                request.start_at - chrono::Duration::hours(24), // Announce 24h before
                Some(request.end_at),
                true,
                false,
                admin_user_id,
            )
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to create maintenance announcement");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "DATABASE_ERROR",
                        "Failed to create maintenance announcement",
                    )),
                )
            })?;
        Some(ann)
    } else {
        None
    };

    let maintenance = state
        .system_announcement_repo
        .schedule_maintenance(
            &request.title,
            request.description.as_deref(),
            request.start_at,
            request.end_at,
            request.is_read_only_mode,
            announcement.as_ref().map(|a| a.id),
            admin_user_id,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to schedule maintenance");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to schedule maintenance",
                )),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(MaintenanceResponse {
            message: "Maintenance scheduled".to_string(),
            maintenance,
            announcement,
        }),
    ))
}

/// Get upcoming maintenance windows (admin view).
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/maintenance",
    responses(
        (status = 200, description = "Maintenance windows retrieved", body = Vec<ScheduledMaintenance>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Maintenance"
)]
pub async fn get_upcoming_maintenance_admin(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Vec<ScheduledMaintenance>>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let maintenance = state
        .system_announcement_repo
        .get_upcoming_maintenance()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get upcoming maintenance");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get upcoming maintenance",
                )),
            )
        })?;

    Ok(Json(maintenance))
}

/// Delete a scheduled maintenance.
#[utoipa::path(
    delete,
    path = "/api/v1/platform-admin/maintenance/{id}",
    params(
        ("id" = Uuid, Path, description = "Maintenance ID")
    ),
    responses(
        (status = 204, description = "Maintenance deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 404, description = "Maintenance not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Maintenance"
)]
pub async fn delete_scheduled_maintenance(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let deleted = state
        .system_announcement_repo
        .delete_maintenance(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, maintenance_id = %id, "Failed to delete maintenance");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to delete maintenance",
                )),
            )
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "MAINTENANCE_NOT_FOUND",
                "Maintenance not found",
            )),
        ))
    }
}

// ==================== Public Announcement Handlers ====================

/// Get active announcements for current user.
#[utoipa::path(
    get,
    path = "/api/v1/announcements/active",
    responses(
        (status = 200, description = "Active announcements retrieved", body = Vec<ActiveAnnouncement>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Announcements"
)]
pub async fn get_active_announcements(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Vec<ActiveAnnouncement>>, (StatusCode, Json<ErrorResponse>)> {
    // Extract user from token
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

    let announcements = state
        .system_announcement_repo
        .get_active_for_user(user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get active announcements");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get announcements",
                )),
            )
        })?;

    Ok(Json(announcements))
}

/// Acknowledge an announcement.
#[utoipa::path(
    post,
    path = "/api/v1/announcements/{id}/acknowledge",
    params(
        ("id" = Uuid, Path, description = "Announcement ID")
    ),
    responses(
        (status = 200, description = "Announcement acknowledged", body = AcknowledgmentResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Announcements"
)]
pub async fn acknowledge_announcement(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<AcknowledgmentResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Extract user from token
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

    let acknowledgment = state
        .system_announcement_repo
        .record_acknowledgment(id, user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, announcement_id = %id, "Failed to acknowledge announcement");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to acknowledge announcement")),
            )
        })?;

    Ok(Json(AcknowledgmentResponse {
        message: "Announcement acknowledged".to_string(),
        acknowledgment,
    }))
}

/// Get upcoming maintenance (public).
#[utoipa::path(
    get,
    path = "/api/v1/maintenance/upcoming",
    responses(
        (status = 200, description = "Upcoming maintenance retrieved", body = Vec<ScheduledMaintenance>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Maintenance"
)]
pub async fn get_upcoming_maintenance(
    State(state): State<AppState>,
) -> Result<Json<Vec<ScheduledMaintenance>>, (StatusCode, Json<ErrorResponse>)> {
    let maintenance = state
        .system_announcement_repo
        .get_upcoming_maintenance()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get upcoming maintenance");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get upcoming maintenance",
                )),
            )
        })?;

    Ok(Json(maintenance))
}

// ==================== Support Data Access Handlers (Story 10B.5) ====================

/// Query parameters for user search.
#[derive(Debug, Deserialize, utoipa::IntoParams, ToSchema)]
pub struct SearchUsersQuery {
    /// Search query (email, name)
    pub query: Option<String>,
    /// Filter by status
    pub status: Option<String>,
    /// Page number (1-based)
    #[serde(default = "default_page")]
    pub page: u32,
    /// Page size (max 100)
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

/// Response for user search.
#[derive(Debug, Serialize, ToSchema)]
pub struct SearchUsersResponse {
    pub users: Vec<SupportUserInfo>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

/// User detail response for support.
#[derive(Debug, Serialize, ToSchema)]
pub struct SupportUserDetailResponse {
    pub user: SupportUserInfo,
    pub memberships: Vec<SupportUserMembership>,
    pub active_sessions: Vec<SupportUserSession>,
}

/// Response for session revocation.
#[derive(Debug, Serialize, ToSchema)]
pub struct RevokeSessionsResponse {
    pub message: String,
    pub revoked_count: i64,
}

/// Query parameters for activity log.
#[derive(Debug, Deserialize, utoipa::IntoParams, ToSchema)]
pub struct ActivityQuery {
    /// Maximum number of entries to return
    #[serde(default = "default_activity_limit")]
    pub limit: i64,
}

fn default_activity_limit() -> i64 {
    50
}

/// Search users for support purposes.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/support/users",
    params(SearchUsersQuery),
    responses(
        (status = 200, description = "Users found", body = SearchUsersResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Support"
)]
pub async fn search_users_for_support(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<SearchUsersQuery>,
) -> Result<Json<SearchUsersResponse>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let page_size = query.page_size.clamp(1, 100);
    let page = query.page.max(1);
    let offset = ((page - 1) * page_size) as i64;

    let (users, total) = state
        .platform_admin_repo
        .search_users_for_support(
            query.query.as_deref(),
            query.status.as_deref(),
            page_size as i64,
            offset,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to search users");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to search users",
                )),
            )
        })?;

    Ok(Json(SearchUsersResponse {
        users,
        total,
        page,
        page_size,
    }))
}

/// Get user details for support.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/support/users/{id}",
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User details retrieved", body = SupportUserDetailResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Support"
)]
pub async fn get_user_for_support(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<SupportUserDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let user = state
        .platform_admin_repo
        .get_user_for_support(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, user_id = %id, "Failed to get user");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to get user")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("USER_NOT_FOUND", "User not found")),
            )
        })?;

    let memberships = state
        .platform_admin_repo
        .get_user_memberships(id)
        .await
        .unwrap_or_default();

    let active_sessions = state
        .platform_admin_repo
        .get_user_sessions(id)
        .await
        .unwrap_or_default();

    Ok(Json(SupportUserDetailResponse {
        user,
        memberships,
        active_sessions,
    }))
}

/// Get user organization memberships.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/support/users/{id}/memberships",
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Memberships retrieved", body = Vec<SupportUserMembership>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Support"
)]
pub async fn get_user_memberships(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<SupportUserMembership>>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let memberships = state
        .platform_admin_repo
        .get_user_memberships(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, user_id = %id, "Failed to get memberships");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get memberships",
                )),
            )
        })?;

    Ok(Json(memberships))
}

/// Get user active sessions.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/support/users/{id}/sessions",
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Sessions retrieved", body = Vec<SupportUserSession>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Support"
)]
pub async fn get_user_sessions(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<SupportUserSession>>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let sessions = state
        .platform_admin_repo
        .get_user_sessions(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, user_id = %id, "Failed to get sessions");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get sessions",
                )),
            )
        })?;

    Ok(Json(sessions))
}

/// Revoke all user sessions.
#[utoipa::path(
    post,
    path = "/api/v1/platform-admin/support/users/{id}/sessions/revoke",
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Sessions revoked", body = RevokeSessionsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Support"
)]
pub async fn revoke_user_sessions(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<RevokeSessionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (admin_id, admin_email) = extract_super_admin_token(&headers, &state)?;

    let revoked_count = state
        .platform_admin_repo
        .revoke_user_sessions(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, user_id = %id, "Failed to revoke sessions");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to revoke sessions",
                )),
            )
        })?;

    tracing::info!(
        user_id = %id,
        admin_id = %admin_id,
        admin_email = %admin_email,
        revoked_count = revoked_count,
        "User sessions revoked by support"
    );

    Ok(Json(RevokeSessionsResponse {
        message: format!("{} session(s) revoked", revoked_count),
        revoked_count,
    }))
}

/// Get user activity log.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/support/users/{id}/activity",
    params(
        ("id" = Uuid, Path, description = "User ID"),
        ActivityQuery
    ),
    responses(
        (status = 200, description = "Activity log retrieved", body = Vec<SupportActivityLog>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Support"
)]
pub async fn get_user_activity(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
    Query(query): Query<ActivityQuery>,
) -> Result<Json<Vec<SupportActivityLog>>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let limit = query.limit.clamp(1, 500);

    let activity = state
        .platform_admin_repo
        .get_user_activity_log(id, limit)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, user_id = %id, "Failed to get activity log");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get activity log",
                )),
            )
        })?;

    Ok(Json(activity))
}

// ==================== Support Data Aggregation Handler (Story 10B.5) ====================

/// Response body for `GET /api/v1/platform-admin/support-data`.
#[derive(Debug, Serialize, ToSchema)]
pub struct SupportDataResponse {
    /// Platform-wide tenant diagnostics.
    pub data: SupportData,
}

// Silence unused-import warning — FaultStatusCount is re-exported for
// utoipa schema generation even though the route handler doesn't name it
// directly in its return type.
#[allow(unused_imports)]
use db::repositories::FaultStatusCount as _;

/// Get platform tenant diagnostics (support data).
///
/// Returns aggregated user counts, active session count, and fault status
/// breakdown across the whole platform.  Intended for the admin-web Support
/// Data page and platform support engineers.
///
/// Requires `AuditRead` capability (SuperAdmin role is additionally enforced
/// by `extract_super_admin_token` inside the handler body).
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/support-data",
    responses(
        (status = 200, description = "Support data retrieved", body = SupportDataResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Support"
)]
pub async fn get_support_data(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<SupportDataResponse>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let data = state
        .platform_admin_repo
        .get_support_data()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get support data");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to retrieve support data",
                )),
            )
        })?;

    Ok(Json(SupportDataResponse { data }))
}

// ==================== Onboarding Config Handler (Story 10B.6) ====================

/// Response for the onboarding config endpoint.
///
/// Returns all onboarding tour step definitions so that the platform admin
/// can review what tours and steps are configured across the platform.
#[derive(Debug, Serialize, ToSchema)]
pub struct OnboardingConfigResponse {
    /// All tour definitions (active and inactive).
    pub tours: Vec<OnboardingTour>,
    /// Total number of tours.
    pub total: usize,
}

/// Get onboarding tour configuration (platform admin view).
///
/// Returns all onboarding tour definitions including step definitions,
/// target roles, and active status. Useful for platform admins to audit
/// the onboarding experience and ensure tours are correctly configured.
///
/// User progress is tracked in the `user_onboarding_progress` table
/// which is persisted per-user per-tour via the onboarding repository.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/onboarding-config",
    responses(
        (status = 200, description = "Onboarding configuration retrieved", body = OnboardingConfigResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Onboarding"
)]
pub async fn get_onboarding_config(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<OnboardingConfigResponse>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let tours = state.onboarding_repo.list_all_tours().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to list onboarding tours");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to retrieve onboarding configuration",
            )),
        )
    })?;

    let total = tours.len();

    tracing::info!(
        total_tours = total,
        "Platform admin retrieved onboarding config"
    );

    Ok(Json(OnboardingConfigResponse { tours, total }))
}
