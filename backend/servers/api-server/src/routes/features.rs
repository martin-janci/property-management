//! Feature routes (Epic 109, Stories 109.1-109.4).
//!
//! Routes for feature resolution, preferences, and usage analytics.

use api_core::middleware::host_tenant::ResolvedTenant;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::feature_analytics::{
    FeatureEventType, FeatureStatsQuery, FeatureUsageStats, LogFeatureEvent, ResolvedFeature,
    ResolvedFeaturesQuery, SetFeaturePreference, UpgradeOptionsResponse,
};
use db::repositories::FeatureAnalyticsRepository;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::services::FeatureService;
use crate::state::AppState;

/// Create features router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Story 109.1 & 109.2: Feature resolution
        .route("/resolved", get(get_resolved_features))
        .route("/{key}/check", get(check_feature))
        // Story 109.3: Upgrade options
        .route("/{key}/upgrade-options", get(get_upgrade_options))
        // Story 109.1: User preferences
        .route("/{key}/preference", post(set_feature_preference))
        // Story 109.4: Analytics
        .route("/analytics/event", post(log_feature_event))
        .route("/analytics/{feature_id}/stats", get(get_feature_stats))
}

// ==================== Types ====================

/// Response for resolved features endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct ResolvedFeaturesResponse {
    pub features: Vec<ResolvedFeature>,
}

/// Response for feature check.
#[derive(Debug, Serialize, ToSchema)]
pub struct FeatureCheckResponse {
    pub key: String,
    pub is_enabled: bool,
}

/// Response for logging an event.
#[derive(Debug, Serialize, ToSchema)]
pub struct LogEventResponse {
    pub success: bool,
}

/// Response for feature stats.
#[derive(Debug, Serialize, ToSchema)]
pub struct FeatureStatsResponse {
    pub stats: FeatureUsageStats,
}

/// Response for preference update.
#[derive(Debug, Serialize, ToSchema)]
pub struct PreferenceResponse {
    pub success: bool,
    pub is_enabled: bool,
}

// ==================== Helper Functions ====================

/// Extract user context from headers.
///
/// The user comes from the validated access token. The organization comes
/// from the `X-Tenant-ID` header (what the web/mobile clients send — login
/// tokens carry no org claim) or, for tokens minted for one org, the `org_id`
/// claim; if both are present they must agree. When `host_tenant_middleware`
/// pinned the request to an organization (`ResolvedTenant`), that org wins
/// and a header or claim naming a different org is a 403 — the same rule
/// `ValidatedTenantExtractor` applies. SECURITY: the caller must be an active
/// member of that organization (checked in the database), and the role is the
/// membership `role_type`, never a token claim.
async fn extract_user_context(
    headers: &axum::http::HeaderMap,
    resolved: Option<axum::Extension<ResolvedTenant>>,
    state: &AppState,
) -> Result<UserContext, (StatusCode, Json<ErrorResponse>)> {
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
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        ));
    }

    let token = &auth_header[7..];

    let claims = state
        .jwt_service
        .validate_access_token(token)
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "INVALID_TOKEN",
                    "Token verification failed",
                )),
            )
        })?;

    // Parse user ID from claims.sub
    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "INVALID_TOKEN",
                "Invalid user ID in token",
            )),
        )
    })?;

    let header_org: Option<Uuid> = headers
        .get("X-Tenant-ID")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok());
    let claim_org: Option<Uuid> = claims.org_id.as_ref().and_then(|id| id.parse().ok());
    if let (Some(h), Some(c)) = (header_org, claim_org) {
        if h != c {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new("TENANT_MISMATCH", "Tenant mismatch")),
            ));
        }
    }
    // The platform host carries a nil placeholder id, not an organization.
    let host_org = resolved
        .map(|axum::Extension(r)| r)
        .filter(|r| !r.is_platform_host())
        .map(|r| r.organization_id);
    if let Some(host) = host_org {
        if header_org.is_some_and(|h| h != host) || claim_org.is_some_and(|c| c != host) {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new("TENANT_MISMATCH", "Tenant mismatch")),
            ));
        }
    }
    let org_id = host_org.or(header_org).or(claim_org).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "MISSING_ORG",
                "Organization context required",
            )),
        )
    })?;

    // Active membership + role from the database (get_user_role_type only
    // returns active memberships).
    let role_type = state
        .org_member_repo
        .get_user_role_type(org_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to verify organization membership");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "NOT_MEMBER",
                    "You are not a member of this organization",
                )),
            )
        })?;
    let role = common::TenantRole::from_role_type(&role_type);

    // User type and role_id would typically come from database lookup
    // For now, default to "user" type and no specific role
    Ok(UserContext {
        user_id,
        org_id,
        user_type: "user".to_string(),
        role_id: None,
        is_manager: role.is_some_and(|r| r.is_manager()),
    })
}

