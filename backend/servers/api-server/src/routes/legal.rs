//! Legal document and compliance routes (Epic 25).

use api_core::extractors::AuthUser;
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

// ==================== Request/Response Types ====================

/// Organization query parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct OrgQuery {
    pub organization_id: Uuid,
}

/// Create legal document request wrapper.
#[derive(Debug, Deserialize)]
pub struct CreateDocumentRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateLegalDocument,
}

/// List legal documents query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListDocumentsQuery {
    pub organization_id: Uuid,
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
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateComplianceRequirement,
}

/// List compliance requirements query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListRequirementsQuery {
    pub organization_id: Uuid,
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
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateLegalNotice,
}

/// List legal notices query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListNoticesQuery {
    pub organization_id: Uuid,
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
    pub organization_id: Uuid,
    pub category: Option<String>,
}

/// Apply template request.
#[derive(Debug, Deserialize)]
pub struct ApplyTemplateRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: ApplyTemplate,
}

/// Create audit entry request wrapper.
#[derive(Debug, Deserialize)]
pub struct CreateAuditRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateAuditTrailEntry,
}

/// Audit trail query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct AuditTrailQuery {
    pub organization_id: Uuid,
    pub requirement_id: Option<Uuid>,
    pub document_id: Option<Uuid>,
    pub notice_id: Option<Uuid>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

/// Pagination query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct PaginationQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

// ==================== Legal Document Endpoints ====================

async fn create_document(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<CreateDocumentRequest>,
) -> Result<(StatusCode, Json<LegalDocument>), (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .create_document(payload.organization_id, user.user_id, payload.data)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(|e| {
            tracing::error!("Failed to create legal document: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

async fn list_documents(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<ListDocumentsQuery>,
) -> Result<Json<Vec<LegalDocument>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .list_documents(query.organization_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list legal documents: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

async fn list_documents_summary(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<ListDocumentsQuery>,
) -> Result<Json<Vec<LegalDocumentSummary>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .list_documents_with_summary(query.organization_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list legal documents: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

async fn get_document(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<LegalDocument>, (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .find_document_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get legal document: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?
        .map(Json)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
            )
        })
}

async fn update_document(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateLegalDocument>,
) -> Result<Json<LegalDocument>, (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .update_document(id, data)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to update legal document: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

async fn delete_document(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state.legal_repo.delete_document(id).await.map_err(|e| {
        tracing::error!("Failed to delete legal document: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DB_ERROR", e.to_string())),
        )
    })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
        ))
    }
}

// ==================== Document Versions ====================

