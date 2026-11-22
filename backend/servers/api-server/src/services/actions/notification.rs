//! Notification action executor (Epic 94, Story 94.1).
//!
//! Sends in-app notifications as part of workflow execution.

use super::{ActionContext, ActionError, ActionExecutor, ActionResult};
use async_trait::async_trait;
use db::models::action_type;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Notification channel types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum NotificationChannel {
    #[default]
    InApp,
    Push,
    Sms,
    All,
}

impl NotificationChannel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InApp => "in_app",
            Self::Push => "push",
            Self::Sms => "sms",
            Self::All => "all",
        }
    }
}

impl std::fmt::Display for NotificationChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Target type for notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationTarget {
    /// Send to a specific user by ID
    User(String),
    /// Send to all users with a specific role
    Role(String),
    /// Send to all building residents
    Building(String),
    /// Send to all organization members
    Organization,
    /// Send to a dynamic target from trigger event
    Dynamic(String),
}

/// Configuration for notification action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Notification title (supports template variables)
    pub title: String,
    /// Notification message (supports template variables)
    pub message: String,
    /// Channel to send through
    #[serde(default)]
    pub channel: NotificationChannel,
    /// Target for the notification
    pub target: NotificationTarget,
    /// Priority level (1-5, 1 being highest)
    #[serde(default = "default_priority")]
    pub priority: i32,
    /// Optional action URL
    pub action_url: Option<String>,
    /// Optional category for grouping
    pub category: Option<String>,
    /// Custom data to include
    #[serde(default)]
    pub data: serde_json::Value,
}

fn default_priority() -> i32 {
    3
}

/// A fully-resolved notification handed to a [`NotificationSink`] for delivery.
///
/// All template variables have already been substituted and the target has
/// been resolved to concrete recipient identifiers, so a sink only has to
/// deliver/enqueue.
#[derive(Debug, Clone)]
pub struct WorkflowNotification {
    /// Notification title (templates already substituted).
    pub title: String,
    /// Notification message (templates already substituted).
    pub message: String,
    /// Resolved recipient identifiers (user ids / `role:`, `building:`, `org:` selectors).
    pub targets: Vec<String>,
    /// Delivery channel.
    pub channel: NotificationChannel,
    /// Priority level (1-5, 1 being highest).
    pub priority: i32,
    /// Optional action URL (templates already substituted).
    pub action_url: Option<String>,
    /// Optional category for grouping.
    pub category: Option<String>,
    /// Custom data to include.
    pub data: serde_json::Value,
}

/// Sink that actually delivers (or enqueues) a workflow notification.
///
/// The executor is deliberately decoupled from any concrete notification
/// service: production wires a sink backed by the notification pipeline, tests
/// wire a recording sink. Keeping this a trait means the executor never has to
/// pretend a notification was sent — if no sink is wired it refuses to report
/// success (see [`NotificationExecutor::execute`]), consistent with the other
/// action executors, which only return success after the work actually ran.
#[async_trait]
pub trait NotificationSink: Send + Sync {
    /// Deliver or enqueue `notification`, returning the number of recipients
    /// actually dispatched to. An `Err` is surfaced to the workflow engine as
    /// an [`ActionError::ExternalServiceError`] (and is retried like any other
    /// transient action failure).
    async fn dispatch(&self, notification: &WorkflowNotification) -> Result<usize, String>;
}

/// Notification action executor.
///
/// Holds an optional [`NotificationSink`]. When a sink is wired the action
/// dispatches/enqueues for real; when it is absent the action returns a real
/// error rather than a silent success, so a mis-wired deployment surfaces the
/// gap instead of claiming notifications were sent when they were not.
pub struct NotificationExecutor {
    sink: Option<Arc<dyn NotificationSink>>,
}

impl NotificationExecutor {
    /// Create a new notification executor with no dispatcher wired.
    ///
    /// Without a sink the executor refuses to report success — use
    /// [`NotificationExecutor::with_sink`] to attach a real dispatcher.
    pub fn new() -> Self {
        Self { sink: None }
    }

    /// Create a notification executor backed by a concrete dispatcher.
    pub fn with_sink(sink: Arc<dyn NotificationSink>) -> Self {
        Self { sink: Some(sink) }
    }

    /// Parse and validate the notification configuration.
    fn parse_config(config: &serde_json::Value) -> Result<NotificationConfig, ActionError> {
        serde_json::from_value(config.clone()).map_err(|e| {
            ActionError::ConfigurationError(format!("Invalid notification config: {}", e))
        })
    }

