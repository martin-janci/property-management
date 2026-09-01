//! Legal document and compliance routes (Epic 25).
//!
//! # RLS (PAP-80 / PAP-67)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` on all eight
//! legal/compliance tables, so every query MUST run on a connection that has
//! `app.current_org_id` set or it collapses to deny-all. Each handler therefore
//! acquires an [`RlsConnection`] (which authenticates the caller, validates
//! tenant membership, and sets the org/user GUCs on a dedicated connection) and
//! passes `&mut **rls.conn()` to the repository. The authoritative organization
//! is `rls.tenant_id()` — the tenant the caller was validated against — not a
//! client-supplied `organization_id`, so the SQL org filter and the RLS context
//! can never disagree. Cross-tenant access is blocked by RLS: a by-id read of
//! another org's row returns no row (`404`), and a write targeting another org
//! fails the policy's `WITH CHECK`. `rls.release()` clears the context before
//! the connection returns to the pool.

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
    Json, Router,
};
use chrono::NaiveDate;
use common::errors::ErrorResponse;
use db::models::{
    AcknowledgeNotice, ApplyTemplate, ComplianceAuditTrail, ComplianceQuery, ComplianceRequirement,
    ComplianceRequirementWithDetails, ComplianceStatistics, ComplianceTemplate,
    ComplianceVerification, CreateAuditTrailEntry, CreateComplianceRequirement,
    CreateComplianceTemplate, CreateComplianceVerification, CreateLegalDocument,
    CreateLegalDocumentVersion, CreateLegalNotice, LegalDocument, LegalDocumentQuery,
    LegalDocumentSummary, LegalDocumentVersion, LegalNotice, LegalNoticeQuery,
    LegalNoticeRecipient, NoticeStatistics, NoticeWithRecipients, UpdateComplianceRequirement,
    UpdateComplianceTemplate, UpdateLegalDocument, UpdateLegalNotice,
};
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::routes::pagination::clamp_limit;
use crate::state::AppState;

/// Create legal routes router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Legal Documents
        .route("/documents", post(create_document))
        .route("/documents", get(list_documents))
        .route("/documents/summary", get(list_documents_summary))
        .route("/documents/{id}", get(get_document))
        .route("/documents/{id}", patch(update_document))
        .route("/documents/{id}", delete(delete_document))
        // Document Versions
        .route("/documents/{id}/versions", post(add_version))
        .route("/documents/{id}/versions", get(list_versions))
        .route("/documents/{id}/versions/{version}", get(get_version))
        // Compliance Requirements
        .route("/requirements", post(create_requirement))
        .route("/requirements", get(list_requirements))
        .route(
            "/requirements/with-details",
            get(list_requirements_with_details),
        )
        .route("/requirements/statistics", get(get_compliance_statistics))
        .route("/requirements/{id}", get(get_requirement))
        .route("/requirements/{id}", patch(update_requirement))
        .route("/requirements/{id}", delete(delete_requirement))
        // Compliance Verifications
        .route("/requirements/{id}/verify", post(create_verification))
        .route("/requirements/{id}/verifications", get(list_verifications))
        // Legal Notices
        .route("/notices", post(create_notice))
        .route("/notices", get(list_notices))
        .route(
            "/notices/with-recipients",
            get(list_notices_with_recipients),
        )
        .route("/notices/statistics", get(get_notice_statistics))
        .route("/notices/{id}", get(get_notice))
        .route("/notices/{id}", patch(update_notice))
        .route("/notices/{id}", delete(delete_notice))
        .route("/notices/{id}/send", post(send_notice))
        // Notice Recipients
        .route("/notices/{id}/recipients", get(list_recipients))
        .route(
            "/notices/{notice_id}/acknowledge/{recipient_id}",
            post(acknowledge_notice),
        )
        // Compliance Templates
        .route("/templates", post(create_template))
        .route("/templates", get(list_templates))
        .route("/templates/{id}", get(get_template))
        .route("/templates/{id}", patch(update_template))
        .route("/templates/{id}", delete(delete_template))
        .route("/templates/apply", post(apply_template))
        // Audit Trail
        .route("/audit-trail", get(list_audit_trail))
        .route("/audit-trail", post(create_audit_entry))
}

