//! Workflow Automation routes (Epic 38).
//!
//! Routes for automation rules, templates, and execution logs.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post, put},
    Json, Router,
};
use common::{errors::ErrorResponse, TenantContext};
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

/// Extract tenant context from request headers.
fn extract_tenant_context(
    headers: &HeaderMap,
) -> Result<TenantContext, (StatusCode, Json<ErrorResponse>)> {
    let tenant_header = headers
        .get("X-Tenant-Context")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "MISSING_CONTEXT",
                    "Authentication required",
                )),
            )
        })?;

    serde_json::from_str(tenant_header).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_CONTEXT",
                "Invalid authentication context format",
            )),
        )
    })
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
            "/organizations/:org_id/rules/from-template",
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
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Automation"
)]
pub async fn list_rules(
    State(state): State<AppState>,
    Path(path): Path<OrgIdPath>,
) -> Result<Json<Vec<WorkflowAutomationRule>>, (StatusCode, Json<ErrorResponse>)> {
    let rules = state
        .automation_repo
        .list_rules(path.org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list rules");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to list rules")),
            )
        })?;

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
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Automation"
)]
pub async fn create_rule(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(path): Path<OrgIdPath>,
    Json(data): Json<CreateAutomationRule>,
) -> Result<(StatusCode, Json<WorkflowAutomationRule>), (StatusCode, Json<ErrorResponse>)> {
    let context = extract_tenant_context(&headers)?;

    let rule = state
        .automation_repo
        .create_rule(path.org_id, context.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create rule");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to create rule",
                )),
            )
        })?;

    Ok((StatusCode::CREATED, Json(rule)))
}

/// Get an automation rule by ID.
#[utoipa::path(
    get,
    path = "/api/v1/automation/rules/{id}",
    params(RuleIdPath),
    responses(
        (status = 200, description = "Rule retrieved", body = WorkflowAutomationRule),
        (status = 404, description = "Rule not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Automation"
)]
pub async fn get_rule(
    State(state): State<AppState>,
    Path(path): Path<RuleIdPath>,
) -> Result<Json<WorkflowAutomationRule>, (StatusCode, Json<ErrorResponse>)> {
    let rule = state.automation_repo.get_rule(path.id).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get rule");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", "Failed to get rule")),
        )
    })?;

    match rule {
        Some(r) => Ok(Json(r)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Rule not found")),
        )),
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
        (status = 404, description = "Rule not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Automation"
)]
pub async fn update_rule(
    State(state): State<AppState>,
    Path(path): Path<RuleIdPath>,
    Json(data): Json<UpdateAutomationRule>,
) -> Result<Json<WorkflowAutomationRule>, (StatusCode, Json<ErrorResponse>)> {
    let rule = state
        .automation_repo
        .update_rule(path.id, data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update rule");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to update rule",
                )),
            )
        })?;

    Ok(Json(rule))
}

/// Delete an automation rule.
#[utoipa::path(
    delete,
    path = "/api/v1/automation/rules/{id}",
    params(RuleIdPath),
    responses(
        (status = 204, description = "Rule deleted"),
        (status = 404, description = "Rule not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Automation"
)]
pub async fn delete_rule(
    State(state): State<AppState>,
    Path(path): Path<RuleIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state
        .automation_repo
        .delete_rule(path.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to delete rule");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to delete rule",
                )),
            )
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Rule not found")),
        ))
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
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Automation"
)]
pub async fn toggle_rule(
    State(state): State<AppState>,
    Path(path): Path<RuleIdPath>,
    Json(data): Json<ToggleRuleRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    state
        .automation_repo
        .toggle_rule(path.id, data.is_active)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to toggle rule");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to toggle rule",
                )),
            )
        })?;

    Ok(StatusCode::OK)
}

/// Get execution logs for a rule.
#[utoipa::path(
    get,
    path = "/api/v1/automation/rules/{id}/logs",
    params(RuleIdPath, RuleLogsQuery),
    responses(
        (status = 200, description = "Logs retrieved", body = Vec<WorkflowAutomationLog>),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Automation"
)]
pub async fn get_rule_logs(
    State(state): State<AppState>,
    Path(path): Path<RuleIdPath>,
    Query(query): Query<RuleLogsQuery>,
) -> Result<Json<Vec<WorkflowAutomationLog>>, (StatusCode, Json<ErrorResponse>)> {
    let logs = state
        .automation_repo
        .get_rule_logs(path.id, query.limit)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get rule logs");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get rule logs",
                )),
            )
        })?;

    Ok(Json(logs))
}

// ==================== Templates ====================

/// List all automation templates.
#[utoipa::path(
    get,
    path = "/api/v1/automation/templates",
    responses(
        (status = 200, description = "Templates retrieved", body = Vec<WorkflowAutomationTemplate>),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Automation"
)]
pub async fn list_templates(
    State(state): State<AppState>,
) -> Result<Json<Vec<WorkflowAutomationTemplate>>, (StatusCode, Json<ErrorResponse>)> {
    let templates = state.automation_repo.list_templates().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to list templates");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to list templates",
            )),
        )
    })?;

    Ok(Json(templates))
}

/// Get an automation template by ID.
#[utoipa::path(
    get,
    path = "/api/v1/automation/templates/{id}",
    params(TemplateIdPath),
    responses(
        (status = 200, description = "Template retrieved", body = WorkflowAutomationTemplate),
        (status = 404, description = "Template not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Automation"
)]
pub async fn get_template(
    State(state): State<AppState>,
    Path(path): Path<TemplateIdPath>,
) -> Result<Json<WorkflowAutomationTemplate>, (StatusCode, Json<ErrorResponse>)> {
    let template = state
        .automation_repo
        .get_template(path.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get template");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get template",
                )),
            )
        })?;

    match template {
        Some(t) => Ok(Json(t)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Template not found")),
        )),
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
        (status = 404, description = "Template not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Automation"
)]
pub async fn create_from_template(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(path): Path<OrgIdPath>,
    Json(data): Json<CreateRuleFromTemplate>,
) -> Result<(StatusCode, Json<WorkflowAutomationRule>), (StatusCode, Json<ErrorResponse>)> {
    let context = extract_tenant_context(&headers)?;

    let rule = state
        .automation_repo
        .create_from_template(path.org_id, context.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create rule from template");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to create rule from template",
                )),
            )
        })?;

    Ok((StatusCode::CREATED, Json(rule)))
}
