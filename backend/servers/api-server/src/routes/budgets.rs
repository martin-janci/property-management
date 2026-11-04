//! Budget and financial planning routes for Epic 24.
//!
//! Handles budgets, budget items, capital plans, reserve funds, and forecasts.
//!
//! # Authorization
//!
//! Every handler takes an [`RlsConnection`], which validates the caller's
//! membership in the active tenant and resolves their role from the database.
//! On top of that:
//!
//! * any `organization_id` supplied in the query/body must equal the active
//!   tenant (403 otherwise) — defense in depth, because RLS is bypassed by
//!   superuser / `BYPASSRLS` database roles;
//! * id-only routes (`/{id}/items`, `/items/{item_id}`, `/alerts/{id}`, …)
//!   resolve their parent budget scoped to the active tenant (404 otherwise);
//! * every mutation requires a manager-level role ([`TenantRole::is_manager`]);
//!   read-only GETs stay available to all org members;
//! * budget lines may only change while the budget is `draft`, actuals may only
//!   be recorded while it is `active` (400 `INVALID_STATE`, matching the
//!   submit/approve/activate/close transitions);
//! * a budget cannot be approved by the user who created it (403).
//!
//! [`TenantRole::is_manager`]: common::TenantRole::is_manager

use api_core::extractors::{AuthUser, RlsConnection};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use common::ErrorResponse;
use db::models::{
    budget_status, AcknowledgeVarianceAlert, Budget, BudgetQuery, CapitalPlanQuery, CreateBudget,
    CreateBudgetCategory, CreateBudgetItem, CreateCapitalPlan, CreateFinancialForecast,
    ForecastQuery, RecordBudgetActual, UpdateBudget, UpdateBudgetCategory, UpdateBudgetItem,
    UpdateCapitalPlan, UpdateFinancialForecast,
};
use rust_decimal::Decimal;
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::state::AppState;

/// Run an authorization/ownership check; on `Err(response)` release the RLS
/// connection and return that response from the handler. Evaluates to the
/// `Ok` value otherwise.
macro_rules! guard {
    ($rls:ident, $check:expr) => {
        match $check {
            Ok(value) => value,
            Err(rejection) => {
                $rls.release().await;
                return *rejection;
            }
        }
    };
}

// ===========================================
// Query Parameter Types
// ===========================================

/// Organization query parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct OrgQuery {
    pub organization_id: Uuid,
}

/// Building query parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct BuildingQuery {
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
}

