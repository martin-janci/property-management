//! Lease routes (Epic 19: Lease Management & Tenant Screening).
//!
//! Implements tenant application intake, screening, lease creation,
//! lifecycle management, and expiration tracking.

use api_core::extractors::{AuthUser, RlsConnection};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, patch, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::lease::RecordPayment;
use db::models::{
    ApplicationListQuery, CreateAmendment, CreateApplication, CreateLease, CreateLeaseTemplate,
    CreateReminder, InitiateScreening, LeaseListQuery, RenewLease, ReviewApplication,
    ScreeningConsent, SubmitApplication, TerminateLease, UpdateApplication, UpdateLease,
    UpdateLeaseTemplate, UpdateScreeningResult,
};
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::state::AppState;

/// Create lease router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Tenant Applications (Story 19.1)
        .route("/applications", post(create_application))
        .route("/applications", get(list_applications))
        .route("/applications/{id}", get(get_application))
        .route("/applications/{id}", put(update_application))
        .route("/applications/{id}/submit", post(submit_application))
        .route("/applications/{id}/review", post(review_application))
        // Tenant Screening (Story 19.2)
        .route("/applications/{id}/screening", post(initiate_screening))
        .route("/applications/{id}/screening/consent", post(record_consent))
        .route("/screenings/{id}", get(get_screening))
        .route("/screenings/{id}/result", patch(update_screening_result))
        // Lease Templates (Story 19.3)
        .route("/templates", post(create_template))
        .route("/templates", get(list_templates))
        .route("/templates/{id}", get(get_template))
        .route("/templates/{id}", put(update_template))
        // Leases (Story 19.3, 19.4)
        .route("/", post(create_lease))
        .route("/", get(list_leases))
        .route("/{id}", get(get_lease))
        .route("/{id}", put(update_lease))
        .route("/{id}/terminate", post(terminate_lease))
        .route("/{id}/renew", post(renew_lease))
        // Lease Amendments
        .route("/{id}/amendments", post(create_amendment))
        .route("/{id}/amendments", get(list_amendments))
        // Lease Payments
        .route("/{id}/payments", post(record_payment))
        .route("/{id}/payments", get(list_payments))
        .route("/{id}/payments/summary", get(get_payment_summary))
        // Lease Reminders (Story 19.5)
        .route("/{id}/reminders", post(create_reminder))
        .route("/{id}/reminders", get(list_reminders))
        // Dashboard/Statistics
        .route("/expiring", get(get_expiring_leases))
        .route("/statistics", get(get_statistics))
}

// ==================== Request/Response Types ====================

/// Organization query parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct OrgQuery {
    /// Organization ID
    pub organization_id: Uuid,
}

/// Create application with org context.
#[derive(Debug, Deserialize)]
pub struct CreateApplicationRequest {
    /// Organization ID
    pub organization_id: Uuid,
    /// Application data
    #[serde(flatten)]
    pub data: CreateApplication,
}

/// Create lease with org context.
#[derive(Debug, Deserialize)]
pub struct CreateLeaseRequest {
    /// Organization ID
    pub organization_id: Uuid,
    /// Lease data
    #[serde(flatten)]
    pub data: CreateLease,
}

/// Create template with org context.
#[derive(Debug, Deserialize)]
pub struct CreateTemplateRequest {
    /// Organization ID
    pub organization_id: Uuid,
    /// Template data
    #[serde(flatten)]
    pub data: CreateLeaseTemplate,
}

/// Expiring leases query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ExpiringQuery {
    /// Organization ID
    pub organization_id: Uuid,
    /// Days ahead to check (default: 90)
    #[serde(default = "default_days")]
    pub days: i32,
}

fn default_days() -> i32 {
    90
}

// ==================== Application Handlers (Story 19.1) ====================

