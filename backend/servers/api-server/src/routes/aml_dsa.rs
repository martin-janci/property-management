//! Advanced Compliance (AML/DSA) routes (Epic 67).
//!
//! Handles AML risk assessment, Enhanced Due Diligence, DSA transparency
//! reporting, and content moderation dashboard endpoints.

#![allow(clippy::type_complexity)]

use crate::state::AppState;
use api_core::extractors::{AuthUser, RequestPrincipal};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use common::TenantRole;
use db::models::compliance::{
    AmlAssessmentStatus, AmlRiskLevel, CreateAmlRiskAssessment, CreateEnhancedDueDiligence,
    CreateModerationCase, DsaReportStatus, EddStatus, ModeratedContentType, ModerationActionType,
    ModerationStatus, TakeModerationAction, ViolationType,
};
use db::models::{AuditAction, CreateAuditLog};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Create the AML/DSA compliance router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Story 67.1: AML Risk Assessment
        .route("/aml/assess", post(create_aml_assessment))
        .route("/aml/assessments", get(list_aml_assessments))
        .route("/aml/assessments/{id}", get(get_aml_assessment))
        .route("/aml/assessments/{id}/review", post(review_aml_assessment))
        .route("/aml/country-risks", get(get_country_risks))
        .route("/aml/thresholds", get(get_aml_thresholds))
        // Story 67.2: Enhanced Due Diligence
        .route("/edd", post(initiate_edd))
        .route("/edd/{id}", get(get_edd_record))
        .route("/edd/{id}/documents", post(upload_edd_document))
        .route(
            "/edd/{id}/documents/{doc_id}/verify",
            post(verify_edd_document),
        )
        .route("/edd/{id}/notes", post(add_edd_note))
        .route("/edd/{id}/complete", post(complete_edd))
        .route("/edd/pending", get(list_pending_edd))
        // Story 67.3: DSA Transparency Reports
        .route("/dsa/reports", get(list_dsa_reports))
        .route("/dsa/reports", post(generate_dsa_report))
        .route("/dsa/reports/{id}", get(get_dsa_report))
        .route("/dsa/reports/{id}/publish", post(publish_dsa_report))
        .route("/dsa/reports/{id}/download", get(download_dsa_report))
        .route("/dsa/metrics", get(get_dsa_metrics))
        // Story 67.4: Content Moderation Dashboard
        .route("/moderation/queue", get(get_moderation_queue))
        .route("/moderation/queue/stats", get(get_moderation_stats))
        .route("/moderation/cases/{id}", get(get_moderation_case))
        .route(
            "/moderation/cases/{id}/assign",
            post(assign_moderation_case),
        )
        .route(
            "/moderation/cases/{id}/action",
            post(take_moderation_action),
        )
        .route("/moderation/cases/{id}/appeal", post(file_appeal))
        .route("/moderation/cases/{id}/appeal/decide", post(decide_appeal))
        .route("/moderation/report", post(report_content))
        .route("/moderation/templates", get(get_action_templates))
}

// ============================================================================
// AUTH HELPERS
// ============================================================================

/// Check if user has compliance officer role or higher.
fn require_compliance_role(user: &AuthUser) -> Result<(), (StatusCode, String)> {
    match user.role {
        Some(TenantRole::SuperAdmin) | Some(TenantRole::PlatformAdmin) => Ok(()),
        _ => Err((
            StatusCode::FORBIDDEN,
            "This endpoint requires compliance officer privileges".to_string(),
        )),
    }
}

/// Restrict access to platform-operator compliance staff only.
///
/// SECURITY (PAP-46): DSA transparency reports are platform-wide — a period
/// rollup of whole-service moderation metrics with no `organization_id`. Only
/// the platform operator may read/generate/publish/download them. We gate on
/// `PrincipalKind::Platform` (resolved from the trusted `users.principal_kind`
/// column by the `RequestPrincipal` extractor), NOT on the JWT `TenantRole`: a
/// tenant-scoped admin must never reach these handlers even if their per-org
/// membership role happens to be SuperAdmin/PlatformAdmin. This mirrors the
/// `admin-core` platform-principal model and is a distinct guard from
/// `require_compliance_role`, which correctly gates the per-tenant AML/EDD
/// handlers and must not be weakened.
fn require_platform_compliance_role(
    principal: &RequestPrincipal,
) -> Result<(), (StatusCode, String)> {
    if principal.is_platform() {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            "This endpoint requires platform-operator compliance privileges".to_string(),
        ))
    }
}

/// Check if user has moderator role or higher.
fn require_moderator_role(user: &AuthUser) -> Result<(), (StatusCode, String)> {
    match user.role {
        Some(TenantRole::SuperAdmin)
        | Some(TenantRole::PlatformAdmin)
        | Some(TenantRole::Manager) => Ok(()),
        _ => Err((
            StatusCode::FORBIDDEN,
            "This endpoint requires moderator privileges".to_string(),
        )),
    }
}

/// Emit a compliance audit record for an AML/EDD/DSA decision or state change.
///
/// Best-effort: a logging failure must never turn a successful compliance
/// operation into a 500. We log the failure and continue, mirroring the
/// convention in `routes/listings.rs::global_publish`. The substantive
/// who/what/when (actor, tenant, resource, decision payload) lands in the
/// `audit_logs` table so AML record-keeping and EU DSA Art. 17
/// statement-of-reasons obligations (FR115) have a durable trail.
async fn write_compliance_audit(
    state: &AppState,
    user: &AuthUser,
    action: AuditAction,
    resource_type: &str,
    resource_id: Uuid,
    details: serde_json::Value,
) {
    if let Err(e) = state
        .audit_log_repo
        .create(CreateAuditLog {
            user_id: Some(user.user_id),
            action,
            resource_type: Some(resource_type.to_string()),
            resource_id: Some(resource_id),
            org_id: user.tenant_id,
            details: Some(details),
            old_values: None,
            new_values: None,
            ip_address: None,
            user_agent: None,
        })
        .await
    {
        tracing::warn!(
            error = %e,
            resource_type = resource_type,
            resource_id = %resource_id,
            "Failed to write compliance audit log"
        );
    }
}

// ============================================================================
// INPUT-VALIDATION HELPERS (PAP-44)
// ============================================================================
//
// Boundary validation for untrusted request input. These are deliberately
// small, pure functions so they can be unit-tested without a DB or HTTP layer.

/// Default page size when the client does not specify `limit`.
const DEFAULT_PAGE_LIMIT: i64 = 50;
/// Hard upper bound on `limit` to prevent abusive/accidental large scans.
const MAX_PAGE_LIMIT: i64 = 200;

/// Maximum size of an EDD document (50 MiB).
const MAX_EDD_DOCUMENT_BYTES: i64 = 50 * 1024 * 1024;

/// Allow-listed MIME types for EDD document uploads.
const ALLOWED_EDD_MIME_TYPES: &[&str] = &[
    "application/pdf",
    "image/png",
    "image/jpeg",
    "image/tiff",
    "image/webp",
    "application/msword",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
];

/// Maximum span of a DSA reporting period (~2 years).
const MAX_DSA_REPORT_RANGE_DAYS: i64 = 732;

/// Length caps for free-text fields.
const MAX_NOTE_LEN: usize = 10_000;
const MAX_RATIONALE_LEN: usize = 10_000;
const MAX_APPEAL_REASON_LEN: usize = 5_000;
const MAX_FILENAME_LEN: usize = 255;
const MAX_FILE_PATH_LEN: usize = 1024;

/// Clamp a client-supplied `limit` into `[1, MAX_PAGE_LIMIT]`, defaulting when absent.
fn clamp_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(DEFAULT_PAGE_LIMIT).clamp(1, MAX_PAGE_LIMIT)
}

/// Normalize a client-supplied `offset` to a non-negative value.
fn sanitize_offset(offset: Option<i64>) -> i64 {
    offset.unwrap_or(0).max(0)
}

/// Reject paths that are absolute, contain a parent-directory (`..`) component,
/// use Windows-style separators/drives, contain control characters, or are
/// empty — i.e. anything that could escape an intended storage root.
fn validate_storage_path(path: &str) -> Result<(), &'static str> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("must not be empty");
    }
    if trimmed.len() > MAX_FILE_PATH_LEN {
        return Err("is too long");
    }
    if trimmed.starts_with('/') || trimmed.starts_with('\\') {
        return Err("must be a relative path");
    }
    if trimmed.contains(':') || trimmed.contains('\\') {
        return Err("must not contain drive letters or backslashes");
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return Err("must not contain control characters");
    }
    if trimmed.split('/').any(|component| component == "..") {
        return Err("must not contain '..' components");
    }
    Ok(())
}

/// Validate an uploaded filename: non-empty, length-capped, no path separators.
fn validate_filename(name: &str) -> Result<(), &'static str> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("must not be empty");
    }
    if trimmed.len() > MAX_FILENAME_LEN {
        return Err("is too long");
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains('\0') {
        return Err("must not contain path separators");
    }
    if trimmed == "." || trimmed == ".." {
        return Err("is invalid");
    }
    Ok(())
}