/// Budget list query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct BudgetListQuery {
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub fiscal_year: Option<i32>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl From<&BudgetListQuery> for BudgetQuery {
    fn from(q: &BudgetListQuery) -> Self {
        BudgetQuery {
            building_id: q.building_id,
            fiscal_year: q.fiscal_year,
            status: q.status.clone(),
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Capital plan list query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct CapitalPlanListQuery {
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub target_year: Option<i32>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl From<&CapitalPlanListQuery> for CapitalPlanQuery {
    fn from(q: &CapitalPlanListQuery) -> Self {
        CapitalPlanQuery {
            building_id: q.building_id,
            target_year: q.target_year,
            status: q.status.clone(),
            priority: q.priority.clone(),
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Forecast list query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ForecastListQuery {
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub forecast_type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl From<&ForecastListQuery> for ForecastQuery {
    fn from(q: &ForecastListQuery) -> Self {
        ForecastQuery {
            building_id: q.building_id,
            forecast_type: q.forecast_type.clone(),
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Reserve projection query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ProjectionQuery {
    pub years: Option<i32>,
}

/// Request wrappers.
#[derive(Debug, Deserialize)]
pub struct CreateBudgetRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateBudget,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBudgetRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: UpdateBudget,
}

#[derive(Debug, Deserialize)]
pub struct CreateCategoryRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateBudgetCategory,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: UpdateBudgetCategory,
}

#[derive(Debug, Deserialize)]
pub struct CreateCapitalPlanRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateCapitalPlan,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCapitalPlanRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: UpdateCapitalPlan,
}

#[derive(Debug, Deserialize)]
pub struct CompleteCapitalPlanRequest {
    pub organization_id: Uuid,
    pub actual_cost: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct CreateForecastRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateFinancialForecast,
}

#[derive(Debug, Deserialize)]
pub struct UpdateForecastRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: UpdateFinancialForecast,
}

/// Create the budget router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Budget routes
        .route("/", post(create_budget))
        .route("/", get(list_budgets))
        .route("/{id}", get(get_budget))
        .route("/{id}", put(update_budget))
        .route("/{id}", delete(delete_budget))
        .route("/{id}/submit", post(submit_budget))
        .route("/{id}/approve", post(approve_budget))
        .route("/{id}/activate", post(activate_budget))
        .route("/{id}/close", post(close_budget))
        .route("/{id}/summary", get(get_budget_summary))
        .route("/{id}/variance", get(get_category_variance))
        .route("/{id}/alerts", get(list_variance_alerts))
        // Budget items
        .route("/{id}/items", post(add_budget_item))
        .route("/{id}/items", get(list_budget_items))
        .route("/items/{item_id}", put(update_budget_item))
        .route("/items/{item_id}", delete(delete_budget_item))
        .route("/items/{item_id}/actuals", post(record_actual))
        .route("/items/{item_id}/actuals", get(list_actuals))
        // Categories
        .route("/categories", post(create_category))
        .route("/categories", get(list_categories))
        .route("/categories/{id}", put(update_category))
        .route("/categories/{id}", delete(delete_category))
        // Alerts
        .route("/alerts/{id}/acknowledge", post(acknowledge_alert))
        // Dashboard
        .route("/dashboard", get(get_dashboard))
        // Capital plans
        .route("/capital-plans", post(create_capital_plan))
        .route("/capital-plans", get(list_capital_plans))
        .route("/capital-plans/summary", get(get_yearly_capital_summary))
        .route("/capital-plans/{id}", get(get_capital_plan))
        .route("/capital-plans/{id}", put(update_capital_plan))
        .route("/capital-plans/{id}", delete(delete_capital_plan))
        .route("/capital-plans/{id}/start", post(start_capital_plan))
        .route("/capital-plans/{id}/complete", post(complete_capital_plan))
        // Forecasts
        .route("/forecasts", post(create_forecast))
        .route("/forecasts", get(list_forecasts))
        .route("/forecasts/{id}", get(get_forecast))
        .route("/forecasts/{id}", put(update_forecast))
        .route("/forecasts/{id}", delete(delete_forecast))
}

// ===========================================
// Authorization & Ownership Helpers
// ===========================================

/// Boxed error response returned by the guard helpers (keeps `Result` small).
type Rejection = Box<Response>;

fn error_response(status: StatusCode, code: &str, message: &str) -> Response {
    (status, Json(ErrorResponse::new(code, message))).into_response()
}

fn not_found(message: &str) -> Response {
    error_response(StatusCode::NOT_FOUND, "NOT_FOUND", message)
}

/// Manager-level gate for budget mutations. Uses the database-validated role
/// carried by the [`RlsConnection`], not the JWT claim.
fn require_manager(rls: &RlsConnection) -> Result<(), Rejection> {
    if rls.role().is_manager() {
        Ok(())
    } else {
        Err(Box::new(error_response(
            StatusCode::FORBIDDEN,
            "FORBIDDEN",
            "Manager role required to perform this operation",
        )))
    }
}

/// The client-supplied `organization_id` must be the active (membership
/// validated) tenant; otherwise a member of org A could address org B's data
/// whenever RLS is not in force for the database role.
fn require_tenant_org(rls: &RlsConnection, organization_id: Uuid) -> Result<(), Rejection> {
    if organization_id == rls.tenant_id() {
        Ok(())
    } else {
        Err(Box::new(error_response(
            StatusCode::FORBIDDEN,
            "FORBIDDEN",
            "organization_id does not match the active organization",
        )))
    }
}

fn require_manager_in_org(rls: &RlsConnection, organization_id: Uuid) -> Result<(), Rejection> {
    require_tenant_org(rls, organization_id)?;
    require_manager(rls)
}

/// Budget lines and actuals follow the budget status machine
/// (`draft → pending_approval → approved → active → closed`).
fn require_budget_status(budget: &Budget, status: &str, message: &str) -> Result<(), Rejection> {
    if budget.status == status {
        Ok(())
    } else {
        Err(Box::new(error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_STATE",
            message,
        )))
    }
}

/// Map a database error to a response: FK violations become 400, unique
/// violations 409, everything else 500.
fn db_error(context: &str, e: sqlx::Error) -> Response {
    let code = e
        .as_database_error()
        .and_then(|d| d.code())
        .map(|c| c.into_owned());
    match code.as_deref() {
        Some("23503") => error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_REFERENCE",
            "A referenced record does not exist",
        ),
        Some("23505") => error_response(
            StatusCode::CONFLICT,
            "CONFLICT",
            "A conflicting record already exists",
        ),
        _ => {
            tracing::error!("Failed to {}: {:?}", context, e);
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                "Database error",
            )
        }
    }
}

/// Load a budget of the active tenant, or 404.
async fn load_budget(
    state: &AppState,
    rls: &mut RlsConnection,
    budget_id: Uuid,
) -> Result<Budget, Rejection> {
    let org_id = rls.tenant_id();
    state
        .budget_repo
        .find_budget_by_id_rls(&mut **rls.conn(), org_id, budget_id)
        .await
        .map_err(|e| Box::new(db_error("load budget", e)))?
        .ok_or_else(|| Box::new(not_found("Budget not found")))
}

/// Load the budget owning `item_id` within the active tenant, or 404.
async fn load_item_budget(
    state: &AppState,
    rls: &mut RlsConnection,
    item_id: Uuid,
) -> Result<Budget, Rejection> {
    let org_id = rls.tenant_id();
    state
        .budget_repo
        .find_budget_for_item_rls(&mut **rls.conn(), org_id, item_id)
        .await
        .map_err(|e| Box::new(db_error("load budget item", e)))?
        .ok_or_else(|| Box::new(not_found("Budget item not found")))
}

/// The category must exist in the active tenant (FK checks bypass RLS).
async fn require_category_in_org(
    state: &AppState,
    rls: &mut RlsConnection,
    category_id: Uuid,
) -> Result<(), Rejection> {
    let org_id = rls.tenant_id();
    let exists = state
        .budget_repo
        .category_in_org_rls(&mut **rls.conn(), org_id, category_id)
        .await
        .map_err(|e| Box::new(db_error("check budget category", e)))?;
    if exists {
        Ok(())
    } else {
        Err(Box::new(error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_CATEGORY",
            "Budget category not found",
        )))
    }
}

// ===========================================
// Budget Handlers
// ===========================================

async fn create_budget(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Json(req): Json<CreateBudgetRequest>,
) -> impl IntoResponse {
    guard!(rls, require_manager_in_org(&rls, req.organization_id));
    match state
        .budget_repo
        .create_budget_rls(
            &mut **rls.conn(),
            req.organization_id,
            auth.user_id,
            req.data,
        )
        .await
    {
        Ok(budget) => {
            rls.release().await;
            (StatusCode::CREATED, Json(budget)).into_response()
        }
        Err(e) => {
            rls.release().await;
            db_error("create budget", e)
        }
    }
}

async fn list_budgets(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Query(query): Query<BudgetListQuery>,
) -> impl IntoResponse {
    guard!(rls, require_tenant_org(&rls, query.organization_id));
    match state
        .budget_repo
        .list_budgets_rls(
            &mut **rls.conn(),
            query.organization_id,
            BudgetQuery::from(&query),
        )
        .await
    {
        Ok(budgets) => {
            rls.release().await;
            Json(budgets).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to list budgets: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn get_budget(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    guard!(rls, require_tenant_org(&rls, query.organization_id));
    match state
        .budget_repo
        .find_budget_by_id_rls(&mut **rls.conn(), query.organization_id, id)
        .await
    {
        Ok(Some(budget)) => {
            rls.release().await;
            Json(budget).into_response()
        }
        Ok(None) => {
            rls.release().await;
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Budget not found")),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get budget: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn update_budget(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateBudgetRequest>,
) -> impl IntoResponse {
    guard!(rls, require_manager_in_org(&rls, req.organization_id));
    match state
        .budget_repo
        .update_budget_rls(&mut **rls.conn(), req.organization_id, id, req.data)
        .await
    {
        Ok(Some(budget)) => {
            rls.release().await;
            Json(budget).into_response()
        }
        Ok(None) => {
            rls.release().await;
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Budget not found")),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to update budget: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn delete_budget(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    guard!(rls, require_manager_in_org(&rls, query.organization_id));
    guard!(rls, load_budget(&state, &mut rls, id).await);
    match state
        .budget_repo
        .delete_budget_rls(&mut **rls.conn(), query.organization_id, id)
        .await
    {
        Ok(true) => {
            rls.release().await;
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => {
            rls.release().await;
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_STATE",
                    "Only draft budgets can be deleted",
                )),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to delete budget: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn submit_budget(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    guard!(rls, require_manager_in_org(&rls, query.organization_id));
    guard!(rls, load_budget(&state, &mut rls, id).await);
    match state
        .budget_repo
        .submit_budget_for_approval_rls(&mut **rls.conn(), query.organization_id, id)
        .await
    {
        Ok(Some(budget)) => {
            rls.release().await;
            Json(budget).into_response()
        }
        Ok(None) => {
            rls.release().await;
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_STATE",
                    "Budget cannot be submitted",
                )),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to submit budget: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn approve_budget(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    guard!(rls, require_manager_in_org(&rls, query.organization_id));
    let budget = guard!(rls, load_budget(&state, &mut rls, id).await);
    // Separation of duties: the author of a budget cannot be its approver,
    // unless they are the organization's only active manager-level member
    // (otherwise a single-manager org could never approve a budget).
    if budget.created_by == auth.user_id {
        match state
            .org_member_repo
            .count_other_active_managers(budget.organization_id, auth.user_id)
            .await
        {
            Ok(0) => {}
            Ok(_) => {
                rls.release().await;
                return error_response(
                    StatusCode::FORBIDDEN,
                    "FORBIDDEN",
                    "A budget cannot be approved by the user who created it",
                );
            }
            Err(e) => {
                tracing::error!("Failed to count organization managers: {:?}", e);
                rls.release().await;
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DB_ERROR",
                    "Database error",
                );
            }
        }
    }
    match state
        .budget_repo
        .approve_budget_rls(&mut **rls.conn(), query.organization_id, id, auth.user_id)
        .await
    {
        Ok(Some(budget)) => {
            rls.release().await;
            Json(budget).into_response()
        }
        Ok(None) => {
            rls.release().await;
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_STATE",
                    "Budget cannot be approved",
                )),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to approve budget: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn activate_budget(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    guard!(rls, require_manager_in_org(&rls, query.organization_id));
    guard!(rls, load_budget(&state, &mut rls, id).await);
    match state
        .budget_repo
        .activate_budget_rls(&mut **rls.conn(), query.organization_id, id)
        .await
    {
        Ok(Some(budget)) => {
            rls.release().await;
            Json(budget).into_response()
        }
        Ok(None) => {
            rls.release().await;
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_STATE",
                    "Budget cannot be activated",
                )),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to activate budget: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn close_budget(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    guard!(rls, require_manager_in_org(&rls, query.organization_id));
    guard!(rls, load_budget(&state, &mut rls, id).await);
    match state
        .budget_repo
        .close_budget_rls(&mut **rls.conn(), query.organization_id, id)
        .await
    {
        Ok(Some(budget)) => {
            rls.release().await;
            Json(budget).into_response()
        }
        Ok(None) => {
            rls.release().await;
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_STATE",
                    "Budget cannot be closed",
                )),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to close budget: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn get_budget_summary(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    guard!(rls, load_budget(&state, &mut rls, id).await);
    match state
        .budget_repo
        .get_budget_summary_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(summary) => {
            rls.release().await;
            Json(summary).into_response()
        }
        Err(e) => {
            rls.release().await;
            db_error("get budget summary", e)
        }
    }
}

async fn get_category_variance(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    guard!(rls, load_budget(&state, &mut rls, id).await);
    match state
        .budget_repo
        .get_category_variance_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(variance) => {
            rls.release().await;
            Json(variance).into_response()
        }
        Err(e) => {
            rls.release().await;
            db_error("get category variance", e)
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
struct AlertsQuery {
    acknowledged: Option<bool>,
}

async fn list_variance_alerts(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<AlertsQuery>,
) -> impl IntoResponse {
    guard!(rls, load_budget(&state, &mut rls, id).await);
    match state
        .budget_repo
        .list_variance_alerts_rls(&mut **rls.conn(), id, query.acknowledged)
        .await
    {
        Ok(alerts) => {
            rls.release().await;
            Json(alerts).into_response()
        }
        Err(e) => {
            rls.release().await;
            db_error("list variance alerts", e)
        }
    }
}

// ===========================================
// Budget Item Handlers
// ===========================================

async fn add_budget_item(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<CreateBudgetItem>,
) -> impl IntoResponse {
    guard!(rls, require_manager(&rls));
    let budget = guard!(rls, load_budget(&state, &mut rls, id).await);
    guard!(
        rls,
        require_budget_status(
            &budget,
            budget_status::DRAFT,
            "Budget items can only be added while the budget is a draft",
        )
    );
    guard!(
        rls,
        require_category_in_org(&state, &mut rls, data.category_id).await
    );
    match state
        .budget_repo
        .add_budget_item_rls(&mut **rls.conn(), id, data)
        .await
    {
        Ok(item) => {
            rls.release().await;
            (StatusCode::CREATED, Json(item)).into_response()
        }
        Err(e) => {
            rls.release().await;
            db_error("add budget item", e)
        }
    }
}

async fn list_budget_items(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    guard!(rls, load_budget(&state, &mut rls, id).await);
    match state
        .budget_repo
        .list_budget_items_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(items) => {
            rls.release().await;
            Json(items).into_response()
        }
        Err(e) => {
            rls.release().await;
            db_error("list budget items", e)
        }
    }
}

async fn update_budget_item(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(item_id): Path<Uuid>,
    Json(data): Json<UpdateBudgetItem>,
) -> impl IntoResponse {
    guard!(rls, require_manager(&rls));
    let budget = guard!(rls, load_item_budget(&state, &mut rls, item_id).await);
    guard!(
        rls,
        require_budget_status(
            &budget,
            budget_status::DRAFT,
            "Budget items can only be edited while the budget is a draft",
        )
    );
    match state
        .budget_repo
        .update_budget_item_rls(&mut **rls.conn(), item_id, data)
        .await
    {
        Ok(Some(item)) => {
            rls.release().await;
            Json(item).into_response()
        }
        Ok(None) => {
            rls.release().await;
            not_found("Budget item not found")
        }
        Err(e) => {
            rls.release().await;
            db_error("update budget item", e)
        }
    }
}

async fn delete_budget_item(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(item_id): Path<Uuid>,
) -> impl IntoResponse {
    guard!(rls, require_manager(&rls));
    let budget = guard!(rls, load_item_budget(&state, &mut rls, item_id).await);
    guard!(
        rls,
        require_budget_status(
            &budget,
            budget_status::DRAFT,
            "Budget items can only be deleted while the budget is a draft",
        )
    );
    match state
        .budget_repo
        .delete_budget_item_rls(&mut **rls.conn(), item_id)
        .await
    {
        Ok(true) => {
            rls.release().await;
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => {
            rls.release().await;
            not_found("Budget item not found")
        }
        Err(e) => {
            rls.release().await;
            db_error("delete budget item", e)
        }
    }
}

async fn record_actual(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(item_id): Path<Uuid>,
    Json(data): Json<RecordBudgetActual>,
) -> impl IntoResponse {
    guard!(rls, require_manager(&rls));
    let budget = guard!(rls, load_item_budget(&state, &mut rls, item_id).await);
    guard!(
        rls,
        require_budget_status(
            &budget,
            budget_status::ACTIVE,
            "Actuals can only be recorded against an active budget",
        )
    );
    match state
        .budget_repo
        .record_actual_rls(&mut **rls.conn(), item_id, auth.user_id, data)
        .await
    {
        Ok(actual) => {
            rls.release().await;
            (StatusCode::CREATED, Json(actual)).into_response()
        }
        Err(e) => {
            rls.release().await;
            db_error("record actual", e)
        }
    }
}

async fn list_actuals(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(item_id): Path<Uuid>,
) -> impl IntoResponse {
    guard!(rls, load_item_budget(&state, &mut rls, item_id).await);
    match state
        .budget_repo
        .list_actuals_rls(&mut **rls.conn(), item_id)
        .await
    {
        Ok(actuals) => {
            rls.release().await;
            Json(actuals).into_response()
        }
        Err(e) => {
            rls.release().await;
            db_error("list actuals", e)
        }
    }
}

// ===========================================
// Category Handlers
// ===========================================

async fn create_category(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Json(req): Json<CreateCategoryRequest>,
) -> impl IntoResponse {
    guard!(rls, require_manager_in_org(&rls, req.organization_id));
    if let Some(parent_id) = req.data.parent_id {
        guard!(
            rls,
            require_category_in_org(&state, &mut rls, parent_id).await
        );
    }
    match state
        .budget_repo
        .create_category_rls(&mut **rls.conn(), req.organization_id, req.data)
        .await
    {
        Ok(category) => {
            rls.release().await;
            (StatusCode::CREATED, Json(category)).into_response()
        }
        Err(e) => {
            rls.release().await;
            db_error("create category", e)
        }
    }
}

async fn list_categories(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    guard!(rls, require_tenant_org(&rls, query.organization_id));
    match state
        .budget_repo
        .list_categories_rls(&mut **rls.conn(), query.organization_id)
        .await
    {
        Ok(categories) => {
            rls.release().await;
            Json(categories).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to list categories: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn update_category(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCategoryRequest>,
) -> impl IntoResponse {
    guard!(rls, require_manager_in_org(&rls, req.organization_id));
    match state
        .budget_repo
        .update_category_rls(&mut **rls.conn(), req.organization_id, id, req.data)
        .await
    {
        Ok(Some(category)) => {
            rls.release().await;
            Json(category).into_response()
        }
        Ok(None) => {
            rls.release().await;
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Category not found")),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to update category: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn delete_category(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    guard!(rls, require_manager_in_org(&rls, query.organization_id));
    match state
        .budget_repo
        .delete_category_rls(&mut **rls.conn(), query.organization_id, id)
        .await
    {
        Ok(true) => {
            rls.release().await;
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => {
            rls.release().await;
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Category not found")),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to delete category: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

// ===========================================
// Alert Handler
// ===========================================

async fn acknowledge_alert(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<AcknowledgeVarianceAlert>,
) -> impl IntoResponse {
    guard!(rls, require_manager(&rls));
    let org_id = rls.tenant_id();
    match state
        .budget_repo
        .find_budget_for_alert_rls(&mut **rls.conn(), org_id, id)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            rls.release().await;
            return not_found("Alert not found");
        }
        Err(e) => {
            rls.release().await;
            return db_error("load variance alert", e);
        }
    }
    match state
        .budget_repo
        .acknowledge_alert_rls(&mut **rls.conn(), id, auth.user_id, data)
        .await
    {
        Ok(Some(alert)) => {
            rls.release().await;
            Json(alert).into_response()
        }
        Ok(None) => {
            rls.release().await;
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Alert not found")),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to acknowledge alert: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

// ===========================================
// Dashboard Handler
// ===========================================

async fn get_dashboard(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Query(query): Query<BuildingQuery>,
) -> impl IntoResponse {
    guard!(rls, require_tenant_org(&rls, query.organization_id));
    // All dashboard queries run on the RLS-bound connection so RLS policies
    // are enforced consistently across the budget, alert and reserve lookups.
    match state
        .budget_repo
        .get_dashboard(rls.conn(), query.organization_id, query.building_id)
        .await
    {
        Ok(dashboard) => {
            rls.release().await;
            Json(dashboard).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get dashboard: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

// ===========================================
// Capital Plan Handlers
// ===========================================

async fn create_capital_plan(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Json(req): Json<CreateCapitalPlanRequest>,
) -> impl IntoResponse {
    guard!(rls, require_manager_in_org(&rls, req.organization_id));
    match state
        .budget_repo
        .create_capital_plan_rls(
            &mut **rls.conn(),
            req.organization_id,
            auth.user_id,
            req.data,
        )
        .await
    {
        Ok(plan) => {
            rls.release().await;
            (StatusCode::CREATED, Json(plan)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to create capital plan: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn list_capital_plans(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Query(query): Query<CapitalPlanListQuery>,
) -> impl IntoResponse {
    guard!(rls, require_tenant_org(&rls, query.organization_id));
    match state
        .budget_repo
        .list_capital_plans_rls(
            &mut **rls.conn(),
            query.organization_id,
            CapitalPlanQuery::from(&query),
        )
        .await
    {
        Ok(plans) => {
            rls.release().await;
            Json(plans).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to list capital plans: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn get_yearly_capital_summary(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    guard!(rls, require_tenant_org(&rls, query.organization_id));
    match state
        .budget_repo
        .get_yearly_capital_summary_rls(&mut **rls.conn(), query.organization_id)
        .await
    {
        Ok(summary) => {
            rls.release().await;
            Json(summary).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get yearly capital summary: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn get_capital_plan(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    guard!(rls, require_tenant_org(&rls, query.organization_id));
    match state
        .budget_repo
        .find_capital_plan_by_id_rls(&mut **rls.conn(), query.organization_id, id)
        .await
    {
        Ok(Some(plan)) => {
            rls.release().await;
            Json(plan).into_response()
        }
        Ok(None) => {
            rls.release().await;
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Capital plan not found")),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get capital plan: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn update_capital_plan(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCapitalPlanRequest>,
) -> impl IntoResponse {
    guard!(rls, require_manager_in_org(&rls, req.organization_id));
    match state
        .budget_repo
        .update_capital_plan_rls(&mut **rls.conn(), req.organization_id, id, req.data)
        .await
    {
        Ok(Some(plan)) => {
            rls.release().await;
            Json(plan).into_response()
        }
        Ok(None) => {
            rls.release().await;
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Capital plan not found")),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to update capital plan: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn delete_capital_plan(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    guard!(rls, require_manager_in_org(&rls, query.organization_id));
    match state
        .budget_repo
        .delete_capital_plan_rls(&mut **rls.conn(), query.organization_id, id)
        .await
    {
        Ok(true) => {
            rls.release().await;
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => {
            rls.release().await;
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_STATE",
                    "Only planned capital plans can be deleted",
                )),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to delete capital plan: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn start_capital_plan(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    guard!(rls, require_manager_in_org(&rls, query.organization_id));
    match state
        .budget_repo
        .start_capital_plan_rls(&mut **rls.conn(), query.organization_id, id)
        .await
    {
        Ok(Some(plan)) => {
            rls.release().await;
            Json(plan).into_response()
        }
        Ok(None) => {
            rls.release().await;
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_STATE",
                    "Capital plan cannot be started",
                )),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to start capital plan: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn complete_capital_plan(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<CompleteCapitalPlanRequest>,
) -> impl IntoResponse {
    guard!(rls, require_manager_in_org(&rls, req.organization_id));
    match state
        .budget_repo
        .complete_capital_plan_rls(&mut **rls.conn(), req.organization_id, id, req.actual_cost)
        .await
    {
        Ok(Some(plan)) => {
            rls.release().await;
            Json(plan).into_response()
        }
        Ok(None) => {
            rls.release().await;
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_STATE",
                    "Capital plan cannot be completed",
                )),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to complete capital plan: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

// ===========================================
// Forecast Handlers
// ===========================================

async fn create_forecast(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Json(req): Json<CreateForecastRequest>,
) -> impl IntoResponse {
    guard!(rls, require_manager_in_org(&rls, req.organization_id));
    match state
        .budget_repo
        .create_forecast_rls(
            &mut **rls.conn(),
            req.organization_id,
            auth.user_id,
            req.data,
        )
        .await
    {
        Ok(forecast) => {
            rls.release().await;
            (StatusCode::CREATED, Json(forecast)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to create forecast: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn list_forecasts(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Query(query): Query<ForecastListQuery>,
) -> impl IntoResponse {
    guard!(rls, require_tenant_org(&rls, query.organization_id));
    match state
        .budget_repo
        .list_forecasts_rls(
            &mut **rls.conn(),
            query.organization_id,
            ForecastQuery::from(&query),
        )
        .await
    {
        Ok(forecasts) => {
            rls.release().await;
            Json(forecasts).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to list forecasts: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn get_forecast(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    guard!(rls, require_tenant_org(&rls, query.organization_id));
    match state
        .budget_repo
        .find_forecast_by_id_rls(&mut **rls.conn(), query.organization_id, id)
        .await
    {
        Ok(Some(forecast)) => {
            rls.release().await;
            Json(forecast).into_response()
        }
        Ok(None) => {
            rls.release().await;
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Forecast not found")),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get forecast: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn update_forecast(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateForecastRequest>,
) -> impl IntoResponse {
    guard!(rls, require_manager_in_org(&rls, req.organization_id));
    match state
        .budget_repo
        .update_forecast_rls(&mut **rls.conn(), req.organization_id, id, req.data)
        .await
    {
        Ok(Some(forecast)) => {
            rls.release().await;
            Json(forecast).into_response()
        }
        Ok(None) => {
            rls.release().await;
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Forecast not found")),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to update forecast: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn delete_forecast(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    guard!(rls, require_manager_in_org(&rls, query.organization_id));
    match state
        .budget_repo
        .delete_forecast_rls(&mut **rls.conn(), query.organization_id, id)
        .await
    {
        Ok(true) => {
            rls.release().await;
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => {
            rls.release().await;
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Forecast not found")),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to delete forecast: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}
