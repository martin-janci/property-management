//! Workflow execution engine (Epic 94).
//!
//! Executes workflows asynchronously, handling:
//! - Action execution with retry logic (Story 94.1)
//! - Trigger event matching (Story 94.2)
//! - Conditional logic evaluation (Story 94.3)

use crate::services::actions::{ActionContext, ActionError, ActionRegistry, ActionResult};
use db::models::{
    execution_status, on_failure, step_status, trigger_type, TriggerWorkflow, Workflow,
    WorkflowAction,
};
use db::repositories::{WorkflowRepository, MAX_RETRY_COUNT, MAX_RETRY_DELAY_SECONDS};
use db::{RlsGuard, RlsPool};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::Instrument;
use uuid::Uuid;

/// Compute the exponential-backoff sleep for a retry attempt, clamped to a
/// safe ceiling.
///
/// Without a clamp, `2^(attempt-1) * base_delay` overflows or sleeps for
/// effectively forever — with `retry_delay_seconds = 86400` and
/// `attempt = 31`, the naive formula yielded ~2900 years (H4 audit). We:
///
/// 1. Saturate the exponent at 30 so `2_u64.pow(...)` cannot overflow.
/// 2. Use `saturating_mul` for the multiplication.
/// 3. Cap the final value at `MAX_RETRY_DELAY_SECONDS` (1 hour).
///
/// `attempt` is expected to be >= 1 (callers handle `attempt == 0` —
/// the first try has no delay). `attempt == 0` returns 0 defensively.
#[inline]
fn compute_backoff_seconds(base_delay_seconds: i32, attempt: i32) -> u64 {
    if attempt < 1 {
        return 0;
    }
    let base = base_delay_seconds.max(0) as u64;
    // Cap the exponent so 2^exp stays well within u64 (2^62 is plenty).
    let exp = ((attempt - 1) as u32).min(30);
    let multiplier = 1u64 << exp;
    let raw = base.saturating_mul(multiplier);
    raw.min(MAX_RETRY_DELAY_SECONDS as u64)
}

/// Errors that can occur during workflow execution.
#[derive(Debug, Error)]
pub enum WorkflowError {
    #[error("Workflow not found: {0}")]
    NotFound(Uuid),

    #[error("Workflow is disabled: {0}")]
    Disabled(Uuid),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Action execution failed: {0}")]
    ActionFailed(#[from] ActionError),

    #[error("Condition evaluation failed: {0}")]
    ConditionError(String),

    #[error("Workflow cancelled")]
    Cancelled,
}

/// Comparison operators for conditions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOperator {
    Eq,       // ==
    Ne,       // !=
    Gt,       // >
    Gte,      // >=
    Lt,       // <
    Lte,      // <=
    Contains, // string contains
    StartsWith,
    EndsWith,
    In,        // value in list
    NotIn,     // value not in list
    IsNull,    // value is null
    IsNotNull, // value is not null
    Matches,   // regex match
}

/// A single condition to evaluate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    /// Field path to evaluate (e.g., "trigger.amount", "action_1.status")
    pub field: String,
    /// Comparison operator
    pub operator: ComparisonOperator,
    /// Value to compare against (optional for IsNull/IsNotNull)
    pub value: Option<serde_json::Value>,
}

/// Logical operator for combining conditions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum LogicalOperator {
    #[default]
    And,
    Or,
    Not,
}

/// A group of conditions with a logical operator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionGroup {
    /// Logical operator to combine conditions
    #[serde(default)]
    pub operator: LogicalOperator,
    /// Individual conditions
    #[serde(default)]
    pub conditions: Vec<Condition>,
    /// Nested condition groups
    #[serde(default)]
    pub groups: Vec<ConditionGroup>,
}

