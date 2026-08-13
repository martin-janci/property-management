//! API call (webhook) action executor (Epic 94, Story 94.1).
//!
//! Makes HTTP requests to external APIs as part of workflow execution.

use super::{ActionContext, ActionError, ActionExecutor, ActionResult};
use async_trait::async_trait;
use db::models::action_type;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::net::IpAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Type-erased hostname → IPs resolver used by [`SsrfGuardResolver`].
///
/// Injected so the executor's real DNS path and the hermetic
/// DNS-rebinding regression test share one code path (the test drives a
/// stateful resolver that returns a public IP first and a private IP on the
/// connect-time lookup).
type IpResolver = Arc<
    dyn Fn(String) -> Pin<Box<dyn Future<Output = std::io::Result<Vec<IpAddr>>> + Send>>
        + Send
        + Sync,
>;

/// Connect-time SSRF-guard DNS resolver for the workflow HTTP client.
///
/// `reqwest`/`hyper` invoke this at **connect** time for the initial request
/// **and every redirect hop**. It resolves the hostname and validates every
/// resulting address against the shared SSRF forbidden-range policy, erroring
/// out (which aborts the connection) if any address is forbidden. Because the
/// addresses `reqwest` connects to are exactly the ones validated here —
/// resolved once, at connect, in the same step — this closes the
/// DNS-rebinding TOCTOU window that a separate pre-flight lookup left open (a
/// pre-flight lookup and `reqwest`'s own connect-time lookup could disagree,
/// letting a hostname that first resolved public rebind to a private address
/// before the socket is opened). It also re-validates redirects, which a
/// one-shot pre-flight check never covered.
struct SsrfGuardResolver {
    resolve_ips: IpResolver,
}

impl reqwest::dns::Resolve for SsrfGuardResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let resolve_ips = self.resolve_ips.clone();
        Box::pin(async move {
            let host = name.as_str().to_string();
            // The port is irrelevant to SSRF validation (range checks are on
            // the IP) and `hyper` overrides it with the real destination port
            // after resolution, so we resolve/validate with 0.
            let ips = (resolve_ips)(host.clone())
                .await
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
            let addrs = common::url_validation::validate_resolved_addrs(&host, 0, &ips)
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
            let iter: reqwest::dns::Addrs = Box::new(addrs.into_iter());
            Ok(iter)
        })
    }
}

/// Maximum number of redirect hops the workflow HTTP client will follow before
/// aborting. `reqwest`'s default is also 10, but we set it explicitly because
/// the client below installs a *custom* redirect policy (which otherwise has no
/// hop limit of its own) and the bound is a security-relevant part of the SSRF
/// posture, not an incidental default.
const MAX_REDIRECTS: usize = 10;

/// Decide whether a single redirect hop may be followed.
///
/// Extracted from the [`reqwest::redirect::Policy`] closure in
/// [`ApiCallExecutor::with_ip_resolver`] so the SSRF re-validation of redirect
/// targets is unit-testable *without* a live HTTP server: the SSRF DNS guard
/// (`SsrfGuardResolver`) refuses to connect to a loopback address, so a
/// redirect-following test cannot stand up a `127.0.0.1` server to emit a 3xx —
/// the only hermetic way to exercise the redirect branch is this pure decision
/// function.
///
/// `Ok(())` means the hop is safe to follow; `Err(reason)` aborts the whole
/// request (fail closed). A hop is refused when:
///   * the redirect chain has already reached [`MAX_REDIRECTS`], or
///   * the redirect target fails [`common::url_validation::validate_external_url`]
///     — i.e. it points at a blocked host, a private/reserved literal IP, an
///     `http://` downgrade (outside development), or a `.local` / `.internal`
///     TLD. This is what stops a redirect from bouncing an initially-valid
///     request onto an internal/blocked host after the initial-URL check has
///     already passed.
fn redirect_hop_allowed(target: &str, previous_hops: usize) -> Result<(), String> {
    if previous_hops >= MAX_REDIRECTS {
        return Err("too many redirects".to_string());
    }
    common::url_validation::validate_external_url(target)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Real DNS resolver used in production: the OS resolver via Tokio's async
/// wrapper around `getaddrinfo`.
fn real_ip_resolver() -> IpResolver {
    Arc::new(
        |host: String| -> Pin<Box<dyn Future<Output = std::io::Result<Vec<IpAddr>>> + Send>> {
            Box::pin(async move {
                let addrs = tokio::net::lookup_host((host.as_str(), 0)).await?;
                Ok(addrs.map(|sa| sa.ip()).collect::<Vec<IpAddr>>())
            })
        },
    )
}

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
    /// The same resolver the HTTP client uses at connect time, kept so the
    /// fail-fast pre-flight check and the connect-time re-validation observe
    /// one DNS path (and so the rebinding regression test can inject a
    /// stateful resolver that drives both).
    resolve_ips: IpResolver,
}

