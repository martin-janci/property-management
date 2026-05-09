//! Subscription and billing routes (Epic 26).

use api_core::extractors::{AuthUser, RlsConnection};
use axum::{
    extract::{Path, Query, State},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    routing::{delete, get, patch, post},
    Json, Router,
};
use chrono::{NaiveDate, Utc};
use common::errors::ErrorResponse;
use db::models::{
    CancelSubscriptionRequest, ChangePlanRequest, CouponRedemption, CreateOrganizationSubscription,
    CreateSubscriptionCoupon, CreateSubscriptionPaymentMethod, CreateSubscriptionPlan,
    CreateUsageRecord, InvoiceLineItem, InvoiceQueryParams, InvoiceWithDetails,
    OrganizationSubscription, RedeemCouponRequest, SubscriptionCoupon, SubscriptionInvoice,
    SubscriptionPaymentMethod, SubscriptionPlan, SubscriptionStatistics, SubscriptionWithPlan,
    UpdateOrganizationSubscription, UpdateSubscriptionCoupon, UpdateSubscriptionPlan, UsageSummary,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::state::AppState;

// ==================== Authorization Helpers ====================

/// Super admin role names for platform-level operations.
const SUPER_ADMIN_ROLES: &[&str] = &[
    "SuperAdministrator",
    "super_admin",
    "superadmin",
    "platform_admin",
];

/// Check if the user has super admin role.
fn has_super_admin_role(roles: &Option<Vec<String>>) -> bool {
    match roles {
        Some(user_roles) => user_roles.iter().any(|r| {
            SUPER_ADMIN_ROLES
                .iter()
                .any(|admin| r.eq_ignore_ascii_case(admin))
        }),
        None => false,
    }
}

/// Require super admin role for platform-level operations.
fn require_super_admin(
    headers: &HeaderMap,
    state: &AppState,
) -> Result<Uuid, (StatusCode, Json<ErrorResponse>)> {
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "MISSING_TOKEN",
                    "Authorization header required",
                )),
            )
        })?;

    if !auth_header.starts_with("Bearer ") {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Bearer token required")),
        ));
    }

    let token = &auth_header[7..];
    let claims = state
        .jwt_service
        .validate_access_token(token)
        .map_err(|e| {
            tracing::debug!(error = %e, "Invalid access token");
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "INVALID_TOKEN",
                    "Invalid or expired token",
                )),
            )
        })?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    if !has_super_admin_role(&claims.roles) {
        tracing::warn!(
            user_id = %user_id,
            email = %claims.email,
            roles = ?claims.roles,
            "Unauthorized subscription admin access attempt"
        );
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "INSUFFICIENT_PERMISSIONS",
                "Super Admin role required for subscription management",
            )),
        ));
    }

    Ok(user_id)
}

/// Verify user has access to the organization using RLS connection.
async fn verify_org_access_rls(
    rls: &mut RlsConnection,
    user_id: Uuid,
    org_id: Uuid,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let is_member: Option<(bool,)> = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM organization_members WHERE user_id = $1 AND organization_id = $2)",
    )
    .bind(user_id)
    .bind(org_id)
    .fetch_optional(&mut **rls.conn())
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to check org membership");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DB_ERROR", "Database error")),
        )
    })?;

    match is_member {
        Some((true,)) => Ok(()),
        _ => Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "You do not have access to this organization",
            )),
        )),
    }
}

/// Verify user has access to an invoice by its ID using RLS connection.
async fn verify_invoice_access_rls(
    state: &AppState,
    rls: &mut RlsConnection,
    user_id: Uuid,
    invoice_id: Uuid,
) -> Result<Uuid, (StatusCode, Json<ErrorResponse>)> {
    // Get invoice to find org_id
    let invoice = state
        .subscription_repo
        .find_invoice_by_id_rls(&mut **rls.conn(), invoice_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to find invoice");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Invoice not found")),
            )
        })?;

    // Verify user has access to this organization
    verify_org_access_rls(rls, user_id, invoice.organization_id).await?;

    Ok(invoice.organization_id)
}

