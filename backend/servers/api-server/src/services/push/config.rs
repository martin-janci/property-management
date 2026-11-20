//! Push fanout worker configuration.

/// Configuration for the push fanout background worker.
#[derive(Clone, Debug)]
pub struct PushFanoutConfig {
    /// Whether the worker should start (default: `true` unless `PUSH_FANOUT_ENABLED=false`).
    pub enabled: bool,
    /// How often the worker polls for pending push jobs (seconds, default: 30).
    pub poll_interval_secs: u64,
}

impl Default for PushFanoutConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            poll_interval_secs: 30,
        }
    }
}

impl PushFanoutConfig {
    /// Load from environment variables.
    pub fn from_env() -> Self {
        let enabled = std::env::var("PUSH_FANOUT_ENABLED")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(true);
        let poll_interval_secs = std::env::var("PUSH_FANOUT_POLL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);
        Self {
            enabled,
            poll_interval_secs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_fanout_config_defaults() {
        let config = PushFanoutConfig::default();
        assert!(config.enabled);
        assert_eq!(config.poll_interval_secs, 30);
    }
}
