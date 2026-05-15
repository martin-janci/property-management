//! Per-tenant admin endpoints (Phase 3: Hosting & Theming).
//!
//! Two routers:
//!
//! * `branding_router()` → `PUT /admin/tenants/{org_id}/branding`
//! * `feature_flags_router()` → `GET /admin/tenants/{org_id}/feature-flags`
//!                              `PUT /admin/tenants/{org_id}/feature-flags`
//!
//! # Auth (stub)
//!
//! Phase 5 will land a real capability registry (`agencies:write`,
//! `feature_flags:write`, etc). Until then we re-use the existing platform
//! admin gate — `require_platform_principal()` is a thin wrapper around
//! `extract_super_admin_token` from `platform_admin`. When Phase 5 ships,
//! swap the body of `require_platform_principal` for the real check; call
//! sites won't change.
//!
//! # RLS
//!
//! Both endpoints write under a SUPER-ADMIN RLS context (set on a
//! transaction-scoped connection, cleared before commit, same discipline
//! as `agency_provisioning`).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{
    agency_branding::UpdateOrganizationBranding,
    tenant_feature_flag::{TenantFeatureFlag, UpsertTenantFeatureFlag},
    AgencyBranding,
};
use db::repositories::{AgencyBrandingRepository, TenantFeatureFlagRepository};
use serde::Serialize;
use uuid::Uuid;

use crate::routes::platform_admin::extract_super_admin_token;
use crate::state::AppState;

// ============================================================================
// Auth stub
// ============================================================================

/// Phase 3 capability gate — stubbed onto the existing platform-admin token
/// check. Returns `(admin_user_id, admin_email)` on success.
///
/// Phase 5 will replace the body with a real capability lookup
/// (`require_capability(headers, state, "agencies:write")`) without touching
/// call sites.
fn require_platform_principal(
    headers: &axum::http::HeaderMap,
    state: &AppState,
) -> Result<(Uuid, String), (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(headers, state)
}

fn db_error(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new("DATABASE_ERROR", msg)),
    )
}

// ============================================================================
// Branding
// ============================================================================

/// Mount `/branding` as a sub-router under `/admin/tenants/{org_id}`.
pub fn branding_router() -> Router<AppState> {
    Router::new().route("/", put(update_tenant_branding))
}

#[derive(Debug, Serialize)]
pub struct BrandingResponse {
    pub branding: AgencyBranding,
}

/// `PUT /admin/tenants/{org_id}/branding` — upsert the agency's branding row.
pub async fn update_tenant_branding(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<UpdateOrganizationBranding>,
) -> Result<Json<BrandingResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (admin_id, _email) = require_platform_principal(&headers, &state)?;

    // Open a transaction with super-admin RLS context — same pattern as
    // agency_provisioning::create_agency.
    let mut tx = state.db.begin().await.map_err(|e| {
        tracing::error!(error = %e, "begin tx for tenant branding update failed");
        db_error("Failed to begin transaction")
    })?;

    db::tenant_context::set_request_context(&mut *tx, None, Some(admin_id), true)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "set super-admin RLS context failed");
            db_error("Failed to set security context")
        })?;

    let repo = AgencyBrandingRepository::new(state.db.clone());
    let branding = repo
        .upsert_rls(&mut *tx, org_id, payload)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, organization_id = %org_id, "upsert agency_branding failed");
            db_error("Failed to update branding")
        })?;

    if let Err(e) = db::tenant_context::clear_request_context(&mut *tx).await {
        tracing::warn!(error = %e, "Failed to clear RLS context before commit");
    }
    tx.commit().await.map_err(|e| {
        tracing::error!(error = %e, "commit tenant branding tx failed");
        db_error("Failed to commit transaction")
    })?;

    tracing::info!(admin_id = %admin_id, organization_id = %org_id, "tenant branding updated");
    Ok(Json(BrandingResponse { branding }))
}

// ============================================================================
// Feature flags
// ============================================================================

/// Mount `/feature-flags` as a sub-router under `/admin/tenants/{org_id}`.
pub fn feature_flags_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_tenant_feature_flags))
        .route("/", put(upsert_tenant_feature_flag))
}

#[derive(Debug, Serialize)]
pub struct ListFeatureFlagsResponse {
    pub flags: Vec<TenantFeatureFlag>,
}

#[derive(Debug, Serialize)]
pub struct UpsertFeatureFlagResponse {
    pub flag: TenantFeatureFlag,
}

pub async fn list_tenant_feature_flags(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ListFeatureFlagsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (admin_id, _email) = require_platform_principal(&headers, &state)?;

    let mut tx = state.db.begin().await.map_err(|e| {
        tracing::error!(error = %e, "begin tx for list_tenant_feature_flags failed");
        db_error("Failed to begin transaction")
    })?;
    db::tenant_context::set_request_context(&mut *tx, None, Some(admin_id), true)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "set super-admin RLS context failed");
            db_error("Failed to set security context")
        })?;

    let repo = TenantFeatureFlagRepository::new(state.db.clone());
    let flags = repo.list_by_organization_rls(&mut *tx, org_id).await.map_err(|e| {
        tracing::error!(error = %e, organization_id = %org_id, "list tenant_feature_flags failed");
        db_error("Failed to list feature flags")
    })?;

    if let Err(e) = db::tenant_context::clear_request_context(&mut *tx).await {
        tracing::warn!(error = %e, "Failed to clear RLS context");
    }
    tx.commit().await.map_err(|e| {
        tracing::error!(error = %e, "commit list flags tx failed");
        db_error("Failed to commit transaction")
    })?;

    Ok(Json(ListFeatureFlagsResponse { flags }))
}

pub async fn upsert_tenant_feature_flag(
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<UpsertTenantFeatureFlag>,
) -> Result<Json<UpsertFeatureFlagResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (admin_id, _email) = require_platform_principal(&headers, &state)?;

    if payload.flag_key.is_empty() || payload.flag_key.len() > 100 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_FLAG_KEY",
                "flag_key must be 1-100 chars",
            )),
        ));
    }

    let mut tx = state.db.begin().await.map_err(|e| {
        tracing::error!(error = %e, "begin tx for upsert_tenant_feature_flag failed");
        db_error("Failed to begin transaction")
    })?;
    db::tenant_context::set_request_context(&mut *tx, None, Some(admin_id), true)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "set super-admin RLS context failed");
            db_error("Failed to set security context")
        })?;

    let repo = TenantFeatureFlagRepository::new(state.db.clone());
    let flag = repo
        .upsert_rls(&mut *tx, org_id, Some(admin_id), payload)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, organization_id = %org_id, "upsert tenant_feature_flag failed");
            db_error("Failed to upsert feature flag")
        })?;

    if let Err(e) = db::tenant_context::clear_request_context(&mut *tx).await {
        tracing::warn!(error = %e, "Failed to clear RLS context");
    }
    tx.commit().await.map_err(|e| {
        tracing::error!(error = %e, "commit upsert flag tx failed");
        db_error("Failed to commit transaction")
    })?;

    tracing::info!(
        admin_id = %admin_id,
        organization_id = %org_id,
        flag_key = %flag.flag_key,
        enabled = flag.enabled,
        "tenant feature flag upserted"
    );
    Ok(Json(UpsertFeatureFlagResponse { flag }))
}
