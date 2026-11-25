//! Automation engine (gap audit 3.1): executes `workflow_automation_rules`.
//!
//! Rules were stored and edited through `/api/v1/automation`, but nothing
//! ever ran them. This module is the runner:
//!
//! - **Schedule** rules (`trigger_config.cron` + `timezone`) fire from the
//!   [`AutomationWorker`] loop when `next_run_at` elapses.
//! - **Event** rules fire when a handler or the scheduler calls
//!   [`AutomationEngine::emit`] (`fault_created`, `fault_status_changed`,
//!   `payment_received`, `payment_overdue`, `document_uploaded`,
//!   `announcement_published`, `vote_started`, `vote_ended`). Optional
//!   `trigger_config.filters` and the rule's `conditions` narrow the match.
//! - **Condition** rules are evaluated periodically against open faults or
//!   unpaid invoices (picked by the condition fields, `fault.*` or
//!   `payment.*`); each matching record fires the rule once.
//! - **Manual** rules (and any rule, for testing) run from
//!   `POST /api/v1/automation/rules/{id}/run`.
//!
//! Actions run in order: `send_notification`, `send_email`,
//! `webhook`/`call_webhook`, `generate_report`, `update_status` and
//! `assign_user`/`assign_fault` (fault events), and `delay`, which parks the
//! remaining actions as a background job resumed after the delay. Every run is
//! recorded in `workflow_automation_logs` with a per-action result, and the
//! rule's run/error counters are updated.
//!
//! RLS: the rule tables carry FORCE RLS. Cross-org due-work reads run under
//! the global-read context (migration 00251); everything else runs on a
//! connection scoped to the rule's organization.

use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{NaiveDate, Utc};
use db::models::{
    infrastructure::CreateBackgroundJob, AssignFault, UpdateFaultStatus, WorkflowAutomationRule,
};
use db::repositories::{AutomationRepository, BackgroundJobRepository, FaultRepository};
use db::DbPool;
use integrations::StorageService;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sqlx::pool::PoolConnection;
use sqlx::{PgConnection, Postgres};
use uuid::Uuid;

use super::actions::{ActionContext, ActionExecutor, ApiCallExecutor};
use super::email::EmailService;
use super::notification::{
    Notification, NotificationChannel, NotificationService, NotificationType,
};
use super::report_generator::{ReportFormat, ReportGenerator, ReportKind, ReportParams};
use crate::state::AppState;

/// Event names emitted to the engine. They match ppt-web's
/// `EventTriggerType` values.
pub mod event {
    pub const FAULT_CREATED: &str = "fault_created";
    pub const FAULT_STATUS_CHANGED: &str = "fault_status_changed";
    pub const PAYMENT_RECEIVED: &str = "payment_received";
    pub const PAYMENT_OVERDUE: &str = "payment_overdue";
    pub const DOCUMENT_UPLOADED: &str = "document_uploaded";
    pub const ANNOUNCEMENT_PUBLISHED: &str = "announcement_published";
    pub const VOTE_STARTED: &str = "vote_started";
    pub const VOTE_ENDED: &str = "vote_ended";
}

/// Background-job queue holding delayed continuations.
pub const AUTOMATION_QUEUE: &str = "automation";
/// Job type of a delayed continuation.
pub const CONTINUE_JOB: &str = "automation_continue";
const WORKER_ID: &str = "api-server-automation-worker";
/// Longest supported `delay` action.
const MAX_DELAY_SECS: i64 = 30 * 24 * 3600;
/// Upper bound of records one condition rule may fire for in one pass.
const MAX_CONDITION_MATCHES: usize = 100;
/// Fault statuses that can be set by `update_status`.
const FAULT_STATUSES: &[&str] = &[
    "new",
    "triaged",
    "in_progress",
    "waiting_parts",
    "scheduled",
    "resolved",
    "closed",
    "reopened",
];

// ============================================================================
// Pure helpers: path lookup, templates, conditions
// ============================================================================

/// Resolve a dotted path (`fault.priority`) in a JSON object.
pub fn lookup<'a>(data: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = data;
    for part in path.split('.') {
        current = current.get(part)?;
    }
    Some(current)
}

