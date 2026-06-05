//! Platform-admin tenant (organization) management endpoints.
//!
//! Story 10B.1 — list / get / suspend / reactivate organizations plus the
//! platform-wide statistics endpoint. Behaviour is identical to the original
//! `platform_admin.rs`; this is a pure structural move.

use admin_core::RequireCapability;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use common::errors::ErrorResponse;
use db::models::{
    AdminOrganizationDetail, AdminOrganizationSummary, ReactivateOrganizationRequest,
    SuspendOrganizationRequest,
};
use db::repositories::PlatformStats;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::extract_super_admin_token;
use crate::state::AppState;

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

pub(super) fn default_page() -> u32 {
    1
}

pub(super) fn default_page_size() -> u32 {
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