/// Validate EDD document upload metadata: path traversal, MIME allow-list, size bounds.
fn validate_edd_document(req: &UploadEddDocumentRequest) -> Result<(), String> {
    validate_storage_path(&req.file_path).map_err(|e| format!("file_path {e}"))?;
    validate_filename(&req.original_filename).map_err(|e| format!("original_filename {e}"))?;
    if req.file_size_bytes <= 0 {
        return Err("file_size_bytes must be greater than 0".to_string());
    }
    if req.file_size_bytes > MAX_EDD_DOCUMENT_BYTES {
        return Err(format!(
            "file_size_bytes exceeds the maximum of {MAX_EDD_DOCUMENT_BYTES} bytes"
        ));
    }
    let mime = req.mime_type.trim().to_ascii_lowercase();
    if !ALLOWED_EDD_MIME_TYPES.contains(&mime.as_str()) {
        return Err(format!("mime_type '{}' is not allowed", req.mime_type));
    }
    Ok(())
}

/// Validate a DSA reporting period: end after start, not in the future, within max span.
fn validate_report_period(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<(), &'static str> {
    if end <= start {
        return Err("period_end must be after period_start");
    }
    if end > now {
        return Err("period_end must not be in the future");
    }
    if (end - start) > Duration::days(MAX_DSA_REPORT_RANGE_DAYS) {
        return Err("reporting period is too large");
    }
    Ok(())
}

/// Validate a required free-text field: non-empty (after trim) and within `max` chars.
fn validate_text_field(value: &str, max: usize, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    if value.chars().count() > max {
        return Err(format!(
            "{field} exceeds the maximum length of {max} characters"
        ));
    }
    Ok(())
}

/// Build a scoped, opaque download reference for a DSA report that never
/// discloses the internal filesystem path. Returns `None` when no file exists.
fn dsa_report_download_ref(report_id: Uuid, file_path: &Option<String>) -> Option<String> {
    file_path
        .as_ref()
        .map(|_| format!("/api/v1/aml-dsa/dsa/reports/{report_id}/download"))
}

// ============================================================================
// STORY 67.1: AML RISK ASSESSMENT
// ============================================================================

/// Request to create AML assessment.
#[derive(Debug, Deserialize)]
pub struct CreateAmlAssessmentRequest {
    /// Party to assess
    pub party_id: Uuid,
    /// Party type (individual, company)
    pub party_type: String,
    /// Transaction ID (if assessing a transaction)
    pub transaction_id: Option<Uuid>,
    /// Transaction amount in cents
    pub transaction_amount_cents: Option<i64>,
    /// Currency code
    pub currency: Option<String>,
    /// Country code
    pub country_code: Option<String>,
}

/// Risk factor in assessment.
#[derive(Debug, Serialize, Deserialize)]
pub struct RiskFactor {
    pub factor_type: String,
    pub description: String,
    pub weight: i32,
    pub mitigated: bool,
}

/// AML assessment response.
#[derive(Debug, Serialize)]
pub struct AmlAssessmentResponse {
    pub id: Uuid,
    pub party_id: Uuid,
    pub party_type: String,
    pub transaction_id: Option<Uuid>,
    pub transaction_amount_cents: Option<i64>,
    pub currency: Option<String>,
    pub risk_score: i32,
    pub risk_level: AmlRiskLevel,
    pub status: AmlAssessmentStatus,
    pub risk_factors: Vec<RiskFactor>,
    pub country_code: Option<String>,
    pub country_risk: Option<String>,
    pub id_verified: bool,
    pub source_of_funds_documented: bool,
    pub pep_check_completed: bool,
    pub is_pep: Option<bool>,
    pub sanctions_check_completed: bool,
    pub sanctions_match: Option<bool>,
    pub flagged_for_review: bool,
    pub review_reason: Option<String>,
    pub recommendations: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub assessed_at: Option<DateTime<Utc>>,
}

/// Create a new AML risk assessment.
async fn create_aml_assessment(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateAmlAssessmentRequest>,
) -> Result<Json<AmlAssessmentResponse>, (StatusCode, String)> {
    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    let create_req = CreateAmlRiskAssessment {
        organization_id: org_id,
        transaction_id: req.transaction_id,
        party_id: req.party_id,
        party_type: req.party_type.clone(),
        transaction_amount_cents: req.transaction_amount_cents,
        currency: req.currency.clone(),
        country_code: req.country_code.clone(),
    };

    let assessment = state
        .edd_repo
        .create_aml_assessment(create_req)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create AML assessment: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create AML assessment".to_string(),
            )
        })?;

    // Parse risk factors from JSON
    let risk_factors: Vec<RiskFactor> = assessment
        .risk_factors
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    // Generate recommendations based on risk level
    let mut recommendations = Vec::new();
    if assessment.flagged_for_review {
        recommendations.push("Initiate Enhanced Due Diligence (EDD) process".to_string());
        recommendations.push("Verify source of funds documentation".to_string());
    }
    if matches!(
        assessment.risk_level,
        AmlRiskLevel::High | AmlRiskLevel::Critical
    ) {
        recommendations.push("Request additional identification documents".to_string());
        recommendations.push("Conduct PEP screening".to_string());
    }

    let country_risk_str = assessment.country_risk.map(|r| r.to_string());

    Ok(Json(AmlAssessmentResponse {
        id: assessment.id,
        party_id: assessment.party_id,
        party_type: assessment.party_type,
        transaction_id: assessment.transaction_id,
        transaction_amount_cents: assessment.transaction_amount_cents,
        currency: assessment.currency,
        risk_score: assessment.risk_score,
        risk_level: assessment.risk_level,
        status: assessment.status,
        risk_factors,
        country_code: assessment.country_code,
        country_risk: country_risk_str,
        id_verified: assessment.id_verified,
        source_of_funds_documented: assessment.source_of_funds_documented,
        pep_check_completed: assessment.pep_check_completed,
        is_pep: assessment.is_pep,
        sanctions_check_completed: assessment.sanctions_check_completed,
        sanctions_match: assessment.sanctions_match,
        flagged_for_review: assessment.flagged_for_review,
        review_reason: assessment.review_reason,
        recommendations,
        created_at: assessment.created_at,
        assessed_at: assessment.assessed_at,
    }))
}

/// Query parameters for listing assessments.
#[derive(Debug, Deserialize)]
pub struct ListAmlAssessmentsQuery {
    pub status: Option<AmlAssessmentStatus>,
    pub risk_level: Option<AmlRiskLevel>,
    pub flagged_only: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// List AML assessments.
async fn list_aml_assessments(
    State(state): State<AppState>,
    user: AuthUser,
    Query(params): Query<ListAmlAssessmentsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    require_compliance_role(&user)?;

    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    let limit = clamp_limit(params.limit);
    let offset = sanitize_offset(params.offset);
    let flagged_only = params.flagged_only.unwrap_or(false);

    let (assessments, total) = state
        .edd_repo
        .list_aml_assessments(
            org_id,
            params.status,
            params.risk_level,
            flagged_only,
            limit,
            offset,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to list AML assessments: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to list assessments".to_string(),
            )
        })?;

    Ok(Json(serde_json::json!({
        "assessments": assessments,
        "total": total,
        "limit": limit,
        "offset": offset
    })))
}

/// Get a specific AML assessment.
async fn get_aml_assessment(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<AmlAssessmentResponse>, (StatusCode, String)> {
    require_compliance_role(&user)?;

    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    let assessment = state
        .edd_repo
        .get_aml_assessment(id, org_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get AML assessment: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get assessment".to_string(),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            format!("Assessment {} not found", id),
        ))?;

    let risk_factors: Vec<RiskFactor> = assessment
        .risk_factors
        .clone()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    let country_risk_str = assessment.country_risk.map(|r| r.to_string());

    let mut recommendations = Vec::new();
    if assessment.flagged_for_review {
        recommendations.push("Initiate Enhanced Due Diligence (EDD) process".to_string());
    }

    Ok(Json(AmlAssessmentResponse {
        id: assessment.id,
        party_id: assessment.party_id,
        party_type: assessment.party_type,
        transaction_id: assessment.transaction_id,
        transaction_amount_cents: assessment.transaction_amount_cents,
        currency: assessment.currency,
        risk_score: assessment.risk_score,
        risk_level: assessment.risk_level,
        status: assessment.status,
        risk_factors,
        country_code: assessment.country_code,
        country_risk: country_risk_str,
        id_verified: assessment.id_verified,
        source_of_funds_documented: assessment.source_of_funds_documented,
        pep_check_completed: assessment.pep_check_completed,
        is_pep: assessment.is_pep,
        sanctions_check_completed: assessment.sanctions_check_completed,
        sanctions_match: assessment.sanctions_match,
        flagged_for_review: assessment.flagged_for_review,
        review_reason: assessment.review_reason,
        recommendations,
        created_at: assessment.created_at,
        assessed_at: assessment.assessed_at,
    }))
}

