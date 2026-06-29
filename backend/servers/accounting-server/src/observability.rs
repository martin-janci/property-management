//! Observability module for the accounting server (Epic 95).
//!
//! Provides:
//! - Distributed tracing with OpenTelemetry (Story 95.1)
//! - Enhanced error reporting with Sentry integration (Story 95.2)
//! - Prometheus metrics (Story 95.4)
//!
//! Copied from `reality-server`'s observability module with the service name
//! and default metrics port adjusted for accounting-server. The OTel / Sentry /
//! Prometheus wiring is identical so the deploy/ops tooling treats all three
//! servers uniformly.

use metrics::{counter, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::trace::SdkTracer;
use sentry::ClientInitGuard;
use std::sync::OnceLock;
use std::time::Instant;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
use uuid::Uuid;

/// Global Prometheus handle for metrics export.
static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// OpenTelemetry configuration.
#[derive(Clone, Debug)]
pub struct OtelConfig {
    /// Service name for tracing.
    pub service_name: String,
    /// OTLP endpoint for exporting traces (e.g., "http://localhost:4317").
    pub otlp_endpoint: Option<String>,
    /// Enable OpenTelemetry tracing.
    pub enabled: bool,
}

impl Default for OtelConfig {
    fn default() -> Self {
        Self {
            service_name: "accounting-server".to_string(),
            otlp_endpoint: std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok(),
            enabled: std::env::var("OTEL_ENABLED")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
        }
    }
}

/// Sentry configuration for error tracking.
#[derive(Clone, Debug)]
pub struct SentryConfig {
    /// Sentry DSN.
    pub dsn: Option<String>,
    /// Environment (e.g., "production", "staging", "development").
    pub environment: String,
    /// Release version.
    pub release: Option<String>,
    /// Sample rate for error tracking (0.0 to 1.0).
    pub sample_rate: f32,
    /// Enable Sentry integration.
    pub enabled: bool,
}

impl Default for SentryConfig {
    fn default() -> Self {
        Self {
            dsn: std::env::var("SENTRY_DSN").ok(),
            environment: std::env::var("RUST_ENV").unwrap_or_else(|_| "development".to_string()),
            release: Some(env!("CARGO_PKG_VERSION").to_string()),
            sample_rate: std::env::var("SENTRY_SAMPLE_RATE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1.0),
            enabled: std::env::var("SENTRY_ENABLED")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
        }
    }
}

/// Metrics configuration.
#[derive(Clone, Debug)]
pub struct MetricsConfig {
    /// Port for Prometheus metrics endpoint.
    pub port: u16,
    /// Enable metrics collection.
    pub enabled: bool,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            port: std::env::var("METRICS_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(9092), // Distinct default port (api=9090, reality=9091)
            enabled: std::env::var("METRICS_ENABLED")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true), // Enabled by default
        }
    }
}

/// Initialize OpenTelemetry tracing.
fn init_otel_tracer(config: &OtelConfig) -> Option<SdkTracer> {
    if !config.enabled {
        tracing::info!("OpenTelemetry tracing disabled");
        return None;
    }

    let endpoint = config.otlp_endpoint.as_deref()?;
    tracing::info!(
        "Initializing OpenTelemetry tracing with endpoint: {}",
        endpoint
    );

    // Create OTLP span exporter (tonic/gRPC transport).
    let exporter = match SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
    {
        Ok(e) => e,
        Err(e) => {
            tracing::error!(error = %e, "Failed to create OTLP exporter");
            return None;
        }
    };

    // Build the resource describing this service.
    let resource = opentelemetry_sdk::Resource::builder()
        .with_attributes([
            opentelemetry::KeyValue::new("service.name", config.service_name.clone()),
            opentelemetry::KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        ])
        .build();

    // Build tracer provider with a batch span processor.
    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build();

    let tracer = provider.tracer(config.service_name.clone());

    // Set global provider.
    opentelemetry::global::set_tracer_provider(provider);

    Some(tracer)
}

/// Initialize Sentry for error tracking.
fn init_sentry(config: &SentryConfig) -> Option<ClientInitGuard> {
    if !config.enabled {
        tracing::info!("Sentry error tracking disabled");
        return None;
    }

    let dsn = config.dsn.as_deref()?;
    tracing::info!(
        "Initializing Sentry with environment: {}",
        config.environment
    );

    let guard = sentry::init((
        dsn,
        sentry::ClientOptions {
            release: config.release.clone().map(|v| v.into()),
            environment: Some(config.environment.clone().into()),
            sample_rate: config.sample_rate,
            traces_sample_rate: 0.1, // Sample 10% of transactions
            attach_stacktrace: true,
            send_default_pii: false, // PII redaction
            before_send: Some(std::sync::Arc::new(|mut event| {
                // Redact sensitive data from error events
                redact_pii(&mut event);
                Some(event)
            })),
            ..Default::default()
        },
    ));

    Some(guard)
}

