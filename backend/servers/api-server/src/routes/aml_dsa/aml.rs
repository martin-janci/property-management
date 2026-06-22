//! Story 67.1: AML Risk Assessment endpoints.

use crate::state::AppState;
use api_core::extractors::AuthUser;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use db::models::compliance::{AmlAssessmentStatus, AmlRiskLevel, CreateAmlRiskAssessment};
use db::models::AuditAction;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::shared::*;

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
pub(super) async fn create_aml_assessment(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateAmlAssessmentRequest>,
) -> Result<Json<AmlAssessmentResponse>, (StatusCode, String)> {
    // Creating an AML risk assessment is a compliance-officer action, like every
    // other AML/EDD handler in this module. Without this check any authenticated
    // tenant user could create assessments (PAP-60/PAP-43).
    require_compliance_role(&user)?;

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
pub(super) async fn list_aml_assessments(
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
pub(super) async fn get_aml_assessment(
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
pub(super) async fn review_aml_assessment(
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
pub(super) async fn get_country_risks(
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
pub(super) async fn get_aml_thresholds(
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