/// Request to review an assessment.
#[derive(Debug, Deserialize)]
pub struct ReviewAmlAssessmentRequest {
    pub decision: String, // approved, rejected
    pub notes: Option<String>,
}

/// Review an AML assessment.
async fn review_aml_assessment(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ReviewAmlAssessmentRequest>,
) -> Result<Json<AmlAssessmentResponse>, (StatusCode, String)> {
    require_compliance_role(&user)?;

    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    let decision = match req.decision.to_lowercase().as_str() {
        "approved" => AmlAssessmentStatus::Approved,
        "rejected" => AmlAssessmentStatus::Rejected,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Decision must be 'approved' or 'rejected'".to_string(),
            ))
        }
    };

    let assessment = state
        .edd_repo
        .review_aml_assessment(id, org_id, user.user_id, decision, req.notes.as_deref())
        .await
        .map_err(|e| {
            tracing::error!("Failed to review AML assessment: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to review assessment".to_string(),
            )
        })?;

    write_compliance_audit(
        &state,
        &user,
        AuditAction::ResourceUpdated,
        "aml_assessment",
        assessment.id,
        serde_json::json!({
            "operation": "review_aml_assessment",
            "decision": req.decision.to_lowercase(),
            "resulting_status": assessment.status,
            "notes_provided": req.notes.is_some(),
        }),
    )
    .await;

    let risk_factors: Vec<RiskFactor> = assessment
        .risk_factors
        .clone()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    let country_risk_str = assessment.country_risk.map(|r| r.to_string());

    Ok(Json(AmlAssessmentResponse {
        id: assessment.id,
        party_id: assessment.party_id,
        party_type: assessment.party_type,
        transaction_id: assessment.transaction_id,
        transaction_amount_cents: assessment.transaction_amount_cents,
        currency: assessment.currency,
        risk_score: assessment.risk_score,
        risk_level: assessment.risk_level,
        status: assessment.status,
        risk_factors,
        country_code: assessment.country_code,
        country_risk: country_risk_str,
        id_verified: assessment.id_verified,
        source_of_funds_documented: assessment.source_of_funds_documented,
        pep_check_completed: assessment.pep_check_completed,
        is_pep: assessment.is_pep,
        sanctions_check_completed: assessment.sanctions_check_completed,
        sanctions_match: assessment.sanctions_match,
        flagged_for_review: assessment.flagged_for_review,
        review_reason: assessment.review_reason,
        recommendations: vec![],
        created_at: assessment.created_at,
        assessed_at: assessment.assessed_at,
    }))
}

/// Country risk entry.
#[derive(Debug, Serialize)]
pub struct CountryRiskEntry {
    pub country_code: String,
    pub country_name: String,
    pub risk_rating: String,
    pub is_sanctioned: bool,
    pub fatf_status: Option<String>,
}

/// Get country risk database.
async fn get_country_risks(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<CountryRiskEntry>>, (StatusCode, String)> {
    require_compliance_role(&user)?;

    let risks = state.edd_repo.list_country_risks().await.map_err(|e| {
        tracing::error!("Failed to get country risks: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to get country risks".to_string(),
        )
    })?;

    let entries: Vec<CountryRiskEntry> = risks
        .into_iter()
        .map(|r| CountryRiskEntry {
            country_code: r.country_code,
            country_name: r.country_name,
            risk_rating: r.risk_rating.to_string(),
            is_sanctioned: r.is_sanctioned,
            fatf_status: r.fatf_status,
        })
        .collect();

    Ok(Json(entries))
}

/// AML thresholds response.
#[derive(Debug, Serialize)]
pub struct AmlThresholdsResponse {
    pub transaction_threshold_eur: i64,
    pub transaction_threshold_cents: i64,
    pub cumulative_threshold_eur: i64,
    pub review_threshold_score: i32,
}

/// Get AML thresholds configuration.
async fn get_aml_thresholds(
    State(_state): State<AppState>,
    user: AuthUser,
) -> Result<Json<AmlThresholdsResponse>, (StatusCode, String)> {
    require_compliance_role(&user)?;

    Ok(Json(AmlThresholdsResponse {
        transaction_threshold_eur: 10_000,
        transaction_threshold_cents: 1_000_000,
        cumulative_threshold_eur: 15_000,
        review_threshold_score: 50,
    }))
}

// ============================================================================
// STORY 67.2: ENHANCED DUE DILIGENCE
// ============================================================================

/// Request to initiate EDD.
#[derive(Debug, Deserialize)]
pub struct InitiateEddRequest {
    pub aml_assessment_id: Uuid,
    pub party_id: Uuid,
    pub documents_requested: Vec<String>,
}

/// EDD record response.
#[derive(Debug, Serialize)]
pub struct EddRecordResponse {
    pub id: Uuid,
    pub aml_assessment_id: Uuid,
    pub party_id: Uuid,
    pub status: EddStatus,
    pub source_of_wealth: Option<String>,
    pub source_of_funds: Option<String>,
    pub beneficial_ownership: Option<serde_json::Value>,
    pub documents_requested: Vec<String>,
    pub documents_received: Vec<EddDocumentResponse>,
    pub compliance_notes: Vec<ComplianceNoteResponse>,
    pub initiated_at: DateTime<Utc>,
    pub initiated_by: Uuid,
    pub completed_at: Option<DateTime<Utc>>,
    pub next_review_date: Option<DateTime<Utc>>,
}

/// EDD document response.
#[derive(Debug, Serialize)]
pub struct EddDocumentResponse {
    pub id: Uuid,
    pub document_type: String,
    pub original_filename: String,
    pub verification_status: String,
    pub verified_at: Option<DateTime<Utc>>,
    pub expiry_date: Option<DateTime<Utc>>,
    pub uploaded_at: DateTime<Utc>,
}

/// Compliance note response.
#[derive(Debug, Serialize)]
pub struct ComplianceNoteResponse {
    pub id: Uuid,
    pub content: String,
    pub added_by_name: String,
    pub added_at: DateTime<Utc>,
}

/// Initiate Enhanced Due Diligence.
async fn initiate_edd(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<InitiateEddRequest>,
) -> Result<Json<EddRecordResponse>, (StatusCode, String)> {
    require_compliance_role(&user)?;

    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    // NOTE (PAP-44): ownership verification of `aml_assessment_id` / `party_id`
    // against the caller's org is deferred to the IDOR/authz child PAP-38, which
    // introduces the shared ownership-check helpers this should reuse.

    let create_req = CreateEnhancedDueDiligence {
        aml_assessment_id: req.aml_assessment_id,
        organization_id: org_id,
        party_id: req.party_id,
        initiated_by: user.user_id,
        documents_requested: Some(req.documents_requested.clone()),
    };

    let edd = state.edd_repo.create_edd(create_req).await.map_err(|e| {
        tracing::error!("Failed to create EDD: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to initiate EDD".to_string(),
        )
    })?;

    write_compliance_audit(
        &state,
        &user,
        AuditAction::ResourceCreated,
        "enhanced_due_diligence",
        edd.id,
        serde_json::json!({
            "operation": "initiate_edd",
            "resulting_status": edd.status,
            "party_id": edd.party_id,
            "aml_assessment_id": edd.aml_assessment_id,
        }),
    )
    .await;

    let documents_requested: Vec<String> = edd
        .documents_requested
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    Ok(Json(EddRecordResponse {
        id: edd.id,
        aml_assessment_id: edd.aml_assessment_id,
        party_id: edd.party_id,
        status: edd.status,
        source_of_wealth: edd.source_of_wealth,
        source_of_funds: edd.source_of_funds,
        beneficial_ownership: edd.beneficial_ownership,
        documents_requested,
        documents_received: vec![],
        compliance_notes: vec![],
        initiated_at: edd.initiated_at,
        initiated_by: edd.initiated_by,
        completed_at: edd.completed_at,
        next_review_date: edd.next_review_date,
    }))
}

/// Get EDD record by ID.
async fn get_edd_record(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<EddRecordResponse>, (StatusCode, String)> {
    require_compliance_role(&user)?;

    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    let edd = state
        .edd_repo
        .get_edd(id, org_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get EDD: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get EDD record".to_string(),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            format!("EDD record {} not found", id),
        ))?;

    // Get documents
    let docs = state.edd_repo.list_edd_documents(id).await.map_err(|e| {
        tracing::error!("Failed to list EDD documents: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to get EDD documents".to_string(),
        )
    })?;

    let documents_received: Vec<EddDocumentResponse> = docs
        .into_iter()
        .map(|d| EddDocumentResponse {
            id: d.id,
            document_type: d.document_type,
            original_filename: d.original_filename,
            verification_status: d.verification_status.to_string(),
            verified_at: d.verified_at,
            expiry_date: d.expiry_date,
            uploaded_at: d.uploaded_at,
        })
        .collect();

    // Get compliance notes
    let notes = state.edd_repo.get_compliance_notes(id).await.map_err(|e| {
        tracing::error!("Failed to get compliance notes: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to get compliance notes".to_string(),
        )
    })?;

    let compliance_notes: Vec<ComplianceNoteResponse> = notes
        .into_iter()
        .map(|n| ComplianceNoteResponse {
            id: n.id,
            content: n.content,
            added_by_name: n.added_by_name,
            added_at: n.added_at,
        })
        .collect();

    let documents_requested: Vec<String> = edd
        .documents_requested
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    Ok(Json(EddRecordResponse {
        id: edd.id,
        aml_assessment_id: edd.aml_assessment_id,
        party_id: edd.party_id,
        status: edd.status,
        source_of_wealth: edd.source_of_wealth,
        source_of_funds: edd.source_of_funds,
        beneficial_ownership: edd.beneficial_ownership,
        documents_requested,
        documents_received,
        compliance_notes,
        initiated_at: edd.initiated_at,
        initiated_by: edd.initiated_by,
        completed_at: edd.completed_at,
        next_review_date: edd.next_review_date,
    }))
}

/// Upload EDD document request (metadata only, actual file via multipart).
#[derive(Debug, Deserialize)]
pub struct UploadEddDocumentRequest {
    pub document_type: String,
    pub original_filename: String,
    pub file_path: String,
    pub file_size_bytes: i64,
    pub mime_type: String,
    pub expiry_date: Option<DateTime<Utc>>,
}

/// Upload a document for EDD.
async fn upload_edd_document(
    State(state): State<AppState>,
    user: AuthUser,
    Path(edd_id): Path<Uuid>,
    Json(req): Json<UploadEddDocumentRequest>,
) -> Result<Json<EddDocumentResponse>, (StatusCode, String)> {
    require_compliance_role(&user)?;

    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    // Validate client-supplied document metadata at the boundary: reject
    // path-traversal / absolute paths, spoofed MIME types, and bad sizes.
    validate_edd_document(&req).map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    // Verify the EDD record exists and belongs to the caller's organization
    let _edd = state
        .edd_repo
        .get_edd(edd_id, org_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get EDD: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to verify EDD record".to_string(),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            format!("EDD record {} not found", edd_id),
        ))?;

    let create_doc = db::models::compliance::CreateEddDocument {
        edd_id,
        document_type: req.document_type.clone(),
        file_path: req.file_path,
        original_filename: req.original_filename.clone(),
        file_size_bytes: req.file_size_bytes,
        mime_type: req.mime_type,
        uploaded_by: user.user_id,
        expiry_date: req.expiry_date,
    };

    let doc = state
        .edd_repo
        .upload_edd_document(create_doc)
        .await
        .map_err(|e| {
            tracing::error!("Failed to upload EDD document: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to upload document".to_string(),
            )
        })?;

    Ok(Json(EddDocumentResponse {
        id: doc.id,
        document_type: doc.document_type,
        original_filename: doc.original_filename,
        verification_status: doc.verification_status.to_string(),
        verified_at: doc.verified_at,
        expiry_date: doc.expiry_date,
        uploaded_at: doc.uploaded_at,
    }))
}

