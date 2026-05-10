//! Epic 144: Portfolio Performance Analytics routes.
//! API endpoints for portfolio configuration, income/expense tracking,
//! ROI calculations, benchmarking, and analytics dashboard.

use crate::state::AppState;
use api_core::extractors::AuthUser;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::portfolio_performance::{
    BenchmarkComparison, CalculateMetricsRequest, CashFlowTrendPoint, CreateBenchmarkComparison,
    CreateMarketBenchmark, CreatePerformanceAlert, CreatePerformancePortfolio,
    CreatePortfolioProperty, CreatePropertyTransaction, DashboardSummary, FinancialMetrics,
    MarketBenchmark, PerformanceAlert, PerformancePortfolio, PortfolioMetricsSummary,
    PortfolioProperty, PropertyCashFlow, PropertyPerformanceCard, PropertyTransaction,
    TransactionQuery, UpdateMarketBenchmark, UpdatePerformancePortfolio, UpdatePortfolioProperty,
    UpdatePropertyTransaction, UpsertPropertyCashFlow,
};
use serde::Deserialize;
use uuid::Uuid;

type ApiResult<T> = Result<T, (StatusCode, Json<ErrorResponse>)>;

fn internal_error(e: impl std::fmt::Display) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new("INTERNAL_ERROR", e.to_string())),
    )
}

fn not_found(resource: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new(
            "NOT_FOUND",
            format!("{} not found", resource),
        )),
    )
}

fn get_org_id(auth: &AuthUser) -> ApiResult<Uuid> {
    auth.tenant_id.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "UNAUTHORIZED",
                "Organization context required",
            )),
        )
    })
}

// =============================================================================
// ROUTER
// =============================================================================

pub fn router() -> Router<AppState> {
    Router::new()
        // Portfolio CRUD
        .route("/portfolios", post(create_portfolio))
        .route("/portfolios", get(list_portfolios))
        .route("/portfolios/{id}", get(get_portfolio))
        .route("/portfolios/{id}", put(update_portfolio))
        .route("/portfolios/{id}", delete(delete_portfolio))
        // Portfolio properties
        .route("/portfolios/{id}/properties", post(add_property))
        .route("/portfolios/{id}/properties", get(list_properties))
        .route(
            "/portfolios/{id}/properties/{property_id}",
            get(get_property),
        )
        .route(
            "/portfolios/{id}/properties/{property_id}",
            put(update_property),
        )
        .route(
            "/portfolios/{id}/properties/{property_id}",
            delete(remove_property),
        )
        // Transactions
        .route("/portfolios/{id}/transactions", post(create_transaction))
        .route("/portfolios/{id}/transactions", get(list_transactions))
        .route(
            "/portfolios/{id}/transactions/{transaction_id}",
            get(get_transaction),
        )
        .route(
            "/portfolios/{id}/transactions/{transaction_id}",
            put(update_transaction),
        )
        .route(
            "/portfolios/{id}/transactions/{transaction_id}",
            delete(delete_transaction),
        )
        // Cash flows
        .route("/portfolios/{id}/cash-flows", post(upsert_cash_flow))
        .route("/portfolios/{id}/cash-flows", get(get_cash_flows))
        // Metrics
        .route(
            "/portfolios/{id}/metrics/calculate",
            post(calculate_metrics),
        )
        .route("/portfolios/{id}/metrics/latest", get(get_latest_metrics))
        .route("/portfolios/{id}/metrics/summary", get(get_metrics_summary))
        // Benchmarks
        .route("/benchmarks", post(create_benchmark))
        .route("/benchmarks", get(list_benchmarks))
        .route("/benchmarks/{id}", get(get_benchmark))
        .route("/benchmarks/{id}", put(update_benchmark))
        .route("/benchmarks/{id}", delete(delete_benchmark))
        // Comparisons
        .route("/portfolios/{id}/comparisons", post(create_comparison))
        .route("/portfolios/{id}/comparisons", get(list_comparisons))
        .route(
            "/portfolios/{id}/comparisons/{comparison_id}",
            get(get_comparison),
        )
        // Dashboard
        .route(
            "/portfolios/{id}/dashboard/summary",
            get(get_dashboard_summary),
        )
        .route(
            "/portfolios/{id}/dashboard/property-cards",
            get(get_property_cards),
        )
        .route(
            "/portfolios/{id}/dashboard/cash-flow-trend",
            get(get_cash_flow_trend),
        )
        // Alerts
        .route("/portfolios/{id}/alerts", post(create_alert))
        .route("/portfolios/{id}/alerts", get(list_alerts))
        .route(
            "/portfolios/{id}/alerts/{alert_id}/read",
            post(mark_alert_read),
        )
        .route(
            "/portfolios/{id}/alerts/{alert_id}/resolve",
            post(resolve_alert),
        )
}

