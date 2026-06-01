//! Reserve Fund Management routes for Epic 141.
//!
//! REST API endpoints for HOA/Condo reserve fund management.

use crate::state::AppState;
use api_core::extractors::AuthUser;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::reserve_funds::{
    CreateContributionSchedule, CreateFundComponent, CreateFundProjection, CreateInvestmentPolicy,
    CreateProjectionItem, CreateReserveFund, FundAlert, FundComponent, FundContributionSchedule,
    FundDashboard, FundHealthReport, FundInvestmentPolicy, FundProjection, FundProjectionItem,
    FundTransaction, FundTransferRequest, FundType, RecordFundTransaction, ReserveFund,
    TransactionQuery, UpdateContributionSchedule, UpdateFundComponent, UpdateReserveFund,
};
use serde::Deserialize;
use uuid::Uuid;

type ApiResult<T> = Result<T, (StatusCode, Json<ErrorResponse>)>;

/// Create the reserve funds router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Fund CRUD
        .route("/", get(list_funds).post(create_fund))
        .route("/dashboard", get(get_dashboard))
        .route("/{fund_id}", get(get_fund).put(update_fund))
        .route("/{fund_id}/health", get(get_fund_health))
        // Contribution schedules
        .route(
            "/{fund_id}/schedules",
            get(list_schedules).post(create_schedule),
        )
        .route("/{fund_id}/schedules/{schedule_id}", put(update_schedule))
        // Transactions
        .route(
            "/{fund_id}/transactions",
            get(list_transactions).post(record_transaction),
        )
        .route("/transfers", post(transfer_funds))
        // Investment policies
        .route(
            "/{fund_id}/policies",
            get(list_policies).post(create_policy),
        )
        .route("/{fund_id}/policies/active", get(get_active_policy))
        // Projections
        .route("/{fund_id}/projections", post(create_projection))
        .route(
            "/{fund_id}/projections/current",
            get(get_current_projection),
        )
        .route(
            "/{fund_id}/projections/{projection_id}/items",
            get(get_projection_items).post(add_projection_items),
        )
        // Components
        .route(
            "/{fund_id}/components",
            get(list_components).post(create_component),
        )
        .route(
            "/{fund_id}/components/{component_id}",
            put(update_component),
        )
        // Alerts
        .route("/alerts", get(list_alerts))
        .route("/alerts/{alert_id}/acknowledge", post(acknowledge_alert))
        .route("/alerts/{alert_id}/resolve", post(resolve_alert))
}

// ============================================================================
// Query Parameters
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct FundListQuery {
    pub fund_type: Option<FundType>,
    pub building_id: Option<Uuid>,
    pub active_only: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ActiveOnlyQuery {
    pub active_only: Option<bool>,
}

// ============================================================================
// Helper Functions
// ============================================================================

fn internal_error(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::internal_error(msg)),
    )
}

fn forbidden_error(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (StatusCode::FORBIDDEN, Json(ErrorResponse::forbidden(msg)))
}

fn not_found_error(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (StatusCode::NOT_FOUND, Json(ErrorResponse::not_found(msg)))
}

/// Map a repository error to an HTTP response.
///
/// Org-scoped repo methods surface a foreign-tenant or missing row as
/// [`sqlx::Error::RowNotFound`]; we render that as a 404 (never 500, and
/// never leaking that the row exists in another org) — this is the
/// caller-facing half of the #810 IDOR fix. Any other error is a genuine
/// 500.
fn repo_error(
    action: &str,
    not_found_msg: &str,
    e: sqlx::Error,
) -> (StatusCode, Json<ErrorResponse>) {
    match e {
        sqlx::Error::RowNotFound => not_found_error(not_found_msg),
        other => internal_error(&format!("Failed to {}: {}", action, other)),
    }
}

// ============================================================================
// Fund CRUD Handlers
// ============================================================================

/// List reserve funds.
async fn list_funds(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<FundListQuery>,
) -> ApiResult<Json<Vec<ReserveFund>>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let funds = state
        .reserve_fund_repo
        .list_funds(
            org_id,
            query.fund_type,
            query.building_id,
            query.active_only.unwrap_or(false),
        )
        .await
        .map_err(|e| internal_error(&format!("Failed to list funds: {}", e)))?;

    Ok(Json(funds))
}

/// Create a reserve fund.
async fn create_fund(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateReserveFund>,
) -> ApiResult<Json<ReserveFund>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let fund = state
        .reserve_fund_repo
        .create_fund(org_id, req, user.user_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to create fund: {}", e)))?;

    Ok(Json(fund))
}

/// Get a reserve fund by ID.
async fn get_fund(
    State(state): State<AppState>,
    user: AuthUser,
    Path(fund_id): Path<Uuid>,
) -> ApiResult<Json<ReserveFund>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let fund = state
        .reserve_fund_repo
        .get_fund(org_id, fund_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to get fund: {}", e)))?
        .ok_or_else(|| not_found_error("Fund not found"))?;

    Ok(Json(fund))
}

