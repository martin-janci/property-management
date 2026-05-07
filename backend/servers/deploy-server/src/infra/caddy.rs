// backend/servers/deploy-server/src/infra/caddy.rs
use crate::Result;
use serde_json::json;

pub struct CaddyClient {
    base: String,
    http: reqwest::Client,
}

impl CaddyClient {
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            http: reqwest::Client::builder().build().unwrap(),
        }
    }

    /// Register a host → upstream mapping. Idempotent: replaces existing route for `host`.
    pub async fn register_route(&self, host: &str, upstream: &str) -> Result<()> {
        let route_id = format!("ppt-deploy-{}", sanitize_id(host));
        let payload = json!({
            "@id": route_id,
            "match": [{"host": [host]}],
            "handle": [
                {
                    "handler": "reverse_proxy",
                    "upstreams": [{"dial": upstream}]
                }
            ]
        });
        let url = format!("{}/id/{}", self.base, route_id);
        let resp = self.http.put(&url).json(&payload).send().await?;
        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }
        // Fallback only on 404 (route doesn't exist yet) — append to
        // apps.http.servers.srv0.routes. Any other status (401/403/5xx) is a real
        // failure and we surface it directly instead of masking with a duplicate POST.
        if status.as_u16() != 404 {
            return Err(crate::DeployError::Internal(format!(
                "caddy register PUT failed: {status}"
            )));
        }
        let append_url = format!("{}/config/apps/http/servers/srv0/routes/...", self.base);
        self.http
            .post(&append_url)
            .json(&json!([payload]))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn unregister_route(&self, host: &str) -> Result<()> {
        let route_id = format!("ppt-deploy-{}", sanitize_id(host));
        let url = format!("{}/id/{}", self.base, route_id);
        let resp = self.http.delete(&url).send().await?;
        if resp.status().is_success() || resp.status().as_u16() == 404 {
            Ok(())
        } else {
            Err(crate::DeployError::Internal(format!(
                "caddy unregister: {}",
                resp.status()
            )))
        }
    }
}

fn sanitize_id(host: &str) -> String {
    host.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    #[tokio::test]
    async fn register_route_calls_admin_api() {
        let server = MockServer::start();
        let m = server.mock(|when, then| {
            when.method(PUT).path_contains("/id/ppt-deploy-");
            then.status(200);
        });
        let client = CaddyClient::new(server.base_url());
        client
            .register_route("wt-uc14.dev.ppt.rlt.sk", "127.0.0.1:51001")
            .await
            .unwrap();
        m.assert();
    }

    #[tokio::test]
    async fn unregister_404_is_ok() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(DELETE).path_contains("/id/ppt-deploy-");
            then.status(404);
        });
        let client = CaddyClient::new(server.base_url());
        client.unregister_route("missing.dev.rlt.sk").await.unwrap();
    }
}