// =============================================================================
// QUERY PARAMS
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct ListPortfoliosQuery {
    pub active_only: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CashFlowQuery {
    pub property_id: Option<Uuid>,
    pub start_year: Option<i32>,
    pub start_month: Option<i32>,
    pub end_year: Option<i32>,
    pub end_month: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct MetricsQuery {
    pub property_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct MetricsSummaryQuery {
    pub period_start: String,
    pub period_end: String,
}

#[derive(Debug, Deserialize)]
pub struct BenchmarkListQuery {
    pub active_only: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CashFlowTrendQuery {
    pub months: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct AlertsQuery {
    pub unread_only: Option<bool>,
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct DashboardQuery {
    pub as_of_date: Option<String>,
}

// =============================================================================
// STORY 144.1: PORTFOLIO CONFIGURATION
// =============================================================================

async fn create_portfolio(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreatePerformancePortfolio>,
) -> ApiResult<Json<PerformancePortfolio>> {
    let org_id = get_org_id(&auth)?;
    let portfolio = state
        .portfolio_performance_repo
        .create_portfolio(org_id, req, auth.user_id)
        .await
        .map_err(internal_error)?;

    Ok(Json(portfolio))
}

async fn list_portfolios(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListPortfoliosQuery>,
) -> ApiResult<Json<Vec<PerformancePortfolio>>> {
    let org_id = get_org_id(&auth)?;
    let portfolios = state
        .portfolio_performance_repo
        .list_portfolios(org_id, query.active_only.unwrap_or(false))
        .await
        .map_err(internal_error)?;

    Ok(Json(portfolios))
}

async fn get_portfolio(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<PerformancePortfolio>> {
    let org_id = get_org_id(&auth)?;
    let portfolio = state
        .portfolio_performance_repo
        .get_portfolio(id, org_id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found("Portfolio"))?;

    Ok(Json(portfolio))
}

async fn update_portfolio(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdatePerformancePortfolio>,
) -> ApiResult<Json<PerformancePortfolio>> {
    let org_id = get_org_id(&auth)?;
    let portfolio = state
        .portfolio_performance_repo
        .update_portfolio(id, org_id, req)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found("Portfolio"))?;

    Ok(Json(portfolio))
}

async fn delete_portfolio(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let org_id = get_org_id(&auth)?;
    let deleted = state
        .portfolio_performance_repo
        .delete_portfolio(id, org_id)
        .await
        .map_err(internal_error)?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found("Portfolio"))
    }
}

async fn add_property(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(portfolio_id): Path<Uuid>,
    Json(req): Json<CreatePortfolioProperty>,
) -> ApiResult<Json<PortfolioProperty>> {
    let property = state
        .portfolio_performance_repo
        .add_property(portfolio_id, req)
        .await
        .map_err(internal_error)?;

    Ok(Json(property))
}

async fn list_properties(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(portfolio_id): Path<Uuid>,
) -> ApiResult<Json<Vec<PortfolioProperty>>> {
    let properties = state
        .portfolio_performance_repo
        .list_properties(portfolio_id)
        .await
        .map_err(internal_error)?;

    Ok(Json(properties))
}

async fn get_property(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((portfolio_id, property_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<PortfolioProperty>> {
    let property = state
        .portfolio_performance_repo
        .get_property(property_id, portfolio_id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found("Property"))?;

    Ok(Json(property))
}

async fn update_property(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((portfolio_id, property_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdatePortfolioProperty>,
) -> ApiResult<Json<PortfolioProperty>> {
    let property = state
        .portfolio_performance_repo
        .update_property(property_id, portfolio_id, req)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found("Property"))?;

    Ok(Json(property))
}

async fn remove_property(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((portfolio_id, property_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    let removed = state
        .portfolio_performance_repo
        .remove_property(property_id, portfolio_id)
        .await
        .map_err(internal_error)?;

    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found("Property"))
    }
}

// =============================================================================
// STORY 144.2: INCOME & EXPENSE TRACKING
// =============================================================================

async fn create_transaction(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(portfolio_id): Path<Uuid>,
    Json(req): Json<CreatePropertyTransaction>,
) -> ApiResult<Json<PropertyTransaction>> {
    let transaction = state
        .portfolio_performance_repo
        .create_transaction(portfolio_id, req, auth.user_id)
        .await
        .map_err(internal_error)?;

    Ok(Json(transaction))
}

async fn list_transactions(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(portfolio_id): Path<Uuid>,
    Query(query): Query<TransactionQuery>,
) -> ApiResult<Json<Vec<PropertyTransaction>>> {
    let transactions = state
        .portfolio_performance_repo
        .list_transactions(portfolio_id, query)
        .await
        .map_err(internal_error)?;

    Ok(Json(transactions))
}

async fn get_transaction(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((portfolio_id, transaction_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<PropertyTransaction>> {
    let transaction = state
        .portfolio_performance_repo
        .get_transaction(transaction_id, portfolio_id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found("Transaction"))?;

    Ok(Json(transaction))
}

async fn update_transaction(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((portfolio_id, transaction_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdatePropertyTransaction>,
) -> ApiResult<Json<PropertyTransaction>> {
    let transaction = state
        .portfolio_performance_repo
        .update_transaction(transaction_id, portfolio_id, req)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found("Transaction"))?;

    Ok(Json(transaction))
}

async fn delete_transaction(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((portfolio_id, transaction_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    let deleted = state
        .portfolio_performance_repo
        .delete_transaction(transaction_id, portfolio_id)
        .await
        .map_err(internal_error)?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found("Transaction"))
    }
}

async fn upsert_cash_flow(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(portfolio_id): Path<Uuid>,
    Json(req): Json<UpsertPropertyCashFlow>,
) -> ApiResult<Json<PropertyCashFlow>> {
    let cash_flow = state
        .portfolio_performance_repo
        .upsert_cash_flow(portfolio_id, req)
        .await
        .map_err(internal_error)?;

    Ok(Json(cash_flow))
}

async fn get_cash_flows(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(portfolio_id): Path<Uuid>,
    Query(query): Query<CashFlowQuery>,
) -> ApiResult<Json<Vec<PropertyCashFlow>>> {
    let cash_flows = state
        .portfolio_performance_repo
        .get_cash_flows(
            portfolio_id,
            query.property_id,
            query.start_year,
            query.start_month,
            query.end_year,
            query.end_month,
        )
        .await
        .map_err(internal_error)?;

    Ok(Json(cash_flows))
}

// =============================================================================
// STORY 144.3: ROI & FINANCIAL METRICS CALCULATOR
// =============================================================================

async fn calculate_metrics(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(portfolio_id): Path<Uuid>,
    Json(req): Json<CalculateMetricsRequest>,
) -> ApiResult<Json<FinancialMetrics>> {
    let metrics = state
        .portfolio_performance_repo
        .calculate_metrics(portfolio_id, req)
        .await
        .map_err(internal_error)?;

    Ok(Json(metrics))
}

async fn get_latest_metrics(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(portfolio_id): Path<Uuid>,
    Query(query): Query<MetricsQuery>,
) -> ApiResult<Json<Option<FinancialMetrics>>> {
    let metrics = state
        .portfolio_performance_repo
        .get_latest_metrics(portfolio_id, query.property_id)
        .await
        .map_err(internal_error)?;

    Ok(Json(metrics))
}

async fn get_metrics_summary(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(portfolio_id): Path<Uuid>,
    Query(query): Query<MetricsSummaryQuery>,
) -> ApiResult<Json<PortfolioMetricsSummary>> {
    let period_start =
        chrono::NaiveDate::parse_from_str(&query.period_start, "%Y-%m-%d").map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "BAD_REQUEST",
                    "Invalid period_start date format",
                )),
            )
        })?;
    let period_end =
        chrono::NaiveDate::parse_from_str(&query.period_end, "%Y-%m-%d").map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "BAD_REQUEST",
                    "Invalid period_end date format",
                )),
            )
        })?;

    let summary = state
        .portfolio_performance_repo
        .get_portfolio_metrics_summary(portfolio_id, period_start, period_end)
        .await
        .map_err(internal_error)?;

    Ok(Json(summary))
}

// =============================================================================
// STORY 144.4: PERFORMANCE BENCHMARKING
// =============================================================================

async fn create_benchmark(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateMarketBenchmark>,
) -> ApiResult<Json<MarketBenchmark>> {
    let org_id = get_org_id(&auth)?;
    let benchmark = state
        .portfolio_performance_repo
        .create_benchmark(org_id, req)
        .await
        .map_err(internal_error)?;

    Ok(Json(benchmark))
}

async fn list_benchmarks(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<BenchmarkListQuery>,
) -> ApiResult<Json<Vec<MarketBenchmark>>> {
    let org_id = get_org_id(&auth)?;
    let benchmarks = state
        .portfolio_performance_repo
        .list_benchmarks(org_id, query.active_only.unwrap_or(false))
        .await
        .map_err(internal_error)?;

    Ok(Json(benchmarks))
}

async fn get_benchmark(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<MarketBenchmark>> {
    let org_id = get_org_id(&auth)?;
    let benchmark = state
        .portfolio_performance_repo
        .get_benchmark(id, org_id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found("Benchmark"))?;

    Ok(Json(benchmark))
}

async fn update_benchmark(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateMarketBenchmark>,
) -> ApiResult<Json<MarketBenchmark>> {
    let org_id = get_org_id(&auth)?;
    let benchmark = state
        .portfolio_performance_repo
        .update_benchmark(id, org_id, req)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found("Benchmark"))?;

    Ok(Json(benchmark))
}

async fn delete_benchmark(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let org_id = get_org_id(&auth)?;
    let deleted = state
        .portfolio_performance_repo
        .delete_benchmark(id, org_id)
        .await
        .map_err(internal_error)?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found("Benchmark"))
    }
}