fn value_to_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Replace `{{path}}` placeholders with values from `data`. Unknown
/// placeholders become empty strings.
pub fn render_template(template: &str, data: &Value) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find("}}") {
            Some(end) => {
                let path = after[..end].trim();
                if let Some(v) = lookup(data, path) {
                    out.push_str(&value_to_text(v));
                }
                rest = &after[end + 2..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// One condition from the rule builder.
#[derive(Debug, Clone, Deserialize)]
pub struct Condition {
    pub field: String,
    pub operator: String,
    #[serde(default)]
    pub value: Value,
}

fn as_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn list_items(value: &Value) -> Vec<String> {
    match value {
        Value::Array(items) => items
            .iter()
            .map(|v| value_to_text(v).trim().to_lowercase())
            .collect(),
        other => value_to_text(other)
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect(),
    }
}

/// Evaluate one condition against event data.
pub fn condition_matches(condition: &Condition, data: &Value) -> bool {
    let actual = lookup(data, &condition.field);
    let is_empty = match actual {
        None | Some(Value::Null) => true,
        Some(Value::String(s)) => s.trim().is_empty(),
        Some(Value::Array(a)) => a.is_empty(),
        _ => false,
    };
    match condition.operator.as_str() {
        "is_empty" => return is_empty,
        "is_not_empty" => return !is_empty,
        _ => {}
    }
    let Some(actual) = actual else {
        // A missing field only satisfies negative comparisons.
        return matches!(
            condition.operator.as_str(),
            "not_equals" | "not_contains" | "not_in_list"
        );
    };
    let actual_text = value_to_text(actual).to_lowercase();
    let expected_text = value_to_text(&condition.value).to_lowercase();
    let numbers = as_number(actual).zip(as_number(&condition.value));
    match condition.operator.as_str() {
        "equals" => match numbers {
            Some((a, b)) => (a - b).abs() < f64::EPSILON,
            None => actual_text == expected_text,
        },
        "not_equals" => match numbers {
            Some((a, b)) => (a - b).abs() >= f64::EPSILON,
            None => actual_text != expected_text,
        },
        "contains" => actual_text.contains(&expected_text),
        "not_contains" => !actual_text.contains(&expected_text),
        "greater_than" => numbers.is_some_and(|(a, b)| a > b),
        "less_than" => numbers.is_some_and(|(a, b)| a < b),
        "greater_than_or_equals" => numbers.is_some_and(|(a, b)| a >= b),
        "less_than_or_equals" => numbers.is_some_and(|(a, b)| a <= b),
        "in_list" => list_items(&condition.value).contains(&actual_text),
        "not_in_list" => !list_items(&condition.value).contains(&actual_text),
        _ => false,
    }
}

/// All conditions of a rule: the builder's `conditions` column plus any
/// `trigger_config.conditions` array (used by the built-in templates).
pub fn rule_conditions(rule: &WorkflowAutomationRule) -> Vec<Condition> {
    let mut out = Vec::new();
    for source in [
        rule.conditions.as_ref(),
        rule.trigger_config.get("conditions"),
    ]
    .into_iter()
    .flatten()
    {
        if let Value::Array(items) = source {
            out.extend(
                items
                    .iter()
                    .filter_map(|c| serde_json::from_value::<Condition>(c.clone()).ok()),
            );
        }
    }
    out
}

/// `trigger_config.filters` (or a legacy `conditions` object) as key/value
/// equality checks. A key is looked up as a path, then inside each top-level
/// entity object (`priority` matches `fault.priority`).
pub fn filters_match(trigger_config: &Value, data: &Value) -> bool {
    let filters = trigger_config
        .get("filters")
        .or_else(|| trigger_config.get("conditions"))
        .and_then(Value::as_object);
    let Some(filters) = filters else {
        return true;
    };
    filters.iter().all(|(key, expected)| {
        let actual = lookup(data, key).or_else(|| {
            data.as_object()
                .and_then(|obj| obj.values().find_map(|entity| entity.get(key)))
        });
        match actual {
            Some(actual) => {
                let expected_items = list_items(expected);
                expected_items.contains(&value_to_text(actual).to_lowercase())
            }
            None => false,
        }
    })
}

/// Which records a condition rule is evaluated against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionEntity {
    Fault,
    Payment,
}

pub fn condition_entity(conditions: &[Condition]) -> Option<ConditionEntity> {
    conditions.iter().find_map(|c| {
        if c.field.starts_with("payment.") {
            Some(ConditionEntity::Payment)
        } else if c.field.starts_with("fault.") {
            Some(ConditionEntity::Fault)
        } else {
            None
        }
    })
}

/// An action of a rule as stored by the rule builder.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuleAction {
    #[serde(rename = "type")]
    pub action_type: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub config: Value,
    #[serde(default)]
    pub order: Option<i64>,
}

/// Parse and order a rule's actions.
pub fn parse_actions(actions: &Value) -> Vec<RuleAction> {
    let mut parsed: Vec<(usize, RuleAction)> = actions
        .as_array()
        .map(|items| {
            items
                .iter()
                .enumerate()
                .filter_map(|(i, a)| {
                    serde_json::from_value::<RuleAction>(a.clone())
                        .ok()
                        .map(|a| (i, a))
                })
                .collect()
        })
        .unwrap_or_default();
    parsed.sort_by_key(|(i, a)| (a.order.unwrap_or(*i as i64), *i));
    parsed.into_iter().map(|(_, a)| a).collect()
}

/// Delay length in seconds from a `delay` action config.
pub fn delay_seconds(config: &Value) -> Result<i64, String> {
    let duration = config
        .get("duration")
        .and_then(as_number)
        .ok_or("delay requires a numeric duration")?;
    if duration <= 0.0 {
        return Err("delay duration must be positive".into());
    }
    let unit = config
        .get("unit")
        .and_then(Value::as_str)
        .unwrap_or("minutes");
    let factor = match unit {
        "seconds" => 1.0,
        "minutes" => 60.0,
        "hours" => 3600.0,
        "days" => 86400.0,
        other => return Err(format!("unknown delay unit {}", other)),
    };
    let secs = (duration * factor).round();
    if secs > MAX_DELAY_SECS as f64 {
        return Err("delay may not exceed 30 days".into());
    }
    Ok(secs as i64)
}

/// Recipient keywords and addresses from a config value (`"managers, a@b.c"`
/// or `["managers", "a@b.c"]`).
fn recipient_tokens(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .map(|v| value_to_text(v).trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        Some(Value::String(s)) => s
            .split([',', ';'])
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

/// Next run for a schedule rule (`trigger_config.cron`/`schedule` +
/// `timezone`, default Europe/Bratislava).
pub fn next_schedule_run(
    trigger_config: &Value,
    after: chrono::DateTime<Utc>,
) -> Option<chrono::DateTime<Utc>> {
    let cron = trigger_config
        .get("cron")
        .or_else(|| trigger_config.get("schedule"))
        .and_then(Value::as_str)?;
    let tz: chrono_tz::Tz = trigger_config
        .get("timezone")
        .and_then(Value::as_str)
        .and_then(|t| t.parse().ok())
        .unwrap_or(chrono_tz::Europe::Bratislava);
    crate::routes::reports::compute_next_run_from_cron(after.with_timezone(&tz), cron)
}

/// Action types the engine executes.
pub const SUPPORTED_ACTIONS: &[&str] = &[
    "send_notification",
    "send_email",
    "webhook",
    "call_webhook",
    "generate_report",
    "update_status",
    "assign_user",
    "assign_fault",
    "delay",
];

/// Validate a rule definition before it is stored. Returns a message
/// suitable for a 400 response.
pub fn validate_rule(
    trigger_type: &str,
    trigger_config: &Value,
    actions: &Value,
) -> Result<(), String> {
    match trigger_type {
        "schedule" => {
            let cron = trigger_config
                .get("cron")
                .or_else(|| trigger_config.get("schedule"))
                .and_then(Value::as_str)
                .ok_or("Schedule rules need a cron expression")?;
            if !crate::routes::reports::validate_cron_expression(cron) {
                return Err(format!("Invalid cron expression: {}", cron));
            }
            if let Some(tz) = trigger_config.get("timezone").and_then(Value::as_str) {
                tz.parse::<chrono_tz::Tz>()
                    .map_err(|_| format!("Invalid timezone: {}", tz))?;
            }
        }
        "event" => {
            trigger_config
                .get("event_type")
                .and_then(Value::as_str)
                .ok_or("Event rules need an event_type")?;
        }
        "condition" | "manual" => {}
        other => return Err(format!("Unknown trigger type: {}", other)),
    }
    let Some(items) = actions.as_array() else {
        return Err("actions must be a list".into());
    };
    for action in parse_actions(actions) {
        if !SUPPORTED_ACTIONS.contains(&action.action_type.as_str()) {
            return Err(format!(
                "Action type {} is not supported",
                action.action_type
            ));
        }
        if action.action_type == "delay" {
            delay_seconds(&action.config)?;
        }
        if matches!(action.action_type.as_str(), "webhook" | "call_webhook") {
            let url = action
                .config
                .get("url")
                .and_then(Value::as_str)
                .unwrap_or("");
            if !url.starts_with("https://") {
                return Err("Webhook URL must start with https://".into());
            }
        }
        if action.action_type == "generate_report"
            && action
                .config
                .get("report")
                .and_then(Value::as_str)
                .and_then(ReportKind::parse)
                .is_none()
        {
            return Err(
                "generate_report needs report: faults, voting, occupancy or consumption".into(),
            );
        }
    }
    if items.len() != parse_actions(actions).len() {
        return Err("Every action needs a type".into());
    }
    Ok(())
}

// ============================================================================
// Event payload builders
// ============================================================================

/// Event data for a fault (`fault.*`, `building.id`, `unit.id`).
pub fn fault_event_data(fault: &db::models::Fault) -> Value {
    json!({
        "fault": {
            "id": fault.id,
            "title": fault.title,
            "description": fault.description,
            "category": fault.category,
            "priority": fault.priority,
            "status": fault.status,
            "location": fault.location_description,
            "reporterId": fault.reporter_id,
            "assignedTo": fault.assigned_to,
            "buildingId": fault.building_id,
            "unitId": fault.unit_id,
        },
        "building": { "id": fault.building_id },
        "unit": { "id": fault.unit_id },
    })
}

/// Event data for a vote (`vote.*`, `building.id`).
pub fn vote_event_data(vote: &db::models::Vote) -> Value {
    json!({
        "vote": {
            "id": vote.id,
            "title": vote.title,
            "status": vote.status,
            "buildingId": vote.building_id,
            "startAt": vote.start_at,
            "endAt": vote.end_at,
        },
        "building": { "id": vote.building_id },
    })
}

/// Event data for a document (`document.type` is its category).
pub fn document_event_data(document: &db::models::Document) -> Value {
    json!({
        "document": {
            "id": document.id,
            "title": document.title,
            "type": document.category,
            "fileName": document.file_name,
            "mimeType": document.mime_type,
            "size": document.size_bytes,
            "createdBy": document.created_by,
        },
    })
}

/// Event data for an announcement (`announcement.*`).
pub fn announcement_event_data(announcement: &db::models::Announcement) -> Value {
    json!({
        "announcement": {
            "id": announcement.id,
            "title": announcement.title,
            "content": announcement.content,
            "targetType": announcement.target_type,
            "authorId": announcement.author_id,
            "publishedAt": announcement.published_at,
        },
    })
}

// ============================================================================
// Engine
// ============================================================================

/// Result of one action.
#[derive(Debug, Clone, Serialize)]
pub struct ActionOutcome {
    #[serde(rename = "type")]
    pub action_type: String,
    pub name: String,
    pub status: &'static str,
    pub duration_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Summary of one rule run.
#[derive(Debug, Clone, Serialize)]
pub struct RunSummary {
    pub log_id: Uuid,
    pub status: &'static str,
    pub actions: Vec<ActionOutcome>,
}

/// Payload of a delayed continuation job.
#[derive(Debug, Serialize, Deserialize)]
struct ContinuationPayload {
    rule_id: Uuid,
    organization_id: Uuid,
    trigger_data: Value,
    start_at: usize,
}

/// A resolved recipient.
#[derive(Debug, Clone)]
struct Recipient {
    user_id: Option<Uuid>,
    email: Option<String>,
}

/// The automation engine. Cheap to clone.
#[derive(Clone)]
pub struct AutomationEngine {
    pool: DbPool,
    repo: AutomationRepository,
    jobs: BackgroundJobRepository,
    faults: FaultRepository,
    email: EmailService,
    notifications: Arc<NotificationService>,
    storage: Option<StorageService>,
    generator: ReportGenerator,
    webhook: Arc<ApiCallExecutor>,
}

impl AutomationEngine {
    pub fn new(
        pool: DbPool,
        email: EmailService,
        notifications: Arc<NotificationService>,
        storage: Option<StorageService>,
    ) -> Self {
        Self {
            repo: AutomationRepository::new(pool.clone()),
            jobs: BackgroundJobRepository::new(pool.clone()),
            faults: FaultRepository::new(pool.clone()),
            generator: ReportGenerator::from_pool(pool.clone()),
            pool,
            email,
            notifications,
            storage,
            webhook: Arc::new(ApiCallExecutor::new()),
        }
    }

    pub fn from_state(state: &AppState) -> Self {
        Self::new(
            state.db.clone(),
            state.email_service.clone(),
            state.notification_service.clone(),
            state.storage_service.clone(),
        )
    }

    /// Fire-and-forget: run the org's rules listening to `event_type` in the
    /// background so the request that caused the event is not slowed down.
    pub fn emit(state: &AppState, organization_id: Uuid, event_type: &'static str, data: Value) {
        Self::from_state(state).spawn_event(organization_id, event_type, data);
    }

    /// Same as [`Self::emit`] for callers holding an engine.
    pub fn spawn_event(&self, organization_id: Uuid, event_type: &'static str, data: Value) {
        let engine = self.clone();
        tokio::spawn(async move {
            match engine.handle_event(organization_id, event_type, data).await {
                Ok(0) => {}
                Ok(n) => {
                    tracing::info!(organization_id = %organization_id, event = event_type, rules = n, "Automation rules ran for event")
                }
                Err(e) => {
                    tracing::error!(organization_id = %organization_id, event = event_type, error = %e, "Automation event dispatch failed")
                }
            }
        });
    }

    async fn tenant_conn(&self, org: Uuid) -> Result<PoolConnection<Postgres>, String> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| format!("acquire: {}", e))?;
        if let Err(e) = db::tenant_context::set_tenant_context(&mut *conn, org).await {
            db::tenant_context::clear_or_discard(conn, "automation_engine").await;
            return Err(format!("tenant context: {}", e));
        }
        Ok(conn)
    }

    async fn release(conn: PoolConnection<Postgres>) {
        db::tenant_context::clear_or_discard(conn, "automation_engine").await;
    }

    /// Run the org's active rules for an event. Returns how many ran.
    pub async fn handle_event(
        &self,
        organization_id: Uuid,
        event_type: &str,
        data: Value,
    ) -> Result<usize, String> {
        let mut conn = self.tenant_conn(organization_id).await?;
        let rules = self
            .repo
            .list_event_rules_in(&mut *conn, organization_id, event_type)
            .await;
        Self::release(conn).await;
        let rules = rules.map_err(|e| e.to_string())?;

        let mut trigger_data = data;
        if let Value::Object(map) = &mut trigger_data {
            map.insert("event".into(), Value::String(event_type.to_string()));
        }

        let mut ran = 0;
        for rule in &rules {
            if !filters_match(&rule.trigger_config, &trigger_data) {
                continue;
            }
            if !rule_conditions(rule)
                .iter()
                .all(|c| condition_matches(c, &trigger_data))
            {
                continue;
            }
            match self.run_rule(rule, trigger_data.clone(), 0).await {
                Ok(_) => ran += 1,
                Err(e) => {
                    tracing::error!(rule_id = %rule.id, error = %e, "Automation rule run failed")
                }
            }
        }
        Ok(ran)
    }

    /// Run a rule's actions starting at `start_at`, logging the run.
    pub async fn run_rule(
        &self,
        rule: &WorkflowAutomationRule,
        trigger_data: Value,
        start_at: usize,
    ) -> Result<RunSummary, String> {
        let mut trigger_data = trigger_data;
        if let Value::Object(map) = &mut trigger_data {
            map.entry("rule")
                .or_insert_with(|| json!({ "id": rule.id, "name": rule.name }));
            map.entry("triggeredAt")
                .or_insert_with(|| Value::String(Utc::now().to_rfc3339()));
        }

        let mut conn = self.tenant_conn(rule.organization_id).await?;
        let log_id = match self
            .repo
            .start_log_in(&mut *conn, rule.id, &trigger_data)
            .await
        {
            Ok(id) => id,
            Err(e) => {
                Self::release(conn).await;
                return Err(format!("start log: {}", e));
            }
        };

        let actions = parse_actions(&rule.actions);
        let mut outcomes = Vec::new();
        for (index, action) in actions.iter().enumerate().skip(start_at) {
            let started = Instant::now();
            let name = action
                .name
                .clone()
                .unwrap_or_else(|| action.action_type.clone());

            if action.action_type == "delay" {
                let outcome = self
                    .schedule_continuation(rule, &trigger_data, index + 1, &action.config)
                    .await;
                let deferred = outcome.is_ok() && index + 1 < actions.len();
                outcomes.push(match outcome {
                    Ok(output) => ActionOutcome {
                        action_type: action.action_type.clone(),
                        name,
                        status: "completed",
                        duration_ms: started.elapsed().as_millis() as i64,
                        output: Some(output),
                        error: None,
                    },
                    Err(e) => ActionOutcome {
                        action_type: action.action_type.clone(),
                        name,
                        status: "failed",
                        duration_ms: started.elapsed().as_millis() as i64,
                        output: None,
                        error: Some(e),
                    },
                });
                if deferred {
                    break; // the rest runs from the continuation job
                }
                if outcomes.last().is_some_and(|o| o.status == "failed") {
                    break; // don't run the post-delay actions early
                }
                continue;
            }

            let result = self
                .execute_action(&mut conn, rule, action, &trigger_data, log_id)
                .await;
            outcomes.push(ActionOutcome {
                action_type: action.action_type.clone(),
                name,
                status: if result.is_ok() {
                    "completed"
                } else {
                    "failed"
                },
                duration_ms: started.elapsed().as_millis() as i64,
                error: result.as_ref().err().cloned(),
                output: result.ok(),
            });
        }

        let failed = outcomes.iter().filter(|o| o.status == "failed").count();
        let status = if failed == 0 {
            "success"
        } else if failed == outcomes.len() {
            "failed"
        } else {
            "partial_failure"
        };
        let error_message = outcomes
            .iter()
            .filter_map(|o| o.error.as_ref().map(|e| format!("{}: {}", o.name, e)))
            .collect::<Vec<_>>()
            .join("; ");
        let error_message = (!error_message.is_empty()).then_some(error_message);

        let finish = self
            .repo
            .finish_log_in(
                &mut *conn,
                log_id,
                status,
                &serde_json::to_value(&outcomes).unwrap_or(Value::Null),
                error_message.as_deref(),
            )
            .await;
        Self::release(conn).await;
        finish.map_err(|e| format!("finish log: {}", e))?;

        Ok(RunSummary {
            log_id,
            status,
            actions: outcomes,
        })
    }

    async fn schedule_continuation(
        &self,
        rule: &WorkflowAutomationRule,
        trigger_data: &Value,
        start_at: usize,
        config: &Value,
    ) -> Result<Value, String> {
        let secs = delay_seconds(config)?;
        let resume_at = Utc::now() + chrono::Duration::seconds(secs);
        let payload = ContinuationPayload {
            rule_id: rule.id,
            organization_id: rule.organization_id,
            trigger_data: trigger_data.clone(),
            start_at,
        };
        let job = self
            .jobs
            .create(
                CreateBackgroundJob {
                    job_type: CONTINUE_JOB.to_string(),
                    priority: Some(0),
                    payload: serde_json::to_value(&payload).map_err(|e| e.to_string())?,
                    scheduled_at: Some(resume_at),
                    queue: Some(AUTOMATION_QUEUE.to_string()),
                    max_attempts: Some(3),
                    org_id: Some(rule.organization_id),
                },
                rule.created_by,
            )
            .await
            .map_err(|e| format!("Failed to schedule continuation: {}", e))?;
        Ok(json!({ "resume_at": resume_at, "job_id": job.id }))
    }

    async fn execute_action(
        &self,
        conn: &mut PgConnection,
        rule: &WorkflowAutomationRule,
        action: &RuleAction,
        data: &Value,
        log_id: Uuid,
    ) -> Result<Value, String> {
        let config = &action.config;
        let text = |key: &str| -> Option<String> {
            config
                .get(key)
                .and_then(Value::as_str)
                .map(|s| render_template(s, data))
                .filter(|s| !s.trim().is_empty())
        };
        match action.action_type.as_str() {
            "send_notification" => {
                let title = text("title").unwrap_or_else(|| rule.name.clone());
                let message = text("message").unwrap_or_default();
                let mut tokens = recipient_tokens(config.get("recipients"));
                if tokens.is_empty() {
                    tokens.push("managers".into());
                }
                let recipients = self
                    .resolve_recipients(conn, rule.organization_id, &tokens, data)
                    .await?;
                let user_ids: Vec<Uuid> = recipients.iter().filter_map(|r| r.user_id).collect();
                if user_ids.is_empty() {
                    return Err("No recipients found".into());
                }
                let notification = Notification {
                    notification_type: NotificationType::WorkflowAction,
                    title,
                    body: message,
                    data: json!({ "rule_id": rule.id, "log_id": log_id }),
                    channels: vec![NotificationChannel::InApp, NotificationChannel::Push],
                };
                let delivered = self
                    .notifications
                    .send_to_users(&user_ids, &notification, Some(log_id))
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(json!({ "recipients": user_ids.len(), "delivered": delivered }))
            }
            "send_email" => {
                let subject = text("subject").unwrap_or_else(|| rule.name.clone());
                let body = text("message")
                    .or_else(|| text("body"))
                    .unwrap_or_else(|| default_email_body(rule, data));
                let tokens =
                    recipient_tokens(config.get("to").or_else(|| config.get("recipients")));
                let tokens: Vec<String> = tokens.iter().map(|t| render_template(t, data)).collect();
                if tokens.is_empty() {
                    return Err("send_email requires a recipient (to)".into());
                }
                let emails = self
                    .resolve_emails(conn, rule.organization_id, &tokens, data)
                    .await?;
                if emails.is_empty() {
                    return Err("No e-mail recipients found".into());
                }
                let mut sent = Vec::new();
                let mut errors = Vec::new();
                for to in &emails {
                    match self
                        .email
                        .send_action_email(to, &[], &[], &subject, &body, false)
                        .await
                    {
                        Ok(()) => sent.push(to.clone()),
                        Err(e) => errors.push(format!("{}: {}", to, e)),
                    }
                }
                if sent.is_empty() {
                    return Err(errors.join("; "));
                }
                Ok(json!({ "sent_to": sent, "failed": errors }))
            }
            "webhook" | "call_webhook" => {
                let url = text("url").ok_or("webhook requires a url")?;
                if !url.starts_with("https://") {
                    return Err("webhook url must use https".into());
                }
                let body = config.get("body").cloned().unwrap_or_else(|| {
                    json!({
                        "rule_id": rule.id,
                        "rule_name": rule.name,
                        "organization_id": rule.organization_id,
                        "execution_id": log_id,
                        "trigger": data,
                    })
                });
                let mut call = Map::new();
                call.insert("url".into(), Value::String(url));
                call.insert(
                    "method".into(),
                    config
                        .get("method")
                        .cloned()
                        .unwrap_or_else(|| json!("POST")),
                );
                call.insert("body".into(), body);
                if let Some(headers) = config.get("headers") {
                    call.insert("headers".into(), headers.clone());
                }
                let call = Value::Object(call);
                let context =
                    ActionContext::new(rule.organization_id, rule.id, log_id, data.clone());
                let result = self
                    .webhook
                    .execute(&call, &context)
                    .await
                    .map_err(|e| e.to_string())?;
                if result.success {
                    Ok(result.output)
                } else {
                    Err(result.error.unwrap_or_else(|| "webhook failed".into()))
                }
            }
            "generate_report" => {
                self.generate_report_action(conn, rule, config, data, log_id)
                    .await
            }
            "update_status" => {
                let fault_id = fault_id(data).ok_or("update_status needs a fault event")?;
                let status = text("status").ok_or("update_status requires a status")?;
                if !FAULT_STATUSES.contains(&status.as_str()) {
                    return Err(format!("Invalid fault status {}", status));
                }
                let actor = rule.created_by.ok_or("Rule has no owner to act as")?;
                let current: Option<String> = sqlx::query_scalar(
                    "SELECT status::text FROM faults WHERE id = $1 AND organization_id = $2",
                )
                .bind(fault_id)
                .bind(rule.organization_id)
                .fetch_optional(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
                let current = current.ok_or("Fault not found")?;
                if current == status {
                    return Ok(json!({ "fault_id": fault_id, "status": status, "changed": false }));
                }
                self.faults
                    .update_status_rls(
                        &mut *conn,
                        fault_id,
                        actor,
                        UpdateFaultStatus {
                            status: status.clone(),
                            note: Some(format!("Automation: {}", rule.name)),
                            scheduled_date: None,
                            estimated_completion: None,
                        },
                        current.clone(),
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(
                    json!({ "fault_id": fault_id, "from": current, "status": status, "changed": true }),
                )
            }
            "assign_user" | "assign_fault" => {
                let fault_id = fault_id(data).ok_or("assign_user needs a fault event")?;
                let user_id = text("user_id")
                    .or_else(|| text("assign_to"))
                    .and_then(|s| Uuid::parse_str(&s).ok())
                    .ok_or("assign_user requires a user_id")?;
                let actor = rule.created_by.ok_or("Rule has no owner to act as")?;
                let is_member: bool = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM organization_members \
                     WHERE organization_id = $1 AND user_id = $2 AND status = 'active')",
                )
                .bind(rule.organization_id)
                .bind(user_id)
                .fetch_one(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
                if !is_member {
                    return Err("Assignee is not an active member of the organization".into());
                }
                let in_org: bool = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM faults WHERE id = $1 AND organization_id = $2)",
                )
                .bind(fault_id)
                .bind(rule.organization_id)
                .fetch_one(&mut *conn)
                .await
                .map_err(|e| e.to_string())?;
                if !in_org {
                    return Err("Fault not found".into());
                }
                self.faults
                    .assign(
                        fault_id,
                        actor,
                        AssignFault {
                            assigned_to: user_id,
                        },
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(json!({ "fault_id": fault_id, "assigned_to": user_id }))
            }
            other => Err(format!("Action type {} is not supported", other)),
        }
    }

    async fn generate_report_action(
        &self,
        conn: &mut PgConnection,
        rule: &WorkflowAutomationRule,
        config: &Value,
        data: &Value,
        log_id: Uuid,
    ) -> Result<Value, String> {
        let kind = config
            .get("report")
            .and_then(Value::as_str)
            .and_then(ReportKind::parse)
            .ok_or("generate_report requires report: faults, voting, occupancy or consumption")?;
        let format = config
            .get("format")
            .and_then(Value::as_str)
            .map(|f| ReportFormat::parse(f).ok_or(format!("Unsupported format {}", f)))
            .transpose()?
            .unwrap_or(ReportFormat::Pdf);
        let days = config
            .get("period_days")
            .and_then(as_number)
            .map(|d| d.clamp(1.0, 1830.0) as i64)
            .unwrap_or(30);
        let today: NaiveDate = Utc::now().date_naive();
        let params = ReportParams::new(
            kind,
            rule.organization_id,
            None,
            Some(today - chrono::Duration::days(days)),
            Some(today),
        );
        let report = self.generator.generate(&params, format).await?;

        let mut output = json!({
            "report": kind.as_str(),
            "file_name": report.file_name,
            "file_size": report.bytes.len(),
        });
        if let Some(storage) = &self.storage {
            let key = format!(
                "reports/{}/automations/{}.{}",
                rule.organization_id,
                log_id,
                format.extension()
            );
            storage
                .upload_system_artifact(&key, report.bytes.clone(), report.content_type)
                .await
                .map_err(|e| format!("Failed to store report: {}", e))?;
            output["file_key"] = Value::String(key);
        }

        let tokens: Vec<String> = recipient_tokens(config.get("to"))
            .iter()
            .map(|t| render_template(t, data))
            .collect();
        if tokens.is_empty() {
            if self.storage.is_none() {
                return Err("generate_report needs recipients (to) or file storage".into());
            }
            return Ok(output);
        }
        let emails = self
            .resolve_emails(conn, rule.organization_id, &tokens, data)
            .await?;
        let subject = format!("{}: {} ({})", rule.name, kind.title(), today);
        let body = format!(
            "The automation rule \"{}\" generated the attached {} report for {} to {}.\n",
            rule.name,
            kind.title().to_lowercase(),
            params.from,
            params.to
        );
        let mut sent = Vec::new();
        for to in &emails {
            match self
                .email
                .send_email_with_attachment(
                    to,
                    &subject,
                    &body,
                    &report.file_name,
                    report.content_type,
                    report.bytes.clone(),
                )
                .await
            {
                Ok(()) => sent.push(to.clone()),
                Err(e) => {
                    tracing::warn!(to = %to, error = %e, "Failed to e-mail automation report")
                }
            }
        }
        if sent.is_empty() {
            return Err("The report could not be e-mailed to any recipient".into());
        }
        output["sent_to"] = json!(sent);
        Ok(output)
    }

    /// Resolve recipient tokens to users: `managers`, `reporter`, `assignee`,
    /// `unit_residents`, a user id, or an e-mail address.
    async fn resolve_recipients(
        &self,
        conn: &mut PgConnection,
        org: Uuid,
        tokens: &[String],
        data: &Value,
    ) -> Result<Vec<Recipient>, String> {
        let mut out: Vec<Recipient> = Vec::new();
        for token in tokens {
            let lowered = token.to_lowercase();
            let rows: Vec<(Uuid, String)> = match lowered.as_str() {
                "managers" | "manager" => sqlx::query_as(
                    "SELECT u.id, u.email FROM organization_members m \
                     JOIN users u ON u.id = m.user_id \
                     WHERE m.organization_id = $1 AND m.status = 'active' \
                       AND m.role_type IN ('org_admin', 'manager', 'technical_manager')",
                )
                .bind(org)
                .fetch_all(&mut *conn)
                .await
                .map_err(|e| e.to_string())?,
                "reporter" | "assignee" => {
                    let key = if lowered == "reporter" {
                        "fault.reporterId"
                    } else {
                        "fault.assignedTo"
                    };
                    match lookup(data, key)
                        .and_then(Value::as_str)
                        .and_then(|s| Uuid::parse_str(s).ok())
                    {
                        Some(id) => self.member_users(conn, org, &[id]).await?,
                        None => Vec::new(),
                    }
                }
                "unit_residents" | "residents" => {
                    match lookup(data, "unit.id")
                        .and_then(Value::as_str)
                        .and_then(|s| Uuid::parse_str(s).ok())
                    {
                        Some(unit) => sqlx::query_as(
                            "SELECT DISTINCT u.id, u.email FROM unit_residents r \
                             JOIN units un ON un.id = r.unit_id \
                             JOIN buildings b ON b.id = un.building_id \
                             JOIN users u ON u.id = r.user_id \
                             WHERE r.unit_id = $1 AND r.end_date IS NULL \
                               AND b.organization_id = $2",
                        )
                        .bind(unit)
                        .bind(org)
                        .fetch_all(&mut *conn)
                        .await
                        .map_err(|e| e.to_string())?,
                        None => Vec::new(),
                    }
                }
                _ => {
                    if let Ok(id) = Uuid::parse_str(token) {
                        self.member_users(conn, org, &[id]).await?
                    } else if token.contains('@') {
                        out.push(Recipient {
                            user_id: None,
                            email: Some(token.clone()),
                        });
                        Vec::new()
                    } else {
                        return Err(format!("Unknown recipient {}", token));
                    }
                }
            };
            out.extend(rows.into_iter().map(|(id, email)| Recipient {
                user_id: Some(id),
                email: Some(email),
            }));
        }
        let mut seen = std::collections::HashSet::new();
        out.retain(|r| seen.insert((r.user_id, r.email.clone())));
        Ok(out)
    }

    /// Only users who are active members of the org may be addressed by id.
    async fn member_users(
        &self,
        conn: &mut PgConnection,
        org: Uuid,
        ids: &[Uuid],
    ) -> Result<Vec<(Uuid, String)>, String> {
        sqlx::query_as(
            "SELECT u.id, u.email FROM organization_members m \
             JOIN users u ON u.id = m.user_id \
             WHERE m.organization_id = $1 AND m.status = 'active' AND u.id = ANY($2)",
        )
        .bind(org)
        .bind(ids)
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| e.to_string())
    }

    async fn resolve_emails(
        &self,
        conn: &mut PgConnection,
        org: Uuid,
        tokens: &[String],
        data: &Value,
    ) -> Result<Vec<String>, String> {
        let mut emails: Vec<String> = self
            .resolve_recipients(conn, org, tokens, data)
            .await?
            .into_iter()
            .filter_map(|r| r.email)
            .collect();
        emails.sort();
        emails.dedup();
        Ok(emails)
    }

    /// Recompute a schedule rule's `next_run_at` after create/update so a
    /// changed cron takes effect immediately (other trigger types are cleared).
    pub async fn reschedule(&self, rule: &WorkflowAutomationRule) -> Result<(), String> {
        let next = if rule.trigger_type == "schedule" && rule.is_active {
            next_schedule_run(&rule.trigger_config, Utc::now())
        } else {
            None
        };
        let mut conn = self.tenant_conn(rule.organization_id).await?;
        let result = self.repo.set_next_run_in(&mut *conn, rule.id, next).await;
        Self::release(conn).await;
        result.map_err(|e| e.to_string())
    }

    /// Load a rule of an org under its tenant context.
    pub async fn find_rule(
        &self,
        id: Uuid,
        organization_id: Uuid,
    ) -> Result<Option<WorkflowAutomationRule>, String> {
        let mut conn = self.tenant_conn(organization_id).await?;
        let result = self
            .repo
            .find_rule_in(&mut *conn, id, organization_id)
            .await;
        Self::release(conn).await;
        result.map_err(|e| e.to_string())
    }

    // ------------------------------------------------------------------
    // Worker passes
    // ------------------------------------------------------------------

    /// Read rules cross-org under the global-read context.
    async fn global_read<T, F>(&self, f: F) -> Result<T, String>
    where
        F: for<'c> FnOnce(
            &'c AutomationRepository,
            &'c mut PgConnection,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<T, sqlx::Error>> + Send + 'c>,
        >,
    {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| format!("acquire: {}", e))?;
        if let Err(e) = db::tenant_context::set_global_read_context(&mut *conn, true).await {
            Self::release(conn).await;
            return Err(format!("global read: {}", e));
        }
        let result = f(&self.repo, &mut conn).await;
        // `clear_or_discard` resets app.global_read too, or drops the conn.
        Self::release(conn).await;
        result.map_err(|e| e.to_string())
    }

    /// Fire due schedule rules. A rule seen for the first time
    /// (`next_run_at` NULL) is only scheduled, not run.
    pub async fn run_due_schedules(&self) -> Result<usize, String> {
        let rules = self
            .global_read(|repo, conn| Box::pin(repo.list_due_schedule_rules_in(conn)))
            .await?;
        let now = Utc::now();
        let mut ran = 0;
        for rule in &rules {
            let next = next_schedule_run(&rule.trigger_config, now)
                .unwrap_or_else(|| now + chrono::Duration::days(1));
            let mut conn = self.tenant_conn(rule.organization_id).await?;
            let advanced = self
                .repo
                .set_next_run_in(&mut *conn, rule.id, Some(next))
                .await;
            Self::release(conn).await;
            if let Err(e) = advanced {
                tracing::error!(rule_id = %rule.id, error = %e, "Failed to advance automation rule");
                continue;
            }
            if rule.next_run_at.is_none() {
                continue;
            }
            match self
                .run_rule(
                    rule,
                    json!({ "event": "schedule", "scheduledFor": rule.next_run_at }),
                    0,
                )
                .await
            {
                Ok(_) => ran += 1,
                Err(e) => {
                    tracing::error!(rule_id = %rule.id, error = %e, "Scheduled automation rule failed")
                }
            }
        }
        Ok(ran)
    }

    /// Evaluate condition rules against open faults / unpaid invoices.
    pub async fn run_condition_rules(&self) -> Result<usize, String> {
        let rules = self
            .global_read(|repo, conn| Box::pin(repo.list_condition_rules_in(conn)))
            .await?;
        let mut ran = 0;
        for rule in &rules {
            let conditions = rule_conditions(rule);
            let Some(entity) = condition_entity(&conditions) else {
                continue;
            };
            let candidates = match self
                .condition_candidates(rule.organization_id, entity)
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!(rule_id = %rule.id, error = %e, "Failed to load condition candidates");
                    continue;
                }
            };
            let mut fired = 0;
            for (entity_key, mut data) in candidates {
                if fired >= MAX_CONDITION_MATCHES {
                    break;
                }
                if !conditions.iter().all(|c| condition_matches(c, &data)) {
                    continue;
                }
                let mut conn = self.tenant_conn(rule.organization_id).await?;
                let seen = self
                    .repo
                    .has_fired_for_entity_in(&mut *conn, rule.id, &entity_key)
                    .await;
                Self::release(conn).await;
                match seen {
                    Ok(true) => continue,
                    Ok(false) => {}
                    Err(e) => {
                        tracing::error!(rule_id = %rule.id, error = %e, "Dedupe lookup failed");
                        continue;
                    }
                }
                if let Value::Object(map) = &mut data {
                    map.insert("event".into(), json!("condition"));
                    map.insert("entity_key".into(), json!(entity_key));
                }
                match self.run_rule(rule, data, 0).await {
                    Ok(_) => {
                        fired += 1;
                        ran += 1;
                    }
                    Err(e) => {
                        tracing::error!(rule_id = %rule.id, error = %e, "Condition automation rule failed")
                    }
                }
            }
        }
        Ok(ran)
    }

    async fn condition_candidates(
        &self,
        org: Uuid,
        entity: ConditionEntity,
    ) -> Result<Vec<(String, Value)>, String> {
        let mut conn = self.tenant_conn(org).await?;
        let result = match entity {
            ConditionEntity::Fault => {
                let rows: Result<Vec<db::models::Fault>, _> = sqlx::query_as(
                    "SELECT id, organization_id, building_id, unit_id, reporter_id, \
                     title, description, location_description, \
                     category::text AS category, priority::text AS priority, status::text AS status, \
                     ai_category::text AS ai_category, ai_priority::text AS ai_priority, \
                     ai_confidence, ai_processed_at, assigned_to, assigned_at, triaged_by, triaged_at, \
                     resolved_at, resolved_by, resolution_notes, confirmed_at, confirmed_by, rating, \
                     feedback, scheduled_date, estimated_completion, idempotency_key, created_at, updated_at \
                     FROM faults WHERE organization_id = $1 AND status NOT IN ('resolved', 'closed') \
                     ORDER BY created_at DESC LIMIT 1000",
                )
                .bind(org)
                .fetch_all(&mut *conn)
                .await;
                rows.map(|faults| {
                    faults
                        .iter()
                        .map(|f| {
                            let mut data = fault_event_data(f);
                            data["fault"]["daysOpen"] =
                                json!((Utc::now() - f.created_at).num_days());
                            (format!("fault:{}", f.id), data)
                        })
                        .collect()
                })
            }
            ConditionEntity::Payment => {
                type InvoiceRow = (
                    Uuid,
                    Uuid,
                    String,
                    NaiveDate,
                    rust_decimal::Decimal,
                    rust_decimal::Decimal,
                    String,
                    String,
                );
                let rows: Result<Vec<InvoiceRow>, _> = sqlx::query_as(
                    "SELECT id, unit_id, invoice_number, due_date, total, balance_due, currency, status::text \
                     FROM invoices WHERE organization_id = $1 \
                       AND status::text IN ('sent', 'partial', 'overdue') AND balance_due > 0 \
                     ORDER BY due_date ASC LIMIT 1000",
                )
                .bind(org)
                .fetch_all(&mut *conn)
                .await;
                let today = Utc::now().date_naive();
                rows.map(|invoices| {
                    invoices
                        .into_iter()
                        .map(|(id, unit_id, number, due, total, balance, currency, status)| {
                            let days_overdue = (today - due).num_days().max(0);
                            (
                                // Keyed by due date so a re-issued invoice fires again.
                                format!("invoice:{}:{}", id, due),
                                json!({
                                    "payment": {
                                        "id": id,
                                        "invoiceNumber": number,
                                        "amount": total.to_string().parse::<f64>().unwrap_or(0.0),
                                        "balanceDue": balance.to_string(),
                                        "currency": currency,
                                        "dueDate": due,
                                        "daysOverdue": days_overdue,
                                        "status": status,
                                    },
                                    "unit": { "id": unit_id },
                                }),
                            )
                        })
                        .collect()
                })
            }
        };
        Self::release(conn).await;
        result.map_err(|e| e.to_string())
    }

    /// Resume rules parked by a `delay` action.
    pub async fn run_continuations(&self, limit: usize) -> Result<usize, String> {
        let mut processed = 0;
        while processed < limit {
            let Some(job) = self
                .jobs
                .claim_job(AUTOMATION_QUEUE, WORKER_ID, Some(&[CONTINUE_JOB]))
                .await
                .map_err(|e| format!("claim: {}", e))?
            else {
                break;
            };
            processed += 1;
            let outcome = async {
                let payload: ContinuationPayload = serde_json::from_value(job.payload.clone())
                    .map_err(|e| format!("Invalid continuation payload: {}", e))?;
                let mut conn = self.tenant_conn(payload.organization_id).await?;
                let rule = self
                    .repo
                    .find_rule_in(&mut *conn, payload.rule_id, payload.organization_id)
                    .await;
                Self::release(conn).await;
                let rule = rule.map_err(|e| e.to_string())?;
                match rule {
                    Some(rule) if rule.is_active => self
                        .run_rule(&rule, payload.trigger_data, payload.start_at)
                        .await
                        .map(|s| json!({ "log_id": s.log_id, "status": s.status })),
                    Some(_) => Ok(json!({ "skipped": "rule is paused" })),
                    None => Ok(json!({ "skipped": "rule was deleted" })),
                }
            }
            .await;
            let finalize = match outcome {
                Ok(result) => self.jobs.complete_job(job.id, Some(result)).await,
                Err(e) => self.jobs.fail_job(job.id, &e, None).await,
            };
            if let Err(e) = finalize {
                tracing::error!(job_id = %job.id, error = %e, "Failed to finalize automation continuation");
            }
        }
        Ok(processed)
    }
}

