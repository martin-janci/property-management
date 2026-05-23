//! E-Signature API routes (Story 7B.3).
//!
//! Provides endpoints for managing electronic signature workflows on documents.

use std::sync::LazyLock;

use api_core::{AuthUser, RlsConnection};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{
    CancelSignatureRequestRequest, CancelSignatureRequestResponse, CreateDocument,
    CreateSignatureRequest, CreateSignatureRequestResponse, ListSignatureRequestsResponse, Locale,
    SendReminderRequest, SendReminderResponse, SignatureRequestResponse, SignatureWebhookEvent,
    WebhookResponse,
};
use integrations::generate_storage_key;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::state::AppState;

/// Default base URL for signature links (used in emails).
const DEFAULT_BASE_URL: &str = "http://localhost:3000";

/// Base URL for signature links, read from environment once.
static BASE_URL: LazyLock<String> =
    LazyLock::new(|| std::env::var("BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string()));

/// Create router for signature endpoints.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(list_signature_requests).post(create_signature_request),
        )
        .route("/{id}", get(get_signature_request))
        .route("/{id}/remind", post(send_reminder))
        .route("/{id}/cancel", post(cancel_signature_request))
        .route("/webhook/{provider}", post(handle_webhook))
}

/// Create a new signature request for a document.
pub async fn create_signature_request(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(document_id): Path<Uuid>,
    Json(request): Json<CreateSignatureRequest>,
) -> Result<(StatusCode, Json<CreateSignatureRequestResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Validate request
    if request.signers.is_empty() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "VALIDATION_ERROR",
                "At least one signer is required",
            )),
        ));
    }

    // Get the document to verify it exists and get organization_id
    let document = match state
        .document_repo
        .find_by_id_rls(&mut **rls.conn(), document_id)
        .await
    {
        Ok(Some(doc)) => doc,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
            ));
        }
        Err(e) => {
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            ));
        }
    };

    // Check if document already has pending signature request
    let existing = match state
        .signature_request_repo
        .find_by_document(document_id)
        .await
    {
        Ok(reqs) => reqs,
        Err(e) => {
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            ));
        }
    };

    if existing.iter().any(|r| {
        matches!(
            r.status,
            db::models::SignatureRequestStatus::Pending
                | db::models::SignatureRequestStatus::InProgress
        )
    }) {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "PENDING_REQUEST_EXISTS",
                "Document already has a pending signature request",
            )),
        ));
    }

    let created_by = auth.user_id;

    let signature_request = match state
        .signature_request_repo
        .create(document_id, document.organization_id, created_by, &request)
        .await
    {
        Ok(req) => req,
        Err(e) => {
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            ));
        }
    };

    rls.release().await;

    info!(
        signature_request_id = %signature_request.id,
        document_id = %document_id,
        signers_count = request.signers.len(),
        "Created signature request"
    );

    // Send invitation emails to signers
    let subject = signature_request
        .subject
        .clone()
        .unwrap_or_else(|| "You have been requested to sign a document".to_string());

    for signer in &signature_request.signers {
        let sign_url = format!(
            "{}/sign?request_id={}&email={}",
            *BASE_URL, signature_request.id, signer.email
        );
        let email_body = format!(
            "Hello {},\n\nYou have been requested to electronically sign a document.\n\n{}\n\nPlease click the link below to review and sign the document:\n\n{}\n\nIf you have any questions, please contact the person who sent this request.\n\nBest regards,\nProperty Management System",
            signer.name,
            signature_request.message.as_deref().unwrap_or(""),
            sign_url
        );

        if let Err(e) = state
            .email_service
            .send_notification_email(
                &signer.email,
                &signer.name,
                &subject,
                &email_body,
                &Locale::English,
            )
            .await
        {
            warn!(
                error = %e,
                email = %signer.email,
                signature_request_id = %signature_request.id,
                "Failed to send signature request email to signer"
            );
            // Continue sending to other signers even if one fails
        }
    }

    Ok((
        StatusCode::CREATED,
        Json(CreateSignatureRequestResponse {
            signature_request,
            message: "Signature request created. Signers will receive email invitations.".into(),
        }),
    ))
}

/// List signature requests for a document.
pub async fn list_signature_requests(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(document_id): Path<Uuid>,
) -> Result<Json<ListSignatureRequestsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify document exists
    let _document = match state
        .document_repo
        .find_by_id_rls(&mut **rls.conn(), document_id)
        .await
    {
        Ok(Some(doc)) => doc,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
            ));
        }
        Err(e) => {
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            ));
        }
    };

    let requests = match state
        .signature_request_repo
        .find_by_document(document_id)
        .await
    {
        Ok(reqs) => reqs,
        Err(e) => {
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            ));
        }
    };

    let total = requests.len() as i64;

    rls.release().await;
    Ok(Json(ListSignatureRequestsResponse {
        signature_requests: requests,
        total,
    }))
}

