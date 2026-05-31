//! Insurance management routes for Epic 22.
//!
//! Handles insurance policies, claims, documents, and renewal reminders.
//!
//! # Authentication & tenancy (SECURITY-CRITICAL)
//!
//! Every handler derives the owning organization from the verified
//! [`RequestPrincipal`] (built from the bearer JWT + host-resolved tenant),
//! never from a client-supplied `organization_id` query param / request-body
//! field. The previous implementation read tenancy from
//! `query.building_id.unwrap_or_default()` (a placeholder that resolved to
//! `Uuid::nil()` when absent) on the list endpoints, and accepted an
//! `?organization_id=` query param / body `organization_id` on the
//! get/mutate endpoints. Both let any caller read or mutate another org's
//! policies/claims (a cross-tenant IDOR) and produced empty/garbage results
//! for legitimate callers. See issue #826.

use api_core::extractors::principal::RequestPrincipal;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{
    AddClaimDocument, AddPolicyDocument, ClaimStatusSummary, CreateInsuranceClaim,
    CreateInsurancePolicy, CreateRenewalReminder, ExpiringPolicy, InsuranceClaim,
    InsuranceClaimDocument, InsuranceClaimHistory, InsuranceClaimWithPolicy, InsurancePolicy,
    InsurancePolicyDocument, InsuranceRenewalReminder, InsuranceStatistics, PolicyTypeSummary,
    UpdateInsuranceClaim, UpdateInsurancePolicy, UpdateRenewalReminder,
};
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use uuid::Uuid;

use crate::state::AppState;

// ============================================
// Helper Functions
// ============================================

/// Resolve the effective tenant (organization) id from a verified
/// [`RequestPrincipal`].
///
/// Insurance data is per-tenant by definition. A platform-kind principal
/// hitting the platform host has no `effective_org` — for those callers we
/// refuse with 403 rather than fall back to a wildcard / nil org. Non-platform
/// principals without a host-resolved tenant never reach this point because
/// `RequestPrincipal` rejects them earlier.
fn require_org_id(principal: &RequestPrincipal) -> Result<Uuid, (StatusCode, Json<ErrorResponse>)> {
    principal.effective_org.ok_or_else(|| {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "TENANT_REQUIRED",
                "Insurance endpoints require a tenant-resolved request",
            )),
        )
    })
}

// ============================================
// Request/Response Types
// ============================================