async fn create_application(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(payload): Json<CreateApplicationRequest>,
) -> Result<(StatusCode, Json<db::models::TenantApplication>), (StatusCode, Json<ErrorResponse>)> {
    // Verify request org_id matches authenticated tenant
    if payload.organization_id != rls.tenant_id() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "Organization ID mismatch",
            )),
        ));
    }

    let result = state
        .lease_repo
        .create_application_rls(&mut **rls.conn(), payload.organization_id, payload.data)
        .await
        .map(|app| (StatusCode::CREATED, Json(app)))
        .map_err(|e| {
            tracing::error!("Failed to create application: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to create application",
                )),
            )
        });

    rls.release().await;
    result
}

/// List applications query with org.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListApplicationsQuery {
    /// Organization ID
    pub organization_id: Uuid,
    /// Filter by unit
    pub unit_id: Option<Uuid>,
    /// Filter by status
    pub status: Option<String>,
    /// Page limit
    pub limit: Option<i32>,
    /// Page offset
    pub offset: Option<i32>,
}

/// List applications response with pagination.
#[derive(Debug, serde::Serialize)]
pub struct ListApplicationsResponse {
    /// Applications
    pub items: Vec<db::models::ApplicationSummary>,
    /// Total count
    pub total: i64,
}

async fn list_applications(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListApplicationsQuery>,
) -> Result<Json<ListApplicationsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify request org_id matches authenticated tenant
    if query.organization_id != rls.tenant_id() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "Organization ID mismatch",
            )),
        ));
    }

    let org_id = query.organization_id;
    let app_query = ApplicationListQuery {
        unit_id: query.unit_id,
        status: query.status,
        limit: query.limit,
        offset: query.offset,
    };

    let result = state
        .lease_repo
        .list_applications_rls(&mut **rls.conn(), org_id, app_query)
        .await
        .map(|(items, total)| Json(ListApplicationsResponse { items, total }))
        .map_err(|e| {
            tracing::error!("Failed to list applications: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to list applications",
                )),
            )
        });

    rls.release().await;
    result
}

async fn get_application(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<db::models::TenantApplication>, (StatusCode, Json<ErrorResponse>)> {
    let result = match state
        .lease_repo
        .find_application_by_id_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(Some(app)) => Ok(Json(app)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Application not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get application: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get application")),
            ))
        }
    };

    rls.release().await;
    result
}

async fn update_application(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateApplication>,
) -> Result<Json<db::models::TenantApplication>, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .lease_repo
        .update_application_rls(&mut **rls.conn(), id, payload)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to update application: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to update application",
                )),
            )
        });

    rls.release().await;
    result
}

async fn submit_application(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(payload): Json<SubmitApplication>,
) -> Result<Json<db::models::TenantApplication>, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .lease_repo
        .submit_application_rls(&mut **rls.conn(), id, payload)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to submit application: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to submit application",
                )),
            )
        });

    rls.release().await;
    result
}

async fn review_application(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(payload): Json<ReviewApplication>,
) -> Result<Json<db::models::TenantApplication>, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .lease_repo
        .review_application_rls(&mut **rls.conn(), id, auth.user_id, payload)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to review application: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to review application",
                )),
            )
        });

    rls.release().await;
    result
}

// ==================== Screening Handlers (Story 19.2) ====================

/// Initiate screening with org context.
#[derive(Debug, Deserialize)]
pub struct InitiateScreeningRequest {
    /// Organization ID
    pub organization_id: Uuid,
    /// Screening data
    #[serde(flatten)]
    pub data: InitiateScreening,
}

async fn initiate_screening(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(payload): Json<InitiateScreeningRequest>,
) -> Result<(StatusCode, Json<Vec<db::models::TenantScreening>>), (StatusCode, Json<ErrorResponse>)>
{
    // Verify request org_id matches authenticated tenant
    if payload.organization_id != rls.tenant_id() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "Organization ID mismatch",
            )),
        ));
    }

    let result = state
        .lease_repo
        .initiate_screening_rls(&mut **rls.conn(), id, payload.organization_id, payload.data)
        .await
        .map(|s| (StatusCode::CREATED, Json(s)))
        .map_err(|e| {
            tracing::error!("Failed to initiate screening: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to initiate screening",
                )),
            )
        });

    rls.release().await;
    result
}

