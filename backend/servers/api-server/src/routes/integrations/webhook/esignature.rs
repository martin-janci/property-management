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
use uuid::Uuid;

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
            // HelloSign signature verification is DEFERRED to after the owning
            // workflow is resolved from the envelope id (see the post-resolution
            // block below). Unlike DocuSign/Adobe — which MAC the full request
            // body (envelope id included) and can therefore be verified up front
            // with an app-global secret — HelloSign's `event_hash` MACs only
            // `event_time + event_type`. Verifying that with a single global key
            // proves nothing about which tenant owns the envelope, so the same
            // tuple is replayable across every tenant (#3666). We instead verify
            // it against the per-organization HelloSign secret of the resolved
            // workflow, binding the signature to the owning account.
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
                // Account binding (#3666): verify the HelloSign signature with
                // the per-ORGANIZATION secret of the resolved workflow, not a
                // shared app-global key. HelloSign's `event_hash` MACs only
                // `event_time + event_type` (the envelope id is unsigned), so a
                // single valid tuple verified under one global key is otherwise
                // valid for EVERY tenant's workflow of that event type — an
                // attacker who captures one such tuple from their own HelloSign
                // account can replay it against any victim org's envelope id and
                // force that workflow to a terminal state. Verifying against the
                // owning org's own secret makes a signature valid only for the
                // account that actually owns the envelope. Fail closed when no
                // per-tenant secret is configured — never fall back to a shared
                // key that would reopen the cross-tenant replay. DocuSign/Adobe
                // already bind the envelope (they MAC the full body), so they
                // keep their up-front verification above and are skipped here.
                if provider == esignature_provider::HELLOSIGN {
                    let event_time = payload.event_time.as_deref().unwrap_or("");
                    let event_hash = payload.event_hash.as_deref().unwrap_or("");
                    let verified = hellosign_org_secret(wf.organization_id)
                        .map(|secret| {
                            verify_hellosign_signature(&secret, event_time, event_type, event_hash)
                        })
                        .unwrap_or(false);
                    if !verified {
                        tracing::warn!(
                            envelope_id = %envelope_id,
                            organization_id = %wf.organization_id,
                            "HelloSign webhook signature not valid for the envelope's owning \
                             account — rejecting (cross-tenant replay guard)"
                        );
                        return Err((
                            StatusCode::UNAUTHORIZED,
                            Json(ErrorResponse::new(
                                "INVALID_SIGNATURE",
                                "Invalid webhook signature",
                            )),
                        ));
                    }
                }

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

/// Resolve the per-organization HelloSign webhook secret used to verify an
/// inbound HelloSign notification for a specific tenant.
///
/// HelloSign signs its webhooks with an account API key and does **not** include
/// the envelope id in the signed material, so verifying every tenant's webhook
/// with one shared app-global key cannot tell one org's workflow apart from
/// another's (#3666). Until durable per-connection secret storage lands — the
/// prerequisite to remounting this surface; the `esignature_workflows` table
/// itself ships in no migration today (PAP-122) — per-tenant HelloSign secrets
/// are sourced from the `HELLOSIGN_WEBHOOK_SECRETS` env var: a JSON object
/// mapping organization id to that org's HelloSign API key, e.g.
/// `{"<org-uuid>":"<secret>"}`.
///
/// Returns `None` when no secret is configured for `organization_id`; the caller
/// treats that as a hard rejection (fail closed) and never falls back to a
/// shared key that would reopen the cross-tenant replay.
fn hellosign_org_secret(organization_id: Uuid) -> Option<String> {
    let raw = std::env::var("HELLOSIGN_WEBHOOK_SECRETS").ok()?;
    lookup_hellosign_secret(&raw, organization_id)
}

