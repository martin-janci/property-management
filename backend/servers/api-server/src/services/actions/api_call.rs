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

/// Maximum number of response-body bytes the API-call action will buffer into
/// memory. A workflow action issues one small outbound request but then reads
/// the *entire* upstream response with `response.text()`, which buffers with no
/// ceiling — a malicious or misbehaving endpoint can reply with a multi-GB (or,
/// via chunked transfer with no `Content-Length`, effectively unbounded) body
/// and amplify a trivial request into an out-of-memory DoS on the worker. We
/// cap the read at this many bytes; anything beyond is dropped and the stored
/// body is marked truncated. 8 MiB comfortably covers legitimate JSON webhook
/// responses while bounding worst-case allocation per in-flight action.
const MAX_RESPONSE_BODY_BYTES: usize = 8 * 1024 * 1024;

/// Appended to a response body that was cut off at `MAX_RESPONSE_BODY_BYTES`,
/// so the persisted workflow-execution record makes the truncation explicit
/// instead of silently storing a partial payload.
const RESPONSE_TRUNCATION_MARKER: &str = "\u{2026}[truncated: response exceeded 8 MiB cap]";

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

    /// Read an upstream response body while hard-capping how many bytes are
    /// buffered, defending against a memory-amplification DoS (a small request
    /// provoking an enormous reply). Returns `(body_text, truncated)`.
    ///
    /// Two layers of defence:
    /// 1. **`Content-Length` pre-check** — if the upstream already advertises a
    ///    body larger than the cap, refuse to buffer any of it.
    /// 2. **Streaming bounded read** — otherwise pull the body chunk-by-chunk
    ///    and stop the moment the accumulated size would exceed the cap. This
    ///    is what protects against a chunked-transfer body with no (or a lying)
    ///    `Content-Length`, which the pre-check alone cannot catch.
    ///
    /// A mid-stream network error keeps whatever was read so far (best-effort),
    /// mirroring the previous `unwrap_or_default()` behaviour.
    async fn read_capped_body(mut response: reqwest::Response) -> (String, bool) {
        // Layer 1: honest oversized Content-Length — never start buffering.
        if let Some(len) = response.content_length() {
            if len > MAX_RESPONSE_BODY_BYTES as u64 {
                return (RESPONSE_TRUNCATION_MARKER.to_string(), true);
            }
        }

        // Layer 2: bounded streaming read.
        let mut buf: Vec<u8> = Vec::new();
        let mut truncated = false;
        loop {
            match response.chunk().await {
                Ok(Some(chunk)) => {
                    let remaining = MAX_RESPONSE_BODY_BYTES.saturating_sub(buf.len());
                    if chunk.len() > remaining {
                        buf.extend_from_slice(&chunk[..remaining]);
                        truncated = true;
                        break;
                    }
                    buf.extend_from_slice(&chunk);
                }
                Ok(None) => break,
                // Network error mid-read: keep the partial body, don't fail the
                // whole action on it (matches prior best-effort semantics).
                Err(_) => break,
            }
        }

        let mut text = String::from_utf8_lossy(&buf).into_owned();
        if truncated {
            text.push_str(RESPONSE_TRUNCATION_MARKER);
        }
        (text, truncated)
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

        // Get response body — capped read so an oversized/unbounded upstream
        // reply cannot amplify into an out-of-memory DoS on the worker.
        let response_body = if api_config.include_response {
            let (text, truncated) = Self::read_capped_body(response).await;
            if truncated {
                tracing::warn!(
                    workflow_id = %context.workflow_id,
                    execution_id = %context.execution_id,
                    cap_bytes = MAX_RESPONSE_BODY_BYTES,
                    "Workflow API-call response body exceeded cap; truncated before persisting"
                );
            }
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

    /// Regression (memory-amplification DoS): the executor used to buffer the
    /// *entire* upstream response with `response.text().await` and no ceiling,
    /// so a small outbound request could provoke a multi-GB (or, via chunked
    /// transfer, effectively unbounded) reply and OOM the worker. `read_capped_body`
    /// now hard-caps the buffered bytes at `MAX_RESPONSE_BODY_BYTES` and marks
    /// the result truncated.
    ///
    /// Hermetic: we hand `read_capped_body` a synthesized over-limit response
    /// (built via `http::Response` → `reqwest::Response`) instead of standing
    /// up a network server — the SSRF validator would reject a `127.0.0.1`
    /// loopback server anyway, so a real request is not an option here.
    /// Fails on `dev` (the old code returned the full body, so the non-marker
    /// payload length would exceed the cap and `truncated` would be false).
    #[tokio::test]
    async fn read_capped_body_truncates_oversized_response() {
        use axum::http;

        // A body deliberately larger than the cap.
        let oversized = vec![b'a'; MAX_RESPONSE_BODY_BYTES + 4096];
        let http_response = http::Response::builder()
            .status(200)
            .body(oversized)
            .expect("build synthetic http::Response");
        let response = reqwest::Response::from(http_response);

        let (text, truncated) = ApiCallExecutor::read_capped_body(response).await;

        assert!(truncated, "an over-limit body must be reported truncated");
        assert!(
            text.ends_with(RESPONSE_TRUNCATION_MARKER),
            "truncated body must carry the truncation marker"
        );
        // The retained payload (excluding the marker) must never exceed the cap
        // — i.e. the full body was NOT buffered.
        let payload_len = text.len() - RESPONSE_TRUNCATION_MARKER.len();
        assert!(
            payload_len <= MAX_RESPONSE_BODY_BYTES,
            "buffered payload {payload_len} must not exceed cap {MAX_RESPONSE_BODY_BYTES}"
        );
    }

    /// Companion: a body comfortably under the cap is returned verbatim and is
    /// not flagged truncated (guards against an over-eager cap).
    #[tokio::test]
    async fn read_capped_body_passes_small_response_through() {
        use axum::http;

        let payload = b"{\"ok\":true}".to_vec();
        let http_response = http::Response::builder()
            .status(200)
            .body(payload.clone())
            .expect("build synthetic http::Response");
        let response = reqwest::Response::from(http_response);

        let (text, truncated) = ApiCallExecutor::read_capped_body(response).await;

        assert!(!truncated, "a small body must not be flagged truncated");
        assert_eq!(text.as_bytes(), payload.as_slice());
    }
}