impl ApiCallExecutor {
    /// Create a new API call executor backed by the real OS DNS resolver.
    pub fn new() -> Self {
        Self::with_ip_resolver(real_ip_resolver())
    }

    /// Build an executor whose HTTP client resolves and SSRF-validates every
    /// connection (initial request + redirects) through `resolve_ips`. The
    /// connect-time [`SsrfGuardResolver`] is what closes the DNS-rebinding
    /// TOCTOU: `reqwest` connects only to addresses this resolver validated.
    fn with_ip_resolver(resolve_ips: IpResolver) -> Self {
        // Re-validate every redirect hop's URL statically before following it.
        // The connect-time `SsrfGuardResolver` already blocks any hop that
        // resolves to a forbidden IP (including rebinds), but the static
        // `validate_external_url` additionally rejects a redirect that
        // downgrades to plain `http://`, or targets a blocked hostname /
        // `.local` / `.internal` TLD, which an IP-range check alone would not
        // catch. Fail closed: an invalid hop aborts the request.
        let redirect_policy =
            reqwest::redirect::Policy::custom(|attempt| {
                match redirect_hop_allowed(attempt.url().as_str(), attempt.previous().len()) {
                    Ok(()) => attempt.follow(),
                    Err(reason) => attempt.error(reason),
                }
            });

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .user_agent("PPT-Workflow/1.0")
            .redirect(redirect_policy)
            .dns_resolver(Arc::new(SsrfGuardResolver {
                resolve_ips: resolve_ips.clone(),
            }))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            resolve_ips,
        }
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
        // address at request time sails straight through it. Two layers guard
        // the DNS path:
        //
        //   1. This fail-fast pre-flight: resolve now and reject if *any*
        //      resolved address is forbidden, so an obviously-internal or
        //      unresolvable host is rejected with a clean `ConfigurationError`
        //      before a request is ever built.
        //   2. The connect-time `SsrfGuardResolver` wired into `self.client`
        //      (see `with_ip_resolver`), which re-resolves-and-validates at
        //      the moment `reqwest` opens the socket, for the initial request
        //      and every redirect hop. THIS is what actually closes the
        //      DNS-rebinding TOCTOU: a hostname that resolves public here but
        //      rebinds to a private address before connect is rejected at
        //      connect, because the client connects only to addresses the
        //      resolver validated. Both layers share `self.resolve_ips`, so
        //      the pre-flight and the connection observe one DNS code path.
        //
        // Fails closed: a resolution error also rejects.
        let host = parsed_url.host_str().ok_or_else(|| {
            ActionError::ConfigurationError("API call URL must have a host".to_string())
        })?;
        let port = parsed_url.port_or_known_default().unwrap_or(443);
        let resolved_ips = (self.resolve_ips)(host.to_string()).await.map_err(|e| {
            ActionError::ConfigurationError(
                common::url_validation::UrlValidationError::ResolutionFailed(e.to_string())
                    .to_string(),
            )
        })?;
        common::url_validation::validate_resolved_addrs(host, port, &resolved_ips)
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

    /// IG3 regression (SSRF DNS-rebinding TOCTOU, #2703): the pre-fix executor
    /// resolved+validated the host **once** in a pre-flight lookup, then let
    /// `reqwest` resolve the same host **again** at connect time. An attacker
    /// controlling DNS could answer the first lookup with a public IP (passing
    /// validation) and the second with a private/link-local IP (the socket the
    /// client actually opens) — a classic time-of-check/time-of-use bypass of
    /// the anti-SSRF gate.
    ///
    /// The fix wires a connect-time `SsrfGuardResolver` into the HTTP client so
    /// the addresses `reqwest` connects to are exactly the ones re-validated at
    /// connect. This test models the rebind with a hermetic, stateful resolver
    /// (no real network): the **first** resolution — the pre-flight — returns a
    /// public IP so validation passes, and the **second** — `reqwest`'s
    /// connect-time lookup — returns `10.0.0.5`. The request must be rejected,
    /// and the resolver must have been consulted at least twice (proving the
    /// connect-time layer ran and observed the rebind).
    ///
    /// Fails on `dev`: there the client uses its own default resolver, so a
    /// rebind to a private IP at connect is not re-validated and the request
    /// would proceed to open a socket to the private address rather than being
    /// rejected here.
    #[tokio::test]
    async fn execute_rejects_dns_rebinding_to_private_ip_at_connect() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let calls = Arc::new(AtomicUsize::new(0));
        let calls_in_resolver = calls.clone();