/// Update a reserve fund.
async fn update_fund(
    State(state): State<AppState>,
    user: AuthUser,
    Path(fund_id): Path<Uuid>,
    Json(req): Json<UpdateReserveFund>,
) -> ApiResult<Json<ReserveFund>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let fund = state
        .reserve_fund_repo
        .update_fund(org_id, fund_id, req)
        .await
        .map_err(|e| repo_error("update fund", "Fund not found", e))?;

    Ok(Json(fund))
}

/// Get fund dashboard.
async fn get_dashboard(
    State(state): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<FundDashboard>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let dashboard = state
        .reserve_fund_repo
        .get_fund_dashboard(org_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to get dashboard: {}", e)))?;

    Ok(Json(dashboard))
}

/// Get fund health report.
async fn get_fund_health(
    State(state): State<AppState>,
    user: AuthUser,
    Path(fund_id): Path<Uuid>,
) -> ApiResult<Json<FundHealthReport>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let report = state
        .reserve_fund_repo
        .get_fund_health_report(org_id, fund_id)
        .await
        .map_err(|e| repo_error("get health report", "Fund not found", e))?;

    Ok(Json(report))
}

// ============================================================================
// Contribution Schedule Handlers
// ============================================================================

/// List contribution schedules.
async fn list_schedules(
    State(state): State<AppState>,
    user: AuthUser,
    Path(fund_id): Path<Uuid>,
    Query(query): Query<ActiveOnlyQuery>,
) -> ApiResult<Json<Vec<FundContributionSchedule>>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let schedules = state
        .reserve_fund_repo
        .list_contribution_schedules(org_id, fund_id, query.active_only.unwrap_or(false))
        .await
        .map_err(|e| repo_error("list schedules", "Fund not found", e))?;

    Ok(Json(schedules))
}

/// Create a contribution schedule.
async fn create_schedule(
    State(state): State<AppState>,
    user: AuthUser,
    Path(fund_id): Path<Uuid>,
    Json(req): Json<CreateContributionSchedule>,
) -> ApiResult<Json<FundContributionSchedule>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let schedule = state
        .reserve_fund_repo
        .create_contribution_schedule(org_id, fund_id, req)
        .await
        .map_err(|e| repo_error("create schedule", "Fund not found", e))?;

    Ok(Json(schedule))
}

/// Update a contribution schedule.
async fn update_schedule(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_fund_id, schedule_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateContributionSchedule>,
) -> ApiResult<Json<FundContributionSchedule>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let schedule = state
        .reserve_fund_repo
        .update_contribution_schedule(org_id, schedule_id, req)
        .await
        .map_err(|e| repo_error("update schedule", "Schedule not found", e))?;

    Ok(Json(schedule))
}

// ============================================================================
// Transaction Handlers
// ============================================================================

/// List transactions.
async fn list_transactions(
    State(state): State<AppState>,
    user: AuthUser,
    Path(fund_id): Path<Uuid>,
    Query(mut query): Query<TransactionQuery>,
) -> ApiResult<Json<Vec<FundTransaction>>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    query.fund_id = Some(fund_id);

    let transactions = state
        .reserve_fund_repo
        .list_transactions(org_id, query)
        .await
        .map_err(|e| internal_error(&format!("Failed to list transactions: {}", e)))?;

    Ok(Json(transactions))
}

/// Record a transaction.
async fn record_transaction(
    State(state): State<AppState>,
    user: AuthUser,
    Path(fund_id): Path<Uuid>,
    Json(req): Json<RecordFundTransaction>,
) -> ApiResult<Json<FundTransaction>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let transaction = state
        .reserve_fund_repo
        .record_transaction(org_id, fund_id, req, user.user_id)
        .await
        .map_err(|e| repo_error("record transaction", "Fund not found", e))?;

    Ok(Json(transaction))
}

/// Transfer funds between accounts.
async fn transfer_funds(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<FundTransferRequest>,
) -> ApiResult<Json<(FundTransaction, FundTransaction)>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let transactions = state
        .reserve_fund_repo
        .transfer_funds(org_id, req, user.user_id)
        .await
        .map_err(|e| repo_error("transfer funds", "Fund not found", e))?;

    Ok(Json(transactions))
}

// ============================================================================
// Investment Policy Handlers
// ============================================================================

/// List investment policies.
async fn list_policies(
    State(state): State<AppState>,
    user: AuthUser,
    Path(fund_id): Path<Uuid>,
) -> ApiResult<Json<Vec<FundInvestmentPolicy>>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let policies = state
        .reserve_fund_repo
        .list_investment_policies(org_id, fund_id)
        .await
        .map_err(|e| repo_error("list policies", "Fund not found", e))?;

    Ok(Json(policies))
}

