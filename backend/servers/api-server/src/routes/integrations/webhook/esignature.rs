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
                // Bind the envelope to the signature-verified provider before
                // mutating anything. HelloSign's `event_hash` covers only
                // `event_time + event_type`, so the `envelope_id` in the body
                // is unsigned — a captured HelloSign notification could
                // otherwise be replayed with a substituted envelope id to drive
                // an arbitrary (cross-provider / cross-tenant) workflow into a
                // terminal state. Refuse any envelope that does not belong to
                // the provider whose signature we verified.
                if !webhook_envelope_is_bound(provider, envelope_id, &wf.provider) {
                    tracing::warn!(
                        envelope_id = %envelope_id,
                        verified_provider = %provider,
                        workflow_provider = %wf.provider,
                        "E-signature webhook: envelope not bound to verified provider — rejecting"
                    );
                    return Err((
                        StatusCode::UNAUTHORIZED,
                        Json(ErrorResponse::new(
                            "ENVELOPE_NOT_BOUND",
                            "Webhook envelope is not bound to the verified provider",
                        )),
                    ));
                }

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

/// Decide whether a signature-verified webhook is allowed to act on the
/// workflow that its `envelope_id` resolved to.
///
/// HelloSign's `event_hash` MACs only `event_time + event_type` — the
/// `envelope_id` is NOT part of the signed data (DocuSign and Adobe Sign sign
/// the full raw request body, so they bind it). A captured HelloSign
/// notification can therefore be replayed with an arbitrary substituted
/// `envelope_id` in the JSON body; the `event_hash` still verifies, so the
/// owning org is resolved FROM the attacker-chosen envelope id and an
/// unrelated (cross-provider / cross-tenant) workflow is forced into a terminal
/// signed/voided/declined state.
///
/// Bind the envelope to the verified provider: only act when the resolved
/// workflow was actually created for the provider whose signature we verified,
/// and never when the envelope id is empty (an unbound envelope).
fn webhook_envelope_is_bound(
    verified_provider: &str,
    envelope_id: &str,
    workflow_provider: &str,
) -> bool {
    !envelope_id.is_empty()
        && !verified_provider.is_empty()
        && workflow_provider.eq_ignore_ascii_case(verified_provider)
}

#[cfg(test)]
mod tests {
    use super::webhook_envelope_is_bound;
    use db::models::esignature_provider;

    #[test]
    fn verified_provider_may_act_on_its_own_workflow() {
        assert!(webhook_envelope_is_bound(
            esignature_provider::HELLOSIGN,
            "env-123",
            esignature_provider::HELLOSIGN,
        ));
    }

    #[test]
    fn hellosign_webhook_rejected_for_foreign_provider_workflow() {
        // Core finding: a verified HelloSign webhook must NOT be able to drive
        // a DocuSign / Adobe / internal workflow to a terminal state, because
        // HelloSign's signature does not cover the envelope id.
        assert!(!webhook_envelope_is_bound(
            esignature_provider::HELLOSIGN,
            "env-belonging-to-docusign",
            esignature_provider::DOCUSIGN,
        ));
        assert!(!webhook_envelope_is_bound(
            esignature_provider::HELLOSIGN,
            "env-x",
            esignature_provider::INTERNAL,
        ));
    }

    #[test]
    fn empty_envelope_id_is_unbound() {
        assert!(!webhook_envelope_is_bound(
            esignature_provider::HELLOSIGN,
            "",
            esignature_provider::HELLOSIGN,
        ));
    }

    #[test]
    fn provider_match_is_case_insensitive() {
        assert!(webhook_envelope_is_bound("HelloSign", "env-1", "hellosign"));
    }
}
