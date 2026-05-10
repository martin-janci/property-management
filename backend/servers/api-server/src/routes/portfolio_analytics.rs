//! Portfolio Analytics routes (Epic 140).
//! Multi-Property Portfolio Analytics with benchmarking, KPIs, and trend analysis.

use crate::state::AppState;
use api_core::extractors::AuthUser;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::NaiveDate;
use common::errors::ErrorResponse;
use db::models::portfolio_analytics::{
    AcknowledgeAlert, AggregationPeriod, CreateAlertRule, CreatePortfolioBenchmark,
    CreatePropertyComparison, CreatePropertyMetrics, RecordTrend, ResolveAlert, UpdateAlertRule,
    UpdatePortfolioBenchmark,
};
use serde::Deserialize;
use serde_json::json;
use utoipa::IntoParams;
use uuid::Uuid;

type ApiResult<T> = Result<T, (StatusCode, Json<ErrorResponse>)>;

/// Helper to serialize a value to JSON.
fn to_json_value<T: serde::Serialize>(value: T) -> ApiResult<serde_json::Value> {
    serde_json::to_value(value).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&format!(
                "Failed to serialize: {}",
                e
            ))),
        )
    })
}

pub fn router() -> Router<AppState> {
    Router::new()
        // Portfolio Summary
        .route("/summary", get(get_portfolio_summary))
        // Benchmarks
        .route("/benchmarks", get(list_benchmarks))
        .route("/benchmarks", post(create_benchmark))
        .route("/benchmarks/{id}", get(get_benchmark))
        .route("/benchmarks/{id}", put(update_benchmark))
        .route("/benchmarks/{id}", delete(delete_benchmark))
        // Property Metrics
        .route("/properties/metrics", get(list_property_metrics))
        .route("/properties/metrics", post(upsert_property_metrics))
        .route(
            "/properties/{building_id}/metrics",
            get(get_property_metrics),
        )
        // Portfolio Metrics
        .route("/metrics", get(get_portfolio_metrics))
        .route("/metrics/calculate", post(calculate_portfolio_metrics))
        // Property Comparisons
        .route("/comparisons", get(list_comparisons))
        .route("/comparisons", post(create_comparison))
        .route("/comparisons/{id}", get(get_comparison))
        .route("/comparisons/{id}", delete(delete_comparison))
        // Trends
        .route("/trends", get(get_trends))
        .route("/trends", post(record_trend))
        // Alert Rules
        .route("/alerts/rules", get(list_alert_rules))
        .route("/alerts/rules", post(create_alert_rule))
        .route("/alerts/rules/{id}", get(get_alert_rule))
        .route("/alerts/rules/{id}", put(update_alert_rule))
        .route("/alerts/rules/{id}", delete(delete_alert_rule))
        // Alerts
        .route("/alerts", get(list_alerts))
        .route("/alerts/stats", get(get_alert_stats))
        .route("/alerts/{id}", get(get_alert))
        .route("/alerts/{id}/acknowledge", post(acknowledge_alert))
        .route("/alerts/{id}/resolve", post(resolve_alert))
}

// =============================================================================
// PORTFOLIO SUMMARY
// =============================================================================

