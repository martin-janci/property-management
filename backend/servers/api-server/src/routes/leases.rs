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
use common::TenantRole;
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
        // E-signing (issue #977): wire to the hardened signature-request subsystem
        .route("/{id}/send-for-signature", post(send_lease_for_signature))
        // Lease Amendments
        .route("/{id}/amendments", post(create_amendment))
        .route("/{id}/amendments", get(list_amendments))
        // Lease Payments
        // DEPRECATED: `{id}` here is the *payment* id. Kept for backward
        // compatibility; use `/{id}/payments/{payment_id}` instead.
        .route("/{id}/payments", post(record_payment_legacy))
        .route("/{id}/payments/{payment_id}", post(record_lease_payment))
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

// ==================== Authorization Guards ====================
//
// The `*_rls` repository queries scope only by id and rely on the table RLS
// policies + the connection's `app.current_org_id` GUC for org isolation. That
// fails open on a BYPASSRLS/superuser pool (e.g. the integration-test pool), so
// every handler taking an id re-checks ownership here: a foreign-org id is
// indistinguishable from a missing one (404). Mutations that are staff
// operations are additionally gated on manager role (403), mirroring
// `review_application` (#806).

type ApiError = (StatusCode, Json<ErrorResponse>);

fn not_found(msg: &str) -> ApiError {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", msg)),
    )
}

fn db_error(msg: &str) -> ApiError {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new("DB_ERROR", msg)),
    )
}

/// Release the RLS connection and return `err` when a guard fails.
macro_rules! guard {
    ($rls:ident, $check:expr) => {
        if let Err(err) = $check {
            $rls.release().await;
            return Err(err);
        }
    };
}

/// Manager-or-higher (incl. super-admins), the same gate `review_application`
/// and `send_lease_for_signature` use.
fn is_lease_manager(rls: &RlsConnection) -> bool {
    rls.is_super_admin() || rls.has_role(TenantRole::Manager)
}

/// 403 unless the caller is a manager.
fn require_lease_manager(rls: &RlsConnection, msg: &str) -> Result<(), ApiError> {
    if is_lease_manager(rls) {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new("FORBIDDEN", msg)),
        ))
    }
}

/// 404 unless the lease exists in the caller's organization.
async fn ensure_lease_in_org(
    state: &AppState,
    rls: &mut RlsConnection,
    lease_id: Uuid,
) -> Result<(), ApiError> {
    let org_id = rls.tenant_id();
    match state
        .lease_repo
        .lease_org_rls(&mut **rls.conn(), lease_id)
        .await
    {
        Ok(Some(org)) if org == org_id => Ok(()),
        Ok(_) => Err(not_found("Lease not found")),
        Err(e) => {
            tracing::error!("Failed to resolve lease org: {:?}", e);
            Err(db_error("Failed to load lease"))
        }
    }
}

/// Load a lease, 404 unless it exists in the caller's organization.
async fn load_lease_in_org(
    state: &AppState,
    rls: &mut RlsConnection,
    lease_id: Uuid,
) -> Result<db::models::Lease, ApiError> {
    let org_id = rls.tenant_id();
    match state
        .lease_repo
        .find_lease_by_id_rls(&mut **rls.conn(), lease_id)
        .await
    {
        Ok(Some(lease)) if lease.organization_id == org_id => Ok(lease),
        Ok(_) => Err(not_found("Lease not found")),
        Err(e) => {
            tracing::error!("Failed to load lease: {:?}", e);
            Err(db_error("Failed to load lease"))
        }
    }
}

fn bad_request(msg: &str) -> ApiError {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse::new("VALIDATION_ERROR", msg)),
    )
}

