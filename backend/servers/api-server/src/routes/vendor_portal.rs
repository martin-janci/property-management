//! Vendor Operations Portal routes (Epic 78).
//!
//! Provides vendor-facing endpoints for job management, property access,
//! work completion, invoicing, and performance tracking.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use common::{errors::ErrorResponse, TenantContext};
use db::models::{
    AcceptJobRequest, AccessCodeResponse, DeclineJobRequest, GenerateAccessCode,
    PropertyAccessInfo, SubmitWorkCompletion, VendorDashboardStats, VendorEarningsSummary,
    VendorFeedback, VendorInvoiceWithTracking, VendorJob, VendorJobQuery, VendorJobSummary,
    VendorProfile, WorkCompletion,
};
use rust_decimal::Decimal;
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::state::AppState;

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract vendor context from request headers.
/// Vendors must be authenticated and have vendor role.
fn extract_vendor_context(
    headers: &HeaderMap,
) -> Result<TenantContext, (StatusCode, Json<ErrorResponse>)> {
    let tenant_header = headers
        .get("X-Tenant-Context")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "MISSING_CONTEXT",
                    "Vendor authentication required",
                )),
            )
        })?;

    let context: TenantContext = serde_json::from_str(tenant_header).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_CONTEXT",
                "Invalid vendor context format",
            )),
        )
    })?;

    // Verify the user has vendor role
    // In a full implementation, this would check context.role == TenantRole::Vendor
    // For now, we accept any authenticated user but this should be restricted
    Ok(context)
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
        (status = 200, description = "Dashboard statistics", body = VendorDashboardStats),
        (status = 401, description = "Unauthorized"),
    )
)]
async fn get_dashboard_stats(
    State(_state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<VendorDashboardStats>, (StatusCode, Json<ErrorResponse>)> {
    let _context = extract_vendor_context(&headers)?;

    let stats = VendorDashboardStats {
        today_jobs: 3,
        upcoming_jobs: 12,
        pending_action_jobs: 2,
        completed_this_month: 28,
        total_earnings_this_month: Decimal::new(450000, 2),
        average_rating: Some(Decimal::new(47, 1)),
    };

    Ok(Json(stats))
}

// ==================== Job Handlers (Story 78.1) ====================

/// List jobs for vendor.
#[utoipa::path(
    get,
    path = "/api/v1/vendor-portal/jobs",
    tag = "Vendor Portal",
    params(VendorJobQuery),
    responses(
        (status = 200, description = "List of jobs", body = Vec<VendorJobSummary>),
        (status = 401, description = "Unauthorized"),
    )
)]
async fn list_jobs(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Query(_query): Query<VendorJobQuery>,
) -> Result<Json<Vec<VendorJobSummary>>, (StatusCode, Json<ErrorResponse>)> {
    let _context = extract_vendor_context(&headers)?;
    let jobs = vec![
        VendorJobSummary {
            id: Uuid::new_v4(),
            work_order_id: Uuid::new_v4(),
            title: "Fix leaky faucet".to_string(),
            building_name: "Sunset Apartments".to_string(),
            unit_number: Some("304".to_string()),
            scheduled_date: Some(Utc::now().date_naive()),
            status: "scheduled".to_string(),
            priority: "medium".to_string(),
            service_type: "plumbing".to_string(),
        },
        VendorJobSummary {
            id: Uuid::new_v4(),
            work_order_id: Uuid::new_v4(),
            title: "HVAC maintenance".to_string(),
            building_name: "Downtown Tower".to_string(),
            unit_number: None,
            scheduled_date: Some(Utc::now().date_naive()),
            status: "pending".to_string(),
            priority: "high".to_string(),
            service_type: "hvac".to_string(),
        },
    ];

    Ok(Json(jobs))
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
        (status = 200, description = "Job details", body = VendorJob),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Job not found"),
    )
)]
async fn get_job_details(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Path(job_id): Path<Uuid>,
) -> Result<Json<VendorJob>, (StatusCode, Json<ErrorResponse>)> {
    let _context = extract_vendor_context(&headers)?;
    let job = VendorJob {
        id: job_id,
        work_order_id: Uuid::new_v4(),
        vendor_id: Uuid::new_v4(),
        building_id: Uuid::new_v4(),
        unit_id: Some(Uuid::new_v4()),
        title: "Fix leaky faucet".to_string(),
        description: Some("Kitchen faucet is dripping constantly".to_string()),
        scheduled_date: Some(Utc::now().date_naive()),
        scheduled_time: Some("10:00".to_string()),
        estimated_duration_hours: Some(Decimal::new(2, 0)),
        status: "scheduled".to_string(),
        priority: "medium".to_string(),
        service_type: "plumbing".to_string(),
        building_name: "Sunset Apartments".to_string(),
        building_address: "123 Sunset Blvd".to_string(),
        unit_number: Some("304".to_string()),
        contact_name: Some("John Smith".to_string()),
        contact_phone: Some("+1-555-123-4567".to_string()),
        special_instructions: Some("Ring doorbell twice".to_string()),
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
    };

    Ok(Json(job))
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
        (status = 200, description = "Job accepted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Job not found"),
        (status = 409, description = "Job already accepted or declined"),
    )
)]
async fn accept_job(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Path(_job_id): Path<Uuid>,
    Json(_request): Json<AcceptJobRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let _context = extract_vendor_context(&headers)?;
    Ok(StatusCode::OK)
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
        (status = 200, description = "Job declined"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Job not found"),
        (status = 409, description = "Job already accepted or completed"),
    )
)]
async fn decline_job(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Path(_job_id): Path<Uuid>,
    Json(_request): Json<DeclineJobRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let _context = extract_vendor_context(&headers)?;
    Ok(StatusCode::OK)
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
        (status = 200, description = "Alternative time proposed"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Job not found"),
    )
)]
async fn propose_alternative_time(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Path(_job_id): Path<Uuid>,
    Json(_request): Json<db::models::ProposeAlternativeTime>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let _context = extract_vendor_context(&headers)?;
    Ok(StatusCode::OK)
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
        (status = 200, description = "Access information", body = PropertyAccessInfo),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Job not found"),
    )
)]
async fn get_access_info(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Path(job_id): Path<Uuid>,
) -> Result<Json<PropertyAccessInfo>, (StatusCode, Json<ErrorResponse>)> {
    let _context = extract_vendor_context(&headers)?;
    let access_info = PropertyAccessInfo {
        job_id,
        building_id: Uuid::new_v4(),
        unit_id: Some(Uuid::new_v4()),
        access_method: "key_box".to_string(),
        access_code: Some("1234".to_string()),
        key_box_location: Some("Next to main entrance, behind planter".to_string()),
        smart_lock_info: None,
        contact_name: Some("John Smith".to_string()),
        contact_phone: Some("+1-555-123-4567".to_string()),
        special_instructions: Some("Call before entering".to_string()),
        access_valid_from: Some(Utc::now()),
        access_valid_until: Some(Utc::now() + chrono::Duration::hours(8)),
    };

    Ok(Json(access_info))
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
        (status = 200, description = "Access code generated", body = AccessCodeResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Job not found"),
    )
)]
async fn generate_access_code(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Path(_job_id): Path<Uuid>,
    Json(request): Json<GenerateAccessCode>,
) -> Result<Json<AccessCodeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let _context = extract_vendor_context(&headers)?;
    let now = Utc::now();
    let response = AccessCodeResponse {
        code: "847291".to_string(),
        valid_from: now,
        valid_until: now + chrono::Duration::hours(request.valid_hours as i64),
    };

    Ok(Json(response))
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
        (status = 200, description = "Work completion submitted", body = WorkCompletion),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Job not found"),
        (status = 409, description = "Job already completed"),
    )
)]
async fn submit_work_completion(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Path(job_id): Path<Uuid>,
    Json(request): Json<SubmitWorkCompletion>,
) -> Result<Json<WorkCompletion>, (StatusCode, Json<ErrorResponse>)> {
    let _context = extract_vendor_context(&headers)?;
    let materials_total: Decimal = request.materials_used.iter().map(|m| m.total_cost).sum();

    let completion = WorkCompletion {
        job_id,
        completed_at: Utc::now(),
        before_photos: request.before_photos,
        after_photos: request.after_photos,
        time_spent_hours: request.time_spent_hours,
        materials_used: request.materials_used,
        notes: request.notes,
        labor_cost: request.labor_cost,
        materials_cost: materials_total,
        total_cost: request.labor_cost + materials_total,
    };

    Ok(Json(completion))
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
        (status = 200, description = "Work completion details", body = WorkCompletion),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Job or completion not found"),
    )
)]
async fn get_work_completion(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Path(job_id): Path<Uuid>,
) -> Result<Json<WorkCompletion>, (StatusCode, Json<ErrorResponse>)> {
    let _context = extract_vendor_context(&headers)?;
    let completion = WorkCompletion {
        job_id,
        completed_at: Utc::now(),
        before_photos: vec!["photo1.jpg".to_string()],
        after_photos: vec!["photo2.jpg".to_string()],
        time_spent_hours: Decimal::new(2, 0),
        materials_used: vec![],
        notes: Some("Replaced washer and tightened fittings".to_string()),
        labor_cost: Decimal::new(15000, 2),
        materials_cost: Decimal::new(2500, 2),
        total_cost: Decimal::new(17500, 2),
    };

    Ok(Json(completion))
}

