// ROADMAP(PAP-24): roadmap stub — handlers return 501. Unmounted in lib.rs (no /api/v1/vendor-portal
// route) so callers get 404 instead of a false 501. The live, implemented vendor surface is vendors.rs.
//! Vendor Operations Portal routes (Epic 78).
//!
//! Provides vendor-facing endpoints for job management, property access,
//! work completion, invoicing, and performance tracking.
//!
//! # Authorization & data model (issue #828)
//!
//! Two security defects were closed here:
//!
//! 1. **Fabricated PII.** Every handler previously returned hard-coded sample
//!    data — fake `VendorJob`s with invented `contact_phone` /
//!    `building_address`, fake invoices, fake access codes — without ever
//!    touching the database (`State(_state)` was unused). Any authenticated
//!    staff principal received this fabricated PII as if it were live data.
//!
//! 2. **Unscoped access with no vendor identity.** The portal's request DTOs
//!    (`VendorJob`, `PropertyAccessInfo`, `VendorProfile`, …) describe a
//!    vendor-operations data model — a *vendor staff member* signed in to see
//!    *their vendor company's* jobs, access codes, completions, and earnings.
//!    That model has **no backing schema**: there is no `vendor_staff` /
//!    `vendor_companies` table linking a `users` row to a `vendors` row, no
//!    `vendor_jobs` table, no temporary-access-code store, and no
//!    work-completion store. Without a vendor-identity mapping there is no
//!    correct way to scope a request to "this caller's vendor" — returning
//!    org-wide rows instead would expose every work order / invoice in the org
//!    to any staff member through the vendor portal, which is a *new* leak.
//!
//! Until the vendor-identity schema lands (a `vendor_staff` membership table
//! plus the per-job access-code / work-completion stores Epic 78 assumes),
//! every handler returns `501 Not Implemented` — an honest, uniform signal
//! that the feature has no persistence yet — rather than fabricating PII or
//! leaking unscoped data. This mirrors the developer-portal fix in
//! `routes/public_api.rs` (#851), which replaced fabricated API-key/usage
//! responses with `501` for the same reason.
//!
//! The existing `require_vendor_principal` gate still runs first: a non-staff
//! (public / reality-portal) principal is rejected with `403` before any
//! further work, and `RequestPrincipal` itself rejects unauthenticated callers
//! (`401`) and callers with no active membership in the resolved org (`403`).
//!
//! SECURITY (retained from #790): the previous `extract_vendor_context` helper
//! deserialized the client-supplied `X-Tenant-Context` JSON header directly
//! into a `TenantContext` with no JWT verification — any unauthenticated caller
//! could forge tenancy. That helper is gone; every handler now goes through
//! `RequestPrincipal` (verified bearer JWT + host-resolved tenant).

use api_core::extractors::principal::RequestPrincipal;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::PrincipalKind;
use db::models::{
    AcceptJobRequest, AccessCodeResponse, DeclineJobRequest, GenerateAccessCode,
    PropertyAccessInfo, SubmitWorkCompletion, VendorDashboardStats, VendorEarningsSummary,
    VendorFeedback, VendorInvoiceWithTracking, VendorJob, VendorJobQuery, VendorJobSummary,
    VendorProfile, WorkCompletion,
};
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::state::AppState;

// ============================================================================
// Helper Functions
// ============================================================================

/// Reject any principal that isn't Staff/Platform.
///
/// First-line gate: a `Public` (reality-portal) principal must never reach a
/// vendor-portal handler. Note this is *not* a vendor-identity check — see the
/// module docs: the schema to bind a user to a specific vendor company does not
/// exist yet, so finer-grained "is this caller a member of vendor X" scoping
/// cannot be enforced and the handlers below fail closed with `501`.
fn require_vendor_principal(
    principal: &RequestPrincipal,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    match principal.kind {
        PrincipalKind::Staff | PrincipalKind::Platform => Ok(()),
        PrincipalKind::Public => Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "VENDOR_PRINCIPAL_REQUIRED",
                "Vendor portal endpoints require a staff-side principal",
            )),
        )),
    }
}

