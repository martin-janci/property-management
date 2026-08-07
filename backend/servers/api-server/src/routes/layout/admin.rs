use crate::routes::platform_admin::extract_super_admin_token;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use db::models::layout::LayoutConfigRow;
use db::repositories::{LayoutChangeEventKind, LayoutRepository};
use db::{PublicConnection, RlsPool};
use uuid::Uuid;

use super::types::{
    KillRequest, PreviewResolveRequest, PublishRequest, PutDraftRequest, PutManifestRequest,
    PutRailsRequest, RollbackRequest, ScreenQuery, ValidationErrorsResponse,
};
use super::webhook;

type ErrorResponse = (StatusCode, Json<ValidationErrorsResponse>);

/// Build a single-message error body. Mirrors the sibling tenant-override
/// handlers (`tenant.rs`) so error construction lives in one place.
fn error_response(status: StatusCode, msg: impl Into<String>) -> ErrorResponse {
    (
        status,
        Json(ValidationErrorsResponse {
            errors: vec![msg.into()],
        }),
    )
}

/// 422 — strictly for validation results on the *request* (unparseable request
/// payloads, publish-gate validation errors). Infra failures go through
/// [`internal_error`] instead. Takes the full error list because publish-gate
/// validation can surface many at once.
fn bad_request(errors: Vec<String>) -> ErrorResponse {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ValidationErrorsResponse { errors }),
    )
}

/// 500 — infra failures (pool acquire, query errors, corrupt stored data).
/// Logs the real error server-side; the client gets a generic message so raw
/// sqlx/serde error text never leaks.
fn internal_error(err: impl std::fmt::Display) -> ErrorResponse {
    tracing::error!(error = %err, "layout admin: internal error");
    error_response(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
}

/// 403 — the shared platform-admin auth gate rejection.
fn forbidden() -> ErrorResponse {
    error_response(StatusCode::FORBIDDEN, "forbidden")
}

/// 404 — unknown screen (config row missing).
fn unknown_screen() -> ErrorResponse {
    error_response(StatusCode::NOT_FOUND, "unknown screen")
}

/// Platform-admin gate shared by every handler: validate the super-admin bearer
/// token and map any rejection to a uniform 403. Returns the admin id (+ token
/// string) for handlers that record it on change events.
fn require_super_admin(
    headers: &axum::http::HeaderMap,
    state: &AppState,
) -> Result<(Uuid, String), ErrorResponse> {
    extract_super_admin_token(headers, state).map_err(|_| forbidden())
}

/// Acquire the sanctioned public/global (no-RLS) connection for the global
/// layout tables, mapping a pool failure to 500. Every layout-admin read/write
/// runs against this connection (clears stale RLS context on acquire).
async fn acquire_public_conn(state: &AppState) -> Result<PublicConnection, ErrorResponse> {
    RlsPool::new(state.db.clone())
        .acquire_public()
        .await
        .map_err(internal_error)
}

/// Load a screen's layout config, mapping a missing row to 404 and any sqlx
/// failure to 500. Shared by `get_config` and `publish` so the fetch +
/// not-found mapping stays in one place (mirrors `tenant.rs::load_screen_config`).
async fn load_config(
    repo: &LayoutRepository,
    conn: &mut sqlx::PgConnection,
    screen: &str,
) -> Result<LayoutConfigRow, ErrorResponse> {
    repo.get_config(conn, screen)
        .await
        .map_err(internal_error)?
        .ok_or_else(unknown_screen)
}

/// Persist a `layout_change_published` analytics event to the append-only sink
/// (migration 00225). Fire-and-forget: a failure is logged at `warn!` and never
/// fails the admin mutation, which already committed (events doc §7).
async fn record_change_published(
    repo: &LayoutRepository,
    conn: &mut sqlx::PgConnection,
    screen: &str,
    delivery_id: Uuid,
    published_by: Option<Uuid>,
    props: serde_json::Value,
) {
    if let Err(e) = repo
        .record_change_event(
            &mut *conn,
            LayoutChangeEventKind::Published,
            Some(screen),
            Some(delivery_id),
            published_by,
            &props,
        )
        .await
    {
        tracing::warn!(
            error = %e,
            screen = %screen,
            delivery_id = %delivery_id,
            "failed to persist layout_change_published event"
        );
    }
}

#[utoipa::path(get, path = "/api/v1/platform-admin/layout/screens", tag = "Layout Admin",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "All layout configs")))]
pub async fn list_screens(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    require_super_admin(&headers, &state)?;
    let mut conn = acquire_public_conn(&state).await?;
    let rows = LayoutRepository::new()
        .list_configs(&mut **conn)
        .await
        .map_err(internal_error)?;
    Ok(Json(serde_json::to_value(rows).map_err(internal_error)?))
}

#[utoipa::path(get, path = "/api/v1/platform-admin/layout/config", tag = "Layout Admin",
    security(("bearer_auth" = [])), params(("screen" = String, Query, description = "Screen id")),
    responses((status = 200, description = "Config with versions and kills"), (status = 404, description = "Unknown screen")))]
