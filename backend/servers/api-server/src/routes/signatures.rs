//! E-Signature API routes (Story 7B.3 / Epic 84.2).
//!
//! Provides endpoints for managing electronic signature workflows on documents.
//! Integrates with the `LightweightProvider` (self-hosted HMAC signing links) with
//! optional DocuSign support via the `integrations::esignature` module.

use std::sync::LazyLock;

use api_core::{AuthUser, RlsConnection};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{
    CancelSignatureRequestRequest, CancelSignatureRequestResponse, CreateDocument,
    CreateSignatureRequest, CreateSignatureRequestResponse, ListSignatureRequestsResponse, Locale,
    SendReminderRequest, SendReminderResponse, SignatureRequestResponse, SignatureRequestStatus,
    SignatureWebhookEvent, SignerStatus, WebhookResponse,
};
use db::repositories::{ConsumeOutcome, NonceState};
use integrations::{generate_storage_key, ESignatureError, LightweightProvider};
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use tracing::{error, info, warn};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::state::AppState;

/// Default base URL for signature links (used in emails).
const DEFAULT_BASE_URL: &str = "http://localhost:3000";

/// Base URL for signature links, read from environment once.
static BASE_URL: LazyLock<String> =
    LazyLock::new(|| std::env::var("BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string()));

/// Lightweight e-signature provider, initialised once from env.
///
/// The `.expect` is safe because `api-server::main` runs the same
/// `LightweightProvider::from_env` check at startup and refuses to boot if it
/// fails — this static is only touched on the request path, after that gate.
static ESIGN_PROVIDER: LazyLock<LightweightProvider> = LazyLock::new(|| {
    LightweightProvider::from_env()
        .expect("ESIGN_TOKEN_SECRET must be configured (validated at startup)")
});

/// Create router for signature endpoints.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{id}", get(get_signature_request))
        .route("/{id}/remind", post(send_reminder))
        .route("/{id}/cancel", post(cancel_signature_request))
        .route("/webhook/{provider}", post(handle_webhook))
}

/// Document-nested signature-request routes.
///
/// Mounted by `routes::documents` under `/api/v1/documents`, yielding
/// `/api/v1/documents/{id}/signature-requests`. The `list`/`create` handlers
/// extract `Path(document_id)`, so they must live under a route that actually
/// carries that path segment — the previous bare `/` mount under
/// `/api/v1/signature-requests` left `Path<Uuid>` extraction with nothing to
/// bind, making both endpoints unreachable (BIT-313). The document param is
/// named `{id}` to match the sibling document sub-routes and keep axum's
/// path-parameter names consistent across the shared prefix.
pub fn document_signature_router() -> Router<AppState> {
    Router::new().route(
        "/{id}/signature-requests",
        get(list_signature_requests).post(create_signature_request),
    )
}

/// Create a new signature request for a document.
pub async fn create_signature_request(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(document_id): Path<Uuid>,
    Json(request): Json<CreateSignatureRequest>,
) -> Result<(StatusCode, Json<CreateSignatureRequestResponse>), (StatusCode, Json<ErrorResponse>)> {
    let result =
        create_signature_request_for_document(&state, &auth, &mut rls, document_id, request).await;
    rls.release().await;
    result.map(|resp| (StatusCode::CREATED, Json(resp)))
}

/// Core signature-request creation flow, shared by the document-scoped HTTP
/// handler and other subsystems (e.g. lease e-signing in `routes::leases`).
///
/// This is the single hardened path that owns the HMAC-signed-link + nonce +
/// email logic (issue #527 / #673). Callers must NOT duplicate that logic — they
/// resolve a `document_id` and delegate here. The caller owns the
/// [`RlsConnection`] lifecycle and is responsible for calling `rls.release()`
/// after this returns (this function does not release it, so the caller can keep
/// using the connection for follow-up work in the same request).
pub(crate) async fn create_signature_request_for_document(
    state: &AppState,
    auth: &AuthUser,
    rls: &mut RlsConnection,
    document_id: Uuid,
    request: CreateSignatureRequest,
) -> Result<CreateSignatureRequestResponse, (StatusCode, Json<ErrorResponse>)> {
    // Validate request
    if request.signers.is_empty() {
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
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
            ));
        }
        Err(e) => {
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
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            ));
        }
    };

    info!(
        signature_request_id = %signature_request.id,
        document_id = %document_id,
        signers_count = request.signers.len(),
        "Created signature request"
    );

    // Send invitation emails to signers using the lightweight provider
    // to generate HMAC-signed signing URLs (Story 84.2).
    let expires_str = signature_request
        .expires_at
        .map(|e| e.format("%Y-%m-%d").to_string());
    let doc_title = document.title.clone();
    let requester_display = if auth.name.is_empty() {
        auth.email.clone()
    } else {
        auth.name.clone()
    };

    let org_id_str = signature_request.organization_id.to_string();
    for signer in &signature_request.signers {
        // Build a HMAC-secured signing URL via the lightweight provider.
        // The token binds signer email, request id, organisation, and the
        // signer's current status; the embedded nonce is persisted to
        // `e_signature_nonces` so the future `/sign` consumer can enforce
        // one-shot use (issue #673 follow-up to PR #542).
        // Issue #527 (c): org_id bound into HMAC prevents cross-tenant replay.
        let signer_status = signer.status.to_string();
        let sign_url = match ESIGN_PROVIDER.build_signing_url(
            &signer.email,
            &signature_request.id.to_string(),
            &org_id_str,
            &signer_status,
        ) {
            Ok(signed) => {
                // Persist the nonce BEFORE handing the URL to the email
                // service. If persistence fails we abandon this signer's
                // link rather than email a link with no DB-side enforcement
                // — the email is best-effort anyway and the signer can be
                // re-reminded later.
                if let Err(e) = state
                    .e_signature_nonce_repo
                    .record_nonce(signature_request.id, signed.nonce)
                    .await
                {
                    warn!(
                        error = %e,
                        email = %signer.email,
                        signature_request_id = %signature_request.id,
                        "Failed to persist e-signature nonce; skipping signer email"
                    );
                    continue;
                }
                signed.url
            }
            Err(e) => {
                // Fail closed: never emit an unsigned `/sign` link. A link
                // without a valid HMAC token defeats the signed-link integrity
                // guarantee, so we abandon this signer's email rather than send
                // an unauthenticated URL. The signer can be re-reminded later.
                warn!(
                    error = %e,
                    email = %signer.email,
                    signature_request_id = %signature_request.id,
                    "Failed to build HMAC-signed signing URL; skipping signer email (refusing to emit unsigned link)"
                );
                continue;
            }
        };

        if let Err(e) = state
            .email_service
            .send_signature_request_email(
                &signer.email,
                &signer.name,
                &doc_title,
                &requester_display,
                &sign_url,
                signature_request.message.as_deref(),
                expires_str.as_deref(),
            )
            .await
        {
            warn!(
                error = %e,
                email = %signer.email,
                signature_request_id = %signature_request.id,
                "Failed to send signature request email to signer"
            );
        }
    }

    Ok(CreateSignatureRequestResponse {
        signature_request,
        message: "Signature request created. Signers will receive email invitations.".into(),
    })
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
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<SignatureRequestResponse>, (StatusCode, Json<ErrorResponse>)> {
    let request = state
        .signature_request_repo
        .find_by_id_rls(&mut **rls.conn(), id)
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

    rls.release().await;

    let signer_counts = request.signer_counts();

    Ok(Json(SignatureRequestResponse {
        signature_request: request,
        signer_counts,
    }))
}