/// Query parameters for listing policies.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListPoliciesQuery {
    pub policy_type: Option<String>,
    pub status: Option<String>,
    pub building_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub provider_name: Option<String>,
    pub expiring_within_days: Option<i32>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl From<ListPoliciesQuery> for db::models::InsurancePolicyQuery {
    fn from(q: ListPoliciesQuery) -> Self {
        Self {
            policy_type: q.policy_type,
            status: q.status,
            building_id: q.building_id,
            unit_id: q.unit_id,
            provider_name: q.provider_name,
            expiring_within_days: q.expiring_within_days,
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Query parameters for listing claims.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListClaimsQuery {
    pub policy_id: Option<Uuid>,
    pub status: Option<String>,
    pub building_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub incident_date_from: Option<chrono::NaiveDate>,
    pub incident_date_to: Option<chrono::NaiveDate>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl From<ListClaimsQuery> for db::models::InsuranceClaimQuery {
    fn from(q: ListClaimsQuery) -> Self {
        Self {
            policy_id: q.policy_id,
            status: q.status,
            building_id: q.building_id,
            unit_id: q.unit_id,
            incident_date_from: q.incident_date_from,
            incident_date_to: q.incident_date_to,
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Request to create a policy.
///
/// The owning organization is derived from the authenticated principal, never
/// from the request body (issue #826).
#[derive(Debug, Deserialize, IntoParams)]
pub struct CreatePolicyRequest {
    #[serde(flatten)]
    pub data: CreateInsurancePolicy,
}

/// Request to update a policy.
#[derive(Debug, Deserialize, IntoParams)]
pub struct UpdatePolicyRequest {
    #[serde(flatten)]
    pub data: UpdateInsurancePolicy,
}

/// Request to create a claim.
#[derive(Debug, Deserialize, IntoParams)]
pub struct CreateClaimRequest {
    #[serde(flatten)]
    pub data: CreateInsuranceClaim,
}

/// Request to update a claim.
#[derive(Debug, Deserialize, IntoParams)]
pub struct UpdateClaimRequest {
    #[serde(flatten)]
    pub data: UpdateInsuranceClaim,
}

/// Request to review a claim.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ReviewClaimRequest {
    pub status: String,
    pub approved_amount: Option<rust_decimal::Decimal>,
    pub denial_reason: Option<String>,
    pub resolution_notes: Option<String>,
}

/// Request to record claim payment.
#[derive(Debug, Deserialize, IntoParams)]
pub struct RecordClaimPaymentRequest {
    pub payment_amount: rust_decimal::Decimal,
}

/// Response for policy list.
#[derive(Debug, Serialize, IntoParams)]
pub struct ListPoliciesResponse {
    pub policies: Vec<InsurancePolicy>,
}

/// Response for claim list.
#[derive(Debug, Serialize, IntoParams)]
pub struct ListClaimsResponse {
    pub claims: Vec<InsuranceClaimWithPolicy>,
}

/// Response for expiring policies.
#[derive(Debug, Serialize, IntoParams)]
pub struct ExpiringPoliciesResponse {
    pub policies: Vec<ExpiringPolicy>,
}

/// Response for policy documents.
#[derive(Debug, Serialize, IntoParams)]
pub struct PolicyDocumentsResponse {
    pub documents: Vec<InsurancePolicyDocument>,
}

/// Response for claim documents.
#[derive(Debug, Serialize, IntoParams)]
pub struct ClaimDocumentsResponse {
    pub documents: Vec<InsuranceClaimDocument>,
}

/// Response for claim history.
#[derive(Debug, Serialize, IntoParams)]
pub struct ClaimHistoryResponse {
    pub history: Vec<InsuranceClaimHistory>,
}

/// Response for reminders.
#[derive(Debug, Serialize, IntoParams)]
pub struct RemindersResponse {
    pub reminders: Vec<InsuranceRenewalReminder>,
}

/// Response for statistics.
#[derive(Debug, Serialize, IntoParams)]
pub struct StatisticsResponse {
    pub statistics: InsuranceStatistics,
    pub claims_by_status: Vec<ClaimStatusSummary>,
    pub policies_by_type: Vec<PolicyTypeSummary>,
}

/// Response for delete operation.
#[derive(Debug, Serialize, IntoParams)]
pub struct DeleteResponse {
    pub success: bool,
}

// ============================================
// Policy Routes
// ============================================

/// Create insurance router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Policy routes
        .route("/policies", get(list_policies).post(create_policy))
        .route(
            "/policies/{policy_id}",
            get(get_policy).put(update_policy).delete(delete_policy),
        )
        .route("/policies/expiring", get(get_expiring_policies))
        // Policy document routes
        .route(
            "/policies/{policy_id}/documents",
            get(list_policy_documents).post(add_policy_document),
        )
        .route(
            "/policies/{policy_id}/documents/{document_id}",
            delete(remove_policy_document),
        )
        // Renewal reminder routes
        .route(
            "/policies/{policy_id}/reminders",
            get(list_reminders).post(create_reminder),
        )
        .route(
            "/reminders/{reminder_id}",
            put(update_reminder).delete(delete_reminder),
        )
        // Claim routes
        .route("/claims", get(list_claims).post(create_claim))
        .route(
            "/claims/{claim_id}",
            get(get_claim).put(update_claim).delete(delete_claim),
        )
        .route("/claims/{claim_id}/submit", post(submit_claim))
        .route("/claims/{claim_id}/review", post(review_claim))
        .route("/claims/{claim_id}/payment", post(record_claim_payment))
        .route("/claims/{claim_id}/history", get(get_claim_history))
        // Claim document routes
        .route(
            "/claims/{claim_id}/documents",
            get(list_claim_documents).post(add_claim_document),
        )
        .route(
            "/claims/{claim_id}/documents/{document_id}",
            delete(remove_claim_document),
        )
        // Statistics
        .route("/statistics", get(get_statistics))
}

// ============================================
// Policy Handlers
// ============================================

/// List insurance policies.
async fn list_policies(
    State(state): State<AppState>,
    Query(query): Query<ListPoliciesQuery>,
    principal: RequestPrincipal,
) -> Result<Json<ListPoliciesResponse>, (StatusCode, Json<ErrorResponse>)> {
    // SECURITY: the org is derived from the verified principal, never from a
    // client-supplied query param (issue #826 — was `building_id.unwrap_or_default()`).
    let org_id = require_org_id(&principal)?;

    let policies = state
        .insurance_repo
        .list_policies(org_id, query.into())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(ListPoliciesResponse { policies }))
}

/// Create a new insurance policy.
async fn create_policy(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Json(payload): Json<CreatePolicyRequest>,
) -> Result<Json<InsurancePolicy>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = require_org_id(&principal)?;
    let policy = state
        .insurance_repo
        .create_policy(org_id, payload.data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(policy))
}

/// Get a policy by ID.
async fn get_policy(
    State(state): State<AppState>,
    Path(policy_id): Path<Uuid>,
    principal: RequestPrincipal,
) -> Result<Json<InsurancePolicy>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = require_org_id(&principal)?;
    let policy = state
        .insurance_repo
        .find_policy_by_id(org_id, policy_id)
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
                Json(ErrorResponse::new("NOT_FOUND", "Policy not found")),
            )
        })?;

    Ok(Json(policy))
}