pub async fn get_config(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(q): Query<ScreenQuery>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    require_super_admin(&headers, &state)?;
    let mut conn = acquire_public_conn(&state).await?;
    let repo = LayoutRepository::new();
    let cfg = load_config(&repo, &mut **conn, &q.screen).await?;
    let versions = repo
        .list_versions(&mut **conn, &q.screen)
        .await
        .map_err(internal_error)?;
    let kills = repo
        .list_kills(&mut **conn, &q.screen)
        .await
        .map_err(internal_error)?;
    Ok(Json(
        serde_json::json!({ "config": cfg, "versions": versions, "kills": kills }),
    ))
}

#[utoipa::path(put, path = "/api/v1/platform-admin/layout/draft", tag = "Layout Admin",
    security(("bearer_auth" = [])), request_body = PutDraftRequest,
    responses((status = 200, description = "Draft saved"), (status = 422, description = "Config does not parse as a ScreenConfig")))]
pub async fn put_draft(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<PutDraftRequest>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let (admin_id, _) = require_super_admin(&headers, &state)?;
    // shape gate: must parse as the layout-core contract type
    if let Err(e) = serde_json::from_value::<layout_core::ScreenConfig>(req.config.clone()) {
        return Err(bad_request(vec![format!("invalid ScreenConfig: {e}")]));
    }
    let mut conn = acquire_public_conn(&state).await?;
    let row = LayoutRepository::new()
        .upsert_draft(&mut **conn, &req.screen, &req.config, Some(admin_id))
        .await
        .map_err(internal_error)?;
    Ok(Json(serde_json::to_value(row).map_err(internal_error)?))
}

#[utoipa::path(put, path = "/api/v1/platform-admin/layout/rails", tag = "Layout Admin",
    security(("bearer_auth" = [])), request_body = PutRailsRequest,
    responses((status = 200, description = "Rails saved"), (status = 422, description = "Rails do not parse")))]
pub async fn put_rails(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<PutRailsRequest>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let (admin_id, _) = require_super_admin(&headers, &state)?;
    if let Err(e) = serde_json::from_value::<layout_core::Rails>(req.rails.clone()) {
        return Err(bad_request(vec![format!("invalid Rails: {e}")]));
    }
    let mut conn = acquire_public_conn(&state).await?;
    let row = LayoutRepository::new()
        .set_rails(&mut **conn, &req.screen, &req.rails, Some(admin_id))
        .await
        .map_err(internal_error)?;
    Ok(Json(serde_json::to_value(row).map_err(internal_error)?))
}

#[utoipa::path(get, path = "/api/v1/platform-admin/layout/manifests", tag = "Layout Admin",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "All registry manifests")))]
pub async fn list_manifests(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    require_super_admin(&headers, &state)?;
    let mut conn = acquire_public_conn(&state).await?;
    let rows = LayoutRepository::new()
        .list_manifests(&mut **conn)
        .await
        .map_err(internal_error)?;
    Ok(Json(serde_json::to_value(rows).map_err(internal_error)?))
}

#[utoipa::path(put, path = "/api/v1/platform-admin/layout/manifests", tag = "Layout Admin",
    security(("bearer_auth" = [])), request_body = PutManifestRequest,
    responses((status = 200, description = "Manifest saved"), (status = 422, description = "Manifest invalid")))]
pub async fn put_manifest(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<PutManifestRequest>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let (admin_id, _) = require_super_admin(&headers, &state)?;
    let parsed: layout_core::RegistryManifest = serde_json::from_value(req.manifest.clone())
        .map_err(|e| bad_request(vec![format!("invalid RegistryManifest: {e}")]))?;
    let platform_str = match parsed.platform {
        layout_core::Platform::Web => "web",
        layout_core::Platform::Mobile => "mobile",
    };
    if platform_str != req.platform {
        return Err(bad_request(vec![format!(
            "platform mismatch: body says {}, manifest says {platform_str}",
            req.platform
        )]));
    }
    let mut conn = acquire_public_conn(&state).await?;
    let row = LayoutRepository::new()
        .upsert_manifest(&mut **conn, &req.platform, &req.manifest, Some(admin_id))
        .await
        .map_err(internal_error)?;
    Ok(Json(serde_json::to_value(row).map_err(internal_error)?))
}