/// Event that can trigger workflows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEvent {
    /// Event type (matches trigger_type constants)
    pub event_type: String,
    /// Organization ID
    pub organization_id: Uuid,
    /// Event data/payload
    pub data: serde_json::Value,
    /// Optional building ID
    pub building_id: Option<Uuid>,
    /// Optional user ID who triggered the event
    pub triggered_by: Option<Uuid>,
    /// Timestamp of the event
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl WorkflowEvent {
    /// Create a new workflow event.
    pub fn new(event_type: &str, organization_id: Uuid, data: serde_json::Value) -> Self {
        Self {
            event_type: event_type.to_string(),
            organization_id,
            data,
            building_id: None,
            triggered_by: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Set the building ID.
    pub fn with_building(mut self, building_id: Uuid) -> Self {
        self.building_id = Some(building_id);
        self
    }

    /// Set the user who triggered the event.
    pub fn with_user(mut self, user_id: Uuid) -> Self {
        self.triggered_by = Some(user_id);
        self
    }
}

/// Configuration for the workflow executor.
#[derive(Debug, Clone)]
pub struct WorkflowExecutorConfig {
    /// Maximum concurrent workflow executions
    pub max_concurrent_executions: usize,
    /// Default timeout for action execution
    pub default_action_timeout: Duration,
    /// Maximum retries for failed actions
    pub max_retries: i32,
    /// Base delay between retries (exponential backoff)
    pub retry_base_delay: Duration,
}

impl Default for WorkflowExecutorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_executions: 10,
            default_action_timeout: Duration::from_secs(30),
            max_retries: 3,
            retry_base_delay: Duration::from_secs(5),
        }
    }
}

/// The workflow executor service.
///
/// RLS (PAP-77): the executor runs outside an HTTP request, so it cannot use
/// the `RlsConnection` extractor. It instead holds an [`RlsPool`] and
/// acquires a context-set connection per database operation, with the org
/// derived from the **already-loaded workflow row** (or the verified event)
/// — never from client input. Guards are scoped tightly so no pooled
/// connection is held across action execution or retry sleeps.
pub struct WorkflowExecutor {
    db: RlsPool,
    workflow_repo: WorkflowRepository,
    action_registry: ActionRegistry,
    config: WorkflowExecutorConfig,
}

impl WorkflowExecutor {
    /// Create a new workflow executor.
    pub fn new(db: RlsPool, workflow_repo: WorkflowRepository) -> Self {
        Self::with_config(db, workflow_repo, WorkflowExecutorConfig::default())
    }

    /// Create a new workflow executor with custom configuration.
    pub fn with_config(
        db: RlsPool,
        workflow_repo: WorkflowRepository,
        config: WorkflowExecutorConfig,
    ) -> Self {
        Self {
            db,
            workflow_repo,
            action_registry: ActionRegistry::new(),
            config,
        }
    }

    /// Execute a workflow.
    ///
    /// Takes the **already-loaded** workflow row: callers (the trigger route,
    /// `handle_event`) have necessarily fetched it org-scoped, so no unscoped
    /// by-id read exists anymore. The RLS context for all execution writes is
    /// derived from `workflow.organization_id`.
    pub async fn execute_workflow(
        &self,
        workflow: Workflow,
        trigger_event: serde_json::Value,
        context: Option<serde_json::Value>,
    ) -> Result<Uuid, WorkflowError> {
        let workflow_id = workflow.id;

        if !workflow.enabled {
            return Err(WorkflowError::Disabled(workflow_id));
        }

        // Create execution record on an org-context connection.
        let mut guard = self
            .db
            .acquire_with_rls(workflow.organization_id, workflow.created_by, false)
            .await?;
        let execution = self
            .workflow_repo
            .create_execution(
                guard.conn(),
                TriggerWorkflow {
                    workflow_id,
                    trigger_event: trigger_event.clone(),
                    context: context.clone(),
                },
            )
            .await;
        guard.release().await;
        let execution = execution?;

        let execution_id = execution.id;

        // Spawn async execution
        let executor = WorkflowExecutorTask {
            db: self.db.clone(),
            org_id: workflow.organization_id,
            acting_user: workflow.created_by,
            workflow_repo: self.workflow_repo.clone(),
            action_registry: ActionRegistry::new(),
            config: self.config.clone(),
        };

        tokio::spawn(
            async move {
                if let Err(e) = executor
                    .run(workflow, execution_id, trigger_event, context)
                    .await
                {
                    tracing::error!(
                        workflow_id = %workflow_id,
                        execution_id = %execution_id,
                        error = %e,
                        "Workflow execution failed"
                    );
                }
            }
            .instrument(tracing::info_span!(
                "bg.workflow_step",
                workflow_id = %workflow_id,
                execution_id = %execution_id,
            )),
        );

        Ok(execution_id)
    }