/// Send reminder to pending signers.
pub async fn send_reminder(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(request): Json<SendReminderRequest>,
) -> Result<Json<SendReminderResponse>, (StatusCode, Json<ErrorResponse>)> {
    let signature_request = state
        .signature_request_repo
        .find_by_id_rls(&mut **rls.conn(), id)
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
    rls.release().await;

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

    // Send reminder emails using the dedicated signature reminder template (Story 84.2).
    let mut reminders_sent = 0i32;
    let expires_str = signature_request
        .expires_at
        .map(|e| e.format("%Y-%m-%d").to_string());
    let doc_label = signature_request
        .subject
        .clone()
        .unwrap_or_else(|| "Document".to_string());

    let org_id_str = signature_request.organization_id.to_string();
    for signer in pending_signers {
        let signer_status = signer.status.to_string();
        let sign_url = match ESIGN_PROVIDER.build_signing_url(
            &signer.email,
            &signature_request.id.to_string(),
            &org_id_str,
            &signer_status,
        ) {
            Ok(signed) => {
                // Persist the nonce so the future /sign consumer can refuse
                // a replay of a captured reminder URL (issue #673).
                if let Err(e) = state
                    .e_signature_nonce_repo
                    .record_nonce(signature_request.id, signed.nonce)
                    .await
                {
                    warn!(
                        error = %e,
                        email = %signer.email,
                        signature_request_id = %signature_request.id,
                        "Failed to persist e-signature nonce; skipping reminder"
                    );
                    continue;
                }
                signed.url
            }
            Err(e) => {
                // Fail closed: never emit an unsigned `/sign` link. A reminder
                // URL without a valid HMAC token defeats the signed-link
                // integrity guarantee, so we skip this signer rather than send
                // an unauthenticated URL.
                warn!(
                    error = %e,
                    email = %signer.email,
                    signature_request_id = %signature_request.id,
                    "Failed to build HMAC-signed signing URL; skipping reminder (refusing to emit unsigned link)"
                );
                continue;
            }
        };

        if let Err(e) = state
            .email_service
            .send_signature_reminder_email(
                &signer.email,
                &signer.name,
                &doc_label,
                &sign_url,
                expires_str.as_deref(),
            )
            .await
        {
            warn!(
                error = %e,
                email = %signer.email,
                signature_request_id = %id,
                "Failed to send reminder email to signer"
            );
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
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(request): Json<CancelSignatureRequestRequest>,
) -> Result<Json<CancelSignatureRequestResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify the request exists and belongs to this org (RLS-enforced).
    let exists = state
        .signature_request_repo
        .find_by_id_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;
    rls.release().await;

    if exists.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Signature request not found",
            )),
        ));
    }

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
    headers: HeaderMap,
    Path(provider): Path<String>,
    Json(event): Json<SignatureWebhookEvent>,
) -> Result<Json<WebhookResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify the webhook secret header to prevent forged events.
    //
    // Fail-closed: if `ESIGN_WEBHOOK_SECRET` is unset or empty the handler
    // returns 503 rather than waving the request through. `api-server::main`
    // validates the env var at startup and panics outside of development, so
    // hitting this branch in production indicates the env was cleared
    // post-start — still better to refuse than to forge-complete signature
    // requests.
    // Issue #527 (d): fail CLOSED in non-debug builds; debug builds log a
    // warning and pass through to ease local development.
    // Issue #527 (e): constant-time comparison via subtle to defeat timing
    // side channels on the webhook secret.
    let expected_secret = match std::env::var("ESIGN_WEBHOOK_SECRET") {
        Ok(s) if !s.is_empty() => s,
        _ => {
            if cfg!(debug_assertions) {
                warn!(
                    provider = %provider,
                    "ESIGN_WEBHOOK_SECRET unset — accepting webhook (debug build only)"
                );
                String::new()
            } else {
                error!(
                    provider = %provider,
                    "Webhook rejected: ESIGN_WEBHOOK_SECRET unset at runtime"
                );
                return Err((
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(ErrorResponse::new(
                        "WEBHOOK_NOT_CONFIGURED",
                        "Webhook receiver is not configured",
                    )),
                ));
            }
        }
    };
    if !expected_secret.is_empty() {
        let provided = headers
            .get("x-webhook-secret")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let provided_b = provided.as_bytes();
        let expected_b = expected_secret.as_bytes();
        let matches =
            provided_b.len() == expected_b.len() && bool::from(provided_b.ct_eq(expected_b));
        if !matches {
            warn!(provider = %provider, "Webhook rejected: invalid or missing X-Webhook-Secret");
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("UNAUTHORIZED", "Invalid webhook secret")),
            ));
        }
    }

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

    // Idempotency guard (bug-esignature-webhook-idempotency-guard, Story 84.2):
    // e-signature providers deliver webhooks at-least-once, so a `completed` /
    // `declined` event can arrive more than once. If the request is already in
    // a terminal state, replaying the webhook must be a no-op — otherwise we
    // would re-store the signed document, re-fire the requester completion /
    // decline emails, and churn signer status on every duplicate delivery.
    // Acknowledge with 200 so the provider stops retrying.
    if signature_request.status.is_terminal() {
        info!(
            provider = %provider,
            provider_request_id = %event.provider_request_id,
            signature_request_id = %signature_request.id,
            status = %signature_request.status,
            event_type = %event.event_type,
            "Ignoring duplicate webhook for already-finalized signature request"
        );
        return Ok(Json(WebhookResponse {
            success: true,
            message: "Signature request already finalized; webhook ignored".into(),
        }));
    }

    // Process signer-specific events and send appropriate email notifications (Story 84.2).
    let updated_request = if let (Some(signer_email), Some(signer_status)) =
        (&event.signer_email, &event.signer_status)
    {
        let updated = state
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

        // Send decline notification to the requester when a signer declines (Story 84.2).
        if matches!(signer_status, db::models::SignerStatus::Declined) {
            let manage_url = format!(
                "{}/documents/{}/signatures/{}",
                *BASE_URL, signature_request.document_id, signature_request.id
            );
            let signer_name = signature_request
                .signers
                .iter()
                .find(|s| s.email.eq_ignore_ascii_case(signer_email))
                .map(|s| s.name.as_str())
                .unwrap_or("Signer");
            match state
                .user_repo
                .find_by_id(signature_request.created_by)
                .await
            {
                Ok(Some(requester)) => {
                    if let Err(e) = state
                        .email_service
                        .send_signature_declined_email(
                            &requester.email,
                            &requester.name,
                            signature_request.subject.as_deref().unwrap_or("Document"),
                            signer_name,
                            signer_email,
                            event.decline_reason.as_deref(),
                            &manage_url,
                        )
                        .await
                    {
                        warn!(
                            error = %e,
                            signature_request_id = %signature_request.id,
                            "Failed to send decline notification to requester"
                        );
                    } else {
                        info!(
                            signature_request_id = %signature_request.id,
                            requester_email = %requester.email,
                            "Sent decline notification to requester"
                        );
                    }
                }
                Ok(None) => {
                    warn!(
                        signature_request_id = %signature_request.id,
                        "Requester not found, skipping decline notification"
                    );
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        signature_request_id = %signature_request.id,
                        "Failed to look up requester for decline notification"
                    );
                }
            }
        }

        updated
    } else {
        signature_request.clone()
    };

    // Handle voided event (Coverage 84-2): a "voided" provider event means the
    // envelope was voided by the sender before completion. Map it to
    // `Cancelled` so the request enters a terminal state and subsequent
    // duplicate "voided" webhooks hit the idempotency guard above.
    //
    // We use the existing `cancel()` repository method; its
    // `WHERE status IN ('pending','in_progress')` guard makes this naturally
    // idempotent at the DB level — if somehow the request slipped into another
    // terminal state between the guard above and this call, `cancel()` returns
    // RowNotFound, which we treat as a harmless no-op (the request is already
    // finalized).
    if event.event_type == "voided" {
        match state
            .signature_request_repo
            .cancel(signature_request.id, Some("Voided by e-signature provider"))
            .await
        {
            Ok(_) => {
                info!(
                    provider = %provider,
                    provider_request_id = %event.provider_request_id,
                    signature_request_id = %signature_request.id,
                    "Marked signature request as cancelled due to provider void event"
                );
            }
            Err(e) => {
                // RowNotFound means the request is already terminal (race with
                // another path) — safe to ignore; any other error is logged but
                // we still acknowledge the webhook so the provider stops retrying.
                warn!(
                    provider = %provider,
                    provider_request_id = %event.provider_request_id,
                    signature_request_id = %signature_request.id,
                    error = %e,
                    "Failed to cancel signature request on voided webhook (may already be terminal)"
                );
            }
        }
        return Ok(Json(WebhookResponse {
            success: true,
            message: "Signature request voided".into(),
        }));
    }

    // Handle completion event: store signed document and notify requester (Stories 84.2, 88.2).
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
                    error!(
                        signature_request_id = %signature_request.id,
                        error = %e,
                        "Failed to store signed document"
                    );
                }
            }
        }

        // Notify requester that all signatures have been collected.
        let manage_url = format!(
            "{}/documents/{}/signatures/{}",
            *BASE_URL, signature_request.document_id, signature_request.id
        );
        match state
            .user_repo
            .find_by_id(signature_request.created_by)
            .await
        {
            Ok(Some(requester)) => {
                let signers_count = updated_request.signers.len();
                if let Err(e) = state
                    .email_service
                    .send_signature_completed_email(
                        &requester.email,
                        &requester.name,
                        signature_request.subject.as_deref().unwrap_or("Document"),
                        signers_count,
                        &manage_url,
                    )
                    .await
                {
                    warn!(
                        error = %e,
                        signature_request_id = %signature_request.id,
                        "Failed to send completion notification to requester"
                    );
                } else {
                    info!(
                        signature_request_id = %signature_request.id,
                        requester_email = %requester.email,
                        "Sent completion notification to requester"
                    );
                }
            }
            Ok(None) => {
                warn!(
                    signature_request_id = %signature_request.id,
                    "Requester not found, skipping completion notification"
                );
            }
            Err(e) => {
                warn!(
                    error = %e,
                    signature_request_id = %signature_request.id,
                    "Failed to look up requester for completion notification"
                );
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
    // P1-05 (follow-up): validate the URL before fetching (SSRF).
    // SSRF gate: validate the provider-supplied signed_url before fetching.
    // This prevents a malicious/compromised e-signature provider from
    // directing us at internal cloud-metadata endpoints or private networks.
    common::url_validation::validate_external_url(signed_url)
        .map_err(|e| format!("SSRF validation rejected signed_url: {}", e))?;

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

// ===========================================================================
// Signer-facing /sign consumer endpoint (Epic 84.2, issue #761 part 2)
// ===========================================================================
//
// The invitation/reminder emails embed a HMAC-signed URL of the form
// `{BASE_URL}/sign?token=<v1...>` (see `LightweightProvider::build_signing_url`).
// The frontend signing page lifts that `token` and calls these endpoints:
//
//   GET  /api/v1/signatures/sign?token=...  -> render context (no mutation)
//   POST /api/v1/signatures/sign?token=...  -> record the signature (consumes)
//
// Both are PUBLIC (no `AuthUser`): the signer is typically not a platform
// user. ALL authority comes from the verifiable token. Defence in depth:
//   1. HMAC + expiry          (`verify_token`)
//   2. org binding            (token.org_id == request.organization_id)
//   3. request lifecycle      (must be pending / in_progress)
//   4. signer not yet complete (no re-sign after signed/declined)
//   5. nonce single-use        (issued & unconsumed; POST consumes atomically)

/// Query string carrying the HMAC signing token. Only `Deserialize` is needed
/// (the `Query` extractor); these signer-facing routes are not registered in
/// `ApiDoc`, mirroring the other inquiry/sign handlers.
#[derive(Debug, Deserialize)]
pub struct SignTokenQuery {
    /// The `v1.` HMAC signing token from the emailed `/sign?token=` link.
    pub token: String,
}

/// Render context returned by `GET /sign` so the frontend can show the
/// document + signer details before the signer commits.
#[derive(Debug, Serialize, ToSchema)]
pub struct SignRenderContext {
    /// The signature request id (envelope).
    pub request_id: Uuid,
    /// The document being signed.
    pub document_id: Uuid,
    /// Human-facing subject/title of the request (document title at creation).
    pub subject: Option<String>,
    /// Optional message the requester attached for signers.
    pub message: Option<String>,
    /// The signer's display name (from the request roster).
    pub signer_name: String,
    /// The signer's email (echoed from the verified token).
    pub signer_email: String,
    /// The signer's current status (e.g. `pending`, `viewed`).
    pub signer_status: String,
}

/// Body for `POST /sign`. All fields optional so a bare `{}` is accepted;
/// `typed_name` is captured as lightweight signing evidence in the log trail.
#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(default)]
pub struct SubmitSignatureRequest {
    /// The full name the signer typed to adopt their signature (evidence).
    pub typed_name: Option<String>,
}

/// Response from `POST /sign`.
#[derive(Debug, Serialize, ToSchema)]
pub struct SubmitSignatureResponse {
    /// The request status AFTER recording this signature (`in_progress` while
    /// other signers remain, `completed` once everyone has signed).
    pub status: String,
    /// Human-facing confirmation message.
    pub message: String,
}

/// Public router for the signer-facing `/sign` endpoint. Merged at
/// `/api/v1/signatures` in `route_table()`; carries NO auth extractor.
pub fn public_sign_router() -> Router<AppState> {
    Router::new().route("/sign", get(get_sign_context).post(submit_signature))
}

/// Map a token-verification error to a public HTTP status. We deliberately
/// keep the body generic — never echo which check failed in a way that helps
/// an attacker distinguish "bad MAC" from "unknown request".
fn token_error_response(e: ESignatureError) -> (StatusCode, Json<ErrorResponse>) {
    match e {
        ESignatureError::TokenExpired => (
            StatusCode::GONE,
            Json(ErrorResponse::new(
                "SIGNING_LINK_EXPIRED",
                "This signing link has expired. Ask the sender for a new one.",
            )),
        ),
        _ => (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "INVALID_SIGNING_LINK",
                "This signing link is invalid.",
            )),
        ),
    }
}

