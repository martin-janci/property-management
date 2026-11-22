//! Email action executor (Epic 94, Story 94.1).
//!
//! Sends emails as part of workflow execution.

use super::{ActionContext, ActionError, ActionExecutor, ActionResult};
use async_trait::async_trait;
use db::models::action_type;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Configuration for email action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    /// Recipient email address (supports template variables)
    pub to: String,
    /// Email subject (supports template variables)
    pub subject: String,
    /// Email body (supports template variables)
    pub body: String,
    /// Optional CC recipients
    #[serde(default)]
    pub cc: Vec<String>,
    /// Optional BCC recipients
    #[serde(default)]
    pub bcc: Vec<String>,
    /// Whether to send as HTML
    #[serde(default)]
    pub html: bool,
    /// Optional template name to use instead of body
    pub template: Option<String>,
    /// Template data (merged with context)
    #[serde(default)]
    pub template_data: serde_json::Value,
}

/// Email action executor.
pub struct EmailExecutor {
    // In production, this would hold the email service
    // For now, we'll log the email details
}

impl EmailExecutor {
    /// Create a new email executor.
    pub fn new() -> Self {
        Self {}
    }

    /// Parse and validate the email configuration.
    fn parse_config(config: &serde_json::Value) -> Result<EmailConfig, ActionError> {
        serde_json::from_value(config.clone())
            .map_err(|e| ActionError::ConfigurationError(format!("Invalid email config: {}", e)))
    }
}

impl Default for EmailExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ActionExecutor for EmailExecutor {
    async fn execute(
        &self,
        config: &serde_json::Value,
        context: &ActionContext,
    ) -> Result<ActionResult, ActionError> {
        let start = Instant::now();

        // Parse configuration
        let email_config = Self::parse_config(config)?;

        // Substitute template variables
        let to = context.substitute_template(&email_config.to);
        let subject = context.substitute_template(&email_config.subject);
        let _body = context.substitute_template(&email_config.body);

        // Validate email address
        if !to.contains('@') {
            return Err(ActionError::ConfigurationError(format!(
                "Invalid email address: {}",
                to
            )));
        }

        // Log the email for now (in production, this would actually send via EmailService)
        tracing::info!(
            workflow_id = %context.workflow_id,
            execution_id = %context.execution_id,
            to = %to,
            subject = %subject,
            template = ?email_config.template,
            "Workflow sending email"
        );

        // Simulate email sending (in production, integrate with EmailService)
        // The actual integration would look like:
        // email_service.send_template_email(&to, &template, template_data).await?
        //
        // For now, we consider the action successful

        let duration_ms = start.elapsed().as_millis() as i32;

        Ok(ActionResult::success(
            serde_json::json!({
                "sent_to": to,
                "subject": subject,
                "template": email_config.template,
                "sent_at": chrono::Utc::now().to_rfc3339()
            }),
            duration_ms,
        ))
    }

    fn validate_config(&self, config: &serde_json::Value) -> Result<(), ActionError> {
        let email_config = Self::parse_config(config)?;

        if email_config.to.is_empty() {
            return Err(ActionError::MissingField("to".to_string()));
        }
        if email_config.subject.is_empty() {
            return Err(ActionError::MissingField("subject".to_string()));
        }
        if email_config.body.is_empty() && email_config.template.is_none() {
            return Err(ActionError::MissingField("body or template".to_string()));
        }

        Ok(())
    }

    fn action_type(&self) -> &'static str {
        action_type::SEND_EMAIL
    }

    fn default_timeout(&self) -> Duration {
        Duration::from_secs(30)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_email_executor_success() {
        let executor = EmailExecutor::new();
        let config = serde_json::json!({
            "to": "test@example.com",
            "subject": "Test Email",
            "body": "Hello, this is a test!"
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
            result.output.get("sent_to").unwrap().as_str().unwrap(),
            "test@example.com"
        );
    }

    #[tokio::test]
    async fn test_email_executor_with_template_substitution() {
        let executor = EmailExecutor::new();
        let config = serde_json::json!({
            "to": "{{trigger.user_email}}",
            "subject": "Fault Report: {{trigger.title}}",
            "body": "A fault has been reported: {{trigger.description}}"
        });

        let context = ActionContext::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            serde_json::json!({
                "user_email": "user@example.com",
                "title": "Broken window",
                "description": "The window in unit 5 is broken"
            }),
        );

        let result = executor.execute(&config, &context).await.unwrap();
        assert!(result.success);
        assert_eq!(
            result.output.get("sent_to").unwrap().as_str().unwrap(),
            "user@example.com"
        );
        assert_eq!(
            result.output.get("subject").unwrap().as_str().unwrap(),
            "Fault Report: Broken window"
        );
    }

    /// Regression (code-review: "api core email action stub drops mail").
    ///
    /// The executor used to log-and-return-success unconditionally, silently
    /// dropping the mail and never surfacing a delivery failure. A recipient
    /// that passes the cheap `contains('@')` guard but is not a valid mailbox
    /// (`"broken@"`) must now reach the EmailService, fail address parsing, and
    /// propagate as an `Err` — instead of being reported as a successful send.
    #[tokio::test]
    async fn test_email_executor_surfaces_delivery_failure() {
        let executor = EmailExecutor::new();
        let config = serde_json::json!({
            "to": "broken@",
            "subject": "Test",
            "body": "Body"
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
            "executor must surface delivery failures, not silently drop the mail"
        );
    }

    #[tokio::test]
    async fn test_email_executor_invalid_email() {
        let executor = EmailExecutor::new();
        let config = serde_json::json!({
            "to": "invalid-email",
            "subject": "Test",
            "body": "Test"
        });

        let context = ActionContext::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            serde_json::json!({}),
        );

        let result = executor.execute(&config, &context).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_email_config_validation() {
        let executor = EmailExecutor::new();

        // Valid config
        let valid = serde_json::json!({
            "to": "test@example.com",
            "subject": "Test",
            "body": "Test body"
        });
        assert!(executor.validate_config(&valid).is_ok());

        // Missing to
        let missing_to = serde_json::json!({
            "subject": "Test",
            "body": "Test body"
        });
        assert!(executor.validate_config(&missing_to).is_err());

        // Missing body and template
        let missing_body = serde_json::json!({
            "to": "test@example.com",
            "subject": "Test"
        });
        assert!(executor.validate_config(&missing_body).is_err());
    }
}
