//! Workflow Execution Engine (Epic 97, Story 97.3).
//!
//! Executes workflow actions in sequence with retry and failure handling.
//!
//! Supported action types:
//! - send_email: Send email via configured provider
//! - send_notification: Push notification to users
//! - create_fault: Create a new fault report
//! - assign_task: Assign task to a user
//! - webhook: Call external HTTP endpoint
//! - llm_response: Generate AI response using LLM

use crate::{ChatCompletionRequest, ChatMessage, LlmClient, LlmError};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use thiserror::Error;
use tokio::time::sleep;
use uuid::Uuid;

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during workflow execution.
#[derive(Debug, Error)]
pub enum WorkflowExecutionError {
    #[error("Action type not supported: {0}")]
    UnsupportedActionType(String),

    #[error("Invalid action configuration: {0}")]
    InvalidConfig(String),

    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("LLM call failed: {0}")]
    LlmError(#[from] LlmError),

    #[error("Action execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Retry limit exceeded after {0} attempts")]
    RetryLimitExceeded(i32),

    #[error("Action timed out")]
    Timeout,
}

// =============================================================================
// Action Types
// =============================================================================

/// Supported action types in workflows.
pub mod action_type {
    pub const SEND_EMAIL: &str = "send_email";
    pub const SEND_NOTIFICATION: &str = "send_notification";
    pub const SEND_SMS: &str = "send_sms";
    pub const CREATE_TASK: &str = "create_task";
    pub const UPDATE_STATUS: &str = "update_status";
    pub const ASSIGN_TO_USER: &str = "assign_to_user";
    pub const CREATE_ANNOUNCEMENT: &str = "create_announcement";
    pub const CREATE_FAULT: &str = "create_fault";
    pub const CALL_WEBHOOK: &str = "call_webhook";
    pub const DELAY: &str = "delay";
    pub const LLM_RESPONSE: &str = "llm_response";
}

// =============================================================================
// Action Configuration Types
// =============================================================================

/// Email action configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendEmailConfig {
    pub to: Vec<String>,
    pub cc: Option<Vec<String>>,
    pub bcc: Option<Vec<String>>,
    pub subject: String,
    pub body: String,
    pub html_body: Option<String>,
    pub template_id: Option<String>,
    pub template_data: Option<Value>,
}

/// Notification action configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendNotificationConfig {
    pub user_ids: Vec<Uuid>,
    pub title: String,
    pub body: String,
    pub data: Option<Value>,
    pub priority: Option<String>,
}

/// Webhook action configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub url: String,
    pub method: Option<String>,
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub body: Option<Value>,
    pub timeout_seconds: Option<u64>,
}

/// Create fault action configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFaultConfig {
    pub building_id: Uuid,
    pub unit_id: Option<Uuid>,
    pub title: String,
    pub description: String,
    pub category: String,
    pub priority: Option<String>,
    pub assigned_to: Option<Uuid>,
}

/// Assign task action configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignTaskConfig {
    pub user_id: Uuid,
    pub task_type: String,
    pub title: String,
    pub description: Option<String>,
    pub due_date: Option<String>,
    pub priority: Option<String>,
    pub related_entity_id: Option<Uuid>,
    pub related_entity_type: Option<String>,
}

/// LLM response action configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponseConfig {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub system_prompt: String,
    pub user_prompt: String,
    pub max_tokens: Option<i32>,
    pub temperature: Option<f32>,
    pub output_variable: Option<String>,
}

/// Delay action configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelayConfig {
    pub seconds: u64,
}

// =============================================================================
// Action Result
// =============================================================================

/// Result of executing an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub success: bool,
    pub output: Value,
    pub error_message: Option<String>,
    pub duration_ms: i32,
    pub retry_attempt: i32,
}

impl ActionResult {
    pub fn success(output: Value, duration_ms: i32) -> Self {
        Self {
            success: true,
            output,
            error_message: None,
            duration_ms,
            retry_attempt: 0,
        }
    }

    pub fn failure(error: String, duration_ms: i32, retry_attempt: i32) -> Self {
        Self {
            success: false,
            output: Value::Null,
            error_message: Some(error),
            duration_ms,
            retry_attempt,
        }
    }
}

// =============================================================================
// Workflow Executor
// =============================================================================

/// Workflow execution engine.
///
/// Executes workflow actions with:
/// - Sequential action processing
/// - Retry logic with exponential backoff
/// - Failure handling (stop, continue, retry)
/// - Context variable interpolation
#[derive(Clone)]
pub struct WorkflowExecutor {
    http_client: Client,
    llm_client: Option<LlmClient>,
    /// Webhook for action callbacks (to notify external systems)
    action_callback_url: Option<String>,
}