/// Shared verification for both GET and POST: verify the token's HMAC/expiry,
/// load the request, and run the org / lifecycle / signer-state checks that
/// do NOT mutate. Returns the verified token, the loaded request, and the
/// matched signer's index so the caller can act on it.
async fn verify_and_load(
    state: &AppState,
    token_str: &str,
) -> Result<
    (
        integrations::SigningToken,
        db::models::SignatureRequest,
        usize,
    ),
    (StatusCode, Json<ErrorResponse>),
> {
    // (1) HMAC + expiry. `None` skips the explicit org arg here — we bind the
    // org below against the loaded request, which is strictly stronger.
    let token = ESIGN_PROVIDER
        .verify_token(token_str, None)
        .map_err(token_error_response)?;

    // (2) Parse the embedded request id.
    let request_id = Uuid::parse_str(&token.request_id).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "INVALID_SIGNING_LINK",
                "This signing link is invalid.",
            )),
        )
    })?;

    // (3) Load the request on the bare pool (public endpoint — no user RLS
    // context). `signature_requests` is ENABLE (not FORCE) RLS, so the
    // service path reads it directly, exactly like the webhook handler.
    let request = state
        .signature_request_repo
        .find_by_id(request_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "INVALID_SIGNING_LINK",
                    "This signing link is invalid.",
                )),
            )
        })?;

    // (4) Org binding: the token's org must match the loaded request's org.
    // Defends against a token minted for org A being used against a request
    // whose id collides in org B (belt-and-braces; the HMAC already binds it).
    if token.org_id != request.organization_id.to_string() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "INVALID_SIGNING_LINK",
                "This signing link is invalid.",
            )),
        ));
    }

    // (5) Request lifecycle: only an open request may be signed.
    if !matches!(
        request.status,
        SignatureRequestStatus::Pending | SignatureRequestStatus::InProgress
    ) {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse::new(
                "REQUEST_NOT_OPEN",
                "This signature request is no longer open for signing.",
            )),
        ));
    }

    // (6) Locate the signer by the token's (verified) email, case-insensitive.
    let signer_idx = request
        .signers
        .iter()
        .position(|s| s.email.eq_ignore_ascii_case(&token.signer_email))
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "INVALID_SIGNING_LINK",
                    "This signing link is invalid.",
                )),
            )
        })?;

    // (7) The signer must not have already completed (signed or declined).
    // This is the invariant that makes a stale-but-unconsumed reminder nonce
    // harmless: once the signer is complete, every /sign attempt is refused
    // regardless of which reminder link is replayed.
    if request.signers[signer_idx].is_complete() {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse::new(
                "ALREADY_SIGNED",
                "You have already responded to this signature request.",
            )),
        ));
    }

    Ok((token, request, signer_idx))
}