/// Verify document request.
#[derive(Debug, Deserialize)]
pub struct VerifyDocumentRequest {
    pub status: String, // verified, rejected
    pub rejection_reason: Option<String>,
}

/// Verify an EDD document.
async fn verify_edd_document(
    State(state): State<AppState>,
    user: AuthUser,
    Path((edd_id, doc_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<VerifyDocumentRequest>,
) -> Result<Json<EddDocumentResponse>, (StatusCode, String)> {
    require_compliance_role(&user)?;

    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    // Verify the EDD record exists and belongs to the caller's organization
    let _edd = state
        .edd_repo
        .get_edd(edd_id, org_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get EDD: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to verify EDD record".to_string(),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            format!("EDD record {} not found", edd_id),
        ))?;

    let status = match req.status.to_lowercase().as_str() {
        "verified" => db::models::compliance::DocumentVerificationStatus::Verified,
        "rejected" => db::models::compliance::DocumentVerificationStatus::Rejected,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Status must be 'verified' or 'rejected'".to_string(),
            ))
        }
    };

    let doc = state
        .edd_repo
        .verify_edd_document(
            doc_id,
            edd_id,
            user.user_id,
            status,
            req.rejection_reason.as_deref(),
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to verify EDD document: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to verify document".to_string(),
            )
        })?;

    write_compliance_audit(
        &state,
        &user,
        AuditAction::ResourceUpdated,
        "edd_document",
        doc.id,
        serde_json::json!({
            "operation": "verify_edd_document",
            "edd_id": edd_id,
            "verification_status": req.status.to_lowercase(),
            "rejection_provided": req.rejection_reason.is_some(),
        }),
    )
    .await;

    Ok(Json(EddDocumentResponse {
        id: doc.id,
        document_type: doc.document_type,
        original_filename: doc.original_filename,
        verification_status: doc.verification_status.to_string(),
        verified_at: doc.verified_at,
        expiry_date: doc.expiry_date,
        uploaded_at: doc.uploaded_at,
    }))
}

/// Add compliance note request.
#[derive(Debug, Deserialize)]
pub struct AddComplianceNoteRequest {
    pub content: String,
}

/// Add a compliance note to EDD record.
async fn add_edd_note(
    State(state): State<AppState>,
    user: AuthUser,
    Path(edd_id): Path<Uuid>,
    Json(req): Json<AddComplianceNoteRequest>,
) -> Result<Json<ComplianceNoteResponse>, (StatusCode, String)> {
    require_compliance_role(&user)?;

    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    validate_text_field(&req.content, MAX_NOTE_LEN, "content")
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    // Verify the EDD record exists and belongs to the caller's organization
    state
        .edd_repo
        .get_edd(edd_id, org_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get EDD: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to verify EDD record".to_string(),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            format!("EDD record {} not found", edd_id),
        ))?;

    // Get user name for the note
    let user_info = state
        .user_repo
        .find_by_id(user.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get user: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get user info".to_string(),
            )
        })?;

    let user_name = user_info
        .map(|u| u.name.clone())
        .unwrap_or_else(|| "Unknown User".to_string());

    let note_req = db::models::compliance::AddComplianceNote {
        content: req.content,
    };

    let note = state
        .edd_repo
        .add_compliance_note(edd_id, note_req, user.user_id, &user_name)
        .await
        .map_err(|e| {
            tracing::error!("Failed to add compliance note: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to add note".to_string(),
            )
        })?;

    Ok(Json(ComplianceNoteResponse {
        id: note.id,
        content: note.content,
        added_by_name: note.added_by_name,
        added_at: note.added_at,
    }))
}

/// Complete EDD process.
async fn complete_edd(
    State(state): State<AppState>,
    user: AuthUser,
    Path(edd_id): Path<Uuid>,
) -> Result<Json<EddRecordResponse>, (StatusCode, String)> {
    require_compliance_role(&user)?;

    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    // Verify the EDD record exists and belongs to the caller's organization
    state
        .edd_repo
        .get_edd(edd_id, org_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get EDD: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to verify EDD record".to_string(),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            format!("EDD record {} not found", edd_id),
        ))?;

    let edd = state
        .edd_repo
        .complete_edd(edd_id, user.user_id, None)
        .await
        .map_err(|e| {
            tracing::error!("Failed to complete EDD: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to complete EDD".to_string(),
            )
        })?;

    write_compliance_audit(
        &state,
        &user,
        AuditAction::ResourceUpdated,
        "enhanced_due_diligence",
        edd.id,
        serde_json::json!({
            "operation": "complete_edd",
            "resulting_status": edd.status,
        }),
    )
    .await;

    let documents_requested: Vec<String> = edd
        .documents_requested
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    Ok(Json(EddRecordResponse {
        id: edd.id,
        aml_assessment_id: edd.aml_assessment_id,
        party_id: edd.party_id,
        status: edd.status,
        source_of_wealth: edd.source_of_wealth,
        source_of_funds: edd.source_of_funds,
        beneficial_ownership: edd.beneficial_ownership,
        documents_requested,
        documents_received: vec![],
        compliance_notes: vec![],
        initiated_at: edd.initiated_at,
        initiated_by: edd.initiated_by,
        completed_at: edd.completed_at,
        next_review_date: edd.next_review_date,
    }))
}