impl WorkflowExecutor {
    /// Create a new workflow executor.
    pub fn new(llm_client: Option<LlmClient>) -> Self {
        Self {
            http_client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            llm_client,
            action_callback_url: std::env::var("WORKFLOW_ACTION_CALLBACK_URL").ok(),
        }
    }

    /// Execute a single action with retry logic.
    pub async fn execute_action(
        &self,
        action_type: &str,
        action_config: &Value,
        context: &Value,
        retry_count: i32,
        retry_delay_seconds: i32,
    ) -> Result<ActionResult, WorkflowExecutionError> {
        let mut last_error = None;
        let mut retry_attempt = 0;

        while retry_attempt <= retry_count {
            let start = std::time::Instant::now();

            match self
                .execute_action_internal(action_type, action_config, context)
                .await
            {
                Ok(output) => {
                    let duration_ms = start.elapsed().as_millis() as i32;
                    return Ok(ActionResult {
                        success: true,
                        output,
                        error_message: None,
                        duration_ms,
                        retry_attempt,
                    });
                }
                Err(e) => {
                    let _duration_ms = start.elapsed().as_millis() as i32;
                    tracing::warn!(
                        action_type = action_type,
                        attempt = retry_attempt + 1,
                        max_attempts = retry_count + 1,
                        error = %e,
                        "Action failed"
                    );

                    last_error = Some(e);

                    if retry_attempt < retry_count {
                        // Exponential backoff
                        let delay = retry_delay_seconds as u64 * (2_u64.pow(retry_attempt as u32));
                        tracing::info!("Retrying in {} seconds...", delay);
                        sleep(Duration::from_secs(delay)).await;
                    }

                    retry_attempt += 1;
                }
            }
        }

        // All retries exhausted
        let err = last_error.unwrap_or(WorkflowExecutionError::RetryLimitExceeded(retry_count));
        Err(WorkflowExecutionError::ExecutionFailed(format!(
            "Failed after {} retries: {}",
            retry_count, err
        )))
    }

    /// Internal action execution dispatch.
    async fn execute_action_internal(
        &self,
        action_type: &str,
        config: &Value,
        context: &Value,
    ) -> Result<Value, WorkflowExecutionError> {
        // Interpolate context variables in config
        let interpolated_config = interpolate_variables(config, context);

        match action_type {
            action_type::SEND_EMAIL => self.execute_send_email(&interpolated_config).await,
            action_type::SEND_NOTIFICATION => {
                self.execute_send_notification(&interpolated_config).await
            }
            action_type::CALL_WEBHOOK => self.execute_webhook(&interpolated_config).await,
            action_type::CREATE_FAULT => self.execute_create_fault(&interpolated_config).await,
            action_type::ASSIGN_TO_USER => self.execute_assign_task(&interpolated_config).await,
            action_type::LLM_RESPONSE => self.execute_llm_response(&interpolated_config).await,
            action_type::DELAY => self.execute_delay(&interpolated_config).await,
            action_type::CREATE_TASK => self.execute_create_task(&interpolated_config).await,
            action_type::SEND_SMS => self.execute_send_sms(&interpolated_config).await,
            _ => Err(WorkflowExecutionError::UnsupportedActionType(
                action_type.to_string(),
            )),
        }
    }

    // =========================================================================
    // Action Implementations
    // =========================================================================

    /// Execute send email action.
    async fn execute_send_email(&self, config: &Value) -> Result<Value, WorkflowExecutionError> {
        let email_config: SendEmailConfig =
            serde_json::from_value(config.clone()).map_err(|e| {
                WorkflowExecutionError::InvalidConfig(format!("Invalid email config: {}", e))
            })?;

        // In production, this would call an email service (SendGrid, SES, etc.)
        // For now, log and return success
        tracing::info!(
            "Sending email to {:?} with subject: {}",
            email_config.to,
            email_config.subject
        );

        // If there's a callback URL configured, notify it
        if let Some(ref callback_url) = self.action_callback_url {
            let _ = self
                .http_client
                .post(format!("{}/email", callback_url))
                .json(&serde_json::json!({
                    "action": "send_email",
                    "to": email_config.to,
                    "subject": email_config.subject,
                    "status": "sent"
                }))
                .send()
                .await;
        }

        Ok(serde_json::json!({
            "sent": true,
            "recipients": email_config.to.len(),
            "subject": email_config.subject
        }))
    }

