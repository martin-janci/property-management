//! Story 67.2: Enhanced Due Diligence endpoints.

use crate::state::AppState;
use api_core::extractors::AuthUser;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use db::models::compliance::{CreateEnhancedDueDiligence, EddStatus};
use db::models::AuditAction;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::shared::*;

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
pub(super) async fn initiate_edd(
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
pub(super) async fn get_edd_record(
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
pub(super) async fn upload_edd_document(
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
pub(super) async fn verify_edd_document(
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
pub(super) async fn add_edd_note(
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
pub(super) async fn complete_edd(
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
pub(super) async fn list_pending_edd(
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
