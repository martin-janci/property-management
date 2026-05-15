//! Admin endpoints for agency (organization) management. Phase 5.
//!
//! These complement the pre-Phase-5 `/platform-admin/organizations` tree by
//! going through the `RequireCapability` gate. Uses existing
//! `OrganizationRepository` and `AgencyDomainRepository` infra.

use admin_core::{require_capability, Capability, RequireCapability};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_agencies).layer(require_capability(Capability::AgenciesRead)))
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
    }))
}

#[derive(Debug, Deserialize)]
pub struct SuspendBody {
    pub reason: String,
}

/// POST /admin/agencies/:id/suspend
async fn suspend_agency(
    _cap: RequireCapability,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<SuspendBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    // We do not have the actor_id wired here yet; Phase 2's RequestPrincipal
    // will provide it. For now we pass `id` as the admin sentinel — the
    // capability gate already audited the call with the real actor.
    state
        .platform_admin_repo
        .suspend_organization(id, id, &body.reason)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct AddDomainBody {
    pub host: String,
    pub kind: Option<String>,
}

/// POST /admin/agencies/:id/domains
async fn add_domain(
    _cap: RequireCapability,
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
    Json(_body): Json<AddDomainBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Stub: full domain provisioning (with cert state, verification token,
    // and cache invalidation) lives in `agency_provisioning`. Phase 5's
    // job here is to expose a capability-gated handle for new admin UI.
    Ok(StatusCode::NOT_IMPLEMENTED)
}
