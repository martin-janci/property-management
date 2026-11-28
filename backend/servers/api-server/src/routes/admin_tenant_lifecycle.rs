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
use std::path::{Component, PathBuf};
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
    /// Optional output sub-directory, interpreted *relative to* the
    /// server-controlled export root (`PPT_EXPORT_ROOT`, default `./exports/`).
    /// Absolute paths and `..` traversal are rejected — see
    /// [`resolve_export_dir`].
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
    let out_dir = resolve_export_dir(body.and_then(|Json(b)| b.out_dir))?;
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
    if let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("multipart: {e}")))?
    {
        let data = field
            .bytes()
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("read field: {e}")))?;
        bytes = Some(data.to_vec());
    }
    let bytes = bytes.ok_or((
        StatusCode::BAD_REQUEST,
        "missing tarball upload".to_string(),
    ))?;

    // Write to a tempfile so tenant-ops can stream it back in.
    let tmpdir = std::env::temp_dir();
    let tmpfile = tmpdir.join(format!("ppt-restore-{}.tar.gz", Uuid::new_v4()));
    std::fs::write(&tmpfile, &bytes).map_err(|e| {
        // The io::Error carries the temp path; log it, don't return it.
        tracing::error!(path = ?tmpfile, error = %e, "restore tempfile write failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            INTERNAL_ERROR_BODY.to_string(),
        )
    })?;

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

/// Fixed, client-safe phrase returned for any *internal* tenant-lifecycle
/// failure. The real cause — filesystem paths, internal script paths, raw
/// `sqlx`/tarball errors carried by [`tenant_ops::TenantOpsError`] — is logged
/// server-side and never surfaced to the caller (leak hardening: an admin
/// error body must not expose server paths or internal infrastructure detail).
const INTERNAL_ERROR_BODY: &str = "internal error during tenant lifecycle operation";

/// Server-controlled root that every tenant export is confined to. A deploy
/// can relocate it via `PPT_EXPORT_ROOT`; otherwise exports land under
/// `./exports` (the documented default).
fn export_root() -> PathBuf {
    std::env::var_os("PPT_EXPORT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./exports"))
}

/// Resolve the client-supplied `out_dir` to a concrete directory *inside* the
/// server-controlled [`export_root`].
///
/// The export runs through a super-admin pool that bypasses RLS, so an
/// unvalidated `out_dir` is an arbitrary-directory write primitive (a caller
/// could point it at `/etc`, `../../…`, a UNC/drive path, …). We treat
/// `out_dir` strictly as a *relative* sub-path of the export root and reject
/// anything that could escape it: absolute paths, Windows drive/UNC prefixes,
/// backslashes, control characters, and any `..` parent-directory component.
/// Mirrors the existing `aml_dsa::validate_storage_path` convention.
fn resolve_export_dir(out_dir: Option<String>) -> Result<PathBuf, (StatusCode, String)> {
    let root = export_root();
    let Some(raw) = out_dir else {
        return Ok(root);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(root);
    }
    let reject = |msg: &str| Err((StatusCode::BAD_REQUEST, format!("out_dir {msg}")));
    if trimmed.chars().any(|c| c.is_control()) {
        return reject("must not contain control characters");
    }
    // Backslashes and colons would be plain filename characters on Unix but
    // path separators / drive markers on Windows — reject for cross-platform
    // safety and defense in depth.
    if trimmed.contains('\\') || trimmed.contains(':') {
        return reject("must not contain backslashes or drive letters");
    }
    // Component-level check: the only acceptable components are plain names and
    // `.`; a root/prefix means an absolute path, and `..` escapes the root.
    let requested = PathBuf::from(trimmed);
    for component in requested.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir => return reject("must not contain '..' components"),
            Component::RootDir | Component::Prefix(_) => return reject("must be a relative path"),
        }
    }
    Ok(root.join(requested))
}