/// User context extracted from JWT.
struct UserContext {
    user_id: Uuid,
    org_id: Uuid,
    user_type: String,
    role_id: Option<Uuid>,
    /// Manager-class role in `org_id` (from the membership row).
    is_manager: bool,
}

// ==================== Route Handlers ====================

/// Get resolved features for the current user.
///
/// Returns all features with their resolved enabled state based on:
/// - User type access matrix
/// - Organization packages
/// - User preferences
/// - Feature flag overrides
#[utoipa::path(
    get,
    path = "/api/v1/features/resolved",
    tag = "Features",
    params(
        ("category" = Option<String>, Query, description = "Filter by category"),
        ("enabled_only" = Option<bool>, Query, description = "Only return enabled features")
    ),
    responses(
        (status = 200, description = "Resolved features", body = ResolvedFeaturesResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
async fn get_resolved_features(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    resolved: Option<axum::Extension<ResolvedTenant>>,
    Query(query): Query<ResolvedFeaturesQuery>,
) -> Result<Json<ResolvedFeaturesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = extract_user_context(&headers, resolved, &state).await?;

    let service = FeatureService::new(state.db.clone());

    let features = service
        .resolve_features_for_user(
            ctx.user_id,
            ctx.org_id,
            &ctx.user_type,
            ctx.role_id,
            query.category.as_deref(),
            query.enabled_only.unwrap_or(false),
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to resolve features: {}", e),
                )),
            )
        })?;

    Ok(Json(ResolvedFeaturesResponse { features }))
}

/// Check if a specific feature is enabled.
#[utoipa::path(
    get,
    path = "/api/v1/features/{key}/check",
    tag = "Features",
    params(
        ("key" = String, Path, description = "Feature key")
    ),
    responses(
        (status = 200, description = "Feature check result", body = FeatureCheckResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
async fn check_feature(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    resolved: Option<axum::Extension<ResolvedTenant>>,
    Path(key): Path<String>,
) -> Result<Json<FeatureCheckResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = extract_user_context(&headers, resolved, &state).await?;

    let service = FeatureService::new(state.db.clone());

    let is_enabled = service
        .is_feature_enabled(&key, ctx.user_id, ctx.org_id, &ctx.user_type, ctx.role_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to check feature: {}", e),
                )),
            )
        })?;

    Ok(Json(FeatureCheckResponse { key, is_enabled }))
}

/// Get upgrade options for a feature.
///
/// Returns packages that contain the specified feature, sorted by price.
#[utoipa::path(
    get,
    path = "/api/v1/features/{key}/upgrade-options",
    tag = "Features",
    params(
        ("key" = String, Path, description = "Feature key")
    ),
    responses(
        (status = 200, description = "Upgrade options", body = UpgradeOptionsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
async fn get_upgrade_options(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    resolved: Option<axum::Extension<ResolvedTenant>>,
    Path(key): Path<String>,
) -> Result<Json<UpgradeOptionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify authentication
    let _ = extract_user_context(&headers, resolved, &state).await?;

    let service = FeatureService::new(state.db.clone());

    let packages = service.get_upgrade_options(&key).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                format!("Failed to get upgrade options: {}", e),
            )),
        )
    })?;

    Ok(Json(UpgradeOptionsResponse {
        feature_key: key,
        packages,
    }))
}

