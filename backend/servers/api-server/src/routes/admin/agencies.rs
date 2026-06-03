//! Admin endpoints for agency (organization) management. Phase 5.
//!
//! These complement the pre-Phase-5 `/platform-admin/organizations` tree by
//! going through the `RequireCapability` gate. Uses existing
//! `OrganizationRepository` and `AgencyDomainRepository` infra.

use admin_core::{require_capability, AuditOutcome, AuditWriter, Capability, RequireCapability};
use api_core::{extractors::principal::RequestPrincipal, AuthUser};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Extension, Json, Router,
};
use db::models::agency_domain::{AgencyDomain, AgencyDomainKind, CreateAgencyDomain};
use db::repositories::AgencyDomainRepository;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(list_agencies).layer(require_capability(Capability::AgenciesRead)),
        )
        .route(
            "/{id}",
            get(get_agency).layer(require_capability(Capability::AgenciesRead)),
        )
        .route(
            "/{id}/suspend",
            post(suspend_agency).layer(require_capability(Capability::AgenciesSuspend)),
        )
        .route(
            "/{id}/domains",
            post(add_domain).layer(require_capability(Capability::AgenciesWrite)),
        )
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub page_size: Option<u32>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub search: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AgencySummary {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub member_count: i64,
    pub building_count: i64,
}

#[derive(Debug, Serialize)]
pub struct AgencyListResponse {
    pub items: Vec<AgencySummary>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

/// GET /admin/agencies
async fn list_agencies(
    _cap: RequireCapability,
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<AgencyListResponse>, (StatusCode, String)> {
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).clamp(1, 100);
    let offset = ((page - 1) * page_size) as i64;

    let (orgs, total) = state
        .platform_admin_repo
        .list_organizations_with_metrics(
            offset,
            page_size as i64,
            q.status.as_deref(),
            q.search.as_deref(),
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = orgs
        .into_iter()
        .map(|m| AgencySummary {
            id: m.organization_id,
            name: m.name,
            slug: m.slug,
            status: m.status,
            created_at: m.created_at,
            member_count: m.member_count,
            building_count: m.building_count,
        })
        .collect();

    Ok(Json(AgencyListResponse {
        items,
        total,
        page,
        page_size,
    }))
}

#[derive(Debug, Serialize)]
pub struct AgencyDetailResponse {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub member_count: i64,
    pub active_member_count: i64,
    pub building_count: i64,
    pub unit_count: i64,
}

/// GET /admin/agencies/:id
async fn get_agency(
    _cap: RequireCapability,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<AgencyDetailResponse>, (StatusCode, String)> {
    let detail = state
        .platform_admin_repo
        .get_organization_detail(id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "agency not found".into()))?;

    Ok(Json(AgencyDetailResponse {
        id: detail.id,
        name: detail.name,
        slug: detail.slug,
        status: detail.status,
        created_at: detail.created_at,
        updated_at: detail.updated_at,
        member_count: detail.metrics.member_count,
        active_member_count: detail.metrics.active_member_count,
        building_count: detail.metrics.building_count,
        unit_count: detail.metrics.unit_count,
    }))
}

#[derive(Debug, Deserialize)]
pub struct SuspendBody {
    pub reason: String,
}

/// POST /admin/agencies/:id/suspend
async fn suspend_agency(
    _cap: RequireCapability,
    principal: RequestPrincipal,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<SuspendBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Pass the authenticated platform principal as actor_id so the audit row
    // attributes the suspension to the admin, not the target org. The old code
    // here passed `id` (the target org id) as the actor — a passive audit-trail
    // forgery vulnerability.
    state
        .platform_admin_repo
        .suspend_organization(id, principal.user_id, &body.reason)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Cascade the suspension: revoke every org member's sessions so the block
    // takes effect immediately rather than waiting for token expiry. Mirrors
    // the cascade in `platform_admin::suspend_organization`. Failures here are
    // logged but do not fail the request — the org is already suspended and the
    // auth path also drops suspended-org memberships at login/refresh.
    let user_ids = state
        .platform_admin_repo
        .get_org_member_user_ids(id)
        .await
        .unwrap_or_default();

    for user_id in &user_ids {
        if let Err(e) = state
            .session_repo
            .revoke_all_user_tokens(*user_id, None)
            .await
        {
            tracing::warn!(
                user_id = %user_id,
                error = %e,
                "Failed to revoke sessions for suspended org member"
            );
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct AddDomainBody {
    pub host: String,
    pub kind: Option<String>,
}

/// Basic host validation — lowercase ASCII, dots / hyphens / digits only,
/// no leading/trailing dot, length ≤ 253. Matches `is_valid_host` in
/// `routes::agency_provisioning`. Rejects schemes (`http://…`) and paths.
fn is_valid_host(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 253
        && host
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.')
        && !host.starts_with('.')
        && !host.ends_with('.')
        && host.contains('.')
}

/// POST /admin/agencies/:id/domains
///
/// Attaches a custom domain to an agency (organization). The `RequireCapability`
/// extractor (mounted via `require_capability(Capability::AgenciesWrite)`) has
/// already enforced platform-principal + recent-MFA + active grant by the
/// time we get here, so this handler can focus on validation + persistence.
async fn add_domain(
    _cap: RequireCapability,
    auth: AuthUser,
    State(state): State<AppState>,
    Extension(audit): Extension<Arc<dyn AuditWriter>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(body): Json<AddDomainBody>,
) -> Result<(StatusCode, Json<AgencyDomain>), (StatusCode, String)> {
    let host = body.host.trim().to_ascii_lowercase();
    if !is_valid_host(&host) {
        return Err((
            StatusCode::BAD_REQUEST,
            "host must be a valid lowercase DNS host name (no scheme, no path)".into(),
        ));
    }

    let kind = match body.kind.as_deref() {
        Some("subdomain") => Some(AgencyDomainKind::Subdomain),
        Some("custom") | None => Some(AgencyDomainKind::Custom),
        Some(other) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("invalid kind '{}': expected 'subdomain' or 'custom'", other),
            ));
        }
    };

    let repo = AgencyDomainRepository::new(state.db.clone())
        .with_cache(state.tenant_resolution_cache.clone());

    let domain = repo
        .create_rls(
            &state.db,
            CreateAgencyDomain {
                organization_id: id,
                host: host.clone(),
                kind,
                is_primary: false,
            },
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to attach domain to agency");
            (StatusCode::BAD_REQUEST, e.to_string())
        })?;

    let ip = headers.get("x-forwarded-for").and_then(|h| h.to_str().ok());
    let ua = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|h| h.to_str().ok());
    let payload = serde_json::json!({ "host": host, "kind": body.kind });
    if let Err(e) = audit
        .record(
            Some(auth.user_id),
            Capability::AgenciesWrite,
            AuditOutcome::Allowed,
            Some("agency_domain"),
            Some(domain.id),
            Some(&payload),
            ip,
            ua,
        )
        .await
    {
        tracing::warn!(error = %e, "audit write for agency domain attach failed");
    }

    Ok((StatusCode::CREATED, Json(domain)))
}