/// Map a lease write error to HTTP: a vanished row (`RowNotFound`) is 404;
/// invalid input rejected by Postgres (SQLSTATE class 22 — data exception,
/// e.g. an unknown enum value — or class 23 — integrity constraint
/// violation) is 400; anything else is a genuine 500.
fn map_lease_write_err(e: sqlx::Error, not_found_msg: &str, ctx: &str) -> ApiError {
    match &e {
        sqlx::Error::RowNotFound => not_found(not_found_msg),
        sqlx::Error::Database(db)
            if db
                .code()
                .is_some_and(|c| c.starts_with("22") || c.starts_with("23")) =>
        {
            tracing::warn!("{ctx}: rejected input: {:?}", e);
            bad_request(&format!("{ctx}: invalid input"))
        }
        _ => {
            tracing::error!("{ctx}: {:?}", e);
            db_error(ctx)
        }
    }
}

/// Valid `termination_reason` enum values (migration 00052).
const TERMINATION_REASONS: &[&str] = &[
    "end_of_term",
    "mutual_agreement",
    "tenant_breach",
    "landlord_breach",
    "property_sale",
    "renovation",
    "other",
];

/// Lease statuses from which no further lifecycle transition is allowed.
const CLOSED_LEASE_STATUSES: &[&str] = &["terminated", "renewed", "expired", "cancelled"];

/// 404 unless the lease template exists in the caller's organization.
async fn ensure_template_in_org(
    state: &AppState,
    rls: &mut RlsConnection,
    template_id: Uuid,
) -> Result<(), ApiError> {
    let org_id = rls.tenant_id();
    match state
        .lease_repo
        .template_org_rls(&mut **rls.conn(), template_id)
        .await
    {
        Ok(Some(org)) if org == org_id => Ok(()),
        Ok(_) => Err(not_found("Template not found")),
        Err(e) => {
            tracing::error!("Failed to resolve template org: {:?}", e);
            Err(db_error("Failed to load template"))
        }
    }
}

/// Access to a single tenant application: 404 unless it belongs to the
/// caller's organization; then 403 unless the caller is a manager or the
/// applicant themself (authenticated email equals `applicant_email`).
///
/// Applications carry no user FK, so the applicant email is the only link
/// between an application and a platform user. Applications hold sensitive
/// PII (national id, income, screening state), so ordinary org members who
/// are not the applicant must not read or change them.
async fn ensure_application_access(
    state: &AppState,
    rls: &mut RlsConnection,
    auth: &AuthUser,
    application_id: Uuid,
) -> Result<(), ApiError> {
    let org_id = rls.tenant_id();
    let applicant_email = match state
        .lease_repo
        .application_scope_rls(&mut **rls.conn(), application_id)
        .await
    {
        Ok(Some((org, email))) if org == org_id => email,
        Ok(_) => return Err(not_found("Application not found")),
        Err(e) => {
            tracing::error!("Failed to resolve application org: {:?}", e);
            return Err(db_error("Failed to load application"));
        }
    };
    if is_lease_manager(rls)
        || applicant_email
            .trim()
            .eq_ignore_ascii_case(auth.email.trim())
    {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers or the applicant can access this application",
            )),
        ))
    }
}

/// 404 unless the application exists in the caller's organization.
async fn ensure_application_in_org(
    state: &AppState,
    rls: &mut RlsConnection,
    application_id: Uuid,
) -> Result<(), ApiError> {
    let org_id = rls.tenant_id();
    match state
        .lease_repo
        .application_scope_rls(&mut **rls.conn(), application_id)
        .await
    {
        Ok(Some((org, _))) if org == org_id => Ok(()),
        Ok(_) => Err(not_found("Application not found")),
        Err(e) => {
            tracing::error!("Failed to resolve application org: {:?}", e);
            Err(db_error("Failed to load application"))
        }
    }
}

/// Resolve a screening's parent application id: 404 unless the screening
/// exists in the caller's organization.
async fn screening_application_in_org(
    state: &AppState,
    rls: &mut RlsConnection,
    screening_id: Uuid,
) -> Result<Uuid, ApiError> {
    let org_id = rls.tenant_id();
    match state
        .lease_repo
        .screening_scope_rls(&mut **rls.conn(), screening_id)
        .await
    {
        Ok(Some((org, application_id))) if org == org_id => Ok(application_id),
        Ok(_) => Err(not_found("Screening not found")),
        Err(e) => {
            tracing::error!("Failed to resolve screening org: {:?}", e);
            Err(db_error("Failed to load screening"))
        }
    }
}

