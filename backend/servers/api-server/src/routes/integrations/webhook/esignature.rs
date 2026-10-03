//! Inbound e-signature provider webhook receiver (DocuSign / Adobe Sign /
//! HelloSign).
//!
//! UNMOUNTED (PAP-122): the handler writes the migration-less
//! `esignature_workflows` table. Split out of the former monolithic
//! `webhook.rs` (behaviour preserved).

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Deserialize;

use common::errors::ErrorResponse;
use db::models::esignature_provider;
use db::RlsPool;

use crate::routes::integrations::sync::{
    verify_adobe_sign_signature, verify_docusign_signature, verify_hellosign_signature,
};
use crate::state::AppState;

// ==================== Inbound Webhook Payload Types ====================

/// E-signature webhook payload with provider-specific fields.
#[derive(Debug, Deserialize)]
struct ESignatureWebhookPayload {
    provider: Option<String>,
    event_type: Option<String>,
    envelope_id: Option<String>,
    event_time: Option<String>,
    event_hash: Option<String>,
    #[serde(flatten)]
    data: serde_json::Value,
}

// ==================== Inbound: E-Signature Webhook ====================

/// E-signature webhook endpoint.
///
/// Receives webhooks from DocuSign, Adobe Sign, and HelloSign.
/// Verifies the HMAC signature before processing.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/esignatures/webhook",
    request_body(content = String, description = "Provider-specific webhook payload"),
    responses(
        (status = 200, description = "Webhook processed"),
        (status = 400, description = "Invalid payload"),
        (status = 401, description = "Invalid signature"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Integrations"
)]
pub async fn esignature_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let payload: ESignatureWebhookPayload = serde_json::from_slice(&body).map_err(|e| {
        tracing::error!(error = %e, "Failed to parse e-signature webhook payload");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_PAYLOAD",
                "Invalid webhook payload",
            )),
        )
    })?;

    let provider = payload.provider.as_deref().unwrap_or_else(|| {
        if headers.contains_key("x-docusign-signature-1") {
            esignature_provider::DOCUSIGN
        } else if headers.contains_key("x-adobesign-clientid") {
            esignature_provider::ADOBE_SIGN
        } else if payload.event_hash.is_some() {
            esignature_provider::HELLOSIGN
        } else {
            "unknown"
        }
    });

    match provider {
        esignature_provider::DOCUSIGN => {
            let secret = std::env::var("DOCUSIGN_WEBHOOK_SECRET").unwrap_or_else(|_| String::new());
            if secret.is_empty() {
                tracing::warn!("DOCUSIGN_WEBHOOK_SECRET not configured");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "CONFIG_ERROR",
                        "Webhook verification not configured",
                    )),
                ));
            }

            let signature = headers
                .get("x-docusign-signature-1")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            if !verify_docusign_signature(&secret, &body, signature) {
                tracing::warn!("Invalid DocuSign webhook signature");
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse::new(
                        "INVALID_SIGNATURE",
                        "Invalid webhook signature",
                    )),
                ));
            }
        }
        esignature_provider::ADOBE_SIGN => {
            let client_secret =
                std::env::var("ADOBE_SIGN_CLIENT_SECRET").unwrap_or_else(|_| String::new());
            if client_secret.is_empty() {
                tracing::warn!("ADOBE_SIGN_CLIENT_SECRET not configured");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "CONFIG_ERROR",
                        "Webhook verification not configured",
                    )),
                ));
            }

            let signature = headers
                .get("x-adobesign-signature")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            if !verify_adobe_sign_signature(&client_secret, &body, signature) {
                tracing::warn!("Invalid Adobe Sign webhook signature");
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse::new(
                        "INVALID_SIGNATURE",
                        "Invalid webhook signature",
                    )),
                ));
            }
        }
        esignature_provider::HELLOSIGN => {
            let api_key = std::env::var("HELLOSIGN_API_KEY").unwrap_or_else(|_| String::new());
            if api_key.is_empty() {
                tracing::warn!("HELLOSIGN_API_KEY not configured");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "CONFIG_ERROR",
                        "Webhook verification not configured",
                    )),
                ));
            }

            let event_time = payload.event_time.as_deref().unwrap_or("");
            let event_type = payload.event_type.as_deref().unwrap_or("");
            let event_hash = payload.event_hash.as_deref().unwrap_or("");

            if !verify_hellosign_signature(&api_key, event_time, event_type, event_hash) {
                tracing::warn!("Invalid HelloSign webhook signature");
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse::new(
                        "INVALID_SIGNATURE",
                        "Invalid webhook signature",
                    )),
                ));
            }
        }
        _ => {
            tracing::warn!(provider = %provider, "Unknown e-signature provider");
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "UNKNOWN_PROVIDER",
                    "Unknown e-signature provider",
                )),
            ));
        }
    }

    tracing::info!(
        provider = %provider,
        event_type = ?payload.event_type,
        envelope_id = ?payload.envelope_id,
        "Processing verified e-signature webhook"
    );

    if let (Some(envelope_id), Some(event_type)) = (
        payload.envelope_id.as_deref(),
        payload.event_type.as_deref(),
    ) {
        let new_status = match event_type {
            "envelope-completed" | "agreement_all_signed" | "signature_request_all_signed" => {
                Some("completed")
            }
            "envelope-voided" | "agreement_cancelled" | "signature_request_canceled" => {
                Some("voided")
            }
            "envelope-declined" | "agreement_rejected" | "signature_request_declined" => {
                Some("declined")
            }
            "envelope-sent" | "agreement_created" | "signature_request_sent" => Some("sent"),
            _ => None,
        };

        if let Some(status) = new_status {
            // PAP-170 (PAP-150 P5): inbound provider webhook — no request
            // principal. Resolve the owning org from the provider-signed
            // envelope id, then apply the status update under that tenant's RLS
            // context (defense in depth — the write is confined to the resolved
            // org instead of running on the raw pool).
            let rls_pool = RlsPool::new(state.db.clone());

            // Bootstrap read on a context-cleared connection: esignature_workflows
            // is keyed by the unguessable, provider-signed envelope id.
            let workflow = match rls_pool.acquire_public().await {
                Ok(mut lookup) => state
                    .integration_repo
                    .find_esignature_workflow_by_external_id(&mut **lookup.conn(), envelope_id)
                    .await
                    .unwrap_or_else(|e| {
                        tracing::warn!(
                            error = %e,
                            envelope_id = %envelope_id,
                            "Failed to resolve workflow org from webhook envelope id"
                        );
                        None
                    }),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Failed to acquire connection for e-signature webhook"
                    );
                    None
                }
            };

            if let Some(wf) = workflow {
                match rls_pool
                    .acquire_with_rls(wf.organization_id, wf.created_by, false)
                    .await
                {
                    Ok(mut guard) => {
                        if let Err(e) = state
                            .integration_repo
                            .update_esignature_workflow_by_external_id(
                                &mut **guard.conn(),
                                envelope_id,
                                status,
                            )
                            .await
                        {
                            tracing::warn!(
                                error = %e,
                                envelope_id = %envelope_id,
                                "Failed to update workflow status from webhook"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            envelope_id = %envelope_id,
                            "Failed to bind RLS context for e-signature webhook update"
                        );
                    }
                }
            } else {
                tracing::warn!(
                    envelope_id = %envelope_id,
                    "E-signature webhook: no workflow matches envelope id, ignoring"
                );
            }
        }
    }

    Ok(StatusCode::OK)
}
