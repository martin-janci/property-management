//! Workflow automation (Story 13.6 & 13.7), event handling (Story 94.2),
//! and the templates marketplace (Story 94.4).
//!
//! # RLS (PAP-77)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` on the workflow tables,
//! so every query MUST run on a connection that has `app.current_org_id` set
//! or it collapses to deny-all. Each database handler therefore acquires an
//! [`RlsConnection`] (which validates tenant membership and sets the org/user
//! GUCs on a dedicated connection) and passes `&mut **rls.conn()` to the
//! repository. The authoritative organization is `rls.tenant_id()` — the
//! tenant the caller was validated against — never a client-supplied value,
//! so the SQL org filter and the RLS context can never disagree. By-id repo
//! queries stay keyed on `(id, organization_id)` (child tables via the parent
//! workflow) as defense-in-depth, so a cross-tenant probe resolves to no row
//! → `404` even on an RLS-exempt pool. `rls.release()` clears the context
//! before the connection returns to the pool. The background
//! `WorkflowExecutor` acquires its own org-context connections from an
//! `RlsPool` (org derived from the org-scoped-loaded workflow row).
//!
//! Template marketplace endpoints that serve only built-in (non-database)
//! templates keep the lighter `RequestPrincipal` auth gate.

use crate::routes::ai::not_found;
use crate::routes::pagination::clamp_limit;
use crate::state::AppState;
use api_core::extractors::principal::RequestPrincipal;
use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{
    get_builtin_templates, template_category, CreateWorkflow, CreateWorkflowAction, ExecutionQuery,
    ImportTemplateRequest, TriggerWorkflow, UpdateWorkflow, WorkflowQuery,
};
use db::RlsPool;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn internal_error(e: impl std::fmt::Display) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new("INTERNAL_ERROR", e.to_string())),
    )
}

// ============================================================================
// Workflow Router (Story 13.6 & 13.7)
// ============================================================================

pub fn workflow_router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_workflow))
        .route("/", get(list_workflows))
        .route("/{id}", get(get_workflow))
        .route("/{id}", put(update_workflow))
        .route("/{id}", delete(delete_workflow))
        .route("/{id}/actions", get(list_actions))
        .route("/{id}/actions", post(add_action))
        .route("/actions/{action_id}", delete(delete_action))
        .route("/{id}/trigger", post(trigger_workflow))
        .route("/executions", get(list_executions))
        .route("/executions/{id}", get(get_execution))
        .route("/executions/{id}/steps", get(list_execution_steps))
        // Story 94.2: Event handling endpoint
        .route("/events", post(handle_workflow_event))
        // Story 94.4: Templates marketplace
        .route("/templates", get(list_workflow_templates))
        .route("/templates/builtin", get(list_builtin_templates))
        .route("/templates/{id}", get(get_workflow_template))
        .route("/templates/{id}/import", post(import_workflow_template))
}

async fn create_workflow(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateWorkflow>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    // SECURITY: owning org and creator are derived from the validated tenant
    // context, never from the request body.
    let organization_id = rls.tenant_id();
    let user_id = rls.user_id();
    let result = state
        .workflow_repo
        .create(&mut **rls.conn(), organization_id, user_id, req)
        .await;
    rls.release().await;
    match result {
        Ok(workflow) => Ok((StatusCode::CREATED, Json(serde_json::json!(workflow)))),
        Err(e) => {
            tracing::error!("Failed to create workflow: {}", e);
            Err(internal_error("Failed to create"))
        }
    }
}

async fn list_workflows(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<WorkflowQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = rls.tenant_id();
    let result = state
        .workflow_repo
        .list(&mut **rls.conn(), tenant_id, query)
        .await;
    rls.release().await;
    match result {
        Ok(workflows) => Ok(Json(serde_json::json!({ "workflows": workflows }))),
        Err(e) => {
            tracing::error!("Failed to list workflows: {}", e);
            Err(internal_error("Failed to list"))
        }
    }
}

async fn get_workflow(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let workflow = state
        .workflow_repo
        .find_by_id_for_org(&mut **rls.conn(), id, org_id)
        .await;
    let result = match workflow {
        Ok(Some(workflow)) => {
            // list_actions is scoped by the same org_id, so we cannot leak
            // actions from a workflow we just verified, but pass org_id
            // anyway for symmetry / future-proofing against ID swaps.
            let actions = state
                .workflow_repo
                .list_actions(rls.conn(), id, org_id)
                .await;
            match actions {
                Ok(actions) => Ok(Json(serde_json::json!({
                    "workflow": workflow,
                    "actions": actions.unwrap_or_default()
                }))),
                Err(e) => {
                    tracing::error!("Failed to list actions: {}", e);
                    Err(internal_error("Failed to get"))
                }
            }
        }
        Ok(None) => Err(not_found("Workflow not found")),
        Err(e) => {
            tracing::error!("Failed to get workflow: {}", e);
            Err(internal_error("Failed to get"))
        }
    };
    rls.release().await;
    result
}

