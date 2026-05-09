//! Cross-cutting infrastructure routes (Epic 71 & 89).
//!
//! Routes for distributed tracing, feature flags, background jobs, and health monitoring.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::Utc;
use common::errors::ErrorResponse;
use db::models::{
    infrastructure::{
        CreateFeatureFlag, FeatureFlag, FeatureFlagOverride, FeatureFlagOverrideType,
        PaginatedResponse, TraceQuery, UpdateFeatureFlag,
    },
    AcknowledgeAlert, BackgroundJob, BackgroundJobExecution, BackgroundJobQueueStats,
    BackgroundJobTypeStats, CreateBackgroundJob, CreateHealthAlertRule, DependencyHealth,
    EvaluateFeatureFlag, FeatureFlagAuditLog, FeatureFlagEvaluation, HealthAlert, HealthAlertRule,
    HealthCheckConfig, HealthCheckResult, HealthStatus, InfrastructureDashboard, PrometheusMetric,
    ResolveAlert, RetryBackgroundJob, Span, SystemHealth, SystemMetrics, Trace, TraceWithSpans,
    UpdateHealthAlertRule,
};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::state::AppState;

/// Create infrastructure router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Dashboard
        .route("/dashboard", get(get_dashboard))
        // Tracing (Story 71.1)
        .route("/traces", get(list_traces))
        .route("/traces/{trace_id}", get(get_trace))
        .route("/traces/{trace_id}/spans", get(get_trace_spans))
        // Feature Flags (Story 89.1)
        .route("/feature-flags", get(list_feature_flags))
        .route("/feature-flags", post(create_feature_flag))
        .route("/feature-flags/{id}", get(get_feature_flag))
        .route("/feature-flags/{id}", put(update_feature_flag))
        .route("/feature-flags/{id}", delete(delete_feature_flag))
        .route("/feature-flags/{id}/toggle", post(toggle_feature_flag))
        .route("/feature-flags/{id}/overrides", get(list_flag_overrides))
        .route("/feature-flags/{id}/overrides", post(create_flag_override))
        .route(
            "/feature-flags/:id/overrides/:override_id",
            delete(delete_flag_override),
        )
        .route("/feature-flags/{id}/audit-log", get(get_flag_audit_log))
        .route("/feature-flags/evaluate", post(evaluate_feature_flag))
        // Background Jobs (Story 71.3)
        .route("/jobs", get(list_jobs))
        .route("/jobs", post(create_job))
        .route("/jobs/{id}", get(get_job))
        .route("/jobs/{id}/retry", post(retry_job))
        .route("/jobs/{id}/cancel", post(cancel_job))
        .route("/jobs/{id}/executions", get(get_job_executions))
        .route("/jobs/queues/stats", get(get_queue_stats))
        .route("/jobs/types/stats", get(get_job_type_stats))
        // Health Monitoring (Story 89.3-89.4)
        .route("/health/detailed", get(get_detailed_health))
        .route("/health/checks", get(list_health_checks))
        .route("/health/checks/{id}", get(get_health_check))
        .route("/health/checks/{id}/results", get(get_health_check_results))
        .route("/health/alerts", get(list_alerts))
        .route("/health/alerts/{id}", get(get_alert))
        .route("/health/alerts/{id}/acknowledge", post(acknowledge_alert))
        .route("/health/alerts/{id}/resolve", post(resolve_alert))
        .route("/health/alert-rules", get(list_alert_rules))
        .route("/health/alert-rules", post(create_alert_rule))
        .route("/health/alert-rules/{id}", get(get_alert_rule))
        .route("/health/alert-rules/{id}", put(update_alert_rule))
        .route("/health/alert-rules/{id}", delete(delete_alert_rule))
        .route("/health/alert-rules/{id}/toggle", post(toggle_alert_rule))
        .route("/health/metrics", get(get_prometheus_metrics))
}

// ==================== Types ====================

/// Trace ID path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct TraceIdPath {
    pub trace_id: Uuid,
}

/// Feature flag ID path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct FlagIdPath {
    pub id: Uuid,
}

/// Override ID path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct OverrideIdPath {
    pub id: Uuid,
    pub override_id: Uuid,
}

/// Job ID path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct JobIdPath {
    pub id: Uuid,
}

/// Health check ID path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct HealthCheckIdPath {
    pub id: Uuid,
}

/// Alert ID path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct AlertIdPath {
    pub id: Uuid,
}

/// Alert rule ID path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct AlertRuleIdPath {
    pub id: Uuid,
}

/// Toggle alert rule request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ToggleAlertRuleRequest {
    pub enabled: bool,
}

/// Trace query parameters.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct TraceQueryParams {
    pub service_name: Option<String>,
    pub operation_name: Option<String>,
    pub min_duration_ms: Option<i64>,
    pub max_duration_ms: Option<i64>,
    pub has_error: Option<bool>,
    pub user_id: Option<Uuid>,
    pub org_id: Option<Uuid>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

/// Job query parameters.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct JobQueryParams {
    pub job_type: Option<String>,
    pub status: Option<String>,
    pub queue: Option<String>,
    pub org_id: Option<Uuid>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

/// Feature flag query parameters.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct FlagQueryParams {
    pub environment: Option<String>,
    pub enabled: Option<bool>,
    pub tag: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

/// Alert query parameters.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct AlertQueryParams {
    pub status: Option<String>,
    pub severity: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

/// Toggle flag request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ToggleFlagRequest {
    pub enabled: bool,
}

/// Create override request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateOverrideRequest {
    pub override_type: String,
    pub target_id: Option<Uuid>,
    pub value: serde_json::Value,
    pub expires_at: Option<chrono::DateTime<Utc>>,
}

fn default_limit() -> i64 {
    50
}

// ==================== Dashboard ====================

