//! Lease Abstraction routes (Epic 133).
//! AI Lease Abstraction & Document Intelligence for automated extraction of key terms.

use crate::state::AppState;
use api_core::extractors::AuthUser;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::lease_abstraction::{
    ApproveExtraction, CreateExtractionCorrection, CreateLeaseDocument, CreateLeaseExtraction,
    ImportExtractionRequest, LeaseDocumentQuery, ProcessDocumentRequest, RejectExtraction,
};
use serde_json::json;
use tracing::{info, warn};
use uuid::Uuid;

type ApiResult<T> = Result<T, (StatusCode, Json<ErrorResponse>)>;

pub fn router() -> Router<AppState> {
    Router::new()
        // Lease Documents (Story 133.1)
        .route("/documents", get(list_documents))
        .route("/documents", post(upload_document))
        .route("/documents/{id}", get(get_document))
        .route("/documents/{id}", delete(delete_document))
        .route("/documents/{id}/process", post(process_document))
        // Lease Extractions (Story 133.2)
        .route("/documents/{id}/extractions", get(list_extractions))
        .route("/extractions/{id}", get(get_extraction))
        .route("/extractions/{id}/fields", get(get_extraction_fields))
        // Human Review (Story 133.3)
        .route("/extractions/{id}/approve", post(approve_extraction))
        .route("/extractions/{id}/reject", post(reject_extraction))
        .route("/extractions/{id}/corrections", get(list_corrections))
        .route("/extractions/{id}/corrections", post(add_correction))
        // Lease Import (Story 133.4)
        .route("/extractions/{id}/validate", post(validate_import))
        .route("/extractions/{id}/import", post(import_to_lease))
        .route("/imports", get(list_imports))
        .route("/imports/{id}", get(get_import))
}

// =============================================================================
// LEASE DOCUMENTS (Story 133.1)
// =============================================================================

/// List lease documents for the organization.
async fn list_documents(
    State(s): State<AppState>,
    user: AuthUser,
    Query(query): Query<LeaseDocumentQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s.lease_abstraction_repo.list_documents(org_id, query).await {
        Ok(documents) => Ok(Json(json!({ "documents": documents }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

/// Upload a lease document for processing.
#[allow(deprecated)]
async fn upload_document(
    State(s): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateLeaseDocument>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;
    let user_id = user.user_id;

    // Validate mime type
    let allowed_types = ["application/pdf", "image/png", "image/jpeg", "image/tiff"];
    if !allowed_types.contains(&req.mime_type.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&format!(
                "Unsupported file type: {}. Supported types: PDF, PNG, JPEG, TIFF",
                req.mime_type
            ))),
        ));
    }

    // Validate unit belongs to organization if provided
    if let Some(unit_id) = req.unit_id {
        let unit = s.unit_repo.find_by_id(unit_id).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?;

        if let Some(unit) = unit {
            let building = s
                .building_repo
                .find_by_id(unit.building_id)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::internal_error(&e.to_string())),
                    )
                })?;

            if let Some(building) = building {
                if building.organization_id != org_id {
                    return Err((
                        StatusCode::FORBIDDEN,
                        Json(ErrorResponse::forbidden(
                            "Unit does not belong to your organization",
                        )),
                    ));
                }
            }
        } else {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::not_found("Unit not found")),
            ));
        }
    }

    info!(
        "Uploading lease document: {} ({} bytes)",
        req.file_name, req.file_size_bytes
    );

    match s
        .lease_abstraction_repo
        .create_document(org_id, user_id, req)
        .await
    {
        Ok(doc) => Ok(Json(serde_json::to_value(doc).unwrap())),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

/// Get a lease document by ID.
async fn get_document(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s.lease_abstraction_repo.get_document(id, org_id).await {
        Ok(Some(doc)) => Ok(Json(serde_json::to_value(doc).unwrap())),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Document not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

/// Delete a lease document.
async fn delete_document(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s.lease_abstraction_repo.delete_document(id, org_id).await {
        Ok(true) => Ok(Json(json!({ "deleted": true }))),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Document not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

/// Process a document with AI extraction.
async fn process_document(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ProcessDocumentRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    // Verify document belongs to organization
    let doc = s
        .lease_abstraction_repo
        .get_document(id, org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::not_found("Document not found")),
            )
        })?;

    // Check if already processed (unless force)
    if doc.status == "completed" && !req.force {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse::bad_request(
                "Document already processed. Use force=true to reprocess.",
            )),
        ));
    }

    // Update document status to processing
    s.lease_abstraction_repo
        .update_document_status(id, "processing", None)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?;

    info!("Processing lease document {} with AI extraction", id);

    // Perform AI extraction (this would use LLM in production)
    let extraction_result = perform_ai_extraction(&s, &doc).await;

    match extraction_result {
        Ok(extraction_data) => {
            // Create extraction record
            let extraction = s
                .lease_abstraction_repo
                .create_extraction(extraction_data)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::internal_error(&e.to_string())),
                    )
                })?;

            // Update document status based on extraction confidence
            let overall_confidence = extraction.overall_confidence.unwrap_or_default();
            let new_status = if overall_confidence >= rust_decimal::Decimal::new(80, 0) {
                "completed"
            } else {
                "review_required"
            };

            s.lease_abstraction_repo
                .update_document_status(id, new_status, None)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::internal_error(&e.to_string())),
                    )
                })?;

            Ok(Json(json!({
                "status": "processed",
                "extraction_id": extraction.id,
                "overall_confidence": extraction.overall_confidence,
                "fields_extracted": extraction.fields_extracted,
                "fields_flagged": extraction.fields_flagged,
                "document_status": new_status
            })))
        }
        Err(error_msg) => {
            // Update document status to failed
            s.lease_abstraction_repo
                .update_document_status(id, "failed", Some(&error_msg))
                .await
                .ok();

            warn!("AI extraction failed for document {}: {}", id, error_msg);

            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&format!(
                    "AI extraction failed: {}",
                    error_msg
                ))),
            ))
        }
    }
}