/// `GET /api/v1/signatures/sign?token=...` — render context for the signing
/// page. Read-only: validates everything but does NOT consume the nonce, so
/// the signer can load the page, reload it, and only spend the nonce on POST.
pub async fn get_sign_context(
    State(state): State<AppState>,
    Query(query): Query<SignTokenQuery>,
) -> Result<Json<SignRenderContext>, (StatusCode, Json<ErrorResponse>)> {
    let (token, request, signer_idx) = verify_and_load(&state, &query.token).await?;

    // Nonce must be issued and not yet consumed. A consumed nonce on an
    // otherwise-open request means this exact link was already used.
    match state
        .e_signature_nonce_repo
        .nonce_state(request.id, token.nonce)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })? {
        NonceState::Pending => {}
        NonceState::Consumed => {
            return Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse::new(
                    "LINK_ALREADY_USED",
                    "This signing link has already been used.",
                )),
            ));
        }
        NonceState::NotIssued => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "INVALID_SIGNING_LINK",
                    "This signing link is invalid.",
                )),
            ));
        }
    }

    let signer = &request.signers[signer_idx];
    Ok(Json(SignRenderContext {
        request_id: request.id,
        document_id: request.document_id,
        subject: request.subject.clone(),
        message: request.message.clone(),
        signer_name: signer.name.clone(),
        signer_email: signer.email.clone(),
        signer_status: signer.status.to_string(),
    }))
}