fn load_manifest() -> Result<TenantDataManifest, (StatusCode, String)> {
    // Allow override via env so a deploy can ship the manifest at any path.
    let path = std::env::var("PPT_TENANT_MANIFEST_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| TenantDataManifest::default_path());
    TenantDataManifest::load(&path).map_err(|e| {
        // Log the real path + cause server-side; never return either to the
        // client (`{path:?}` previously leaked the server-side manifest path).
        tracing::error!(path = ?path, error = %e, "tenant manifest load failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            INTERNAL_ERROR_BODY.to_string(),
        )
    })
}

fn internal(e: tenant_ops::TenantOpsError) -> (StatusCode, String) {
    // `TenantOpsError`'s Display embeds internal detail (OS paths from
    // io::Error, internal script paths, raw sqlx/tarball errors). Log it, but
    // return only the fixed opaque phrase to the caller.
    tracing::error!(error = %e, "tenant-ops error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        INTERNAL_ERROR_BODY.to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // Regression: the admin tenant-export handler writes via an RLS-bypassing
    // super-admin pool, so a client-supplied `out_dir` must never be allowed to
    // escape the server-controlled export root. An absolute path, a `..`
    // traversal, or a drive/backslash path would be an arbitrary-directory
    // write primitive.

    #[test]
    fn resolve_export_dir_rejects_absolute_path() {
        let err = resolve_export_dir(Some("/etc/cron.d".to_string()))
            .expect_err("absolute out_dir must be rejected");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn resolve_export_dir_rejects_parent_traversal() {
        let err = resolve_export_dir(Some("../../var/www".to_string()))
            .expect_err("'..' traversal must be rejected");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn resolve_export_dir_rejects_embedded_traversal() {
        let err = resolve_export_dir(Some("nightly/../../../root".to_string()))
            .expect_err("embedded '..' traversal must be rejected");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn resolve_export_dir_rejects_backslash_and_control_chars() {
        assert!(
            resolve_export_dir(Some("..\\..\\windows".to_string())).is_err(),
            "backslash path must be rejected"
        );
        assert!(
            resolve_export_dir(Some("a\0b".to_string())).is_err(),
            "control characters must be rejected"
        );
    }

    #[test]
    fn resolve_export_dir_confines_relative_subdir_under_root() {
        let resolved = resolve_export_dir(Some("nightly/2026".to_string()))
            .expect("a legitimate relative subdir is allowed");
        assert!(
            resolved.starts_with(export_root()),
            "resolved path {resolved:?} escaped the export root"
        );
        assert!(resolved.ends_with("nightly/2026"));
    }

    #[test]
    fn resolve_export_dir_defaults_to_root_when_absent() {
        assert_eq!(resolve_export_dir(None).unwrap(), export_root());
    }

    // Regression (code-review: tenant-lifecycle path leak): the client-facing
    // error body for an internal failure must never carry internal detail —
    // filesystem paths, internal script paths, or raw DB/tarball errors. Those
    // belong in the server log only; the caller gets a fixed, opaque phrase.

    #[test]
    fn internal_error_does_not_leak_paths_or_internal_detail() {
        // `TenantOpsError`'s Display embeds an internal script path and the
        // offending table name — exactly the kind of detail we must not return.
        let leaky = tenant_ops::TenantOpsError::ManifestStale("secret_internal_table".to_string());
        let rendered = leaky.to_string();
        // Precondition: the error genuinely carries detail we have to hide.
        assert!(
            rendered.contains("check-rls-coverage.sh"),
            "test precondition changed: {rendered:?}"
        );
        assert!(rendered.contains("secret_internal_table"));

        let (status, body) = internal(leaky);
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(
            !body.contains("check-rls-coverage.sh"),
            "client body leaked an internal script path: {body:?}"
        );
        assert!(
            !body.contains("secret_internal_table"),
            "client body leaked internal detail: {body:?}"
        );
        assert!(
            !body.contains('/'),
            "client body still contains a path-like component: {body:?}"
        );
    }

    #[test]
    fn load_manifest_failure_does_not_leak_filesystem_path() {
        // Forcing a load failure must not surface the server-side manifest path
        // (previously interpolated via `{path:?}`) to the client.
        let bogus = "/var/lib/ppt-nonexistent/secret-manifest-location.json";
        // Safe here: edition 2021, and no other test mutates this var.
        std::env::set_var("PPT_TENANT_MANIFEST_PATH", bogus);
        let result = load_manifest();
        std::env::remove_var("PPT_TENANT_MANIFEST_PATH");

        let (status, body) = result.expect_err("a missing manifest must error");
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(
            !body.contains("secret-manifest-location") && !body.contains(bogus),
            "client body leaked the manifest filesystem path: {body:?}"
        );
        assert!(
            !body.contains('/'),
            "client body still contains a path-like component: {body:?}"
        );
    }
}