/// Perform AI extraction using LLM.
///
/// Story 133.2: This function would integrate with the LLM client to extract
/// lease terms from the document. For now, it returns a placeholder extraction.
async fn perform_ai_extraction(
    _state: &AppState,
    doc: &db::models::lease_abstraction::LeaseDocument,
) -> Result<CreateLeaseExtraction, String> {
    // In production, this would:
    // 1. Retrieve document from storage
    // 2. Convert to text (OCR for images, PDF parsing for PDFs)
    // 3. Send to LLM for extraction
    // 4. Parse LLM response into structured data

    // Placeholder extraction result
    Ok(CreateLeaseExtraction {
        document_id: doc.id,
        tenant_name: None,
        tenant_name_confidence: None,
        tenant_name_location: None,
        landlord_name: None,
        landlord_name_confidence: None,
        landlord_name_location: None,
        property_address: None,
        property_address_confidence: None,
        property_address_location: None,
        lease_start_date: None,
        lease_start_date_confidence: None,
        lease_start_date_location: None,
        lease_end_date: None,
        lease_end_date_confidence: None,
        lease_end_date_location: None,
        monthly_rent: None,
        monthly_rent_confidence: None,
        monthly_rent_location: None,
        rent_currency: "EUR".to_string(),
        security_deposit: None,
        security_deposit_confidence: None,
        security_deposit_location: None,
        payment_due_day: None,
        payment_due_day_confidence: None,
        payment_due_day_location: None,
        special_clauses: serde_json::json!([]),
        model_used: Some("placeholder".to_string()),
        extraction_duration_ms: Some(0),
    })
}

// =============================================================================
// LEASE EXTRACTIONS (Story 133.2)
// =============================================================================

/// List extractions for a document.
async fn list_extractions(
    State(s): State<AppState>,
    user: AuthUser,
    Path(document_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    // Verify document belongs to organization
    let doc = s
        .lease_abstraction_repo
        .get_document(document_id, org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?;

    if doc.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Document not found")),
        ));
    }

    match s.lease_abstraction_repo.list_extractions(document_id).await {
        Ok(extractions) => Ok(Json(json!({ "extractions": extractions }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

/// Get extraction by ID.
async fn get_extraction(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .lease_abstraction_repo
        .get_extraction_for_org(id, org_id)
        .await
    {
        Ok(Some(extraction)) => Ok(Json(serde_json::to_value(extraction).unwrap())),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Extraction not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

/// Get extraction with all fields expanded for UI.
async fn get_extraction_fields(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .lease_abstraction_repo
        .get_extraction_with_fields(id, org_id)
        .await
    {
        Ok(Some(extraction_with_fields)) => {
            Ok(Json(serde_json::to_value(extraction_with_fields).unwrap()))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Extraction not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

// =============================================================================
// HUMAN REVIEW (Story 133.3)
// =============================================================================

/// Approve an extraction with optional corrections.
async fn approve_extraction(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ApproveExtraction>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;
    let user_id = user.user_id;

    // Verify extraction belongs to organization
    let extraction = s
        .lease_abstraction_repo
        .get_extraction_for_org(id, org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::not_found("Extraction not found")),
            )
        })?;

    // Check if already reviewed
    if extraction.review_status != "pending" {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse::bad_request("Extraction already reviewed")),
        ));
    }

    // Add corrections if any
    for correction in &req.corrections {
        s.lease_abstraction_repo
            .add_correction(id, user_id, correction.clone())
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::internal_error(&e.to_string())),
                )
            })?;
    }

    // Approve extraction
    match s
        .lease_abstraction_repo
        .approve_extraction_returning(id, user_id)
        .await
    {
        Ok(Some(extraction)) => {
            info!("Extraction {} approved by user {}", id, user_id);
            Ok(Json(serde_json::to_value(extraction).unwrap()))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Extraction not found")),
        )),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

