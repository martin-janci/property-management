//! Delay action executor (Epic 94, Story 94.1).
//!
//! Pauses workflow execution for a specified duration.

use super::{ActionContext, ActionError, ActionExecutor, ActionResult};
use async_trait::async_trait;
use db::models::action_type;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Time unit for delay.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimeUnit {
    #[default]
    Seconds,
    Minutes,
    Hours,
    Days,
}

impl TimeUnit {
    /// Stable snake_case name. Use instead of `{:?}` in audit/log records.
    pub fn as_str(&self) -> &'static str {
        match self {
            TimeUnit::Seconds => "seconds",
            TimeUnit::Minutes => "minutes",
            TimeUnit::Hours => "hours",
            TimeUnit::Days => "days",
        }
    }
}

impl std::fmt::Display for TimeUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Configuration for delay action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelayConfig {
    /// Duration value
    pub duration: u64,
    /// Time unit for the duration
    #[serde(default)]
    pub unit: TimeUnit,
    /// Optional reason for the delay
    pub reason: Option<String>,
}

impl DelayConfig {
    /// Convert the delay to a Duration.
    pub fn to_duration(&self) -> Duration {
        match self.unit {
            TimeUnit::Seconds => Duration::from_secs(self.duration),
            TimeUnit::Minutes => Duration::from_secs(self.duration * 60),
            TimeUnit::Hours => Duration::from_secs(self.duration * 3600),
            TimeUnit::Days => Duration::from_secs(self.duration * 86400),
        }
    }
}

/// Maximum delay allowed (7 days in seconds).
const MAX_DELAY_SECONDS: u64 = 7 * 24 * 3600;

/// Delay action executor.
pub struct DelayExecutor {}

impl DelayExecutor {
    /// Create a new delay executor.
    pub fn new() -> Self {
        Self {}
    }

    /// Parse and validate the delay configuration.
    fn parse_config(config: &serde_json::Value) -> Result<DelayConfig, ActionError> {
        serde_json::from_value(config.clone())
            .map_err(|e| ActionError::ConfigurationError(format!("Invalid delay config: {}", e)))
    }
}

impl Default for DelayExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ActionExecutor for DelayExecutor {
    async fn execute(
        &self,
        config: &serde_json::Value,
        context: &ActionContext,
    ) -> Result<ActionResult, ActionError> {
        let start = Instant::now();

        // Parse configuration
        let delay_config = Self::parse_config(config)?;
        let duration = delay_config.to_duration();

        // Validate duration
        if duration.as_secs() > MAX_DELAY_SECONDS {
            return Err(ActionError::ConfigurationError(format!(
                "Delay exceeds maximum of {} seconds (7 days)",
                MAX_DELAY_SECONDS
            )));
        }

        tracing::info!(
            workflow_id = %context.workflow_id,
            execution_id = %context.execution_id,
            delay_seconds = duration.as_secs(),
            reason = ?delay_config.reason,
            "Workflow delaying execution"
        );

        // Perform the delay
        tokio::time::sleep(duration).await;

        let actual_duration_ms = start.elapsed().as_millis() as i32;

        Ok(ActionResult::success(
            serde_json::json!({
                "delayed_for_seconds": duration.as_secs(),
                "unit": delay_config.unit.to_string(),
                "reason": delay_config.reason,
                "actual_duration_ms": actual_duration_ms,
                "completed_at": chrono::Utc::now().to_rfc3339()
            }),
            actual_duration_ms,
        ))
    }

    fn validate_config(&self, config: &serde_json::Value) -> Result<(), ActionError> {
        let delay_config = Self::parse_config(config)?;

        if delay_config.duration == 0 {
            return Err(ActionError::ConfigurationError(
                "Delay duration must be greater than 0".to_string(),
            ));
        }

        let duration = delay_config.to_duration();
        if duration.as_secs() > MAX_DELAY_SECONDS {
            return Err(ActionError::ConfigurationError(format!(
                "Delay exceeds maximum of {} seconds (7 days)",
                MAX_DELAY_SECONDS
            )));
        }

        Ok(())
    }

    fn action_type(&self) -> &'static str {
        action_type::DELAY
    }

    fn default_timeout(&self) -> Duration {
        // Delay timeout should be longer than the delay itself
        Duration::from_secs(MAX_DELAY_SECONDS + 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delay_config_to_duration() {
        let config = DelayConfig {
            duration: 5,
            unit: TimeUnit::Minutes,
            reason: None,
        };
        assert_eq!(config.to_duration().as_secs(), 300);

        let config = DelayConfig {
            duration: 2,
            unit: TimeUnit::Hours,
            reason: None,
        };
        assert_eq!(config.to_duration().as_secs(), 7200);

        let config = DelayConfig {
            duration: 1,
            unit: TimeUnit::Days,
            reason: None,
        };
        assert_eq!(config.to_duration().as_secs(), 86400);
    }

    #[tokio::test]
    async fn test_delay_executor_short_delay() {
        let executor = DelayExecutor::new();
        let config = serde_json::json!({
            "duration": 100,
            "unit": "seconds",
            "reason": "Wait for external system"
        });

        // We can't actually wait 100 seconds in a test, so we'll just validate
        assert!(executor.validate_config(&config).is_ok());
    }

    #[tokio::test]
    async fn test_delay_executor_very_short() {
        let executor = DelayExecutor::new();
        let config = serde_json::json!({
            "duration": 1,
            "unit": "seconds"
        });

        let context = ActionContext::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            serde_json::json!({}),
        );

        // Use a shorter delay for testing
        let start = Instant::now();
        let result = executor.execute(&config, &context).await.unwrap();
        let elapsed = start.elapsed();

        assert!(result.success);
        assert!(elapsed >= Duration::from_secs(1));
        assert!(elapsed < Duration::from_secs(2));
    }

    #[test]
    fn test_delay_config_validation() {
        let executor = DelayExecutor::new();

        // Valid config
        let valid = serde_json::json!({
            "duration": 60,
            "unit": "seconds"
        });
        assert!(executor.validate_config(&valid).is_ok());

        // Zero duration
        let zero = serde_json::json!({
            "duration": 0,
            "unit": "seconds"
        });
        assert!(executor.validate_config(&zero).is_err());

        // Exceeds max
        let too_long = serde_json::json!({
            "duration": 10,
            "unit": "days"
        });
        assert!(executor.validate_config(&too_long).is_err());
    }
}