/// List vendor invoices.
#[utoipa::path(
    get,
    path = "/api/v1/vendor-portal/invoices",
    tag = "Vendor Portal",
    params(InvoiceQuery),
    responses(
        (status = 200, description = "List of invoices", body = Vec<VendorInvoiceWithTracking>),
        (status = 401, description = "Unauthorized"),
    )
)]
async fn list_invoices(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Query(_query): Query<InvoiceQuery>,
) -> Result<Json<Vec<VendorInvoiceWithTracking>>, (StatusCode, Json<ErrorResponse>)> {
    let _context = extract_vendor_context(&headers)?;
    let invoices = vec![
        VendorInvoiceWithTracking {
            id: Uuid::new_v4(),
            invoice_number: "INV-2024-001".to_string(),
            job_id: Some(Uuid::new_v4()),
            job_title: Some("Fix leaky faucet".to_string()),
            invoice_date: Utc::now().date_naive(),
            due_date: Some(Utc::now().date_naive() + chrono::Duration::days(30)),
            total_amount: Decimal::new(17500, 2),
            paid_amount: Decimal::ZERO,
            status: "pending".to_string(),
            payment_expected_date: Some(Utc::now().date_naive() + chrono::Duration::days(14)),
        },
        VendorInvoiceWithTracking {
            id: Uuid::new_v4(),
            invoice_number: "INV-2024-002".to_string(),
            job_id: Some(Uuid::new_v4()),
            job_title: Some("HVAC maintenance".to_string()),
            invoice_date: Utc::now().date_naive() - chrono::Duration::days(15),
            due_date: Some(Utc::now().date_naive() + chrono::Duration::days(15)),
            total_amount: Decimal::new(45000, 2),
            paid_amount: Decimal::new(45000, 2),
            status: "paid".to_string(),
            payment_expected_date: None,
        },
    ];

    Ok(Json(invoices))
}

