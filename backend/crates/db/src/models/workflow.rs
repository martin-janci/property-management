//! Workflow automation models (Epic 13, Story 13.6 & 13.7).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Workflow {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub trigger_type: String,
    pub trigger_config: sqlx::types::Json<serde_json::Value>,
    pub conditions: sqlx::types::Json<Vec<serde_json::Value>>,
    pub enabled: bool,
    pub last_triggered_at: Option<DateTime<Utc>>,
    pub trigger_count: i32,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Workflow action in sequence.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WorkflowAction {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub action_order: i32,
    pub action_type: String,
    pub action_config: sqlx::types::Json<serde_json::Value>,
    pub on_failure: String,
    pub retry_count: i32,
    pub retry_delay_seconds: i32,
    pub created_at: DateTime<Utc>,
}

/// Workflow execution instance.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WorkflowExecution {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub trigger_event: sqlx::types::Json<serde_json::Value>,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub context: sqlx::types::Json<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Workflow execution step detail.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WorkflowExecutionStep {
    pub id: Uuid,
    pub execution_id: Uuid,
    pub action_id: Uuid,
    pub status: String,
    pub input: sqlx::types::Json<serde_json::Value>,
    pub output: sqlx::types::Json<serde_json::Value>,
    pub error_message: Option<String>,
    pub retry_attempt: i32,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// Workflow schedule for cron-based triggers.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WorkflowSchedule {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub cron_expression: String,
    pub timezone: String,
    pub next_run_at: DateTime<Utc>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Trigger type constants.
pub mod trigger_type {
    pub const FAULT_CREATED: &str = "fault_created";
    pub const FAULT_STATUS_CHANGED: &str = "fault_status_changed";
    pub const FAULT_RESOLVED: &str = "fault_resolved";
    pub const PAYMENT_DUE: &str = "payment_due";
    pub const PAYMENT_OVERDUE: &str = "payment_overdue";
    pub const PAYMENT_RECEIVED: &str = "payment_received";
    pub const DOCUMENT_UPLOADED: &str = "document_uploaded";
    pub const DOCUMENT_SIGNED: &str = "document_signed";
    pub const VOTE_CREATED: &str = "vote_created";
    pub const VOTE_ENDED: &str = "vote_ended";
    pub const ANNOUNCEMENT_CREATED: &str = "announcement_created";
    pub const METER_READING_DUE: &str = "meter_reading_due";
    pub const METER_READING_ANOMALY: &str = "meter_reading_anomaly";
    pub const SCHEDULE: &str = "schedule";
    pub const MANUAL: &str = "manual";

    pub const ALL: &[&str] = &[
        FAULT_CREATED,
        FAULT_STATUS_CHANGED,
        FAULT_RESOLVED,
        PAYMENT_DUE,
        PAYMENT_OVERDUE,
        PAYMENT_RECEIVED,
        DOCUMENT_UPLOADED,
        DOCUMENT_SIGNED,
        VOTE_CREATED,
        VOTE_ENDED,
        ANNOUNCEMENT_CREATED,
        METER_READING_DUE,
        METER_READING_ANOMALY,
        SCHEDULE,
        MANUAL,
    ];
}

/// Action type constants.
pub mod action_type {
    pub const SEND_NOTIFICATION: &str = "send_notification";
    pub const SEND_EMAIL: &str = "send_email";
    pub const SEND_SMS: &str = "send_sms";
    pub const CREATE_TASK: &str = "create_task";
    pub const UPDATE_STATUS: &str = "update_status";
    pub const ASSIGN_TO_USER: &str = "assign_to_user";
    pub const CREATE_ANNOUNCEMENT: &str = "create_announcement";
    pub const CREATE_FAULT: &str = "create_fault";
    pub const CALL_WEBHOOK: &str = "call_webhook";
    pub const DELAY: &str = "delay";
    pub const CONDITION_BRANCH: &str = "condition_branch";

    pub const ALL: &[&str] = &[
        SEND_NOTIFICATION,
        SEND_EMAIL,
        SEND_SMS,
        CREATE_TASK,
        UPDATE_STATUS,
        ASSIGN_TO_USER,
        CREATE_ANNOUNCEMENT,
        CREATE_FAULT,
        CALL_WEBHOOK,
        DELAY,
        CONDITION_BRANCH,
    ];
}

/// Execution status constants.
pub mod execution_status {
    pub const PENDING: &str = "pending";
    pub const RUNNING: &str = "running";
    pub const COMPLETED: &str = "completed";
    pub const FAILED: &str = "failed";
    pub const CANCELLED: &str = "cancelled";
    pub const ALL: &[&str] = &[PENDING, RUNNING, COMPLETED, FAILED, CANCELLED];
}

/// Step status constants.
pub mod step_status {
    pub const PENDING: &str = "pending";
    pub const RUNNING: &str = "running";
    pub const COMPLETED: &str = "completed";
    pub const FAILED: &str = "failed";
    pub const SKIPPED: &str = "skipped";
    pub const ALL: &[&str] = &[PENDING, RUNNING, COMPLETED, FAILED, SKIPPED];
}

/// Failure handling mode.
pub mod on_failure {
    pub const STOP: &str = "stop";
    pub const CONTINUE: &str = "continue";
    pub const RETRY: &str = "retry";
    pub const ALL: &[&str] = &[STOP, CONTINUE, RETRY];
}

/// Request to create a workflow.
///
/// `organization_id` and `created_by` are intentionally NOT part of this
/// struct: they are always derived from the verified `RequestPrincipal`
/// server-side, never trusted from client input (prevents IDOR).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkflow {
    pub name: String,
    pub description: Option<String>,
    pub trigger_type: String,
    pub trigger_config: Option<serde_json::Value>,
    pub conditions: Option<Vec<serde_json::Value>>,
}

/// Request to update a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWorkflow {
    pub name: Option<String>,
    pub description: Option<String>,
    pub trigger_type: Option<String>,
    pub trigger_config: Option<serde_json::Value>,
    pub conditions: Option<Vec<serde_json::Value>>,
    pub enabled: Option<bool>,
}

/// Request to create a workflow action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkflowAction {
    pub workflow_id: Uuid,
    pub action_order: i32,
    pub action_type: String,
    pub action_config: serde_json::Value,
    pub on_failure: Option<String>,
    pub retry_count: Option<i32>,
    pub retry_delay_seconds: Option<i32>,
}

/// Request to trigger a workflow execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerWorkflow {
    pub workflow_id: Uuid,
    pub trigger_event: serde_json::Value,
    pub context: Option<serde_json::Value>,
}

/// Workflow with actions and stats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowWithDetails {
    pub workflow: Workflow,
    pub actions: Vec<WorkflowAction>,
    pub schedule: Option<WorkflowSchedule>,
    pub recent_executions: Vec<WorkflowExecution>,
}

/// Workflow summary for listing.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WorkflowSummary {
    pub id: Uuid,
    pub name: String,
    pub trigger_type: String,
    pub enabled: bool,
    pub trigger_count: i32,
    pub last_triggered_at: Option<DateTime<Utc>>,
    pub action_count: i64,
}

/// Query parameters for workflows.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowQuery {
    pub trigger_type: Option<String>,
    pub enabled: Option<bool>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Query parameters for executions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionQuery {
    pub workflow_id: Option<Uuid>,
    pub status: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