async fn update_workflow(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateWorkflow>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let result = state
        .workflow_repo
        .update(&mut **rls.conn(), id, org_id, req)
        .await;
    rls.release().await;
    match result {
        Ok(Some(workflow)) => Ok(Json(serde_json::json!(workflow))),
        Ok(None) => Err(not_found("Workflow not found")),
        Err(e) => {
            tracing::error!("Failed to update workflow: {}", e);
            Err(internal_error("Failed to update"))
        }
    }
}

async fn delete_workflow(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let result = state
        .workflow_repo
        .delete(&mut **rls.conn(), id, org_id)
        .await;
    rls.release().await;
    match result {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err(not_found("Workflow not found")),
        Err(e) => {
            tracing::error!("Failed to delete workflow: {}", e);
            Err(internal_error("Failed to delete"))
        }
    }
}

async fn list_actions(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let result = state
        .workflow_repo
        .list_actions(rls.conn(), id, org_id)
        .await;
    rls.release().await;
    match result {
        Ok(Some(actions)) => Ok(Json(serde_json::json!({ "actions": actions }))),
        Ok(None) => Err(not_found("Workflow not found")),
        Err(e) => {
            tracing::error!("Failed to list actions: {}", e);
            Err(internal_error("Failed to list"))
        }
    }
}

async fn add_action(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(mut req): Json<CreateWorkflowAction>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    // Pin the workflow_id to the path parameter so a caller can't
    // shadow-target a different workflow via the JSON body.
    req.workflow_id = id;
    let result = state
        .workflow_repo
        .add_action(rls.conn(), org_id, req)
        .await;
    rls.release().await;
    match result {
        Ok(Some(action)) => Ok((StatusCode::CREATED, Json(serde_json::json!(action)))),
        Ok(None) => Err(not_found("Workflow not found")),
        Err(e) => {
            tracing::error!("Failed to add action: {}", e);
            Err(internal_error("Failed to add"))
        }
    }
}