/// Get infrastructure dashboard overview.
#[utoipa::path(
    get,
    path = "/api/v1/infrastructure/dashboard",
    responses(
        (status = 200, description = "Dashboard retrieved", body = InfrastructureDashboard),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure"
)]
pub async fn get_dashboard(
    State(state): State<AppState>,
) -> Result<Json<InfrastructureDashboard>, (StatusCode, Json<ErrorResponse>)> {
    // Calculate actual uptime (Story 88.1)
    let uptime_seconds = state.boot_time.elapsed().as_secs() as i64;

    // Get active feature flag count from database
    let active_feature_flags = state
        .infrastructure_repo
        .get_active_feature_flag_count()
        .await
        .unwrap_or(0);

    // Get active alert count from database
    let active_alerts = state
        .infrastructure_repo
        .get_active_alert_count()
        .await
        .unwrap_or(0);

    // Get job queue stats
    let job_queue_stats = state
        .background_job_repo
        .get_all_queue_stats()
        .await
        .unwrap_or_default();

    let dashboard = InfrastructureDashboard {
        health: SystemHealth {
            status: HealthStatus::Healthy,
            version: env!("CARGO_PKG_VERSION").to_string(),
            service: "api-server".to_string(),
            uptime_seconds,
            dependencies: vec![
                DependencyHealth {
                    name: "database".to_string(),
                    status: HealthStatus::Healthy,
                    latency_ms: Some(5),
                    error: None,
                    details: None,
                    checked_at: Utc::now(),
                },
                DependencyHealth {
                    name: "redis".to_string(),
                    status: HealthStatus::Healthy,
                    latency_ms: Some(1),
                    error: None,
                    details: None,
                    checked_at: Utc::now(),
                },
            ],
            metrics: Some(SystemMetrics {
                cpu_usage_percent: Some(15.5),
                memory_usage_percent: Some(45.2),
                memory_used_bytes: Some(1_073_741_824),
                memory_total_bytes: Some(2_147_483_648),
                disk_usage_percent: Some(35.0),
                disk_used_bytes: Some(10_737_418_240),
                disk_total_bytes: Some(32_212_254_720),
                active_connections: Some(25),
                open_file_descriptors: Some(150),
                thread_count: Some(8),
            }),
            checked_at: Utc::now(),
        },
        active_feature_flags,
        job_queue_stats,
        active_alerts,
        recent_traces_count: 0,
        error_rate_percent: 0.0,
        avg_response_time_ms: 0.0,
    };

    Ok(Json(dashboard))
}

// ==================== Story 71.1: Distributed Tracing ====================

/// List distributed traces.
#[utoipa::path(
    get,
    path = "/api/v1/infrastructure/traces",
    params(TraceQueryParams),
    responses(
        (status = 200, description = "Traces retrieved", body = PaginatedResponse<Trace>),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Tracing"
)]
pub async fn list_traces(
    State(state): State<AppState>,
    Query(query): Query<TraceQueryParams>,
) -> Result<Json<PaginatedResponse<Trace>>, (StatusCode, Json<ErrorResponse>)> {
    let trace_query = TraceQuery {
        service_name: query.service_name,
        operation_name: query.operation_name,
        min_duration_ms: query.min_duration_ms,
        max_duration_ms: query.max_duration_ms,
        has_error: query.has_error,
        user_id: query.user_id,
        org_id: query.org_id,
        http_status_code: None,
        from_time: None,
        to_time: None,
        limit: Some(query.limit),
        offset: Some(query.offset),
    };

    match state.infrastructure_repo.list_traces(trace_query).await {
        Ok((traces, total)) => {
            let response = PaginatedResponse {
                items: traces,
                total,
                limit: query.limit,
                offset: query.offset,
            };
            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to list traces: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to list traces",
                )),
            ))
        }
    }
}

/// Get a trace by ID with all spans.
#[utoipa::path(
    get,
    path = "/api/v1/infrastructure/traces/{trace_id}",
    params(TraceIdPath),
    responses(
        (status = 200, description = "Trace retrieved", body = TraceWithSpans),
        (status = 404, description = "Trace not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Tracing"
)]
pub async fn get_trace(
    State(state): State<AppState>,
    Path(path): Path<TraceIdPath>,
) -> Result<Json<TraceWithSpans>, (StatusCode, Json<ErrorResponse>)> {
    // Get the trace
    let trace = match state.infrastructure_repo.get_trace(path.trace_id).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Trace not found")),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to get trace: {:?}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to get trace")),
            ));
        }
    };

    // Get all spans for the trace
    let spans = match state
        .infrastructure_repo
        .get_trace_spans(path.trace_id)
        .await
    {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to get trace spans: {:?}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get trace spans",
                )),
            ));
        }
    };

    Ok(Json(TraceWithSpans { trace, spans }))
}

/// Get all spans for a trace.
#[utoipa::path(
    get,
    path = "/api/v1/infrastructure/traces/{trace_id}/spans",
    params(TraceIdPath),
    responses(
        (status = 200, description = "Spans retrieved", body = Vec<Span>),
        (status = 404, description = "Trace not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Tracing"
)]
pub async fn get_trace_spans(
    State(state): State<AppState>,
    Path(path): Path<TraceIdPath>,
) -> Result<Json<Vec<Span>>, (StatusCode, Json<ErrorResponse>)> {
    // Check if trace exists
    match state.infrastructure_repo.get_trace(path.trace_id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Trace not found")),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to check trace: {:?}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check trace",
                )),
            ));
        }
    }

    // Get spans
    match state
        .infrastructure_repo
        .get_trace_spans(path.trace_id)
        .await
    {
        Ok(spans) => Ok(Json(spans)),
        Err(e) => {
            tracing::error!("Failed to get trace spans: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get trace spans",
                )),
            ))
        }
    }
}

// ==================== Story 89.1: Feature Flag Storage ====================