/// `POST /api/v1/signatures/sign?token=...` — record the signature.
///
/// Atomically consumes the nonce (single-use) BEFORE mutating signer status,
/// so a replay or a concurrent double-submit is rejected with 409 and never
/// double-counts a signature.
pub async fn submit_signature(
    State(state): State<AppState>,
    Query(query): Query<SignTokenQuery>,
    body: Option<Json<SubmitSignatureRequest>>,
) -> Result<Json<SubmitSignatureResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (token, request, signer_idx) = verify_and_load(&state, &query.token).await?;
    let typed_name = body.and_then(|b| b.0.typed_name);

    // Single-use gate: flip the nonce NULL -> NOW() atomically. The winner of
    // any race proceeds; everyone else (and every replay) is rejected here.
    match state
        .e_signature_nonce_repo
        .consume_nonce(request.id, token.nonce)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })? {
        ConsumeOutcome::Consumed => {}
        ConsumeOutcome::AlreadyConsumed => {
            warn!(
                signature_request_id = %request.id,
                signer_email = %token.signer_email,
                "Rejected /sign replay: nonce already consumed"
            );
            return Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse::new(
                    "LINK_ALREADY_USED",
                    "This signing link has already been used.",
                )),
            ));
        }
        ConsumeOutcome::NotIssued => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "INVALID_SIGNING_LINK",
                    "This signing link is invalid.",
                )),
            ));
        }
    }

    // Nonce is spent — record the signature. `update_signer_status` recomputes
    // the request status (in_progress / completed) and the document's
    // signature_status under the request's org context.
    let signer_email = request.signers[signer_idx].email.clone();
    let updated = state
        .signature_request_repo
        .update_signer_status(request.id, &signer_email, SignerStatus::Signed, None)
        .await
        .map_err(|e| {
            error!(
                signature_request_id = %request.id,
                error = %e,
                "Nonce consumed but failed to record signer status"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to record signature",
                )),
            )
        })?;

    info!(
        signature_request_id = %request.id,
        signer_email = %signer_email,
        typed_name = ?typed_name,
        new_status = %updated.status,
        "Recorded signature via /sign"
    );

    let message = if matches!(updated.status, SignatureRequestStatus::Completed) {
        "Thank you — all signatures have been collected.".to_string()
    } else {
        "Thank you — your signature has been recorded.".to_string()
    };

    Ok(Json(SubmitSignatureResponse {
        status: updated.status.to_string(),
        message,
    }))
}