/// Uniform "not implemented" response for vendor-portal endpoints whose
/// persistence / identity model does not exist yet (closes #828).
///
/// These handlers previously fabricated sample data (including PII) instead of
/// querying the database. Until the vendor-identity schema (`vendor_staff`
/// membership + per-job access-code / work-completion stores) lands, every
/// endpoint returns `501` so callers get an honest signal rather than
/// fabricated or unscoped data. Mirrors `routes/public_api.rs::not_implemented`
/// (#851).
fn not_implemented(feature: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse::new(
            "NOT_IMPLEMENTED",
            format!("{feature} is not implemented yet (no vendor-identity schema; see issue #828)"),
        )),
    )
}

/// Query parameters for invoice listing.
#[derive(Debug, Deserialize, IntoParams)]
pub struct InvoiceQuery {
    pub status: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

/// Query parameters for feedback listing.
#[derive(Debug, Deserialize, IntoParams)]
pub struct FeedbackQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

/// Query parameters for earnings summary.
#[derive(Debug, Deserialize, IntoParams)]
pub struct EarningsQuery {
    pub period_months: Option<i32>,
}

/// Create vendor portal router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Dashboard (Story 78.1)
        .route("/dashboard/stats", get(get_dashboard_stats))
        // Jobs (Story 78.1)
        .route("/jobs", get(list_jobs))
        .route("/jobs/{job_id}", get(get_job_details))
        .route("/jobs/{job_id}/accept", post(accept_job))
        .route("/jobs/{job_id}/decline", post(decline_job))
        .route(
            "/jobs/{job_id}/propose-time",
            post(propose_alternative_time),
        )
        // Property Access (Story 78.2)
        .route("/jobs/{job_id}/access", get(get_access_info))
        .route(
            "/jobs/{job_id}/access/generate-code",
            post(generate_access_code),
        )
        // Work Completion & Invoicing (Story 78.3)
        .route("/jobs/{job_id}/complete", post(submit_work_completion))
        .route("/jobs/{job_id}/completion", get(get_work_completion))
        .route("/invoices", get(list_invoices))
        // Profile & Feedback (Story 78.4)
        .route("/profile", get(get_profile))
        .route("/feedback", get(list_feedback))
        .route("/earnings", get(get_earnings_summary))
}

// ==================== Dashboard Handlers (Story 78.1) ====================

/// Get vendor dashboard statistics.
#[utoipa::path(
    get,
    path = "/api/v1/vendor-portal/dashboard/stats",
    tag = "Vendor Portal",
    responses(
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a staff principal"),
        (status = 501, description = "Not implemented (no vendor-identity schema)"),
    )
)]
async fn get_dashboard_stats(
    State(_state): State<AppState>,
    principal: RequestPrincipal,
) -> Result<Json<VendorDashboardStats>, (StatusCode, Json<ErrorResponse>)> {
    require_vendor_principal(&principal)?;
    Err(not_implemented("Vendor dashboard statistics"))
}

// ==================== Job Handlers (Story 78.1) ====================

/// List jobs for vendor.
#[utoipa::path(
    get,
    path = "/api/v1/vendor-portal/jobs",
    tag = "Vendor Portal",
    params(VendorJobQuery),
    responses(
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a staff principal"),
        (status = 501, description = "Not implemented (no vendor-identity schema)"),
    )
)]
async fn list_jobs(
    State(_state): State<AppState>,
    principal: RequestPrincipal,
    Query(_query): Query<VendorJobQuery>,
) -> Result<Json<Vec<VendorJobSummary>>, (StatusCode, Json<ErrorResponse>)> {
    require_vendor_principal(&principal)?;
    Err(not_implemented("Vendor job listing"))
}

/// Get job details.
#[utoipa::path(
    get,
    path = "/api/v1/vendor-portal/jobs/{job_id}",
    tag = "Vendor Portal",
    params(
        ("job_id" = Uuid, Path, description = "Job ID")
    ),
    responses(
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a staff principal"),
        (status = 501, description = "Not implemented (no vendor-identity schema)"),
    )
)]
async fn get_job_details(
    State(_state): State<AppState>,
    principal: RequestPrincipal,
    Path(_job_id): Path<Uuid>,
) -> Result<Json<VendorJob>, (StatusCode, Json<ErrorResponse>)> {
    require_vendor_principal(&principal)?;
    Err(not_implemented("Vendor job details"))
}

