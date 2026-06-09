//! Reserve Fund Management routes for Epic 141.
//!
//! REST API endpoints for HOA/Condo reserve fund management.
//!
//! # RLS (PAP-67 / PAP-79)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` on the reserve-fund cluster,
//! so every query MUST run on a connection that has `app.current_org_id` set or
//! it collapses to deny-all. Each handler therefore acquires an [`RlsConnection`]
//! (which validates tenant membership and sets the org/user GUCs on a dedicated
//! connection) and passes `&mut **rls.conn()` to the repository. The
//! authoritative organization is `rls.tenant_id()` — the tenant the caller was
//! validated against — not a client-supplied `organization_id`, so the SQL org
//! filter and the RLS context can never disagree. Cross-tenant access is blocked
//! by RLS: a by-id read of another org's row returns no row (`404`), and a write
//! targeting another org fails the policy's `WITH CHECK`. `rls.release()` clears
//! the context before the connection returns to the pool (both on the Ok and Err
//! paths).

use crate::state::AppState;
use api_core::extractors::RlsConnection;
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
    mut rls: RlsConnection,
    Query(query): Query<FundListQuery>,
) -> ApiResult<Json<Vec<ReserveFund>>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .list_funds(
            &mut **rls.conn(),
            org_id,
            query.fund_type,
            query.building_id,
            query.active_only.unwrap_or(false),
        )
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to list funds: {}", e)));
    rls.release().await;
    out
}

/// Create a reserve fund.
async fn create_fund(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateReserveFund>,
) -> ApiResult<Json<ReserveFund>> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .reserve_fund_repo
        .create_fund(&mut **rls.conn(), org_id, req, user_id)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to create fund: {}", e)));
    rls.release().await;
    out
}

/// Get a reserve fund by ID.
async fn get_fund(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
) -> ApiResult<Json<ReserveFund>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .get_fund(&mut **rls.conn(), org_id, fund_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to get fund: {}", e)))
        .and_then(|f| f.map(Json).ok_or_else(|| not_found_error("Fund not found")));
    rls.release().await;
    out
}

/// Update a reserve fund.
async fn update_fund(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
    Json(req): Json<UpdateReserveFund>,
) -> ApiResult<Json<ReserveFund>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .update_fund(&mut **rls.conn(), org_id, fund_id, req)
        .await
        .map(Json)
        .map_err(|e| repo_error("update fund", "Fund not found", e));
    rls.release().await;
    out
}

/// Get fund dashboard.
async fn get_dashboard(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> ApiResult<Json<FundDashboard>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .get_fund_dashboard(&mut **rls.conn(), org_id)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to get dashboard: {}", e)));
    rls.release().await;
    out
}

/// Get fund health report.
async fn get_fund_health(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
) -> ApiResult<Json<FundHealthReport>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .get_fund_health_report(&mut **rls.conn(), org_id, fund_id)
        .await
        .map(Json)
        .map_err(|e| repo_error("get health report", "Fund not found", e));
    rls.release().await;
    out
}

// ============================================================================
// Contribution Schedule Handlers
// ============================================================================

/// List contribution schedules.
async fn list_schedules(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
    Query(query): Query<ActiveOnlyQuery>,
) -> ApiResult<Json<Vec<FundContributionSchedule>>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .list_contribution_schedules(
            &mut **rls.conn(),
            org_id,
            fund_id,
            query.active_only.unwrap_or(false),
        )
        .await
        .map(Json)
        .map_err(|e| repo_error("list schedules", "Fund not found", e));
    rls.release().await;
    out
}

/// Create a contribution schedule.
async fn create_schedule(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
    Json(req): Json<CreateContributionSchedule>,
) -> ApiResult<Json<FundContributionSchedule>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .create_contribution_schedule(&mut **rls.conn(), org_id, fund_id, req)
        .await
        .map(Json)
        .map_err(|e| repo_error("create schedule", "Fund not found", e));
    rls.release().await;
    out
}

/// Update a contribution schedule.
async fn update_schedule(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((_fund_id, schedule_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateContributionSchedule>,
) -> ApiResult<Json<FundContributionSchedule>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .update_contribution_schedule(&mut **rls.conn(), org_id, schedule_id, req)
        .await
        .map(Json)
        .map_err(|e| repo_error("update schedule", "Schedule not found", e));
    rls.release().await;
    out
}

// ============================================================================
// Transaction Handlers
// ============================================================================

/// List transactions.
async fn list_transactions(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
    Query(mut query): Query<TransactionQuery>,
) -> ApiResult<Json<Vec<FundTransaction>>> {
    let org_id = rls.tenant_id();
    query.fund_id = Some(fund_id);

    let out = state
        .reserve_fund_repo
        .list_transactions(&mut **rls.conn(), org_id, query)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to list transactions: {}", e)));
    rls.release().await;
    out
}

/// Record a transaction.
async fn record_transaction(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
    Json(req): Json<RecordFundTransaction>,
) -> ApiResult<Json<FundTransaction>> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .reserve_fund_repo
        .record_transaction(&mut **rls.conn(), org_id, fund_id, req, user_id)
        .await
        .map(Json)
        .map_err(|e| repo_error("record transaction", "Fund not found", e));
    rls.release().await;
    out
}

