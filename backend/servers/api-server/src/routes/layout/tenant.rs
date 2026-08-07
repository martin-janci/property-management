use crate::state::AppState;
use api_core::extractors::RlsConnection;
use api_core::AuthUser;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use common::TenantRole;
use db::models::layout::LayoutConfigRow;
use db::repositories::LayoutRepository;
use db::RlsPool;
use sqlx::{Executor, Postgres};
use uuid::Uuid;

use super::types::{PutTenantOverrideRequest, ScreenQuery, ValidationErrorsResponse};

/// Serialized-size cap for a tenant override (64 KiB). Overrides are sparse
/// deltas; anything bigger is abuse or a bug.
const MAX_OVERRIDE_BYTES: usize = 64 * 1024;

type ErrorResponse = (StatusCode, Json<ValidationErrorsResponse>);

fn error_response(status: StatusCode, msg: impl Into<String>) -> ErrorResponse {
    (
        status,
        Json(ValidationErrorsResponse {
            errors: vec![msg.into()],
        }),
    )
}

/// 500 — infra failures. Logs the real error server-side; the client gets a
/// generic message so raw sqlx/serde error text never leaks.
fn internal_error(err: impl std::fmt::Display) -> ErrorResponse {
    tracing::error!(error = %err, "layout tenant: internal error");
    error_response(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
}

/// Nil organization sentinel guard: on platform hosts the middleware resolves
/// `organization_id = Uuid::nil()`. A layout override stored under the nil org
/// would be shared across all orgs, so reject with 400 before any DB access.
fn nil_org_error() -> ErrorResponse {
    error_response(
        StatusCode::BAD_REQUEST,
        "layout tenant overrides require an organization-scoped host",
    )
}

fn admin_required_error() -> ErrorResponse {
    error_response(StatusCode::FORBIDDEN, "org admin role required")
}

/// Org-admin gate shared by both handlers. Rejects the nil-org sentinel
/// (platform hosts — an override stored under `Uuid::nil()` would leak across
/// every org) with 400, then non-admin roles with 403. Rails/manifest/config
/// are org-admin material. A caller still holding an `RlsConnection` must
/// release it before propagating the returned error (see `get_tenant_override`).
fn require_org_admin(org_id: Uuid, role: TenantRole) -> Result<(), ErrorResponse> {
    if org_id.is_nil() {
        return Err(nil_org_error());
    }
    if !role.is_admin() {
        return Err(admin_required_error());
    }
    Ok(())
}

/// Load a screen's layout config from a public/global (no-RLS) connection,
/// mapping a missing row to 404 and any sqlx failure to 500. Shared by both
/// handlers so the fetch + not-found mapping stays in one place.
async fn load_screen_config<'e, E>(
    repo: &LayoutRepository,
    executor: E,
    screen: &str,
) -> Result<LayoutConfigRow, ErrorResponse>
where
    E: Executor<'e, Database = Postgres>,
{
    repo.get_config(executor, screen)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "unknown screen"))
}

#[utoipa::path(get, path = "/api/v1/layout/tenant-override", tag = "Layout",
    security(("bearer_auth" = [])), params(("screen" = String, Query, description = "Screen id")),
    responses((status = 200, description = "Override + rails + published base + web manifest for the org"),
              (status = 400, description = "No organization-scoped host"),
              (status = 403, description = "Org admin role required"),
              (status = 404, description = "Unknown screen")))]
pub async fn get_tenant_override(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Query(q): Query<ScreenQuery>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    // Authz on the DB-validated identity (RlsConnection validates membership),
    // not the raw JWT claim. Refuse the nil-org sentinel / non-admins before
    // touching the DB; release the held connection on rejection.
    let org_id = rls.tenant_id();
    if let Err(e) = require_org_admin(org_id, rls.role()) {
        rls.release().await;
        return Err(e);
    }
    let repo = LayoutRepository::new();
    let ov = repo
        .get_tenant_override(&mut **rls.conn(), org_id, &q.screen)
        .await;
    rls.release().await;
    let ov = ov.map_err(internal_error)?;
    // Global no-RLS layout tables — sanctioned public connection (clears stale context).
    let mut pub_conn = RlsPool::new(state.db.clone())
        .acquire_public()
        .await
        .map_err(internal_error)?;
    let cfg = load_screen_config(&repo, &mut **pub_conn, &q.screen).await?;
    let manifest_row = repo
        .get_manifest(&mut **pub_conn, "web")
        .await
        .map_err(internal_error)?;
    Ok(Json(serde_json::json!({
        "override": ov,
        "rails": cfg.rails,
        "published": cfg.published,
        "manifest": manifest_row.map(|r| r.manifest),
    })))
}