    /// Execute send notification action.
    async fn execute_send_notification(
        &self,
        config: &Value,
    ) -> Result<Value, WorkflowExecutionError> {
        let notif_config: SendNotificationConfig =
            serde_json::from_value(config.clone()).map_err(|e| {
                WorkflowExecutionError::InvalidConfig(format!("Invalid notification config: {}", e))
            })?;

        tracing::info!(
            "Sending notification to {} users: {}",
            notif_config.user_ids.len(),
            notif_config.title
        );

        // Notify callback if configured
        if let Some(ref callback_url) = self.action_callback_url {
            let _ = self
                .http_client
                .post(format!("{}/notification", callback_url))
                .json(&serde_json::json!({
                    "action": "send_notification",
                    "user_ids": notif_config.user_ids,
                    "title": notif_config.title,
                    "status": "sent"
                }))
                .send()
                .await;
        }

        Ok(serde_json::json!({
            "sent": true,
            "recipients": notif_config.user_ids.len(),
            "title": notif_config.title
        }))
    }

    /// Execute webhook action.
    async fn execute_webhook(&self, config: &Value) -> Result<Value, WorkflowExecutionError> {
        let webhook_config: WebhookConfig =
            serde_json::from_value(config.clone()).map_err(|e| {
                WorkflowExecutionError::InvalidConfig(format!("Invalid webhook config: {}", e))
            })?;

        // SSRF guard (#835): a workflow's webhook URL is attacker-influenced
        // config, so validate it before issuing the request — reject private/
        // reserved IPs, localhost, link-local (cloud metadata), .local/.internal
        // and non-http(s) schemes. Mirrors routes/integrations/webhook.rs.
        common::url_validation::validate_external_url(&webhook_config.url).map_err(|e| {
            WorkflowExecutionError::InvalidConfig(format!("Webhook URL rejected: {}", e))
        })?;

        let method = webhook_config
            .method
            .unwrap_or_else(|| "POST".to_string())
            .to_uppercase();
        let timeout = webhook_config.timeout_seconds.unwrap_or(30);

        let client = Client::builder()
            .timeout(Duration::from_secs(timeout))
            .build()
            .map_err(|e| WorkflowExecutionError::ExecutionFailed(e.to_string()))?;

        let mut request = match method.as_str() {
            "GET" => client.get(&webhook_config.url),
            "POST" => client.post(&webhook_config.url),
            "PUT" => client.put(&webhook_config.url),
            "DELETE" => client.delete(&webhook_config.url),
            "PATCH" => client.patch(&webhook_config.url),
            _ => {
                return Err(WorkflowExecutionError::InvalidConfig(format!(
                    "Unsupported HTTP method: {}",
                    method
                )))
            }
        };

        // Add custom headers
        if let Some(headers) = webhook_config.headers {
            for (key, value) in headers {
                request = request.header(&key, &value);
            }
        }

        // Add body for methods that support it
        if let Some(body) = webhook_config.body {
            request = request.json(&body);
        }

        let response = request.send().await?;
        let status = response.status();
        let response_body: Value = response.json().await.unwrap_or(Value::Null);

        if status.is_success() {
            Ok(serde_json::json!({
                "status_code": status.as_u16(),
                "response": response_body
            }))
        } else {
            Err(WorkflowExecutionError::ExecutionFailed(format!(
                "Webhook returned error status {}: {:?}",
                status, response_body
            )))
        }
    }

    /// Execute create fault action.
    async fn execute_create_fault(&self, config: &Value) -> Result<Value, WorkflowExecutionError> {
        let fault_config: CreateFaultConfig =
            serde_json::from_value(config.clone()).map_err(|e| {
                WorkflowExecutionError::InvalidConfig(format!("Invalid fault config: {}", e))
            })?;

        tracing::info!(
            "Creating fault: {} in building {}",
            fault_config.title,
            fault_config.building_id
        );

        // Notify callback if configured
        if let Some(ref callback_url) = self.action_callback_url {
            let _ = self
                .http_client
                .post(format!("{}/fault", callback_url))
                .json(&serde_json::json!({
                    "action": "create_fault",
                    "building_id": fault_config.building_id,
                    "title": fault_config.title,
                    "category": fault_config.category,
                    "status": "created"
                }))
                .send()
                .await;
        }

        // Return a mock fault ID (in production, this would be created via the fault repository)
        Ok(serde_json::json!({
            "created": true,
            "fault_id": Uuid::new_v4(),
            "title": fault_config.title,
            "building_id": fault_config.building_id
        }))
    }

