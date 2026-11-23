//! Workflow action executors (Epic 94, Story 94.1).
//!
//! Each action type has a dedicated executor that handles the specific
//! logic for that action type.

use crate::services::notification::NotificationService;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

pub mod api_call;
pub mod delay;
pub mod email;
pub mod notification;

pub use api_call::ApiCallExecutor;
pub use delay::DelayExecutor;
pub use email::EmailExecutor;
pub use notification::NotificationExecutor;

/// Errors that can occur during action execution.
#[derive(Debug, Error)]
pub enum ActionError {
    #[error("Action configuration error: {0}")]
    ConfigurationError(String),

    #[error("Action execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Action timed out after {0} seconds")]
    Timeout(u64),

    #[error("Retry limit exceeded after {0} attempts")]
    RetryLimitExceeded(i32),

    #[error("Invalid action type: {0}")]
    InvalidActionType(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("External service error: {0}")]
    ExternalServiceError(String),
}

/// Result of an action execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    /// Whether the action succeeded
    pub success: bool,
    /// Output data from the action (can be used by subsequent actions)
    pub output: serde_json::Value,
    /// Error message if the action failed
    pub error: Option<String>,
    /// Duration of the action in milliseconds
    pub duration_ms: i32,
    /// Whether this was a retry attempt
    pub retry_attempt: i32,
}

impl ActionResult {
    /// Create a successful result.
    pub fn success(output: serde_json::Value, duration_ms: i32) -> Self {
        Self {
            success: true,
            output,
            error: None,
            duration_ms,
            retry_attempt: 0,
        }
    }

    /// Create a failed result.
    pub fn failure(error: String, duration_ms: i32) -> Self {
        Self {
            success: false,
            output: serde_json::json!({}),
            error: Some(error),
            duration_ms,
            retry_attempt: 0,
        }
    }

    /// Set the retry attempt number.
    pub fn with_retry(mut self, attempt: i32) -> Self {
        self.retry_attempt = attempt;
        self
    }
}

/// Context passed to action executors.
#[derive(Debug, Clone)]
pub struct ActionContext {
    /// Organization ID
    pub organization_id: uuid::Uuid,
    /// Workflow ID
    pub workflow_id: uuid::Uuid,
    /// Execution ID
    pub execution_id: uuid::Uuid,
    /// Trigger event data
    pub trigger_event: serde_json::Value,
    /// Accumulated context from previous actions
    pub context: HashMap<String, serde_json::Value>,
    /// Variables available for template substitution
    pub variables: HashMap<String, String>,
}

impl ActionContext {
    /// Create a new action context.
    pub fn new(
        organization_id: uuid::Uuid,
        workflow_id: uuid::Uuid,
        execution_id: uuid::Uuid,
        trigger_event: serde_json::Value,
    ) -> Self {
        Self {
            organization_id,
            workflow_id,
            execution_id,
            trigger_event,
            context: HashMap::new(),
            variables: HashMap::new(),
        }
    }

    /// Add output from a completed action.
    pub fn add_action_output(&mut self, action_order: i32, output: serde_json::Value) {
        self.context
            .insert(format!("action_{}", action_order), output);
    }

    /// Get a variable value.
    pub fn get_variable(&self, name: &str) -> Option<&String> {
        self.variables.get(name)
    }

    /// Set a variable value.
    pub fn set_variable(&mut self, name: String, value: String) {
        self.variables.insert(name, value);
    }

    /// Substitute variables in a template string.
    /// Variables are referenced as {{variable_name}}
    pub fn substitute_template(&self, template: &str) -> String {
        let mut result = template.to_string();

        // Substitute explicit variables
        for (key, value) in &self.variables {
            result = result.replace(&format!("{{{{{}}}}}", key), value);
        }

        // Substitute trigger event fields (e.g., {{trigger.field}})
        if let Some(obj) = self.trigger_event.as_object() {
            for (key, value) in obj {
                let placeholder = format!("{{{{trigger.{}}}}}", key);
                let replacement = match value {
                    serde_json::Value::String(s) => s.clone(),
                    v => v.to_string(),
                };
                result = result.replace(&placeholder, &replacement);
            }
        }

        result
    }
}