/// List pending EDD records.
async fn list_pending_edd(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<EddRecordResponse>>, (StatusCode, String)> {
    require_compliance_role(&user)?;

    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    let edds = state.edd_repo.list_pending_edd(org_id).await.map_err(|e| {
        tracing::error!("Failed to list pending EDD: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to list pending EDD".to_string(),
        )
    })?;

    let responses: Vec<EddRecordResponse> = edds
        .into_iter()
        .map(|edd| {
            let documents_requested: Vec<String> = edd
                .documents_requested
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default();

            EddRecordResponse {
                id: edd.id,
                aml_assessment_id: edd.aml_assessment_id,
                party_id: edd.party_id,
                status: edd.status,
                source_of_wealth: edd.source_of_wealth,
                source_of_funds: edd.source_of_funds,
                beneficial_ownership: edd.beneficial_ownership,
                documents_requested,
                documents_received: vec![],
                compliance_notes: vec![],
                initiated_at: edd.initiated_at,
                initiated_by: edd.initiated_by,
                completed_at: edd.completed_at,
                next_review_date: edd.next_review_date,
            }
        })
        .collect();

    Ok(Json(responses))
}

// ============================================================================
// STORY 67.3: DSA TRANSPARENCY REPORTS
// ============================================================================

/// DSA report summary statistics.
#[derive(Debug, Serialize)]
pub struct DsaReportSummary {
    pub total_moderation_actions: i64,
    pub content_removed: i64,
    pub content_restricted: i64,
    pub warnings_issued: i64,
    pub user_reports_received: i64,
    pub user_reports_resolved: i64,
    pub avg_resolution_time_hours: Option<f64>,
    pub automated_decisions: i64,
    pub automated_decisions_overturned: i64,
    pub appeals_received: i64,
    pub appeals_upheld: i64,
    pub appeals_rejected: i64,
}

/// Content type count.
#[derive(Debug, Serialize)]
pub struct ContentTypeCount {
    pub content_type: String,
    pub count: i64,
}

/// Violation type count.
#[derive(Debug, Serialize)]
pub struct ViolationTypeCountResponse {
    pub violation_type: String,
    pub count: i64,
}

/// DSA transparency report response.
#[derive(Debug, Serialize)]
pub struct DsaTransparencyReportResponse {
    pub id: Uuid,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub status: DsaReportStatus,
    pub summary: DsaReportSummary,
    pub content_type_breakdown: Vec<ContentTypeCount>,
    pub violation_type_breakdown: Vec<ViolationTypeCountResponse>,
    pub download_url: Option<String>,
    pub generated_at: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
}

/// Request to generate DSA report.
#[derive(Debug, Deserialize)]
pub struct GenerateDsaReportRequest {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

/// Short-lived presigned download URL for a DSA report file.
#[derive(Debug, Serialize)]
pub struct DsaReportDownloadResponse {
    pub url: String,
    pub expires_at: DateTime<Utc>,
}

// NOTE: `dsa_report_download_ref` is defined once near the top of this module
// (added by PAP-44). PAP-47 landed an identical second definition here, which
// collided (E0428) once both were on `dev`; the duplicate has been removed.
// The shared helper builds the opaque `/api/v1/aml-dsa/dsa/reports/{id}/download`
// reference and never discloses the internal `report_file_path`.
// ----------------------------------------------------------------------------
// DSA transparency reports (Epic 67 / Story 67.3) — platform-VLOP model.
//
// Per the PAP-40 -> PAP-47 decision, DSA transparency reporting is a
// PLATFORM-LEVEL regulatory artifact (32bit is the regulated VLOP/intermediary
// under the EU DSA), not a per-tenant one. These reports are platform-wide by
// design: every handler below is gated to platform roles via
// `require_compliance_role` (SuperAdmin/PlatformAdmin) and accepts NO
// client-supplied org/tenant filter, so scope cannot be narrowed or injected
// from request input.
// ----------------------------------------------------------------------------

/// List DSA transparency reports.
async fn list_dsa_reports(
    State(state): State<AppState>,
    principal: RequestPrincipal,
) -> Result<Json<Vec<DsaTransparencyReportResponse>>, (StatusCode, String)> {
    require_platform_compliance_role(&principal)?;

    let reports = state
        .compliance_repo
        .list_dsa_reports(None, 50, 0)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list DSA reports: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to list reports".to_string(),
            )
        })?;

    let responses: Vec<DsaTransparencyReportResponse> = reports
        .into_iter()
        .map(|r| DsaTransparencyReportResponse {
            id: r.id,
            period_start: r.period_start,
            period_end: r.period_end,
            status: r.status,
            summary: DsaReportSummary {
                total_moderation_actions: r.total_moderation_actions,
                content_removed: r.content_removed_count,
                content_restricted: r.content_restricted_count,
                warnings_issued: r.warnings_issued_count,
                user_reports_received: r.user_reports_received,
                user_reports_resolved: r.user_reports_resolved,
                avg_resolution_time_hours: r.avg_resolution_time_hours,
                automated_decisions: r.automated_decisions_count,
                automated_decisions_overturned: r.automated_decisions_overturned,
                appeals_received: r.appeals_received,
                appeals_upheld: r.appeals_upheld,
                appeals_rejected: r.appeals_rejected,
            },
            content_type_breakdown: vec![],
            violation_type_breakdown: vec![],
            download_url: dsa_report_download_ref(r.id, &r.report_file_path),
            generated_at: r.generated_at,
            published_at: r.published_at,
        })
        .collect();

    Ok(Json(responses))
}

/// Generate a new DSA transparency report.
async fn generate_dsa_report(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    user: AuthUser,
    Json(req): Json<GenerateDsaReportRequest>,
) -> Result<Json<DsaTransparencyReportResponse>, (StatusCode, String)> {
    require_platform_compliance_role(&principal)?;

    // Bound the reporting period: end after start, not in the future, sane span.
    validate_report_period(req.period_start, req.period_end, Utc::now())
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let report = state
        .compliance_repo
        .create_dsa_report(req.period_start, req.period_end, user.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to generate DSA report: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to generate report".to_string(),
            )
        })?;

    Ok(Json(DsaTransparencyReportResponse {
        id: report.id,
        period_start: report.period_start,
        period_end: report.period_end,
        status: report.status,
        summary: DsaReportSummary {
            total_moderation_actions: report.total_moderation_actions,
            content_removed: report.content_removed_count,
            content_restricted: report.content_restricted_count,
            warnings_issued: report.warnings_issued_count,
            user_reports_received: report.user_reports_received,
            user_reports_resolved: report.user_reports_resolved,
            avg_resolution_time_hours: report.avg_resolution_time_hours,
            automated_decisions: report.automated_decisions_count,
            automated_decisions_overturned: report.automated_decisions_overturned,
            appeals_received: report.appeals_received,
            appeals_upheld: report.appeals_upheld,
            appeals_rejected: report.appeals_rejected,
        },
        content_type_breakdown: vec![],
        violation_type_breakdown: vec![],
        download_url: dsa_report_download_ref(report.id, &report.report_file_path),
        generated_at: report.generated_at,
        published_at: report.published_at,
    }))
}

/// Get a specific DSA report.
async fn get_dsa_report(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<DsaTransparencyReportResponse>, (StatusCode, String)> {
    require_platform_compliance_role(&principal)?;

    let report = state
        .compliance_repo
        .get_dsa_report(id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get DSA report: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get report".to_string(),
            )
        })?
        .ok_or((StatusCode::NOT_FOUND, format!("Report {} not found", id)))?;

    Ok(Json(DsaTransparencyReportResponse {
        id: report.id,
        period_start: report.period_start,
        period_end: report.period_end,
        status: report.status,
        summary: DsaReportSummary {
            total_moderation_actions: report.total_moderation_actions,
            content_removed: report.content_removed_count,
            content_restricted: report.content_restricted_count,
            warnings_issued: report.warnings_issued_count,
            user_reports_received: report.user_reports_received,
            user_reports_resolved: report.user_reports_resolved,
            avg_resolution_time_hours: report.avg_resolution_time_hours,
            automated_decisions: report.automated_decisions_count,
            automated_decisions_overturned: report.automated_decisions_overturned,
            appeals_received: report.appeals_received,
            appeals_upheld: report.appeals_upheld,
            appeals_rejected: report.appeals_rejected,
        },
        content_type_breakdown: vec![],
        violation_type_breakdown: vec![],
        download_url: dsa_report_download_ref(report.id, &report.report_file_path),
        generated_at: report.generated_at,
        published_at: report.published_at,
    }))
}

/// Publish a DSA report.
async fn publish_dsa_report(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<DsaTransparencyReportResponse>, (StatusCode, String)> {
    require_platform_compliance_role(&principal)?;

    let report = state
        .compliance_repo
        .publish_dsa_report(id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to publish DSA report: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to publish report".to_string(),
            )
        })?;

    write_compliance_audit(
        &state,
        &user,
        AuditAction::ResourceUpdated,
        "dsa_transparency_report",
        report.id,
        serde_json::json!({
            "operation": "publish_dsa_report",
            "resulting_status": report.status,
            "published_at": report.published_at,
        }),
    )
    .await;

    Ok(Json(DsaTransparencyReportResponse {
        id: report.id,
        period_start: report.period_start,
        period_end: report.period_end,
        status: report.status,
        summary: DsaReportSummary {
            total_moderation_actions: report.total_moderation_actions,
            content_removed: report.content_removed_count,
            content_restricted: report.content_restricted_count,
            warnings_issued: report.warnings_issued_count,
            user_reports_received: report.user_reports_received,
            user_reports_resolved: report.user_reports_resolved,
            avg_resolution_time_hours: report.avg_resolution_time_hours,
            automated_decisions: report.automated_decisions_count,
            automated_decisions_overturned: report.automated_decisions_overturned,
            appeals_received: report.appeals_received,
            appeals_upheld: report.appeals_upheld,
            appeals_rejected: report.appeals_rejected,
        },
        content_type_breakdown: vec![],
        violation_type_breakdown: vec![],
        download_url: dsa_report_download_ref(report.id, &report.report_file_path),
        generated_at: report.generated_at,
        published_at: report.published_at,
    }))
}

