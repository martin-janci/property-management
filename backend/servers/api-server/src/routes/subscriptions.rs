//! Subscription and billing routes (Epic 26).
//!
//! # RLS (PAP-112 / PAP-80)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` on the org-scoped billing
//! tables, so every query MUST run on a connection that has
//! `app.current_org_id` set or it collapses to deny-all. Each handler
//! acquires an [`RlsConnection`] (which validates tenant membership and sets
//! the org/user GUCs on a dedicated connection) and passes
//! `&mut **rls.conn()` to the repository. The authoritative organization is
//! `rls.tenant_id()` — the tenant the caller was validated against — so
//! request bodies/queries that carry an `organization_id` are checked against
//! it (`403` on mismatch) instead of re-querying membership. Cross-tenant
//! access is blocked by RLS: a by-id read of another org's row returns no row
//! (`404`), and a write targeting another org fails the policy `WITH CHECK`.
//! `rls.release()` clears the context before the connection returns to the
//! pool.
//!
//! The public `/plans/public` endpoint has no tenant principal; it reads
//! `subscription_plans` (not FORCE-bound, public read policy) on the plain
//! pool.

use api_core::extractors::RlsConnection;
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

use crate::routes::pagination::clamp_limit;
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

/// Require that a request-supplied organization id matches the tenant the
/// caller was validated against.
///
/// `RlsConnection` already validates org membership via
/// `ValidatedTenantExtractor`, so no DB round-trip is needed — only the
/// equality check. Releases the connection on mismatch.
async fn ensure_org_matches(
    rls: &mut RlsConnection,
    organization_id: Uuid,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if organization_id != rls.tenant_id() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "You do not have access to this organization",
            )),
        ));
    }
    Ok(())
}

/// Map a repository error: under RLS a cross-tenant (or missing) row surfaces
/// as `RowNotFound` on mutation paths — translate that to `404` instead of
/// `500`.
fn db_error(e: sqlx::Error) -> (StatusCode, Json<ErrorResponse>) {
    match e {
        sqlx::Error::RowNotFound => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Resource not found")),
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DB_ERROR", e.to_string())),
        ),
    }
}