/// Get a signature request by ID.
pub async fn get_signature_request(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SignatureRequestResponse>, (StatusCode, Json<ErrorResponse>)> {
    let request = state
        .signature_request_repo
        .find_by_id(id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Signature request not found",
                )),
            )
        })?;

    let signer_counts = request.signer_counts();

    Ok(Json(SignatureRequestResponse {
        signature_request: request,
        signer_counts,
    }))
}

/// Send reminder to pending signers.
pub async fn send_reminder(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<SendReminderRequest>,
) -> Result<Json<SendReminderResponse>, (StatusCode, Json<ErrorResponse>)> {
    let signature_request = state
        .signature_request_repo
        .find_by_id(id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Signature request not found",
                )),
            )
        })?;

    // Check if request is still active
    if !signature_request.can_cancel() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_STATE",
                "Cannot send reminders for completed or cancelled requests",
            )),
        ));
    }

    // Find pending signers
    let pending_signers: Vec<_> = signature_request
        .signers
        .iter()
        .filter(|s| !s.is_complete())
        .filter(|s| {
            request.signer_emails.is_empty()
                || request
                    .signer_emails
                    .iter()
                    .any(|e| e.eq_ignore_ascii_case(&s.email))
        })
        .collect();

    if pending_signers.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "NO_PENDING_SIGNERS",
                "No pending signers to remind",
            )),
        ));
    }

    // Send reminder emails to pending signers
    let subject = format!(
        "Reminder: {}",
        signature_request
            .subject
            .as_deref()
            .unwrap_or("Signature request pending")
    );
    let mut reminders_sent = 0i32;

    for signer in pending_signers {
        let sign_url = format!(
            "{}/sign?request_id={}&email={}",
            *BASE_URL, signature_request.id, signer.email
        );
        let email_body = format!(
            "Hello {},\n\nThis is a reminder that you have a pending signature request.\n\nPlease click the link below to review and sign the document:\n\n{}\n\nIf you have any questions, please contact the person who sent this request.\n\nBest regards,\nProperty Management System",
            signer.name, sign_url
        );

        if let Err(e) = state
            .email_service
            .send_notification_email(
                &signer.email,
                &signer.name,
                &subject,
                &email_body,
                &Locale::English,
            )
            .await
        {
            warn!(
                error = %e,
                email = %signer.email,
                signature_request_id = %id,
                "Failed to send reminder email to signer"
            );
            // Continue sending to other signers even if one fails
        } else {
            reminders_sent += 1;
        }
    }

    info!(
        signature_request_id = %id,
        reminders_sent = reminders_sent,
        "Sent signature reminders"
    );

    Ok(Json(SendReminderResponse {
        reminders_sent,
        message: format!("Sent {} reminder(s)", reminders_sent),
    }))
}

/// Cancel a signature request.
pub async fn cancel_signature_request(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<CancelSignatureRequestRequest>,
) -> Result<Json<CancelSignatureRequestResponse>, (StatusCode, Json<ErrorResponse>)> {
    let signature_request = state
        .signature_request_repo
        .cancel(id, request.reason.as_deref())
        .await
        .map_err(|e| {
            let err_msg = e.to_string();
            if err_msg.contains("RowNotFound") {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new(
                        "INVALID_STATE",
                        "Cannot cancel: request not found or already completed/cancelled",
                    )),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("DATABASE_ERROR", &err_msg)),
                )
            }
        })?;

    info!(
        signature_request_id = %id,
        reason = ?request.reason,
        "Cancelled signature request"
    );

    // Notify signers of cancellation
    let subject = format!(
        "Signature Request Cancelled: {}",
        signature_request
            .subject
            .as_deref()
            .unwrap_or("Document signing")
    );
    let reason_text = request
        .reason
        .as_deref()
        .map(|r| format!("\n\nReason: {}", r))
        .unwrap_or_default();

    for signer in &signature_request.signers {
        // Only notify signers who haven't completed signing yet
        if !signer.is_complete() {
            let email_body = format!(
                "Hello {},\n\nThe signature request you received has been cancelled.{}\n\nNo further action is required.\n\nBest regards,\nProperty Management System",
                signer.name, reason_text
            );

            if let Err(e) = state
                .email_service
                .send_notification_email(
                    &signer.email,
                    &signer.name,
                    &subject,
                    &email_body,
                    &Locale::English,
                )
                .await
            {
                warn!(
                    error = %e,
                    email = %signer.email,
                    signature_request_id = %id,
                    "Failed to send cancellation notification to signer"
                );
            }
        }
    }

    Ok(Json(CancelSignatureRequestResponse {
        signature_request,
        message: "Signature request cancelled".into(),
    }))
}