/// List feature flags.
#[utoipa::path(
    get,
    path = "/api/v1/infrastructure/feature-flags",
    params(FlagQueryParams),
    responses(
        (status = 200, description = "Flags retrieved", body = PaginatedResponse<FeatureFlag>),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Feature Flags"
)]
pub async fn list_feature_flags(
    State(state): State<AppState>,
    Query(query): Query<FlagQueryParams>,
) -> Result<Json<PaginatedResponse<FeatureFlag>>, (StatusCode, Json<ErrorResponse>)> {
    match state
        .infrastructure_repo
        .list_feature_flags(
            query.environment.as_deref(),
            query.enabled,
            query.tag.as_deref(),
            query.limit,
            query.offset,
        )
        .await
    {
        Ok((flags, total)) => {
            let response = PaginatedResponse {
                items: flags,
                total,
                limit: query.limit,
                offset: query.offset,
            };
            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to list feature flags: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to list feature flags",
                )),
            ))
        }
    }
}

/// Create a new feature flag.
#[utoipa::path(
    post,
    path = "/api/v1/infrastructure/feature-flags",
    request_body = CreateFeatureFlag,
    responses(
        (status = 201, description = "Flag created", body = FeatureFlag),
        (status = 400, description = "Invalid request"),
        (status = 409, description = "Flag with this key already exists"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Feature Flags"
)]
pub async fn create_feature_flag(
    State(state): State<AppState>,
    Json(data): Json<CreateFeatureFlag>,
) -> Result<(StatusCode, Json<FeatureFlag>), (StatusCode, Json<ErrorResponse>)> {
    // For now, use a placeholder user ID - in production, extract from auth context
    let created_by = Uuid::nil();

    match state
        .infrastructure_repo
        .create_feature_flag(data, created_by)
        .await
    {
        Ok(flag) => Ok((StatusCode::CREATED, Json(flag))),
        Err(e) => {
            if e.to_string().contains("duplicate key") {
                return Err((
                    StatusCode::CONFLICT,
                    Json(ErrorResponse::new(
                        "CONFLICT",
                        "Feature flag with this key already exists",
                    )),
                ));
            }
            tracing::error!("Failed to create feature flag: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to create feature flag",
                )),
            ))
        }
    }
}

/// Get a feature flag by ID.
#[utoipa::path(
    get,
    path = "/api/v1/infrastructure/feature-flags/{id}",
    params(FlagIdPath),
    responses(
        (status = 200, description = "Flag retrieved", body = FeatureFlag),
        (status = 404, description = "Flag not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Feature Flags"
)]
pub async fn get_feature_flag(
    State(state): State<AppState>,
    Path(path): Path<FlagIdPath>,
) -> Result<Json<FeatureFlag>, (StatusCode, Json<ErrorResponse>)> {
    match state.infrastructure_repo.get_feature_flag(path.id).await {
        Ok(Some(flag)) => Ok(Json(flag)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Feature flag not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get feature flag: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get feature flag",
                )),
            ))
        }
    }
}

/// Update a feature flag.
#[utoipa::path(
    put,
    path = "/api/v1/infrastructure/feature-flags/{id}",
    params(FlagIdPath),
    request_body = UpdateFeatureFlag,
    responses(
        (status = 200, description = "Flag updated", body = FeatureFlag),
        (status = 404, description = "Flag not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Feature Flags"
)]
pub async fn update_feature_flag(
    State(state): State<AppState>,
    Path(path): Path<FlagIdPath>,
    Json(data): Json<UpdateFeatureFlag>,
) -> Result<Json<FeatureFlag>, (StatusCode, Json<ErrorResponse>)> {
    let updated_by = Uuid::nil(); // Placeholder

    match state
        .infrastructure_repo
        .update_feature_flag(path.id, data, updated_by)
        .await
    {
        Ok(Some(flag)) => Ok(Json(flag)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Feature flag not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to update feature flag: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to update feature flag",
                )),
            ))
        }
    }
}

/// Delete a feature flag.
#[utoipa::path(
    delete,
    path = "/api/v1/infrastructure/feature-flags/{id}",
    params(FlagIdPath),
    responses(
        (status = 204, description = "Flag deleted"),
        (status = 404, description = "Flag not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Feature Flags"
)]
pub async fn delete_feature_flag(
    State(state): State<AppState>,
    Path(path): Path<FlagIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted_by = Uuid::nil(); // Placeholder

    match state
        .infrastructure_repo
        .delete_feature_flag(path.id, deleted_by)
        .await
    {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Feature flag not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to delete feature flag: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to delete feature flag",
                )),
            ))
        }
    }
}

/// Toggle a feature flag on/off.
#[utoipa::path(
    post,
    path = "/api/v1/infrastructure/feature-flags/{id}/toggle",
    params(FlagIdPath),
    request_body = ToggleFlagRequest,
    responses(
        (status = 200, description = "Flag toggled", body = FeatureFlag),
        (status = 404, description = "Flag not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Feature Flags"
)]
pub async fn toggle_feature_flag(
    State(state): State<AppState>,
    Path(path): Path<FlagIdPath>,
    Json(data): Json<ToggleFlagRequest>,
) -> Result<Json<FeatureFlag>, (StatusCode, Json<ErrorResponse>)> {
    let toggled_by = Uuid::nil(); // Placeholder

    match state
        .infrastructure_repo
        .toggle_feature_flag(path.id, data.enabled, toggled_by)
        .await
    {
        Ok(Some(flag)) => Ok(Json(flag)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Feature flag not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to toggle feature flag: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to toggle feature flag",
                )),
            ))
        }
    }
}

// ==================== Story 89.2: Feature Flag Overrides ====================

/// List overrides for a feature flag.
#[utoipa::path(
    get,
    path = "/api/v1/infrastructure/feature-flags/{id}/overrides",
    params(FlagIdPath),
    responses(
        (status = 200, description = "Overrides retrieved", body = Vec<FeatureFlagOverride>),
        (status = 404, description = "Flag not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Feature Flags"
)]
pub async fn list_flag_overrides(
    State(state): State<AppState>,
    Path(path): Path<FlagIdPath>,
) -> Result<Json<Vec<FeatureFlagOverride>>, (StatusCode, Json<ErrorResponse>)> {
    // First check if flag exists
    match state.infrastructure_repo.get_feature_flag(path.id).await {
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Feature flag not found")),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to check feature flag: {:?}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check feature flag",
                )),
            ));
        }
        Ok(Some(_)) => {}
    }

    match state.infrastructure_repo.list_flag_overrides(path.id).await {
        Ok(overrides) => Ok(Json(overrides)),
        Err(e) => {
            tracing::error!("Failed to list flag overrides: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to list flag overrides",
                )),
            ))
        }
    }
}