        // Stateful rebinding resolver: lookup #0 (pre-flight validation) →
        // public; lookup #1+ (connect-time) → private.
        let resolver: IpResolver = Arc::new(
            move |_host: String| -> Pin<Box<dyn Future<Output = std::io::Result<Vec<IpAddr>>> + Send>> {
                let calls = calls_in_resolver.clone();
                Box::pin(async move {
                    let n = calls.fetch_add(1, Ordering::SeqCst);
                    let ip: IpAddr = if n == 0 {
                        "93.184.216.34".parse().unwrap() // public — first (pre-flight) lookup
                    } else {
                        "10.0.0.5".parse().unwrap() // private — connect-time rebind
                    };
                    Ok(vec![ip])
                })
            },
        );

        let executor = ApiCallExecutor::with_ip_resolver(resolver);
        let context = ActionContext::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            serde_json::json!({}),
        );
        let config = serde_json::json!({ "url": "https://rebind.attacker.example/webhook" });

        let result = executor.execute(&config, &context).await;

        assert!(
            result.is_err(),
            "a host that rebinds to a private IP at connect must be rejected, got: {result:?}"
        );
        assert!(
            calls.load(Ordering::SeqCst) >= 2,
            "expected the connect-time resolver to re-resolve after the pre-flight \
             lookup (rebinding observed), but it was consulted {} time(s)",
            calls.load(Ordering::SeqCst)
        );
    }

    /// IG3 regression (SSRF via HTTP redirect): the workflow HTTP client is
    /// built with a *custom* redirect policy. Before that policy existed the
    /// client followed up to 10 redirects with `reqwest`'s default policy,
    /// which re-validates nothing — so an attacker-controlled endpoint that
    /// first passed `validate_external_url` could reply `302 Location:
    /// http://169.254.169.254/…` (or any private/blocked host) and the client
    /// would happily follow the hop straight into the internal network,
    /// defeating the entire SSRF guard.
    ///
    /// The redirect decision lives in `redirect_hop_allowed`, which the policy
    /// closure calls for every hop. This test exercises that decision directly
    /// (the SSRF DNS guard blocks standing up a loopback test server, so the
    /// redirect branch can only be reached hermetically via this pure
    /// function): a hop whose target is a private or link-local/metadata
    /// literal IP must be **refused** (`Err`), which makes the policy return
    /// `attempt.error(..)` and aborts the request rather than following it.
    ///
    /// Both targets asserted here are rejected by `validate_external_url`
    /// unconditionally (private + link-local ranges are forbidden regardless of
    /// `RUST_ENV`), so the assertion is environment-independent.
    ///
    /// Fails on `dev` before the redirect policy was wired in: there the client
    /// used `reqwest`'s default follow-everything policy with no per-hop
    /// re-validation, so a redirect to a blocked/internal host WAS followed.
    #[test]
    fn redirect_to_blocked_or_internal_host_is_not_followed() {
        // Redirect target = private RFC1918 host → must be refused.
        assert!(
            redirect_hop_allowed("https://10.0.0.1/internal", 0).is_err(),
            "a redirect to a private (10.0.0.0/8) host must not be followed"
        );
        // Redirect target = link-local cloud-metadata endpoint → must be refused.
        assert!(
            redirect_hop_allowed("https://169.254.169.254/latest/meta-data/", 0).is_err(),
            "a redirect to the 169.254.169.254 metadata host must not be followed"
        );
    }

    /// Companion: a redirect to a legitimate public `https://` host is allowed
    /// (guards against an over-broad policy that would break normal redirects),
    /// while a redirect chain that has reached `MAX_REDIRECTS` is refused
    /// (bounded-hops half of the policy).
    #[test]
    fn redirect_hop_allows_public_https_and_bounds_chain_length() {
        assert!(
            redirect_hop_allowed("https://api.example.com/next", 0).is_ok(),
            "a redirect to a public https host should be followed"
        );
        assert!(
            redirect_hop_allowed("https://api.example.com/next", MAX_REDIRECTS).is_err(),
            "a redirect chain at the hop limit must be refused"
        );
    }
}
