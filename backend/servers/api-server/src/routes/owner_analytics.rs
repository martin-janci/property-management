//! Owner Investment Analytics routes (Epic 74).
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
use db::models::owner_analytics::{
    AddComparableProperty, CalculateROIRequest, CreateAutoApprovalRule, CreatePropertyValuation,
    ExpenseRequestsQuery, OwnerPropertiesQuery, PortfolioComparisonRequest, ReviewExpenseRequest,
    SubmitExpenseForApproval, UpdateAutoApprovalRule, ValueHistoryQuery,
};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

type ApiResult<T> = Result<T, (StatusCode, Json<ErrorResponse>)>;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/units/{unit_id}/valuation", get(get_unit_valuation))
        .route("/units/{unit_id}/valuation", post(create_valuation))
        .route(
            "/valuations/{valuation_id}",
            get(get_valuation_with_comparables),
        )
        .route(
            "/valuations/{valuation_id}/comparables",
            post(add_comparable),
        )
        .route("/units/{unit_id}/value-history", get(get_value_history))
        .route("/units/{unit_id}/value-trend", get(get_value_trend))
        .route("/units/{unit_id}/roi", post(calculate_roi))
        .route("/units/{unit_id}/cash-flow", get(get_cash_flow_breakdown))
        .route("/units/{unit_id}/roi-dashboard", get(get_roi_dashboard))
        .route("/portfolio", get(get_portfolio_summary))
        .route("/portfolio/compare", post(compare_properties))
        .route("/expense-rules", get(list_auto_approval_rules))
        .route("/expense-rules", post(create_auto_approval_rule))
        .route("/expense-rules/{id}", put(update_auto_approval_rule))
        .route("/expense-rules/{id}", delete(delete_auto_approval_rule))
        .route("/expenses/submit", post(submit_expense))
        .route("/expenses", get(list_expense_requests))
        .route("/expenses/{id}/review", post(review_expense))
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ValueHistoryParams {
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct CashFlowParams {
    pub from_date: NaiveDate,
    pub to_date: NaiveDate,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ROIDashboardParams {
    pub from_date: NaiveDate,
    pub to_date: NaiveDate,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct PortfolioParams {
    pub owner_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateValuationRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreatePropertyValuation,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CalculateROIWithOrg {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CalculateROIRequest,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRuleRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateAutoApprovalRule,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SubmitExpenseRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: SubmitExpenseForApproval,
}

async fn get_unit_valuation(
    State(s): State<AppState>,
    user: AuthUser,
    Path(uid): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let org = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;
    match s.owner_analytics_repo.get_latest_valuation(uid, org).await {
        Ok(Some(v)) => Ok(Json(serde_json::to_value(v).unwrap())),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn create_valuation(
    State(s): State<AppState>,
    _user: AuthUser,
    Path(_uid): Path<Uuid>,
    Json(r): Json<CreateValuationRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    match s
        .owner_analytics_repo
        .create_valuation(r.organization_id, r.data)
        .await
    {
        Ok(v) => Ok(Json(serde_json::to_value(v).unwrap())),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

async fn get_valuation_with_comparables(
    State(s): State<AppState>,
    user: AuthUser,
    Path(vid): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let org = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;
    match s
        .owner_analytics_repo
        .get_valuation_with_comparables(vid, org)
        .await
    {
        Ok(Some(v)) => Ok(Json(serde_json::to_value(v).unwrap())),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn add_comparable(
    State(s): State<AppState>,
    _user: AuthUser,
    Path(vid): Path<Uuid>,
    Json(r): Json<AddComparableProperty>,
) -> ApiResult<Json<serde_json::Value>> {
    match s.owner_analytics_repo.add_comparable(vid, r).await {
        Ok(c) => Ok(Json(serde_json::to_value(c).unwrap())),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

async fn get_value_history(
    State(s): State<AppState>,
    _user: AuthUser,
    Path(uid): Path<Uuid>,
    Query(p): Query<ValueHistoryParams>,
) -> ApiResult<Json<serde_json::Value>> {
    match s
        .owner_analytics_repo
        .get_value_history(ValueHistoryQuery {
            unit_id: uid,
            limit: p.limit,
        })
        .await
    {
        Ok(h) => Ok(Json(serde_json::to_value(h).unwrap())),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn get_value_trend(
    State(s): State<AppState>,
    user: AuthUser,
    Path(uid): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let org = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;
    match s.owner_analytics_repo.get_value_trend(uid, org).await {
        Ok(Some(t)) => Ok(Json(serde_json::to_value(t).unwrap())),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn calculate_roi(
    State(s): State<AppState>,
    _user: AuthUser,
    Path(_uid): Path<Uuid>,
    Json(r): Json<CalculateROIWithOrg>,
) -> ApiResult<Json<serde_json::Value>> {
    match s
        .owner_analytics_repo
        .calculate_roi(r.organization_id, r.data)
        .await
    {
        Ok(roi) => Ok(Json(serde_json::to_value(roi).unwrap())),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

async fn get_cash_flow_breakdown(
    State(s): State<AppState>,
    user: AuthUser,
    Path(uid): Path<Uuid>,
    Query(p): Query<CashFlowParams>,
) -> ApiResult<Json<serde_json::Value>> {
    let org = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;
    match s
        .owner_analytics_repo
        .get_cash_flow_breakdown(uid, org, p.from_date, p.to_date)
        .await
    {
        Ok(cf) => Ok(Json(serde_json::to_value(cf).unwrap())),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn get_roi_dashboard(
    State(s): State<AppState>,
    user: AuthUser,
    Path(uid): Path<Uuid>,
    Query(p): Query<ROIDashboardParams>,
) -> ApiResult<Json<serde_json::Value>> {
    let org = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;
    match s
        .owner_analytics_repo
        .get_roi_dashboard(uid, org, p.from_date, p.to_date)
        .await
    {
        Ok(d) => Ok(Json(serde_json::to_value(d).unwrap())),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn get_portfolio_summary(
    State(s): State<AppState>,
    user: AuthUser,
    Query(p): Query<PortfolioParams>,
) -> ApiResult<Json<serde_json::Value>> {
    match s
        .owner_analytics_repo
        .get_portfolio_summary(OwnerPropertiesQuery {
            owner_id: p.owner_id,
            organization_id: user.tenant_id,
        })
        .await
    {
        Ok(ps) => Ok(Json(serde_json::to_value(ps).unwrap())),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn compare_properties(
    State(s): State<AppState>,
    user: AuthUser,
    Json(r): Json<PortfolioComparisonRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let org = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;
    match s.owner_analytics_repo.compare_properties(org, r).await {
        Ok(c) => Ok(Json(serde_json::to_value(c).unwrap())),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

async fn list_auto_approval_rules(
    State(s): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let org = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;
    match s
        .owner_analytics_repo
        .get_auto_approval_rules(user.user_id, org)
        .await
    {
        Ok(r) => Ok(Json(serde_json::to_value(r).unwrap())),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn create_auto_approval_rule(
    State(s): State<AppState>,
    user: AuthUser,
    Json(r): Json<CreateRuleRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    match s
        .owner_analytics_repo
        .create_auto_approval_rule(user.user_id, r.organization_id, r.data)
        .await
    {
        Ok(rule) => Ok(Json(serde_json::to_value(rule).unwrap())),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

async fn update_auto_approval_rule(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(r): Json<UpdateAutoApprovalRule>,
) -> ApiResult<Json<serde_json::Value>> {
    match s
        .owner_analytics_repo
        .update_auto_approval_rule(id, user.user_id, r)
        .await
    {
        Ok(rule) => Ok(Json(serde_json::to_value(rule).unwrap())),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

async fn delete_auto_approval_rule(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    match s
        .owner_analytics_repo
        .delete_auto_approval_rule(id, user.user_id)
        .await
    {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found(&e.to_string())),
        )),
    }
}

async fn submit_expense(
    State(s): State<AppState>,
    user: AuthUser,
    Json(r): Json<SubmitExpenseRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    match s
        .owner_analytics_repo
        .submit_expense_for_approval(user.user_id, r.organization_id, r.data)
        .await
    {
        Ok(resp) => Ok(Json(serde_json::to_value(resp).unwrap())),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

async fn list_expense_requests(
    State(s): State<AppState>,
    user: AuthUser,
    Query(q): Query<ExpenseRequestsQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let org = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;
    match s.owner_analytics_repo.list_expense_requests(org, q).await {
        Ok(r) => Ok(Json(serde_json::to_value(r).unwrap())),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn review_expense(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(r): Json<ReviewExpenseRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    match s
        .owner_analytics_repo
        .review_expense(id, user.user_id, r)
        .await
    {
        Ok(e) => Ok(Json(serde_json::to_value(e).unwrap())),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}