async fn create_comparison(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(portfolio_id): Path<Uuid>,
    Json(req): Json<CreateBenchmarkComparison>,
) -> ApiResult<Json<BenchmarkComparison>> {
    let comparison = state
        .portfolio_performance_repo
        .create_comparison(portfolio_id, req)
        .await
        .map_err(internal_error)?;

    Ok(Json(comparison))
}

async fn list_comparisons(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(portfolio_id): Path<Uuid>,
) -> ApiResult<Json<Vec<BenchmarkComparison>>> {
    let comparisons = state
        .portfolio_performance_repo
        .list_comparisons(portfolio_id)
        .await
        .map_err(internal_error)?;

    Ok(Json(comparisons))
}

async fn get_comparison(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((portfolio_id, comparison_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<BenchmarkComparison>> {
    let comparison = state
        .portfolio_performance_repo
        .get_comparison(comparison_id, portfolio_id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found("Comparison"))?;

    Ok(Json(comparison))
}

// =============================================================================
// STORY 144.5: PORTFOLIO ANALYTICS DASHBOARD
// =============================================================================

async fn get_dashboard_summary(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(portfolio_id): Path<Uuid>,
    Query(query): Query<DashboardQuery>,
) -> ApiResult<Json<DashboardSummary>> {
    let as_of_date = query
        .as_of_date
        .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| chrono::Utc::now().date_naive());

    let summary = state
        .portfolio_performance_repo
        .get_dashboard_summary(portfolio_id, as_of_date)
        .await
        .map_err(internal_error)?;

    Ok(Json(summary))
}

async fn get_property_cards(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(portfolio_id): Path<Uuid>,
) -> ApiResult<Json<Vec<PropertyPerformanceCard>>> {
    let cards = state
        .portfolio_performance_repo
        .get_property_performance_cards(portfolio_id)
        .await
        .map_err(internal_error)?;

    Ok(Json(cards))
}

async fn get_cash_flow_trend(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(portfolio_id): Path<Uuid>,
    Query(query): Query<CashFlowTrendQuery>,
) -> ApiResult<Json<Vec<CashFlowTrendPoint>>> {
    let months = query.months.unwrap_or(12);

    let trend = state
        .portfolio_performance_repo
        .get_cash_flow_trend(portfolio_id, months)
        .await
        .map_err(internal_error)?;

    Ok(Json(trend))
}

async fn create_alert(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(portfolio_id): Path<Uuid>,
    Json(req): Json<CreatePerformanceAlert>,
) -> ApiResult<Json<PerformanceAlert>> {
    let alert = state
        .portfolio_performance_repo
        .create_alert(portfolio_id, req)
        .await
        .map_err(internal_error)?;

    Ok(Json(alert))
}

async fn list_alerts(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(portfolio_id): Path<Uuid>,
    Query(query): Query<AlertsQuery>,
) -> ApiResult<Json<Vec<PerformanceAlert>>> {
    let alerts = state
        .portfolio_performance_repo
        .list_alerts(
            portfolio_id,
            query.unread_only.unwrap_or(false),
            query.limit.unwrap_or(50),
        )
        .await
        .map_err(internal_error)?;

    Ok(Json(alerts))
}

async fn mark_alert_read(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((portfolio_id, alert_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    state
        .portfolio_performance_repo
        .mark_alert_read(alert_id, portfolio_id)
        .await
        .map_err(internal_error)?;

    Ok(StatusCode::NO_CONTENT)
}

async fn resolve_alert(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((portfolio_id, alert_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    state
        .portfolio_performance_repo
        .resolve_alert(alert_id, portfolio_id)
        .await
        .map_err(internal_error)?;

    Ok(StatusCode::NO_CONTENT)
}