#[utoipa::path(post, path = "/api/v1/platform-admin/layout/publish", tag = "Layout Admin",
    security(("bearer_auth" = [])), request_body = PublishRequest,
    responses((status = 200, description = "Published"),
              (status = 404, description = "Unknown screen"),
              (status = 409, description = "No registry manifests uploaded yet, or draft changed during publish (retry)"),
              (status = 422, description = "Validation errors — publish blocked")))]
pub async fn publish(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<PublishRequest>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let (admin_id, _) = require_super_admin(&headers, &state)?;
    let mut conn = acquire_public_conn(&state).await?;
    let repo = LayoutRepository::new();

    let cfg_row = load_config(&repo, &mut **conn, &req.screen).await?;

    let draft: layout_core::ScreenConfig =
        serde_json::from_value(cfg_row.draft.clone()).map_err(|e| {
            bad_request(vec![format!(
                "stored draft is not a valid ScreenConfig: {e}"
            )])
        })?;
    // The exact draft value we validated — publish is guarded on it (TOCTOU).
    let validated_draft = cfg_row.draft.clone();

    let manifest_rows = repo
        .list_manifests(&mut **conn)
        .await
        .map_err(internal_error)?;
    if manifest_rows.is_empty() {
        return Err(error_response(
            StatusCode::CONFLICT,
            "no registry manifests uploaded; cannot validate publish",
        ));
    }
    let manifests: Vec<layout_core::RegistryManifest> = manifest_rows
        .iter()
        .map(|r| serde_json::from_value(r.manifest.clone()))
        .collect::<Result<_, _>>()
        .map_err(internal_error)?;

    let errors = layout_core::validate_publish(&draft, &manifests);
    if !errors.is_empty() {
        return Err(bad_request(errors.iter().map(|e| e.to_string()).collect()));
    }

    // Guarded on the validated draft: a concurrent draft PUT between the
    // validation above and this UPDATE yields DraftChanged (409), never an
    // unvalidated publish.
    let row = repo
        .publish(&mut conn, &req.screen, &validated_draft, Some(admin_id))
        .await
        .map_err(|e| match e {
            db::repositories::LayoutPublishError::ScreenNotFound => unknown_screen(),
            db::repositories::LayoutPublishError::DraftChanged => {
                error_response(StatusCode::CONFLICT, "draft changed during publish, retry")
            }
            db::repositories::LayoutPublishError::Sqlx(e) => internal_error(e),
        })?;

    // One correlation id shared by the published event, the outbound webhook,
    // and (echoed) the receiver's revalidate event (events doc gap D).
    let delivery_id = Uuid::new_v4();
    let props = serde_json::json!({
        "event": LayoutChangeEventKind::Published.as_str(),
        "change_kind": "published",
        "published_by": admin_id,
        "layout_version": row.published_version,
        "target_tenant": "*",
    });
    record_change_published(
        &repo,
        &mut conn,
        &req.screen,
        delivery_id,
        Some(admin_id),
        props,
    )
    .await;
    webhook::notify_layout_change(
        state.db.clone(),
        &req.screen,
        "published",
        Some(row.published_version),
        delivery_id,
    );
    Ok(Json(serde_json::to_value(row).map_err(internal_error)?))
}

#[utoipa::path(post, path = "/api/v1/platform-admin/layout/rollback", tag = "Layout Admin",
    security(("bearer_auth" = [])), request_body = RollbackRequest,
    responses((status = 200, description = "Rolled back"), (status = 404, description = "Unknown screen or version")))]
pub async fn rollback(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<RollbackRequest>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let (admin_id, _) = require_super_admin(&headers, &state)?;
    let mut conn = acquire_public_conn(&state).await?;
    let repo = LayoutRepository::new();
    let row = repo
        .rollback(&mut conn, &req.screen, req.version, Some(admin_id))
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => {
                error_response(StatusCode::NOT_FOUND, "unknown screen or version")
            }
            other => internal_error(other),
        })?;

    let delivery_id = Uuid::new_v4();
    let props = serde_json::json!({
        "event": LayoutChangeEventKind::Published.as_str(),
        "change_kind": "rolled_back",
        "published_by": admin_id,
        "layout_version": row.published_version,
        // The source version requested — distinct from the new layout_version.
        "rolled_back_to": req.version,
        "target_tenant": "*",
    });
    record_change_published(
        &repo,
        &mut conn,
        &req.screen,
        delivery_id,
        Some(admin_id),
        props,
    )
    .await;
    webhook::notify_layout_change(
        state.db.clone(),
        &req.screen,
        "rolled_back",
        Some(row.published_version),
        delivery_id,
    );
    Ok(Json(serde_json::to_value(row).map_err(internal_error)?))
}

