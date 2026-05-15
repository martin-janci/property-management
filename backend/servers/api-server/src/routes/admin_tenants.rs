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

/// Server-side branding sanitizer (N7).
///
/// Mirrors the reality-web client-side `sanitizeCssValue` so a malicious
/// `css_vars` payload cannot reach the DB and infect every render until
/// someone notices. Rejects on first offense rather than silently dropping
/// — admins should see a clear "no" instead of "saved but ignored".
///
/// Rules (kept identical to the frontend so behaviour is symmetric):
///
/// * `css_vars` MUST be a JSON object whose values are strings.
/// * Keys MUST match `^--[a-z0-9-]+$` and be ≤ 64 chars.
/// * Values MUST NOT contain `<`, `>`, `"`, newlines, `{`, `}`, `;`.
/// * Values MUST NOT contain (case-insensitive, whitespace-collapsed)
///   `url(`, `expression(`, `javascript:`, or `data:`.
/// * Values MUST be ≤ 200 chars.
/// * Color-shaped vars (name matches `color|background|border`, case
///   insensitive) MUST be a recognized color literal: `#hex`, `rgb()/rgba()`,
///   `hsl()/hsla()`, or a bare keyword.
fn validate_css_vars(value: &serde_json::Value) -> Result<(), &'static str> {
    let obj = value.as_object().ok_or("css_vars must be a JSON object")?;
    for (k, v) in obj {
        if !is_safe_css_var_name(k) {
            return Err("css_vars contains an invalid property name");
        }
        let s = v.as_str().ok_or("css_vars values must be strings")?;
        validate_css_var_value(k, s)?;
    }
    Ok(())
}

fn is_safe_css_var_name(name: &str) -> bool {
    if name.len() > 64 {
        return false;
    }
    if !name.starts_with("--") {
        return false;
    }
    name[2..]
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && name.len() > 2
}

fn validate_css_var_value(var_name: &str, value: &str) -> Result<(), &'static str> {
    if value.len() > 200 {
        return Err("css_vars value exceeds 200 chars");
    }
    if value
        .chars()
        .any(|c| matches!(c, '<' | '>' | '"' | '\n' | '\r' | '{' | '}' | ';'))
    {
        return Err("css_vars value contains forbidden characters");
    }
    let normalized: String = value.chars().filter(|c| !c.is_whitespace()).collect();
    let lower = normalized.to_ascii_lowercase();
    if lower.contains("url(")
        || lower.contains("expression(")
        || lower.contains("javascript:")
        || lower.contains("data:")
    {
        return Err("css_vars value contains a forbidden token");
    }
    let lower_name = var_name.to_ascii_lowercase();
    if lower_name.contains("color")
        || lower_name.contains("background")
        || lower_name.contains("border")
    {
        let trimmed = value.trim();
        let is_hex = trimmed.starts_with('#')
            && trimmed.len() >= 4
            && trimmed.len() <= 9
            && trimmed[1..].chars().all(|c| c.is_ascii_hexdigit());
        let is_func = matches!(
            trimmed
                .split('(')
                .next()
                .map(|p| p.to_ascii_lowercase())
                .as_deref(),
            Some("rgb") | Some("rgba") | Some("hsl") | Some("hsla")
        ) && trimmed.ends_with(')');
        let is_keyword = !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_alphabetic());
        if !(is_hex || is_func || is_keyword) {
            return Err("css_vars value for a color-shaped property must be a CSS color literal");
        }
    }
    Ok(())
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

    // N7: defense-in-depth — validate the css_vars blob server-side before
    // it ever reaches the DB. The frontend (reality-web) re-sanitizes on
    // render, but treating the row as trusted-once-stored only works if
    // every reader runs the same sanitizer; cheaper to refuse the write.
    if let Some(vars) = payload.css_vars.as_ref() {
        if let Err(reason) = validate_css_vars(vars) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("INVALID_BRANDING", reason)),
            ));
        }
    }

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
        get(list_tenant_feature_flags).layer(require_capability(Capability::SiteSettingsRead)),
    );
    let write = Router::new().route(
        "/",
        put(upsert_tenant_feature_flag).layer(require_capability(Capability::FeatureFlagsWrite)),
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

#[cfg(test)]
mod sanitizer_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_url_token() {
        let v = json!({"--ppt-bg-image": "url(//evil.com/x.png)"});
        assert!(validate_css_vars(&v).is_err());
    }

    #[test]
    fn rejects_data_uri() {
        let v = json!({"--ppt-bg-image": "data:image/png;base64,AAAA"});
        assert!(validate_css_vars(&v).is_err());
    }

    #[test]
    fn rejects_javascript_scheme() {
        let v = json!({"--ppt-color-primary": "javascript:alert(1)"});
        assert!(validate_css_vars(&v).is_err());
    }

    #[test]
    fn rejects_expression() {
        let v = json!({"--ppt-radius-md": "expression(alert(1))"});
        assert!(validate_css_vars(&v).is_err());
    }

    #[test]
    fn rejects_breakout_chars() {
        let v = json!({"--ppt-color-primary": "red; background:url(x)"});
        assert!(validate_css_vars(&v).is_err());
    }

    #[test]
    fn rejects_uppercase_or_long_name() {
        assert!(validate_css_vars(&json!({"--PPT-X": "8px"})).is_err());
        assert!(validate_css_vars(&json!({"color": "8px"})).is_err());
        let long = format!("--{}", "a".repeat(70));
        assert!(validate_css_vars(&json!({long: "8px"})).is_err());
    }

    #[test]
    fn rejects_non_color_in_color_var() {
        let v = json!({"--ppt-color-primary": "var(--evil)"});
        assert!(validate_css_vars(&v).is_err());
    }

    #[test]
    fn accepts_hex_and_rgb_colors() {
        assert!(validate_css_vars(&json!({"--ppt-color-primary": "#abc"})).is_ok());
        assert!(validate_css_vars(&json!({"--ppt-color-primary": "#aabbcc"})).is_ok());
        assert!(validate_css_vars(&json!({"--ppt-color-primary": "rgba(1,2,3,0.4)"})).is_ok());
        assert!(validate_css_vars(&json!({"--ppt-color-primary": "transparent"})).is_ok());
    }

    #[test]
    fn accepts_non_color_safe_value() {
        assert!(validate_css_vars(&json!({"--ppt-radius-md": "8px"})).is_ok());
    }

    #[test]
    fn rejects_non_string_value() {
        assert!(validate_css_vars(&json!({"--ppt-radius-md": 8})).is_err());
        assert!(validate_css_vars(&json!({"--ppt-radius-md": null})).is_err());
    }

    #[test]
    fn rejects_non_object_root() {
        assert!(validate_css_vars(&json!(["--ppt-radius-md", "8px"])).is_err());
    }
}