async fn delete_action(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(action_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let result = state
        .workflow_repo
        .delete_action(&mut **rls.conn(), action_id, org_id)
        .await;
    rls.release().await;
    match result {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err(not_found("Action not found")),
        Err(e) => {
            tracing::error!("Failed to delete action: {}", e);
            Err(internal_error("Failed to delete"))
        }
    }
}

async fn trigger_workflow(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<TriggerWorkflow>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    use crate::services::WorkflowExecutor;

    let org_id = rls.tenant_id();

    // SECURITY (H1): load the workflow org-scoped BEFORE handing off to the
    // executor. A 404 (not 403) is the correct response — distinguishing "no
    // such workflow" from "wrong tenant" would let a caller enumerate
    // workflow IDs cross-tenant. The loaded row is what the executor runs:
    // it never re-fetches by bare ID.
    let workflow = state
        .workflow_repo
        .find_by_id_for_org(&mut **rls.conn(), id, org_id)
        .await;
    rls.release().await;
    let workflow = match workflow {
        Ok(Some(workflow)) => workflow,
        Ok(None) => return Err(not_found("Workflow not found")),
        Err(e) => {
            tracing::error!("Failed to load workflow for tenant check: {}", e);
            return Err(internal_error("Failed to trigger"));
        }
    };

    // Create the workflow executor; it acquires its own RLS-context
    // connections (org derived from the row loaded above).
    let executor =
        WorkflowExecutor::new(RlsPool::new(state.db.clone()), state.workflow_repo.clone());

    // Execute the workflow asynchronously (Epic 94)
    match executor
        .execute_workflow(workflow, req.trigger_event, req.context)
        .await
    {
        Ok(execution_id) => {
            tracing::info!(
                workflow_id = %id,
                execution_id = %execution_id,
                "Workflow triggered successfully"
            );
            Ok((
                StatusCode::CREATED,
                Json(serde_json::json!({
                    "execution_id": execution_id,
                    "workflow_id": id,
                    "status": "running"
                })),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to trigger workflow: {}", e);
            Err(internal_error(e))
        }
    }
}

async fn list_executions(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ExecutionQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = rls.tenant_id();
    let result = state
        .workflow_repo
        .list_executions(&mut **rls.conn(), tenant_id, query)
        .await;
    rls.release().await;
    match result {
        Ok(executions) => Ok(Json(serde_json::json!({ "executions": executions }))),
        Err(e) => {
            tracing::error!("Failed to list executions: {}", e);
            Err(internal_error("Failed to list"))
        }
    }
}

async fn get_execution(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let execution = state
        .workflow_repo
        .find_execution_by_id_for_org(&mut **rls.conn(), id, org_id)
        .await;
    let result = match execution {
        Ok(Some(execution)) => {
            let steps = state
                .workflow_repo
                .list_execution_steps_for_org(rls.conn(), id, org_id)
                .await;
            match steps {
                Ok(steps) => Ok(Json(serde_json::json!({
                    "execution": execution,
                    "steps": steps.unwrap_or_default()
                }))),
                Err(e) => {
                    tracing::error!("Failed to list steps: {}", e);
                    Err(internal_error("Failed to get"))
                }
            }
        }
        Ok(None) => Err(not_found("Execution not found")),
        Err(e) => {
            tracing::error!("Failed to get execution: {}", e);
            Err(internal_error("Failed to get"))
        }
    };
    rls.release().await;
    result
}

async fn list_execution_steps(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let result = state
        .workflow_repo
        .list_execution_steps_for_org(rls.conn(), id, org_id)
        .await;
    rls.release().await;
    match result {
        Ok(Some(steps)) => Ok(Json(serde_json::json!({ "steps": steps }))),
        Ok(None) => Err(not_found("Execution not found")),
        Err(e) => {
            tracing::error!("Failed to list steps: {}", e);
            Err(internal_error("Failed to list"))
        }
    }
}

// ============================================================================
// Story 94.2: Workflow Event Handling
// ============================================================================

/// Request to trigger workflows by event.
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowEventRequest {
    /// Event type (matches trigger_type constants)
    pub event_type: String,
    /// Event data/payload
    pub data: serde_json::Value,
    /// Optional building ID
    pub building_id: Option<Uuid>,
}

/// Handle workflow trigger events (Story 94.2).
async fn handle_workflow_event(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<WorkflowEventRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    use crate::services::{WorkflowEvent, WorkflowExecutor};

    // The RlsConnection validated tenant membership; the executor acquires
    // its own org-context connections, so release this one immediately.
    let tenant_id = rls.tenant_id();
    let user_id = rls.user_id();
    rls.release().await;

    // Create the workflow executor
    let executor =
        WorkflowExecutor::new(RlsPool::new(state.db.clone()), state.workflow_repo.clone());

    // Create the event
    let mut event = WorkflowEvent::new(&req.event_type, tenant_id, req.data);
    if let Some(building_id) = req.building_id {
        event = event.with_building(building_id);
    }
    // Set the user who triggered the event
    event = event.with_user(user_id);

    // Handle the event - this finds matching workflows and executes them
    match executor.handle_event(event).await {
        Ok(execution_ids) => {
            tracing::info!(
                event_type = %req.event_type,
                triggered_count = execution_ids.len(),
                "Workflow event processed"
            );
            Ok((
                StatusCode::OK,
                Json(serde_json::json!({
                    "triggered_workflows": execution_ids.len(),
                    "execution_ids": execution_ids
                })),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to handle workflow event: {}", e);
            Err(internal_error(e))
        }
    }
}

// ============================================================================
// Story 94.4: Workflow Templates Marketplace
// ============================================================================

/// Query parameters for template search.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TemplateQuery {
    pub category: Option<String>,
    pub trigger_type: Option<String>,
    pub search: Option<String>,
    pub featured: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// List available workflow templates.
async fn list_workflow_templates(
    _principal: RequestPrincipal,
    Query(query): Query<TemplateQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // For now, return built-in templates
    // In production, this would query from workflow_templates table
    let builtin = get_builtin_templates();

    let templates: Vec<serde_json::Value> = builtin
        .iter()
        .enumerate()
        .filter(|(_, (template, _))| {
            // Apply category filter
            if let Some(ref cat) = query.category {
                if template.category != *cat {
                    return false;
                }
            }
            // Apply trigger type filter
            if let Some(ref trigger) = query.trigger_type {
                if template.trigger_type != *trigger {
                    return false;
                }
            }
            // Apply search filter
            if let Some(ref search) = query.search {
                let search_lower = search.to_lowercase();
                if !template.name.to_lowercase().contains(&search_lower)
                    && !template
                        .description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&search_lower))
                        .unwrap_or(false)
                {
                    return false;
                }
            }
            true
        })
        .map(|(idx, (template, actions))| {
            serde_json::json!({
                "id": format!("builtin-{}", idx),
                "name": template.name,
                "description": template.description,
                "category": template.category,
                "trigger_type": template.trigger_type,
                "scope": template.scope,
                "tags": template.tags,
                "icon": template.icon,
                "action_count": actions.len(),
                "featured": false,
                "use_count": 0
            })
        })
        .collect();

    // Apply pagination (clamp before the `as usize` cast so a hostile
    // negative/huge limit can't wrap into an unbounded `take`).
    let limit = clamp_limit(query.limit, 50) as usize;
    let offset = query.offset.unwrap_or(0).max(0) as usize;
    let paginated: Vec<_> = templates.into_iter().skip(offset).take(limit).collect();

    Ok(Json(serde_json::json!({
        "templates": paginated,
        "categories": template_category::ALL
    })))
}

/// Get a specific workflow template.
async fn get_workflow_template(
    _principal: RequestPrincipal,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Handle built-in templates
    if id.starts_with("builtin-") {
        let idx: usize = id
            .strip_prefix("builtin-")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new("INVALID_ID", "Invalid template ID")),
                )
            })?;

        let builtin = get_builtin_templates();
        if let Some((template, actions)) = builtin.get(idx) {
            return Ok(Json(serde_json::json!({
                "id": id,
                "template": {
                    "name": template.name,
                    "description": template.description,
                    "category": template.category,
                    "trigger_type": template.trigger_type,
                    "trigger_config": template.trigger_config,
                    "conditions": template.conditions,
                    "scope": template.scope,
                    "tags": template.tags,
                    "icon": template.icon
                },
                "actions": actions.iter().map(|a| serde_json::json!({
                    "action_order": a.action_order,
                    "action_type": a.action_type,
                    "action_config": a.action_config,
                    "description": a.description,
                    "on_failure": a.on_failure,
                    "retry_count": a.retry_count,
                    "retry_delay_seconds": a.retry_delay_seconds
                })).collect::<Vec<_>>(),
                "variables": []
            })));
        }
    }

    // Template not found
    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", "Template not found")),
    ))
}