/// Handle webhook from e-signature provider.
pub async fn handle_webhook(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    Json(event): Json<SignatureWebhookEvent>,
) -> Result<Json<WebhookResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        provider = %provider,
        event_type = %event.event_type,
        provider_request_id = %event.provider_request_id,
        "Received signature webhook"
    );

    // Find the signature request by provider request ID
    let signature_request = state
        .signature_request_repo
        .find_by_provider_request_id(&provider, &event.provider_request_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            warn!(
                provider = %provider,
                provider_request_id = %event.provider_request_id,
                "Signature request not found for webhook"
            );
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Signature request not found",
                )),
            )
        })?;

    // Process signer-specific events
    if let (Some(signer_email), Some(signer_status)) = (&event.signer_email, &event.signer_status) {
        state
            .signature_request_repo
            .update_signer_status(
                signature_request.id,
                signer_email,
                *signer_status,
                event.decline_reason.as_deref(),
            )
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
                )
            })?;

        info!(
            signature_request_id = %signature_request.id,
            signer_email = %signer_email,
            new_status = ?signer_status,
            "Updated signer status from webhook"
        );
    }

    // Handle completion event with signed document (Story 88.2)
    if event.event_type == "completed" {
        if let Some(signed_url) = &event.signed_document_url {
            match store_signed_document(&state, &signature_request, signed_url).await {
                Ok(signed_doc_id) => {
                    info!(
                        signature_request_id = %signature_request.id,
                        signed_document_id = %signed_doc_id,
                        "Signed document stored successfully"
                    );
                }
                Err(e) => {
                    // Log error but don't fail the webhook - document can be retrieved later
                    error!(
                        signature_request_id = %signature_request.id,
                        error = %e,
                        "Failed to store signed document"
                    );
                }
            }
        }
    }

    Ok(Json(WebhookResponse {
        success: true,
        message: "Webhook processed successfully".into(),
    }))
}