/// Create subscription routes router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Subscription Plans (Admin only)
        .route("/plans", post(create_plan))
        .route("/plans", get(list_plans))
        .route("/plans/public", get(list_public_plans))
        .route("/plans/{id}", get(get_plan))
        .route("/plans/{id}", patch(update_plan))
        .route("/plans/{id}", delete(delete_plan))
        // Organization Subscriptions
        .route("/", post(create_subscription))
        .route("/", get(get_subscription))
        .route("/with-plan", get(get_subscription_with_plan))
        .route("/{id}", patch(update_subscription))
        .route("/{id}/change-plan", post(change_plan))
        .route("/{id}/cancel", post(cancel_subscription))
        .route("/{id}/reactivate", post(reactivate_subscription))
        // Payment Methods
        .route("/payment-methods", post(create_payment_method))
        .route("/payment-methods", get(list_payment_methods))
        .route(
            "/payment-methods/{id}/default",
            post(set_default_payment_method),
        )
        .route("/payment-methods/{id}", delete(delete_payment_method))
        // Invoices
        .route("/invoices", get(list_invoices))
        .route("/invoices/{id}", get(get_invoice))
        .route("/invoices/{id}/line-items", get(get_invoice_line_items))
        .route("/invoices/{id}/pay", post(mark_invoice_paid))
        .route("/invoices/{id}/void", post(void_invoice))
        // Usage
        .route("/usage", post(record_usage))
        .route("/usage/summary", get(get_usage_summary))
        .route("/usage/current", get(get_current_usage))
        // Coupons (Admin only)
        .route("/coupons", post(create_coupon))
        .route("/coupons", get(list_coupons))
        .route("/coupons/{id}", patch(update_coupon))
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
        .create_plan(&mut **rls.conn(), data)
        .await
        .map_err(db_error)?;

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
    mut rls: RlsConnection,
    Query(query): Query<ListPlansQuery>,
) -> Result<Json<Vec<SubscriptionPlan>>, (StatusCode, Json<ErrorResponse>)> {
    let plans = state
        .subscription_repo
        .list_plans(&mut **rls.conn(), query.active_only.unwrap_or(true))
        .await
        .map_err(db_error)?;

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
    // No tenant principal here: subscription_plans is not FORCE-bound and
    // carries a public read policy. PAP-150: take a sanctioned public
    // connection (clears any stale RLS context from a prior request) instead
    // of touching the raw pool directly.
    let mut conn = db::RlsPool::new(state.db.clone())
        .acquire_public()
        .await
        .map_err(db_error)?;
    let plans = state
        .subscription_repo
        .list_public_plans(&mut **conn)
        .await
        .map_err(db_error)?;

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
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<SubscriptionPlan>, (StatusCode, Json<ErrorResponse>)> {
    let plan = state
        .subscription_repo
        .find_plan_by_id(&mut **rls.conn(), id)
        .await
        .map_err(db_error)?;

    rls.release().await;

    let plan = plan.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Plan not found")),
        )
    })?;

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
        .update_plan(&mut **rls.conn(), id, data)
        .await
        .map_err(db_error)?;

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
        .delete_plan(&mut **rls.conn(), id)
        .await
        .map_err(db_error)?;

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
        (status = 403, description = "Forbidden - organization mismatch"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn create_subscription(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(request): Json<CreateSubscriptionRequest>,
) -> Result<(StatusCode, Json<OrganizationSubscription>), (StatusCode, Json<ErrorResponse>)> {
    ensure_org_matches(&mut rls, request.organization_id).await?;
    let org_id = rls.tenant_id();

    let subscription = state
        .subscription_repo
        .create_subscription(rls.conn(), org_id, request.data)
        .await
        .map_err(db_error)?;

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
    mut rls: RlsConnection,
    Query(query): Query<OrgQuery>,
) -> Result<Json<OrganizationSubscription>, (StatusCode, Json<ErrorResponse>)> {
    ensure_org_matches(&mut rls, query.organization_id).await?;
    let org_id = rls.tenant_id();

    let subscription = state
        .subscription_repo
        .find_subscription_by_org(&mut **rls.conn(), org_id)
        .await
        .map_err(db_error)?;

    rls.release().await;

    let subscription = subscription.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "No active subscription")),
        )
    })?;

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
    mut rls: RlsConnection,
    Query(query): Query<OrgQuery>,
) -> Result<Json<SubscriptionWithPlan>, (StatusCode, Json<ErrorResponse>)> {
    ensure_org_matches(&mut rls, query.organization_id).await?;
    let org_id = rls.tenant_id();

    let subscription = state
        .subscription_repo
        .get_subscription_with_plan(&mut **rls.conn(), org_id)
        .await
        .map_err(db_error)?;

    rls.release().await;

    let subscription = subscription.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "No active subscription")),
        )
    })?;

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
        (status = 404, description = "Subscription not found"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn update_subscription(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateOrganizationSubscription>,
) -> Result<Json<OrganizationSubscription>, (StatusCode, Json<ErrorResponse>)> {
    // RLS scopes the by-id update to the caller's org: a cross-tenant id
    // matches no row and surfaces as 404.
    let subscription = state
        .subscription_repo
        .update_subscription(&mut **rls.conn(), id, data)
        .await
        .map_err(db_error)?;

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
        (status = 404, description = "Subscription not found"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn change_plan(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<ChangePlanRequest>,
) -> Result<Json<OrganizationSubscription>, (StatusCode, Json<ErrorResponse>)> {
    let subscription = state
        .subscription_repo
        .change_plan(&mut **rls.conn(), id, data)
        .await
        .map_err(db_error)?;

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
        (status = 404, description = "Subscription not found"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn cancel_subscription(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<CancelSubscriptionRequest>,
) -> Result<Json<OrganizationSubscription>, (StatusCode, Json<ErrorResponse>)> {
    let subscription = state
        .subscription_repo
        .cancel_subscription(&mut **rls.conn(), id, data)
        .await
        .map_err(db_error)?;

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
        (status = 404, description = "Subscription not found"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn reactivate_subscription(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<OrganizationSubscription>, (StatusCode, Json<ErrorResponse>)> {
    let subscription = state
        .subscription_repo
        .reactivate_subscription(&mut **rls.conn(), id)
        .await
        .map_err(db_error)?;

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
        .list_all_subscriptions(
            &mut **rls.conn(),
            query.status.as_deref(),
            clamp_limit(query.limit.map(i64::from), 50) as i32,
            query.offset.unwrap_or(0),
        )
        .await
        .map_err(db_error)?;

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
        (status = 403, description = "Forbidden - organization mismatch"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn create_payment_method(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<OrgQuery>,
    Json(data): Json<CreateSubscriptionPaymentMethod>,
) -> Result<(StatusCode, Json<SubscriptionPaymentMethod>), (StatusCode, Json<ErrorResponse>)> {
    ensure_org_matches(&mut rls, query.organization_id).await?;
    let org_id = rls.tenant_id();

    let method = state
        .subscription_repo
        .create_payment_method(&mut **rls.conn(), org_id, data)
        .await
        .map_err(db_error)?;

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
    mut rls: RlsConnection,
    Query(query): Query<OrgQuery>,
) -> Result<Json<Vec<SubscriptionPaymentMethod>>, (StatusCode, Json<ErrorResponse>)> {
    ensure_org_matches(&mut rls, query.organization_id).await?;
    let org_id = rls.tenant_id();

    let methods = state
        .subscription_repo
        .list_payment_methods(&mut **rls.conn(), org_id)
        .await
        .map_err(db_error)?;

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
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    ensure_org_matches(&mut rls, query.organization_id).await?;
    let org_id = rls.tenant_id();

    // Transactional, but runs on the RLS-context connection: payment_methods
    // is FORCE-RLS-bound, so the raw pool would be deny-all here.
    state
        .subscription_repo
        .set_default_payment_method(rls.conn(), org_id, id)
        .await
        .map_err(db_error)?;

    rls.release().await;
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
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<OrgQuery>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    ensure_org_matches(&mut rls, query.organization_id).await?;
    let org_id = rls.tenant_id();

    let deleted = state
        .subscription_repo
        .delete_payment_method(&mut **rls.conn(), id, org_id)
        .await
        .map_err(db_error)?;

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
    mut rls: RlsConnection,
    Query(query): Query<ListInvoicesQuery>,
) -> Result<Json<Vec<SubscriptionInvoice>>, (StatusCode, Json<ErrorResponse>)> {
    ensure_org_matches(&mut rls, query.organization_id).await?;
    let org_id = rls.tenant_id();

    let invoices = state
        .subscription_repo
        .list_invoices(&mut **rls.conn(), org_id, InvoiceQueryParams::from(&query))
        .await
        .map_err(db_error)?;

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
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<SubscriptionInvoice>, (StatusCode, Json<ErrorResponse>)> {
    // RLS scopes the by-id read to the caller's org: a cross-tenant invoice
    // is indistinguishable from a missing one (404).
    let invoice = state
        .subscription_repo
        .find_invoice_by_id(&mut **rls.conn(), id)
        .await
        .map_err(db_error)?;

    rls.release().await;

    let invoice = invoice.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Invoice not found")),
        )
    })?;

    Ok(Json(invoice))
}

/// Get invoice line items.
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/invoices/{id}/line-items",
    params(("id" = Uuid, Path, description = "Invoice ID")),
    responses(
        (status = 200, description = "Line items retrieved", body = Vec<InvoiceLineItem>),
        (status = 404, description = "Invoice not found"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn get_invoice_line_items(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<InvoiceLineItem>>, (StatusCode, Json<ErrorResponse>)> {
    // Resolve the invoice first so a cross-tenant (RLS-invisible) id keeps
    // returning 404 rather than an empty list.
    let invoice = state
        .subscription_repo
        .find_invoice_by_id(&mut **rls.conn(), id)
        .await
        .map_err(db_error)?;

    if invoice.is_none() {
        rls.release().await;
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Invoice not found")),
        ));
    }

    let items = state
        .subscription_repo
        .get_invoice_line_items(&mut **rls.conn(), id)
        .await
        .map_err(db_error)?;

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
        (status = 404, description = "Invoice not found"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn mark_invoice_paid(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<SubscriptionInvoice>, (StatusCode, Json<ErrorResponse>)> {
    let invoice = state
        .subscription_repo
        .mark_invoice_paid(&mut **rls.conn(), id, None)
        .await
        .map_err(db_error)?;

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
        (status = 404, description = "Invoice not found"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn void_invoice(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<SubscriptionInvoice>, (StatusCode, Json<ErrorResponse>)> {
    let invoice = state
        .subscription_repo
        .void_invoice(&mut **rls.conn(), id)
        .await
        .map_err(db_error)?;

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
        .list_all_invoices(&mut **rls.conn(), InvoiceQueryParams::from(&query))
        .await
        .map_err(db_error)?;

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
        (status = 403, description = "Forbidden - organization mismatch"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Subscriptions"
)]
async fn record_usage(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(request): Json<RecordUsageRequest>,
) -> Result<(StatusCode, Json<db::models::UsageRecord>), (StatusCode, Json<ErrorResponse>)> {
    ensure_org_matches(&mut rls, request.organization_id).await?;
    let org_id = rls.tenant_id();

    // Get subscription for org
    let subscription = state
        .subscription_repo
        .find_subscription_by_org(&mut **rls.conn(), org_id)
        .await
        .map_err(db_error)?;

    let record = state
        .subscription_repo
        .record_usage(
            &mut **rls.conn(),
            org_id,
            subscription.map(|s| s.id),
            request.data,
        )
        .await
        .map_err(db_error)?;

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
    mut rls: RlsConnection,
    Query(query): Query<UsageSummaryQuery>,
) -> Result<Json<Vec<UsageSummary>>, (StatusCode, Json<ErrorResponse>)> {
    ensure_org_matches(&mut rls, query.organization_id).await?;
    let org_id = rls.tenant_id();

    let now = Utc::now();
    let period_start = query.period_start.unwrap_or(
        now.checked_sub_signed(chrono::Duration::days(30))
            .unwrap_or(now),
    );
    let period_end = query.period_end.unwrap_or(now);

    let summary = state
        .subscription_repo
        .get_usage_summary(&mut **rls.conn(), org_id, period_start, period_end)
        .await
        .map_err(db_error)?;

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
    mut rls: RlsConnection,
    Query(query): Query<OrgQuery>,
) -> Result<Json<CurrentUsageResponse>, (StatusCode, Json<ErrorResponse>)> {
    ensure_org_matches(&mut rls, query.organization_id).await?;
    let org_id = rls.tenant_id();

    let (buildings, units, users, storage) = state
        .subscription_repo
        .get_current_usage(&mut **rls.conn(), org_id)
        .await
        .map_err(db_error)?;

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
        .create_coupon(&mut **rls.conn(), data)
        .await
        .map_err(db_error)?;

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
    mut rls: RlsConnection,
    Query(query): Query<ListCouponsQuery>,
) -> Result<Json<Vec<SubscriptionCoupon>>, (StatusCode, Json<ErrorResponse>)> {
    let coupons = state
        .subscription_repo
        .list_coupons(&mut **rls.conn(), query.active_only.unwrap_or(true))
        .await
        .map_err(db_error)?;

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
        (status = 404, description = "Coupon not found"),
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
        .update_coupon(&mut **rls.conn(), id, data)
        .await
        .map_err(db_error)?;

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
    mut rls: RlsConnection,
    Query(query): Query<OrgQuery>,
    Json(data): Json<RedeemCouponRequest>,
) -> Result<Json<CouponRedemption>, (StatusCode, Json<ErrorResponse>)> {
    ensure_org_matches(&mut rls, query.organization_id).await?;
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();

    // Find the coupon
    let coupon = state
        .subscription_repo
        .find_coupon_by_code(&mut **rls.conn(), &data.coupon_code)
        .await
        .map_err(db_error)?
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
        .find_subscription_by_org(&mut **rls.conn(), org_id)
        .await
        .map_err(db_error)?;

    // Transactional, but runs on the RLS-context connection: the
    // coupon_redemptions insert is FORCE-RLS-bound, so the raw pool would
    // fail the policy WITH CHECK.
    let redemption = state
        .subscription_repo
        .redeem_coupon(
            rls.conn(),
            coupon.id,
            org_id,
            subscription.map(|s| s.id),
            user_id,
        )
        .await
        .map_err(db_error)?;

    rls.release().await;
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
        .get_statistics(rls.conn())
        .await
        .map_err(db_error)?;

    rls.release().await;
    Ok(Json(stats))
}