    /// Handle an event and trigger matching workflows.
    pub async fn handle_event(&self, event: WorkflowEvent) -> Result<Vec<Uuid>, WorkflowError> {
        tracing::info!(
            event_type = %event.event_type,
            organization_id = %event.organization_id,
            "Processing workflow trigger event"
        );

        // Find matching workflows on an org-context connection. The org is
        // the verified event org (set by the route from the request
        // principal, never client input). `triggered_by` is always set on the
        // route path; nil only pads the user GUC, which the workflow
        // policies don't consult.
        let mut guard = self
            .db
            .acquire_with_rls(
                event.organization_id,
                event.triggered_by.unwrap_or(Uuid::nil()),
                false,
            )
            .await?;
        let workflows = self
            .workflow_repo
            .list_by_trigger_type(
                &mut **guard.conn(),
                event.organization_id,
                &event.event_type,
            )
            .await;
        guard.release().await;
        let workflows = workflows?;

        let mut execution_ids = Vec::new();

        for workflow in workflows {
            // Check if workflow conditions are met
            let conditions = workflow.conditions.0.clone();
            if !conditions.is_empty() {
                let context = ActionContext::new(
                    event.organization_id,
                    workflow.id,
                    Uuid::new_v4(), // Temporary, will be replaced with actual execution ID
                    event.data.clone(),
                );

                if !self.evaluate_conditions(&conditions, &context)? {
                    tracing::debug!(
                        workflow_id = %workflow.id,
                        "Workflow conditions not met, skipping"
                    );
                    continue;
                }
            }

            // Check trigger config filters
            if !self.matches_trigger_config(&workflow, &event) {
                tracing::debug!(
                    workflow_id = %workflow.id,
                    "Workflow trigger config not matched, skipping"
                );
                continue;
            }

            // Execute the workflow (moves the loaded row into the executor).
            let workflow_id = workflow.id;
            match self
                .execute_workflow(
                    workflow,
                    event.data.clone(),
                    Some(serde_json::json!({
                        "event_type": event.event_type,
                        "building_id": event.building_id,
                        "triggered_by": event.triggered_by,
                        "timestamp": event.timestamp.to_rfc3339()
                    })),
                )
                .await
            {
                Ok(execution_id) => {
                    execution_ids.push(execution_id);
                }
                Err(e) => {
                    tracing::warn!(
                        workflow_id = %workflow_id,
                        error = %e,
                        "Failed to start workflow execution"
                    );
                }
            }
        }

        tracing::info!(
            event_type = %event.event_type,
            triggered_workflows = execution_ids.len(),
            "Workflow event processing complete"
        );

        Ok(execution_ids)
    }

    /// Check if an event matches a workflow's trigger configuration.
    fn matches_trigger_config(&self, workflow: &Workflow, event: &WorkflowEvent) -> bool {
        let config = &workflow.trigger_config.0;

        // Check building filter if present
        if let Some(building_ids) = config.get("building_ids").and_then(|v| v.as_array()) {
            if let Some(event_building_id) = event.building_id {
                let building_id_str = event_building_id.to_string();
                if !building_ids
                    .iter()
                    .any(|id| id.as_str() == Some(&building_id_str))
                {
                    return false;
                }
            } else {
                // Workflow requires specific buildings but event has no building
                return false;
            }
        }

        // Check additional trigger-specific filters
        match event.event_type.as_str() {
            trigger_type::FAULT_CREATED | trigger_type::FAULT_STATUS_CHANGED => {
                // Check fault category filter
                if let Some(categories) = config.get("fault_categories").and_then(|v| v.as_array())
                {
                    if let Some(category) = event.data.get("category").and_then(|v| v.as_str()) {
                        if !categories.iter().any(|c| c.as_str() == Some(category)) {
                            return false;
                        }
                    }
                }

                // Check priority filter
                if let Some(priorities) = config.get("fault_priorities").and_then(|v| v.as_array())
                {
                    if let Some(priority) = event.data.get("priority").and_then(|v| v.as_str()) {
                        if !priorities.iter().any(|p| p.as_str() == Some(priority)) {
                            return false;
                        }
                    }
                }
            }
            trigger_type::PAYMENT_OVERDUE => {
                // Check minimum amount threshold
                if let Some(min_amount) = config.get("min_amount").and_then(|v| v.as_f64()) {
                    if let Some(amount) = event.data.get("amount").and_then(|v| v.as_f64()) {
                        if amount < min_amount {
                            return false;
                        }
                    }
                }
            }
            _ => {}
        }

        true
    }

