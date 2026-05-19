//! Workflow Automation routes (Epic 38).
//!
//! Routes for automation rules, templates, and execution logs.
//!
//! # Auth model (SECURITY)
//!
//! Every handler in this module that reads or mutates tenant-scoped state
//! requires a verified `RequestPrincipal` (bearer JWT + host-resolved
//! tenant). The historical `extract_tenant_context` helper deserialized a
//! client-supplied `X-Tenant-Context` JSON header with no JWT verification
//! — any unauthenticated caller could forge tenancy. PR #335 closed that
//! bypass on `create_rule` / `create_from_template`; this file extends the
//! same fix to the rest of the handlers, which previously took NO auth
//! extractor at all and were reachable with zero credentials (audit gap
//! against PR #332 + #335).
//!
//! Tenant scoping is now enforced at the repository layer: per-rule
//! operations call `*_for_org(id, org_id)` variants so cross-tenant IDOR
//! is closed (mirrors `WorkflowRepository::find_by_id_for_org` from PR
//! #322's workflow-IDOR fix). 404 — not 403 — is returned on a miss so
//! callers cannot enumerate IDs by status code.

use api_core::extractors::principal::RequestPrincipal;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{
    CreateAutomationRule, CreateRuleFromTemplate, UpdateAutomationRule, WorkflowAutomationLog,
    WorkflowAutomationRule, WorkflowAutomationTemplate,
};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::state::AppState;

// ============================================================================
// Helper Functions
// ============================================================================
//
// SECURITY: The previous `extract_tenant_context` helper deserialized the
// client-supplied `X-Tenant-Context` JSON header directly into a
// `TenantContext`. No JWT verification — any unauthenticated caller could
// forge tenancy. That helper has been deleted; every handler now goes
// through `RequestPrincipal` (verified bearer JWT + host-resolved tenant).

type ApiResult<T> = Result<T, (StatusCode, Json<ErrorResponse>)>;

/// Pull the effective tenant id off a verified principal, or surface a 401.
///
/// Mirrors the pattern from `routes/board_meetings.rs::require_tenant`. A
/// non-platform principal is rejected by the extractor itself if there is
/// no resolved tenant, so this only fires for platform principals reaching
/// a tenant-scoped endpoint without a host-resolved tenant — that's still
/// the correct surface: "tenant context required".
fn require_tenant_id(principal: &RequestPrincipal) -> ApiResult<Uuid> {
    principal.effective_org.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "UNAUTHORIZED",
                "Organization context required",
            )),
        )
    })
}

/// Enforce that a `{org_id}` path parameter matches the caller's tenant.
///
/// Platform principals (which legitimately operate cross-tenant) are
/// allowed through. For everyone else, a mismatch is a 404 — same shape
/// as a not-found rule, so a caller cannot probe org existence by
/// comparing 404 vs 403 vs 400.
fn enforce_org_match(principal: &RequestPrincipal, path_org_id: Uuid) -> ApiResult<()> {
    if principal.is_platform() {
        return Ok(());
    }
    let caller_org = require_tenant_id(principal)?;
    if caller_org != path_org_id {
        return Err(not_found("Organization"));
    }
    Ok(())
}

fn not_found(resource: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new(
            "NOT_FOUND",
            format!("{} not found", resource),
        )),
    )
}

fn db_error(label: &'static str) -> impl Fn(sqlx::Error) -> (StatusCode, Json<ErrorResponse>) {
    move |e| {
        tracing::error!(error = %e, "{}", label);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", label)),
        )
    }
}

/// Create automation router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Rules (Story 38.1-38.3)
        .route("/organizations/{org_id}/rules", get(list_rules))
        .route("/organizations/{org_id}/rules", post(create_rule))
        .route("/rules/{id}", get(get_rule))
        .route("/rules/{id}", put(update_rule))
        .route("/rules/{id}", delete(delete_rule))
        .route("/rules/{id}/toggle", post(toggle_rule))
        .route("/rules/{id}/logs", get(get_rule_logs))
        // Templates
        .route("/templates", get(list_templates))
        .route("/templates/{id}", get(get_template))
        .route(
            "/organizations/{org_id}/rules/from-template",
            post(create_from_template),
        )
}