#[utoipa::path(put, path = "/api/v1/layout/tenant-override", tag = "Layout",
    security(("bearer_auth" = [])), request_body = PutTenantOverrideRequest,
    responses((status = 200, description = "Override saved"),
              (status = 400, description = "No organization-scoped host"),
              (status = 403, description = "Org admin role required"),
              (status = 404, description = "Screen not published"),
              (status = 422, description = "Out-of-rails edits, oversized override, or unparseable override rejected")))]
pub async fn put_tenant_override(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Json(req): Json<PutTenantOverrideRequest>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    // Gates on the DB-validated identity from RlsConnection (membership
    // checked), then release the connection immediately: the public/global
    // reads below must not run while an RLS connection is held (pool
    // starvation risk — mirror the GET handler's read-then-acquire pattern).
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let role = rls.role();
    let is_super_admin = rls.is_super_admin();
    rls.release().await;

    require_org_admin(org_id, role)?;

    let parse_err = |e: serde_json::Error, what: &str| {
        error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("invalid {what}: {e}"),
        )
    };

    let ov: layout_core::TenantOverride = serde_json::from_value(req.override_config.clone())
        .map_err(|e| parse_err(e, "TenantOverride"))?;

    // Canonical form: what the parsed/validated struct round-trips to. This is
    // what gets persisted — unknown junk keys in the raw request are dropped.
    let canonical = serde_json::to_value(&ov).map_err(internal_error)?;

    // Size cap on the canonical serialized form.
    let serialized_len = canonical.to_string().len();
    if serialized_len > MAX_OVERRIDE_BYTES {
        return Err(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("override_config too large: {serialized_len} bytes (max {MAX_OVERRIDE_BYTES})"),
        ));
    }

    let repo = LayoutRepository::new();
    let rls_pool = RlsPool::new(state.db.clone());

    // Phase 1: public/global reads (config + rails), no RLS connection held.
    // Global no-RLS layout tables — sanctioned public connection (clears stale context).
    let (base, rails_value) = {
        let mut pub_conn = rls_pool.acquire_public().await.map_err(internal_error)?;
        let cfg = load_screen_config(&repo, &mut **pub_conn, &req.screen).await?;
        let published = cfg.published.clone().ok_or_else(|| {
            error_response(StatusCode::NOT_FOUND, "screen has no published config")
        })?;
        let base: layout_core::ScreenConfig =
            serde_json::from_value(published).map_err(internal_error)?;
        (base, cfg.rails)
    };
    let rails: layout_core::Rails = serde_json::from_value(rails_value).map_err(internal_error)?;

    let errors = layout_core::validate_tenant_override(&ov, &base, &rails);
    if !errors.is_empty() {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ValidationErrorsResponse {
                errors: errors.iter().map(|e| e.to_string()).collect(),
            }),
        ));
    }

    // Phase 2: org-scoped write on a fresh RLS-contexted connection, acquired
    // only after the public connection above was released.
    let mut guard = rls_pool
        .acquire_with_rls(org_id, user_id, is_super_admin)
        .await
        .map_err(internal_error)?;
    let saved = repo
        .upsert_tenant_override(
            &mut **guard.conn(),
            org_id,
            &req.screen,
            &canonical,
            Some(user_id),
        )
        .await;
    guard.release().await;
    let saved = saved.map_err(internal_error)?;
    Ok(Json(serde_json::to_value(saved).map_err(internal_error)?))
}