/// Set user preference for an optional feature.
#[utoipa::path(
    post,
    path = "/api/v1/features/{key}/preference",
    tag = "Features",
    params(
        ("key" = String, Path, description = "Feature key")
    ),
    request_body = SetFeaturePreference,
    responses(
        (status = 200, description = "Preference updated", body = PreferenceResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 400, description = "Feature not found or not toggleable", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
async fn set_feature_preference(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    resolved: Option<axum::Extension<ResolvedTenant>>,
    Path(key): Path<String>,
    Json(body): Json<SetFeaturePreference>,
) -> Result<Json<PreferenceResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = extract_user_context(&headers, resolved, &state).await?;

    let service = FeatureService::new(state.db.clone());

    // First check if the feature is toggleable for this user
    let analytics_repo = FeatureAnalyticsRepository::new(state.db.clone());

    let flag = state
        .feature_flag_repo
        .get_by_key(&key)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("FEATURE_NOT_FOUND", "Feature not found")),
            )
        })?;

    // Check if optional for user type
    let access = analytics_repo
        .get_user_type_access(flag.id, &ctx.user_type)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    let is_optional = access
        .map(|a| a.access_state == db::models::feature_analytics::FeatureAccessState::Optional)
        .unwrap_or(false);

    if !is_optional {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "NOT_TOGGLEABLE",
                "This feature cannot be toggled by the user",
            )),
        ));
    }

    let success = service
        .toggle_user_feature(ctx.user_id, &key, body.is_enabled)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to update preference: {}", e),
                )),
            )
        })?;

    // Log the toggle event
    let event_type = if body.is_enabled {
        FeatureEventType::ToggledOn
    } else {
        FeatureEventType::ToggledOff
    };

    let _ = service
        .log_feature_event(
            &key,
            Some(ctx.user_id),
            Some(ctx.org_id),
            event_type,
            Some(&ctx.user_type),
            serde_json::json!({}),
        )
        .await;

    Ok(Json(PreferenceResponse {
        success,
        is_enabled: body.is_enabled,
    }))
}

/// Log a feature usage event.
#[utoipa::path(
    post,
    path = "/api/v1/features/analytics/event",
    tag = "Features",
    request_body = LogFeatureEvent,
    responses(
        (status = 200, description = "Event logged", body = LogEventResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
async fn log_feature_event(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    resolved: Option<axum::Extension<ResolvedTenant>>,
    Json(body): Json<LogFeatureEvent>,
) -> Result<Json<LogEventResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = extract_user_context(&headers, resolved, &state).await?;

    let service = FeatureService::new(state.db.clone());

    service
        .log_feature_event(
            &body.feature_key,
            Some(ctx.user_id),
            Some(ctx.org_id),
            body.event_type,
            Some(&ctx.user_type),
            body.metadata,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to log event: {}", e),
                )),
            )
        })?;

    Ok(Json(LogEventResponse { success: true }))
}

/// Get feature usage statistics.
#[utoipa::path(
    get,
    path = "/api/v1/features/analytics/{feature_id}/stats",
    tag = "Features",
    params(
        ("feature_id" = Uuid, Path, description = "Feature flag ID"),
        ("start_date" = Option<String>, Query, description = "Start date (ISO 8601)"),
        ("end_date" = Option<String>, Query, description = "End date (ISO 8601)")
    ),
    responses(
        (status = 200, description = "Feature statistics", body = FeatureStatsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - requires admin", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
async fn get_feature_stats(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    resolved: Option<axum::Extension<ResolvedTenant>>,
    Path(feature_id): Path<Uuid>,
    Query(query): Query<FeatureStatsQuery>,
) -> Result<Json<FeatureStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Usage analytics are a manager view, and only of the caller's own org:
    // the shared feature_usage_events table holds every tenant's events.
    let ctx = extract_user_context(&headers, resolved, &state).await?;
    if !ctx.is_manager {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Manager-level access required",
            )),
        ));
    }

    let analytics_repo = FeatureAnalyticsRepository::new(state.db.clone());

    let stats = analytics_repo
        .get_feature_stats_for_org(feature_id, ctx.org_id, query.start_date, query.end_date)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to get stats: {}", e),
                )),
            )
        })?;

    Ok(Json(FeatureStatsResponse { stats }))
}
