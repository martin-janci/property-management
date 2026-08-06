//! API call (webhook) action executor (Epic 94, Story 94.1).
//!
//! Makes HTTP requests to external APIs as part of workflow execution.

use super::{ActionContext, ActionError, ActionExecutor, ActionResult};
use async_trait::async_trait;
use db::models::action_type;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// HTTP method for the API call.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
#[allow(clippy::upper_case_acronyms)]
pub enum HttpMethod {
    #[default]
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::GET => write!(f, "GET"),
            HttpMethod::POST => write!(f, "POST"),
            HttpMethod::PUT => write!(f, "PUT"),
            HttpMethod::PATCH => write!(f, "PATCH"),
            HttpMethod::DELETE => write!(f, "DELETE"),
        }
    }
}

/// Authentication type for the API call.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthConfig {
    #[default]
    None,
    Bearer {
        token: String,
    },
    Basic {
        username: String,
        password: String,
    },
    Header {
        name: String,
        value: String,
    },
}

/// Configuration for API call action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCallConfig {
    /// Target URL (supports template variables)
    pub url: String,
    /// HTTP method
    #[serde(default)]
    pub method: HttpMethod,
    /// Request headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Request body (for POST/PUT/PATCH)
    pub body: Option<serde_json::Value>,
    /// Authentication configuration
    #[serde(default)]
    pub auth: AuthConfig,
    /// Timeout in seconds
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
    /// Expected success status codes (default: 200-299)
    #[serde(default = "default_success_codes")]
    pub success_codes: Vec<u16>,
    /// Whether to parse response as JSON
    #[serde(default = "default_true")]
    pub parse_json_response: bool,
    /// Whether to include response in output
    #[serde(default = "default_true")]
    pub include_response: bool,
}

fn default_timeout_seconds() -> u64 {
    30
}

fn default_success_codes() -> Vec<u16> {
    (200..300).collect()
}

fn default_true() -> bool {
    true
}

/// API call action executor.
pub struct ApiCallExecutor {
    client: reqwest::Client,
}