/// Accept a job.
#[utoipa::path(
    post,
    path = "/api/v1/vendor-portal/jobs/{job_id}/accept",
    tag = "Vendor Portal",
    params(
        ("job_id" = Uuid, Path, description = "Job ID")
    ),
    request_body = AcceptJobRequest,
    responses(
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a staff principal"),
        (status = 501, description = "Not implemented (no vendor-identity schema)"),
    )
)]
async fn accept_job(
    State(_state): State<AppState>,
    principal: RequestPrincipal,
    Path(_job_id): Path<Uuid>,
    Json(_request): Json<AcceptJobRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    require_vendor_principal(&principal)?;
    Err(not_implemented("Accept job"))
}

/// Decline a job.
#[utoipa::path(
    post,
    path = "/api/v1/vendor-portal/jobs/{job_id}/decline",
    tag = "Vendor Portal",
    params(
        ("job_id" = Uuid, Path, description = "Job ID")
    ),
    request_body = DeclineJobRequest,
    responses(
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a staff principal"),
        (status = 501, description = "Not implemented (no vendor-identity schema)"),
    )
)]
async fn decline_job(
    State(_state): State<AppState>,
    principal: RequestPrincipal,
    Path(_job_id): Path<Uuid>,
    Json(_request): Json<DeclineJobRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    require_vendor_principal(&principal)?;
    Err(not_implemented("Decline job"))
}

/// Propose alternative time for a job.
#[utoipa::path(
    post,
    path = "/api/v1/vendor-portal/jobs/{job_id}/propose-time",
    tag = "Vendor Portal",
    params(
        ("job_id" = Uuid, Path, description = "Job ID")
    ),
    request_body = db::models::ProposeAlternativeTime,
    responses(
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a staff principal"),
        (status = 501, description = "Not implemented (no vendor-identity schema)"),
    )
)]
async fn propose_alternative_time(
    State(_state): State<AppState>,
    principal: RequestPrincipal,
    Path(_job_id): Path<Uuid>,
    Json(_request): Json<db::models::ProposeAlternativeTime>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    require_vendor_principal(&principal)?;
    Err(not_implemented("Propose alternative time"))
}

// ==================== Property Access Handlers (Story 78.2) ====================

/// Get property access information for a job.
#[utoipa::path(
    get,
    path = "/api/v1/vendor-portal/jobs/{job_id}/access",
    tag = "Vendor Portal",
    params(
        ("job_id" = Uuid, Path, description = "Job ID")
    ),
    responses(
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a staff principal"),
        (status = 501, description = "Not implemented (no access-code store)"),
    )
)]
async fn get_access_info(
    State(_state): State<AppState>,
    principal: RequestPrincipal,
    Path(_job_id): Path<Uuid>,
) -> Result<Json<PropertyAccessInfo>, (StatusCode, Json<ErrorResponse>)> {
    require_vendor_principal(&principal)?;
    Err(not_implemented("Property access information"))
}

/// Generate temporary access code.
#[utoipa::path(
    post,
    path = "/api/v1/vendor-portal/jobs/{job_id}/access/generate-code",
    tag = "Vendor Portal",
    params(
        ("job_id" = Uuid, Path, description = "Job ID")
    ),
    request_body = GenerateAccessCode,
    responses(
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a staff principal"),
        (status = 501, description = "Not implemented (no access-code store)"),
    )
)]
async fn generate_access_code(
    State(_state): State<AppState>,
    principal: RequestPrincipal,
    Path(_job_id): Path<Uuid>,
    Json(_request): Json<GenerateAccessCode>,
) -> Result<Json<AccessCodeResponse>, (StatusCode, Json<ErrorResponse>)> {
    require_vendor_principal(&principal)?;
    Err(not_implemented("Access code generation"))
}

// ==================== Work Completion Handlers (Story 78.3) ====================