/// Verify user has access to a subscription by its ID using RLS connection.
async fn verify_subscription_access_rls(
    state: &AppState,
    rls: &mut RlsConnection,
    user_id: Uuid,
    subscription_id: Uuid,
) -> Result<Uuid, (StatusCode, Json<ErrorResponse>)> {
    // Get subscription to find org_id
    let subscription = state
        .subscription_repo
        .find_subscription_by_id_rls(&mut **rls.conn(), subscription_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to find subscription");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Subscription not found")),
            )
        })?;

    // Verify user has access to this organization
    verify_org_access_rls(rls, user_id, subscription.organization_id).await?;

    Ok(subscription.organization_id)
}

/// Create subscription routes router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Subscription Plans (Admin only)
        .route("/plans", post(create_plan))
        .route("/plans", get(list_plans))
        .route("/plans/public", get(list_public_plans))
        .route("/plans/:id", get(get_plan))
        .route("/plans/:id", patch(update_plan))
        .route("/plans/:id", delete(delete_plan))
        // Organization Subscriptions
        .route("/", post(create_subscription))
        .route("/", get(get_subscription))
        .route("/with-plan", get(get_subscription_with_plan))
        .route("/:id", patch(update_subscription))
        .route("/:id/change-plan", post(change_plan))
        .route("/:id/cancel", post(cancel_subscription))
        .route("/:id/reactivate", post(reactivate_subscription))
        // Payment Methods
        .route("/payment-methods", post(create_payment_method))
        .route("/payment-methods", get(list_payment_methods))
        .route(
            "/payment-methods/:id/default",
            post(set_default_payment_method),
        )
        .route("/payment-methods/:id", delete(delete_payment_method))
        // Invoices
        .route("/invoices", get(list_invoices))
        .route("/invoices/:id", get(get_invoice))
        .route("/invoices/:id/line-items", get(get_invoice_line_items))
        .route("/invoices/:id/pay", post(mark_invoice_paid))
        .route("/invoices/:id/void", post(void_invoice))
        // Usage
        .route("/usage", post(record_usage))
        .route("/usage/summary", get(get_usage_summary))
        .route("/usage/current", get(get_current_usage))
        // Coupons (Admin only)
        .route("/coupons", post(create_coupon))
        .route("/coupons", get(list_coupons))
        .route("/coupons/:id", patch(update_coupon))
        .route("/coupons/redeem", post(redeem_coupon))
        // Statistics (Admin only)
        .route("/statistics", get(get_statistics))
}

/// Create admin routes for platform operators.
pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/subscriptions", get(list_all_subscriptions))
        .route("/invoices", get(list_all_invoices))
}

// ==================== Request/Response Types ====================

/// Organization query parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct OrgQuery {
    pub organization_id: Uuid,
}

/// List plans query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListPlansQuery {
    pub active_only: Option<bool>,
}

