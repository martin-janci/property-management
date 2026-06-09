//! Insurance management routes for Epic 22.
//!
//! Handles insurance policies, claims, documents, and renewal reminders.
//!
//! # Authentication & tenancy (SECURITY-CRITICAL)
//!
//! # RLS (PAP-67)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` on every insurance table, so
//! each query MUST run on a connection that has `app.current_org_id` set or it
//! collapses to deny-all. Each handler therefore acquires an [`RlsConnection`]
//! (which validates tenant membership and sets the org/user GUCs on a dedicated
//! connection) and passes `&mut **rls.conn()` to the repository. The
//! authoritative organization is `rls.tenant_id()` — the tenant the caller was
//! validated against — not a client-supplied `organization_id`, so the SQL org
//! filter and the RLS context can never disagree. Cross-tenant access is blocked
//! by RLS: a by-id read of another org's row returns no row (`404`), and a write
//! targeting another org fails the policy's `WITH CHECK`. `rls.release()` clears
//! the context before the connection returns to the pool.
//!
//! This replaces the previous `RequestPrincipal` + `require_org_id` scheme that
//! derived the org from the principal but ran every query on a raw pool (issue
//! #826 closed the client-supplied-org IDOR; PAP-67 closes the deny-all under
//! `FORCE` and pushes cross-tenant enforcement into the database).

use api_core::extractors::RlsConnection;
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
// Error Helpers
// ============================================

/// Map a repository error to a `500` with a stable code, logging the cause.
fn db_error(msg: &'static str, e: sqlx::Error) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!("{}: {:?}", msg, e);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new("DB_ERROR", msg)),
    )
}

/// Build a `404` response.
fn not_found(msg: &'static str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", msg)),
    )
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
/// The owning organization is derived from the RLS-validated tenant
/// (`rls.tenant_id()`), never from the request body (issue #826).
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
    mut rls: RlsConnection,
    Query(query): Query<ListPoliciesQuery>,
) -> Result<Json<ListPoliciesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .insurance_repo
        .list_policies(&mut **rls.conn(), org_id, query.into())
        .await
        .map(|policies| Json(ListPoliciesResponse { policies }))
        .map_err(|e| db_error("Failed to list policies", e));
    rls.release().await;
    out
}

/// Create a new insurance policy.
async fn create_policy(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(payload): Json<CreatePolicyRequest>,
) -> Result<Json<InsurancePolicy>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .insurance_repo
        .create_policy(&mut **rls.conn(), org_id, payload.data)
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to create policy", e));
    rls.release().await;
    out
}

/// Get a policy by ID.
async fn get_policy(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(policy_id): Path<Uuid>,
) -> Result<Json<InsurancePolicy>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .insurance_repo
        .find_policy_by_id(&mut **rls.conn(), org_id, policy_id)
        .await
        .map_err(|e| db_error("Failed to get policy", e))
        .and_then(|p| p.map(Json).ok_or_else(|| not_found("Policy not found")));
    rls.release().await;
    out
}

/// Update a policy.
async fn update_policy(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(policy_id): Path<Uuid>,
    Json(payload): Json<UpdatePolicyRequest>,
) -> Result<Json<InsurancePolicy>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .insurance_repo
        .update_policy(&mut **rls.conn(), org_id, policy_id, payload.data)
        .await
        .map_err(|e| db_error("Failed to update policy", e))
        .and_then(|p| p.map(Json).ok_or_else(|| not_found("Policy not found")));
    rls.release().await;
    out
}

/// Delete a policy.
async fn delete_policy(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(policy_id): Path<Uuid>,
) -> Result<Json<DeleteResponse>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .insurance_repo
        .delete_policy(&mut **rls.conn(), org_id, policy_id)
        .await
        .map(|success| Json(DeleteResponse { success }))
        .map_err(|e| db_error("Failed to delete policy", e));
    rls.release().await;
    out
}

/// Get expiring policies.
async fn get_expiring_policies(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(params): Query<ExpiringQuery>,
) -> Result<Json<ExpiringPoliciesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let days_ahead = params.days_ahead.unwrap_or(30);
    let out = state
        .insurance_repo
        .get_expiring_policies(&mut **rls.conn(), org_id, days_ahead)
        .await
        .map(|policies| Json(ExpiringPoliciesResponse { policies }))
        .map_err(|e| db_error("Failed to get expiring policies", e));
    rls.release().await;
    out
}

// ============================================
// Policy Document Handlers
// ============================================