/// Redact PII from Sentry events.
fn redact_pii(event: &mut sentry::protocol::Event) {
    // Redact user information
    if let Some(ref mut user) = event.user {
        // Keep user ID but redact email
        if user.email.is_some() {
            user.email = Some("[REDACTED]".to_string());
        }
        if user.ip_address.is_some() {
            user.ip_address = Some(sentry::protocol::IpAddress::Auto);
        }
    }

    // Redact sensitive headers from request
    if let Some(ref mut request) = event.request {
        let sensitive_headers = ["authorization", "cookie", "x-api-key", "x-auth-token"];
        for (key, value) in request.headers.iter_mut() {
            if sensitive_headers.contains(&key.to_lowercase().as_str()) {
                *value = "[REDACTED]".to_string();
            }
        }
    }

    // Redact sensitive data from extras
    let sensitive_keys = [
        "password",
        "token",
        "secret",
        "api_key",
        "auth",
        "credential",
    ];
    let mut redacted_extra = std::collections::BTreeMap::new();
    for (key, value) in &event.extra {
        let should_redact = sensitive_keys
            .iter()
            .any(|s| key.to_lowercase().contains(s));
        if should_redact {
            redacted_extra.insert(key.clone(), sentry::protocol::Value::from("[REDACTED]"));
        } else {
            redacted_extra.insert(key.clone(), value.clone());
        }
    }
    event.extra = redacted_extra;
}

/// Initialize Prometheus metrics.
fn init_metrics(config: &MetricsConfig) -> Option<PrometheusHandle> {
    if !config.enabled {
        tracing::info!("Prometheus metrics disabled");
        return None;
    }

    tracing::info!("Initializing Prometheus metrics");

    let builder = PrometheusBuilder::new();
    let handle = match builder.install_recorder() {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Failed to install Prometheus recorder: {}", e);
            return None;
        }
    };

    // Store handle globally
    PROMETHEUS_HANDLE.set(handle.clone()).ok();

    // Register default metrics by describing them
    metrics::describe_counter!("http_requests_total", "Total number of HTTP requests");
    metrics::describe_histogram!(
        "http_request_duration_seconds",
        "HTTP request duration in seconds"
    );
    metrics::describe_counter!(
        "http_requests_errors_total",
        "Total number of HTTP request errors"
    );
    metrics::describe_gauge!(
        "database_connections_active",
        "Number of active database connections"
    );
    metrics::describe_gauge!(
        "database_connections_idle",
        "Number of idle database connections"
    );
    metrics::describe_histogram!(
        "database_query_duration_seconds",
        "Database query duration in seconds"
    );
    // Accounting-specific metrics
    metrics::describe_counter!("invoices_created_total", "Total number of invoices created");
    metrics::describe_counter!("payments_matched_total", "Total number of payments matched");
    metrics::describe_counter!("errors_total", "Total number of errors by category");

    Some(handle)
}

/// Error context for enhanced error reporting.
#[derive(Clone, Debug)]
pub struct ErrorContext {
    /// Trace ID for correlation.
    pub trace_id: Option<String>,
    /// User ID if authenticated.
    pub user_id: Option<Uuid>,
    /// Request endpoint.
    pub endpoint: String,
    /// HTTP method.
    pub method: String,
    /// Error category.
    pub category: ErrorCategory,
    /// Region/deployment.
    pub region: String,
}

/// Error categories for classification.
#[derive(Clone, Debug)]
pub enum ErrorCategory {
    /// Authentication/authorization errors.
    Auth,
    /// Validation errors.
    Validation,
    /// Database errors.
    Database,
    /// External service errors.
    ExternalService,
    /// Rate limiting.
    RateLimit,
    /// Internal errors.
    Internal,
    /// Not found errors.
    NotFound,
}

impl std::fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCategory::Auth => write!(f, "auth"),
            ErrorCategory::Validation => write!(f, "validation"),
            ErrorCategory::Database => write!(f, "database"),
            ErrorCategory::ExternalService => write!(f, "external_service"),
            ErrorCategory::RateLimit => write!(f, "rate_limit"),
            ErrorCategory::Internal => write!(f, "internal"),
            ErrorCategory::NotFound => write!(f, "not_found"),
        }
    }
}