/// Reject an extraction with reason.
async fn reject_extraction(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<RejectExtraction>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;
    let user_id = user.user_id;

    // Verify extraction belongs to organization
    let extraction = s
        .lease_abstraction_repo
        .get_extraction_for_org(id, org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::not_found("Extraction not found")),
            )
        })?;

    // Check if already reviewed
    if extraction.review_status != "pending" {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse::bad_request("Extraction already reviewed")),
        ));
    }

    match s
        .lease_abstraction_repo
        .reject_extraction_returning(id, user_id, &req.reason)
        .await
    {
        Ok(Some(extraction)) => {
            info!(
                "Extraction {} rejected by user {}: {}",
                id, user_id, req.reason
            );
            Ok(Json(serde_json::to_value(extraction).unwrap()))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Extraction not found")),
        )),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

/// List corrections for an extraction.
async fn list_corrections(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    // Verify extraction belongs to organization
    let extraction = s
        .lease_abstraction_repo
        .get_extraction_for_org(id, org_id)
        .await;
    if matches!(extraction, Ok(None) | Err(_)) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Extraction not found")),
        ));
    }

    match s.lease_abstraction_repo.list_corrections(id).await {
        Ok(corrections) => Ok(Json(json!({ "corrections": corrections }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

/// Add a correction to an extraction.
async fn add_correction(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateExtractionCorrection>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;
    let user_id = user.user_id;

    // Verify extraction belongs to organization
    let extraction = s
        .lease_abstraction_repo
        .get_extraction_for_org(id, org_id)
        .await;
    if matches!(extraction, Ok(None) | Err(_)) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Extraction not found")),
        ));
    }

    match s
        .lease_abstraction_repo
        .add_correction(id, user_id, req)
        .await
    {
        Ok(correction) => Ok(Json(serde_json::to_value(correction).unwrap())),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

// =============================================================================
// LEASE IMPORT (Story 133.4)
// =============================================================================

/// Validate extraction before import.
#[allow(deprecated)]
async fn validate_import(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ImportExtractionRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    // Verify extraction belongs to organization
    let extraction = s
        .lease_abstraction_repo
        .get_extraction_for_org(id, org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::not_found("Extraction not found")),
            )
        })?;

    // Verify unit belongs to organization
    let unit = s.unit_repo.find_by_id(req.unit_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )
    })?;

    let unit = unit.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Unit not found")),
        )
    })?;

    let building = s
        .building_repo
        .find_by_id(unit.building_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?;

    if building.is_none() || building.as_ref().unwrap().organization_id != org_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::forbidden(
                "Unit does not belong to your organization",
            )),
        ));
    }

    // Validate extraction data
    match s
        .lease_abstraction_repo
        .validate_import(id, &extraction)
        .await
    {
        Ok(result) => Ok(Json(serde_json::to_value(result).unwrap())),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

/// Import extraction to create a lease.
#[allow(deprecated)]
async fn import_to_lease(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ImportExtractionRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;
    let user_id = user.user_id;

    // Verify extraction belongs to organization
    let extraction = s
        .lease_abstraction_repo
        .get_extraction_for_org(id, org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::not_found("Extraction not found")),
            )
        })?;

    // Check if extraction is approved
    if extraction.review_status != "approved" {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse::bad_request(
                "Extraction must be approved before import",
            )),
        ));
    }

    // Verify unit belongs to organization
    let unit = s.unit_repo.find_by_id(req.unit_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )
    })?;

    let unit = unit.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Unit not found")),
        )
    })?;

    let building = s
        .building_repo
        .find_by_id(unit.building_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?;

    if building.is_none() || building.as_ref().unwrap().organization_id != org_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::forbidden(
                "Unit does not belong to your organization",
            )),
        ));
    }

    // Import to lease
    match s
        .lease_abstraction_repo
        .import_to_lease(id, req.unit_id, user_id, req.overrides)
        .await
    {
        Ok(result) => {
            info!(
                "Extraction {} imported to lease {:?} by user {}",
                id, result.lease_id, user_id
            );
            Ok(Json(serde_json::to_value(result).unwrap()))
        }
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

/// List lease imports.
async fn list_imports(
    State(s): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s.lease_abstraction_repo.list_imports(org_id).await {
        Ok(imports) => Ok(Json(json!({ "imports": imports }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

/// Get import by ID.
async fn get_import(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s.lease_abstraction_repo.get_import(id, org_id).await {
        Ok(Some(import)) => Ok(Json(serde_json::to_value(import).unwrap())),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Import not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}