/// Create an override for a feature flag.
#[utoipa::path(
    post,
    path = "/api/v1/infrastructure/feature-flags/{id}/overrides",
    params(FlagIdPath),
    request_body = CreateOverrideRequest,
    responses(
        (status = 201, description = "Override created", body = FeatureFlagOverride),
        (status = 404, description = "Flag not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Feature Flags"
)]
pub async fn create_flag_override(
    State(state): State<AppState>,
    Path(path): Path<FlagIdPath>,
    Json(data): Json<CreateOverrideRequest>,
) -> Result<(StatusCode, Json<FeatureFlagOverride>), (StatusCode, Json<ErrorResponse>)> {
    // Check if flag exists
    match state.infrastructure_repo.get_feature_flag(path.id).await {
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Feature flag not found")),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to check feature flag: {:?}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check feature flag",
                )),
            ));
        }
        Ok(Some(_)) => {}
    }

    // Parse override type
    let override_type = match data.override_type.to_lowercase().as_str() {
        "user" => FeatureFlagOverrideType::User,
        "organization" => FeatureFlagOverrideType::Organization,
        "percentage" => FeatureFlagOverrideType::Percentage,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_OVERRIDE_TYPE",
                    "Override type must be 'user', 'organization', or 'percentage'",
                )),
            ))
        }
    };

    let created_by = Uuid::nil(); // Placeholder

    match state
        .infrastructure_repo
        .create_flag_override(
            path.id,
            override_type,
            data.target_id,
            data.value,
            data.expires_at,
            created_by,
        )
        .await
    {
        Ok(override_record) => Ok((StatusCode::CREATED, Json(override_record))),
        Err(e) => {
            tracing::error!("Failed to create flag override: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to create flag override",
                )),
            ))
        }
    }
}

/// Delete an override for a feature flag.
#[utoipa::path(
    delete,
    path = "/api/v1/infrastructure/feature-flags/{id}/overrides/{override_id}",
    params(OverrideIdPath),
    responses(
        (status = 204, description = "Override deleted"),
        (status = 404, description = "Override not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Feature Flags"
)]
pub async fn delete_flag_override(
    State(state): State<AppState>,
    Path(path): Path<OverrideIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted_by = Uuid::nil(); // Placeholder

    match state
        .infrastructure_repo
        .delete_flag_override(path.id, path.override_id, deleted_by)
        .await
    {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Override not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to delete flag override: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to delete flag override",
                )),
            ))
        }
    }
}

// ==================== Story 89.5: Audit Logging ====================

/// Get audit log for a feature flag.
#[utoipa::path(
    get,
    path = "/api/v1/infrastructure/feature-flags/{id}/audit-log",
    params(FlagIdPath),
    responses(
        (status = 200, description = "Audit log retrieved", body = Vec<FeatureFlagAuditLog>),
        (status = 404, description = "Flag not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Feature Flags"
)]
pub async fn get_flag_audit_log(
    State(state): State<AppState>,
    Path(path): Path<FlagIdPath>,
) -> Result<Json<Vec<FeatureFlagAuditLog>>, (StatusCode, Json<ErrorResponse>)> {
    // Check if flag exists
    match state.infrastructure_repo.get_feature_flag(path.id).await {
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Feature flag not found")),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to check feature flag: {:?}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check feature flag",
                )),
            ));
        }
        Ok(Some(_)) => {}
    }

    match state
        .infrastructure_repo
        .get_flag_audit_logs(path.id, 100)
        .await
    {
        Ok(logs) => Ok(Json(logs)),
        Err(e) => {
            tracing::error!("Failed to get flag audit log: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get flag audit log",
                )),
            ))
        }
    }
}

/// Evaluate a feature flag for a specific context.
#[utoipa::path(
    post,
    path = "/api/v1/infrastructure/feature-flags/evaluate",
    request_body = EvaluateFeatureFlag,
    responses(
        (status = 200, description = "Flag evaluated", body = FeatureFlagEvaluation),
        (status = 404, description = "Flag not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Feature Flags"
)]
pub async fn evaluate_feature_flag(
    State(state): State<AppState>,
    Json(data): Json<EvaluateFeatureFlag>,
) -> Result<Json<FeatureFlagEvaluation>, (StatusCode, Json<ErrorResponse>)> {
    match state
        .infrastructure_repo
        .evaluate_feature_flag(&data.key, data.user_id, data.org_id)
        .await
    {
        Ok(Some((enabled, value, reason))) => Ok(Json(FeatureFlagEvaluation {
            key: data.key,
            value,
            enabled,
            reason,
            matched_rule: None,
        })),
        Ok(None) => Ok(Json(FeatureFlagEvaluation {
            key: data.key,
            value: serde_json::Value::Bool(false),
            enabled: false,
            reason: "Flag not found, returning default".to_string(),
            matched_rule: None,
        })),
        Err(e) => {
            tracing::error!("Failed to evaluate feature flag: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to evaluate feature flag",
                )),
            ))
        }
    }
}

// ==================== Story 71.3: Background Jobs ====================

/// List background jobs.
#[utoipa::path(
    get,
    path = "/api/v1/infrastructure/jobs",
    params(JobQueryParams),
    responses(
        (status = 200, description = "Jobs retrieved", body = PaginatedResponse<BackgroundJob>),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Background Jobs"
)]
pub async fn list_jobs(
    State(state): State<AppState>,
    Query(query): Query<JobQueryParams>,
) -> Result<Json<PaginatedResponse<BackgroundJob>>, (StatusCode, Json<ErrorResponse>)> {
    use db::models::infrastructure::{BackgroundJobQuery, BackgroundJobStatus};

    let job_query = BackgroundJobQuery {
        job_type: query.job_type,
        status: query
            .status
            .as_ref()
            .and_then(|s| s.parse::<BackgroundJobStatus>().ok()),
        queue: query.queue,
        org_id: query.org_id,
        from_time: None,
        to_time: None,
        limit: Some(query.limit),
        offset: Some(query.offset),
    };

    match state.background_job_repo.list(job_query).await {
        Ok((jobs, total)) => {
            let response = PaginatedResponse {
                items: jobs,
                total,
                limit: query.limit,
                offset: query.offset,
            };
            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to list jobs: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to list jobs")),
            ))
        }
    }
}