/// Trait for action executors.
#[async_trait]
pub trait ActionExecutor: Send + Sync {
    /// Execute the action.
    async fn execute(
        &self,
        config: &serde_json::Value,
        context: &ActionContext,
    ) -> Result<ActionResult, ActionError>;

    /// Validate the action configuration.
    fn validate_config(&self, config: &serde_json::Value) -> Result<(), ActionError>;

    /// Get the action type name.
    fn action_type(&self) -> &'static str;

    /// Get default timeout for this action type.
    fn default_timeout(&self) -> Duration {
        Duration::from_secs(30)
    }
}

/// Registry of action executors.
pub struct ActionRegistry {
    executors: HashMap<String, Box<dyn ActionExecutor>>,
}

impl ActionRegistry {
    /// Create a new registry with all built-in executors.
    ///
    /// The `send_notification` action uses the logging fallback sink (it logs
    /// the resolved notification without delivering). Production should build
    /// the registry via [`ActionRegistry::with_notification_service`] so the
    /// notification action actually dispatches — this mirrors the email action
    /// degrading to logging only when no mail service is configured.
    pub fn new() -> Self {
        Self::with_notification_sink(Arc::new(notification::LoggingNotificationSink))
    }

    /// Create a registry whose `send_notification` action delivers through the
    /// real [`NotificationService`] pipeline (persisting in-app notifications,
    /// honouring per-user channel preferences) instead of only logging.
    ///
    /// This is the production constructor: it wires a
    /// [`NotificationServiceSink`](notification::NotificationServiceSink) so the
    /// notification action is no longer a silent no-op.
    pub fn with_notification_service(notification_service: Arc<NotificationService>) -> Self {
        Self::with_notification_sink(Arc::new(notification::NotificationServiceSink::new(
            notification_service,
        )))
    }

    /// Shared builder: register every built-in executor, wiring the given sink
    /// into the notification action.
    fn with_notification_sink(sink: Arc<dyn notification::NotificationSink>) -> Self {
        let mut registry = Self {
            executors: HashMap::new(),
        };

        // Register built-in executors
        registry.register(Box::new(EmailExecutor::new()));
        registry.register(Box::new(NotificationExecutor::with_sink(sink)));
        registry.register(Box::new(ApiCallExecutor::new()));
        registry.register(Box::new(DelayExecutor::new()));

        registry
    }

    /// Register an action executor.
    pub fn register(&mut self, executor: Box<dyn ActionExecutor>) {
        self.executors
            .insert(executor.action_type().to_string(), executor);
    }

    /// Get an executor by action type.
    pub fn get(&self, action_type: &str) -> Option<&dyn ActionExecutor> {
        self.executors.get(action_type).map(|e| e.as_ref())
    }

    /// Check if an action type is supported.
    pub fn supports(&self, action_type: &str) -> bool {
        self.executors.contains_key(action_type)
    }

    /// Get all supported action types.
    pub fn supported_types(&self) -> Vec<&str> {
        self.executors.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for ActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_result_success() {
        let result = ActionResult::success(serde_json::json!({"key": "value"}), 100);
        assert!(result.success);
        assert!(result.error.is_none());
        assert_eq!(result.duration_ms, 100);
    }

    #[test]
    fn test_action_result_failure() {
        let result = ActionResult::failure("Something went wrong".to_string(), 50);
        assert!(!result.success);
        assert_eq!(result.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_action_context_substitute_template() {
        let mut context = ActionContext::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            serde_json::json!({"fault_id": "123", "title": "Broken pipe"}),
        );
        context.set_variable("building_name".to_string(), "Test Building".to_string());

        let template = "Fault {{trigger.title}} reported in {{building_name}}";
        let result = context.substitute_template(template);

        assert_eq!(result, "Fault Broken pipe reported in Test Building");
    }

    #[test]
    fn test_action_registry_default() {
        let registry = ActionRegistry::new();
        assert!(registry.supports("send_email"));
        assert!(registry.supports("send_notification"));
        assert!(registry.supports("call_webhook"));
        assert!(registry.supports("delay"));
    }
}
