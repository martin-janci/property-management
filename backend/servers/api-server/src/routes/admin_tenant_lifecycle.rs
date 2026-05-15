//! Admin tenant-lifecycle routes (Phase 5.5 — Tenant Lifecycle & Operability).
//!
//! Mounts under `/api/v1/admin/tenants/:id/{export,purge,restore}`. Defenses
//! #16 (no in-place restore), #17 (manifest-driven purge), #18 (per-tenant
//! logical export).
//!
//! # Auth (Phase 5 — N4)
//!
//! Capability gating is wired via `admin-core::RequireCapability`. Each route
//! declares its required capability at router-build time:
//!
//!   * `POST /tenants/{id}/export`  → `Capability::TenantExport`
//!   * `POST /tenants/{id}/purge`   → `Capability::TenantPurge`
//!   * `POST /tenants/restore`      → `Capability::TenantRestore`
//!
//! The extractor enforces platform-principal + active capability grant +
//! recent MFA, and writes an audit row for both allowed and denied invocations.
//! This replaces the previous `AuthUser::is_platform_admin()` check, which
//! trusted the JWT `roles` claim — a token-claim-forgery risk (leak #11).
//
// TODO(N4-followup): expose lifecycle status polling under
// /admin/tenants/{id}/operations/{job_id} so long-running export/purge/restore
// jobs can be observed without holding the HTTP request open.

use admin_core::{require_capability, Capability, RequireCapability};
use api_core::extractors::principal::RequestPrincipal;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use axum_extra::extract::Multipart;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tenant_ops::{
    export_tenant, purge_tenant, restore_tenant_export, PurgeReport, RestoreReport,
    TenantDataManifest, TenantExport,
};
use uuid::Uuid;

use crate::state::AppState;

/// Tenant-lifecycle router.
///
/// Each route lives in its own one-route sub-router so the per-route tower
/// layer (which carries the `RequiredCapabilityMarker` for that capability)
/// stays scoped correctly when the routers are merged.
pub fn router() -> Router<AppState> {
    let export = Router::new().route(
        "/tenants/{id}/export",
        post(export_handler).layer(require_capability(Capability::TenantExport)),
    );
    let purge = Router::new().route(
        "/tenants/{id}/purge",
        post(purge_handler).layer(require_capability(Capability::TenantPurge)),
    );
    let restore = Router::new().route(
        "/tenants/restore",
        post(restore_handler).layer(require_capability(Capability::TenantRestore)),
    );
    export.merge(purge).merge(restore)
}

#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    /// Override output directory. Defaults to `./exports/`.
    pub out_dir: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExportResponse {
    pub tarball_path: String,
    pub bytes_written: u64,
    pub rows_exported: u64,
    pub tables_exported: u32,
}

impl From<TenantExport> for ExportResponse {
    fn from(e: TenantExport) -> Self {
        Self {
            tarball_path: e.tarball_path.to_string_lossy().to_string(),
            bytes_written: e.bytes_written,
            rows_exported: e.rows_exported,
            tables_exported: e.tables_exported,
        }
    }
}

async fn export_handler(
    _cap: RequireCapability,
    principal: RequestPrincipal,
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    body: Option<Json<ExportRequest>>,
) -> Result<Json<ExportResponse>, (StatusCode, String)> {
    let manifest = load_manifest()?;
    let out_dir = body
        .and_then(|Json(b)| b.out_dir)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./exports"));
    let report = export_tenant(&state.db, org_id, &manifest, &out_dir)
        .await
        .map_err(internal)?;
    tracing::info!(
        actor = %principal.user_id,
        org = %org_id,
        rows = report.rows_exported,
        bytes = report.bytes_written,
        "tenant export complete"
    );
    Ok(Json(report.into()))
}

#[derive(Debug, Serialize)]
pub struct PurgeResponse {
    pub organization_id: Uuid,
    pub total_rows_deleted: u64,
    pub organization_dropped: bool,
    pub tables_touched: usize,
    pub s3_keys_to_delete: Vec<String>,
}

impl From<PurgeReport> for PurgeResponse {
    fn from(r: PurgeReport) -> Self {
        Self {
            organization_id: r.organization_id,
            total_rows_deleted: r.total_rows_deleted,
            organization_dropped: r.organization_dropped,
            tables_touched: r.per_table.len(),
            s3_keys_to_delete: r.s3_keys_to_delete,
        }
    }
}

async fn purge_handler(
    _cap: RequireCapability,
    principal: RequestPrincipal,
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
) -> Result<Json<PurgeResponse>, (StatusCode, String)> {
    let manifest = load_manifest()?;
    let report = purge_tenant(&state.db, org_id, &manifest)
        .await
        .map_err(internal)?;
    tracing::warn!(
        actor = %principal.user_id,
        org = %org_id,
        rows = report.total_rows_deleted,
        org_dropped = report.organization_dropped,
        "tenant PURGE executed"
    );
    Ok(Json(report.into()))
}

#[derive(Debug, Serialize)]
pub struct RestoreResponse {
    pub original_organization_id: Uuid,
    pub new_organization_id: Uuid,
    pub rows_restored: u64,
    pub tables_restored: u32,
}

impl From<RestoreReport> for RestoreResponse {
    fn from(r: RestoreReport) -> Self {
        Self {
            original_organization_id: r.original_organization_id,
            new_organization_id: r.new_organization_id,
            rows_restored: r.rows_restored,
            tables_restored: r.tables_restored,
        }
    }
}

/// Restore takes a multipart upload of a previously-exported tarball; the
/// route writes it to a tempfile and feeds it to `restore_tenant_export`.
/// Defense #16 — restore *always* lands as a NEW org id, never in place.
async fn restore_handler(
    _cap: RequireCapability,
    principal: RequestPrincipal,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<RestoreResponse>, (StatusCode, String)> {
    // Pull the first file part — accept any field name.
    let mut bytes: Option<Vec<u8>> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("multipart: {e}")))?
    {
        let data = field
            .bytes()
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("read field: {e}")))?;
        bytes = Some(data.to_vec());
        break;
    }
    let bytes =
        bytes.ok_or((StatusCode::BAD_REQUEST, "missing tarball upload".to_string()))?;

    // Write to a tempfile so tenant-ops can stream it back in.
    let tmpdir = std::env::temp_dir();
    let tmpfile = tmpdir.join(format!("ppt-restore-{}.tar.gz", Uuid::new_v4()));
    std::fs::write(&tmpfile, &bytes)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("write tmp: {e}")))?;

    let manifest = load_manifest()?;
    let report = restore_tenant_export(&state.db, &tmpfile, &manifest)
        .await
        .map_err(internal)?;

    let _ = std::fs::remove_file(&tmpfile);

    tracing::warn!(
        actor = %principal.user_id,
        original = %report.original_organization_id,
        new = %report.new_organization_id,
        rows = report.rows_restored,
        "tenant restore complete (NEW org id minted)"
    );
    Ok(Json(report.into()))
}

// ============================================================================
// Helpers
// ============================================================================

fn load_manifest() -> Result<TenantDataManifest, (StatusCode, String)> {
    // Allow override via env so a deploy can ship the manifest at any path.
    let path = std::env::var("PPT_TENANT_MANIFEST_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| TenantDataManifest::default_path());
    TenantDataManifest::load(&path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("manifest load failed at {path:?}: {e}"),
        )
    })
}

fn internal(e: tenant_ops::TenantOpsError) -> (StatusCode, String) {
    tracing::error!(error = %e, "tenant-ops error");
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}