impl ApiCallExecutor {
    /// Create a new API call executor.
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .user_agent("PPT-Workflow/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Parse and validate the API call configuration.
    fn parse_config(config: &serde_json::Value) -> Result<ApiCallConfig, ActionError> {
        serde_json::from_value(config.clone())
            .map_err(|e| ActionError::ConfigurationError(format!("Invalid API call config: {}", e)))
    }

    /// Substitute template variables in the body.
    fn substitute_body(body: &serde_json::Value, context: &ActionContext) -> serde_json::Value {
        match body {
            serde_json::Value::String(s) => {
                serde_json::Value::String(context.substitute_template(s))
            }
            serde_json::Value::Object(map) => {
                let mut new_map = serde_json::Map::new();
                for (k, v) in map {
                    new_map.insert(k.clone(), Self::substitute_body(v, context));
                }
                serde_json::Value::Object(new_map)
            }
            serde_json::Value::Array(arr) => serde_json::Value::Array(
                arr.iter()
                    .map(|v| Self::substitute_body(v, context))
                    .collect(),
            ),
            other => other.clone(),
        }
    }
}

impl Default for ApiCallExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ActionExecutor for ApiCallExecutor {
    async fn execute(
        &self,
        config: &serde_json::Value,
        context: &ActionContext,
    ) -> Result<ActionResult, ActionError> {
        let start = Instant::now();

        // Parse configuration
        let api_config = Self::parse_config(config)?;

        // Substitute template variables in URL
        let url = context.substitute_template(&api_config.url);

        // Validate URL — SSRF defence via the shared, single-source-of-truth
        // validator (`common::url_validation`). This replaces a private copy
        // that had drifted from the canonical policy: it never rejected plain
        // `http://` in production and missed IPv4 multicast / TEST-NET
        // documentation ranges. Delegating keeps scheme allowlisting
        // (http(s) only, plain-http gated to `RUST_ENV=development`), blocked
        // hosts/TLDs, and private/reserved IP checks in lock-step everywhere.
        let parsed_url = common::url_validation::validate_external_url(&url)
            .map_err(|e| ActionError::ConfigurationError(e.to_string()))?;

        // SSRF via DNS: `validate_external_url` only inspects the literal
        // string — a hostname with no literal-IP form (e.g.
        // "attacker.example.com") that resolves to a private/internal
        // address at request time sailed straight through it. Resolve now,
        // right before the request is issued, and reject if *any* resolved
        // address falls in a forbidden range. Fails closed: a resolution
        // error also rejects.
        let host = parsed_url.host_str().ok_or_else(|| {
            ActionError::ConfigurationError("API call URL must have a host".to_string())
        })?;
        let port = parsed_url.port_or_known_default().unwrap_or(443);
        common::url_validation::validate_resolved_host(host, port)
            .await
            .map_err(|e| ActionError::ConfigurationError(e.to_string()))?;

        // Build request
        let mut request = match api_config.method {
            HttpMethod::GET => self.client.get(&url),
            HttpMethod::POST => self.client.post(&url),
            HttpMethod::PUT => self.client.put(&url),
            HttpMethod::PATCH => self.client.patch(&url),
            HttpMethod::DELETE => self.client.delete(&url),
        };

        // Set timeout
        request = request.timeout(Duration::from_secs(api_config.timeout_seconds));

        // Add headers
        for (key, value) in &api_config.headers {
            let value = context.substitute_template(value);
            request = request.header(key, value);
        }

        // Add authentication
        match &api_config.auth {
            AuthConfig::None => {}
            AuthConfig::Bearer { token } => {
                let token = context.substitute_template(token);
                request = request.bearer_auth(token);
            }
            AuthConfig::Basic { username, password } => {
                let username = context.substitute_template(username);
                let password = context.substitute_template(password);
                request = request.basic_auth(username, Some(password));
            }
            AuthConfig::Header { name, value } => {
                let value = context.substitute_template(value);
                request = request.header(name, value);
            }
        }

        // Add body for methods that support it
        if let Some(body) = &api_config.body {
            let substituted_body = Self::substitute_body(body, context);
            request = request.json(&substituted_body);
        }

        // Log the request (in production, be careful with sensitive data)
        tracing::info!(
            workflow_id = %context.workflow_id,
            execution_id = %context.execution_id,
            method = %api_config.method,
            url = %url,
            "Workflow making API call"
        );

        // Execute the request
        let response = request.send().await.map_err(|e| {
            ActionError::ExternalServiceError(format!("HTTP request failed: {}", e))
        })?;

        let status = response.status().as_u16();
        let headers: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        // Check if status code indicates success
        let is_success = api_config.success_codes.contains(&status);

        // Get response body
        let response_body = if api_config.include_response {
            let text = response.text().await.unwrap_or_default();
            if api_config.parse_json_response {
                serde_json::from_str(&text).unwrap_or(serde_json::Value::String(text))
            } else {
                serde_json::Value::String(text)
            }
        } else {
            serde_json::Value::Null
        };

        let duration_ms = start.elapsed().as_millis() as i32;

        if is_success {
            Ok(ActionResult::success(
                serde_json::json!({
                    "status_code": status,
                    "headers": headers,
                    "body": response_body,
                    "url": url,
                    "method": api_config.method.to_string()
                }),
                duration_ms,
            ))
        } else {
            Ok(ActionResult::failure(
                format!(
                    "API call returned status {} (expected one of {:?})",
                    status, api_config.success_codes
                ),
                duration_ms,
            ))
        }
    }

