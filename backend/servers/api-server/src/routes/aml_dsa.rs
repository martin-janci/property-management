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
    CreateModerationCase, DsaReportStatus, DsaTransparencyReport, EddStatus, ModeratedContentType,
    ModerationActionType, ModerationStatus, TakeModerationAction, ViolationType,
};
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
/// column by the `RequestPrincipal` extractor), NOT on `TenantRole`: a
/// tenant-scoped admin must never reach these handlers even if their
/// per-org membership role happens to be SuperAdmin/PlatformAdmin. This is a
/// distinct guard from `require_compliance_role`, which correctly gates the
/// per-tenant AML/EDD handlers and must not be weakened.
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

    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);
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
        .review_aml_assessment(id, user.user_id, decision, req.notes.as_deref())
        .await
        .map_err(|e| {
            tracing::error!("Failed to review AML assessment: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to review assessment".to_string(),
            )
        })?;

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

/// Map a stored DSA report to its API response.
///
/// SECURITY (PAP-46): `download_url` is intentionally `None` here — the stored
/// `report_file_path` is a server-side object-storage key and must never be
/// returned to clients. Downloads go through `download_dsa_report`, which
/// returns a short-lived signed URL instead.
fn dsa_report_to_response(report: DsaTransparencyReport) -> DsaTransparencyReportResponse {
    DsaTransparencyReportResponse {
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
        download_url: None,
        generated_at: report.generated_at,
        published_at: report.published_at,
    }
}

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

    let responses: Vec<DsaTransparencyReportResponse> =
        reports.into_iter().map(dsa_report_to_response).collect();

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

    Ok(Json(dsa_report_to_response(report)))
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

    Ok(Json(dsa_report_to_response(report)))
}

/// Publish a DSA report.
async fn publish_dsa_report(
    State(state): State<AppState>,
    principal: RequestPrincipal,
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

    Ok(Json(dsa_report_to_response(report)))
}

/// Response for a DSA report download request: a short-lived signed URL.
#[derive(Debug, Serialize)]
pub struct DsaDownloadResponse {
    pub download_url: String,
    pub expires_at: DateTime<Utc>,
}

/// Download DSA report as PDF.
async fn download_dsa_report(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<DsaDownloadResponse>, (StatusCode, String)> {
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

    // SECURITY (PAP-46): never return the raw stored path. The artifact lives
    // in object storage; hand the client a short-lived signed URL instead.
    let file_key = report.report_file_path.ok_or((
        StatusCode::NOT_FOUND,
        "Report file not yet generated".to_string(),
    ))?;

    let storage = state.storage_service.as_ref().ok_or_else(|| {
        tracing::error!("Storage service not configured — DSA report downloads unavailable");
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "Report storage is not configured".to_string(),
        )
    })?;

    let filename = format!("dsa-transparency-report-{}.pdf", report.id);
    let presigned = storage
        .generate_download_url(
            &file_key,
            &filename,
            "application/pdf",
            // Short-lived TTL from S3_PRESIGNED_URL_TTL_SECS (default 15 min).
            Some(storage.download_ttl_secs()),
        )
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                report_id = %report.id,
                "Failed to sign DSA report download URL"
            );
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "Failed to generate download URL".to_string(),
            )
        })?;

    Ok(Json(DsaDownloadResponse {
        download_url: presigned.url,
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

    let stats = state
        .compliance_repo
        .get_moderation_queue_stats()
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

    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);
    let unassigned_only = params.unassigned_only.unwrap_or(false);

    let (cases, total) = state
        .compliance_repo
        .list_moderation_cases(
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

    let stats = state
        .compliance_repo
        .get_moderation_queue_stats()
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

    let case = state
        .compliance_repo
        .get_moderation_case(id)
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

    let assignee = req.moderator_id.unwrap_or(user.user_id);

    let case = state
        .compliance_repo
        .assign_moderation_case(id, assignee)
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

    let action = TakeModerationAction {
        action: req.action,
        rationale: req.rationale,
        template_id: req.template_id,
    };

    let case = state
        .compliance_repo
        .take_moderation_action(id, action, user.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to take moderation action: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to take action".to_string(),
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

    let case = state
        .compliance_repo
        .decide_appeal(id, &req.decision, &req.rationale, user.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to decide appeal: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to decide appeal".to_string(),
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
// TESTS (PAP-46): DSA-report authz boundary + download-path non-disclosure
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use db::models::PrincipalKind;

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

    fn sample_report(file_path: Option<String>) -> DsaTransparencyReport {
        DsaTransparencyReport {
            id: Uuid::nil(),
            period_start: Utc::now(),
            period_end: Utc::now(),
            status: DsaReportStatus::Published,
            total_moderation_actions: 0,
            content_removed_count: 0,
            content_restricted_count: 0,
            warnings_issued_count: 0,
            user_reports_received: 0,
            user_reports_resolved: 0,
            avg_resolution_time_hours: None,
            automated_decisions_count: 0,
            automated_decisions_overturned: 0,
            appeals_received: 0,
            appeals_upheld: 0,
            appeals_rejected: 0,
            content_type_breakdown: None,
            violation_type_breakdown: None,
            report_file_path: file_path,
            generated_at: None,
            generated_by: None,
            published_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn response_never_leaks_stored_file_path() {
        // Even when a server-side path/key is stored, the API response must
        // not expose it; clients must use the signed-URL download endpoint.
        let resp = dsa_report_to_response(sample_report(Some(
            "/var/lib/reports/secret/dsa-2026.pdf".to_string(),
        )));
        assert!(
            resp.download_url.is_none(),
            "download_url must never carry the raw stored path"
        );
    }
}