/// Import a workflow template.
async fn import_workflow_template(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<String>,
    Json(req): Json<ImportTemplateRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = rls.tenant_id();
    let user_id = rls.user_id();

    // Get the template
    let (template, actions) = if id.starts_with("builtin-") {
        let idx: usize = id
            .strip_prefix("builtin-")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new("INVALID_ID", "Invalid template ID")),
                )
            })?;

        let builtin = get_builtin_templates();
        builtin.get(idx).cloned().ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Template not found")),
            )
        })?
    } else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Template not found")),
        ));
    };

    // Create the workflow from template
    let workflow_name = req
        .name
        .unwrap_or_else(|| format!("{} (from template)", template.name));

    let create_workflow = CreateWorkflow {
        name: workflow_name.clone(),
        description: template.description,
        trigger_type: template.trigger_type,
        trigger_config: template.trigger_config,
        conditions: template.conditions,
    };

    // Create the workflow. Org and creator come from the validated tenant
    // context.
    let created = state
        .workflow_repo
        .create(&mut **rls.conn(), tenant_id, user_id, create_workflow)
        .await;
    let workflow = match created {
        Ok(workflow) => workflow,
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to create workflow from template: {}", e);
            return Err(internal_error("Failed to import template"));
        }
    };

    // Add actions
    for action in actions {
        let create_action = CreateWorkflowAction {
            workflow_id: workflow.id,
            action_order: action.action_order,
            action_type: action.action_type,
            action_config: action.action_config,
            on_failure: action.on_failure,
            retry_count: action.retry_count,
            retry_delay_seconds: action.retry_delay_seconds,
        };

        // The workflow we just created belongs to `tenant_id` by
        // construction, so this is the correct org scope.
        if let Err(e) = state
            .workflow_repo
            .add_action(rls.conn(), tenant_id, create_action)
            .await
        {
            tracing::warn!("Failed to add action to imported workflow: {}", e);
        }
    }

    // Enable if requested
    if req.enabled.unwrap_or(false) {
        let _ = state
            .workflow_repo
            .update(
                &mut **rls.conn(),
                workflow.id,
                tenant_id,
                UpdateWorkflow {
                    name: None,
                    description: None,
                    trigger_type: None,
                    trigger_config: None,
                    conditions: None,
                    enabled: Some(true),
                },
            )
            .await;
    }

    rls.release().await;

    tracing::info!(
        workflow_id = %workflow.id,
        template_id = %id,
        workflow_name = %workflow_name,
        "Workflow imported from template"
    );

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "workflow_id": workflow.id,
            "name": workflow_name,
            "enabled": req.enabled.unwrap_or(false),
            "imported_from": id
        })),
    ))
}

/// List all built-in templates.
async fn list_builtin_templates(
    _principal: RequestPrincipal,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let builtin = get_builtin_templates();

    let templates: Vec<serde_json::Value> = builtin
        .iter()
        .enumerate()
        .map(|(idx, (template, actions))| {
            serde_json::json!({
                "id": format!("builtin-{}", idx),
                "name": template.name,
                "description": template.description,
                "category": template.category,
                "trigger_type": template.trigger_type,
                "tags": template.tags,
                "icon": template.icon,
                "action_count": actions.len()
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "templates": templates,
        "categories": template_category::ALL
    })))
}