/// List policy documents.
async fn list_policy_documents(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(policy_id): Path<Uuid>,
) -> Result<Json<PolicyDocumentsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .insurance_repo
        .list_policy_documents(&mut **rls.conn(), policy_id)
        .await
        .map(|documents| Json(PolicyDocumentsResponse { documents }))
        .map_err(|e| db_error("Failed to list policy documents", e));
    rls.release().await;
    out
}

/// Add document to policy.
async fn add_policy_document(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(policy_id): Path<Uuid>,
    Json(payload): Json<AddPolicyDocument>,
) -> Result<Json<InsurancePolicyDocument>, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .insurance_repo
        .add_policy_document(&mut **rls.conn(), policy_id, payload)
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to add policy document", e));
    rls.release().await;
    out
}

/// Remove document from policy.
async fn remove_policy_document(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((policy_id, document_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<DeleteResponse>, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .insurance_repo
        .remove_policy_document(&mut **rls.conn(), policy_id, document_id)
        .await
        .map(|success| Json(DeleteResponse { success }))
        .map_err(|e| db_error("Failed to remove policy document", e));
    rls.release().await;
    out
}

// ============================================
// Renewal Reminder Handlers
// ============================================

/// List policy reminders.
async fn list_reminders(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(policy_id): Path<Uuid>,
) -> Result<Json<RemindersResponse>, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .insurance_repo
        .list_policy_reminders(&mut **rls.conn(), policy_id)
        .await
        .map(|reminders| Json(RemindersResponse { reminders }))
        .map_err(|e| db_error("Failed to list reminders", e));
    rls.release().await;
    out
}

/// Create reminder for policy.
async fn create_reminder(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(policy_id): Path<Uuid>,
    Json(payload): Json<CreateRenewalReminder>,
) -> Result<Json<InsuranceRenewalReminder>, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .insurance_repo
        .create_reminder(&mut **rls.conn(), policy_id, payload)
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to create reminder", e));
    rls.release().await;
    out
}

/// Update a reminder.
async fn update_reminder(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(reminder_id): Path<Uuid>,
    Json(payload): Json<UpdateRenewalReminder>,
) -> Result<Json<InsuranceRenewalReminder>, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .insurance_repo
        .update_reminder(&mut **rls.conn(), reminder_id, payload)
        .await
        .map_err(|e| db_error("Failed to update reminder", e))
        .and_then(|r| r.map(Json).ok_or_else(|| not_found("Reminder not found")));
    rls.release().await;
    out
}

/// Delete a reminder.
async fn delete_reminder(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(reminder_id): Path<Uuid>,
) -> Result<Json<DeleteResponse>, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .insurance_repo
        .delete_reminder(&mut **rls.conn(), reminder_id)
        .await
        .map(|success| Json(DeleteResponse { success }))
        .map_err(|e| db_error("Failed to delete reminder", e));
    rls.release().await;
    out
}

// ============================================
// Claim Handlers
// ============================================

/// List insurance claims.
async fn list_claims(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListClaimsQuery>,
) -> Result<Json<ListClaimsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .insurance_repo
        .list_claims(&mut **rls.conn(), org_id, query.into())
        .await
        .map(|claims| Json(ListClaimsResponse { claims }))
        .map_err(|e| db_error("Failed to list claims", e));
    rls.release().await;
    out
}

/// Create a new insurance claim.
async fn create_claim(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(payload): Json<CreateClaimRequest>,
) -> Result<Json<InsuranceClaim>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .insurance_repo
        .create_claim(&mut **rls.conn(), org_id, user_id, payload.data)
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to create claim", e));
    rls.release().await;
    out
}

/// Get a claim by ID.
async fn get_claim(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(claim_id): Path<Uuid>,
) -> Result<Json<InsuranceClaimWithPolicy>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .insurance_repo
        .find_claim_with_policy(&mut **rls.conn(), org_id, claim_id)
        .await
        .map_err(|e| db_error("Failed to get claim", e))
        .and_then(|c| c.map(Json).ok_or_else(|| not_found("Claim not found")));
    rls.release().await;
    out
}

/// Update a claim.
async fn update_claim(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(claim_id): Path<Uuid>,
    Json(payload): Json<UpdateClaimRequest>,
) -> Result<Json<InsuranceClaim>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .insurance_repo
        .update_claim(&mut **rls.conn(), org_id, claim_id, payload.data)
        .await
        .map_err(|e| db_error("Failed to update claim", e))
        .and_then(|c| c.map(Json).ok_or_else(|| not_found("Claim not found")));
    rls.release().await;
    out
}

