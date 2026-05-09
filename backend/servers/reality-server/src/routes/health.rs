//! Health & readiness endpoints (Epic 95.3, Epic 104.1).
//!
//! E8 (post-prod-launch): split shallow liveness from deep readiness.
//! Reality-server's `check_pm_api` reaches out to api-server for SSO health,
//! and the cascade was real — when api-server's `/health` flapped on a
//! Redis blip, reality-server's `/health` flapped too, which made
//! reality-web's HEALTHCHECK flap, which made the deploy-server's
//! `wait_until_ready` bail and tear down a fine blue/green flip. Splitting
//! the probes breaks the chain: `/health` is local-only, `/readiness` keeps
//! the deep check for dashboards.

use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use sqlx::Row;
use std::time::Instant;
use utoipa::ToSchema;

use crate::state::{AppState, CacheMetrics};

/// Minimal liveness probe response.
#[derive(Serialize, ToSchema)]
pub struct LivenessResponse {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
}

/// Liveness probe. No I/O. See module docstring for the cascade story.
#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    responses((status = 200, description = "Process alive", body = LivenessResponse))
)]
pub async fn liveness() -> (StatusCode, Json<LivenessResponse>) {
    (
        StatusCode::OK,
        Json(LivenessResponse {
            status: "ok",
            service: "reality-server",
            version: env!("CARGO_PKG_VERSION"),
        }),
    )
}

/// Health status enumeration.
#[derive(Debug, Clone, Copy, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// All systems operational
    Healthy,
    /// Some systems degraded but functional
    Degraded,
    /// Critical systems down
    Unhealthy,
}

/// Dependency health check result.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DependencyHealth {
    /// Name of the dependency
    pub name: String,
    /// Health status
    pub status: HealthStatus,
    /// Response time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    /// Error message if unhealthy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Health check response.
#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    /// Overall service status
    pub status: HealthStatus,
    /// Service version
    pub version: String,
    /// Service name
    pub service: String,
    /// Region/deployment
    pub region: String,
    /// Dependency health checks (Story 95.3)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<DependencyHealth>>,
    /// Cache metrics (Epic 104)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_metrics: Option<CacheMetricsResponse>,
    /// Current timestamp
    pub timestamp: String,
}

/// Cache metrics response (Epic 104).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CacheMetricsResponse {
    /// Health check cache metrics
    pub health_cache: CacheMetricsDetail,
    /// Token validation cache metrics
    pub token_cache: CacheMetricsDetail,
}

/// Detailed cache metrics (Epic 104).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CacheMetricsDetail {
    /// Total cache hits
    pub hits: u64,
    /// Total cache misses
    pub misses: u64,
    /// Total evictions
    pub evictions: u64,
    /// Hit rate percentage
    pub hit_rate_percent: f64,
    /// Current cache size (for token cache only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<usize>,
}

/// Check database connectivity and measure latency.
async fn check_database(pool: &sqlx::PgPool) -> DependencyHealth {
    let start = Instant::now();

    let result = sqlx::query("SELECT 1 as health_check")
        .fetch_one(pool)
        .await;

    let latency_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(row) => {
            // Verify the result
            let _: i32 = row.get("health_check");
            DependencyHealth {
                name: "database".to_string(),
                status: if latency_ms > 1000 {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Healthy
                },
                latency_ms: Some(latency_ms),
                error: None,
            }
        }
        Err(e) => DependencyHealth {
            name: "database".to_string(),
            status: HealthStatus::Unhealthy,
            latency_ms: Some(latency_ms),
            error: Some(format!("Database connection failed: {}", e)),
        },
    }
}