    /// Resolve the target to user IDs.
    fn resolve_target(
        target: &NotificationTarget,
        context: &ActionContext,
    ) -> Result<Vec<String>, ActionError> {
        match target {
            NotificationTarget::User(user_id) => {
                // Substitute template if it looks like a variable
                let resolved = if user_id.starts_with("{{") {
                    context.substitute_template(user_id)
                } else {
                    user_id.clone()
                };
                Ok(vec![resolved])
            }
            NotificationTarget::Role(role) => {
                // In production, this would query users with the role
                tracing::debug!(role = %role, "Would send to users with role");
                Ok(vec![format!("role:{}", role)])
            }
            NotificationTarget::Building(building_id) => {
                // Substitute template if needed
                let resolved = if building_id.starts_with("{{") {
                    context.substitute_template(building_id)
                } else {
                    building_id.clone()
                };
                // In production, this would query building residents
                tracing::debug!(building_id = %resolved, "Would send to building residents");
                Ok(vec![format!("building:{}", resolved)])
            }
            NotificationTarget::Organization => {
                // Send to all organization members
                Ok(vec![format!("org:{}", context.organization_id)])
            }
            NotificationTarget::Dynamic(field) => {
                // Get target from trigger event
                let template = format!("{{{{{}}}}}", field);
                let resolved = context.substitute_template(&template);
                if resolved == template {
                    Err(ActionError::ConfigurationError(format!(
                        "Dynamic field '{}' not found in trigger event",
                        field
                    )))
                } else {
                    Ok(vec![resolved])
                }
            }
        }
    }
}

impl Default for NotificationExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ActionExecutor for NotificationExecutor {
    async fn execute(
        &self,
        config: &serde_json::Value,
        context: &ActionContext,
    ) -> Result<ActionResult, ActionError> {
        let start = Instant::now();

        // Parse configuration
        let notif_config = Self::parse_config(config)?;

        // Substitute template variables
        let title = context.substitute_template(&notif_config.title);
        let message = context.substitute_template(&notif_config.message);
        let action_url = notif_config
            .action_url
            .as_ref()
            .map(|url| context.substitute_template(url));

        // Resolve targets
        let targets = Self::resolve_target(&notif_config.target, context)?;

        tracing::info!(
            workflow_id = %context.workflow_id,
            execution_id = %context.execution_id,
            title = %title,
            targets = ?targets,
            channel = ?notif_config.channel,
            priority = notif_config.priority,
            "Workflow sending notification"
        );

        let notification = WorkflowNotification {
            title: title.clone(),
            message: message.clone(),
            targets: targets.clone(),
            channel: notif_config.channel.clone(),
            priority: notif_config.priority,
            action_url: action_url.clone(),
            category: notif_config.category.clone(),
            data: notif_config.data.clone(),
        };

        // Actually dispatch/enqueue via the wired sink. If no sink is
        // configured, refuse to report success: a silent "sent" when nothing
        // was delivered is the bug this executor used to have. This mirrors the
        // other action executors, which only return success once the work ran.
        let Some(sink) = self.sink.as_ref() else {
            return Err(ActionError::ExternalServiceError(
                "notification action has no dispatcher wired; refusing to report \
                 a notification as sent without delivering it"
                    .to_string(),
            ));
        };

        let recipients = sink
            .dispatch(&notification)
            .await
            .map_err(ActionError::ExternalServiceError)?;

        let duration_ms = start.elapsed().as_millis() as i32;

        Ok(ActionResult::success(
            serde_json::json!({
                "title": title,
                "message": message,
                "targets": targets,
                "channel": notif_config.channel.to_string(),
                "priority": notif_config.priority,
                "action_url": action_url,
                "dispatched": true,
                "recipients": recipients,
                "sent_at": chrono::Utc::now().to_rfc3339()
            }),
            duration_ms,
        ))
    }

    fn validate_config(&self, config: &serde_json::Value) -> Result<(), ActionError> {
        let notif_config = Self::parse_config(config)?;

        if notif_config.title.is_empty() {
            return Err(ActionError::MissingField("title".to_string()));
        }
        if notif_config.message.is_empty() {
            return Err(ActionError::MissingField("message".to_string()));
        }
        if notif_config.priority < 1 || notif_config.priority > 5 {
            return Err(ActionError::ConfigurationError(
                "Priority must be between 1 and 5".to_string(),
            ));
        }

        Ok(())
    }