/// Submit a claim for review.
async fn submit_claim(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(claim_id): Path<Uuid>,
) -> Result<Json<InsuranceClaim>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .insurance_repo
        .submit_claim(&mut **rls.conn(), org_id, claim_id, user_id)
        .await
        .map_err(|e| db_error("Failed to submit claim", e))
        .and_then(|c| {
            c.map(Json)
                .ok_or_else(|| not_found("Claim not found or already submitted"))
        });
    rls.release().await;
    out
}

/// Review a claim (approve/deny).
async fn review_claim(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(claim_id): Path<Uuid>,
    Json(payload): Json<ReviewClaimRequest>,
) -> Result<Json<InsuranceClaim>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .insurance_repo
        .review_claim(
            &mut **rls.conn(),
            org_id,
            claim_id,
            user_id,
            &payload.status,
            payload.approved_amount,
            payload.denial_reason.as_deref(),
            payload.resolution_notes.as_deref(),
        )
        .await
        .map_err(|e| db_error("Failed to review claim", e))
        .and_then(|c| c.map(Json).ok_or_else(|| not_found("Claim not found")));
    rls.release().await;
    out
}

/// Record payment for a claim.
async fn record_claim_payment(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(claim_id): Path<Uuid>,
    Json(payload): Json<RecordClaimPaymentRequest>,
) -> Result<Json<InsuranceClaim>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .insurance_repo
        .record_claim_payment(&mut **rls.conn(), org_id, claim_id, payload.payment_amount)
        .await
        .map_err(|e| db_error("Failed to record claim payment", e))
        .and_then(|c| c.map(Json).ok_or_else(|| not_found("Claim not found")));
    rls.release().await;
    out
}

/// Delete a claim (only drafts).
async fn delete_claim(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(claim_id): Path<Uuid>,
) -> Result<Json<DeleteResponse>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .insurance_repo
        .delete_claim(&mut **rls.conn(), org_id, claim_id)
        .await
        .map(|success| Json(DeleteResponse { success }))
        .map_err(|e| db_error("Failed to delete claim", e));
    rls.release().await;
    out
}

/// Get claim history.
async fn get_claim_history(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(claim_id): Path<Uuid>,
) -> Result<Json<ClaimHistoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .insurance_repo
        .get_claim_history(&mut **rls.conn(), claim_id)
        .await
        .map(|history| Json(ClaimHistoryResponse { history }))
        .map_err(|e| db_error("Failed to get claim history", e));
    rls.release().await;
    out
}

// ============================================
// Claim Document Handlers
// ============================================

/// List claim documents.
async fn list_claim_documents(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(claim_id): Path<Uuid>,
) -> Result<Json<ClaimDocumentsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .insurance_repo
        .list_claim_documents(&mut **rls.conn(), claim_id)
        .await
        .map(|documents| Json(ClaimDocumentsResponse { documents }))
        .map_err(|e| db_error("Failed to list claim documents", e));
    rls.release().await;
    out
}

/// Add document to claim.
async fn add_claim_document(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(claim_id): Path<Uuid>,
    Json(payload): Json<AddClaimDocument>,
) -> Result<Json<InsuranceClaimDocument>, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .insurance_repo
        .add_claim_document(&mut **rls.conn(), claim_id, payload)
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to add claim document", e));
    rls.release().await;
    out
}

/// Remove document from claim.
async fn remove_claim_document(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((claim_id, document_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<DeleteResponse>, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .insurance_repo
        .remove_claim_document(&mut **rls.conn(), claim_id, document_id)
        .await
        .map(|success| Json(DeleteResponse { success }))
        .map_err(|e| db_error("Failed to remove claim document", e));
    rls.release().await;
    out
}

// ============================================
// Statistics Handler
// ============================================

/// Get insurance statistics.
async fn get_statistics(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<StatisticsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = async {
        let statistics = state
            .insurance_repo
            .get_statistics(&mut **rls.conn(), org_id)
            .await
            .map_err(|e| db_error("Failed to get statistics", e))?;

        let claims_by_status = state
            .insurance_repo
            .get_claim_summary_by_status(&mut **rls.conn(), org_id)
            .await
            .map_err(|e| db_error("Failed to get claim summary", e))?;

        let policies_by_type = state
            .insurance_repo
            .get_policy_summary_by_type(&mut **rls.conn(), org_id)
            .await
            .map_err(|e| db_error("Failed to get policy summary", e))?;

        Ok(Json(StatisticsResponse {
            statistics,
            claims_by_status,
            policies_by_type,
        }))
    }
    .await;
    rls.release().await;
    out
}

// ============================================
// Helper Query Types
// ============================================

/// Query parameter for expiring policies.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ExpiringQuery {
    pub days_ahead: Option<i32>,
}
