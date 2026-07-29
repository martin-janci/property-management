use crate::state::AppState;
use api_core::extractors::RlsConnection;
use api_core::{AuthUser, TenantExtractor};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use db::repositories::LayoutRepository;
use db::RlsPool;
use std::collections::BTreeSet;

use super::types::{ResolvedQuery, ValidationErrorsResponse};

pub fn parse_platform(s: Option<&str>) -> Result<layout_core::Platform, String> {
    match s.unwrap_or("web") {
        "web" => Ok(layout_core::Platform::Web),
        "mobile" => Ok(layout_core::Platform::Mobile),
        other => Err(format!("unknown platform {other:?} (expected web|mobile)")),
    }
}

#[utoipa::path(get, path = "/api/v1/layout/resolved/{screen}", tag = "Layout",
    security(("bearer_auth" = [])),
    params(("screen" = String, Path, description = "Screen id, e.g. ppt/dashboard"),
           ("platform" = Option<String>, Query, description = "web|mobile, default web")),
    responses((status = 200, description = "Resolved section list"),
              (status = 400, description = "Unknown platform or no organization-scoped host"),
              (status = 404, description = "Screen not published or manifest missing")))]
pub async fn get_resolved(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(screen): Path<String>,
    Query(q): Query<ResolvedQuery>,
) -> Result<Json<layout_core::ResolvedScreen>, (StatusCode, Json<ValidationErrorsResponse>)> {
    let err500 = |msg: String| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ValidationErrorsResponse { errors: vec![msg] }),
        )
    };
    let err404 = |msg: &str| {
        (
            StatusCode::NOT_FOUND,
            Json(ValidationErrorsResponse {
                errors: vec![msg.to_string()],
            }),
        )
    };

    let platform = match parse_platform(q.platform.as_deref()) {
        Ok(p) => p,
        Err(e) => {
            rls.release().await;
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ValidationErrorsResponse { errors: vec![e] }),
            ));
        }
    };
    let repo = LayoutRepository::new();
    // org_id comes from TenantExtractor while the override read runs on the
    // RLS connection whose context was bound by ValidatedTenantExtractor —
    // if the two ever disagreed, RLS returns no row (fails safe, no leak).
    let org_id = tenant.tenant_id;
    // Nil-org sentinel (platform hosts): a tenant override keyed on Uuid::nil()
    // would be shared across all orgs — refuse before any DB access.
    if org_id.is_nil() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ValidationErrorsResponse {
                errors: vec!["layout resolution requires an organization-scoped host".into()],
            }),
        ));
    }

    let tenant_ov_row = repo
        .get_tenant_override(&mut **rls.conn(), org_id, &screen)
        .await;
    rls.release().await;
    let tenant_ov_row = tenant_ov_row.map_err(|e| err500(format!("db error: {e}")))?;

    // Global no-RLS layout tables — sanctioned public connection (clears stale context).
    let mut pub_conn = RlsPool::new(state.db.clone())
        .acquire_public()
        .await
        .map_err(|e| err500(format!("db error: {e}")))?;
    let cfg = repo
        .get_config(&mut **pub_conn, &screen)
        .await
        .map_err(|e| err500(format!("db error: {e}")))?
        .ok_or_else(|| err404("unknown screen"))?;
    let Some(published) = cfg.published else {
        return Err(err404("screen has no published config"));
    };
    let base: layout_core::ScreenConfig = serde_json::from_value(published)
        .map_err(|e| err500(format!("stored published config invalid: {e}")))?;

    let platform_key = match platform {
        layout_core::Platform::Web => "web",
        layout_core::Platform::Mobile => "mobile",
    };
    let manifest_row = repo
        .get_manifest(&mut **pub_conn, platform_key)
        .await
        .map_err(|e| err500(format!("db error: {e}")))?
        .ok_or_else(|| err404("no registry manifest for platform"))?;
    let manifest: layout_core::RegistryManifest = serde_json::from_value(manifest_row.manifest)
        .map_err(|e| err500(format!("stored manifest invalid: {e}")))?;

    let kills: BTreeSet<layout_core::SectionType> = repo
        .list_kills(&mut **pub_conn, &screen)
        .await
        .map_err(|e| err500(format!("db error: {e}")))?
        .into_iter()
        .map(|k| layout_core::SectionType(k.section_type))
        .collect();

    let tenant_ov: Option<layout_core::TenantOverride> = match tenant_ov_row {
        Some(row) => Some(
            serde_json::from_value(row.override_config)
                .map_err(|e| err500(format!("stored tenant override invalid: {e}")))?,
        ),
        None => None,
    };

    let resolved = layout_core::resolve(&base, platform, tenant_ov.as_ref(), &kills, &manifest);
    Ok(Json(resolved))
}