/// Update a policy.
async fn update_policy(
    State(state): State<AppState>,
    Path(policy_id): Path<Uuid>,
    principal: RequestPrincipal,
    Json(payload): Json<UpdatePolicyRequest>,
) -> Result<Json<InsurancePolicy>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = require_org_id(&principal)?;
    let policy = state
        .insurance_repo
        .update_policy(org_id, policy_id, payload.data)
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
                Json(ErrorResponse::new("NOT_FOUND", "Policy not found")),
            )
        })?;

    Ok(Json(policy))
}

/// Delete a policy.
async fn delete_policy(
    State(state): State<AppState>,
    Path(policy_id): Path<Uuid>,
    principal: RequestPrincipal,
) -> Result<Json<DeleteResponse>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = require_org_id(&principal)?;
    let success = state
        .insurance_repo
        .delete_policy(org_id, policy_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(DeleteResponse { success }))
}

/// Get expiring policies.
async fn get_expiring_policies(
    State(state): State<AppState>,
    Query(params): Query<ExpiringQuery>,
    principal: RequestPrincipal,
) -> Result<Json<ExpiringPoliciesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = require_org_id(&principal)?;
    let days_ahead = params.days_ahead.unwrap_or(30);

    let policies = state
        .insurance_repo
        .get_expiring_policies(org_id, days_ahead)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(ExpiringPoliciesResponse { policies }))
}

// ============================================
// Policy Document Handlers
// ============================================

/// List policy documents.
async fn list_policy_documents(
    State(state): State<AppState>,
    Path(policy_id): Path<Uuid>,
    _principal: RequestPrincipal,
) -> Result<Json<PolicyDocumentsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let documents = state
        .insurance_repo
        .list_policy_documents(policy_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(PolicyDocumentsResponse { documents }))
}

/// Add document to policy.
async fn add_policy_document(
    State(state): State<AppState>,
    Path(policy_id): Path<Uuid>,
    _principal: RequestPrincipal,
    Json(payload): Json<AddPolicyDocument>,
) -> Result<Json<InsurancePolicyDocument>, (StatusCode, Json<ErrorResponse>)> {
    let document = state
        .insurance_repo
        .add_policy_document(policy_id, payload)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(document))
}

/// Remove document from policy.
async fn remove_policy_document(
    State(state): State<AppState>,
    Path((policy_id, document_id)): Path<(Uuid, Uuid)>,
    _principal: RequestPrincipal,
) -> Result<Json<DeleteResponse>, (StatusCode, Json<ErrorResponse>)> {
    let success = state
        .insurance_repo
        .remove_policy_document(policy_id, document_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(DeleteResponse { success }))
}