// ==================== Types ====================

/// Organization ID path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct OrgIdPath {
    pub org_id: Uuid,
}

/// Rule ID path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct RuleIdPath {
    pub id: Uuid,
}

/// Template ID path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct TemplateIdPath {
    pub id: Uuid,
}

/// Rule logs query.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct RuleLogsQuery {
    #[serde(default = "default_limit")]
    pub limit: i32,
}

fn default_limit() -> i32 {
    50
}

/// Toggle rule request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ToggleRuleRequest {
    pub is_active: bool,
}

// ==================== Rules (Story 38.1-38.3) ====================

/// List automation rules for an organization.
#[utoipa::path(
    get,
    path = "/api/v1/automation/organizations/{org_id}/rules",
    params(OrgIdPath),
    responses(
        (status = 200, description = "Rules retrieved", body = Vec<WorkflowAutomationRule>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Organization not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Automation"
)]
pub async fn list_rules(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<OrgIdPath>,
) -> ApiResult<Json<Vec<WorkflowAutomationRule>>> {
    enforce_org_match(&principal, path.org_id)?;

    let rules = state
        .automation_repo
        .list_rules(path.org_id)
        .await
        .map_err(db_error("Failed to list rules"))?;

    Ok(Json(rules))
}

/// Create an automation rule.
#[utoipa::path(
    post,
    path = "/api/v1/automation/organizations/{org_id}/rules",
    params(OrgIdPath),
    request_body = CreateAutomationRule,
    responses(
        (status = 201, description = "Rule created", body = WorkflowAutomationRule),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Organization not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Automation"
)]
pub async fn create_rule(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<OrgIdPath>,
    Json(data): Json<CreateAutomationRule>,
) -> ApiResult<(StatusCode, Json<WorkflowAutomationRule>)> {
    enforce_org_match(&principal, path.org_id)?;

    let rule = state
        .automation_repo
        .create_rule(path.org_id, principal.user_id, data)
        .await
        .map_err(db_error("Failed to create rule"))?;

    Ok((StatusCode::CREATED, Json(rule)))
}

/// Get an automation rule by ID.
#[utoipa::path(
    get,
    path = "/api/v1/automation/rules/{id}",
    params(RuleIdPath),
    responses(
        (status = 200, description = "Rule retrieved", body = WorkflowAutomationRule),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Rule not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Automation"
)]
pub async fn get_rule(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<RuleIdPath>,
) -> ApiResult<Json<WorkflowAutomationRule>> {
    let org_id = require_tenant_id(&principal)?;

    let rule = state
        .automation_repo
        .find_rule_for_org(path.id, org_id)
        .await
        .map_err(db_error("Failed to get rule"))?;

    match rule {
        Some(r) => Ok(Json(r)),
        // 404 — not 403 — on cross-tenant access so a caller cannot
        // enumerate rule IDs by comparing not-found vs forbidden.
        None => Err(not_found("Rule")),
    }
}

/// Update an automation rule.
#[utoipa::path(
    put,
    path = "/api/v1/automation/rules/{id}",
    params(RuleIdPath),
    request_body = UpdateAutomationRule,
    responses(
        (status = 200, description = "Rule updated", body = WorkflowAutomationRule),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Rule not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Automation"
)]
pub async fn update_rule(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<RuleIdPath>,
    Json(data): Json<UpdateAutomationRule>,
) -> ApiResult<Json<WorkflowAutomationRule>> {
    let org_id = require_tenant_id(&principal)?;

    let rule = state
        .automation_repo
        .update_rule(path.id, org_id, data)
        .await
        .map_err(db_error("Failed to update rule"))?;

    match rule {
        Some(r) => Ok(Json(r)),
        None => Err(not_found("Rule")),
    }
}

/// Delete an automation rule.
#[utoipa::path(
    delete,
    path = "/api/v1/automation/rules/{id}",
    params(RuleIdPath),
    responses(
        (status = 204, description = "Rule deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Rule not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Automation"
)]
pub async fn delete_rule(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<RuleIdPath>,
) -> ApiResult<StatusCode> {
    let org_id = require_tenant_id(&principal)?;

    let deleted = state
        .automation_repo
        .delete_rule(path.id, org_id)
        .await
        .map_err(db_error("Failed to delete rule"))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found("Rule"))
    }
}

