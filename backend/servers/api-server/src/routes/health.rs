//! Health & readiness endpoints (Epic 95.3 - Enhanced Health Checks).
//!
//! Story 103.2: Added Redis health check.
//!
//! E8 (post-prod-launch): split shallow liveness from deep readiness so a
//! degraded dependency on one service doesn't cascade-kill its consumers.
//!
//! - `GET /health` → liveness. No I/O. 200 as long as the process is alive
//!   and the listener accepted the request. Used by Docker HEALTHCHECK and
//!   the deploy-server's `wait_until_ready` so a transient DB blip doesn't
//!   flip the container to UNHEALTHY and tear down a working blue/green
//!   flip.
//! - `GET /readiness` → deep check. DB + Redis on api-server (DB + PM API
//!   on reality-server). DB is the only critical dependency: a DB outage
//!   returns 503. Redis failure is reported as `degraded` and stays 200 —
//!   sessions degrade to short-lived in-memory state but the service still
//!   serves requests. Used by operator dashboards and any external monitor
//!   that wants to know if the service is *useful*, not just *up*.
//!
//! Why both: the Docker docs are clear that HEALTHCHECK should be cheap and
//! local; coupling it to upstream availability is the standard recipe for
//! cascading outages.

use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use sqlx::Row;
use std::time::Instant;
use utoipa::ToSchema;

use crate::state::AppState;

/// Minimal liveness probe. No I/O, no AppState. Returning at all means the
/// tokio runtime is scheduling and the listener accepted us.
#[derive(Serialize, ToSchema)]
pub struct LivenessResponse {
    /// Always `"ok"` — clients should match on the HTTP status code, not this
    /// field, but it's here for human-readable curl output.
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
}

/// Liveness probe.
///
/// Distinct from `/readiness` (deep check). Docker HEALTHCHECK and the
/// deploy-server's blue/green wait loop call this; if it ever 5xxs, the
/// container is genuinely gone (panic, lock-up, OOM). DB or Redis being
/// down does not flip this to failure — that's `/readiness`'s job.
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
            service: "api-server",
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
    /// Uptime in seconds (Story 88.1)
    pub uptime_seconds: u64,
    /// Dependency health checks (Story 95.3)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<DependencyHealth>>,
    /// Current timestamp
    pub timestamp: String,
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

/// Check Redis connectivity and measure latency (Story 103.2).
async fn check_redis(redis_client: &Option<integrations::RedisClient>) -> DependencyHealth {
    let start = Instant::now();

    match redis_client {
        Some(client) => {
            let result = client.health_check().await;
            let latency_ms = start.elapsed().as_millis() as u64;

            match result {
                Ok(true) => DependencyHealth {
                    name: "redis".to_string(),
                    status: if latency_ms > 500 {
                        HealthStatus::Degraded
                    } else {
                        HealthStatus::Healthy
                    },
                    latency_ms: Some(latency_ms),
                    error: None,
                },
                // Redis failures map to `Degraded`, not `Unhealthy`:
                // sessions and pub/sub fall back to in-memory state but
                // the service still serves traffic. The module docstring
                // pins this contract — only the DB is treated as a 503-
                // worthy critical dependency.
                Ok(false) => DependencyHealth {
                    name: "redis".to_string(),
                    status: HealthStatus::Degraded,
                    latency_ms: Some(latency_ms),
                    error: Some("Redis PING returned unexpected response".to_string()),
                },
                Err(e) => DependencyHealth {
                    name: "redis".to_string(),
                    status: HealthStatus::Degraded,
                    latency_ms: Some(latency_ms),
                    error: Some(format!("Redis connection failed: {}", e)),
                },
            }
        }
        None => {
            // Redis not configured - report as degraded (not critical)
            DependencyHealth {
                name: "redis".to_string(),
                status: HealthStatus::Degraded,
                latency_ms: None,
                error: Some("Redis not configured".to_string()),
            }
        }
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
/// Reports whether the service is *useful*, not just *up*. Returns 503 if
/// any critical dependency is unhealthy (DB), 200 with `degraded` body if a
/// non-critical dep is unhealthy (Redis), 200 if everything is healthy.
///
/// Use this from operator dashboards and external monitoring. Do NOT wire
/// this to Docker HEALTHCHECK — that's `/health` (liveness). See the module
/// docstring for the rationale.
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
    let uptime_seconds = state.boot_time.elapsed().as_secs();

    // Check all dependencies
    let db_health = check_database(&state.db).await;

    // Story 103.2: Redis health check
    let redis_health = check_redis(&state.redis_client).await;

    let dependencies = vec![db_health, redis_health];
    let overall_status = determine_overall_status(&dependencies);

    let status_code = match overall_status {
        HealthStatus::Healthy => StatusCode::OK,
        HealthStatus::Degraded => StatusCode::OK, // Still return 200 for degraded
        HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    };

    let response = HealthResponse {
        status: overall_status,
        version: env!("CARGO_PKG_VERSION").to_string(),
        service: "api-server".to_string(),
        uptime_seconds,
        dependencies: Some(dependencies),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    (status_code, Json(response))
}
