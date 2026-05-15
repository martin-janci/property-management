//! Per-tenant admin endpoints (Phase 3: Hosting & Theming).
//!
//! Two routers:
//!
//! * `branding_router()` → `GET /admin/tenants/{org_id}/branding`
//!                         `PUT /admin/tenants/{org_id}/branding`
//! * `feature_flags_router()` → `GET /admin/tenants/{org_id}/feature-flags`
//!                              `PUT /admin/tenants/{org_id}/feature-flags`
//!
//! # Auth (Phase 5 — N5)
//!
//! Each route declares the capability it needs via the `RequireCapability`
//! extractor + `require_capability(...)` tower layer:
//!
//!   * `GET  /branding`           → `Capability::SiteSettingsRead`
//!   * `PUT  /branding`           → `Capability::SiteSettingsWrite`
//!   * `GET  /feature-flags`      → `Capability::SiteSettingsRead`
//!   * `PUT  /feature-flags`      → `Capability::FeatureFlagsWrite`
//!
//! The previous `require_platform_principal` stub (a thin wrapper around
//! `extract_super_admin_token`) has been replaced. Admin actor identity for
//! RLS context comes from `RequestPrincipal::user_id` — the same trusted
//! source `RequireCapability` uses internally for capability lookup.
//!
//! # RLS
//!
//! All endpoints write/read under a SUPER-ADMIN RLS context (set on a
//! transaction-scoped connection, cleared before commit, same discipline
//! as `agency_provisioning`).

use admin_core::{require_capability, Capability, RequireCapability};
use api_core::extractors::principal::RequestPrincipal;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{
    agency_branding::{AgencyBranding, UpdateOrganizationBranding},
    tenant_feature_flag::{TenantFeatureFlag, UpsertTenantFeatureFlag},
};
use db::repositories::{AgencyBrandingRepository, TenantFeatureFlagRepository};
use serde::Serialize;
use uuid::Uuid;

use crate::state::AppState;

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
///
/// The two methods carry DIFFERENT capability requirements
/// (`SiteSettingsRead` for GET vs `SiteSettingsWrite` for PUT). Because
/// `RequireCapability`'s tower layer attaches a marker that applies to the
/// whole `MethodRouter`, each method lives in its own one-route sub-router
/// and we `.merge(...)` them to share the same `/` path.
pub fn branding_router() -> Router<AppState> {
    let read = Router::new().route(
        "/",
        get(get_tenant_branding).layer(require_capability(Capability::SiteSettingsRead)),
    );
    let write = Router::new().route(
        "/",
        put(update_tenant_branding).layer(require_capability(Capability::SiteSettingsWrite)),
    );
    read.merge(write)
}

#[derive(Debug, Serialize)]
pub struct BrandingResponse {
    pub branding: AgencyBranding,
}

#[derive(Debug, Serialize)]
pub struct BrandingReadResponse {
    pub branding: Option<AgencyBranding>,
}

/// `GET /admin/tenants/{org_id}/branding` — read the agency's branding row.
pub async fn get_tenant_branding(
    _cap: RequireCapability,
    principal: RequestPrincipal,
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
) -> Result<Json<BrandingReadResponse>, (StatusCode, Json<ErrorResponse>)> {
    let _ = principal; // identity already trusted via `RequireCapability`.
    let repo = AgencyBrandingRepository::new(state.db.clone());
    let branding = repo
        .fetch_by_organization_system(org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, organization_id = %org_id, "fetch agency_branding failed");
            db_error("Failed to read branding")
        })?;
    Ok(Json(BrandingReadResponse { branding }))
}

/// `PUT /admin/tenants/{org_id}/branding` — upsert the agency's branding row.
pub async fn update_tenant_branding(
    _cap: RequireCapability,
    principal: RequestPrincipal,
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    Json(payload): Json<UpdateOrganizationBranding>,
) -> Result<Json<BrandingResponse>, (StatusCode, Json<ErrorResponse>)> {
    let admin_id = principal.user_id;

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
///
/// Per-method capabilities differ (`SiteSettingsRead` for GET, `FeatureFlagsWrite`
/// for PUT). See the comment on [`branding_router`] for why each method lives
/// in its own one-route sub-router that we merge together.
pub fn feature_flags_router() -> Router<AppState> {
    let read = Router::new().route(
        "/",
        get(list_tenant_feature_flags)
            .layer(require_capability(Capability::SiteSettingsRead)),
    );
    let write = Router::new().route(
        "/",
        put(upsert_tenant_feature_flag)
            .layer(require_capability(Capability::FeatureFlagsWrite)),
    );
    read.merge(write)
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
    _cap: RequireCapability,
    principal: RequestPrincipal,
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
) -> Result<Json<ListFeatureFlagsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let admin_id = principal.user_id;

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
    _cap: RequireCapability,
    principal: RequestPrincipal,
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    Json(payload): Json<UpsertTenantFeatureFlag>,
) -> Result<Json<UpsertFeatureFlagResponse>, (StatusCode, Json<ErrorResponse>)> {
    let admin_id = principal.user_id;

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