/// Report an error to Sentry with context.
pub fn report_error(error: &dyn std::error::Error, context: &ErrorContext) {
    use sentry::protocol::{Event, Level};

    sentry::with_scope(
        |scope| {
            // Add trace ID as tag
            if let Some(ref trace_id) = context.trace_id {
                scope.set_tag("trace_id", trace_id);
            }

            // Add user context (redacted)
            if let Some(user_id) = context.user_id {
                scope.set_user(Some(sentry::User {
                    id: Some(user_id.to_string()),
                    ..Default::default()
                }));
            }

            // Add request context
            scope.set_tag("endpoint", &context.endpoint);
            scope.set_tag("method", &context.method);
            scope.set_tag("error_category", context.category.to_string());
            scope.set_tag("region", &context.region);
        },
        || {
            let mut event = Event::new();
            event.message = Some(error.to_string());
            event.level = match context.category {
                ErrorCategory::Internal | ErrorCategory::Database => Level::Error,
                ErrorCategory::ExternalService => Level::Warning,
                _ => Level::Info,
            };
            sentry::capture_event(event);
        },
    );

    // Also increment error counter
    counter!("errors_total").increment(1);
}

/// Request metrics tracking.
pub struct RequestMetrics {
    start_time: Instant,
    method: String,
    path: String,
}

impl RequestMetrics {
    /// Start tracking a request.
    pub fn new(method: &str, path: &str) -> Self {
        Self {
            start_time: Instant::now(),
            method: method.to_string(),
            path: path.to_string(),
        }
    }

    /// Record request completion.
    pub fn record(self, status_code: u16) {
        let duration = self.start_time.elapsed().as_secs_f64();

        // Record request duration
        histogram!("http_request_duration_seconds").record(duration);

        // Increment request counter
        counter!("http_requests_total").increment(1);

        // Track errors separately
        if status_code >= 400 {
            counter!("http_requests_errors_total").increment(1);
        }
    }
}

/// Get Prometheus metrics as text.
pub fn get_metrics_text() -> String {
    PROMETHEUS_HANDLE
        .get()
        .map(|h| h.render())
        .unwrap_or_else(|| "# Metrics not initialized\n".to_string())
}

/// Observability guard that holds initialized resources.
pub struct ObservabilityGuard {
    _sentry_guard: Option<ClientInitGuard>,
    _prometheus_handle: Option<PrometheusHandle>,
}

/// Initialize all observability components.
pub fn init_observability(
    otel_config: OtelConfig,
    sentry_config: SentryConfig,
    metrics_config: MetricsConfig,
) -> ObservabilityGuard {
    // Initialize Prometheus metrics first (before tracing subscriber)
    let prometheus_handle = init_metrics(&metrics_config);

    // Initialize Sentry
    let sentry_guard = init_sentry(&sentry_config);

    // Build tracing subscriber with layers
    let registry = tracing_subscriber::registry();

    // Add env filter
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "accounting_server=debug,tower_http=debug".into());

    // Add JSON formatting for structured logs
    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(true)
        .with_file(true)
        .with_line_number(true)
        .with_filter(env_filter);

    // Try to add OpenTelemetry layer if enabled
    if let Some(tracer) = init_otel_tracer(&otel_config) {
        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        // Add Sentry layer if enabled
        if sentry_guard.is_some() {
            registry
                .with(fmt_layer)
                .with(otel_layer)
                .with(sentry::integrations::tracing::layer())
                .init();
        } else {
            registry.with(fmt_layer).with(otel_layer).init();
        }
    } else {
        // No OpenTelemetry, but maybe Sentry
        if sentry_guard.is_some() {
            registry
                .with(fmt_layer)
                .with(sentry::integrations::tracing::layer())
                .init();
        } else {
            // Just basic tracing
            registry.with(fmt_layer).init();
        }
    }

    ObservabilityGuard {
        _sentry_guard: sentry_guard,
        _prometheus_handle: prometheus_handle,
    }
}

/// Get current trace ID from the active span.
pub fn current_trace_id() -> Option<String> {
    use opentelemetry::trace::TraceContextExt;
    use tracing_opentelemetry::OpenTelemetrySpanExt;

    let span = tracing::Span::current();
    let context = span.context();
    let span_ref = context.span();
    let span_context = span_ref.span_context();

    if span_context.is_valid() {
        Some(span_context.trace_id().to_string())
    } else {
        None
    }
}