    /// Execute assign task action.
    async fn execute_assign_task(&self, config: &Value) -> Result<Value, WorkflowExecutionError> {
        let task_config: AssignTaskConfig =
            serde_json::from_value(config.clone()).map_err(|e| {
                WorkflowExecutionError::InvalidConfig(format!("Invalid task config: {}", e))
            })?;

        tracing::info!(
            "Assigning task '{}' to user {}",
            task_config.title,
            task_config.user_id
        );

        // Notify callback if configured
        if let Some(ref callback_url) = self.action_callback_url {
            let _ = self
                .http_client
                .post(format!("{}/task", callback_url))
                .json(&serde_json::json!({
                    "action": "assign_task",
                    "user_id": task_config.user_id,
                    "title": task_config.title,
                    "status": "assigned"
                }))
                .send()
                .await;
        }

        Ok(serde_json::json!({
            "assigned": true,
            "task_id": Uuid::new_v4(),
            "user_id": task_config.user_id,
            "title": task_config.title
        }))
    }

    /// Execute create task action.
    async fn execute_create_task(&self, config: &Value) -> Result<Value, WorkflowExecutionError> {
        // Similar to assign task but without specific user assignment
        let task_config: AssignTaskConfig =
            serde_json::from_value(config.clone()).map_err(|e| {
                WorkflowExecutionError::InvalidConfig(format!("Invalid task config: {}", e))
            })?;

        tracing::info!("Creating task: {}", task_config.title);

        Ok(serde_json::json!({
            "created": true,
            "task_id": Uuid::new_v4(),
            "title": task_config.title
        }))
    }