/// List subscriptions query (admin).
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListSubscriptionsQuery {
    pub status: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

/// List invoices query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListInvoicesQuery {
    pub organization_id: Uuid,
    pub status: Option<String>,
    pub from_date: Option<NaiveDate>,
    pub to_date: Option<NaiveDate>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl From<&ListInvoicesQuery> for InvoiceQueryParams {
    fn from(q: &ListInvoicesQuery) -> Self {
        InvoiceQueryParams {
            status: q.status.clone(),
            from_date: q.from_date,
            to_date: q.to_date,
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// List all invoices query (admin).
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListAllInvoicesQuery {
    pub status: Option<String>,
    pub from_date: Option<NaiveDate>,
    pub to_date: Option<NaiveDate>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl From<&ListAllInvoicesQuery> for InvoiceQueryParams {
    fn from(q: &ListAllInvoicesQuery) -> Self {
        InvoiceQueryParams {
            status: q.status.clone(),
            from_date: q.from_date,
            to_date: q.to_date,
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Usage summary query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct UsageSummaryQuery {
    pub organization_id: Uuid,
    pub period_start: Option<chrono::DateTime<Utc>>,
    pub period_end: Option<chrono::DateTime<Utc>>,
}

/// List coupons query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListCouponsQuery {
    pub active_only: Option<bool>,
}

/// Current usage response.
#[derive(Debug, Serialize, ToSchema)]
pub struct CurrentUsageResponse {
    pub buildings: i64,
    pub units: i64,
    pub users: i64,
    pub storage_bytes: i64,
}

/// Create subscription request wrapper.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSubscriptionRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateOrganizationSubscription,
}

/// Record usage request wrapper.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RecordUsageRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateUsageRecord,
}

// ==================== Plan Routes ====================

/// Create a subscription plan.
#[utoipa::path(
    post,
    path = "/api/v1/subscriptions/plans",
    request_body = CreateSubscriptionPlan,
    responses(
        (status = 201, description = "Plan created successfully", body = SubscriptionPlan),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - platform admin only"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn create_plan(
    headers: HeaderMap,
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(data): Json<CreateSubscriptionPlan>,
) -> Result<(StatusCode, Json<SubscriptionPlan>), (StatusCode, Json<ErrorResponse>)> {
    // Require super admin role for plan management
    let _admin_id = require_super_admin(&headers, &state)?;

    let plan = state
        .subscription_repo
        .create_plan_rls(&mut **rls.conn(), data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;
    Ok((StatusCode::CREATED, Json(plan)))
}

/// List subscription plans.
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/plans",
    params(ListPlansQuery),
    responses(
        (status = 200, description = "Plans retrieved successfully", body = Vec<SubscriptionPlan>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn list_plans(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Query(query): Query<ListPlansQuery>,
) -> Result<Json<Vec<SubscriptionPlan>>, (StatusCode, Json<ErrorResponse>)> {
    let plans = state
        .subscription_repo
        .list_plans_rls(&mut **rls.conn(), query.active_only.unwrap_or(true))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;
    Ok(Json(plans))
}

/// List public subscription plans.
///
/// This endpoint is intentionally public to allow potential customers to view
/// available pricing plans without authentication. Rate limiting should be
/// applied at the infrastructure level (e.g., API gateway, load balancer)
/// to prevent abuse.
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/plans/public",
    responses(
        (status = 200, description = "Public plans retrieved", body = Vec<SubscriptionPlan>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn list_public_plans(
    State(state): State<AppState>,
) -> Result<Json<Vec<SubscriptionPlan>>, (StatusCode, Json<ErrorResponse>)> {
    // Public endpoint - no RLS needed as subscription_plans is not tenant-scoped
    // Using the legacy method is acceptable here since there's no tenant context
    #[allow(deprecated)]
    let plans = state
        .subscription_repo
        .list_public_plans()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(plans))
}

/// Get a subscription plan by ID.
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/plans/{id}",
    params(("id" = Uuid, Path, description = "Plan ID")),
    responses(
        (status = 200, description = "Plan retrieved", body = SubscriptionPlan),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Plan not found"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn get_plan(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<SubscriptionPlan>, (StatusCode, Json<ErrorResponse>)> {
    let plan = state
        .subscription_repo
        .find_plan_by_id_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Plan not found")),
            )
        })?;

    rls.release().await;
    Ok(Json(plan))
}

/// Update a subscription plan.
#[utoipa::path(
    patch,
    path = "/api/v1/subscriptions/plans/{id}",
    params(("id" = Uuid, Path, description = "Plan ID")),
    request_body = UpdateSubscriptionPlan,
    responses(
        (status = 200, description = "Plan updated", body = SubscriptionPlan),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - platform admin only"),
        (status = 404, description = "Plan not found"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn update_plan(
    headers: HeaderMap,
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateSubscriptionPlan>,
) -> Result<Json<SubscriptionPlan>, (StatusCode, Json<ErrorResponse>)> {
    // Require super admin role for plan management
    let _admin_id = require_super_admin(&headers, &state)?;

    let plan = state
        .subscription_repo
        .update_plan_rls(&mut **rls.conn(), id, data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;
    Ok(Json(plan))
}

/// Delete a subscription plan.
#[utoipa::path(
    delete,
    path = "/api/v1/subscriptions/plans/{id}",
    params(("id" = Uuid, Path, description = "Plan ID")),
    responses(
        (status = 204, description = "Plan deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - platform admin only"),
        (status = 404, description = "Plan not found"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn delete_plan(
    headers: HeaderMap,
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Require super admin role for plan management
    let _admin_id = require_super_admin(&headers, &state)?;

    let deleted = state
        .subscription_repo
        .delete_plan_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Plan not found")),
        ))
    }
}

// ==================== Subscription Routes ====================

/// Create an organization subscription.
#[utoipa::path(
    post,
    path = "/api/v1/subscriptions",
    request_body = CreateSubscriptionRequest,
    responses(
        (status = 201, description = "Subscription created", body = OrganizationSubscription),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn create_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Json(request): Json<CreateSubscriptionRequest>,
) -> Result<(StatusCode, Json<OrganizationSubscription>), (StatusCode, Json<ErrorResponse>)> {
    // Verify user has access to this organization
    verify_org_access_rls(&mut rls, auth.user_id, request.organization_id).await?;

    let subscription = state
        .subscription_repo
        .create_subscription_rls(&mut **rls.conn(), request.organization_id, request.data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;
    Ok((StatusCode::CREATED, Json(subscription)))
}

/// Get organization subscription.
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions",
    params(OrgQuery),
    responses(
        (status = 200, description = "Subscription retrieved", body = OrganizationSubscription),
        (status = 404, description = "No active subscription"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn get_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Query(query): Query<OrgQuery>,
) -> Result<Json<OrganizationSubscription>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user has access to this organization
    verify_org_access_rls(&mut rls, auth.user_id, query.organization_id).await?;

    let subscription = state
        .subscription_repo
        .find_subscription_by_org_rls(&mut **rls.conn(), query.organization_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "No active subscription")),
            )
        })?;

    rls.release().await;
    Ok(Json(subscription))
}

/// Get subscription with plan details.
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/with-plan",
    params(OrgQuery),
    responses(
        (status = 200, description = "Subscription with plan retrieved", body = SubscriptionWithPlan),
        (status = 404, description = "No active subscription"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn get_subscription_with_plan(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Query(query): Query<OrgQuery>,
) -> Result<Json<SubscriptionWithPlan>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user has access to this organization
    verify_org_access_rls(&mut rls, auth.user_id, query.organization_id).await?;

    let subscription = state
        .subscription_repo
        .get_subscription_with_plan_rls(&mut **rls.conn(), query.organization_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "No active subscription")),
            )
        })?;

    rls.release().await;
    Ok(Json(subscription))
}

/// Update an organization subscription.
#[utoipa::path(
    patch,
    path = "/api/v1/subscriptions/{id}",
    params(("id" = Uuid, Path, description = "Subscription ID")),
    request_body = UpdateOrganizationSubscription,
    responses(
        (status = 200, description = "Subscription updated", body = OrganizationSubscription),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn update_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateOrganizationSubscription>,
) -> Result<Json<OrganizationSubscription>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user has access to this subscription's organization
    let _org_id = verify_subscription_access_rls(&state, &mut rls, auth.user_id, id).await?;

    let subscription = state
        .subscription_repo
        .update_subscription_rls(&mut **rls.conn(), id, data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;
    Ok(Json(subscription))
}

/// Change subscription plan.
#[utoipa::path(
    post,
    path = "/api/v1/subscriptions/{id}/change-plan",
    params(("id" = Uuid, Path, description = "Subscription ID")),
    request_body = ChangePlanRequest,
    responses(
        (status = 200, description = "Plan changed", body = OrganizationSubscription),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn change_plan(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<ChangePlanRequest>,
) -> Result<Json<OrganizationSubscription>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user has access to this subscription's organization
    let _org_id = verify_subscription_access_rls(&state, &mut rls, auth.user_id, id).await?;

    let subscription = state
        .subscription_repo
        .change_plan_rls(&mut **rls.conn(), id, data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;
    Ok(Json(subscription))
}

/// Cancel a subscription.
#[utoipa::path(
    post,
    path = "/api/v1/subscriptions/{id}/cancel",
    params(("id" = Uuid, Path, description = "Subscription ID")),
    request_body = CancelSubscriptionRequest,
    responses(
        (status = 200, description = "Subscription cancelled", body = OrganizationSubscription),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn cancel_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<CancelSubscriptionRequest>,
) -> Result<Json<OrganizationSubscription>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user has access to this subscription's organization
    let _org_id = verify_subscription_access_rls(&state, &mut rls, auth.user_id, id).await?;

    let subscription = state
        .subscription_repo
        .cancel_subscription_rls(&mut **rls.conn(), id, data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;
    Ok(Json(subscription))
}

/// Reactivate a cancelled subscription.
#[utoipa::path(
    post,
    path = "/api/v1/subscriptions/{id}/reactivate",
    params(("id" = Uuid, Path, description = "Subscription ID")),
    responses(
        (status = 200, description = "Subscription reactivated", body = OrganizationSubscription),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn reactivate_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<OrganizationSubscription>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user has access to this subscription's organization
    let _org_id = verify_subscription_access_rls(&state, &mut rls, auth.user_id, id).await?;

    let subscription = state
        .subscription_repo
        .reactivate_subscription_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;
    Ok(Json(subscription))
}

/// List all subscriptions (admin).
#[utoipa::path(
    get,
    path = "/api/v1/admin/subscriptions",
    params(ListSubscriptionsQuery),
    responses(
        (status = 200, description = "Subscriptions retrieved", body = Vec<SubscriptionWithPlan>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - platform admin only"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions Admin"
)]
async fn list_all_subscriptions(
    headers: HeaderMap,
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListSubscriptionsQuery>,
) -> Result<Json<Vec<SubscriptionWithPlan>>, (StatusCode, Json<ErrorResponse>)> {
    // Require super admin role for admin dashboard
    let _admin_id = require_super_admin(&headers, &state)?;

    let subscriptions = state
        .subscription_repo
        .list_all_subscriptions_rls(
            &mut **rls.conn(),
            query.status.as_deref(),
            query.limit.unwrap_or(50),
            query.offset.unwrap_or(0),
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;
    Ok(Json(subscriptions))
}

// ==================== Payment Method Routes ====================

/// Create a payment method.
#[utoipa::path(
    post,
    path = "/api/v1/subscriptions/payment-methods",
    request_body = CreateSubscriptionPaymentMethod,
    responses(
        (status = 201, description = "Payment method created", body = SubscriptionPaymentMethod),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn create_payment_method(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Query(query): Query<OrgQuery>,
    Json(data): Json<CreateSubscriptionPaymentMethod>,
) -> Result<(StatusCode, Json<SubscriptionPaymentMethod>), (StatusCode, Json<ErrorResponse>)> {
    // Verify user has access to this organization
    verify_org_access_rls(&mut rls, auth.user_id, query.organization_id).await?;

    let method = state
        .subscription_repo
        .create_payment_method_rls(&mut **rls.conn(), query.organization_id, data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;
    Ok((StatusCode::CREATED, Json(method)))
}

/// List payment methods.
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/payment-methods",
    params(OrgQuery),
    responses(
        (status = 200, description = "Payment methods retrieved", body = Vec<SubscriptionPaymentMethod>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn list_payment_methods(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Query(query): Query<OrgQuery>,
) -> Result<Json<Vec<SubscriptionPaymentMethod>>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user has access to this organization
    verify_org_access_rls(&mut rls, auth.user_id, query.organization_id).await?;

    let methods = state
        .subscription_repo
        .list_payment_methods_rls(&mut **rls.conn(), query.organization_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;
    Ok(Json(methods))
}

/// Set default payment method.
#[utoipa::path(
    post,
    path = "/api/v1/subscriptions/payment-methods/{id}/default",
    params(
        ("id" = Uuid, Path, description = "Payment method ID"),
        OrgQuery
    ),
    responses(
        (status = 204, description = "Default payment method set"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn set_default_payment_method(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Verify user has access to this organization
    verify_org_access_rls(&mut rls, auth.user_id, query.organization_id).await?;

    // Release RLS connection before calling transaction-based method
    rls.release().await;

    // Note: set_default_payment_method uses internal transaction, not RLS-aware
    // This is intentional as documented in the repository
    state
        .subscription_repo
        .set_default_payment_method(query.organization_id, id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Delete a payment method.
#[utoipa::path(
    delete,
    path = "/api/v1/subscriptions/payment-methods/{id}",
    params(
        ("id" = Uuid, Path, description = "Payment method ID"),
        OrgQuery
    ),
    responses(
        (status = 204, description = "Payment method deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Payment method not found"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn delete_payment_method(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Verify user has access to this organization
    verify_org_access_rls(&mut rls, auth.user_id, query.organization_id).await?;

    let deleted = state
        .subscription_repo
        .delete_payment_method_rls(&mut **rls.conn(), id, query.organization_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Payment method not found")),
        ))
    }
}

// ==================== Invoice Routes ====================

/// List invoices.
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/invoices",
    params(ListInvoicesQuery),
    responses(
        (status = 200, description = "Invoices retrieved", body = Vec<SubscriptionInvoice>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn list_invoices(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Query(query): Query<ListInvoicesQuery>,
) -> Result<Json<Vec<SubscriptionInvoice>>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user has access to this organization
    verify_org_access_rls(&mut rls, auth.user_id, query.organization_id).await?;

    let invoices = state
        .subscription_repo
        .list_invoices_rls(
            &mut **rls.conn(),
            query.organization_id,
            InvoiceQueryParams::from(&query),
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;
    Ok(Json(invoices))
}

/// Get an invoice by ID.
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/invoices/{id}",
    params(("id" = Uuid, Path, description = "Invoice ID")),
    responses(
        (status = 200, description = "Invoice retrieved", body = SubscriptionInvoice),
        (status = 404, description = "Invoice not found"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn get_invoice(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<SubscriptionInvoice>, (StatusCode, Json<ErrorResponse>)> {
    // Verify access and get the invoice
    let _org_id = verify_invoice_access_rls(&state, &mut rls, auth.user_id, id).await?;

    let invoice = state
        .subscription_repo
        .find_invoice_by_id_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Invoice not found")),
            )
        })?;

    rls.release().await;
    Ok(Json(invoice))
}

/// Get invoice line items.
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/invoices/{id}/line-items",
    params(("id" = Uuid, Path, description = "Invoice ID")),
    responses(
        (status = 200, description = "Line items retrieved", body = Vec<InvoiceLineItem>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn get_invoice_line_items(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<InvoiceLineItem>>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user has access to this invoice's organization
    let _org_id = verify_invoice_access_rls(&state, &mut rls, auth.user_id, id).await?;

    let items = state
        .subscription_repo
        .get_invoice_line_items_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;
    Ok(Json(items))
}

/// Mark invoice as paid.
#[utoipa::path(
    post,
    path = "/api/v1/subscriptions/invoices/{id}/pay",
    params(("id" = Uuid, Path, description = "Invoice ID")),
    responses(
        (status = 200, description = "Invoice marked as paid", body = SubscriptionInvoice),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn mark_invoice_paid(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<SubscriptionInvoice>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user has access to this invoice's organization
    let _org_id = verify_invoice_access_rls(&state, &mut rls, auth.user_id, id).await?;

    let invoice = state
        .subscription_repo
        .mark_invoice_paid_rls(&mut **rls.conn(), id, None)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;
    Ok(Json(invoice))
}

/// Void an invoice.
#[utoipa::path(
    post,
    path = "/api/v1/subscriptions/invoices/{id}/void",
    params(("id" = Uuid, Path, description = "Invoice ID")),
    responses(
        (status = 200, description = "Invoice voided", body = SubscriptionInvoice),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn void_invoice(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<SubscriptionInvoice>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user has access to this invoice's organization
    let _org_id = verify_invoice_access_rls(&state, &mut rls, auth.user_id, id).await?;

    let invoice = state
        .subscription_repo
        .void_invoice_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;
    Ok(Json(invoice))
}

/// List all invoices (admin).
#[utoipa::path(
    get,
    path = "/api/v1/admin/invoices",
    params(ListAllInvoicesQuery),
    responses(
        (status = 200, description = "Invoices retrieved", body = Vec<InvoiceWithDetails>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - platform admin only"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions Admin"
)]
async fn list_all_invoices(
    headers: HeaderMap,
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListAllInvoicesQuery>,
) -> Result<Json<Vec<InvoiceWithDetails>>, (StatusCode, Json<ErrorResponse>)> {
    // Require super admin role for admin dashboard
    let _admin_id = require_super_admin(&headers, &state)?;

    let invoices = state
        .subscription_repo
        .list_all_invoices_rls(&mut **rls.conn(), InvoiceQueryParams::from(&query))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;
    Ok(Json(invoices))
}

// ==================== Usage Routes ====================

/// Record usage.
#[utoipa::path(
    post,
    path = "/api/v1/subscriptions/usage",
    request_body = RecordUsageRequest,
    responses(
        (status = 201, description = "Usage recorded", body = db::models::UsageRecord),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn record_usage(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Json(request): Json<RecordUsageRequest>,
) -> Result<(StatusCode, Json<db::models::UsageRecord>), (StatusCode, Json<ErrorResponse>)> {
    // Verify user has access to this organization
    verify_org_access_rls(&mut rls, auth.user_id, request.organization_id).await?;

    // Get subscription for org
    let subscription = state
        .subscription_repo
        .find_subscription_by_org_rls(&mut **rls.conn(), request.organization_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    let record = state
        .subscription_repo
        .record_usage_rls(
            &mut **rls.conn(),
            request.organization_id,
            subscription.map(|s| s.id),
            request.data,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;
    Ok((StatusCode::CREATED, Json(record)))
}

/// Get usage summary.
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/usage/summary",
    params(UsageSummaryQuery),
    responses(
        (status = 200, description = "Usage summary retrieved", body = Vec<UsageSummary>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn get_usage_summary(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Query(query): Query<UsageSummaryQuery>,
) -> Result<Json<Vec<UsageSummary>>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user has access to this organization
    verify_org_access_rls(&mut rls, auth.user_id, query.organization_id).await?;

    let now = Utc::now();
    let period_start = query.period_start.unwrap_or(
        now.checked_sub_signed(chrono::Duration::days(30))
            .unwrap_or(now),
    );
    let period_end = query.period_end.unwrap_or(now);

    let summary = state
        .subscription_repo
        .get_usage_summary_rls(
            &mut **rls.conn(),
            query.organization_id,
            period_start,
            period_end,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;
    Ok(Json(summary))
}

/// Get current usage.
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/usage/current",
    params(OrgQuery),
    responses(
        (status = 200, description = "Current usage retrieved", body = CurrentUsageResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn get_current_usage(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Query(query): Query<OrgQuery>,
) -> Result<Json<CurrentUsageResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user has access to this organization
    verify_org_access_rls(&mut rls, auth.user_id, query.organization_id).await?;

    let (buildings, units, users, storage) = state
        .subscription_repo
        .get_current_usage_rls(&mut **rls.conn(), query.organization_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;
    Ok(Json(CurrentUsageResponse {
        buildings,
        units,
        users,
        storage_bytes: storage,
    }))
}

// ==================== Coupon Routes ====================

/// Create a coupon.
#[utoipa::path(
    post,
    path = "/api/v1/subscriptions/coupons",
    request_body = CreateSubscriptionCoupon,
    responses(
        (status = 201, description = "Coupon created", body = SubscriptionCoupon),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - platform admin only"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn create_coupon(
    headers: HeaderMap,
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(data): Json<CreateSubscriptionCoupon>,
) -> Result<(StatusCode, Json<SubscriptionCoupon>), (StatusCode, Json<ErrorResponse>)> {
    // Require super admin role for coupon management
    let _admin_id = require_super_admin(&headers, &state)?;

    let coupon = state
        .subscription_repo
        .create_coupon_rls(&mut **rls.conn(), data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;
    Ok((StatusCode::CREATED, Json(coupon)))
}

/// List coupons.
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/coupons",
    params(ListCouponsQuery),
    responses(
        (status = 200, description = "Coupons retrieved", body = Vec<SubscriptionCoupon>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn list_coupons(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Query(query): Query<ListCouponsQuery>,
) -> Result<Json<Vec<SubscriptionCoupon>>, (StatusCode, Json<ErrorResponse>)> {
    let coupons = state
        .subscription_repo
        .list_coupons_rls(&mut **rls.conn(), query.active_only.unwrap_or(true))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;
    Ok(Json(coupons))
}

/// Update a coupon.
#[utoipa::path(
    patch,
    path = "/api/v1/subscriptions/coupons/{id}",
    params(("id" = Uuid, Path, description = "Coupon ID")),
    request_body = UpdateSubscriptionCoupon,
    responses(
        (status = 200, description = "Coupon updated", body = SubscriptionCoupon),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - platform admin only"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn update_coupon(
    headers: HeaderMap,
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateSubscriptionCoupon>,
) -> Result<Json<SubscriptionCoupon>, (StatusCode, Json<ErrorResponse>)> {
    // Require super admin role for coupon management
    let _admin_id = require_super_admin(&headers, &state)?;

    let coupon = state
        .subscription_repo
        .update_coupon_rls(&mut **rls.conn(), id, data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;
    Ok(Json(coupon))
}

/// Redeem a coupon.
#[utoipa::path(
    post,
    path = "/api/v1/subscriptions/coupons/redeem",
    request_body = RedeemCouponRequest,
    responses(
        (status = 200, description = "Coupon redeemed", body = CouponRedemption),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Coupon not found"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn redeem_coupon(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Query(query): Query<OrgQuery>,
    Json(data): Json<RedeemCouponRequest>,
) -> Result<Json<CouponRedemption>, (StatusCode, Json<ErrorResponse>)> {
    // Verify user has access to this organization
    verify_org_access_rls(&mut rls, auth.user_id, query.organization_id).await?;

    // Find the coupon
    let coupon = state
        .subscription_repo
        .find_coupon_by_code_rls(&mut **rls.conn(), &data.coupon_code)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Coupon not found or inactive",
                )),
            )
        })?;

    // Get subscription for org
    let subscription = state
        .subscription_repo
        .find_subscription_by_org_rls(&mut **rls.conn(), query.organization_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    // Release RLS connection before calling transaction-based method
    rls.release().await;

    // Note: redeem_coupon uses internal transaction, not RLS-aware
    // This is intentional as documented in the repository
    let redemption = state
        .subscription_repo
        .redeem_coupon(
            coupon.id,
            query.organization_id,
            subscription.map(|s| s.id),
            auth.user_id,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(redemption))
}

// ==================== Statistics Routes ====================

/// Get subscription statistics.
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/statistics",
    responses(
        (status = 200, description = "Statistics retrieved", body = SubscriptionStatistics),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - platform admin only"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn get_statistics(
    headers: HeaderMap,
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<SubscriptionStatistics>, (StatusCode, Json<ErrorResponse>)> {
    // Require super admin role for statistics dashboard
    let _admin_id = require_super_admin(&headers, &state)?;

    let stats = state
        .subscription_repo
        .get_statistics_rls(&mut **rls.conn())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;
    Ok(Json(stats))
}