/// Download DSA report as PDF.
///
/// PAP-47 / PAP-35: returns a short-lived presigned URL generated from the
/// stored storage key — the raw `report_file_path` is NEVER returned to the
/// client (that was the file-path disclosure flagged in the PAP-35 review).
async fn download_dsa_report(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<DsaReportDownloadResponse>, (StatusCode, String)> {
    require_platform_compliance_role(&principal)?;

    let report = state
        .compliance_repo
        .get_dsa_report(id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get DSA report: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get report".to_string(),
            )
        })?
        .ok_or((StatusCode::NOT_FOUND, format!("Report {} not found", id)))?;

    let file_key = report.report_file_path.ok_or((
        StatusCode::NOT_FOUND,
        "Report file not yet generated".to_string(),
    ))?;

    let storage = state.storage_service.as_ref().ok_or_else(|| {
        tracing::error!("Storage service not configured — DSA report downloads unavailable");
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "Report storage is not configured.".to_string(),
        )
    })?;

    let filename = format!("dsa-transparency-report-{id}.pdf");
    let presigned = storage
        .generate_download_url(
            &file_key,
            &filename,
            "application/pdf",
            Some(storage.download_ttl_secs()),
        )
        .await
        .map_err(|e| {
            // Log the internal key for diagnostics; never surface it to the client.
            tracing::error!(error = %e, report_id = %id, "Failed to presign DSA report download");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "Unable to generate download URL. Please try again later.".to_string(),
            )
        })?;

    Ok(Json(DsaReportDownloadResponse {
        url: presigned.url,
        expires_at: presigned.expires_at,
    }))
}

/// DSA metrics for current period.
#[derive(Debug, Serialize)]
pub struct DsaMetricsResponse {
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
    pub moderation_actions_this_period: i64,
    pub pending_cases: i64,
    pub avg_resolution_time_hours: f64,
    pub sla_compliance_rate: f64,
}

/// Get current DSA metrics.
async fn get_dsa_metrics(
    State(state): State<AppState>,
    principal: RequestPrincipal,
) -> Result<Json<DsaMetricsResponse>, (StatusCode, String)> {
    require_platform_compliance_role(&principal)?;

    // DSA transparency metrics are platform-wide (all organizations), per the
    // PAP-40 tenancy decision and the platform-only gate above (PAP-46). There
    // is no org context here — a platform operator has `tenant_id = None`.
    let stats = state
        .compliance_repo
        .get_platform_moderation_queue_stats()
        .await
        .map_err(|e| {
            tracing::error!("Failed to get DSA metrics: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get metrics".to_string(),
            )
        })?;

    let now = Utc::now();
    let period_start = now - Duration::days(30);

    // SLA compliance: percentage of cases resolved within 24 hours
    let total_cases = stats.pending_count + stats.under_review_count;
    let sla_compliance_rate = if total_cases > 0 {
        ((total_cases - stats.overdue_count) as f64 / total_cases as f64) * 100.0
    } else {
        100.0
    };

    Ok(Json(DsaMetricsResponse {
        current_period_start: period_start,
        current_period_end: now,
        moderation_actions_this_period: stats.pending_count + stats.under_review_count,
        pending_cases: stats.pending_count,
        avg_resolution_time_hours: stats.avg_resolution_time_hours,
        sla_compliance_rate,
    }))
}

// ============================================================================
// STORY 67.4: CONTENT MODERATION DASHBOARD
// ============================================================================

/// Content owner info.
#[derive(Debug, Serialize)]
pub struct ContentOwnerInfo {
    pub user_id: Uuid,
    pub name: String,
    pub previous_violations: i32,
}

/// Moderation case response.
#[derive(Debug, Serialize)]
pub struct ModerationCaseResponse {
    pub id: Uuid,
    pub content_type: ModeratedContentType,
    pub content_id: Uuid,
    pub content_preview: Option<String>,
    pub content_owner: ContentOwnerInfo,
    pub report_source: String,
    pub violation_type: Option<ViolationType>,
    pub report_reason: Option<String>,
    pub status: ModerationStatus,
    pub priority: i32,
    pub assigned_to_name: Option<String>,
    pub decision: Option<ModerationActionType>,
    pub decision_rationale: Option<String>,
    pub appeal_filed: bool,
    pub appeal_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub age_hours: f64,
}

/// Moderation queue query parameters.
#[derive(Debug, Deserialize)]
pub struct ModerationQueueQuery {
    pub status: Option<ModerationStatus>,
    pub content_type: Option<ModeratedContentType>,
    pub violation_type: Option<ViolationType>,
    pub priority: Option<i32>,
    pub assigned_to: Option<Uuid>,
    pub unassigned_only: Option<bool>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Get moderation queue.
async fn get_moderation_queue(
    State(state): State<AppState>,
    user: AuthUser,
    Query(params): Query<ModerationQueueQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    require_moderator_role(&user)?;

    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    let limit = clamp_limit(params.limit);
    let offset = sanitize_offset(params.offset);
    let unassigned_only = params.unassigned_only.unwrap_or(false);

    let (cases, total) = state
        .compliance_repo
        .list_moderation_cases(
            org_id,
            params.status,
            params.content_type,
            params.violation_type,
            params.priority,
            params.assigned_to,
            unassigned_only,
            params.sort_by.as_deref(),
            params.sort_order.as_deref(),
            limit,
            offset,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to list moderation cases: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to list cases".to_string(),
            )
        })?;

    let now = Utc::now();
    let responses: Vec<ModerationCaseResponse> = cases
        .into_iter()
        .map(|c| {
            let age_hours = (now - c.created_at).num_minutes() as f64 / 60.0;
            ModerationCaseResponse {
                id: c.id,
                content_type: c.content_type,
                content_id: c.content_id,
                content_preview: c.content_preview,
                content_owner: ContentOwnerInfo {
                    user_id: c.content_owner_id,
                    name: "User".to_string(), // Would fetch from user repo
                    previous_violations: 0,   // Would calculate from repo
                },
                report_source: c.report_source.to_string(),
                violation_type: c.violation_type,
                report_reason: c.report_reason,
                status: c.status,
                priority: c.priority,
                assigned_to_name: None, // Would fetch from user repo
                decision: c.decision,
                decision_rationale: c.decision_rationale,
                appeal_filed: c.appeal_filed,
                appeal_reason: c.appeal_reason,
                created_at: c.created_at,
                age_hours,
            }
        })
        .collect();

    Ok(Json(serde_json::json!({
        "cases": responses,
        "total": total,
        "limit": limit,
        "offset": offset
    })))
}

/// Priority count.
#[derive(Debug, Serialize)]
pub struct PriorityCount {
    pub priority: i32,
    pub count: i64,
}

/// Moderation queue statistics.
#[derive(Debug, Serialize)]
pub struct ModerationQueueStatsResponse {
    pub pending_count: i64,
    pub under_review_count: i64,
    pub by_priority: Vec<PriorityCount>,
    pub by_violation_type: Vec<ViolationTypeCountResponse>,
    pub avg_resolution_time_hours: f64,
    pub overdue_count: i64,
}

/// Get moderation queue statistics.
async fn get_moderation_stats(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<ModerationQueueStatsResponse>, (StatusCode, String)> {
    require_moderator_role(&user)?;

    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    let stats = state
        .compliance_repo
        .get_moderation_queue_stats(org_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get moderation stats: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get stats".to_string(),
            )
        })?;

    Ok(Json(ModerationQueueStatsResponse {
        pending_count: stats.pending_count,
        under_review_count: stats.under_review_count,
        by_priority: stats
            .by_priority
            .into_iter()
            .map(|p| PriorityCount {
                priority: p.priority,
                count: p.count,
            })
            .collect(),
        by_violation_type: stats
            .by_violation_type
            .into_iter()
            .map(|v| ViolationTypeCountResponse {
                violation_type: v.violation_type,
                count: v.count,
            })
            .collect(),
        avg_resolution_time_hours: stats.avg_resolution_time_hours,
        overdue_count: stats.overdue_count,
    }))
}