/// Submit work completion for a job.
#[utoipa::path(
    post,
    path = "/api/v1/vendor-portal/jobs/{job_id}/complete",
    tag = "Vendor Portal",
    params(
        ("job_id" = Uuid, Path, description = "Job ID")
    ),
    request_body = SubmitWorkCompletion,
    responses(
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a staff principal"),
        (status = 501, description = "Not implemented (no work-completion store)"),
    )
)]
async fn submit_work_completion(
    State(_state): State<AppState>,
    principal: RequestPrincipal,
    Path(_job_id): Path<Uuid>,
    Json(_request): Json<SubmitWorkCompletion>,
) -> Result<Json<WorkCompletion>, (StatusCode, Json<ErrorResponse>)> {
    require_vendor_principal(&principal)?;
    Err(not_implemented("Work completion submission"))
}

/// Get work completion details for a job.
#[utoipa::path(
    get,
    path = "/api/v1/vendor-portal/jobs/{job_id}/completion",
    tag = "Vendor Portal",
    params(
        ("job_id" = Uuid, Path, description = "Job ID")
    ),
    responses(
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a staff principal"),
        (status = 501, description = "Not implemented (no work-completion store)"),
    )
)]
async fn get_work_completion(
    State(_state): State<AppState>,
    principal: RequestPrincipal,
    Path(_job_id): Path<Uuid>,
) -> Result<Json<WorkCompletion>, (StatusCode, Json<ErrorResponse>)> {
    require_vendor_principal(&principal)?;
    Err(not_implemented("Work completion details"))
}

/// List vendor invoices.
#[utoipa::path(
    get,
    path = "/api/v1/vendor-portal/invoices",
    tag = "Vendor Portal",
    params(InvoiceQuery),
    responses(
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a staff principal"),
        (status = 501, description = "Not implemented (no vendor-identity schema)"),
    )
)]
async fn list_invoices(
    State(_state): State<AppState>,
    principal: RequestPrincipal,
    Query(_query): Query<InvoiceQuery>,
) -> Result<Json<Vec<VendorInvoiceWithTracking>>, (StatusCode, Json<ErrorResponse>)> {
    require_vendor_principal(&principal)?;
    Err(not_implemented("Vendor invoice listing"))
}

// ==================== Profile & Feedback Handlers (Story 78.4) ====================

/// Get vendor profile.
#[utoipa::path(
    get,
    path = "/api/v1/vendor-portal/profile",
    tag = "Vendor Portal",
    responses(
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a staff principal"),
        (status = 501, description = "Not implemented (no vendor-identity schema)"),
    )
)]
async fn get_profile(
    State(_state): State<AppState>,
    principal: RequestPrincipal,
) -> Result<Json<VendorProfile>, (StatusCode, Json<ErrorResponse>)> {
    require_vendor_principal(&principal)?;
    Err(not_implemented("Vendor profile"))
}

/// List feedback for vendor.
#[utoipa::path(
    get,
    path = "/api/v1/vendor-portal/feedback",
    tag = "Vendor Portal",
    params(FeedbackQuery),
    responses(
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a staff principal"),
        (status = 501, description = "Not implemented (no vendor-identity schema)"),
    )
)]
async fn list_feedback(
    State(_state): State<AppState>,
    principal: RequestPrincipal,
    Query(_query): Query<FeedbackQuery>,
) -> Result<Json<Vec<VendorFeedback>>, (StatusCode, Json<ErrorResponse>)> {
    require_vendor_principal(&principal)?;
    Err(not_implemented("Vendor feedback listing"))
}

/// Get vendor earnings summary.
#[utoipa::path(
    get,
    path = "/api/v1/vendor-portal/earnings",
    tag = "Vendor Portal",
    params(EarningsQuery),
    responses(
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a staff principal"),
        (status = 501, description = "Not implemented (no vendor-identity schema)"),
    )
)]
async fn get_earnings_summary(
    State(_state): State<AppState>,
    principal: RequestPrincipal,
    Query(_query): Query<EarningsQuery>,
) -> Result<Json<VendorEarningsSummary>, (StatusCode, Json<ErrorResponse>)> {
    require_vendor_principal(&principal)?;
    Err(not_implemented("Vendor earnings summary"))
}
