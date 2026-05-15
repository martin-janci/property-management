//! Per-host tenant config endpoint (Phase 3: Hosting & Theming).
//!
//! `GET /tenant-config` is the single endpoint reality-web hits server-side
//! at request time to learn what the current host is *for*: which tenant,
//! how to brand it, which feature flags to apply.
//!
//! # Resolution flow
//!
//! 1. Phase 1's `host_tenant_middleware` resolved the inbound `Host` to a
//!    `ResolvedTenant` (or returned 404). We pull it out of request
//!    extensions via the existing extractor.
//! 2. If unresolved (caller is on the platform host) → return platform
//!    defaults: `tenant_id: null`, brand = "Reality Portal", default
//!    `--ppt-*` tokens, no feature flags.
//! 3. If resolved → fetch `agency_branding` + `tenant_feature_flags` rows
//!    on a SYSTEM connection (we have no JWT — this endpoint is reached
//!    by every browser hit, including bots and unauthenticated users).
//!    Both `*_system` repository methods follow the same set/clear-context
//!    discipline as the Phase 1 host-resolution lookup.
//!
//! # Caching
//!
//! Reality-web caches the response per-host (`Vary: Host`) for 60s. The
//! browser-side TTL is short on purpose — branding flips and kill-switch
//! flips need to propagate within a minute, even at scale.
//!
//! # Why this lives in api-server, not reality-server
//!
//! The platform-host case (no resolved tenant) returns defaults — that
//! requires a SYSTEM read. reality-server's RLS connection extractor is
//! always org-scoped (HostRlsConnection), so the host-less case has no
//! sanctioned read path there. api-server owns this endpoint and exposes
//! it without auth so reality-web can call it directly from its SSR layer.

use axum::{extract::State, http::HeaderMap, response::IntoResponse, routing::get, Json, Router};
use db::repositories::{AgencyBrandingRepository, TenantFeatureFlagRepository};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::state::AppState;
use api_core::middleware::host_tenant::ResolvedTenant;

/// Mount `/tenant-config`. Mounted at the root in `main.rs` (no `/api/v1`
/// prefix) so reality-web can hit it on the same hostname Caddy serves.
pub fn router() -> Router<AppState> {
    Router::new().route("/", get(get_tenant_config))
}

/// Public re-export so integration tests can build a Router with this route
/// alone.
#[allow(unused_imports)]
pub use get_tenant_config as handler;

// ============================================================================
// Response shape
// ============================================================================

#[derive(Debug, Serialize)]
pub struct TenantConfigResponse {
    /// `None` for the platform-host case (the global Reality Portal).
    pub tenant_id: Option<Uuid>,
    pub name: String,
    pub branding: BrandingResponse,
    /// Map of `flag_key` -> `{ enabled, value }`. Stable JSON-object key
    /// ordering via `BTreeMap` so the response hashes deterministically
    /// (Reality-web emits a `x-tenant-branding-hash` header from this).
    pub feature_flags: BTreeMap<String, FeatureFlagState>,
    pub locales: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct BrandingResponse {
    pub logo_url: Option<String>,
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub accent_color: Option<String>,
    pub font_family: Option<String>,
    pub css_vars: Value,
}

#[derive(Debug, Serialize)]
pub struct FeatureFlagState {
    pub enabled: bool,
    pub value: Option<Value>,
}

const SUPPORTED_LOCALES: &[&str] = &["en", "sk", "cs", "de", "pl", "hu"];

fn platform_default_branding() -> BrandingResponse {
    BrandingResponse {
        logo_url: None,
        primary_color: None,
        secondary_color: None,
        accent_color: None,
        font_family: None,
        css_vars: Value::Object(Default::default()),
    }
}

fn platform_default_response() -> TenantConfigResponse {
    TenantConfigResponse {
        tenant_id: None,
        name: "Reality Portal".to_string(),
        branding: platform_default_branding(),
        feature_flags: BTreeMap::new(),
        locales: SUPPORTED_LOCALES.to_vec(),
    }
}

// ============================================================================
// Handler
// ============================================================================

/// `GET /tenant-config`
///
/// Reads `ResolvedTenant` from request extensions (set by Phase 1's
/// `host_tenant_middleware`). When absent — allowlisted paths bypass the
/// middleware entirely, but `/tenant-config` is NOT allowlisted, so an
/// absent extension here means the host did not resolve and the middleware
/// should already have returned 404 before we run. The `Option<Extension>`
/// signature is purely defensive: if the middleware is somehow not wired
/// (or an integration test mounts the route without it), we fall through
/// to the platform-default response rather than 500.
///
/// # Cache headers
///
/// - `Vary: Host` — every host gets its own cache entry.
/// - `Cache-Control: public, max-age=60, s-maxage=60` — short TTL so
///   branding/kill-switch flips propagate quickly.
pub async fn get_tenant_config(
    State(state): State<AppState>,
    resolved: Option<axum::Extension<ResolvedTenant>>,
    _headers: HeaderMap,
) -> impl IntoResponse {
    tenant_config_inner(state, resolved.map(|axum::Extension(rt)| rt)).await
}

/// Pure handler logic: given an optional resolved tenant, build the response.
/// Extracted so tests can call it directly without spinning up a router.
pub async fn tenant_config_inner(
    state: AppState,
    resolved: Option<ResolvedTenant>,
) -> axum::response::Response {
    let body = match resolved {
        None => platform_default_response(),
        Some(rt) => build_tenant_response(&state, rt.organization_id).await,
    };

    let mut response = Json(body).into_response();
    let h = response.headers_mut();
    h.insert(
        axum::http::header::VARY,
        axum::http::HeaderValue::from_static("Host"),
    );
    h.insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("public, max-age=60, s-maxage=60"),
    );
    response
}