async fn record_consent(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(payload): Json<ScreeningConsent>,
) -> Result<Json<db::models::TenantScreening>, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .lease_repo
        .submit_screening_consent_rls(&mut **rls.conn(), id, payload)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to record consent: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to record consent")),
            )
        });

    rls.release().await;
    result
}

async fn get_screening(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<db::models::ScreeningSummary>>, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .lease_repo
        .get_screenings_for_application_rls(&mut **rls.conn(), id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get screenings: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get screenings")),
            )
        });

    rls.release().await;
    result
}

async fn update_screening_result(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateScreeningResult>,
) -> Result<Json<db::models::TenantScreening>, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .lease_repo
        .update_screening_result_rls(&mut **rls.conn(), id, payload)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to update screening result: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to update screening result",
                )),
            )
        });

    rls.release().await;
    result
}

// ==================== Template Handlers (Story 19.3) ====================

async fn create_template(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Json(payload): Json<CreateTemplateRequest>,
) -> Result<(StatusCode, Json<db::models::LeaseTemplate>), (StatusCode, Json<ErrorResponse>)> {
    // Verify request org_id matches authenticated tenant
    if payload.organization_id != rls.tenant_id() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "Organization ID mismatch",
            )),
        ));
    }

    let result = state
        .lease_repo
        .create_template_rls(
            &mut **rls.conn(),
            payload.organization_id,
            auth.user_id,
            payload.data,
        )
        .await
        .map(|t| (StatusCode::CREATED, Json(t)))
        .map_err(|e| {
            tracing::error!("Failed to create template: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to create template")),
            )
        });

    rls.release().await;
    result
}

async fn list_templates(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<OrgQuery>,
) -> Result<Json<Vec<db::models::LeaseTemplate>>, (StatusCode, Json<ErrorResponse>)> {
    // Verify request org_id matches authenticated tenant
    if query.organization_id != rls.tenant_id() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "Organization ID mismatch",
            )),
        ));
    }

    let result = state
        .lease_repo
        .list_templates_rls(&mut **rls.conn(), query.organization_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list templates: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list templates")),
            )
        });

    rls.release().await;
    result
}

async fn get_template(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<db::models::LeaseTemplate>, (StatusCode, Json<ErrorResponse>)> {
    let result = match state
        .lease_repo
        .find_template_by_id_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(Some(t)) => Ok(Json(t)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Template not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get template: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get template")),
            ))
        }
    };

    rls.release().await;
    result
}

/// Update template with org context.
#[derive(Debug, Deserialize)]
pub struct UpdateTemplateRequest {
    /// Organization ID
    pub organization_id: Uuid,
    /// Template data
    #[serde(flatten)]
    pub data: UpdateLeaseTemplate,
}

async fn update_template(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateTemplateRequest>,
) -> Result<Json<db::models::LeaseTemplate>, (StatusCode, Json<ErrorResponse>)> {
    // Verify request org_id matches authenticated tenant
    if payload.organization_id != rls.tenant_id() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "Organization ID mismatch",
            )),
        ));
    }

    let result = state
        .lease_repo
        .update_template_rls(&mut **rls.conn(), id, payload.organization_id, payload.data)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to update template: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to update template")),
            )
        });

    rls.release().await;
    result
}

// ==================== Lease Handlers (Story 19.3, 19.4) ====================

