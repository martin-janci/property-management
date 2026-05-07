// backend/servers/deploy-server/src/infra/health.rs
use crate::Result;
use std::time::Duration;
use tokio::time::sleep;

pub struct HealthProbe {
    http: reqwest::Client,
}

impl Default for HealthProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthProbe {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap(),
        }
    }

    /// Check `attempts` times over `total_secs`, sleeping `total_secs / attempts` between checks.
    /// Returns Ok(()) if all checks pass; Err on the first failure.
    ///
    /// The per-attempt sleep is clamped to **at least 1 second** so that callers
    /// passing `total_secs < attempts` (or very small totals) don't end up in a
    /// tight retry loop hammering the target. The `total_secs` budget remains
    /// the floor — this method may take longer than `total_secs` when the
    /// caller picked an under-budgeted ratio, which is the desired behavior.
    pub async fn grace_check(&self, url: &str, attempts: u32, total_secs: u64) -> Result<()> {
        let interval = (total_secs / attempts.max(1) as u64).max(1);
        for i in 0..attempts {
            sleep(Duration::from_secs(interval)).await;
            let resp = self
                .http
                .get(url)
                .send()
                .await
                .map_err(crate::DeployError::Http)?;
            if !resp.status().is_success() {
                return Err(crate::DeployError::Internal(format!(
                    "health check {} failed: {}",
                    i + 1,
                    resp.status()
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    #[tokio::test]
    async fn grace_check_passes_when_healthy() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/health");
            then.status(200);
        });
        let probe = HealthProbe::new();
        let url = format!("{}/health", server.base_url());
        // Use 1 attempt, 1 sec total to keep test fast.
        probe.grace_check(&url, 1, 1).await.unwrap();
    }

    #[tokio::test]
    async fn grace_check_fails_on_5xx() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/health");
            then.status(500);
        });
        let probe = HealthProbe::new();
        let url = format!("{}/health", server.base_url());
        let res = probe.grace_check(&url, 1, 1).await;
        assert!(res.is_err());
    }
}