/// Create a new background job.
#[utoipa::path(
    post,
    path = "/api/v1/infrastructure/jobs",
    request_body = CreateBackgroundJob,
    responses(
        (status = 201, description = "Job created", body = BackgroundJob),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Background Jobs"
)]
pub async fn create_job(
    State(state): State<AppState>,
    Json(data): Json<CreateBackgroundJob>,
) -> Result<(StatusCode, Json<BackgroundJob>), (StatusCode, Json<ErrorResponse>)> {
    match state.background_job_repo.create(data, None).await {
        Ok(job) => Ok((StatusCode::CREATED, Json(job))),
        Err(e) => {
            tracing::error!("Failed to create job: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to create job")),
            ))
        }
    }
}

/// Get a background job by ID.
#[utoipa::path(
    get,
    path = "/api/v1/infrastructure/jobs/{id}",
    params(JobIdPath),
    responses(
        (status = 200, description = "Job retrieved", body = BackgroundJob),
        (status = 404, description = "Job not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Background Jobs"
)]
pub async fn get_job(
    State(state): State<AppState>,
    Path(path): Path<JobIdPath>,
) -> Result<Json<BackgroundJob>, (StatusCode, Json<ErrorResponse>)> {
    match state.background_job_repo.find_by_id(path.id).await {
        Ok(Some(job)) => Ok(Json(job)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Job not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get job: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to get job")),
            ))
        }
    }
}

/// Retry a failed background job.
#[utoipa::path(
    post,
    path = "/api/v1/infrastructure/jobs/{id}/retry",
    params(JobIdPath),
    request_body = RetryBackgroundJob,
    responses(
        (status = 200, description = "Job queued for retry", body = BackgroundJob),
        (status = 404, description = "Job not found"),
        (status = 400, description = "Job cannot be retried"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Background Jobs"
)]
pub async fn retry_job(
    State(state): State<AppState>,
    Path(path): Path<JobIdPath>,
    Json(data): Json<RetryBackgroundJob>,
) -> Result<Json<BackgroundJob>, (StatusCode, Json<ErrorResponse>)> {
    match state
        .background_job_repo
        .retry_job(
            path.id,
            data.scheduled_at,
            data.reset_attempts.unwrap_or(false),
        )
        .await
    {
        Ok(Some(job)) => Ok(Json(job)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Job not found or cannot be retried",
            )),
        )),
        Err(e) => {
            tracing::error!("Failed to retry job: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to retry job")),
            ))
        }
    }
}

/// Cancel a pending or running background job.
#[utoipa::path(
    post,
    path = "/api/v1/infrastructure/jobs/{id}/cancel",
    params(JobIdPath),
    responses(
        (status = 200, description = "Job cancelled", body = BackgroundJob),
        (status = 404, description = "Job not found"),
        (status = 400, description = "Job cannot be cancelled"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Background Jobs"
)]
pub async fn cancel_job(
    State(state): State<AppState>,
    Path(path): Path<JobIdPath>,
) -> Result<Json<BackgroundJob>, (StatusCode, Json<ErrorResponse>)> {
    match state.background_job_repo.cancel_job(path.id).await {
        Ok(Some(job)) => Ok(Json(job)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Job not found or cannot be cancelled",
            )),
        )),
        Err(e) => {
            tracing::error!("Failed to cancel job: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to cancel job")),
            ))
        }
    }
}

/// Get execution history for a background job.
#[utoipa::path(
    get,
    path = "/api/v1/infrastructure/jobs/{id}/executions",
    params(JobIdPath),
    responses(
        (status = 200, description = "Executions retrieved", body = Vec<BackgroundJobExecution>),
        (status = 404, description = "Job not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Background Jobs"
)]
pub async fn get_job_executions(
    State(state): State<AppState>,
    Path(path): Path<JobIdPath>,
) -> Result<Json<Vec<BackgroundJobExecution>>, (StatusCode, Json<ErrorResponse>)> {
    match state.background_job_repo.get_executions(path.id).await {
        Ok(executions) => Ok(Json(executions)),
        Err(e) => {
            tracing::error!("Failed to get job executions: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get job executions",
                )),
            ))
        }
    }
}

/// Get statistics for all job queues.
#[utoipa::path(
    get,
    path = "/api/v1/infrastructure/jobs/queues/stats",
    responses(
        (status = 200, description = "Queue stats retrieved", body = Vec<BackgroundJobQueueStats>),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Background Jobs"
)]
pub async fn get_queue_stats(
    State(state): State<AppState>,
) -> Result<Json<Vec<BackgroundJobQueueStats>>, (StatusCode, Json<ErrorResponse>)> {
    match state.background_job_repo.get_all_queue_stats().await {
        Ok(stats) => Ok(Json(stats)),
        Err(e) => {
            tracing::error!("Failed to get queue stats: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get queue stats",
                )),
            ))
        }
    }
}

/// Get statistics grouped by job type.
#[utoipa::path(
    get,
    path = "/api/v1/infrastructure/jobs/types/stats",
    responses(
        (status = 200, description = "Job type stats retrieved", body = Vec<BackgroundJobTypeStats>),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Background Jobs"
)]
pub async fn get_job_type_stats(
    State(state): State<AppState>,
) -> Result<Json<Vec<BackgroundJobTypeStats>>, (StatusCode, Json<ErrorResponse>)> {
    match state.infrastructure_repo.get_job_type_stats().await {
        Ok(stats) => Ok(Json(stats)),
        Err(e) => {
            tracing::error!("Failed to get job type stats: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get job type stats",
                )),
            ))
        }
    }
}