/// Toggle rule active status.
#[utoipa::path(
    post,
    path = "/api/v1/automation/rules/{id}/toggle",
    params(RuleIdPath),
    request_body = ToggleRuleRequest,
    responses(
        (status = 200, description = "Rule toggled"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Rule not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Automation"
)]
pub async fn toggle_rule(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<RuleIdPath>,
    Json(data): Json<ToggleRuleRequest>,
) -> ApiResult<StatusCode> {
    let org_id = require_tenant_id(&principal)?;

    let toggled = state
        .automation_repo
        .toggle_rule(path.id, org_id, data.is_active)
        .await
        .map_err(db_error("Failed to toggle rule"))?;

    if toggled {
        Ok(StatusCode::OK)
    } else {
        Err(not_found("Rule"))
    }
}

/// Get execution logs for a rule.
#[utoipa::path(
    get,
    path = "/api/v1/automation/rules/{id}/logs",
    params(RuleIdPath, RuleLogsQuery),
    responses(
        (status = 200, description = "Logs retrieved", body = Vec<WorkflowAutomationLog>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Rule not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Automation"
)]
pub async fn get_rule_logs(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<RuleIdPath>,
    Query(query): Query<RuleLogsQuery>,
) -> ApiResult<Json<Vec<WorkflowAutomationLog>>> {
    let org_id = require_tenant_id(&principal)?;

    let logs = state
        .automation_repo
        .get_rule_logs_for_org(path.id, org_id, query.limit)
        .await
        .map_err(db_error("Failed to get rule logs"))?;

    match logs {
        Some(l) => Ok(Json(l)),
        None => Err(not_found("Rule")),
    }
}

// ==================== Templates ====================

/// List all automation templates.
///
/// Templates are global (system-supplied or shared across tenants) but the
/// endpoint still requires authentication so anonymous callers cannot
/// enumerate the available trigger surface of the platform.
#[utoipa::path(
    get,
    path = "/api/v1/automation/templates",
    responses(
        (status = 200, description = "Templates retrieved", body = Vec<WorkflowAutomationTemplate>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Automation"
)]
pub async fn list_templates(
    State(state): State<AppState>,
    _principal: RequestPrincipal,
) -> ApiResult<Json<Vec<WorkflowAutomationTemplate>>> {
    let templates = state
        .automation_repo
        .list_templates()
        .await
        .map_err(db_error("Failed to list templates"))?;

    Ok(Json(templates))
}

/// Get an automation template by ID.
///
/// Templates are global — auth-gated but not tenant-scoped (see
/// `list_templates`).
#[utoipa::path(
    get,
    path = "/api/v1/automation/templates/{id}",
    params(TemplateIdPath),
    responses(
        (status = 200, description = "Template retrieved", body = WorkflowAutomationTemplate),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Template not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Automation"
)]
pub async fn get_template(
    State(state): State<AppState>,
    _principal: RequestPrincipal,
    Path(path): Path<TemplateIdPath>,
) -> ApiResult<Json<WorkflowAutomationTemplate>> {
    let template = state
        .automation_repo
        .get_template(path.id)
        .await
        .map_err(db_error("Failed to get template"))?;

    match template {
        Some(t) => Ok(Json(t)),
        None => Err(not_found("Template")),
    }
}

/// Create a rule from a template.
#[utoipa::path(
    post,
    path = "/api/v1/automation/organizations/{org_id}/rules/from-template",
    params(OrgIdPath),
    request_body = CreateRuleFromTemplate,
    responses(
        (status = 201, description = "Rule created from template", body = WorkflowAutomationRule),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Template not found or organization not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Automation"
)]
pub async fn create_from_template(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<OrgIdPath>,
    Json(data): Json<CreateRuleFromTemplate>,
) -> ApiResult<(StatusCode, Json<WorkflowAutomationRule>)> {
    enforce_org_match(&principal, path.org_id)?;

    let rule = state
        .automation_repo
        .create_from_template(path.org_id, principal.user_id, data)
        .await
        .map_err(db_error("Failed to create rule from template"))?;

    Ok((StatusCode::CREATED, Json(rule)))
}