    /// Evaluate workflow conditions (Story 94.3).
    pub fn evaluate_conditions(
        &self,
        conditions: &[serde_json::Value],
        context: &ActionContext,
    ) -> Result<bool, WorkflowError> {
        if conditions.is_empty() {
            return Ok(true);
        }

        for condition_json in conditions {
            // Try to parse as a condition group
            if let Ok(group) = serde_json::from_value::<ConditionGroup>(condition_json.clone()) {
                if !evaluate_condition_group(&group, context)? {
                    return Ok(false);
                }
            } else if let Ok(condition) =
                serde_json::from_value::<Condition>(condition_json.clone())
            {
                // Try to parse as a single condition
                if !evaluate_single_condition(&condition, context)? {
                    return Ok(false);
                }
            } else {
                tracing::warn!(
                    condition = ?condition_json,
                    "Could not parse condition, skipping"
                );
            }
        }

        Ok(true)
    }
}

/// Evaluate a condition group.
///
/// Semantics per [`LogicalOperator`]:
/// - `And` — every condition **and** every nested group must hold.
/// - `Or`  — at least one condition or nested group must hold.
/// - `Not` — the negation of the group's conjunction (De Morgan): the group
///   is satisfied iff **not all** of its conditions/nested groups hold. Every
///   child participates — earlier revisions negated only the first condition
///   and silently dropped the rest, so a multi-condition `Not` gave wrong
///   results.
fn evaluate_condition_group(
    group: &ConditionGroup,
    context: &ActionContext,
) -> Result<bool, WorkflowError> {
    match group.operator {
        LogicalOperator::And => evaluate_conjunction(group, context),
        LogicalOperator::Or => {
            // At least one condition must be true
            for condition in &group.conditions {
                if evaluate_single_condition(condition, context)? {
                    return Ok(true);
                }
            }
            for nested_group in &group.groups {
                if evaluate_condition_group(nested_group, context)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        LogicalOperator::Not => {
            // NOT negates the conjunction of the ENTIRE group, so all
            // conditions and nested groups are considered — not just the
            // first. NOT(a AND b AND …) by De Morgan.
            Ok(!evaluate_conjunction(group, context)?)
        }
    }
}

/// Evaluate every condition and nested group of `group` as a conjunction (AND).
///
/// Returns `true` only when all conditions and all nested groups hold. An empty
/// group is vacuously `true`. Shared by the `And` and `Not` operators.
fn evaluate_conjunction(
    group: &ConditionGroup,
    context: &ActionContext,
) -> Result<bool, WorkflowError> {
    for condition in &group.conditions {
        if !evaluate_single_condition(condition, context)? {
            return Ok(false);
        }
    }
    for nested_group in &group.groups {
        if !evaluate_condition_group(nested_group, context)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Evaluate a single condition.
fn evaluate_single_condition(
    condition: &Condition,
    context: &ActionContext,
) -> Result<bool, WorkflowError> {
    // Get the field value from context
    let field_value = get_field_value(&condition.field, context);

    match condition.operator {
        ComparisonOperator::IsNull => Ok(field_value.is_null()),
        ComparisonOperator::IsNotNull => Ok(!field_value.is_null()),
        _ => {
            let compare_value = condition.value.as_ref().ok_or_else(|| {
                WorkflowError::ConditionError(format!(
                    "Missing comparison value for field: {}",
                    condition.field
                ))
            })?;

            match condition.operator {
                ComparisonOperator::Eq => Ok(field_value == *compare_value),
                ComparisonOperator::Ne => Ok(field_value != *compare_value),
                ComparisonOperator::Gt => Ok(compare_numeric(&field_value, compare_value)? > 0),
                ComparisonOperator::Gte => Ok(compare_numeric(&field_value, compare_value)? >= 0),
                ComparisonOperator::Lt => Ok(compare_numeric(&field_value, compare_value)? < 0),
                ComparisonOperator::Lte => Ok(compare_numeric(&field_value, compare_value)? <= 0),
                ComparisonOperator::Contains => {
                    let field_string = field_value.to_string();
                    let compare_string = compare_value.to_string();
                    let field_str = field_value.as_str().unwrap_or(&field_string);
                    let compare_str = compare_value.as_str().unwrap_or(&compare_string);
                    Ok(field_str.contains(compare_str))
                }
                ComparisonOperator::StartsWith => {
                    let field_string = field_value.to_string();
                    let compare_string = compare_value.to_string();
                    let field_str = field_value.as_str().unwrap_or(&field_string);
                    let compare_str = compare_value.as_str().unwrap_or(&compare_string);
                    Ok(field_str.starts_with(compare_str))
                }
                ComparisonOperator::EndsWith => {
                    let field_string = field_value.to_string();
                    let compare_string = compare_value.to_string();
                    let field_str = field_value.as_str().unwrap_or(&field_string);
                    let compare_str = compare_value.as_str().unwrap_or(&compare_string);
                    Ok(field_str.ends_with(compare_str))
                }
                ComparisonOperator::In => {
                    if let Some(arr) = compare_value.as_array() {
                        Ok(arr.contains(&field_value))
                    } else {
                        Ok(false)
                    }
                }
                ComparisonOperator::NotIn => {
                    if let Some(arr) = compare_value.as_array() {
                        Ok(!arr.contains(&field_value))
                    } else {
                        Ok(true)
                    }
                }
                ComparisonOperator::Matches => {
                    let field_string = field_value.to_string();
                    let field_str = field_value.as_str().unwrap_or(&field_string);
                    let pattern = compare_value.as_str().ok_or_else(|| {
                        WorkflowError::ConditionError("Regex pattern must be a string".to_string())
                    })?;
                    let regex = regex::Regex::new(pattern).map_err(|e| {
                        WorkflowError::ConditionError(format!("Invalid regex: {}", e))
                    })?;
                    Ok(regex.is_match(field_str))
                }
                _ => Ok(false),
            }
        }
    }
}

/// Get a field value from the context using dot notation.
fn get_field_value(field: &str, context: &ActionContext) -> serde_json::Value {
    let parts: Vec<&str> = field.split('.').collect();
    if parts.is_empty() {
        return serde_json::Value::Null;
    }

    let root = parts[0];
    let remaining = &parts[1..];

    let root_value = if root == "trigger" {
        context.trigger_event.clone()
    } else if root.starts_with("action_") {
        context
            .context
            .get(root)
            .cloned()
            .unwrap_or(serde_json::Value::Null)
    } else {
        // Direct field access on trigger event
        return traverse_json(&context.trigger_event, &parts);
    };

    traverse_json(&root_value, remaining)
}

/// Traverse a JSON value using field path.
fn traverse_json(value: &serde_json::Value, path: &[&str]) -> serde_json::Value {
    let mut current = value.clone();
    for key in path {
        current = match current {
            serde_json::Value::Object(map) => {
                map.get(*key).cloned().unwrap_or(serde_json::Value::Null)
            }
            serde_json::Value::Array(arr) => {
                if let Ok(index) = key.parse::<usize>() {
                    arr.get(index).cloned().unwrap_or(serde_json::Value::Null)
                } else {
                    serde_json::Value::Null
                }
            }
            _ => return serde_json::Value::Null,
        };
    }
    current
}

/// Compare two JSON values as numbers.
fn compare_numeric(a: &serde_json::Value, b: &serde_json::Value) -> Result<i32, WorkflowError> {
    let a_num = a
        .as_f64()
        .or_else(|| a.as_str().and_then(|s| s.parse::<f64>().ok()));

    let b_num = b
        .as_f64()
        .or_else(|| b.as_str().and_then(|s| s.parse::<f64>().ok()));

    match (a_num, b_num) {
        (Some(a), Some(b)) => {
            if (a - b).abs() < f64::EPSILON {
                Ok(0)
            } else if a > b {
                Ok(1)
            } else {
                Ok(-1)
            }
        }
        _ => Err(WorkflowError::ConditionError(format!(
            "Cannot compare non-numeric values: {:?} and {:?}",
            a, b
        ))),
    }
}

/// Task that runs the actual workflow execution.
///
/// RLS (PAP-77): `org_id` / `acting_user` come from the workflow row loaded
/// org-scoped by the caller. Every database operation acquires its own
/// short-lived context-set connection via [`Self::rls_conn`] — a guard is
/// never held across action execution or retry sleeps (which can last up to
/// an hour), so the pool cannot be starved by slow actions.
struct WorkflowExecutorTask {
    db: RlsPool,
    org_id: Uuid,
    acting_user: Uuid,
    workflow_repo: WorkflowRepository,
    action_registry: ActionRegistry,
    config: WorkflowExecutorConfig,
}

impl WorkflowExecutorTask {
    /// Acquire a connection with this execution's RLS context set.
    async fn rls_conn(&self) -> Result<RlsGuard, sqlx::Error> {
        self.db
            .acquire_with_rls(self.org_id, self.acting_user, false)
            .await
    }

    /// Update the execution status on a fresh org-context connection.
    async fn set_execution_status(
        &self,
        execution_id: Uuid,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<(), WorkflowError> {
        let mut guard = self.rls_conn().await?;
        let res = self
            .workflow_repo
            .update_execution_status(&mut **guard.conn(), execution_id, status, error_message)
            .await;
        guard.release().await;
        res?;
        Ok(())
    }

    /// Run the workflow execution.
    async fn run(
        &self,
        workflow: Workflow,
        execution_id: Uuid,
        trigger_event: serde_json::Value,
        context: Option<serde_json::Value>,
    ) -> Result<(), WorkflowError> {
        tracing::info!(
            workflow_id = %workflow.id,
            execution_id = %execution_id,
            "Starting workflow execution"
        );

        // Get workflow actions. The executor is internal — we already
        // loaded `workflow`, so the organization scope is derived from the
        // row itself (no risk of cross-tenant leakage here).
        let mut guard = self.rls_conn().await?;
        let actions = self
            .workflow_repo
            .list_actions(guard.conn(), workflow.id, workflow.organization_id)
            .await;
        guard.release().await;
        let actions = actions?.unwrap_or_default();

        if actions.is_empty() {
            tracing::warn!(
                workflow_id = %workflow.id,
                execution_id = %execution_id,
                "Workflow has no actions"
            );
            self.set_execution_status(execution_id, execution_status::COMPLETED, None)
                .await?;
            return Ok(());
        }

        // Create action context
        let mut action_context = ActionContext::new(
            workflow.organization_id,
            workflow.id,
            execution_id,
            trigger_event,
        );

        // Add initial context variables
        if let Some(ctx) = context {
            if let Some(obj) = ctx.as_object() {
                for (k, v) in obj {
                    if let Some(s) = v.as_str() {
                        action_context.set_variable(k.clone(), s.to_string());
                    }
                }
            }
        }

        // Execute actions in order
        for action in actions {
            let step_result = self.execute_action(&action, &mut action_context).await;

            match step_result {
                Ok(result) => {
                    // Add action output to context
                    action_context.add_action_output(action.action_order, result.output);
                }
                Err(e) => {
                    // Handle failure based on on_failure setting
                    match action.on_failure.as_str() {
                        on_failure::CONTINUE => {
                            tracing::warn!(
                                workflow_id = %workflow.id,
                                execution_id = %execution_id,
                                action_order = action.action_order,
                                error = %e,
                                "Action failed, continuing execution"
                            );
                        }
                        _ => {
                            tracing::error!(
                                workflow_id = %workflow.id,
                                execution_id = %execution_id,
                                action_order = action.action_order,
                                error = %e,
                                "Action failed, stopping execution"
                            );
                            self.set_execution_status(
                                execution_id,
                                execution_status::FAILED,
                                Some(&e.to_string()),
                            )
                            .await?;
                            return Err(e);
                        }
                    }
                }
            }
        }

        // Mark execution as completed
        self.set_execution_status(execution_id, execution_status::COMPLETED, None)
            .await?;

        tracing::info!(
            workflow_id = %workflow.id,
            execution_id = %execution_id,
            "Workflow execution completed successfully"
        );

        Ok(())
    }

    /// Execute a single action with retry logic.
    async fn execute_action(
        &self,
        action: &WorkflowAction,
        context: &mut ActionContext,
    ) -> Result<ActionResult, WorkflowError> {
        // Create execution step record
        let mut guard = self.rls_conn().await?;
        let step = self
            .workflow_repo
            .create_execution_step(&mut **guard.conn(), context.execution_id, action.id)
            .await;
        guard.release().await;
        let step = step?;

        let start = Instant::now();

        // Get the executor for this action type
        let executor = self
            .action_registry
            .get(&action.action_type)
            .ok_or_else(|| ActionError::InvalidActionType(action.action_type.clone()))?;

        // Execute with retry logic. Defense-in-depth (H4): clamp the
        // retry_count and retry_delay_seconds read from the DB even though
        // `add_action` already clamps on insertion — a row written before
        // the clamp landed (or via a future side channel) must not be able
        // to wedge the executor.
        let mut last_error: Option<ActionError> = None;
        let max_retries = action.retry_count.clamp(0, MAX_RETRY_COUNT);
        let base_delay_seconds = action.retry_delay_seconds.clamp(0, MAX_RETRY_DELAY_SECONDS);

        for attempt in 0..=max_retries {
            if attempt > 0 {
                let delay_secs = compute_backoff_seconds(base_delay_seconds, attempt);
                let delay = Duration::from_secs(delay_secs);
                tracing::info!(
                    action_id = %action.id,
                    attempt = attempt,
                    delay_seconds = delay.as_secs(),
                    "Retrying action after delay"
                );
                tokio::time::sleep(delay).await;
            }

            match executor.execute(&action.action_config.0, context).await {
                Ok(mut result) => {
                    result.retry_attempt = attempt;
                    let duration_ms = start.elapsed().as_millis() as i32;

                    // Update step record
                    let mut guard = self.rls_conn().await?;
                    let updated = self
                        .workflow_repo
                        .update_execution_step(
                            &mut **guard.conn(),
                            step.id,
                            step_status::COMPLETED,
                            result.output.clone(),
                            None,
                            Some(duration_ms),
                        )
                        .await;
                    guard.release().await;
                    updated?;

                    return Ok(result);
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries {
                        tracing::warn!(
                            action_id = %action.id,
                            attempt = attempt,
                            max_retries = max_retries,
                            error = ?last_error,
                            "Action failed, will retry"
                        );
                    }
                }
            }
        }

        // All retries exhausted
        let error = last_error.unwrap_or(ActionError::ExecutionFailed("Unknown error".to_string()));
        let duration_ms = start.elapsed().as_millis() as i32;

        // Update step record with failure
        let mut guard = self.rls_conn().await?;
        let updated = self
            .workflow_repo
            .update_execution_step(
                &mut **guard.conn(),
                step.id,
                step_status::FAILED,
                serde_json::json!({}),
                Some(&error.to_string()),
                Some(duration_ms),
            )
            .await;
        guard.release().await;
        updated?;

        Err(WorkflowError::ActionFailed(error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_condition_parsing() {
        let condition_json = serde_json::json!({
            "field": "trigger.amount",
            "operator": "gt",
            "value": 100
        });

        let condition: Condition = serde_json::from_value(condition_json).unwrap();
        assert_eq!(condition.field, "trigger.amount");
        assert_eq!(condition.operator, ComparisonOperator::Gt);
        assert_eq!(condition.value, Some(serde_json::json!(100)));
    }

    #[test]
    fn test_condition_group_parsing() {
        let group_json = serde_json::json!({
            "operator": "and",
            "conditions": [
                {"field": "trigger.amount", "operator": "gt", "value": 100},
                {"field": "trigger.status", "operator": "eq", "value": "overdue"}
            ]
        });

        let group: ConditionGroup = serde_json::from_value(group_json).unwrap();
        assert_eq!(group.operator, LogicalOperator::And);
        assert_eq!(group.conditions.len(), 2);
    }

    // ------------------------------------------------------------------
    // NOT operator: must consider the FULL condition set, not just the
    // first child. Regression for the bug where evaluate_condition_group
    // negated only `conditions.first()` and dropped the rest.
    // ------------------------------------------------------------------

    fn eval_ctx(trigger_event: serde_json::Value) -> ActionContext {
        ActionContext::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            trigger_event,
        )
    }

    #[test]
    fn test_not_group_considers_all_conditions() {
        // First condition TRUE, second FALSE → conjunction is FALSE →
        // NOT(false) = TRUE. The old first-only path returned NOT(true) =
        // FALSE, silently ignoring the second condition.
        let ctx = eval_ctx(serde_json::json!({ "amount": 50, "status": "open" }));
        let group: ConditionGroup = serde_json::from_value(serde_json::json!({
            "operator": "not",
            "conditions": [
                { "field": "trigger.amount", "operator": "gt", "value": 10 },      // true
                { "field": "trigger.status", "operator": "eq", "value": "closed" } // false
            ]
        }))
        .unwrap();

        assert!(
            evaluate_condition_group(&group, &ctx).unwrap(),
            "NOT(true AND false) must be true; the second condition must not be dropped"
        );
    }

    #[test]
    fn test_not_group_all_true_is_false() {
        // Every condition TRUE → conjunction TRUE → NOT(true) = FALSE.
        let ctx = eval_ctx(serde_json::json!({ "amount": 50, "status": "open" }));
        let group: ConditionGroup = serde_json::from_value(serde_json::json!({
            "operator": "not",
            "conditions": [
                { "field": "trigger.amount", "operator": "gt", "value": 10 },    // true
                { "field": "trigger.status", "operator": "eq", "value": "open" } // true
            ]
        }))
        .unwrap();

        assert!(!evaluate_condition_group(&group, &ctx).unwrap());
    }

    #[test]
    fn test_not_group_does_not_drop_nested_groups() {
        // A TRUE condition plus a FALSE nested group → conjunction FALSE →
        // NOT(false) = TRUE. Confirms nested groups also participate in NOT.
        let ctx = eval_ctx(serde_json::json!({ "amount": 50, "status": "open" }));
        let group: ConditionGroup = serde_json::from_value(serde_json::json!({
            "operator": "not",
            "conditions": [
                { "field": "trigger.amount", "operator": "gt", "value": 10 } // true
            ],
            "groups": [
                {
                    "operator": "and",
                    "conditions": [
                        { "field": "trigger.status", "operator": "eq", "value": "closed" } // false
                    ]
                }
            ]
        }))
        .unwrap();

        assert!(evaluate_condition_group(&group, &ctx).unwrap());
    }

    #[test]
    fn test_workflow_event_creation() {
        let event = WorkflowEvent::new(
            trigger_type::FAULT_CREATED,
            Uuid::new_v4(),
            serde_json::json!({"title": "Test fault"}),
        )
        .with_building(Uuid::new_v4())
        .with_user(Uuid::new_v4());

        assert_eq!(event.event_type, trigger_type::FAULT_CREATED);
        assert!(event.building_id.is_some());
        assert!(event.triggered_by.is_some());
    }

    // ------------------------------------------------------------------
    // H4: retry backoff clamp
    // ------------------------------------------------------------------

    #[test]
    fn test_backoff_attempt_zero_is_zero() {
        // The first attempt has no delay — callers don't sleep on attempt=0,
        // but the helper should still return 0 defensively.
        assert_eq!(compute_backoff_seconds(60, 0), 0);
    }

    #[test]
    fn test_backoff_normal_growth() {
        // base=60, attempts 1..=5 → 60, 120, 240, 480, 960 (all < ceiling).
        assert_eq!(compute_backoff_seconds(60, 1), 60);
        assert_eq!(compute_backoff_seconds(60, 2), 120);
        assert_eq!(compute_backoff_seconds(60, 3), 240);
        assert_eq!(compute_backoff_seconds(60, 4), 480);
        assert_eq!(compute_backoff_seconds(60, 5), 960);
    }

    #[test]
    fn test_backoff_clamps_at_ceiling() {
        // base=60, attempt=7 → 60 * 2^6 = 3840 > 3600 → ceiling = 3600.
        assert_eq!(
            compute_backoff_seconds(60, 7),
            MAX_RETRY_DELAY_SECONDS as u64
        );
        // base=60, attempt=20 → would be 60 * 2^19; must still cap.
        assert_eq!(
            compute_backoff_seconds(60, 20),
            MAX_RETRY_DELAY_SECONDS as u64
        );
    }

    #[test]
    fn test_backoff_huge_input_does_not_overflow() {
        // The audit scenario: retry_count=100, retry_delay_seconds=86400,
        // attempt=99. Before the clamp this computed to ~2900 years and
        // could overflow u64. With the clamp it must be <= 1 hour.
        let delay = compute_backoff_seconds(86_400, 99);
        assert!(
            delay <= MAX_RETRY_DELAY_SECONDS as u64,
            "delay {} exceeded ceiling {}",
            delay,
            MAX_RETRY_DELAY_SECONDS
        );
    }

    #[test]
    fn test_backoff_negative_base_treated_as_zero() {
        // `retry_delay_seconds` is i32; a corrupt -1 row must not panic or
        // wrap to a huge u64.
        assert_eq!(compute_backoff_seconds(-1, 5), 0);
    }

    #[test]
    fn test_backoff_max_base_clamps_immediately() {
        // base at the per-row ceiling, attempt=1 → exactly the ceiling.
        assert_eq!(
            compute_backoff_seconds(MAX_RETRY_DELAY_SECONDS, 1),
            MAX_RETRY_DELAY_SECONDS as u64
        );
    }
}