// ============================================
// Renewal Reminder Handlers
// ============================================

/// List policy reminders.
async fn list_reminders(
    State(state): State<AppState>,
    Path(policy_id): Path<Uuid>,
    _principal: RequestPrincipal,
) -> Result<Json<RemindersResponse>, (StatusCode, Json<ErrorResponse>)> {
    let reminders = state
        .insurance_repo
        .list_policy_reminders(policy_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(RemindersResponse { reminders }))
}

/// Create reminder for policy.
async fn create_reminder(
    State(state): State<AppState>,
    Path(policy_id): Path<Uuid>,
    _principal: RequestPrincipal,
    Json(payload): Json<CreateRenewalReminder>,
) -> Result<Json<InsuranceRenewalReminder>, (StatusCode, Json<ErrorResponse>)> {
    let reminder = state
        .insurance_repo
        .create_reminder(policy_id, payload)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(reminder))
}

/// Update a reminder.
async fn update_reminder(
    State(state): State<AppState>,
    Path(reminder_id): Path<Uuid>,
    _principal: RequestPrincipal,
    Json(payload): Json<UpdateRenewalReminder>,
) -> Result<Json<InsuranceRenewalReminder>, (StatusCode, Json<ErrorResponse>)> {
    let reminder = state
        .insurance_repo
        .update_reminder(reminder_id, payload)
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
                Json(ErrorResponse::new("NOT_FOUND", "Reminder not found")),
            )
        })?;

    Ok(Json(reminder))
}

/// Delete a reminder.
async fn delete_reminder(
    State(state): State<AppState>,
    Path(reminder_id): Path<Uuid>,
    _principal: RequestPrincipal,
) -> Result<Json<DeleteResponse>, (StatusCode, Json<ErrorResponse>)> {
    let success = state
        .insurance_repo
        .delete_reminder(reminder_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(DeleteResponse { success }))
}

// ============================================
// Claim Handlers
// ============================================

/// List insurance claims.
async fn list_claims(
    State(state): State<AppState>,
    Query(query): Query<ListClaimsQuery>,
    principal: RequestPrincipal,
) -> Result<Json<ListClaimsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // SECURITY: the org is derived from the verified principal, never from a
    // client-supplied query param (issue #826 — was `building_id.unwrap_or_default()`).
    let org_id = require_org_id(&principal)?;

    let claims = state
        .insurance_repo
        .list_claims(org_id, query.into())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(ListClaimsResponse { claims }))
}

/// Create a new insurance claim.
async fn create_claim(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Json(payload): Json<CreateClaimRequest>,
) -> Result<Json<InsuranceClaim>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = require_org_id(&principal)?;
    let claim = state
        .insurance_repo
        .create_claim(org_id, principal.user_id, payload.data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(claim))
}

/// Get a claim by ID.
async fn get_claim(
    State(state): State<AppState>,
    Path(claim_id): Path<Uuid>,
    principal: RequestPrincipal,
) -> Result<Json<InsuranceClaimWithPolicy>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = require_org_id(&principal)?;
    let claim = state
        .insurance_repo
        .find_claim_with_policy(org_id, claim_id)
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
                Json(ErrorResponse::new("NOT_FOUND", "Claim not found")),
            )
        })?;

    Ok(Json(claim))
}

/// Update a claim.
async fn update_claim(
    State(state): State<AppState>,
    Path(claim_id): Path<Uuid>,
    principal: RequestPrincipal,
    Json(payload): Json<UpdateClaimRequest>,
) -> Result<Json<InsuranceClaim>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = require_org_id(&principal)?;
    let claim = state
        .insurance_repo
        .update_claim(org_id, claim_id, payload.data)
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
                Json(ErrorResponse::new("NOT_FOUND", "Claim not found")),
            )
        })?;

    Ok(Json(claim))
}

/// Submit a claim for review.
async fn submit_claim(
    State(state): State<AppState>,
    Path(claim_id): Path<Uuid>,
    principal: RequestPrincipal,
) -> Result<Json<InsuranceClaim>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = require_org_id(&principal)?;
    let claim = state
        .insurance_repo
        .submit_claim(org_id, claim_id, principal.user_id)
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
                    "Claim not found or already submitted",
                )),
            )
        })?;

    Ok(Json(claim))
}