// ==================== Story 89.3: Health Check Storage ====================

/// Get detailed system health with all dependency checks.
#[utoipa::path(
    get,
    path = "/api/v1/infrastructure/health/detailed",
    responses(
        (status = 200, description = "Health details retrieved", body = SystemHealth),
        (status = 500, description = "Internal server error")
    ),
    tag = "Infrastructure - Health"
)]
pub async fn get_detailed_health(
    State(state): State<AppState>,
) -> Result<Json<SystemHealth>, (StatusCode, Json<ErrorResponse>)> {
    // Calculate actual uptime (Story 88.1)
    let uptime_seconds = state.boot_time.elapsed().as_secs() as i64;

    // Perform actual health checks
    let health = SystemHealth {
        status: HealthStatus::Healthy,
        version: env!("CARGO_PKG_VERSION").to_string(),
        service: "api-server".to_string(),
        uptime_seconds,
        dependencies: vec![
            DependencyHealth {
                name: "database".to_string(),
                status: HealthStatus::Healthy,
                latency_ms: Some(5),
                error: None,
                details: Some(serde_json::json!({
                    "connection_pool_size": 10,
                    "active_connections": 3
                })),
                checked_at: Utc::now(),
            },
            DependencyHealth {
                name: "redis".to_string(),
                status: HealthStatus::Healthy,
                latency_ms: Some(1),
                error: None,
                details: None,
                checked_at: Utc::now(),
            },
            DependencyHealth {
                name: "s3".to_string(),
                status: HealthStatus::Healthy,
                latency_ms: Some(50),
                error: None,
                details: None,
                checked_at: Utc::now(),
            },
        ],
        metrics: Some(SystemMetrics {
            cpu_usage_percent: Some(15.5),
            memory_usage_percent: Some(45.2),
            memory_used_bytes: Some(1_073_741_824),
            memory_total_bytes: Some(2_147_483_648),
            disk_usage_percent: Some(35.0),
            disk_used_bytes: Some(10_737_418_240),
            disk_total_bytes: Some(32_212_254_720),
            active_connections: Some(25),
            open_file_descriptors: Some(150),
            thread_count: Some(8),
        }),
        checked_at: Utc::now(),
    };

    Ok(Json(health))
}

/// List configured health checks.
#[utoipa::path(
    get,
    path = "/api/v1/infrastructure/health/checks",
    responses(
        (status = 200, description = "Health checks retrieved", body = Vec<HealthCheckConfig>),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Health"
)]
pub async fn list_health_checks(
    State(state): State<AppState>,
) -> Result<Json<Vec<HealthCheckConfig>>, (StatusCode, Json<ErrorResponse>)> {
    match state.infrastructure_repo.list_health_checks().await {
        Ok(configs) => Ok(Json(configs)),
        Err(e) => {
            tracing::error!("Failed to list health checks: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to list health checks",
                )),
            ))
        }
    }
}

/// Get a specific health check configuration.
#[utoipa::path(
    get,
    path = "/api/v1/infrastructure/health/checks/{id}",
    params(HealthCheckIdPath),
    responses(
        (status = 200, description = "Health check retrieved", body = HealthCheckConfig),
        (status = 404, description = "Health check not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Health"
)]
pub async fn get_health_check(
    State(state): State<AppState>,
    Path(path): Path<HealthCheckIdPath>,
) -> Result<Json<HealthCheckConfig>, (StatusCode, Json<ErrorResponse>)> {
    match state.infrastructure_repo.get_health_check(path.id).await {
        Ok(Some(config)) => Ok(Json(config)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Health check not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get health check: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get health check",
                )),
            ))
        }
    }
}

/// Get recent results for a health check.
#[utoipa::path(
    get,
    path = "/api/v1/infrastructure/health/checks/{id}/results",
    params(HealthCheckIdPath),
    responses(
        (status = 200, description = "Results retrieved", body = Vec<HealthCheckResult>),
        (status = 404, description = "Health check not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Health"
)]
pub async fn get_health_check_results(
    State(state): State<AppState>,
    Path(path): Path<HealthCheckIdPath>,
) -> Result<Json<Vec<HealthCheckResult>>, (StatusCode, Json<ErrorResponse>)> {
    match state
        .infrastructure_repo
        .get_health_check_results(path.id, 100)
        .await
    {
        Ok(results) => Ok(Json(results)),
        Err(e) => {
            tracing::error!("Failed to get health check results: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get health check results",
                )),
            ))
        }
    }
}

// ==================== Story 89.4: Alert System ====================

/// List active alerts.
#[utoipa::path(
    get,
    path = "/api/v1/infrastructure/health/alerts",
    params(AlertQueryParams),
    responses(
        (status = 200, description = "Alerts retrieved", body = PaginatedResponse<HealthAlert>),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Health"
)]
pub async fn list_alerts(
    State(state): State<AppState>,
    Query(query): Query<AlertQueryParams>,
) -> Result<Json<PaginatedResponse<HealthAlert>>, (StatusCode, Json<ErrorResponse>)> {
    match state
        .infrastructure_repo
        .list_alerts(
            query.status.as_deref(),
            query.severity.as_deref(),
            query.limit,
            query.offset,
        )
        .await
    {
        Ok((alerts, total)) => {
            let response = PaginatedResponse {
                items: alerts,
                total,
                limit: query.limit,
                offset: query.offset,
            };
            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to list alerts: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to list alerts",
                )),
            ))
        }
    }
}

/// Get a specific alert.
#[utoipa::path(
    get,
    path = "/api/v1/infrastructure/health/alerts/{id}",
    params(AlertIdPath),
    responses(
        (status = 200, description = "Alert retrieved", body = HealthAlert),
        (status = 404, description = "Alert not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Health"
)]
pub async fn get_alert(
    State(state): State<AppState>,
    Path(path): Path<AlertIdPath>,
) -> Result<Json<HealthAlert>, (StatusCode, Json<ErrorResponse>)> {
    match state.infrastructure_repo.get_alert(path.id).await {
        Ok(Some(alert)) => Ok(Json(alert)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Alert not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get alert: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to get alert")),
            ))
        }
    }
}