/// Get a specific moderation case.
async fn get_moderation_case(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ModerationCaseResponse>, (StatusCode, String)> {
    require_moderator_role(&user)?;

    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    let case = state
        .compliance_repo
        .get_moderation_case(id, org_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get moderation case: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get case".to_string(),
            )
        })?
        .ok_or((StatusCode::NOT_FOUND, format!("Case {} not found", id)))?;

    // Get violation count for content owner
    let violation_count = state
        .compliance_repo
        .get_user_violation_count(case.content_owner_id)
        .await
        .unwrap_or(0);

    let now = Utc::now();
    let age_hours = (now - case.created_at).num_minutes() as f64 / 60.0;

    Ok(Json(ModerationCaseResponse {
        id: case.id,
        content_type: case.content_type,
        content_id: case.content_id,
        content_preview: case.content_preview,
        content_owner: ContentOwnerInfo {
            user_id: case.content_owner_id,
            name: "User".to_string(),
            previous_violations: violation_count,
        },
        report_source: case.report_source.to_string(),
        violation_type: case.violation_type,
        report_reason: case.report_reason,
        status: case.status,
        priority: case.priority,
        assigned_to_name: None,
        decision: case.decision,
        decision_rationale: case.decision_rationale,
        appeal_filed: case.appeal_filed,
        appeal_reason: case.appeal_reason,
        created_at: case.created_at,
        age_hours,
    }))
}

/// Assign case request.
#[derive(Debug, Deserialize)]
pub struct AssignCaseRequest {
    pub moderator_id: Option<Uuid>, // None = assign to self
}

/// Assign a moderation case.
async fn assign_moderation_case(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<AssignCaseRequest>,
) -> Result<Json<ModerationCaseResponse>, (StatusCode, String)> {
    require_moderator_role(&user)?;

    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    let assignee = req.moderator_id.unwrap_or(user.user_id);

    // Validate that the target moderator belongs to the caller's org
    if assignee != user.user_id {
        let is_member = state
            .org_member_repo
            .is_member(org_id, assignee)
            .await
            .unwrap_or(false);
        if !is_member {
            return Err((
                StatusCode::FORBIDDEN,
                "Moderator does not belong to this organization".to_string(),
            ));
        }
    }

    let case = state
        .compliance_repo
        .assign_moderation_case(id, org_id, assignee)
        .await
        .map_err(|e| {
            tracing::error!("Failed to assign moderation case: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to assign case".to_string(),
            )
        })?;

    let now = Utc::now();
    let age_hours = (now - case.created_at).num_minutes() as f64 / 60.0;

    Ok(Json(ModerationCaseResponse {
        id: case.id,
        content_type: case.content_type,
        content_id: case.content_id,
        content_preview: case.content_preview,
        content_owner: ContentOwnerInfo {
            user_id: case.content_owner_id,
            name: "User".to_string(),
            previous_violations: 0,
        },
        report_source: case.report_source.to_string(),
        violation_type: case.violation_type,
        report_reason: case.report_reason,
        status: case.status,
        priority: case.priority,
        assigned_to_name: Some("Assigned".to_string()),
        decision: case.decision,
        decision_rationale: case.decision_rationale,
        appeal_filed: case.appeal_filed,
        appeal_reason: case.appeal_reason,
        created_at: case.created_at,
        age_hours,
    }))
}

/// Take moderation action request.
#[derive(Debug, Deserialize)]
pub struct TakeModerationActionRequest {
    pub action: ModerationActionType,
    pub rationale: String,
    pub template_id: Option<Uuid>,
}

/// Take action on a moderation case.
async fn take_moderation_action(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<TakeModerationActionRequest>,
) -> Result<Json<ModerationCaseResponse>, (StatusCode, String)> {
    require_moderator_role(&user)?;

    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    validate_text_field(&req.rationale, MAX_RATIONALE_LEN, "rationale")
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    // Capture the decision payload before it is moved into the repo call so it
    // can be recorded as the DSA Art. 17 statement of reasons (action + rationale).
    let moderation_action = req.action;
    let rationale = req.rationale.clone();

    let action = TakeModerationAction {
        action: req.action,
        rationale: req.rationale,
        template_id: req.template_id,
    };

    let case = state
        .compliance_repo
        .take_moderation_action(id, org_id, action, user.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to take moderation action: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to take action".to_string(),
            )
        })?;

    write_compliance_audit(
        &state,
        &user,
        AuditAction::ResourceUpdated,
        "moderation_case",
        case.id,
        serde_json::json!({
            "operation": "take_moderation_action",
            "action": moderation_action,
            "rationale": rationale,
            "resulting_status": case.status,
        }),
    )
    .await;

    let now = Utc::now();
    let age_hours = (now - case.created_at).num_minutes() as f64 / 60.0;

    Ok(Json(ModerationCaseResponse {
        id: case.id,
        content_type: case.content_type,
        content_id: case.content_id,
        content_preview: case.content_preview,
        content_owner: ContentOwnerInfo {
            user_id: case.content_owner_id,
            name: "User".to_string(),
            previous_violations: 0,
        },
        report_source: case.report_source.to_string(),
        violation_type: case.violation_type,
        report_reason: case.report_reason,
        status: case.status,
        priority: case.priority,
        assigned_to_name: None,
        decision: case.decision,
        decision_rationale: case.decision_rationale,
        appeal_filed: case.appeal_filed,
        appeal_reason: case.appeal_reason,
        created_at: case.created_at,
        age_hours,
    }))
}

/// File appeal request.
#[derive(Debug, Deserialize)]
pub struct FileAppealRequest {
    pub reason: String,
}

/// File an appeal against moderation decision.
async fn file_appeal(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<FileAppealRequest>,
) -> Result<Json<ModerationCaseResponse>, (StatusCode, String)> {
    // Any authenticated user can file appeal for their content

    validate_text_field(&req.reason, MAX_APPEAL_REASON_LEN, "reason")
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let case = state
        .compliance_repo
        .file_appeal(id, &req.reason)
        .await
        .map_err(|e| {
            tracing::error!("Failed to file appeal: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to file appeal".to_string(),
            )
        })?;

    tracing::info!(
        case_id = %id,
        appealed_by = %user.user_id,
        "Appeal filed"
    );

    let now = Utc::now();
    let age_hours = (now - case.created_at).num_minutes() as f64 / 60.0;

    Ok(Json(ModerationCaseResponse {
        id: case.id,
        content_type: case.content_type,
        content_id: case.content_id,
        content_preview: case.content_preview,
        content_owner: ContentOwnerInfo {
            user_id: case.content_owner_id,
            name: "User".to_string(),
            previous_violations: 0,
        },
        report_source: case.report_source.to_string(),
        violation_type: case.violation_type,
        report_reason: case.report_reason,
        status: case.status,
        priority: case.priority,
        assigned_to_name: None,
        decision: case.decision,
        decision_rationale: case.decision_rationale,
        appeal_filed: case.appeal_filed,
        appeal_reason: case.appeal_reason,
        created_at: case.created_at,
        age_hours,
    }))
}

/// Decide appeal request.
#[derive(Debug, Deserialize)]
pub struct DecideAppealRequest {
    pub decision: String, // upheld, rejected
    pub rationale: String,
}

/// Decide on an appeal.
async fn decide_appeal(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<DecideAppealRequest>,
) -> Result<Json<ModerationCaseResponse>, (StatusCode, String)> {
    require_moderator_role(&user)?;

    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    let case = state
        .compliance_repo
        .decide_appeal(id, org_id, &req.decision, &req.rationale, user.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to decide appeal: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to decide appeal".to_string(),
            )
        })?;

    write_compliance_audit(
        &state,
        &user,
        AuditAction::ResourceUpdated,
        "moderation_case",
        case.id,
        serde_json::json!({
            "operation": "decide_appeal",
            "decision": req.decision,
            "rationale": req.rationale,
            "resulting_status": case.status,
        }),
    )
    .await;

    let now = Utc::now();
    let age_hours = (now - case.created_at).num_minutes() as f64 / 60.0;

    Ok(Json(ModerationCaseResponse {
        id: case.id,
        content_type: case.content_type,
        content_id: case.content_id,
        content_preview: case.content_preview,
        content_owner: ContentOwnerInfo {
            user_id: case.content_owner_id,
            name: "User".to_string(),
            previous_violations: 0,
        },
        report_source: case.report_source.to_string(),
        violation_type: case.violation_type,
        report_reason: case.report_reason,
        status: case.status,
        priority: case.priority,
        assigned_to_name: None,
        decision: case.decision,
        decision_rationale: case.decision_rationale,
        appeal_filed: case.appeal_filed,
        appeal_reason: case.appeal_reason,
        created_at: case.created_at,
        age_hours,
    }))
}

/// Report content request.
#[derive(Debug, Deserialize)]
pub struct ReportContentRequest {
    pub content_type: ModeratedContentType,
    pub content_id: Uuid,
    pub violation_type: Option<ViolationType>,
    pub reason: Option<String>,
}