async fn create_lease(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Json(payload): Json<CreateLeaseRequest>,
) -> Result<(StatusCode, Json<db::models::Lease>), (StatusCode, Json<ErrorResponse>)> {
    // Verify request org_id matches authenticated tenant
    if payload.organization_id != rls.tenant_id() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "Organization ID mismatch",
            )),
        ));
    }

    let result = state
        .lease_repo
        .create_lease_rls(
            &mut **rls.conn(),
            payload.organization_id,
            auth.user_id,
            payload.data,
        )
        .await
        .map(|l| (StatusCode::CREATED, Json(l)))
        .map_err(|e| {
            tracing::error!("Failed to create lease: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to create lease")),
            )
        });

    rls.release().await;
    result
}

/// List leases query with org.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListLeasesQuery {
    /// Organization ID
    pub organization_id: Uuid,
    /// Filter by unit
    pub unit_id: Option<Uuid>,
    /// Filter by tenant
    pub tenant_id: Option<Uuid>,
    /// Filter by status
    pub status: Option<String>,
    /// Expiring within days
    pub expiring_within_days: Option<i32>,
    /// Page limit
    pub limit: Option<i32>,
    /// Page offset
    pub offset: Option<i32>,
}

/// List leases response with pagination.
#[derive(Debug, serde::Serialize)]
pub struct ListLeasesResponse {
    /// Leases
    pub items: Vec<db::models::LeaseSummary>,
    /// Total count
    pub total: i64,
}

async fn list_leases(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListLeasesQuery>,
) -> Result<Json<ListLeasesResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify request org_id matches authenticated tenant
    if query.organization_id != rls.tenant_id() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "Organization ID mismatch",
            )),
        ));
    }

    let org_id = query.organization_id;
    let lease_query = LeaseListQuery {
        unit_id: query.unit_id,
        tenant_id: query.tenant_id,
        status: query.status,
        expiring_within_days: query.expiring_within_days,
        limit: query.limit,
        offset: query.offset,
    };

    let result = state
        .lease_repo
        .list_leases_rls(&mut **rls.conn(), org_id, lease_query)
        .await
        .map(|(items, total)| Json(ListLeasesResponse { items, total }))
        .map_err(|e| {
            tracing::error!("Failed to list leases: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list leases")),
            )
        });

    rls.release().await;
    result
}

async fn get_lease(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<db::models::LeaseWithDetails>, (StatusCode, Json<ErrorResponse>)> {
    let result = match state
        .lease_repo
        .get_lease_with_details_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(Some(l)) => Ok(Json(l)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Lease not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get lease: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get lease")),
            ))
        }
    };

    rls.release().await;
    result
}

async fn update_lease(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateLease>,
) -> Result<Json<db::models::Lease>, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .lease_repo
        .update_lease_rls(&mut **rls.conn(), id, payload)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to update lease: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to update lease")),
            )
        });

    rls.release().await;
    result
}

async fn terminate_lease(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(payload): Json<TerminateLease>,
) -> Result<Json<db::models::Lease>, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .lease_repo
        .terminate_lease_rls(&mut **rls.conn(), id, auth.user_id, payload)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to terminate lease: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to terminate lease")),
            )
        });

    rls.release().await;
    result
}

async fn renew_lease(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(payload): Json<RenewLease>,
) -> Result<(StatusCode, Json<db::models::Lease>), (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .lease_repo
        .renew_lease_rls(&mut **rls.conn(), id, auth.user_id, payload)
        .await
        .map(|l| (StatusCode::CREATED, Json(l)))
        .map_err(|e| {
            tracing::error!("Failed to renew lease: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to renew lease")),
            )
        });

    rls.release().await;
    result
}

// ==================== Amendment Handlers ====================

async fn create_amendment(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreateAmendment>,
) -> Result<(StatusCode, Json<db::models::LeaseAmendment>), (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .lease_repo
        .create_amendment_rls(&mut **rls.conn(), id, auth.user_id, payload)
        .await
        .map(|a| (StatusCode::CREATED, Json(a)))
        .map_err(|e| {
            tracing::error!("Failed to create amendment: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to create amendment")),
            )
        });

    rls.release().await;
    result
}