    fn validate_config(&self, config: &serde_json::Value) -> Result<(), ActionError> {
        let api_config = Self::parse_config(config)?;

        if api_config.url.is_empty() {
            return Err(ActionError::MissingField("url".to_string()));
        }

        // Basic URL validation (template variables are allowed)
        if !api_config.url.contains("{{") && !api_config.url.starts_with("http") {
            return Err(ActionError::ConfigurationError(
                "URL must start with http:// or https://".to_string(),
            ));
        }

        if api_config.timeout_seconds == 0 {
            return Err(ActionError::ConfigurationError(
                "Timeout must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }

    fn action_type(&self) -> &'static str {
        action_type::CALL_WEBHOOK
    }

    fn default_timeout(&self) -> Duration {
        Duration::from_secs(60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_call_config_parsing() {
        let config = serde_json::json!({
            "url": "https://api.example.com/webhook",
            "method": "POST",
            "headers": {
                "Content-Type": "application/json"
            },
            "body": {
                "event": "{{trigger.event_type}}",
                "data": "{{trigger.data}}"
            },
            "auth": {
                "type": "bearer",
                "token": "secret-token"
            }
        });

        let parsed = ApiCallExecutor::parse_config(&config).unwrap();
        assert_eq!(parsed.url, "https://api.example.com/webhook");
        assert!(matches!(parsed.method, HttpMethod::POST));
        assert!(matches!(parsed.auth, AuthConfig::Bearer { .. }));
    }

    #[test]
    fn test_api_call_config_validation() {
        let executor = ApiCallExecutor::new();

        // Valid config
        let valid = serde_json::json!({
            "url": "https://api.example.com/webhook"
        });
        assert!(executor.validate_config(&valid).is_ok());

        // Valid with template
        let valid_template = serde_json::json!({
            "url": "{{trigger.webhook_url}}"
        });
        assert!(executor.validate_config(&valid_template).is_ok());

        // Missing URL
        let missing_url = serde_json::json!({
            "method": "POST"
        });
        assert!(executor.validate_config(&missing_url).is_err());

        // Invalid URL (no template, no http)
        let invalid_url = serde_json::json!({
            "url": "invalid-url"
        });
        assert!(executor.validate_config(&invalid_url).is_err());
    }

    #[test]
    fn test_body_substitution() {
        let context = ActionContext::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            serde_json::json!({
                "fault_id": "123",
                "title": "Test Fault"
            }),
        );

        let body = serde_json::json!({
            "id": "{{trigger.fault_id}}",
            "name": "{{trigger.title}}",
            "nested": {
                "value": "{{trigger.fault_id}}"
            }
        });

        let substituted = ApiCallExecutor::substitute_body(&body, &context);

        assert_eq!(substituted["id"], "123");
        assert_eq!(substituted["name"], "Test Fault");
        assert_eq!(substituted["nested"]["value"], "123");
    }

    /// Regression: this executor used to carry a private SSRF validator that
    /// had drifted from `common::url_validation`. Among its gaps, it accepted
    /// IPv4 multicast (`224.0.0.0/4`) — the canonical validator rejects it.
    /// This is an environment-independent proof that the executor now
    /// delegates to the shared validator (the repo convention forbids
    /// `std::env::set_var` in unit tests, so we avoid asserting the
    /// `RUST_ENV`-gated plain-`http://` path here — that policy lives in and
    /// is tested by `common::url_validation`). Fails on `dev` (old validator
    /// let the request through to the network); passes here (rejected before
    /// any request is issued).
    #[tokio::test]
    async fn execute_rejects_multicast_via_shared_validator() {
        let executor = ApiCallExecutor::new();
        let context = ActionContext::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            serde_json::json!({}),
        );
        // https:// keeps the scheme check environment-independent; the drift
        // being exercised is purely the private/reserved IP-range coverage.
        let config = serde_json::json!({ "url": "https://224.0.0.1/latest" });

        match executor.execute(&config, &context).await {
            Err(ActionError::ConfigurationError(_)) => {}
            other => panic!(
                "expected ConfigurationError (SSRF rejection) for a multicast \
                 address, got: {other:?}"
            ),
        }
    }

    /// Regression (SSRF via DNS): `validate_external_url` only inspected the
    /// literal URL string, so a DNS hostname with no literal-IP form (e.g.
    /// `internal.attacker.example`) sailed through even when it resolves to
    /// a private/internal address at request time — the guard never ran a
    /// DNS lookup at all. `common::url_validation::validate_resolved_host`
    /// covers the "resolves to a private IP" case itself, using an injected
    /// fake resolver so it stays hermetic (see
    /// `common::url_validation::tests::validate_resolved_host_rejects_hostname_resolving_to_private_ip`).
    ///
    /// This test proves the executor actually *wires in* that check (rather
    /// than the fix existing only in the library, unused) by exercising the
    /// fail-closed branch: a hostname under the IETF-reserved `.invalid` TLD
    /// (RFC 2606) can never resolve, so DNS resolution errors out and the
    /// fail-closed `ResolutionFailed` branch rejects it — fast and without
    /// depending on any specific external network state, since the
    /// non-delegation of `.invalid` is guaranteed rather than incidental.
    /// Fails on `dev`: `execute` there never calls
    /// `validate_resolved_host`/`validate_resolved_host_with`, so a
    /// non-literal-IP hostname was never re-validated post-DNS and this call
    /// would instead fail later with an `ExternalServiceError` from
    /// `reqwest` (a connect failure), not the `ConfigurationError` this test
    /// asserts.
    #[tokio::test]
    async fn execute_rejects_hostname_that_fails_dns_resolution() {
        let executor = ApiCallExecutor::new();
        let context = ActionContext::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            serde_json::json!({}),
        );
        let config = serde_json::json!({
            "url": "https://this-host-cannot-resolve.invalid/webhook"
        });

        match executor.execute(&config, &context).await {
            Err(ActionError::ConfigurationError(msg)) => {
                assert!(
                    msg.contains("resolve"),
                    "expected a DNS-resolution rejection message, got: {msg}"
                );
            }
            other => panic!(
                "expected ConfigurationError (fail-closed DNS resolution) for an \
                 unresolvable host, got: {other:?}"
            ),
        }
    }
}