fn fault_id(data: &Value) -> Option<Uuid> {
    lookup(data, "fault.id")
        .and_then(Value::as_str)
        .and_then(|s| Uuid::parse_str(s).ok())
}

fn default_email_body(rule: &WorkflowAutomationRule, data: &Value) -> String {
    format!(
        "Automation rule \"{}\" was triggered.\n\n{}\n",
        rule.name,
        serde_json::to_string_pretty(data).unwrap_or_default()
    )
}

// ============================================================================
// Worker
// ============================================================================

/// Worker configuration.
#[derive(Debug, Clone)]
pub struct AutomationWorkerConfig {
    pub enabled: bool,
    pub poll_interval_secs: u64,
    pub condition_interval_secs: u64,
}

impl AutomationWorkerConfig {
    /// `AUTOMATION_WORKER_ENABLED`, `AUTOMATION_WORKER_INTERVAL_SECS` (60),
    /// `AUTOMATION_CONDITION_INTERVAL_SECS` (900).
    pub fn from_env() -> Self {
        let num = |key: &str, default: u64| {
            std::env::var(key)
                .ok()
                .and_then(|v| v.parse().ok())
                .filter(|&n: &u64| n > 0)
                .unwrap_or(default)
        };
        Self {
            enabled: std::env::var("AUTOMATION_WORKER_ENABLED")
                .map(|v| v != "false" && v != "0")
                .unwrap_or(true),
            poll_interval_secs: num("AUTOMATION_WORKER_INTERVAL_SECS", 60),
            condition_interval_secs: num("AUTOMATION_CONDITION_INTERVAL_SECS", 900),
        }
    }
}