    fn action_type(&self) -> &'static str {
        action_type::SEND_NOTIFICATION
    }

    fn default_timeout(&self) -> Duration {
        Duration::from_secs(10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Test sink that records every dispatched notification and reports one
    /// recipient per resolved target.
    #[derive(Default)]
    struct RecordingSink {
        dispatched: Mutex<Vec<WorkflowNotification>>,
    }

    #[async_trait]
    impl NotificationSink for RecordingSink {
        async fn dispatch(&self, notification: &WorkflowNotification) -> Result<usize, String> {
            let count = notification.targets.len();
            self.dispatched.lock().unwrap().push(notification.clone());
            Ok(count)
        }
    }

    /// Test sink that always fails, to exercise error propagation.
    struct FailingSink;

    #[async_trait]
    impl NotificationSink for FailingSink {
        async fn dispatch(&self, _notification: &WorkflowNotification) -> Result<usize, String> {
            Err("downstream notification service unavailable".to_string())
        }
    }

    #[tokio::test]
    async fn test_notification_executor_success() {
        let sink = Arc::new(RecordingSink::default());
        let executor = NotificationExecutor::with_sink(sink.clone());
        let config = serde_json::json!({
            "title": "New Fault Reported",
            "message": "A new fault has been reported in your building",
            "target": {"user": "user-123"},
            "priority": 2
        });

        let context = ActionContext::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            serde_json::json!({}),
        );

        let result = executor.execute(&config, &context).await.unwrap();
        assert!(result.success);
        assert_eq!(
            result.output.get("title").unwrap().as_str().unwrap(),
            "New Fault Reported"
        );
        // The notification was actually handed to the sink, not silently dropped.
        assert_eq!(
            result.output.get("dispatched").unwrap().as_bool(),
            Some(true)
        );
        let recorded = sink.dispatched.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].title, "New Fault Reported");
        assert_eq!(recorded[0].targets, vec!["user-123".to_string()]);
    }

    #[tokio::test]
    async fn test_notification_sink_error_propagates() {
        // A dispatch failure must surface as an error, not a silent success.
        let executor = NotificationExecutor::with_sink(Arc::new(FailingSink));
        let config = serde_json::json!({
            "title": "New Fault Reported",
            "message": "A new fault has been reported in your building",
            "target": {"user": "user-123"}
        });

        let context = ActionContext::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            serde_json::json!({}),
        );

        let err = executor.execute(&config, &context).await.unwrap_err();
        assert!(matches!(err, ActionError::ExternalServiceError(_)));
    }

    #[tokio::test]
    async fn test_notification_without_sink_errors() {
        // Regression (code-review: api-core notif action stub noop): with no
        // dispatcher wired, the executor must surface a real error instead of
        // silently reporting the notification as sent. Before the fix this
        // returned Ok(ActionResult::success(..)) — a silent no-op.
        let executor = NotificationExecutor::new();
        let config = serde_json::json!({
            "title": "New Fault Reported",
            "message": "A new fault has been reported in your building",
            "target": {"user": "user-123"},
            "priority": 2
        });

        let context = ActionContext::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            serde_json::json!({}),
        );

        let result = executor.execute(&config, &context).await;
        assert!(
            result.is_err(),
            "executor with no sink must not report success without dispatching"
        );
    }

    #[tokio::test]
    async fn test_notification_with_dynamic_target() {
        let executor = NotificationExecutor::with_sink(Arc::new(RecordingSink::default()));
        let config = serde_json::json!({
            "title": "Fault Update",
            "message": "Your fault has been updated",
            "target": {"dynamic": "trigger.reporter_id"}
        });

        let context = ActionContext::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            serde_json::json!({
                "reporter_id": "user-456"
            }),
        );

        let result = executor.execute(&config, &context).await.unwrap();
        assert!(result.success);
        let targets = result.output.get("targets").unwrap().as_array().unwrap();
        assert_eq!(targets[0].as_str().unwrap(), "user-456");
    }

    #[tokio::test]
    async fn test_notification_building_target() {
        let executor = NotificationExecutor::with_sink(Arc::new(RecordingSink::default()));
        let config = serde_json::json!({
            "title": "Building Announcement",
            "message": "Important update for all residents",
            "target": {"building": "{{trigger.building_id}}"},
            "channel": "all"
        });

        let context = ActionContext::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            serde_json::json!({
                "building_id": "building-789"
            }),
        );

        let result = executor.execute(&config, &context).await.unwrap();
        assert!(result.success);
        let targets = result.output.get("targets").unwrap().as_array().unwrap();
        assert!(targets[0].as_str().unwrap().contains("building-789"));
    }

    #[test]
    fn test_notification_config_validation() {
        let executor = NotificationExecutor::new();

        // Valid config
        let valid = serde_json::json!({
            "title": "Test",
            "message": "Test message",
            "target": {"organization": null}
        });
        assert!(executor.validate_config(&valid).is_ok());

        // Missing title
        let missing_title = serde_json::json!({
            "message": "Test message",
            "target": {"organization": null}
        });
        assert!(executor.validate_config(&missing_title).is_err());

        // Invalid priority
        let invalid_priority = serde_json::json!({
            "title": "Test",
            "message": "Test message",
            "target": {"organization": null},
            "priority": 10
        });
        assert!(executor.validate_config(&invalid_priority).is_err());
    }
}