/// Report content for moderation.
async fn report_content(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<ReportContentRequest>,
) -> Result<Json<ModerationCaseResponse>, (StatusCode, String)> {
    // Any authenticated user can report content

    let create_req = CreateModerationCase {
        content_type: req.content_type,
        content_id: req.content_id,
        violation_type: req.violation_type,
        report_reason: req.reason,
    };

    // For now, use a placeholder for content_owner_id - in production this would be looked up
    let content_owner_id = Uuid::new_v4();

    let case = state
        .compliance_repo
        .create_moderation_case(create_req, user.user_id, content_owner_id, user.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create moderation case: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to report content".to_string(),
            )
        })?;

    let now = Utc::now();
    let age_hours = (now - case.created_at).num_minutes() as f64 / 60.0;

    Ok(Json(ModerationCaseResponse {
        id: case.id,
        content_type: case.content_type,
        content_id: case.content_id,
        content_preview: case.content_preview,
        content_owner: ContentOwnerInfo {
            user_id: case.content_owner_id,
            name: "Unknown".to_string(),
            previous_violations: 0,
        },
        report_source: case.report_source.to_string(),
        violation_type: case.violation_type,
        report_reason: case.report_reason,
        status: case.status,
        priority: case.priority,
        assigned_to_name: None,
        decision: case.decision,
        decision_rationale: case.decision_rationale,
        appeal_filed: case.appeal_filed,
        appeal_reason: case.appeal_reason,
        created_at: case.created_at,
        age_hours,
    }))
}

/// Action template response.
#[derive(Debug, Serialize)]
pub struct ActionTemplateResponse {
    pub id: Uuid,
    pub name: String,
    pub violation_type: ViolationType,
    pub action_type: ModerationActionType,
    pub rationale_template: String,
    pub notify_owner: bool,
}

/// Get available action templates.
async fn get_action_templates(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<ActionTemplateResponse>>, (StatusCode, String)> {
    require_moderator_role(&user)?;

    let templates = state
        .compliance_repo
        .list_action_templates()
        .await
        .map_err(|e| {
            tracing::error!("Failed to list action templates: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get templates".to_string(),
            )
        })?;

    let responses: Vec<ActionTemplateResponse> = templates
        .into_iter()
        .map(|t| ActionTemplateResponse {
            id: t.id,
            name: t.name,
            violation_type: t.violation_type,
            action_type: t.action_type,
            rationale_template: t.rationale_template,
            notify_owner: t.notify_owner,
        })
        .collect();

    Ok(Json(responses))
}

// ============================================================================
// TESTS (PAP-44): boundary input-validation helpers
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use db::models::PrincipalKind;

    fn sample_upload() -> UploadEddDocumentRequest {
        UploadEddDocumentRequest {
            document_type: "passport".to_string(),
            original_filename: "passport.pdf".to_string(),
            file_path: "edd/2026/passport.pdf".to_string(),
            file_size_bytes: 1024,
            mime_type: "application/pdf".to_string(),
            expiry_date: None,
        }
    }

    // ----- pagination -----

    #[test]
    fn clamp_limit_defaults_when_absent() {
        assert_eq!(clamp_limit(None), DEFAULT_PAGE_LIMIT);
    }

    #[test]
    fn clamp_limit_caps_at_max() {
        assert_eq!(clamp_limit(Some(100_000)), MAX_PAGE_LIMIT);
        assert_eq!(clamp_limit(Some(MAX_PAGE_LIMIT + 1)), MAX_PAGE_LIMIT);
    }

    #[test]
    fn clamp_limit_floors_non_positive() {
        assert_eq!(clamp_limit(Some(0)), 1);
        assert_eq!(clamp_limit(Some(-50)), 1);
    }

    #[test]
    fn clamp_limit_passes_through_in_range() {
        assert_eq!(clamp_limit(Some(75)), 75);
    }

    #[test]
    fn sanitize_offset_rejects_negative() {
        assert_eq!(sanitize_offset(Some(-10)), 0);
        assert_eq!(sanitize_offset(None), 0);
        assert_eq!(sanitize_offset(Some(40)), 40);
    }

    // ----- path traversal / file upload -----

    #[test]
    fn storage_path_accepts_relative() {
        assert!(validate_storage_path("edd/2026/doc.pdf").is_ok());
    }

    #[test]
    fn storage_path_rejects_traversal() {
        assert!(validate_storage_path("../../etc/passwd").is_err());
        assert!(validate_storage_path("edd/../../secret").is_err());
        assert!(validate_storage_path("a/../b").is_err());
    }

    #[test]
    fn storage_path_rejects_absolute_and_windows() {
        assert!(validate_storage_path("/etc/passwd").is_err());
        assert!(validate_storage_path("\\\\server\\share").is_err());
        assert!(validate_storage_path("C:\\Windows\\system32").is_err());
    }

    #[test]
    fn storage_path_rejects_empty_and_control_chars() {
        assert!(validate_storage_path("   ").is_err());
        assert!(validate_storage_path("doc\0.pdf").is_err());
    }

    #[test]
    fn edd_document_accepts_valid() {
        assert!(validate_edd_document(&sample_upload()).is_ok());
    }

    #[test]
    fn edd_document_rejects_path_traversal() {
        let mut req = sample_upload();
        req.file_path = "../../../etc/shadow".to_string();
        assert!(validate_edd_document(&req).is_err());
    }

    #[test]
    fn edd_document_rejects_filename_with_separators() {
        let mut req = sample_upload();
        req.original_filename = "../evil.pdf".to_string();
        assert!(validate_edd_document(&req).is_err());
    }

    #[test]
    fn edd_document_rejects_disallowed_mime() {
        let mut req = sample_upload();
        req.mime_type = "application/x-msdownload".to_string();
        assert!(validate_edd_document(&req).is_err());
    }

    #[test]
    fn edd_document_rejects_bad_size() {
        let mut req = sample_upload();
        req.file_size_bytes = 0;
        assert!(validate_edd_document(&req).is_err());
        req.file_size_bytes = -1;
        assert!(validate_edd_document(&req).is_err());
        req.file_size_bytes = MAX_EDD_DOCUMENT_BYTES + 1;
        assert!(validate_edd_document(&req).is_err());
    }

    #[test]
    fn edd_document_mime_is_case_insensitive() {
        let mut req = sample_upload();
        req.mime_type = "APPLICATION/PDF".to_string();
        assert!(validate_edd_document(&req).is_ok());
    }

    // ----- DSA report period -----

    #[test]
    fn report_period_accepts_valid_past_range() {
        let now = Utc::now();
        let end = now - Duration::days(1);
        let start = end - Duration::days(30);
        assert!(validate_report_period(start, end, now).is_ok());
    }

    #[test]
    fn report_period_rejects_end_before_start() {
        let now = Utc::now();
        let start = now - Duration::days(1);
        let end = now - Duration::days(10);
        assert!(validate_report_period(start, end, now).is_err());
    }

    #[test]
    fn report_period_rejects_future_end() {
        let now = Utc::now();
        let start = now - Duration::days(5);
        let end = now + Duration::days(5);
        assert!(validate_report_period(start, end, now).is_err());
    }

    #[test]
    fn report_period_rejects_absurd_range() {
        let now = Utc::now();
        let end = now - Duration::days(1);
        let start = end - Duration::days(MAX_DSA_REPORT_RANGE_DAYS + 10);
        assert!(validate_report_period(start, end, now).is_err());
    }

    // ----- free-text caps -----

    #[test]
    fn text_field_rejects_empty() {
        assert!(validate_text_field("", MAX_NOTE_LEN, "content").is_err());
        assert!(validate_text_field("   \n\t ", MAX_NOTE_LEN, "content").is_err());
    }

    #[test]
    fn text_field_rejects_over_max() {
        let too_long = "a".repeat(MAX_APPEAL_REASON_LEN + 1);
        assert!(validate_text_field(&too_long, MAX_APPEAL_REASON_LEN, "reason").is_err());
    }

    #[test]
    fn text_field_accepts_within_bounds() {
        assert!(validate_text_field("a reasonable note", MAX_NOTE_LEN, "content").is_ok());
    }

    // ----- scoped download reference -----

    #[test]
    fn download_ref_hides_fs_path() {
        let id = Uuid::nil();
        let raw = Some("/var/lib/app/reports/secret.pdf".to_string());
        let reference = dsa_report_download_ref(id, &raw).unwrap();
        assert!(!reference.contains("/var/lib"));
        assert!(reference.contains(&id.to_string()));
    }

    #[test]
    fn download_ref_none_when_no_file() {
        assert!(dsa_report_download_ref(Uuid::nil(), &None).is_none());
    }

    // ----- platform-authz boundary (PAP-46) -----

    fn principal(kind: PrincipalKind) -> RequestPrincipal {
        RequestPrincipal {
            user_id: Uuid::nil(),
            kind,
            effective_org: None,
        }
    }

    #[test]
    fn platform_principal_passes_dsa_guard() {
        assert!(require_platform_compliance_role(&principal(PrincipalKind::Platform)).is_ok());
    }

    #[test]
    fn tenant_scoped_principals_get_403_from_dsa_guard() {
        // A customer-org admin (Staff principal) — even one whose per-org
        // membership role is SuperAdmin/PlatformAdmin — must be rejected from
        // the platform-wide DSA-report handlers, as must portal (Public) users.
        for kind in [PrincipalKind::Staff, PrincipalKind::Public] {
            let err = require_platform_compliance_role(&principal(kind)).unwrap_err();
            assert_eq!(err.0, StatusCode::FORBIDDEN);
        }
    }
}