/// Review a claim (approve/deny).
async fn review_claim(
    State(state): State<AppState>,
    Path(claim_id): Path<Uuid>,
    principal: RequestPrincipal,
    Json(payload): Json<ReviewClaimRequest>,
) -> Result<Json<InsuranceClaim>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = require_org_id(&principal)?;
    let claim = state
        .insurance_repo
        .review_claim(
            org_id,
            claim_id,
            principal.user_id,
            &payload.status,
            payload.approved_amount,
            payload.denial_reason.as_deref(),
            payload.resolution_notes.as_deref(),
        )
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
                Json(ErrorResponse::new("NOT_FOUND", "Claim not found")),
            )
        })?;

    Ok(Json(claim))
}

/// Record payment for a claim.
async fn record_claim_payment(
    State(state): State<AppState>,
    Path(claim_id): Path<Uuid>,
    principal: RequestPrincipal,
    Json(payload): Json<RecordClaimPaymentRequest>,
) -> Result<Json<InsuranceClaim>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = require_org_id(&principal)?;
    let claim = state
        .insurance_repo
        .record_claim_payment(org_id, claim_id, payload.payment_amount)
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
                Json(ErrorResponse::new("NOT_FOUND", "Claim not found")),
            )
        })?;

    Ok(Json(claim))
}

/// Delete a claim (only drafts).
async fn delete_claim(
    State(state): State<AppState>,
    Path(claim_id): Path<Uuid>,
    principal: RequestPrincipal,
) -> Result<Json<DeleteResponse>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = require_org_id(&principal)?;
    let success = state
        .insurance_repo
        .delete_claim(org_id, claim_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(DeleteResponse { success }))
}

/// Get claim history.
async fn get_claim_history(
    State(state): State<AppState>,
    Path(claim_id): Path<Uuid>,
    _principal: RequestPrincipal,
) -> Result<Json<ClaimHistoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    let history = state
        .insurance_repo
        .get_claim_history(claim_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(ClaimHistoryResponse { history }))
}

// ============================================
// Claim Document Handlers
// ============================================

/// List claim documents.
async fn list_claim_documents(
    State(state): State<AppState>,
    Path(claim_id): Path<Uuid>,
    _principal: RequestPrincipal,
) -> Result<Json<ClaimDocumentsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let documents = state
        .insurance_repo
        .list_claim_documents(claim_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(ClaimDocumentsResponse { documents }))
}

/// Add document to claim.
async fn add_claim_document(
    State(state): State<AppState>,
    Path(claim_id): Path<Uuid>,
    _principal: RequestPrincipal,
    Json(payload): Json<AddClaimDocument>,
) -> Result<Json<InsuranceClaimDocument>, (StatusCode, Json<ErrorResponse>)> {
    let document = state
        .insurance_repo
        .add_claim_document(claim_id, payload)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(document))
}

/// Remove document from claim.
async fn remove_claim_document(
    State(state): State<AppState>,
    Path((claim_id, document_id)): Path<(Uuid, Uuid)>,
    _principal: RequestPrincipal,
) -> Result<Json<DeleteResponse>, (StatusCode, Json<ErrorResponse>)> {
    let success = state
        .insurance_repo
        .remove_claim_document(claim_id, document_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(DeleteResponse { success }))
}

// ============================================
// Statistics Handler
// ============================================

/// Get insurance statistics.
async fn get_statistics(
    State(state): State<AppState>,
    principal: RequestPrincipal,
) -> Result<Json<StatisticsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = require_org_id(&principal)?;
    let statistics = state
        .insurance_repo
        .get_statistics(org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    let claims_by_status = state
        .insurance_repo
        .get_claim_summary_by_status(org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    let policies_by_type = state
        .insurance_repo
        .get_policy_summary_by_type(org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(StatisticsResponse {
        statistics,
        claims_by_status,
        policies_by_type,
    }))
}

// ============================================
// Helper Query Types
// ============================================

/// Query parameter for expiring policies.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ExpiringQuery {
    pub days_ahead: Option<i32>,
}
