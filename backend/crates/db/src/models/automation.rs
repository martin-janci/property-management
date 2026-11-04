//! Workflow Automation models (Epic 38).
//!
//! Models for automation rules, triggers, and execution logs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================
// Automation Rules (Story 38.1-38.3)
// ============================================

/// Workflow automation rule entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct WorkflowAutomationRule {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub trigger_type: String,
    pub trigger_config: serde_json::Value,
    pub conditions: Option<serde_json::Value>,
    pub actions: serde_json::Value,
    pub is_active: bool,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub run_count: i32,
    pub error_count: i32,
    pub last_error: Option<String>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create automation rule request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateAutomationRule {
    pub name: String,
    pub description: Option<String>,
    pub trigger_type: String,
    pub trigger_config: serde_json::Value,
    pub conditions: Option<serde_json::Value>,
    pub actions: serde_json::Value,
    pub is_active: Option<bool>,
}

/// Update automation rule request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateAutomationRule {
    pub name: Option<String>,
    pub description: Option<String>,
    /// New trigger kind (`schedule`, `event`, `condition`, `manual`).
    #[serde(default)]
    pub trigger_type: Option<String>,
    pub trigger_config: Option<serde_json::Value>,
    pub conditions: Option<serde_json::Value>,
    pub actions: Option<serde_json::Value>,
    pub is_active: Option<bool>,
}

/// Automation rule with stats.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AutomationRuleWithStats {
    pub rule: WorkflowAutomationRule,
    pub recent_executions: i32,
    pub success_rate: f64,
}

// ============================================
// Automation Logs
// ============================================

/// Automation execution log entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct WorkflowAutomationLog {
    pub id: Uuid,
    pub rule_id: Uuid,
    pub trigger_data: Option<serde_json::Value>,
    pub actions_executed: Option<serde_json::Value>,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub duration_ms: Option<i32>,
}

/// Automation log summary.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AutomationLogSummary {
    pub id: Uuid,
    pub rule_name: String,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub duration_ms: Option<i32>,
}

/// Execution log row joined with its rule name (org-wide execution list).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AutomationExecution {
    pub id: Uuid,
    pub rule_id: Uuid,
    pub rule_name: String,
    pub trigger_type: String,
    pub trigger_data: Option<serde_json::Value>,
    pub actions_executed: Option<serde_json::Value>,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub duration_ms: Option<i32>,
}

/// Filters for the org-wide execution list / stats.
#[derive(Debug, Clone, Default)]
pub struct AutomationExecutionFilter {
    pub since: Option<DateTime<Utc>>,
    pub status: Option<String>,
    pub rule_id: Option<Uuid>,
}

/// Paginated org-wide execution list.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AutomationExecutionList {
    pub items: Vec<AutomationExecution>,
    pub total: i64,
}

/// Aggregate execution statistics for an organization.
///
/// `failed` counts `failed` + `partial_failure`; `pending` counts `pending` +
/// `running`.
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AutomationExecutionStats {
    pub total: i64,
    pub successful: i64,
    pub failed: i64,
    pub pending: i64,
    pub average_duration_ms: Option<f64>,
}

// ============================================
// Automation Templates
// ============================================

/// Automation template entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct WorkflowAutomationTemplate {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub trigger_type: String,
    pub trigger_config_template: serde_json::Value,
    pub actions_template: serde_json::Value,
    pub is_system: bool,
    pub created_at: DateTime<Utc>,
}

/// Create rule from template request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateRuleFromTemplate {
    pub template_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub trigger_config_overrides: Option<serde_json::Value>,
    pub actions_overrides: Option<serde_json::Value>,
}

// ============================================
// Trigger Configuration Types
// ============================================

/// Schedule trigger configuration.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScheduleTriggerConfig {
    pub cron: String,
    pub timezone: Option<String>,
}

/// Event trigger configuration.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EventTriggerConfig {
    pub event_type: String,
    pub conditions: Option<serde_json::Value>,
}

/// Condition trigger configuration.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConditionTriggerConfig {
    pub check: String,
    pub operator: String,
    pub value: serde_json::Value,
}

// ============================================
// Action Configuration Types
// ============================================

/// Automation action.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AutomationAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub config: serde_json::Value,
}

/// Send notification action config.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SendNotificationConfig {
    pub template: String,
    pub recipients: Vec<String>,
}

/// Send email action config.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SendEmailConfig {
    pub template: String,
    pub recipients: Option<Vec<String>>,
}

/// Generate report action config.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GenerateReportConfig {
    pub report: String,
    pub period: Option<String>,
    pub per_unit: Option<bool>,
}

/// Call webhook action config.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CallWebhookConfig {
    pub url: String,
    pub method: Option<String>,
    pub headers: Option<serde_json::Value>,
    pub body_template: Option<String>,
}