async fn get_portfolio_summary(
    State(s): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .portfolio_analytics_repo
        .get_portfolio_summary(org_id)
        .await
    {
        Ok(summary) => Ok(Json(to_json_value(summary)?)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

// =============================================================================
// BENCHMARKS
// =============================================================================

async fn list_benchmarks(
    State(s): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s.portfolio_analytics_repo.list_benchmarks(org_id).await {
        Ok(benchmarks) => Ok(Json(json!({ "benchmarks": benchmarks }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn create_benchmark(
    State(s): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreatePortfolioBenchmark>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .portfolio_analytics_repo
        .create_benchmark(org_id, req)
        .await
    {
        Ok(benchmark) => Ok(Json(to_json_value(benchmark)?)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

async fn get_benchmark(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s.portfolio_analytics_repo.get_benchmark(id, org_id).await {
        Ok(Some(benchmark)) => Ok(Json(to_json_value(benchmark)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Benchmark not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn update_benchmark(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdatePortfolioBenchmark>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .portfolio_analytics_repo
        .update_benchmark(id, org_id, req)
        .await
    {
        Ok(Some(benchmark)) => Ok(Json(to_json_value(benchmark)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Benchmark not found")),
        )),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

async fn delete_benchmark(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .portfolio_analytics_repo
        .delete_benchmark(id, org_id)
        .await
    {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Benchmark not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

// =============================================================================
// PROPERTY METRICS
// =============================================================================

#[derive(Debug, Deserialize, IntoParams)]
struct PropertyMetricsQuery {
    building_id: Option<Uuid>,
    period_type: Option<AggregationPeriod>,
}

async fn list_property_metrics(
    State(s): State<AppState>,
    user: AuthUser,
    Query(query): Query<PropertyMetricsQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .portfolio_analytics_repo
        .list_property_metrics(org_id, query.building_id, query.period_type)
        .await
    {
        Ok(metrics) => Ok(Json(json!({ "metrics": metrics }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn upsert_property_metrics(
    State(s): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreatePropertyMetrics>,
) -> ApiResult<Json<serde_json::Value>> {
    let _org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .portfolio_analytics_repo
        .upsert_property_metrics(req)
        .await
    {
        Ok(metrics) => Ok(Json(to_json_value(metrics)?)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

#[derive(Debug, Deserialize, IntoParams)]
struct GetPropertyMetricsQuery {
    period_start: NaiveDate,
    period_end: NaiveDate,
}

async fn get_property_metrics(
    State(s): State<AppState>,
    user: AuthUser,
    Path(building_id): Path<Uuid>,
    Query(query): Query<GetPropertyMetricsQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let _org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .portfolio_analytics_repo
        .get_property_metrics(building_id, query.period_start, query.period_end)
        .await
    {
        Ok(Some(metrics)) => Ok(Json(to_json_value(metrics)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Metrics not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

// =============================================================================
// PORTFOLIO METRICS
// =============================================================================

#[derive(Debug, Deserialize, IntoParams)]
struct PortfolioMetricsQuery {
    period_start: NaiveDate,
    period_end: NaiveDate,
}

async fn get_portfolio_metrics(
    State(s): State<AppState>,
    user: AuthUser,
    Query(query): Query<PortfolioMetricsQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .portfolio_analytics_repo
        .get_portfolio_metrics(org_id, query.period_start, query.period_end)
        .await
    {
        Ok(Some(metrics)) => Ok(Json(to_json_value(metrics)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Portfolio metrics not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

#[derive(Debug, Deserialize)]
struct CalculateMetricsRequest {
    period_start: NaiveDate,
    period_end: NaiveDate,
    #[serde(default = "default_period")]
    period_type: AggregationPeriod,
}

fn default_period() -> AggregationPeriod {
    AggregationPeriod::Monthly
}

async fn calculate_portfolio_metrics(
    State(s): State<AppState>,
    user: AuthUser,
    Json(req): Json<CalculateMetricsRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .portfolio_analytics_repo
        .calculate_portfolio_metrics(org_id, req.period_start, req.period_end, req.period_type)
        .await
    {
        Ok(metrics) => Ok(Json(to_json_value(metrics)?)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

// =============================================================================
// PROPERTY COMPARISONS
// =============================================================================

#[derive(Debug, Deserialize, IntoParams)]
struct ComparisonListQuery {
    #[serde(default)]
    saved_only: bool,
}

async fn list_comparisons(
    State(s): State<AppState>,
    user: AuthUser,
    Query(query): Query<ComparisonListQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .portfolio_analytics_repo
        .list_comparisons(org_id, query.saved_only)
        .await
    {
        Ok(comparisons) => Ok(Json(json!({ "comparisons": comparisons }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn create_comparison(
    State(s): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreatePropertyComparison>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    let user_id = user.user_id;

    match s
        .portfolio_analytics_repo
        .create_comparison(org_id, user_id, req)
        .await
    {
        Ok(comparison) => Ok(Json(to_json_value(comparison)?)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

async fn get_comparison(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s.portfolio_analytics_repo.get_comparison(id, org_id).await {
        Ok(Some(comparison)) => Ok(Json(to_json_value(comparison)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Comparison not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn delete_comparison(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .portfolio_analytics_repo
        .delete_comparison(id, org_id)
        .await
    {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Comparison not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

// =============================================================================
// TRENDS
// =============================================================================

#[derive(Debug, Deserialize, IntoParams)]
struct TrendsQuery {
    metric_name: String,
    building_id: Option<Uuid>,
    from_date: Option<NaiveDate>,
    to_date: Option<NaiveDate>,
}

async fn get_trends(
    State(s): State<AppState>,
    user: AuthUser,
    Query(query): Query<TrendsQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .portfolio_analytics_repo
        .get_trends(
            org_id,
            &query.metric_name,
            query.building_id,
            query.from_date,
            query.to_date,
        )
        .await
    {
        Ok(trends) => Ok(Json(json!({ "trends": trends }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn record_trend(
    State(s): State<AppState>,
    user: AuthUser,
    Json(req): Json<RecordTrend>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s.portfolio_analytics_repo.record_trend(org_id, req).await {
        Ok(trend) => Ok(Json(to_json_value(trend)?)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

// =============================================================================
// ALERT RULES
// =============================================================================

#[derive(Debug, Deserialize, IntoParams)]
struct AlertRulesQuery {
    #[serde(default)]
    active_only: bool,
}

async fn list_alert_rules(
    State(s): State<AppState>,
    user: AuthUser,
    Query(query): Query<AlertRulesQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .portfolio_analytics_repo
        .list_alert_rules(org_id, query.active_only)
        .await
    {
        Ok(rules) => Ok(Json(json!({ "rules": rules }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn create_alert_rule(
    State(s): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateAlertRule>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .portfolio_analytics_repo
        .create_alert_rule(org_id, req)
        .await
    {
        Ok(rule) => Ok(Json(to_json_value(rule)?)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

async fn get_alert_rule(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s.portfolio_analytics_repo.get_alert_rule(id, org_id).await {
        Ok(Some(rule)) => Ok(Json(to_json_value(rule)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Alert rule not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn update_alert_rule(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateAlertRule>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .portfolio_analytics_repo
        .update_alert_rule(id, org_id, req)
        .await
    {
        Ok(Some(rule)) => Ok(Json(to_json_value(rule)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Alert rule not found")),
        )),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

async fn delete_alert_rule(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .portfolio_analytics_repo
        .delete_alert_rule(id, org_id)
        .await
    {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Alert rule not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

// =============================================================================
// ALERTS
// =============================================================================

#[derive(Debug, Deserialize, IntoParams)]
struct AlertsQuery {
    #[serde(default)]
    unread_only: bool,
    #[serde(default)]
    unresolved_only: bool,
    #[serde(default = "default_alert_limit")]
    limit: i32,
}

fn default_alert_limit() -> i32 {
    50
}

async fn list_alerts(
    State(s): State<AppState>,
    user: AuthUser,
    Query(query): Query<AlertsQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .portfolio_analytics_repo
        .list_alerts(
            org_id,
            query.unread_only,
            query.unresolved_only,
            query.limit,
        )
        .await
    {
        Ok(alerts) => Ok(Json(json!({ "alerts": alerts }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn get_alert_stats(
    State(s): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s.portfolio_analytics_repo.get_alert_stats(org_id).await {
        Ok(stats) => Ok(Json(to_json_value(stats)?)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn get_alert(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s.portfolio_analytics_repo.get_alert(id, org_id).await {
        Ok(Some(alert)) => Ok(Json(to_json_value(alert)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Alert not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn acknowledge_alert(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<AcknowledgeAlert>,
) -> ApiResult<StatusCode> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .portfolio_analytics_repo
        .acknowledge_alert(id, org_id, req.is_read)
        .await
    {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Alert not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn resolve_alert(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ResolveAlert>,
) -> ApiResult<StatusCode> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    let user_id = user.user_id;

    match s
        .portfolio_analytics_repo
        .resolve_alert(id, org_id, user_id, req.resolution_notes)
        .await
    {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Alert not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}