async fn add_version(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<CreateLegalDocumentVersion>,
) -> Result<(StatusCode, Json<LegalDocumentVersion>), (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .add_document_version(id, user.user_id, data)
        .await
        .map(|v| (StatusCode::CREATED, Json(v)))
        .map_err(|e| {
            tracing::error!("Failed to add document version: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

async fn list_versions(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<LegalDocumentVersion>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .list_document_versions(id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list document versions: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

async fn get_version(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((id, version)): Path<(Uuid, i32)>,
) -> Result<Json<LegalDocumentVersion>, (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .get_document_version(id, version)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get document version: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?
        .map(Json)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Version not found")),
            )
        })
}

// ==================== Compliance Requirements ====================

async fn create_requirement(
    State(state): State<AppState>,
    _user: AuthUser,
    Json(payload): Json<CreateRequirementRequest>,
) -> Result<(StatusCode, Json<ComplianceRequirement>), (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .create_requirement(payload.organization_id, payload.data)
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(|e| {
            tracing::error!("Failed to create compliance requirement: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

async fn list_requirements(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<ListRequirementsQuery>,
) -> Result<Json<Vec<ComplianceRequirement>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .list_requirements(query.organization_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list compliance requirements: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

async fn list_requirements_with_details(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<ListRequirementsQuery>,
) -> Result<Json<Vec<ComplianceRequirementWithDetails>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .list_requirements_with_details(query.organization_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list compliance requirements: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

async fn get_compliance_statistics(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<OrgQuery>,
) -> Result<Json<ComplianceStatistics>, (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .get_compliance_statistics(query.organization_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get compliance statistics: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

async fn get_requirement(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ComplianceRequirement>, (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .find_requirement_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get compliance requirement: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?
        .map(Json)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Requirement not found")),
            )
        })
}

async fn update_requirement(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateComplianceRequirement>,
) -> Result<Json<ComplianceRequirement>, (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .update_requirement(id, data)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to update compliance requirement: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

async fn delete_requirement(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state.legal_repo.delete_requirement(id).await.map_err(|e| {
        tracing::error!("Failed to delete compliance requirement: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DB_ERROR", e.to_string())),
        )
    })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Requirement not found")),
        ))
    }
}

// ==================== Compliance Verifications ====================

async fn create_verification(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<CreateComplianceVerification>,
) -> Result<(StatusCode, Json<ComplianceVerification>), (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .create_verification(id, user.user_id, data)
        .await
        .map(|v| (StatusCode::CREATED, Json(v)))
        .map_err(|e| {
            tracing::error!("Failed to create compliance verification: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

async fn list_verifications(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ComplianceVerification>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .list_verifications(id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list compliance verifications: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

// ==================== Legal Notices ====================

async fn create_notice(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<CreateNoticeRequest>,
) -> Result<(StatusCode, Json<LegalNotice>), (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .create_notice(payload.organization_id, user.user_id, payload.data)
        .await
        .map(|n| (StatusCode::CREATED, Json(n)))
        .map_err(|e| {
            tracing::error!("Failed to create legal notice: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

async fn list_notices(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<ListNoticesQuery>,
) -> Result<Json<Vec<LegalNotice>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .list_notices(query.organization_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list legal notices: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

async fn list_notices_with_recipients(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<ListNoticesQuery>,
) -> Result<Json<Vec<NoticeWithRecipients>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .list_notices_with_recipients(query.organization_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list legal notices: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

async fn get_notice_statistics(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<OrgQuery>,
) -> Result<Json<NoticeStatistics>, (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .get_notice_statistics(query.organization_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get notice statistics: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

async fn get_notice(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<LegalNotice>, (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .find_notice_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get legal notice: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?
        .map(Json)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Notice not found")),
            )
        })
}

async fn update_notice(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateLegalNotice>,
) -> Result<Json<LegalNotice>, (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .update_notice(id, data)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to update legal notice: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

async fn delete_notice(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state.legal_repo.delete_notice(id).await.map_err(|e| {
        tracing::error!("Failed to delete legal notice: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DB_ERROR", e.to_string())),
        )
    })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Notice not found")),
        ))
    }
}

async fn send_notice(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<LegalNotice>, (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .send_notice(id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to send legal notice: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

// ==================== Notice Recipients ====================

async fn list_recipients(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<LegalNoticeRecipient>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .list_notice_recipients(id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list notice recipients: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

async fn acknowledge_notice(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((notice_id, recipient_id)): Path<(Uuid, Uuid)>,
    Json(data): Json<AcknowledgeNotice>,
) -> Result<Json<LegalNoticeRecipient>, (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .acknowledge_notice(notice_id, recipient_id, data)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to acknowledge notice: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

// ==================== Compliance Templates ====================

async fn create_template(
    State(state): State<AppState>,
    _user: AuthUser,
    Json(payload): Json<CreateTemplateRequest>,
) -> Result<(StatusCode, Json<ComplianceTemplate>), (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .create_template(payload.organization_id, payload.data)
        .await
        .map(|t| (StatusCode::CREATED, Json(t)))
        .map_err(|e| {
            tracing::error!("Failed to create compliance template: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

async fn list_templates(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<ListTemplatesQuery>,
) -> Result<Json<Vec<ComplianceTemplate>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .list_templates(query.organization_id, query.category)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list compliance templates: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

async fn get_template(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ComplianceTemplate>, (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .find_template_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get compliance template: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?
        .map(Json)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Template not found")),
            )
        })
}

async fn update_template(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateComplianceTemplate>,
) -> Result<Json<ComplianceTemplate>, (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .update_template(id, data)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to update compliance template: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

async fn delete_template(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state.legal_repo.delete_template(id).await.map_err(|e| {
        tracing::error!("Failed to delete compliance template: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DB_ERROR", e.to_string())),
        )
    })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Template not found (or is system template)",
            )),
        ))
    }
}

async fn apply_template(
    State(state): State<AppState>,
    _user: AuthUser,
    Json(payload): Json<ApplyTemplateRequest>,
) -> Result<(StatusCode, Json<Vec<ComplianceRequirement>>), (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .apply_template(payload.organization_id, payload.data)
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(|e| {
            tracing::error!("Failed to apply compliance template: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

// ==================== Audit Trail ====================

async fn list_audit_trail(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<AuditTrailQuery>,
) -> Result<Json<Vec<ComplianceAuditTrail>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .list_audit_trail(
            query.organization_id,
            query.requirement_id,
            query.document_id,
            query.notice_id,
            query.limit.unwrap_or(50),
            query.offset.unwrap_or(0),
        )
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list audit trail: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}

async fn create_audit_entry(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<CreateAuditRequest>,
) -> Result<(StatusCode, Json<ComplianceAuditTrail>), (StatusCode, Json<ErrorResponse>)> {
    state
        .legal_repo
        .create_audit_entry(payload.organization_id, user.user_id, payload.data)
        .await
        .map(|a| (StatusCode::CREATED, Json(a)))
        .map_err(|e| {
            tracing::error!("Failed to create audit entry: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })
}