/// Download and store a signed document (Story 88.2).
///
/// This function:
/// 1. Downloads the signed document from the provider's URL
/// 2. Creates a new document record with `signed_` prefix
/// 3. Links the signed document to the original document
/// 4. Updates the signature request with the signed document reference
async fn store_signed_document(
    state: &AppState,
    signature_request: &db::models::SignatureRequest,
    signed_url: &str,
) -> Result<Uuid, String> {
    // Acquire a connection and set RLS context for webhook processing
    // Webhooks don't have user auth, so we use the signature request's org/user context
    let mut conn = state
        .db
        .acquire()
        .await
        .map_err(|e| format!("Failed to acquire database connection: {}", e))?;

    db::tenant_context::set_request_context(
        &mut *conn,
        Some(signature_request.organization_id),
        Some(signature_request.created_by),
        false, // Not a super admin context
    )
    .await
    .map_err(|e| format!("Failed to set RLS context: {}", e))?;

    // Get the original document using RLS-aware method
    let original_doc = state
        .document_repo
        .find_by_id_rls(&mut *conn, signature_request.document_id)
        .await
        .map_err(|e| format!("Failed to find original document: {}", e))?
        .ok_or("Original document not found")?;

    // P1-05: SSRF + MIME / size hardening. Previously this trusted the
    // provider response wholesale: the Content-Type header was taken
    // verbatim and persisted as the document's mime_type, no size cap,
    // no magic-byte check. A spoofed e-signature provider response (or
    // a signed_url pointing at an attacker-controlled host) could land
    // an HTML payload as a "signed PDF" which would then render
    // inline via the next presigned GET — stored XSS.
    //
    // Constants are local because this is the only call site.
    const MAX_SIGNED_DOC_BYTES: u64 = 50 * 1024 * 1024; // 50 MiB
    const ALLOWED_MIME_TYPES: &[&str] = &["application/pdf"];
    const PDF_MAGIC: &[u8] = b"%PDF-";

    let client = reqwest::Client::new();
    let response = client
        .get(signed_url)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await
        .map_err(|e| format!("Failed to download signed document: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to download signed document: HTTP {}",
            response.status()
        ));
    }

    // Reject unannounced content-types early; if Content-Length is
    // present, also reject oversize before we allocate.
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(';').next().unwrap_or(s).trim().to_lowercase())
        .unwrap_or_else(|| "application/pdf".to_string());

    if !ALLOWED_MIME_TYPES.contains(&content_type.as_str()) {
        return Err(format!(
            "Signed-document MIME type not allowed: {} (allowed: {})",
            content_type,
            ALLOWED_MIME_TYPES.join(", ")
        ));
    }
    if let Some(len) = response.content_length() {
        if len > MAX_SIGNED_DOC_BYTES {
            return Err(format!(
                "Signed document too large: {} bytes (max {} bytes)",
                len, MAX_SIGNED_DOC_BYTES
            ));
        }
    }

    let content_bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read signed document content: {}", e))?;

    // Re-check size after body read in case Content-Length was absent
    // or lied.
    if content_bytes.len() as u64 > MAX_SIGNED_DOC_BYTES {
        return Err(format!(
            "Signed document too large: {} bytes (max {} bytes)",
            content_bytes.len(),
            MAX_SIGNED_DOC_BYTES
        ));
    }

    // Magic-byte check: every legit signed PDF starts with `%PDF-`.
    if !content_bytes.starts_with(PDF_MAGIC) {
        return Err(
            "Signed document failed PDF magic-byte check (header is not %PDF-)".to_string(),
        );
    }

    let size_bytes = content_bytes.len() as i64;

    // Create signed filename with `signed_` prefix
    let signed_filename = format!("signed_{}", original_doc.file_name);

    // Generate storage key for the signed document
    let file_key = generate_storage_key(original_doc.organization_id, &signed_filename);

    // Story 103.1: Upload signed document to S3 if storage service is available
    if let Some(ref storage_service) = state.storage_service {
        if storage_service.has_s3_client() {
            storage_service
                .upload(&file_key, content_bytes.to_vec(), &content_type)
                .await
                .map_err(|e| format!("Failed to upload signed document to S3: {}", e))?;

            info!(
                file_key = %file_key,
                size_bytes = size_bytes,
                "Uploaded signed document to S3"
            );
        }
    }

    // Create the signed document record
    let create_doc = CreateDocument {
        organization_id: original_doc.organization_id,
        folder_id: original_doc.folder_id,
        title: format!("Signed: {}", original_doc.title),
        description: Some(format!(
            "Electronically signed version of '{}'. Signed via signature request {}.",
            original_doc.title, signature_request.id
        )),
        category: original_doc.category.clone(),
        file_key,
        file_name: signed_filename,
        mime_type: content_type,
        size_bytes,
        access_scope: Some(original_doc.access_scope.clone()),
        access_target_ids: serde_json::from_value(original_doc.access_target_ids.clone()).ok(),
        access_roles: serde_json::from_value(original_doc.access_roles.clone()).ok(),
        created_by: signature_request.created_by,
    };

    // Create signed document using RLS-aware method
    let signed_doc = state
        .document_repo
        .create_rls(&mut *conn, create_doc)
        .await
        .map_err(|e| format!("Failed to create signed document record: {}", e))?;

    // Link the signed document to the signature request
    state
        .signature_request_repo
        .set_signed_document(signature_request.id, signed_doc.id)
        .await
        .map_err(|e| format!("Failed to link signed document to request: {}", e))?;

    info!(
        original_document_id = %original_doc.id,
        signed_document_id = %signed_doc.id,
        signed_filename = %signed_doc.file_name,
        size_bytes = size_bytes,
        "Created signed document record linked to original"
    );

    // Clear RLS context before returning connection to pool
    if let Err(e) = db::tenant_context::clear_request_context(&mut *conn).await {
        warn!(error = %e, "Failed to clear RLS context after storing signed document");
    }

    Ok(signed_doc.id)
}

// Helper function to create document-scoped signature routes
pub fn document_signature_router() -> Router<AppState> {
    Router::new().route(
        "/signature-requests",
        get(list_signature_requests_for_doc).post(create_signature_request_for_doc),
    )
}

/// Create a signature request for a specific document (nested route version).
pub async fn create_signature_request_for_doc(
    State(state): State<AppState>,
    auth: AuthUser,
    rls: RlsConnection,
    Path(document_id): Path<Uuid>,
    Json(request): Json<CreateSignatureRequest>,
) -> Result<(StatusCode, Json<CreateSignatureRequestResponse>), (StatusCode, Json<ErrorResponse>)> {
    create_signature_request(State(state), auth, rls, Path(document_id), Json(request)).await
}

/// List signature requests for a specific document (nested route version).
pub async fn list_signature_requests_for_doc(
    State(state): State<AppState>,
    rls: RlsConnection,
    Path(document_id): Path<Uuid>,
) -> Result<Json<ListSignatureRequestsResponse>, (StatusCode, Json<ErrorResponse>)> {
    list_signature_requests(State(state), rls, Path(document_id)).await
}
