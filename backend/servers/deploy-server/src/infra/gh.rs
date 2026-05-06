// backend/servers/deploy-server/src/infra/gh.rs
use crate::Result;
use serde::Deserialize;

pub struct GhClient {
    token: String,
    repo: String, // "martin-janci/property-management"
    http: reqwest::Client,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowRun {
    pub id: u64,
    pub status: String,             // "queued" | "in_progress" | "completed"
    pub conclusion: Option<String>, // "success" | "failure" | ... when completed
    pub html_url: String,
}

impl GhClient {
    pub fn new(token: impl Into<String>, repo: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            repo: repo.into(),
            http: reqwest::Client::new(),
        }
    }

    /// POST /repos/{repo}/actions/workflows/{workflow_file}/dispatches
    pub async fn dispatch_workflow(&self, workflow_file: &str, branch: &str) -> Result<()> {
        let url = format!(
            "https://api.github.com/repos/{}/actions/workflows/{}/dispatches",
            self.repo, workflow_file
        );
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "ppt-deploy")
            .json(&serde_json::json!({"ref": branch}))
            .send()
            .await?;
        resp.error_for_status()?;
        Ok(())
    }

    /// GET latest workflow run for a branch.
    pub async fn latest_run(
        &self,
        workflow_file: &str,
        branch: &str,
    ) -> Result<Option<WorkflowRun>> {
        let url = format!(
            "https://api.github.com/repos/{}/actions/workflows/{}/runs?branch={}&per_page=1",
            self.repo, workflow_file, branch
        );
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "ppt-deploy")
            .send()
            .await?;
        let body: serde_json::Value = resp.error_for_status()?.json().await?;
        let runs = body["workflow_runs"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        if let Some(run) = runs.into_iter().next() {
            let parsed: WorkflowRun = serde_json::from_value(run)
                .map_err(|e| crate::DeployError::Internal(format!("parse run: {e}")))?;
            return Ok(Some(parsed));
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_run_parses() {
        let json = r#"{"id":1,"status":"completed","conclusion":"success","html_url":"https://github.com/x/y"}"#;
        let run: WorkflowRun = serde_json::from_str(json).unwrap();
        assert_eq!(run.id, 1);
        assert_eq!(run.status, "completed");
        assert_eq!(run.conclusion.as_deref(), Some("success"));
    }
}