// ==================== Error Helpers ====================

/// Map a repository error to a `500`, logging the cause under `msg`.
fn db_error(msg: &str, e: sqlx::Error) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!("{}: {:?}", msg, e);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new("DB_ERROR", e.to_string())),
    )
}

/// Build a `404` response.
fn not_found(msg: &'static str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", msg)),
    )
}

// ==================== Request/Response Types ====================

/// Create legal document request wrapper.
///
/// `organization_id` is accepted for backwards compatibility but ignored — the
/// authoritative org is the caller's validated tenant (`rls.tenant_id()`).
#[derive(Debug, Deserialize)]
pub struct CreateDocumentRequest {
    pub organization_id: Option<Uuid>,
    #[serde(flatten)]
    pub data: CreateLegalDocument,
}

/// List legal documents query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListDocumentsQuery {
    pub organization_id: Option<Uuid>,
    pub building_id: Option<Uuid>,
    pub document_type: Option<String>,
    pub is_confidential: Option<bool>,
    pub expiring_days: Option<i32>,
    pub tag: Option<String>,
    pub search: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl From<&ListDocumentsQuery> for LegalDocumentQuery {
    fn from(q: &ListDocumentsQuery) -> Self {
        LegalDocumentQuery {
            building_id: q.building_id,
            document_type: q.document_type.clone(),
            is_confidential: q.is_confidential,
            expiring_days: q.expiring_days,
            tag: q.tag.clone(),
            search: q.search.clone(),
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Create compliance requirement request wrapper.
#[derive(Debug, Deserialize)]
pub struct CreateRequirementRequest {
    pub organization_id: Option<Uuid>,
    #[serde(flatten)]
    pub data: CreateComplianceRequirement,
}

/// List compliance requirements query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListRequirementsQuery {
    pub organization_id: Option<Uuid>,
    pub building_id: Option<Uuid>,
    pub category: Option<String>,
    pub status: Option<String>,
    pub is_mandatory: Option<bool>,
    pub due_before: Option<NaiveDate>,
    pub overdue_only: Option<bool>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl From<&ListRequirementsQuery> for ComplianceQuery {
    fn from(q: &ListRequirementsQuery) -> Self {
        ComplianceQuery {
            building_id: q.building_id,
            category: q.category.clone(),
            status: q.status.clone(),
            is_mandatory: q.is_mandatory,
            due_before: q.due_before,
            overdue_only: q.overdue_only,
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Create legal notice request wrapper.
#[derive(Debug, Deserialize)]
pub struct CreateNoticeRequest {
    pub organization_id: Option<Uuid>,
    #[serde(flatten)]
    pub data: CreateLegalNotice,
}

/// List legal notices query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListNoticesQuery {
    pub organization_id: Option<Uuid>,
    pub building_id: Option<Uuid>,
    pub notice_type: Option<String>,
    pub priority: Option<String>,
    pub sent: Option<bool>,
    pub requires_acknowledgment: Option<bool>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl From<&ListNoticesQuery> for LegalNoticeQuery {
    fn from(q: &ListNoticesQuery) -> Self {
        LegalNoticeQuery {
            building_id: q.building_id,
            notice_type: q.notice_type.clone(),
            priority: q.priority.clone(),
            sent: q.sent,
            requires_acknowledgment: q.requires_acknowledgment,
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Create template request wrapper.
#[derive(Debug, Deserialize)]
pub struct CreateTemplateRequest {
    pub organization_id: Option<Uuid>,
    #[serde(flatten)]
    pub data: CreateComplianceTemplate,
}

/// List templates query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListTemplatesQuery {
    pub organization_id: Option<Uuid>,
    pub category: Option<String>,
}

/// Apply template request.
#[derive(Debug, Deserialize)]
pub struct ApplyTemplateRequest {
    pub organization_id: Option<Uuid>,
    #[serde(flatten)]
    pub data: ApplyTemplate,
}

/// Create audit entry request wrapper.
#[derive(Debug, Deserialize)]
pub struct CreateAuditRequest {
    pub organization_id: Option<Uuid>,
    #[serde(flatten)]
    pub data: CreateAuditTrailEntry,
}

/// Audit trail query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct AuditTrailQuery {
    pub organization_id: Option<Uuid>,
    pub requirement_id: Option<Uuid>,
    pub document_id: Option<Uuid>,
    pub notice_id: Option<Uuid>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

// ==================== Legal Document Endpoints ====================

async fn create_document(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(payload): Json<CreateDocumentRequest>,
) -> Result<(StatusCode, Json<LegalDocument>), (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .legal_repo
        .create_document(&mut **rls.conn(), org_id, user_id, payload.data)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(|e| db_error("Failed to create legal document", e));
    rls.release().await;
    out
}

async fn list_documents(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListDocumentsQuery>,
) -> Result<Json<Vec<LegalDocument>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .list_documents(&mut **rls.conn(), org_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to list legal documents", e));
    rls.release().await;
    out
}

async fn list_documents_summary(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListDocumentsQuery>,
) -> Result<Json<Vec<LegalDocumentSummary>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .list_documents_with_summary(&mut **rls.conn(), org_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to list legal documents", e));
    rls.release().await;
    out
}

async fn get_document(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<LegalDocument>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .find_document_by_id(&mut **rls.conn(), id, org_id)
        .await
        .map_err(|e| db_error("Failed to get legal document", e))
        .and_then(|opt| opt.map(Json).ok_or_else(|| not_found("Document not found")));
    rls.release().await;
    out
}

async fn update_document(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateLegalDocument>,
) -> Result<Json<LegalDocument>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .update_document(&mut **rls.conn(), id, org_id, data)
        .await
        .map_err(|e| db_error("Failed to update legal document", e))
        .and_then(|opt| opt.map(Json).ok_or_else(|| not_found("Document not found")));
    rls.release().await;
    out
}

async fn delete_document(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .delete_document(&mut **rls.conn(), id, org_id)
        .await
        .map_err(|e| db_error("Failed to delete legal document", e))
        .and_then(|deleted| {
            if deleted {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err(not_found("Document not found"))
            }
        });
    rls.release().await;
    out
}

// ==================== Document Versions ====================

async fn add_version(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<CreateLegalDocumentVersion>,
) -> Result<(StatusCode, Json<LegalDocumentVersion>), (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .legal_repo
        .add_document_version(rls.conn(), id, org_id, user_id, data)
        .await
        .map_err(|e| db_error("Failed to add document version", e))
        .and_then(|opt| {
            opt.map(|v| (StatusCode::CREATED, Json(v)))
                .ok_or_else(|| not_found("Document not found"))
        });
    rls.release().await;
    out
}

async fn list_versions(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<LegalDocumentVersion>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .list_document_versions(rls.conn(), id, org_id)
        .await
        .map_err(|e| db_error("Failed to list document versions", e))
        .and_then(|opt| opt.map(Json).ok_or_else(|| not_found("Document not found")));
    rls.release().await;
    out
}

async fn get_version(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((id, version)): Path<(Uuid, i32)>,
) -> Result<Json<LegalDocumentVersion>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .get_document_version(rls.conn(), id, org_id, version)
        .await
        .map_err(|e| db_error("Failed to get document version", e))
        .and_then(|opt| opt.map(Json).ok_or_else(|| not_found("Version not found")));
    rls.release().await;
    out
}

// ==================== Compliance Requirements ====================

async fn create_requirement(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(payload): Json<CreateRequirementRequest>,
) -> Result<(StatusCode, Json<ComplianceRequirement>), (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .create_requirement(&mut **rls.conn(), org_id, payload.data)
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(|e| db_error("Failed to create compliance requirement", e));
    rls.release().await;
    out
}

async fn list_requirements(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListRequirementsQuery>,
) -> Result<Json<Vec<ComplianceRequirement>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .list_requirements(&mut **rls.conn(), org_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to list compliance requirements", e));
    rls.release().await;
    out
}

async fn list_requirements_with_details(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListRequirementsQuery>,
) -> Result<Json<Vec<ComplianceRequirementWithDetails>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .list_requirements_with_details(&mut **rls.conn(), org_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to list compliance requirements", e));
    rls.release().await;
    out
}

async fn get_compliance_statistics(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<ComplianceStatistics>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .get_compliance_statistics(rls.conn(), org_id)
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to get compliance statistics", e));
    rls.release().await;
    out
}

async fn get_requirement(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<ComplianceRequirement>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .find_requirement_by_id(&mut **rls.conn(), id, org_id)
        .await
        .map_err(|e| db_error("Failed to get compliance requirement", e))
        .and_then(|opt| {
            opt.map(Json)
                .ok_or_else(|| not_found("Requirement not found"))
        });
    rls.release().await;
    out
}

async fn update_requirement(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateComplianceRequirement>,
) -> Result<Json<ComplianceRequirement>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .update_requirement(&mut **rls.conn(), id, org_id, data)
        .await
        .map_err(|e| db_error("Failed to update compliance requirement", e))
        .and_then(|opt| {
            opt.map(Json)
                .ok_or_else(|| not_found("Requirement not found"))
        });
    rls.release().await;
    out
}

async fn delete_requirement(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .delete_requirement(&mut **rls.conn(), id, org_id)
        .await
        .map_err(|e| db_error("Failed to delete compliance requirement", e))
        .and_then(|deleted| {
            if deleted {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err(not_found("Requirement not found"))
            }
        });
    rls.release().await;
    out
}

// ==================== Compliance Verifications ====================

async fn create_verification(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<CreateComplianceVerification>,
) -> Result<(StatusCode, Json<ComplianceVerification>), (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .legal_repo
        .create_verification(rls.conn(), id, org_id, user_id, data)
        .await
        .map_err(|e| db_error("Failed to create compliance verification", e))
        .and_then(|opt| {
            opt.map(|v| (StatusCode::CREATED, Json(v)))
                .ok_or_else(|| not_found("Requirement not found"))
        });
    rls.release().await;
    out
}

async fn list_verifications(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ComplianceVerification>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .list_verifications(rls.conn(), id, org_id)
        .await
        .map_err(|e| db_error("Failed to list compliance verifications", e))
        .and_then(|opt| {
            opt.map(Json)
                .ok_or_else(|| not_found("Requirement not found"))
        });
    rls.release().await;
    out
}

// ==================== Legal Notices ====================

async fn create_notice(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(payload): Json<CreateNoticeRequest>,
) -> Result<(StatusCode, Json<LegalNotice>), (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .legal_repo
        .create_notice(rls.conn(), org_id, user_id, payload.data)
        .await
        .map(|n| (StatusCode::CREATED, Json(n)))
        .map_err(|e| db_error("Failed to create legal notice", e));
    rls.release().await;
    out
}

async fn list_notices(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListNoticesQuery>,
) -> Result<Json<Vec<LegalNotice>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .list_notices(&mut **rls.conn(), org_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to list legal notices", e));
    rls.release().await;
    out
}

async fn list_notices_with_recipients(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListNoticesQuery>,
) -> Result<Json<Vec<NoticeWithRecipients>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .list_notices_with_recipients(&mut **rls.conn(), org_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to list legal notices", e));
    rls.release().await;
    out
}

async fn get_notice_statistics(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<NoticeStatistics>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .get_notice_statistics(rls.conn(), org_id)
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to get notice statistics", e));
    rls.release().await;
    out
}

async fn get_notice(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<LegalNotice>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .find_notice_by_id(&mut **rls.conn(), id, org_id)
        .await
        .map_err(|e| db_error("Failed to get legal notice", e))
        .and_then(|opt| opt.map(Json).ok_or_else(|| not_found("Notice not found")));
    rls.release().await;
    out
}

async fn update_notice(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateLegalNotice>,
) -> Result<Json<LegalNotice>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .update_notice(&mut **rls.conn(), id, org_id, data)
        .await
        .map_err(|e| db_error("Failed to update legal notice", e))
        .and_then(|opt| opt.map(Json).ok_or_else(|| not_found("Notice not found")));
    rls.release().await;
    out
}

async fn delete_notice(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .delete_notice(&mut **rls.conn(), id, org_id)
        .await
        .map_err(|e| db_error("Failed to delete legal notice", e))
        .and_then(|deleted| {
            if deleted {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err(not_found("Notice not found"))
            }
        });
    rls.release().await;
    out
}

async fn send_notice(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<LegalNotice>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .send_notice(rls.conn(), id, org_id)
        .await
        .map_err(|e| db_error("Failed to send legal notice", e))
        .and_then(|opt| opt.map(Json).ok_or_else(|| not_found("Notice not found")));
    rls.release().await;
    out
}

// ==================== Notice Recipients ====================

async fn list_recipients(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<LegalNoticeRecipient>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .list_notice_recipients(rls.conn(), id, org_id)
        .await
        .map_err(|e| db_error("Failed to list notice recipients", e))
        .and_then(|opt| opt.map(Json).ok_or_else(|| not_found("Notice not found")));
    rls.release().await;
    out
}

async fn acknowledge_notice(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((notice_id, recipient_id)): Path<(Uuid, Uuid)>,
    Json(data): Json<AcknowledgeNotice>,
) -> Result<Json<LegalNoticeRecipient>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .acknowledge_notice(rls.conn(), notice_id, recipient_id, org_id, data)
        .await
        .map_err(|e| db_error("Failed to acknowledge notice", e))
        .and_then(|opt| {
            opt.map(Json)
                .ok_or_else(|| not_found("Notice or recipient not found"))
        });
    rls.release().await;
    out
}

// ==================== Compliance Templates ====================

async fn create_template(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(payload): Json<CreateTemplateRequest>,
) -> Result<(StatusCode, Json<ComplianceTemplate>), (StatusCode, Json<ErrorResponse>)> {
    // Authoritative org is the caller's tenant; a tenant cannot create a global
    // (NULL-org) system template through this endpoint.
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .create_template(&mut **rls.conn(), Some(org_id), payload.data)
        .await
        .map(|t| (StatusCode::CREATED, Json(t)))
        .map_err(|e| db_error("Failed to create compliance template", e));
    rls.release().await;
    out
}

async fn list_templates(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListTemplatesQuery>,
) -> Result<Json<Vec<ComplianceTemplate>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .list_templates(&mut **rls.conn(), org_id, query.category)
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to list compliance templates", e));
    rls.release().await;
    out
}

async fn get_template(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<ComplianceTemplate>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .find_template_by_id(&mut **rls.conn(), id, org_id)
        .await
        .map_err(|e| db_error("Failed to get compliance template", e))
        .and_then(|opt| opt.map(Json).ok_or_else(|| not_found("Template not found")));
    rls.release().await;
    out
}

async fn update_template(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateComplianceTemplate>,
) -> Result<Json<ComplianceTemplate>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .update_template(&mut **rls.conn(), id, org_id, data)
        .await
        .map_err(|e| db_error("Failed to update compliance template", e))
        .and_then(|opt| {
            opt.map(Json)
                .ok_or_else(|| not_found("Template not found (or is system template)"))
        });
    rls.release().await;
    out
}

async fn delete_template(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .delete_template(&mut **rls.conn(), id, org_id)
        .await
        .map_err(|e| db_error("Failed to delete compliance template", e))
        .and_then(|deleted| {
            if deleted {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err(not_found("Template not found (or is system template)"))
            }
        });
    rls.release().await;
    out
}

async fn apply_template(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(payload): Json<ApplyTemplateRequest>,
) -> Result<(StatusCode, Json<Vec<ComplianceRequirement>>), (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .apply_template(rls.conn(), org_id, payload.data)
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(|e| db_error("Failed to apply compliance template", e));
    rls.release().await;
    out
}

// ==================== Audit Trail ====================

async fn list_audit_trail(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<AuditTrailQuery>,
) -> Result<Json<Vec<ComplianceAuditTrail>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .legal_repo
        .list_audit_trail(
            &mut **rls.conn(),
            org_id,
            query.requirement_id,
            query.document_id,
            query.notice_id,
            clamp_limit(query.limit.map(i64::from), 50) as i32,
            query.offset.unwrap_or(0),
        )
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to list audit trail", e));
    rls.release().await;
    out
}

async fn create_audit_entry(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(payload): Json<CreateAuditRequest>,
) -> Result<(StatusCode, Json<ComplianceAuditTrail>), (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .legal_repo
        .create_audit_entry(&mut **rls.conn(), org_id, user_id, payload.data)
        .await
        .map(|a| (StatusCode::CREATED, Json(a)))
        .map_err(|e| db_error("Failed to create audit entry", e));
    rls.release().await;
    out
}