// ==================== Profile & Feedback Handlers (Story 78.4) ====================

/// Get vendor profile.
#[utoipa::path(
    get,
    path = "/api/v1/vendor-portal/profile",
    tag = "Vendor Portal",
    responses(
        (status = 200, description = "Vendor profile", body = VendorProfile),
        (status = 401, description = "Unauthorized"),
    )
)]
async fn get_profile(
    State(_state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<VendorProfile>, (StatusCode, Json<ErrorResponse>)> {
    let _context = extract_vendor_context(&headers)?;
    let profile = VendorProfile {
        id: Uuid::new_v4(),
        company_name: "ABC Plumbing Services".to_string(),
        contact_name: Some("Mike Johnson".to_string()),
        phone: Some("+1-555-987-6543".to_string()),
        email: Some("mike@abcplumbing.com".to_string()),
        services: vec!["plumbing".to_string(), "hvac".to_string()],
        average_rating: Some(Decimal::new(47, 1)),
        quality_rating: Some(Decimal::new(48, 1)),
        timeliness_rating: Some(Decimal::new(46, 1)),
        communication_rating: Some(Decimal::new(49, 1)),
        total_jobs: 156,
        completed_jobs: 152,
        completion_rate: Some(Decimal::new(974, 1)),
        average_response_time_hours: Some(Decimal::new(25, 1)),
        badges: vec!["reliable".to_string(), "top_rated".to_string()],
        is_preferred: true,
        member_since: Some(Utc::now() - chrono::Duration::days(365 * 2)),
    };

    Ok(Json(profile))
}

/// List feedback for vendor.
#[utoipa::path(
    get,
    path = "/api/v1/vendor-portal/feedback",
    tag = "Vendor Portal",
    params(FeedbackQuery),
    responses(
        (status = 200, description = "List of feedback", body = Vec<VendorFeedback>),
        (status = 401, description = "Unauthorized"),
    )
)]
async fn list_feedback(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Query(_query): Query<FeedbackQuery>,
) -> Result<Json<Vec<VendorFeedback>>, (StatusCode, Json<ErrorResponse>)> {
    let _context = extract_vendor_context(&headers)?;
    let feedback = vec![
        VendorFeedback {
            id: Uuid::new_v4(),
            job_id: Uuid::new_v4(),
            job_title: "Fix leaky faucet".to_string(),
            building_name: "Sunset Apartments".to_string(),
            rating: 5,
            quality_rating: Some(5),
            timeliness_rating: Some(5),
            communication_rating: Some(5),
            review_text: Some(
                "Excellent work! Fixed the issue quickly and professionally.".to_string(),
            ),
            reviewer_name: Some("Property Manager".to_string()),
            created_at: Utc::now() - chrono::Duration::days(7),
        },
        VendorFeedback {
            id: Uuid::new_v4(),
            job_id: Uuid::new_v4(),
            job_title: "HVAC maintenance".to_string(),
            building_name: "Downtown Tower".to_string(),
            rating: 4,
            quality_rating: Some(5),
            timeliness_rating: Some(4),
            communication_rating: Some(4),
            review_text: Some("Good service, arrived a bit late but did great work.".to_string()),
            reviewer_name: Some("Building Superintendent".to_string()),
            created_at: Utc::now() - chrono::Duration::days(14),
        },
    ];

    Ok(Json(feedback))
}

/// Get vendor earnings summary.
#[utoipa::path(
    get,
    path = "/api/v1/vendor-portal/earnings",
    tag = "Vendor Portal",
    params(EarningsQuery),
    responses(
        (status = 200, description = "Earnings summary", body = VendorEarningsSummary),
        (status = 401, description = "Unauthorized"),
    )
)]
async fn get_earnings_summary(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<EarningsQuery>,
) -> Result<Json<VendorEarningsSummary>, (StatusCode, Json<ErrorResponse>)> {
    let _context = extract_vendor_context(&headers)?;
    let months = query.period_months.unwrap_or(1);
    let today = Utc::now().date_naive();
    let period_start = today - chrono::Duration::days(months as i64 * 30);

    let summary = VendorEarningsSummary {
        period_start,
        period_end: today,
        total_jobs: 28,
        total_earnings: Decimal::new(450000, 2),
        paid_amount: Decimal::new(380000, 2),
        pending_amount: Decimal::new(70000, 2),
    };

    Ok(Json(summary))
}