/// 404 unless the payment exists in the caller's organization and — when
/// `lease_id` is given — belongs to that lease.
async fn ensure_payment_in_org(
    state: &AppState,
    rls: &mut RlsConnection,
    payment_id: Uuid,
    lease_id: Option<Uuid>,
) -> Result<(), ApiError> {
    let org_id = rls.tenant_id();
    match state
        .lease_repo
        .payment_scope_rls(&mut **rls.conn(), payment_id)
        .await
    {
        Ok(Some((org, payment_lease)))
            if org == org_id && lease_id.is_none_or(|l| l == payment_lease) =>
        {
            Ok(())
        }
        Ok(_) => Err(not_found("Payment not found")),
        Err(e) => {
            tracing::error!("Failed to resolve payment org: {:?}", e);
            Err(db_error("Failed to load payment"))
        }
    }
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
    guard!(
        rls,
        require_lease_manager(&rls, "Only managers can list tenant applications")
    );
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
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<db::models::TenantApplication>, (StatusCode, Json<ErrorResponse>)> {
    guard!(
        rls,
        ensure_application_access(&state, &mut rls, &auth, id).await
    );
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
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateApplication>,
) -> Result<Json<db::models::TenantApplication>, (StatusCode, Json<ErrorResponse>)> {
    guard!(
        rls,
        ensure_application_access(&state, &mut rls, &auth, id).await
    );
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
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(payload): Json<SubmitApplication>,
) -> Result<Json<db::models::TenantApplication>, (StatusCode, Json<ErrorResponse>)> {
    guard!(
        rls,
        ensure_application_access(&state, &mut rls, &auth, id).await
    );
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
    // Reviewing (approving/rejecting) a tenant application is a manager action.
    // Without this gate any authenticated member of the org — including a
    // resident — could approve or reject applications (privilege escalation,
    // closes #806). Managers and higher (incl. super-admins) pass.
    if !rls.is_super_admin() && !rls.has_role(TenantRole::Manager) {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can review tenant applications",
            )),
        ));
    }

    guard!(rls, ensure_application_in_org(&state, &mut rls, id).await);

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
    guard!(
        rls,
        require_lease_manager(&rls, "Only managers can initiate tenant screening")
    );
    guard!(rls, ensure_application_in_org(&state, &mut rls, id).await);
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
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(payload): Json<ScreeningConsent>,
) -> Result<Json<db::models::TenantScreening>, (StatusCode, Json<ErrorResponse>)> {
    // `{id}` is the screening id (repository semantics). Consent may be
    // recorded by a manager (on receipt of the signed consent document) or by
    // the applicant themself for their own screening.
    let application_id = match screening_application_in_org(&state, &mut rls, id).await {
        Ok(app_id) => app_id,
        Err(e) => {
            rls.release().await;
            return Err(e);
        }
    };
    guard!(
        rls,
        ensure_application_access(&state, &mut rls, &auth, application_id).await
    );
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
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<db::models::ScreeningSummary>>, (StatusCode, Json<ErrorResponse>)> {
    guard!(
        rls,
        ensure_application_access(&state, &mut rls, &auth, id).await
    );
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
    guard!(
        rls,
        require_lease_manager(&rls, "Only managers can record screening results")
    );
    guard!(
        rls,
        screening_application_in_org(&state, &mut rls, id).await
    );
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
    guard!(
        rls,
        require_lease_manager(&rls, "Only managers can create lease templates")
    );
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
    guard!(rls, ensure_template_in_org(&state, &mut rls, id).await);
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
    guard!(
        rls,
        require_lease_manager(&rls, "Only managers can update lease templates")
    );
    guard!(rls, ensure_template_in_org(&state, &mut rls, id).await);
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
    guard!(
        rls,
        require_lease_manager(&rls, "Only managers can create leases")
    );
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
    guard!(rls, ensure_lease_in_org(&state, &mut rls, id).await);
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
    guard!(
        rls,
        require_lease_manager(&rls, "Only managers can update leases")
    );
    guard!(rls, ensure_lease_in_org(&state, &mut rls, id).await);
    let result = state
        .lease_repo
        .update_lease_rls(&mut **rls.conn(), id, payload)
        .await
        .map(Json)
        .map_err(|e| map_lease_write_err(e, "Lease not found", "Failed to update lease"));

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
    guard!(
        rls,
        require_lease_manager(&rls, "Only managers can terminate leases")
    );
    let lease = match load_lease_in_org(&state, &mut rls, id).await {
        Ok(l) => l,
        Err(e) => {
            rls.release().await;
            return Err(e);
        }
    };
    if !TERMINATION_REASONS.contains(&payload.termination_reason.as_str()) {
        rls.release().await;
        return Err(bad_request(&format!(
            "Invalid termination_reason; expected one of: {}",
            TERMINATION_REASONS.join(", ")
        )));
    }
    if CLOSED_LEASE_STATUSES.contains(&lease.status.as_str()) {
        rls.release().await;
        return Err(bad_request(&format!(
            "Lease cannot be terminated from status '{}'",
            lease.status
        )));
    }
    let result = state
        .lease_repo
        .terminate_lease_rls(&mut **rls.conn(), id, auth.user_id, payload)
        .await
        .map(Json)
        .map_err(|e| map_lease_write_err(e, "Lease not found", "Failed to terminate lease"));

    rls.release().await;
    result
}

/// Request body for sending a lease out for e-signature (issue #977).
///
/// The lease's generated agreement must already exist as a row in the
/// `documents` table; its id is supplied here. The endpoint then delegates to
/// the hardened signature-request subsystem to create the signing workflow.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SendLeaseForSignatureRequest {
    /// Id of the generated lease-agreement document (a `documents` row).
    pub document_id: Uuid,
    /// Landlord's email for the signing invitation. Defaults to the
    /// authenticated requester's email when omitted.
    #[serde(default)]
    pub landlord_email: Option<String>,
    /// Days until the signature request expires (default: 30).
    #[serde(default)]
    pub expires_in_days: Option<i32>,
}