/// Polls schedule rules, condition rules and delayed continuations.
pub struct AutomationWorker {
    engine: AutomationEngine,
    config: AutomationWorkerConfig,
}

impl AutomationWorker {
    pub fn new(engine: AutomationEngine, config: AutomationWorkerConfig) -> Self {
        Self { engine, config }
    }

    pub fn start(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            if !self.config.enabled {
                tracing::info!("Automation worker disabled - not starting");
                return;
            }
            tracing::info!(
                poll_interval_secs = self.config.poll_interval_secs,
                condition_interval_secs = self.config.condition_interval_secs,
                "Automation worker started"
            );
            let mut ticker =
                tokio::time::interval(Duration::from_secs(self.config.poll_interval_secs));
            let condition_every = Duration::from_secs(self.config.condition_interval_secs);
            let mut last_condition_pass: Option<Instant> = None;
            loop {
                ticker.tick().await;
                if let Err(e) = self.engine.run_due_schedules().await {
                    tracing::error!(error = %e, "Automation schedule pass failed");
                }
                if let Err(e) = self.engine.run_continuations(50).await {
                    tracing::error!(error = %e, "Automation continuation pass failed");
                }
                if last_condition_pass.is_none_or(|t| t.elapsed() >= condition_every) {
                    last_condition_pass = Some(Instant::now());
                    if let Err(e) = self.engine.run_condition_rules().await {
                        tracing::error!(error = %e, "Automation condition pass failed");
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cond(field: &str, operator: &str, value: Value) -> Condition {
        Condition {
            field: field.into(),
            operator: operator.into(),
            value,
        }
    }

    #[test]
    fn templates_resolve_paths() {
        let data = json!({ "fault": { "title": "Leak", "priority": "high" }, "n": 3 });
        assert_eq!(
            render_template(
                "{{ fault.title }} ({{fault.priority}}) x{{n}} {{missing}}!",
                &data
            ),
            "Leak (high) x3 !"
        );
        assert_eq!(render_template("open {{ end", &data), "open {{ end");
    }

    #[test]
    fn conditions_cover_operators() {
        let data = json!({ "fault": { "category": "Plumbing", "priority": "high", "location": "" },
                           "payment": { "daysOverdue": 9, "amount": "120.50" } });
        assert!(condition_matches(
            &cond("fault.category", "equals", json!("plumbing")),
            &data
        ));
        assert!(condition_matches(
            &cond("fault.priority", "not_equals", json!("low")),
            &data
        ));
        assert!(condition_matches(
            &cond("fault.category", "contains", json!("plumb")),
            &data
        ));
        assert!(condition_matches(
            &cond("payment.daysOverdue", "greater_than_or_equals", json!(7)),
            &data
        ));
        assert!(!condition_matches(
            &cond("payment.daysOverdue", "less_than", json!("7")),
            &data
        ));
        assert!(condition_matches(
            &cond("payment.amount", "greater_than", json!(100)),
            &data
        ));
        assert!(condition_matches(
            &cond("fault.location", "is_empty", Value::Null),
            &data
        ));
        assert!(condition_matches(
            &cond("fault.priority", "in_list", json!(["high", "urgent"])),
            &data
        ));
        assert!(condition_matches(
            &cond("fault.priority", "not_in_list", json!("low, medium")),
            &data
        ));
        // Missing fields only satisfy negative comparisons.
        assert!(!condition_matches(
            &cond("unit.id", "equals", json!("x")),
            &data
        ));
        assert!(condition_matches(
            &cond("unit.id", "not_equals", json!("x")),
            &data
        ));
        assert!(!condition_matches(
            &cond("fault.priority", "bogus", json!("x")),
            &data
        ));
    }

    #[test]
    fn filters_match_entity_fields() {
        let data = json!({ "fault": { "priority": "high", "category": "plumbing" } });
        assert!(filters_match(
            &json!({ "event_type": "fault_created" }),
            &data
        ));
        assert!(filters_match(
            &json!({ "filters": { "priority": "high" } }),
            &data
        ));
        assert!(filters_match(
            &json!({ "conditions": { "fault.category": "plumbing" } }),
            &data
        ));
        assert!(!filters_match(
            &json!({ "filters": { "priority": "low" } }),
            &data
        ));
        assert!(filters_match(
            &json!({ "filters": { "priority": ["high", "urgent"] } }),
            &data
        ));
    }

    #[test]
    fn actions_are_ordered() {
        let actions = json!([
            { "type": "send_email", "order": 2 },
            { "type": "send_notification", "order": 0 },
            { "type": "delay", "config": { "duration": 5 } },
        ]);
        let parsed = parse_actions(&actions);
        let types: Vec<&str> = parsed.iter().map(|a| a.action_type.as_str()).collect();
        assert_eq!(types, ["send_notification", "send_email", "delay"]);
    }

    #[test]
    fn delays_are_bounded() {
        assert_eq!(
            delay_seconds(&json!({ "duration": 2, "unit": "hours" })),
            Ok(7200)
        );
        assert_eq!(delay_seconds(&json!({ "duration": 5 })), Ok(300));
        assert!(delay_seconds(&json!({ "duration": 0 })).is_err());
        assert!(delay_seconds(&json!({ "duration": 31, "unit": "days" })).is_err());
        assert!(delay_seconds(&json!({ "duration": 1, "unit": "weeks" })).is_err());
    }

    #[test]
    fn recipient_tokens_split_lists() {
        assert_eq!(
            recipient_tokens(Some(&json!("managers; a@b.sk, "))),
            vec!["managers".to_string(), "a@b.sk".to_string()]
        );
        assert_eq!(
            recipient_tokens(Some(&json!(["reporter"]))),
            vec!["reporter".to_string()]
        );
        assert!(recipient_tokens(None).is_empty());
    }

    #[test]
    fn condition_entity_from_fields() {
        assert_eq!(
            condition_entity(&[cond("payment.daysOverdue", "greater_than", json!(1))]),
            Some(ConditionEntity::Payment)
        );
        assert_eq!(
            condition_entity(&[cond("fault.priority", "equals", json!("high"))]),
            Some(ConditionEntity::Fault)
        );
        assert_eq!(
            condition_entity(&[cond("user.role", "equals", json!("x"))]),
            None
        );
    }

    #[test]
    fn rule_validation() {
        let ok = json!([{ "type": "send_email", "config": { "to": "managers" } }]);
        assert!(validate_rule("schedule", &json!({ "cron": "0 9 * * 1" }), &ok).is_ok());
        assert!(validate_rule("schedule", &json!({ "cron": "nope" }), &ok).is_err());
        assert!(validate_rule(
            "schedule",
            &json!({ "cron": "0 9 * * 1", "timezone": "Mars/Base" }),
            &ok
        )
        .is_err());
        assert!(validate_rule("event", &json!({}), &ok).is_err());
        assert!(validate_rule("manual", &json!({}), &json!([{ "type": "add_tag" }])).is_err());
        assert!(validate_rule(
            "manual",
            &json!({}),
            &json!([{ "type": "webhook", "config": { "url": "http://x" } }])
        )
        .is_err());
        assert!(validate_rule("manual", &json!({}), &json!([{ "config": {} }])).is_err());
        assert!(validate_rule(
            "manual",
            &json!({}),
            &json!([{ "type": "generate_report", "config": { "report": "faults" } }])
        )
        .is_ok());
    }

    #[test]
    fn schedule_rules_compute_next_run() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-11-25T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let next =
            next_schedule_run(&json!({ "cron": "0 9 * * 1", "timezone": "UTC" }), now).unwrap();
        assert_eq!(next.to_rfc3339(), "2026-11-30T09:00:00+00:00");
        // Default zone is Europe/Bratislava (UTC+1 in winter).
        let next = next_schedule_run(&json!({ "cron": "0 9 * * *" }), now).unwrap();
        assert_eq!(next.to_rfc3339(), "2026-11-26T08:00:00+00:00");
        assert!(next_schedule_run(&json!({}), now).is_none());
    }
}

#[cfg(test)]
mod db_tests {
    use super::*;
    use crate::services::notification::NotificationServiceConfig;

    async fn seed(pool: &sqlx::PgPool) -> (Uuid, Uuid, Uuid) {
        let org: Uuid = sqlx::query_scalar(
            "INSERT INTO organizations (name, slug, contact_email, status) \
             VALUES ('Auto Org', 'auto-org', 'auto@org.test', 'active') RETURNING id",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        let manager: Uuid = sqlx::query_scalar(
            "INSERT INTO users (email, password_hash, name, status, email_verified_at) \
             VALUES ('manager@auto.test', 'x', 'Manager', 'active', NOW()) RETURNING id",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO organization_members (organization_id, user_id, role_type, status) \
             VALUES ($1, $2, 'manager', 'active')",
        )
        .bind(org)
        .bind(manager)
        .execute(pool)
        .await
        .unwrap();
        let building: Uuid = sqlx::query_scalar(
            "INSERT INTO buildings (organization_id, street, city, postal_code, country) \
             VALUES ($1, 'Main 1', 'Bratislava', '81101', 'Slovakia') RETURNING id",
        )
        .bind(org)
        .fetch_one(pool)
        .await
        .unwrap();
        (org, manager, building)
    }

    async fn insert_rule(
        pool: &sqlx::PgPool,
        org: Uuid,
        owner: Uuid,
        trigger_type: &str,
        trigger_config: Value,
        conditions: Option<Value>,
        actions: Value,
    ) -> Uuid {
        sqlx::query_scalar(
            "INSERT INTO workflow_automation_rules \
             (organization_id, name, trigger_type, trigger_config, conditions, actions, created_by) \
             VALUES ($1, 'Rule', $2::workflow_automation_trigger, $3, $4, $5, $6) RETURNING id",
        )
        .bind(org)
        .bind(trigger_type)
        .bind(trigger_config)
        .bind(conditions)
        .bind(actions)
        .bind(owner)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    fn engine(pool: &sqlx::PgPool) -> AutomationEngine {
        let email = EmailService::development();
        let notifications = Arc::new(NotificationService::new(
            pool.clone(),
            email.clone(),
            NotificationServiceConfig::default(),
        ));
        AutomationEngine::new(pool.clone(), email, notifications, None)
    }

    async fn logs(pool: &sqlx::PgPool, rule: Uuid) -> Vec<(String, Value)> {
        sqlx::query_as(
            "SELECT status, actions_executed FROM workflow_automation_logs \
             WHERE rule_id = $1 ORDER BY started_at",
        )
        .bind(rule)
        .fetch_all(pool)
        .await
        .unwrap()
    }

    async fn seed_fault(
        pool: &sqlx::PgPool,
        org: Uuid,
        building: Uuid,
        reporter: Uuid,
        priority: &str,
    ) -> Uuid {
        sqlx::query_scalar(
            "INSERT INTO faults (organization_id, building_id, reporter_id, title, description, \
             category, priority, status) \
             VALUES ($1, $2, $3, 'Leaking pipe', 'Water everywhere', 'plumbing', \
             $4::fault_priority, 'new') RETURNING id",
        )
        .bind(org)
        .bind(building)
        .bind(reporter)
        .bind(priority)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn event_rules_respect_filters_and_log_runs(pool: sqlx::PgPool) {
        let (org, manager, _building) = seed(&pool).await;
        let rule = insert_rule(
            &pool,
            org,
            manager,
            "event",
            json!({ "event_type": "fault.created", "filters": { "priority": "high" } }),
            Some(json!([{ "field": "fault.category", "operator": "equals", "value": "plumbing" }])),
            json!([
                { "type": "send_notification", "config": { "title": "New: {{fault.title}}", "recipients": ["managers"] } },
                { "type": "send_email", "config": { "to": "managers", "subject": "Fault {{fault.title}}" } },
                { "type": "add_tag", "config": {} },
            ]),
        )
        .await;
        let engine = engine(&pool);

        let low =
            json!({ "fault": { "id": Uuid::new_v4(), "priority": "low", "category": "plumbing" } });
        assert_eq!(
            engine
                .handle_event(org, event::FAULT_CREATED, low)
                .await
                .unwrap(),
            0
        );

        let high = json!({ "fault": { "id": Uuid::new_v4(), "title": "Leak", "priority": "high", "category": "plumbing" } });
        assert_eq!(
            engine
                .handle_event(org, event::FAULT_CREATED, high)
                .await
                .unwrap(),
            1
        );

        let runs = logs(&pool, rule).await;
        assert_eq!(runs.len(), 1);
        let (status, actions) = &runs[0];
        assert_eq!(
            status, "partial_failure",
            "add_tag is unsupported: {actions}"
        );
        assert_eq!(actions[0]["status"], "completed");
        assert_eq!(actions[0]["output"]["recipients"], 1);
        assert_eq!(actions[1]["output"]["sent_to"][0], "manager@auto.test");
        assert_eq!(actions[2]["status"], "failed");

        let (run_count, error_count): (i32, i32) = sqlx::query_as(
            "SELECT run_count, error_count FROM workflow_automation_rules WHERE id = $1",
        )
        .bind(rule)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!((run_count, error_count), (1, 1));
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn schedule_rules_are_scheduled_then_run(pool: sqlx::PgPool) {
        let (org, manager, _) = seed(&pool).await;
        let rule = insert_rule(
            &pool,
            org,
            manager,
            "schedule",
            json!({ "cron": "0 9 * * 1", "timezone": "Europe/Bratislava" }),
            None,
            json!([{ "type": "send_notification", "config": { "title": "Weekly" } }]),
        )
        .await;
        let engine = engine(&pool);

        // First pass only computes next_run_at.
        assert_eq!(engine.run_due_schedules().await.unwrap(), 0);
        let next: Option<chrono::DateTime<Utc>> =
            sqlx::query_scalar("SELECT next_run_at FROM workflow_automation_rules WHERE id = $1")
                .bind(rule)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(next.unwrap() > Utc::now());
        assert!(logs(&pool, rule).await.is_empty());

        // Once due it runs and is pushed to the next occurrence.
        sqlx::query("UPDATE workflow_automation_rules SET next_run_at = NOW() - INTERVAL '1 minute' WHERE id = $1")
            .bind(rule)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(engine.run_due_schedules().await.unwrap(), 1);
        assert_eq!(logs(&pool, rule).await[0].0, "success");
        assert_eq!(engine.run_due_schedules().await.unwrap(), 0);
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn condition_rules_fire_once_per_record(pool: sqlx::PgPool) {
        let (org, manager, building) = seed(&pool).await;
        seed_fault(&pool, org, building, manager, "high").await;
        seed_fault(&pool, org, building, manager, "low").await;
        let rule = insert_rule(
            &pool,
            org,
            manager,
            "condition",
            json!({}),
            Some(json!([{ "field": "fault.priority", "operator": "in_list", "value": ["high", "urgent"] }])),
            json!([{ "type": "send_notification", "config": { "title": "{{fault.title}}", "recipients": ["reporter"] } }]),
        )
        .await;
        let engine = engine(&pool);

        assert_eq!(engine.run_condition_rules().await.unwrap(), 1);
        assert_eq!(engine.run_condition_rules().await.unwrap(), 0, "deduped");
        let runs = logs(&pool, rule).await;
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].0, "success");
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn delay_parks_remaining_actions_until_due(pool: sqlx::PgPool) {
        let (org, manager, building) = seed(&pool).await;
        let fault = seed_fault(&pool, org, building, manager, "medium").await;
        let rule = insert_rule(
            &pool,
            org,
            manager,
            "manual",
            json!({}),
            None,
            json!([
                { "type": "delay", "config": { "duration": 2, "unit": "hours" } },
                { "type": "update_status", "config": { "status": "triaged" } },
            ]),
        )
        .await;
        let engine = engine(&pool);
        let rule_row = engine.find_rule(rule, org).await.unwrap().unwrap();

        let summary = engine
            .run_rule(&rule_row, json!({ "fault": { "id": fault } }), 0)
            .await
            .unwrap();
        assert_eq!(summary.status, "success");
        assert_eq!(
            summary.actions.len(),
            1,
            "update_status waits for the delay"
        );

        // Not due yet.
        assert_eq!(engine.run_continuations(10).await.unwrap(), 0);
        sqlx::query("UPDATE background_jobs SET scheduled_at = NOW() WHERE queue = 'automation'")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(engine.run_continuations(10).await.unwrap(), 1);

        let status: String = sqlx::query_scalar("SELECT status::text FROM faults WHERE id = $1")
            .bind(fault)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status, "triaged");
        assert_eq!(logs(&pool, rule).await.len(), 2);
    }
}
