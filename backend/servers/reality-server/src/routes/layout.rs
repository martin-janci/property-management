//! Public layout resolved endpoint — reality-server side.
//!
//! Serves `GET /api/v1/layout/resolved/{*screen}` with no auth and no tenant
//! layer. Only screens with a published config resolve; others 404.
//! Uses `acquire_public_conn()` to clear stale RLS context (required by
//! `scripts/check-rls-enforcement.sh`).

use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use db::repositories::LayoutRepository;
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Debug, Deserialize)]
pub struct ResolvedQuery {
    pub platform: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/resolved/{*screen}", get(get_resolved))
}

/// Namespace guard for the public, unauthenticated resolved endpoint: only
/// `reality/*` screens are servable here. Internal namespaces (e.g. `ppt/*`
/// management screens) must never leak through the public portal — they 404
/// before any DB access.
fn is_public_layout_screen(screen: &str) -> bool {
    screen.starts_with("reality/")
}

#[utoipa::path(get, path = "/api/v1/layout/resolved/{screen}", tag = "Layout",
    params(("screen" = String, Path, description = "Screen id, e.g. reality/listing-detail"),
           ("platform" = Option<String>, Query, description = "web|mobile, default web")),
    responses((status = 200, description = "Resolved section list (public, no tenant layer)"),
              (status = 404, description = "Screen not published or manifest missing")))]
pub async fn get_resolved(
    State(state): State<AppState>,
    Path(screen): Path<String>,
    Query(q): Query<ResolvedQuery>,
) -> Result<Json<layout_core::ResolvedScreen>, (StatusCode, String)> {
    // Public namespace restriction — before any DB access.
    if !is_public_layout_screen(&screen) {
        return Err((StatusCode::NOT_FOUND, "unknown screen".to_string()));
    }
    let platform = match q.platform.as_deref().unwrap_or("web") {
        "web" => layout_core::Platform::Web,
        "mobile" => layout_core::Platform::Mobile,
        other => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("unknown platform {other:?}"),
            ))
        }
    };
    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}")))?;
    let repo = LayoutRepository::new();

    let cfg = repo
        .get_config(&mut *conn, &screen)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}")))?
        .ok_or((StatusCode::NOT_FOUND, "unknown screen".to_string()))?;
    let published = cfg.published.ok_or((
        StatusCode::NOT_FOUND,
        "screen has no published config".to_string(),
    ))?;
    let base: layout_core::ScreenConfig = serde_json::from_value(published).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("stored config invalid: {e}"),
        )
    })?;

    let platform_key = match platform {
        layout_core::Platform::Web => "web",
        layout_core::Platform::Mobile => "mobile",
    };
    let manifest_row = repo
        .get_manifest(&mut *conn, platform_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}")))?
        .ok_or((
            StatusCode::NOT_FOUND,
            "no registry manifest for platform".to_string(),
        ))?;
    let manifest: layout_core::RegistryManifest = serde_json::from_value(manifest_row.manifest)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("stored manifest invalid: {e}"),
            )
        })?;

    let kills: BTreeSet<layout_core::SectionType> = repo
        .list_kills(&mut *conn, &screen)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}")))?
        .into_iter()
        .map(|k| layout_core::SectionType(k.section_type))
        .collect();

    // Public portal: no tenant layer (spec §3.2 — the tenant layer simply
    // doesn't contribute on public reality screens).
    let resolved = layout_core::resolve(&base, platform, None, &kills, &manifest);
    Ok(Json(resolved))
}

#[cfg(test)]
mod tests {
    use super::is_public_layout_screen;

    #[test]
    fn only_reality_namespace_is_public() {
        assert!(is_public_layout_screen("reality/listing-detail"));
        assert!(is_public_layout_screen("reality/home"));
        // internal namespaces must not be servable on the public portal
        assert!(!is_public_layout_screen("ppt/dashboard"));
        assert!(!is_public_layout_screen("admin/anything"));
        assert!(!is_public_layout_screen("reality")); // no trailing segment
        assert!(!is_public_layout_screen(""));
        assert!(!is_public_layout_screen("Reality/home")); // case-sensitive
    }
}