/// Send a lease out for e-signature (issue #977, Option A).
///
/// Wires leases into the existing hardened signature-request subsystem
/// (`routes::signatures`) rather than a lease-native signing flow: it resolves
/// the lease's generated document and creates a signature request for it with
/// the landlord and tenant as signers, reusing the one HMAC-signed-link / nonce
/// / email path. The lease is then bound to the created request and moved to
/// `pending_signature`.
///
/// Document resolution caveat: a lease currently has no FK to a `documents`
/// row (only a `document_url` text column), and lease-agreement generation is
/// not yet wired to persist a `documents` row. So this endpoint requires the
/// caller to pass the generated document's id and returns `400` if absent.
async fn send_lease_for_signature(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(payload): Json<SendLeaseForSignatureRequest>,
) -> Result<(StatusCode, Json<db::models::SignatureRequest>), (StatusCode, Json<ErrorResponse>)> {
    // Sending a lease out for signature is a manager action (same gate as
    // review_application). Without this, any org member could trigger signing.
    if !rls.is_super_admin() && !rls.has_role(TenantRole::Manager) {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can send a lease for signature",
            )),
        ));
    }

    // Load the lease (RLS-scoped) to derive the tenant signer and confirm it
    // exists within the caller's organisation.
    let lease = match state
        .lease_repo
        .find_lease_by_id_rls(&mut **rls.conn(), id)
        .await
    {
        // Explicit org guard: RLS alone fails open on a BYPASSRLS pool.
        Ok(Some(l)) if l.organization_id == rls.tenant_id() => l,
        Ok(_) => {
            rls.release().await;
            return Err(not_found("Lease not found"));
        }
        Err(e) => {
            tracing::error!("Failed to load lease for signature: {:?}", e);
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to load lease")),
            ));
        }
    };

    // Build the signer set: landlord (the requesting manager, unless an explicit
    // landlord_email is given) + tenant (from the lease record).
    let landlord_email = payload
        .landlord_email
        .clone()
        .unwrap_or_else(|| auth.email.clone());
    if landlord_email.trim().is_empty() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "VALIDATION_ERROR",
                "A landlord email is required to send the lease for signature",
            )),
        ));
    }
    if lease.tenant_email.trim().is_empty() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "VALIDATION_ERROR",
                "Lease has no tenant email; cannot create a signing request",
            )),
        ));
    }

    let signers = vec![
        db::models::CreateSigner {
            email: landlord_email,
            name: lease.landlord_name.clone(),
            order: 0,
        },
        db::models::CreateSigner {
            email: lease.tenant_email.clone(),
            name: lease.tenant_name.clone(),
            order: 0,
        },
    ];

    let create_request = db::models::CreateSignatureRequest {
        signers,
        subject: Some(format!("Lease agreement for {}", lease.tenant_name)),
        message: None,
        provider: None,
        expires_in_days: payload.expires_in_days,
    };

    // Delegate to the single hardened signature-request creation path. This
    // owns document existence/dedup checks plus HMAC link + nonce + email; we do
    // NOT duplicate any of that here. The caller owns the rls lifecycle.
    let response = match crate::routes::signatures::create_signature_request_for_document(
        &state,
        &auth,
        &mut rls,
        payload.document_id,
        create_request,
    )
    .await
    {
        Ok(resp) => resp,
        Err(e) => {
            rls.release().await;
            return Err(e);
        }
    };

    let signature_request_id = response.signature_request.id;

    // Bind the lease to the created request and transition draft ->
    // pending_signature. The actual signing transitions are owned by the
    // signature subsystem, not lease-native logic.
    if let Err(e) = state
        .lease_repo
        .mark_sent_for_signature_rls(&mut **rls.conn(), id, signature_request_id)
        .await
    {
        // The signature request was already created; surface a clear error but
        // do not roll it back (the manager can retry binding via a follow-up).
        tracing::error!("Failed to mark lease as sent for signature: {:?}", e);
        rls.release().await;
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DB_ERROR",
                "Signature request created but failed to update lease status",
            )),
        ));
    }

    rls.release().await;

    Ok((StatusCode::CREATED, Json(response.signature_request)))
}