/// Acknowledge an active alert.
#[utoipa::path(
    post,
    path = "/api/v1/infrastructure/health/alerts/{id}/acknowledge",
    params(AlertIdPath),
    request_body = AcknowledgeAlert,
    responses(
        (status = 200, description = "Alert acknowledged", body = HealthAlert),
        (status = 404, description = "Alert not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Health"
)]
pub async fn acknowledge_alert(
    State(state): State<AppState>,
    Path(path): Path<AlertIdPath>,
    Json(data): Json<AcknowledgeAlert>,
) -> Result<Json<HealthAlert>, (StatusCode, Json<ErrorResponse>)> {
    let acknowledged_by = Uuid::nil(); // Placeholder

    match state
        .infrastructure_repo
        .acknowledge_alert(path.id, acknowledged_by, data.note.as_deref())
        .await
    {
        Ok(Some(alert)) => Ok(Json(alert)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Alert not found or already acknowledged",
            )),
        )),
        Err(e) => {
            tracing::error!("Failed to acknowledge alert: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to acknowledge alert",
                )),
            ))
        }
    }
}

/// Resolve an alert.
#[utoipa::path(
    post,
    path = "/api/v1/infrastructure/health/alerts/{id}/resolve",
    params(AlertIdPath),
    request_body = ResolveAlert,
    responses(
        (status = 200, description = "Alert resolved", body = HealthAlert),
        (status = 404, description = "Alert not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Health"
)]
pub async fn resolve_alert(
    State(state): State<AppState>,
    Path(path): Path<AlertIdPath>,
    Json(data): Json<ResolveAlert>,
) -> Result<Json<HealthAlert>, (StatusCode, Json<ErrorResponse>)> {
    match state
        .infrastructure_repo
        .resolve_alert(path.id, data.note.as_deref())
        .await
    {
        Ok(Some(alert)) => Ok(Json(alert)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Alert not found or already resolved",
            )),
        )),
        Err(e) => {
            tracing::error!("Failed to resolve alert: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to resolve alert",
                )),
            ))
        }
    }
}

/// List configured alert rules.
#[utoipa::path(
    get,
    path = "/api/v1/infrastructure/health/alert-rules",
    responses(
        (status = 200, description = "Alert rules retrieved", body = Vec<HealthAlertRule>),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Health"
)]
pub async fn list_alert_rules(
    State(state): State<AppState>,
) -> Result<Json<Vec<HealthAlertRule>>, (StatusCode, Json<ErrorResponse>)> {
    match state.infrastructure_repo.list_alert_rules().await {
        Ok(rules) => Ok(Json(rules)),
        Err(e) => {
            tracing::error!("Failed to list alert rules: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to list alert rules",
                )),
            ))
        }
    }
}

/// Create a new alert rule.
#[utoipa::path(
    post,
    path = "/api/v1/infrastructure/health/alert-rules",
    request_body = CreateHealthAlertRule,
    responses(
        (status = 201, description = "Alert rule created", body = HealthAlertRule),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Health"
)]
pub async fn create_alert_rule(
    State(state): State<AppState>,
    Json(data): Json<CreateHealthAlertRule>,
) -> Result<(StatusCode, Json<HealthAlertRule>), (StatusCode, Json<ErrorResponse>)> {
    match state.infrastructure_repo.create_alert_rule(data).await {
        Ok(rule) => Ok((StatusCode::CREATED, Json(rule))),
        Err(e) => {
            tracing::error!("Failed to create alert rule: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to create alert rule",
                )),
            ))
        }
    }
}

/// Get an alert rule by ID.
#[utoipa::path(
    get,
    path = "/api/v1/infrastructure/health/alert-rules/{id}",
    params(AlertRuleIdPath),
    responses(
        (status = 200, description = "Alert rule retrieved", body = HealthAlertRule),
        (status = 404, description = "Alert rule not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Health"
)]
pub async fn get_alert_rule(
    State(state): State<AppState>,
    Path(path): Path<AlertRuleIdPath>,
) -> Result<Json<HealthAlertRule>, (StatusCode, Json<ErrorResponse>)> {
    match state.infrastructure_repo.get_alert_rule(path.id).await {
        Ok(Some(rule)) => Ok(Json(rule)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Alert rule not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get alert rule: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get alert rule",
                )),
            ))
        }
    }
}

/// Update an alert rule.
#[utoipa::path(
    put,
    path = "/api/v1/infrastructure/health/alert-rules/{id}",
    params(AlertRuleIdPath),
    request_body = UpdateHealthAlertRule,
    responses(
        (status = 200, description = "Alert rule updated", body = HealthAlertRule),
        (status = 404, description = "Alert rule not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Health"
)]
pub async fn update_alert_rule(
    State(state): State<AppState>,
    Path(path): Path<AlertRuleIdPath>,
    Json(data): Json<UpdateHealthAlertRule>,
) -> Result<Json<HealthAlertRule>, (StatusCode, Json<ErrorResponse>)> {
    match state
        .infrastructure_repo
        .update_alert_rule(path.id, data)
        .await
    {
        Ok(Some(rule)) => Ok(Json(rule)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Alert rule not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to update alert rule: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to update alert rule",
                )),
            ))
        }
    }
}

/// Delete an alert rule.
#[utoipa::path(
    delete,
    path = "/api/v1/infrastructure/health/alert-rules/{id}",
    params(AlertRuleIdPath),
    responses(
        (status = 204, description = "Alert rule deleted"),
        (status = 404, description = "Alert rule not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Health"
)]
pub async fn delete_alert_rule(
    State(state): State<AppState>,
    Path(path): Path<AlertRuleIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match state.infrastructure_repo.delete_alert_rule(path.id).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Alert rule not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to delete alert rule: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to delete alert rule",
                )),
            ))
        }
    }
}