/// Check PM API connectivity and measure latency (Epic 104.1).
///
/// Uses cached result if available and not expired (30 second TTL).
/// This prevents overwhelming the PM API with health checks.
async fn check_pm_api(state: &AppState) -> DependencyHealth {
    // Try to get cached result first
    if let Some(cached) = state.health_cache.get().await {
        tracing::debug!(
            status = %cached.status,
            latency_ms = cached.latency_ms,
            "PM API health check cache hit"
        );

        let health_status = match cached.status.as_str() {
            "healthy" => HealthStatus::Healthy,
            "degraded" => HealthStatus::Degraded,
            _ => HealthStatus::Unhealthy,
        };

        return DependencyHealth {
            name: "pm_api".to_string(),
            status: health_status,
            latency_ms: Some(cached.latency_ms),
            error: cached.error,
        };
    }

    // Cache miss - perform actual health check
    tracing::debug!("PM API health check cache miss, performing check");
    let result = state.pm_api_client.check_health().await;

    // Cache the result
    state.health_cache.set(result.clone()).await;

    let health_status = match result.status.as_str() {
        "healthy" => HealthStatus::Healthy,
        "degraded" => HealthStatus::Degraded,
        _ => HealthStatus::Unhealthy,
    };

    // Log the health check result
    if health_status == HealthStatus::Healthy {
        tracing::info!(
            latency_ms = result.latency_ms,
            version = ?result.version,
            "PM API health check passed"
        );
    } else {
        tracing::warn!(
            status = %result.status,
            latency_ms = result.latency_ms,
            error = ?result.error,
            "PM API health check failed"
        );
    }

    DependencyHealth {
        name: "pm_api".to_string(),
        status: health_status,
        latency_ms: Some(result.latency_ms),
        error: result.error,
    }
}

/// Calculate cache hit rate from metrics.
fn calculate_hit_rate(metrics: &CacheMetrics) -> f64 {
    let total = metrics.hits + metrics.misses;
    if total == 0 {
        0.0
    } else {
        (metrics.hits as f64 / total as f64) * 100.0
    }
}

/// Determine overall health status from dependency checks.
fn determine_overall_status(dependencies: &[DependencyHealth]) -> HealthStatus {
    let has_unhealthy = dependencies
        .iter()
        .any(|d| d.status == HealthStatus::Unhealthy);
    let has_degraded = dependencies
        .iter()
        .any(|d| d.status == HealthStatus::Degraded);

    if has_unhealthy {
        HealthStatus::Unhealthy
    } else if has_degraded {
        HealthStatus::Degraded
    } else {
        HealthStatus::Healthy
    }
}

/// Readiness probe (deep check).
///
/// DB + PM API check. Used by operator dashboards. NOT wired to Docker
/// HEALTHCHECK — see module docstring for why.
#[utoipa::path(
    get,
    path = "/readiness",
    tag = "Health",
    responses(
        (status = 200, description = "Service is ready", body = HealthResponse),
        (status = 503, description = "Service is unhealthy", body = HealthResponse)
    )
)]
pub async fn readiness(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let region = std::env::var("REGION").unwrap_or_else(|_| "local".to_string());

    // Check all dependencies in parallel
    let (db_health, pm_api_health) = tokio::join!(check_database(&state.db), check_pm_api(&state));

    let dependencies = vec![db_health, pm_api_health];
    let overall_status = determine_overall_status(&dependencies);

    // Collect cache metrics (Epic 104)
    let health_cache_metrics = state.health_cache.get_metrics().await;
    let token_cache_metrics = state.token_cache.get_metrics().await;
    let token_cache_size = state.token_cache.size().await;

    let cache_metrics = Some(CacheMetricsResponse {
        health_cache: CacheMetricsDetail {
            hits: health_cache_metrics.hits,
            misses: health_cache_metrics.misses,
            evictions: health_cache_metrics.evictions,
            hit_rate_percent: calculate_hit_rate(&health_cache_metrics),
            size: None, // Health cache has only one entry
        },
        token_cache: CacheMetricsDetail {
            hits: token_cache_metrics.hits,
            misses: token_cache_metrics.misses,
            evictions: token_cache_metrics.evictions,
            hit_rate_percent: calculate_hit_rate(&token_cache_metrics),
            size: Some(token_cache_size),
        },
    });

    let status_code = match overall_status {
        HealthStatus::Healthy => StatusCode::OK,
        HealthStatus::Degraded => StatusCode::OK, // Still return 200 for degraded
        HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    };

    let response = HealthResponse {
        status: overall_status,
        version: env!("CARGO_PKG_VERSION").to_string(),
        service: "reality-server".to_string(),
        region,
        dependencies: Some(dependencies),
        cache_metrics,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    (status_code, Json(response))
}