async fn renew_lease(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(payload): Json<RenewLease>,
) -> Result<(StatusCode, Json<db::models::Lease>), (StatusCode, Json<ErrorResponse>)> {
    guard!(
        rls,
        require_lease_manager(&rls, "Only managers can renew leases")
    );
    let lease = match load_lease_in_org(&state, &mut rls, id).await {
        Ok(l) => l,
        Err(e) => {
            rls.release().await;
            return Err(e);
        }
    };
    // The renewal starts the day after the current end date, so the new end
    // date must be strictly after the current one.
    if payload.new_end_date <= lease.end_date {
        rls.release().await;
        return Err(bad_request(&format!(
            "new_end_date must be after the current end date ({})",
            lease.end_date
        )));
    }
    if payload.term_months <= 0 {
        rls.release().await;
        return Err(bad_request("term_months must be positive"));
    }
    if CLOSED_LEASE_STATUSES.contains(&lease.status.as_str()) {
        rls.release().await;
        return Err(bad_request(&format!(
            "Lease cannot be renewed from status '{}'",
            lease.status
        )));
    }
    let result = state
        .lease_repo
        .renew_lease_rls(&mut **rls.conn(), id, auth.user_id, payload)
        .await
        .map(|l| (StatusCode::CREATED, Json(l)))
        .map_err(|e| map_lease_write_err(e, "Lease not found", "Failed to renew lease"));

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
    guard!(
        rls,
        require_lease_manager(&rls, "Only managers can amend leases")
    );
    guard!(rls, ensure_lease_in_org(&state, &mut rls, id).await);
    if payload.title.trim().is_empty() {
        rls.release().await;
        return Err(bad_request("title must not be empty"));
    }
    let result = state
        .lease_repo
        .create_amendment_rls(&mut **rls.conn(), id, auth.user_id, payload)
        .await
        .map(|a| (StatusCode::CREATED, Json(a)))
        .map_err(|e| map_lease_write_err(e, "Lease not found", "Failed to create amendment"));

    rls.release().await;
    result
}