/// Transfer funds between accounts.
async fn transfer_funds(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<FundTransferRequest>,
) -> ApiResult<Json<(FundTransaction, FundTransaction)>> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .reserve_fund_repo
        .transfer_funds(&mut **rls.conn(), org_id, req, user_id)
        .await
        .map(Json)
        .map_err(|e| repo_error("transfer funds", "Fund not found", e));
    rls.release().await;
    out
}

// ============================================================================
// Investment Policy Handlers
// ============================================================================

/// List investment policies.
async fn list_policies(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
) -> ApiResult<Json<Vec<FundInvestmentPolicy>>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .list_investment_policies(&mut **rls.conn(), org_id, fund_id)
        .await
        .map(Json)
        .map_err(|e| repo_error("list policies", "Fund not found", e));
    rls.release().await;
    out
}

/// Create an investment policy.
async fn create_policy(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
    Json(req): Json<CreateInvestmentPolicy>,
) -> ApiResult<Json<FundInvestmentPolicy>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .create_investment_policy(&mut **rls.conn(), org_id, fund_id, req)
        .await
        .map(Json)
        .map_err(|e| repo_error("create policy", "Fund not found", e));
    rls.release().await;
    out
}

/// Get active investment policy.
async fn get_active_policy(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
) -> ApiResult<Json<Option<FundInvestmentPolicy>>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .get_active_investment_policy(&mut **rls.conn(), org_id, fund_id)
        .await
        .map(Json)
        .map_err(|e| repo_error("get active policy", "Fund not found", e));
    rls.release().await;
    out
}

// ============================================================================
// Projection Handlers
// ============================================================================

/// Create a projection.
async fn create_projection(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
    Json(req): Json<CreateFundProjection>,
) -> ApiResult<Json<FundProjection>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .create_projection(&mut **rls.conn(), org_id, fund_id, req)
        .await
        .map(Json)
        .map_err(|e| repo_error("create projection", "Fund not found", e));
    rls.release().await;
    out
}

/// Get current projection.
async fn get_current_projection(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
) -> ApiResult<Json<Option<FundProjection>>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .get_current_projection(&mut **rls.conn(), org_id, fund_id)
        .await
        .map(Json)
        .map_err(|e| repo_error("get projection", "Fund not found", e));
    rls.release().await;
    out
}

/// Get projection items.
async fn get_projection_items(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((_fund_id, projection_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Vec<FundProjectionItem>>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .get_projection_items(&mut **rls.conn(), org_id, projection_id)
        .await
        .map(Json)
        .map_err(|e| repo_error("get projection items", "Projection not found", e));
    rls.release().await;
    out
}

/// Add projection items.
async fn add_projection_items(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((_fund_id, projection_id)): Path<(Uuid, Uuid)>,
    Json(items): Json<Vec<CreateProjectionItem>>,
) -> ApiResult<Json<Vec<FundProjectionItem>>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .add_projection_items(&mut **rls.conn(), org_id, projection_id, items)
        .await
        .map(Json)
        .map_err(|e| repo_error("add projection items", "Projection not found", e));
    rls.release().await;
    out
}

// ============================================================================
// Component Handlers
// ============================================================================

/// List components.
async fn list_components(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
) -> ApiResult<Json<Vec<FundComponent>>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .list_components(&mut **rls.conn(), org_id, fund_id)
        .await
        .map(Json)
        .map_err(|e| repo_error("list components", "Fund not found", e));
    rls.release().await;
    out
}

/// Create a component.
async fn create_component(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
    Json(req): Json<CreateFundComponent>,
) -> ApiResult<Json<FundComponent>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .create_component(&mut **rls.conn(), org_id, fund_id, req)
        .await
        .map(Json)
        .map_err(|e| repo_error("create component", "Fund not found", e));
    rls.release().await;
    out
}

/// Update a component.
async fn update_component(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((_fund_id, component_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateFundComponent>,
) -> ApiResult<Json<FundComponent>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .update_component(&mut **rls.conn(), org_id, component_id, req)
        .await
        .map(Json)
        .map_err(|e| repo_error("update component", "Component not found", e));
    rls.release().await;
    out
}

// ============================================================================
// Alert Handlers
// ============================================================================

/// List active alerts.
async fn list_alerts(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> ApiResult<Json<Vec<FundAlert>>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .list_active_alerts(&mut **rls.conn(), org_id)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to list alerts: {}", e)));
    rls.release().await;
    out
}

/// Acknowledge an alert.
async fn acknowledge_alert(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(alert_id): Path<Uuid>,
) -> ApiResult<Json<FundAlert>> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .reserve_fund_repo
        .acknowledge_alert(&mut **rls.conn(), org_id, alert_id, user_id)
        .await
        .map(Json)
        .map_err(|e| repo_error("acknowledge alert", "Alert not found", e));
    rls.release().await;
    out
}

/// Resolve an alert.
async fn resolve_alert(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(alert_id): Path<Uuid>,
) -> ApiResult<Json<FundAlert>> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .reserve_fund_repo
        .resolve_alert(&mut **rls.conn(), org_id, alert_id, user_id)
        .await
        .map(Json)
        .map_err(|e| repo_error("resolve alert", "Alert not found", e));
    rls.release().await;
    out
}
