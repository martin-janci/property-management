//! Health & readiness endpoints (Epic 95.3).
//!
//! Mirrors reality-server's split between a shallow, dependency-free liveness
//! probe (`/health`, the Docker HEALTHCHECK target) and a deep readiness probe
//! (`/readiness`) that checks the database. The accounting server has no
//! upstream API dependency to probe, so readiness checks the DB only.

use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use sqlx::Row;
use std::time::Instant;
use utoipa::ToSchema;

use crate::state::AppState;

/// Minimal liveness probe response.
#[derive(Serialize, ToSchema)]
pub struct LivenessResponse {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
}

/// Liveness probe. No I/O — process-alive only. Docker HEALTHCHECK target.
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
            service: "accounting-server",
            version: env!("CARGO_PKG_VERSION"),
        }),
    )
}

/// Health status enumeration.
#[derive(Debug, Clone, Copy, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// All systems operational.
    Healthy,
    /// Some systems degraded but functional.
    Degraded,
    /// Critical systems down.
    Unhealthy,
}

/// Dependency health check result.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DependencyHealth {
    /// Name of the dependency.
    pub name: String,
    /// Health status.
    pub status: HealthStatus,
    /// Response time in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    /// Error message if unhealthy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Deep readiness response.
#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    /// Overall service status.
    pub status: HealthStatus,
    /// Service version.
    pub version: String,
    /// Service name.
    pub service: String,
    /// Region/deployment.
    pub region: String,
    /// Dependency health checks (Story 95.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<DependencyHealth>>,
    /// Current timestamp.
    pub timestamp: String,
}

/// Check database connectivity and measure latency.
// SAFETY: intentionally cross-tenant — pure connectivity probe (`SELECT 1`);
// it reads no tenant data.
async fn check_database(pool: &sqlx::PgPool) -> DependencyHealth {
    let start = Instant::now();

    let result = sqlx::query("SELECT 1 as health_check")
        .fetch_one(pool)
        .await;

    let latency_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(row) => {
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

/// Readiness probe (deep check). DB connectivity only; used by operator
/// dashboards, NOT wired to Docker HEALTHCHECK.
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
    let region = state.config.region.clone();

    // SAFETY: intentionally cross-tenant — `check_database` runs `SELECT 1`;
    // no tenant data is read.
    let db_health = check_database(&state.db).await;

    let dependencies = vec![db_health];
    let overall_status = determine_overall_status(&dependencies);

    let status_code = match overall_status {
        HealthStatus::Healthy => StatusCode::OK,
        HealthStatus::Degraded => StatusCode::OK, // Still 200 for degraded
        HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    };

    let response = HealthResponse {
        status: overall_status,
        version: env!("CARGO_PKG_VERSION").to_string(),
        service: "accounting-server".to_string(),
        region,
        dependencies: Some(dependencies),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    (status_code, Json(response))
}