#[utoipa::path(post, path = "/api/v1/platform-admin/layout/kill", tag = "Layout Admin",
    security(("bearer_auth" = [])), request_body = KillRequest,
    responses((status = 204, description = "Section killed — bypasses the publish gate (spec §5)")))]
pub async fn kill(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<KillRequest>,
) -> Result<StatusCode, ErrorResponse> {
    let (admin_id, _) = require_super_admin(&headers, &state)?;
    let mut conn = acquire_public_conn(&state).await?;
    let repo = LayoutRepository::new();
    repo.kill(&mut **conn, &req.screen, &req.section_type, Some(admin_id))
        .await
        .map_err(internal_error)?;

    let delivery_id = Uuid::new_v4();
    let props = serde_json::json!({
        "event": LayoutChangeEventKind::Published.as_str(),
        "change_kind": "killed",
        "published_by": admin_id,
        // kill/unkill bypass the publish gate (spec §5) — no version row; the
        // killed section identifies the change instead (events doc §5.1).
        "layout_version": serde_json::Value::Null,
        "section_type": req.section_type,
        "target_tenant": "*",
    });
    record_change_published(
        &repo,
        &mut conn,
        &req.screen,
        delivery_id,
        Some(admin_id),
        props,
    )
    .await;
    webhook::notify_layout_change(state.db.clone(), &req.screen, "killed", None, delivery_id);
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/api/v1/platform-admin/layout/preview-resolve", tag = "Layout Admin",
    security(("bearer_auth" = [])), request_body = PreviewResolveRequest,
    responses((status = 200, description = "Resolved section list for the given config"),
              (status = 404, description = "No manifest stored for the platform"),
              (status = 422, description = "Invalid config, unknown platform, or stored manifest unparseable")))]
pub async fn preview_resolve(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<PreviewResolveRequest>,
) -> Result<Json<layout_core::ResolvedScreen>, ErrorResponse> {
    require_super_admin(&headers, &state)?;

    // Parse and validate the submitted config.
    let config: layout_core::ScreenConfig = serde_json::from_value(req.config.clone())
        .map_err(|e| bad_request(vec![format!("invalid ScreenConfig: {e}")]))?;

    // Parse platform string — mirror resolved.rs's parse_platform exactly.
    let platform = super::resolved::parse_platform(Some(req.platform.as_str()))
        .map_err(|e| bad_request(vec![e]))?;

    let platform_key = match platform {
        layout_core::Platform::Web => "web",
        layout_core::Platform::Mobile => "mobile",
    };

    let mut conn = acquire_public_conn(&state).await?;
    let repo = LayoutRepository::new();

    let manifest_row = repo
        .get_manifest(&mut **conn, platform_key)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            error_response(StatusCode::NOT_FOUND, "no registry manifest for platform")
        })?;
    let manifest: layout_core::RegistryManifest = serde_json::from_value(manifest_row.manifest)
        .map_err(|e| bad_request(vec![format!("stored manifest invalid: {e}")]))?;

    let kills: std::collections::BTreeSet<layout_core::SectionType> = repo
        .list_kills(&mut **conn, &config.screen)
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(|k| layout_core::SectionType(k.section_type))
        .collect();

    let resolved = layout_core::resolve(&config, platform, None, &kills, &manifest);
    Ok(Json(resolved))
}

#[utoipa::path(post, path = "/api/v1/platform-admin/layout/unkill", tag = "Layout Admin",
    security(("bearer_auth" = [])), request_body = KillRequest,
    responses((status = 204, description = "Kill flag removed"), (status = 404, description = "No such kill flag")))]
pub async fn unkill(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<KillRequest>,
) -> Result<StatusCode, ErrorResponse> {
    let (admin_id, _) = require_super_admin(&headers, &state)?;
    let mut conn = acquire_public_conn(&state).await?;
    let repo = LayoutRepository::new();
    let removed = repo
        .unkill(&mut **conn, &req.screen, &req.section_type)
        .await
        .map_err(internal_error)?;
    if removed {
        let delivery_id = Uuid::new_v4();
        let props = serde_json::json!({
            "event": LayoutChangeEventKind::Published.as_str(),
            "change_kind": "unkilled",
            "published_by": admin_id,
            "layout_version": serde_json::Value::Null,
            "section_type": req.section_type,
            "target_tenant": "*",
        });
        record_change_published(
            &repo,
            &mut conn,
            &req.screen,
            delivery_id,
            Some(admin_id),
            props,
        )
        .await;
        webhook::notify_layout_change(state.db.clone(), &req.screen, "unkilled", None, delivery_id);
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(error_response(StatusCode::NOT_FOUND, "no such kill flag"))
    }
}
