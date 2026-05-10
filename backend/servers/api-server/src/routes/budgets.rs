//! Budget and financial planning routes for Epic 24.
//!
//! Handles budgets, budget items, capital plans, reserve funds, and forecasts.

use api_core::extractors::{AuthUser, RlsConnection};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::ErrorResponse;
use db::models::{
    AcknowledgeVarianceAlert, BudgetQuery, CapitalPlanQuery, CreateBudget, CreateBudgetCategory,
    CreateBudgetItem, CreateCapitalPlan, CreateFinancialForecast, CreateReserveFund, ForecastQuery,
    RecordBudgetActual, RecordReserveTransaction, UpdateBudget, UpdateBudgetCategory,
    UpdateBudgetItem, UpdateCapitalPlan, UpdateFinancialForecast, UpdateReserveFund,
};
use rust_decimal::Decimal;
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::state::AppState;

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
pub struct CreateReserveFundRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateReserveFund,
}

#[derive(Debug, Deserialize)]
pub struct UpdateReserveFundRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: UpdateReserveFund,
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
        // Reserve funds
        .route("/reserve-funds", post(create_reserve_fund))
        .route("/reserve-funds", get(list_reserve_funds))
        .route("/reserve-funds/{id}", get(get_reserve_fund))
        .route("/reserve-funds/{id}", put(update_reserve_fund))
        .route(
            "/reserve-funds/{id}/transactions",
            post(record_reserve_transaction),
        )
        .route(
            "/reserve-funds/{id}/transactions",
            get(list_reserve_transactions),
        )
        .route(
            "/reserve-funds/{id}/projection",
            get(get_reserve_projection),
        )
        // Forecasts
        .route("/forecasts", post(create_forecast))
        .route("/forecasts", get(list_forecasts))
        .route("/forecasts/{id}", get(get_forecast))
        .route("/forecasts/{id}", put(update_forecast))
        .route("/forecasts/{id}", delete(delete_forecast))
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
            tracing::error!("Failed to create budget: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn list_budgets(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Query(query): Query<BudgetListQuery>,
) -> impl IntoResponse {
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
            tracing::error!("Failed to get budget summary: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn get_category_variance(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
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
            tracing::error!("Failed to get category variance: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
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
            tracing::error!("Failed to list variance alerts: {:?}", e);
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
// Budget Item Handlers
// ===========================================

async fn add_budget_item(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<CreateBudgetItem>,
) -> impl IntoResponse {
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
            tracing::error!("Failed to add budget item: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn list_budget_items(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
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
            tracing::error!("Failed to list budget items: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
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
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Budget item not found")),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to update budget item: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn delete_budget_item(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(item_id): Path<Uuid>,
) -> impl IntoResponse {
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
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Budget item not found")),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to delete budget item: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
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
            tracing::error!("Failed to record actual: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn list_actuals(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(item_id): Path<Uuid>,
) -> impl IntoResponse {
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
            tracing::error!("Failed to list actuals: {:?}", e);
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
// Category Handlers
// ===========================================

async fn create_category(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Json(req): Json<CreateCategoryRequest>,
) -> impl IntoResponse {
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
            tracing::error!("Failed to create category: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn list_categories(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
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
    // The dashboard currently performs multiple queries using the legacy
    // repository method, which internally uses the shared pool rather than
    // the RLS-bound connection provided here. This means the queries do not
    // all run on a single RLS-enforced connection.
    //
    // For full RLS support, this handler (or the repository) needs to be
    // refactored so all dashboard queries execute using `rls.conn()`, either
    // by:
    //   - moving the dashboard logic to use the RLS connection inside a
    //     single transaction, or
    //   - rewriting the dashboard logic into a combined query (or small set
    //     of queries) that can run via the RLS-aware connection.
    #[allow(deprecated)]
    match state
        .budget_repo
        .get_dashboard(query.organization_id, query.building_id)
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
// Reserve Fund Handlers
// ===========================================

async fn create_reserve_fund(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Json(req): Json<CreateReserveFundRequest>,
) -> impl IntoResponse {
    match state
        .budget_repo
        .create_reserve_fund_rls(&mut **rls.conn(), req.organization_id, req.data)
        .await
    {
        Ok(fund) => {
            rls.release().await;
            (StatusCode::CREATED, Json(fund)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to create reserve fund: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn list_reserve_funds(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Query(query): Query<BuildingQuery>,
) -> impl IntoResponse {
    match state
        .budget_repo
        .list_reserve_funds_rls(&mut **rls.conn(), query.organization_id, query.building_id)
        .await
    {
        Ok(funds) => {
            rls.release().await;
            Json(funds).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to list reserve funds: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn get_reserve_fund(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> impl IntoResponse {
    match state
        .budget_repo
        .find_reserve_fund_by_id_rls(&mut **rls.conn(), query.organization_id, id)
        .await
    {
        Ok(Some(fund)) => {
            rls.release().await;
            Json(fund).into_response()
        }
        Ok(None) => {
            rls.release().await;
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Reserve fund not found")),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get reserve fund: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn update_reserve_fund(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateReserveFundRequest>,
) -> impl IntoResponse {
    match state
        .budget_repo
        .update_reserve_fund_rls(&mut **rls.conn(), req.organization_id, id, req.data)
        .await
    {
        Ok(Some(fund)) => {
            rls.release().await;
            Json(fund).into_response()
        }
        Ok(None) => {
            rls.release().await;
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Reserve fund not found")),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to update reserve fund: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn record_reserve_transaction(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<RecordReserveTransaction>,
) -> impl IntoResponse {
    // The current implementation of record_reserve_transaction requires the
    // reserve fund's current balance and therefore issues multiple queries.
    //
    // This bypasses the RLS-aware API for now and uses a deprecated method.
    // TODO(epic-24): Refactor this into a single RLS-compatible operation:
    //   * either implement record_reserve_transaction as a database stored
    //     procedure that atomically reads the balance and records the
    //     transaction under RLS, or
    //   * introduce a record_reserve_transaction_rls(...) repository method
    //     that performs the required queries within a transaction using
    //     RlsConnection so that RLS policies are correctly enforced.
    #[allow(deprecated)]
    match state
        .budget_repo
        .record_reserve_transaction(id, auth.user_id, data)
        .await
    {
        Ok(txn) => {
            rls.release().await;
            (StatusCode::CREATED, Json(txn)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to record reserve transaction: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn list_reserve_transactions(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state
        .budget_repo
        .list_reserve_transactions_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(transactions) => {
            rls.release().await;
            Json(transactions).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to list reserve transactions: {:?}", e);
            rls.release().await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
                .into_response()
        }
    }
}

async fn get_reserve_projection(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<ProjectionQuery>,
) -> impl IntoResponse {
    let years = query.years.unwrap_or(5);
    // The generate_reserve_projection method requires multiple queries
    // and uses the pool directly. For full RLS support, this would need
    // to be refactored.
    match state
        .budget_repo
        .generate_reserve_projection(id, years)
        .await
    {
        Ok(projection) => {
            rls.release().await;
            Json(projection).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get reserve projection: {:?}", e);
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