/// Pure lookup of a per-organization HelloSign secret from the raw
/// `HELLOSIGN_WEBHOOK_SECRETS` JSON map. Split out of [`hellosign_org_secret`]
/// so the account-binding behaviour is unit-testable without touching process
/// environment. Returns `None` for malformed JSON, a missing org entry, or an
/// empty secret.
fn lookup_hellosign_secret(secrets_json: &str, organization_id: Uuid) -> Option<String> {
    let map: std::collections::HashMap<String, String> = serde_json::from_str(secrets_json).ok()?;
    map.get(&organization_id.to_string())
        .filter(|s| !s.is_empty())
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::{lookup_hellosign_secret, webhook_envelope_is_bound};
    use crate::routes::integrations::sync::verify_hellosign_signature;
    use db::models::esignature_provider;
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;
    use uuid::Uuid;

    type HmacSha256 = Hmac<Sha256>;

    /// Mint a HelloSign `event_hash` exactly the way HelloSign does: an
    /// HMAC-SHA256 over `event_time + event_type` keyed by the account API key,
    /// hex-encoded. Used to prove the cross-tenant account-binding guard.
    fn mint_hellosign_hash(api_key: &str, event_time: &str, event_type: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(api_key.as_bytes()).expect("hmac key");
        mac.update(format!("{}{}", event_time, event_type).as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

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

    // ---- #3666: per-account HelloSign secret binding ----

    #[test]
    fn lookup_returns_the_secret_for_the_requested_org() {
        let org_a = Uuid::new_v4();
        let org_b = Uuid::new_v4();
        let json = format!(r#"{{"{}":"secret-a","{}":"secret-b"}}"#, org_a, org_b);
        assert_eq!(
            lookup_hellosign_secret(&json, org_a).as_deref(),
            Some("secret-a")
        );
        assert_eq!(
            lookup_hellosign_secret(&json, org_b).as_deref(),
            Some("secret-b")
        );
    }

    #[test]
    fn lookup_is_none_for_unconfigured_org_malformed_json_or_empty_secret() {
        let org = Uuid::new_v4();
        let other = Uuid::new_v4();
        // org has no entry
        let json = format!(r#"{{"{}":"secret"}}"#, other);
        assert!(lookup_hellosign_secret(&json, org).is_none());
        // malformed JSON
        assert!(lookup_hellosign_secret("not json", org).is_none());
        // empty secret is treated as unconfigured (fail closed)
        let empty = format!(r#"{{"{}":""}}"#, org);
        assert!(lookup_hellosign_secret(&empty, org).is_none());
    }

    #[test]
    fn signature_minted_with_tenant_a_key_is_rejected_against_tenant_b_envelope() {
        // Core regression (#3666): HelloSign MACs only `event_time+event_type`
        // and omits the envelope id, so a tuple captured from tenant A's own
        // HelloSign account must NOT verify against a tenant B envelope once we
        // bind verification to the owning account's per-org secret.
        let org_a = Uuid::new_v4();
        let org_b = Uuid::new_v4();
        let secrets = format!(r#"{{"{}":"key-a","{}":"key-b"}}"#, org_a, org_b);

        let event_time = "1700000000";
        let event_type = "signature_request_all_signed";
        // Attacker mints a valid tuple using THEIR account (org A) key.
        let event_hash = mint_hellosign_hash("key-a", event_time, event_type);

        // Replayed against org A's own envelope: verifies (legitimate path).
        let secret_a = lookup_hellosign_secret(&secrets, org_a).expect("org a secret");
        assert!(
            verify_hellosign_signature(&secret_a, event_time, event_type, &event_hash),
            "legitimate owner must still verify"
        );

        // Replayed against victim org B's envelope: the handler resolves org B,
        // verifies with org B's secret, and the HMAC no longer matches.
        let secret_b = lookup_hellosign_secret(&secrets, org_b).expect("org b secret");
        assert!(
            !verify_hellosign_signature(&secret_b, event_time, event_type, &event_hash),
            "cross-tenant replay must be rejected under the owning account's key"
        );
    }

    #[test]
    fn unconfigured_org_fails_closed() {
        // When no per-tenant secret is configured for the resolved org, the
        // handler gets None and rejects — it must never fall back to a shared
        // key that would reopen the cross-tenant replay.
        let org = Uuid::new_v4();
        let secrets = r#"{}"#;
        assert!(lookup_hellosign_secret(secrets, org).is_none());
    }
}