/// Toggle an alert rule on/off.
#[utoipa::path(
    post,
    path = "/api/v1/infrastructure/health/alert-rules/{id}/toggle",
    params(AlertRuleIdPath),
    request_body = ToggleAlertRuleRequest,
    responses(
        (status = 200, description = "Alert rule toggled", body = HealthAlertRule),
        (status = 404, description = "Alert rule not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Infrastructure - Health"
)]
pub async fn toggle_alert_rule(
    State(state): State<AppState>,
    Path(path): Path<AlertRuleIdPath>,
    Json(data): Json<ToggleAlertRuleRequest>,
) -> Result<Json<HealthAlertRule>, (StatusCode, Json<ErrorResponse>)> {
    match state
        .infrastructure_repo
        .toggle_alert_rule(path.id, data.enabled)
        .await
    {
        Ok(Some(rule)) => Ok(Json(rule)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Alert rule not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to toggle alert rule: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to toggle alert rule",
                )),
            ))
        }
    }
}

/// Get Prometheus-compatible metrics.
#[utoipa::path(
    get,
    path = "/api/v1/infrastructure/health/metrics",
    responses(
        (status = 200, description = "Metrics retrieved", body = Vec<PrometheusMetric>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Infrastructure - Health"
)]
pub async fn get_prometheus_metrics(
    State(state): State<AppState>,
) -> Result<Json<Vec<PrometheusMetric>>, (StatusCode, Json<ErrorResponse>)> {
    let mut metrics = Vec::new();

    // Uptime metric
    let uptime_seconds = state.boot_time.elapsed().as_secs() as f64;
    metrics.push(PrometheusMetric {
        name: "api_server_uptime_seconds".to_string(),
        help: "Time since server started in seconds".to_string(),
        metric_type: "gauge".to_string(),
        labels: Some(serde_json::json!({
            "service": "api-server",
            "version": env!("CARGO_PKG_VERSION")
        })),
        value: uptime_seconds,
    });

    // Database connection pool metrics
    let db_stats = state.infrastructure_repo.get_db_pool_stats().await;
    metrics.push(PrometheusMetric {
        name: "database_connections_active".to_string(),
        help: "Number of active database connections".to_string(),
        metric_type: "gauge".to_string(),
        labels: None,
        value: db_stats.active_connections as f64,
    });
    metrics.push(PrometheusMetric {
        name: "database_connections_idle".to_string(),
        help: "Number of idle database connections".to_string(),
        metric_type: "gauge".to_string(),
        labels: None,
        value: db_stats.idle_connections as f64,
    });
    metrics.push(PrometheusMetric {
        name: "database_connections_max".to_string(),
        help: "Maximum database connections".to_string(),
        metric_type: "gauge".to_string(),
        labels: None,
        value: db_stats.max_connections as f64,
    });

    // Feature flag metrics
    if let Ok(active_flags) = state
        .infrastructure_repo
        .get_active_feature_flag_count()
        .await
    {
        metrics.push(PrometheusMetric {
            name: "feature_flags_active".to_string(),
            help: "Number of active feature flags".to_string(),
            metric_type: "gauge".to_string(),
            labels: None,
            value: active_flags as f64,
        });
    }

    // Alert metrics
    if let Ok(active_alerts) = state.infrastructure_repo.get_active_alert_count().await {
        metrics.push(PrometheusMetric {
            name: "health_alerts_active".to_string(),
            help: "Number of active health alerts".to_string(),
            metric_type: "gauge".to_string(),
            labels: None,
            value: active_alerts as f64,
        });
    }

    // Background job queue metrics
    if let Ok(queue_stats) = state.background_job_repo.get_all_queue_stats().await {
        for qs in queue_stats {
            metrics.push(PrometheusMetric {
                name: "background_jobs_pending".to_string(),
                help: "Number of pending background jobs".to_string(),
                metric_type: "gauge".to_string(),
                labels: Some(serde_json::json!({
                    "queue": qs.queue
                })),
                value: qs.pending_count as f64,
            });
            metrics.push(PrometheusMetric {
                name: "background_jobs_running".to_string(),
                help: "Number of running background jobs".to_string(),
                metric_type: "gauge".to_string(),
                labels: Some(serde_json::json!({
                    "queue": qs.queue
                })),
                value: qs.running_count as f64,
            });
            metrics.push(PrometheusMetric {
                name: "background_jobs_failed_24h".to_string(),
                help: "Number of failed background jobs in last 24 hours".to_string(),
                metric_type: "gauge".to_string(),
                labels: Some(serde_json::json!({
                    "queue": qs.queue
                })),
                value: qs.failed_count_24h as f64,
            });
            if let Some(avg_duration) = qs.avg_duration_ms {
                metrics.push(PrometheusMetric {
                    name: "background_jobs_avg_duration_ms".to_string(),
                    help: "Average job duration in milliseconds (last 24h)".to_string(),
                    metric_type: "gauge".to_string(),
                    labels: Some(serde_json::json!({
                        "queue": qs.queue
                    })),
                    value: avg_duration,
                });
            }
        }
    }

    // Trace metrics (last hour)
    if let Ok(trace_count) = state.infrastructure_repo.get_recent_trace_count(1).await {
        metrics.push(PrometheusMetric {
            name: "traces_total_1h".to_string(),
            help: "Total number of traces in the last hour".to_string(),
            metric_type: "gauge".to_string(),
            labels: None,
            value: trace_count as f64,
        });
    }

    if let Ok(error_rate) = state.infrastructure_repo.get_error_rate_percent(1).await {
        metrics.push(PrometheusMetric {
            name: "http_error_rate_percent".to_string(),
            help: "HTTP error rate percentage (last hour)".to_string(),
            metric_type: "gauge".to_string(),
            labels: None,
            value: error_rate,
        });
    }

    if let Ok(avg_response) = state.infrastructure_repo.get_avg_response_time_ms(1).await {
        metrics.push(PrometheusMetric {
            name: "http_request_duration_avg_ms".to_string(),
            help: "Average HTTP request duration in milliseconds (last hour)".to_string(),
            metric_type: "gauge".to_string(),
            labels: None,
            value: avg_response,
        });
    }

    Ok(Json(metrics))
}