async fn list_amendments(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<db::models::LeaseWithDetails>, (StatusCode, Json<ErrorResponse>)> {
    // For now, use get_lease_with_details which includes amendments
    let result = match state
        .lease_repo
        .get_lease_with_details_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(Some(details)) => Ok(Json(details)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Lease not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get lease with amendments: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list amendments")),
            ))
        }
    };

    rls.release().await;
    result
}

// ==================== Payment Handlers ====================

/// Payment ID path parameter.
#[derive(Debug, Deserialize)]
pub struct PaymentPath {
    /// Lease ID (unused, but required for route matching)
    pub id: Uuid,
    /// Payment ID
    pub payment_id: Uuid,
}

async fn record_payment(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(payment_id): Path<Uuid>,
    Json(payload): Json<RecordPayment>,
) -> Result<Json<db::models::LeasePayment>, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .lease_repo
        .record_payment_rls(&mut **rls.conn(), payment_id, payload)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to record payment: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to record payment")),
            )
        });

    rls.release().await;
    result
}

async fn list_payments(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<db::models::LeaseWithDetails>, (StatusCode, Json<ErrorResponse>)> {
    // Use get_lease_with_details which includes upcoming_payments
    let result = match state
        .lease_repo
        .get_lease_with_details_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(Some(details)) => Ok(Json(details)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Lease not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get lease payments: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list payments")),
            ))
        }
    };

    rls.release().await;
    result
}

async fn get_payment_summary(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<OrgQuery>,
) -> Result<Json<Vec<db::models::PaymentSummary>>, (StatusCode, Json<ErrorResponse>)> {
    // Verify request org_id matches authenticated tenant
    if query.organization_id != rls.tenant_id() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "Organization ID mismatch",
            )),
        ));
    }

    let result = state
        .lease_repo
        .get_overdue_payments_rls(&mut **rls.conn(), query.organization_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get payment summary: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to get payment summary",
                )),
            )
        });

    rls.release().await;
    result
}

// ==================== Reminder Handlers (Story 19.5) ====================

async fn create_reminder(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreateReminder>,
) -> Result<(StatusCode, Json<db::models::LeaseReminder>), (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .lease_repo
        .create_reminder_rls(&mut **rls.conn(), id, payload)
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(|e| {
            tracing::error!("Failed to create reminder: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to create reminder")),
            )
        });

    rls.release().await;
    result
}

async fn list_reminders(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<db::models::LeaseWithDetails>, (StatusCode, Json<ErrorResponse>)> {
    // Use get_lease_with_details which includes reminders
    let result = match state
        .lease_repo
        .get_lease_with_details_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(Some(details)) => Ok(Json(details)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Lease not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to list reminders: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list reminders")),
            ))
        }
    };

    rls.release().await;
    result
}

// ==================== Dashboard/Statistics Handlers ====================

async fn get_expiring_leases(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<OrgQuery>,
) -> Result<Json<db::models::ExpirationOverview>, (StatusCode, Json<ErrorResponse>)> {
    // Verify request org_id matches authenticated tenant
    if query.organization_id != rls.tenant_id() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "Organization ID mismatch",
            )),
        ));
    }

    let result = state
        .lease_repo
        .get_expiration_overview_rls(&mut **rls.conn(), query.organization_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get expiring leases: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to get expiring leases",
                )),
            )
        });

    rls.release().await;
    result
}

async fn get_statistics(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<OrgQuery>,
) -> Result<Json<db::models::LeaseStatistics>, (StatusCode, Json<ErrorResponse>)> {
    // Verify request org_id matches authenticated tenant
    if query.organization_id != rls.tenant_id() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "Organization ID mismatch",
            )),
        ));
    }

    let result = state
        .lease_repo
        .get_statistics_rls(&mut **rls.conn(), query.organization_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get statistics: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get statistics")),
            )
        });

    rls.release().await;
    result
}