    /// Execute LLM response action.
    async fn execute_llm_response(&self, config: &Value) -> Result<Value, WorkflowExecutionError> {
        let llm_config: LlmResponseConfig =
            serde_json::from_value(config.clone()).map_err(|e| {
                WorkflowExecutionError::InvalidConfig(format!("Invalid LLM config: {}", e))
            })?;

        let llm_client = self.llm_client.as_ref().ok_or_else(|| {
            WorkflowExecutionError::ExecutionFailed("LLM client not configured".to_string())
        })?;

        let provider = llm_config
            .provider
            .unwrap_or_else(|| "anthropic".to_string());
        let model = llm_config.model.unwrap_or_else(|| match provider.as_str() {
            "openai" => "gpt-4o-mini".to_string(),
            _ => "claude-3-5-haiku-20241022".to_string(),
        });

        let request = ChatCompletionRequest {
            model,
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: llm_config.system_prompt,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: llm_config.user_prompt,
                },
            ],
            temperature: llm_config.temperature,
            max_tokens: llm_config.max_tokens,
        };

        let response = llm_client.chat(&provider, &request).await?;
        let content = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(serde_json::json!({
            "response": content,
            "tokens_used": response.usage.total_tokens,
            "model": request.model
        }))
    }

    /// Execute delay action.
    async fn execute_delay(&self, config: &Value) -> Result<Value, WorkflowExecutionError> {
        let delay_config: DelayConfig = serde_json::from_value(config.clone()).map_err(|e| {
            WorkflowExecutionError::InvalidConfig(format!("Invalid delay config: {}", e))
        })?;

        tracing::info!("Delaying for {} seconds", delay_config.seconds);
        sleep(Duration::from_secs(delay_config.seconds)).await;

        Ok(serde_json::json!({
            "delayed": true,
            "seconds": delay_config.seconds
        }))
    }

    /// Execute send SMS action.
    async fn execute_send_sms(&self, config: &Value) -> Result<Value, WorkflowExecutionError> {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct SmsConfig {
            phone_numbers: Vec<String>,
            message: String,
        }

        let sms_config: SmsConfig = serde_json::from_value(config.clone()).map_err(|e| {
            WorkflowExecutionError::InvalidConfig(format!("Invalid SMS config: {}", e))
        })?;

        tracing::info!(
            "Sending SMS to {} numbers: {}",
            sms_config.phone_numbers.len(),
            sms_config.message
        );

        // Notify callback if configured
        if let Some(ref callback_url) = self.action_callback_url {
            let _ = self
                .http_client
                .post(format!("{}/sms", callback_url))
                .json(&serde_json::json!({
                    "action": "send_sms",
                    "phone_numbers": sms_config.phone_numbers,
                    "message": sms_config.message,
                    "status": "sent"
                }))
                .send()
                .await;
        }

        Ok(serde_json::json!({
            "sent": true,
            "recipients": sms_config.phone_numbers.len()
        }))
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Interpolate context variables in a JSON value.
/// Replaces {{variable}} patterns with values from context.
fn interpolate_variables(value: &Value, context: &Value) -> Value {
    match value {
        Value::String(s) => {
            let mut result = s.clone();

            // Find all {{variable}} patterns
            let re = regex_lite::Regex::new(r"\{\{([^}]+)\}\}").unwrap();
            for cap in re.captures_iter(s) {
                let full_match = cap.get(0).unwrap().as_str();
                let var_name = cap.get(1).unwrap().as_str().trim();

                // Look up the variable in context (supports dot notation)
                if let Some(replacement) = get_nested_value(context, var_name) {
                    let replacement_str = match replacement {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => replacement.to_string(),
                    };
                    result = result.replace(full_match, &replacement_str);
                }
            }

            Value::String(result)
        }
        Value::Array(arr) => Value::Array(
            arr.iter()
                .map(|v| interpolate_variables(v, context))
                .collect(),
        ),
        Value::Object(obj) => {
            let mut new_obj = serde_json::Map::new();
            for (k, v) in obj {
                new_obj.insert(k.clone(), interpolate_variables(v, context));
            }
            Value::Object(new_obj)
        }
        _ => value.clone(),
    }
}

/// Get a nested value from JSON using dot notation (e.g., "event.fault.id").
fn get_nested_value<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value;

    for part in parts {
        match current {
            Value::Object(obj) => {
                current = obj.get(part)?;
            }
            Value::Array(arr) => {
                let index: usize = part.parse().ok()?;
                current = arr.get(index)?;
            }
            _ => return None,
        }
    }

    Some(current)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate_variables_simple() {
        let config = serde_json::json!({
            "subject": "New fault: {{fault.title}}",
            "body": "A fault has been reported by {{user.name}}"
        });

        let context = serde_json::json!({
            "fault": {
                "title": "Broken elevator",
                "id": "123"
            },
            "user": {
                "name": "John Doe"
            }
        });

        let result = interpolate_variables(&config, &context);

        assert_eq!(result["subject"], "New fault: Broken elevator");
        assert_eq!(result["body"], "A fault has been reported by John Doe");
    }

    #[test]
    fn test_interpolate_variables_missing() {
        let config = serde_json::json!({
            "message": "Hello {{name}}, your ID is {{missing}}"
        });

        let context = serde_json::json!({
            "name": "Alice"
        });

        let result = interpolate_variables(&config, &context);

        // Missing variables should not be replaced
        assert_eq!(result["message"], "Hello Alice, your ID is {{missing}}");
    }

    #[test]
    fn test_get_nested_value() {
        let data = serde_json::json!({
            "level1": {
                "level2": {
                    "value": "found"
                },
                "array": [1, 2, 3]
            }
        });

        assert_eq!(
            get_nested_value(&data, "level1.level2.value"),
            Some(&Value::String("found".to_string()))
        );
        assert_eq!(
            get_nested_value(&data, "level1.array.0"),
            Some(&Value::Number(1.into()))
        );
        assert_eq!(get_nested_value(&data, "missing.path"), None);
    }

    #[test]
    fn test_action_result_success() {
        let result = ActionResult::success(serde_json::json!({"sent": true}), 150);

        assert!(result.success);
        assert_eq!(result.duration_ms, 150);
        assert!(result.error_message.is_none());
    }

    #[test]
    fn test_action_result_failure() {
        let result = ActionResult::failure("Connection timeout".to_string(), 5000, 2);

        assert!(!result.success);
        assert_eq!(result.duration_ms, 5000);
        assert_eq!(result.error_message, Some("Connection timeout".to_string()));
        assert_eq!(result.retry_attempt, 2);
    }

    #[test]
    fn test_email_config_parsing() {
        let config = serde_json::json!({
            "to": ["user@example.com"],
            "subject": "Test Email",
            "body": "Hello World"
        });

        let email_config: SendEmailConfig = serde_json::from_value(config).unwrap();
        assert_eq!(email_config.to, vec!["user@example.com"]);
        assert_eq!(email_config.subject, "Test Email");
    }

    #[test]
    fn test_webhook_config_parsing() {
        let config = serde_json::json!({
            "url": "https://api.example.com/webhook",
            "method": "POST",
            "body": {"event": "test"},
            "timeout_seconds": 10
        });

        let webhook_config: WebhookConfig = serde_json::from_value(config).unwrap();
        assert_eq!(webhook_config.url, "https://api.example.com/webhook");
        assert_eq!(webhook_config.method, Some("POST".to_string()));
        assert_eq!(webhook_config.timeout_seconds, Some(10));
    }
}