/// Create an investment policy.
async fn create_policy(
    State(state): State<AppState>,
    user: AuthUser,
    Path(fund_id): Path<Uuid>,
    Json(req): Json<CreateInvestmentPolicy>,
) -> ApiResult<Json<FundInvestmentPolicy>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let policy = state
        .reserve_fund_repo
        .create_investment_policy(org_id, fund_id, req)
        .await
        .map_err(|e| repo_error("create policy", "Fund not found", e))?;

    Ok(Json(policy))
}

/// Get active investment policy.
async fn get_active_policy(
    State(state): State<AppState>,
    user: AuthUser,
    Path(fund_id): Path<Uuid>,
) -> ApiResult<Json<Option<FundInvestmentPolicy>>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let policy = state
        .reserve_fund_repo
        .get_active_investment_policy(org_id, fund_id)
        .await
        .map_err(|e| repo_error("get active policy", "Fund not found", e))?;

    Ok(Json(policy))
}

// ============================================================================
// Projection Handlers
// ============================================================================

/// Create a projection.
async fn create_projection(
    State(state): State<AppState>,
    user: AuthUser,
    Path(fund_id): Path<Uuid>,
    Json(req): Json<CreateFundProjection>,
) -> ApiResult<Json<FundProjection>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let projection = state
        .reserve_fund_repo
        .create_projection(org_id, fund_id, req)
        .await
        .map_err(|e| repo_error("create projection", "Fund not found", e))?;

    Ok(Json(projection))
}

/// Get current projection.
async fn get_current_projection(
    State(state): State<AppState>,
    user: AuthUser,
    Path(fund_id): Path<Uuid>,
) -> ApiResult<Json<Option<FundProjection>>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let projection = state
        .reserve_fund_repo
        .get_current_projection(org_id, fund_id)
        .await
        .map_err(|e| repo_error("get projection", "Fund not found", e))?;

    Ok(Json(projection))
}

/// Get projection items.
async fn get_projection_items(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_fund_id, projection_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Vec<FundProjectionItem>>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let items = state
        .reserve_fund_repo
        .get_projection_items(org_id, projection_id)
        .await
        .map_err(|e| repo_error("get projection items", "Projection not found", e))?;

    Ok(Json(items))
}

/// Add projection items.
async fn add_projection_items(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_fund_id, projection_id)): Path<(Uuid, Uuid)>,
    Json(items): Json<Vec<CreateProjectionItem>>,
) -> ApiResult<Json<Vec<FundProjectionItem>>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let created = state
        .reserve_fund_repo
        .add_projection_items(org_id, projection_id, items)
        .await
        .map_err(|e| repo_error("add projection items", "Projection not found", e))?;

    Ok(Json(created))
}

// ============================================================================
// Component Handlers
// ============================================================================

/// List components.
async fn list_components(
    State(state): State<AppState>,
    user: AuthUser,
    Path(fund_id): Path<Uuid>,
) -> ApiResult<Json<Vec<FundComponent>>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let components = state
        .reserve_fund_repo
        .list_components(org_id, fund_id)
        .await
        .map_err(|e| repo_error("list components", "Fund not found", e))?;

    Ok(Json(components))
}

/// Create a component.
async fn create_component(
    State(state): State<AppState>,
    user: AuthUser,
    Path(fund_id): Path<Uuid>,
    Json(req): Json<CreateFundComponent>,
) -> ApiResult<Json<FundComponent>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let component = state
        .reserve_fund_repo
        .create_component(org_id, fund_id, req)
        .await
        .map_err(|e| repo_error("create component", "Fund not found", e))?;

    Ok(Json(component))
}

/// Update a component.
async fn update_component(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_fund_id, component_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateFundComponent>,
) -> ApiResult<Json<FundComponent>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let component = state
        .reserve_fund_repo
        .update_component(org_id, component_id, req)
        .await
        .map_err(|e| repo_error("update component", "Component not found", e))?;

    Ok(Json(component))
}

// ============================================================================
// Alert Handlers
// ============================================================================

/// List active alerts.
async fn list_alerts(
    State(state): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<Vec<FundAlert>>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let alerts = state
        .reserve_fund_repo
        .list_active_alerts(org_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to list alerts: {}", e)))?;

    Ok(Json(alerts))
}

/// Acknowledge an alert.
async fn acknowledge_alert(
    State(state): State<AppState>,
    user: AuthUser,
    Path(alert_id): Path<Uuid>,
) -> ApiResult<Json<FundAlert>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let alert = state
        .reserve_fund_repo
        .acknowledge_alert(org_id, alert_id, user.user_id)
        .await
        .map_err(|e| repo_error("acknowledge alert", "Alert not found", e))?;

    Ok(Json(alert))
}

/// Resolve an alert.
async fn resolve_alert(
    State(state): State<AppState>,
    user: AuthUser,
    Path(alert_id): Path<Uuid>,
) -> ApiResult<Json<FundAlert>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let alert = state
        .reserve_fund_repo
        .resolve_alert(org_id, alert_id, user.user_id)
        .await
        .map_err(|e| repo_error("resolve alert", "Alert not found", e))?;

    Ok(Json(alert))
}