async fn list_amendments(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<db::models::LeaseWithDetails>, (StatusCode, Json<ErrorResponse>)> {
    guard!(rls, ensure_lease_in_org(&state, &mut rls, id).await);
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

/// Lease + payment path parameters for
/// `POST /leases/{id}/payments/{payment_id}`.
#[derive(Debug, Deserialize)]
pub struct PaymentPath {
    /// Lease ID
    pub id: Uuid,
    /// Payment ID (must belong to the lease)
    pub payment_id: Uuid,
}

/// Record a payment against a scheduled lease payment:
/// `POST /leases/{id}/payments/{payment_id}`.
///
/// Manager-only (403). 404 unless the lease belongs to the caller's
/// organization and the payment belongs to that lease.
async fn record_lease_payment(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<PaymentPath>,
    Json(payload): Json<RecordPayment>,
) -> Result<Json<db::models::LeasePayment>, ApiError> {
    guard!(
        rls,
        require_lease_manager(&rls, "Only managers can record lease payments")
    );
    guard!(rls, ensure_lease_in_org(&state, &mut rls, path.id).await);
    guard!(
        rls,
        ensure_payment_in_org(&state, &mut rls, path.payment_id, Some(path.id)).await
    );
    let result = persist_payment(&state, &mut rls, path.payment_id, payload).await;
    rls.release().await;
    result
}

/// DEPRECATED legacy route `POST /leases/{id}/payments`, where `{id}` is the
/// **payment** id (not the lease id). Kept for backward compatibility with
/// existing clients; new callers should use
/// `POST /leases/{id}/payments/{payment_id}` ([`record_lease_payment`]).
///
/// Manager-only (403); 404 unless the payment belongs to the caller's org.
async fn record_payment_legacy(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(payment_id): Path<Uuid>,
    Json(payload): Json<RecordPayment>,
) -> Result<Json<db::models::LeasePayment>, ApiError> {
    guard!(
        rls,
        require_lease_manager(&rls, "Only managers can record lease payments")
    );
    guard!(
        rls,
        ensure_payment_in_org(&state, &mut rls, payment_id, None).await
    );
    let result = persist_payment(&state, &mut rls, payment_id, payload).await;
    rls.release().await;
    result
}

async fn persist_payment(
    state: &AppState,
    rls: &mut RlsConnection,
    payment_id: Uuid,
    payload: RecordPayment,
) -> Result<Json<db::models::LeasePayment>, ApiError> {
    state
        .lease_repo
        .record_payment_rls(&mut **rls.conn(), payment_id, payload)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to record payment: {:?}", e);
            db_error("Failed to record payment")
        })
}

async fn list_payments(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<db::models::LeaseWithDetails>, (StatusCode, Json<ErrorResponse>)> {
    guard!(rls, ensure_lease_in_org(&state, &mut rls, id).await);
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
    guard!(
        rls,
        require_lease_manager(&rls, "Only managers can create lease reminders")
    );
    guard!(rls, ensure_lease_in_org(&state, &mut rls, id).await);
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
    guard!(rls, ensure_lease_in_org(&state, &mut rls, id).await);
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