async fn build_tenant_response(state: &AppState, org_id: Uuid) -> TenantConfigResponse {
    let branding_repo = AgencyBrandingRepository::new(state.db.clone());
    let flags_repo = TenantFeatureFlagRepository::new(state.db.clone());

    let branding = match branding_repo.fetch_by_organization_system(org_id).await {
        Ok(opt) => opt,
        Err(e) => {
            tracing::error!(error = %e, organization_id = %org_id, "tenant-config: branding fetch failed");
            None
        }
    };

    let flags = match flags_repo.list_by_organization_system(org_id).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, organization_id = %org_id, "tenant-config: flags fetch failed");
            Vec::new()
        }
    };

    // Org name — best-effort lookup. The tenant resolution itself proves
    // the org exists; if the name lookup fails we fall back to a generic
    // label rather than 5xx.
    let name = fetch_organization_name(state.db.clone(), org_id)
        .await
        .unwrap_or_else(|| "Tenant".to_string());

    let branding_resp = match branding {
        Some(b) => BrandingResponse {
            logo_url: b.logo_url,
            primary_color: b.primary_color,
            secondary_color: b.secondary_color,
            accent_color: None,
            font_family: b.font_family,
            css_vars: b.css_vars,
        },
        None => platform_default_branding(),
    };

    let mut flag_map = BTreeMap::new();
    for f in flags {
        flag_map.insert(
            f.flag_key,
            FeatureFlagState {
                enabled: f.enabled,
                value: f.value,
            },
        );
    }

    TenantConfigResponse {
        tenant_id: Some(org_id),
        name,
        branding: branding_resp,
        feature_flags: flag_map,
        locales: SUPPORTED_LOCALES.to_vec(),
    }
}

async fn fetch_organization_name(pool: db::DbPool, org_id: Uuid) -> Option<String> {
    let mut conn = pool.acquire().await.ok()?;
    if let Err(e) = db::tenant_context::set_request_context(&mut *conn, None, None, true).await {
        tracing::warn!(error = %e, "tenant-config: failed to set system context for org name lookup");
        return None;
    }
    let result = sqlx::query_scalar::<_, String>(
        "SELECT name FROM organizations WHERE id = $1 AND status != 'deleted'",
    )
    .bind(org_id)
    .fetch_optional(&mut *conn)
    .await
    .ok()
    .flatten();
    let _ = db::tenant_context::clear_request_context(&mut *conn).await;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_default_response_has_null_tenant() {
        let r = platform_default_response();
        assert!(r.tenant_id.is_none());
        assert_eq!(r.name, "Reality Portal");
        assert!(r.feature_flags.is_empty());
        assert!(r.locales.contains(&"sk"));
    }
}
