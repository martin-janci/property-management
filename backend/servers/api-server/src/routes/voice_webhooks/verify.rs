//! The JSON signature-introspection endpoint `POST /api/v1/webhooks/voice/verify`
//! (Story 93.3).
//!
//! Split verbatim from `voice_webhooks.rs`. This is a developer-facing
//! introspection endpoint, NOT the cryptographic receiver: it can verify the
//! Google HMAC branch, but it deliberately **fails closed** for Alexa (whose
//! real verification is certificate-chain based and performed by
//! `POST /api/v1/webhooks/voice/alexa`). The old `valid: !signature.is_empty()`
//! behaviour was a forgery vector; that fix is carried over unchanged. The
//! router that mounts this handler lives in [`super`].

use super::signature::verify_hmac_signature;
use axum::Json;
use db::models::WebhookVerificationResult;
use serde::Deserialize;
use utoipa::ToSchema;

// ============================================================================
// Signature Verification endpoint (Story 93.3)
// ============================================================================

/// Verify webhook request signature.
#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct VerifyWebhookRequest {
    pub platform: String,
    pub signature: String,
    pub body: String,
    pub timestamp: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/v1/webhooks/voice/verify",
    request_body = VerifyWebhookRequest,
    responses(
        (status = 200, description = "Verification result", body = WebhookVerificationResult),
    ),
    tag = "Voice Webhooks"
)]
pub(crate) async fn verify_webhook_signature(
    Json(request): Json<VerifyWebhookRequest>,
) -> Json<WebhookVerificationResult> {
    let result = match request.platform.as_str() {
        "alexa" => {
            // SECURITY: Alexa request signing is certificate-chain based — the
            // caller must present a `SignatureCertChainUrl` header, a base64
            // `Signature` header, and the *raw* request body, which are then
            // verified as a PKCS#1 v1.5 RSA-SHA1 signature under an Amazon-issued
            // leaf certificate (see `verify_alexa_signature` /
            // `verify_alexa_cert_and_signature`, wired into the mounted
            // `POST /api/v1/webhooks/voice/alexa` receiver).
            //
            // This JSON introspection endpoint receives none of that material
            // (no cert-chain URL, and `body` is a lossy string rather than the
            // raw signed bytes), so it CANNOT cryptographically verify an Alexa
            // signature and MUST NOT claim it did. The previous implementation
            // returned `valid: !signature.is_empty()`, accepting any non-empty
            // string as a valid Alexa signature — a forgery vector that let an
            // attacker have an arbitrary payload reported as "valid". Fail
            // CLOSED: never report `valid: true` from this endpoint for Alexa.
            WebhookVerificationResult {
                valid: false,
                platform: "alexa".to_string(),
                error: Some(if request.signature.is_empty() {
                    "Missing signature".to_string()
                } else {
                    "Alexa signature verification is certificate-chain based and \
                     is performed by POST /api/v1/webhooks/voice/alexa; this \
                     endpoint cannot verify Alexa signatures"
                        .to_string()
                }),
            }
        }
        "google" => {
            // Google uses HMAC-SHA256
            match verify_hmac_signature(&request.signature, &request.body) {
                Ok(valid) => WebhookVerificationResult {
                    valid,
                    platform: "google".to_string(),
                    error: if valid {
                        None
                    } else {
                        Some("Invalid signature".to_string())
                    },
                },
                Err(e) => WebhookVerificationResult {
                    valid: false,
                    platform: "google".to_string(),
                    error: Some(e),
                },
            }
        }
        _ => WebhookVerificationResult {
            valid: false,
            platform: request.platform.clone(),
            error: Some("Unknown platform".to_string()),
        },
    };

    Json(result)
}

// ============================================================================
// Tests
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

    // Compute a valid HMAC-SHA256 base64 signature for `body` under `secret`,
    // mirroring `signature::verify_hmac_signature`'s algorithm exactly.
    fn hmac_sig(secret: &str, body: &str) -> String {
        use hmac::{Hmac, KeyInit, Mac};
        use sha2::Sha256;
        let mut mac = <Hmac<Sha256>>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body.as_bytes());
        BASE64.encode(mac.finalize().into_bytes())
    }

    // --- verify_hmac_signature + the mounted /verify handler ------------
    //
    // VOICE_WEBHOOK_SECRET is process-global and read by
    // `signature::verify_hmac_signature` (directly and via the `/verify`
    // "google" branch). All of its assertions live in this one test so they run
    // sequentially and never race.
    #[tokio::test]
    async fn hmac_and_verify_endpoint_google_branch() {
        std::env::set_var("VOICE_WEBHOOK_SECRET", "unit-test-secret");
        let body = "payload-to-sign";
        let good = hmac_sig("unit-test-secret", body);

        // Direct helper: matching signature -> true, tampered -> false.
        assert!(verify_hmac_signature(&good, body).unwrap());
        assert!(!verify_hmac_signature("wrong", body).unwrap());

        // Mounted /verify endpoint, "google" platform: valid signature.
        let resp = verify_webhook_signature(Json(VerifyWebhookRequest {
            platform: "google".to_string(),
            signature: good.clone(),
            body: body.to_string(),
            timestamp: None,
        }))
        .await;
        assert!(resp.0.valid);
        assert_eq!(resp.0.platform, "google");
        assert!(resp.0.error.is_none());

        // Mounted /verify endpoint, "google" platform: invalid signature.
        let resp = verify_webhook_signature(Json(VerifyWebhookRequest {
            platform: "google".to_string(),
            signature: "bad".to_string(),
            body: body.to_string(),
            timestamp: None,
        }))
        .await;
        assert!(!resp.0.valid);
        assert_eq!(resp.0.error.as_deref(), Some("Invalid signature"));

        // Issue #2658 (#2): with the secret unset, verification fails closed
        // (Err) rather than silently verifying against a hardcoded default.
        std::env::remove_var("VOICE_WEBHOOK_SECRET");
        assert!(verify_hmac_signature(&good, body).is_err());

        // ...and a *blank* secret is treated the same as unset — never key the
        // HMAC with an empty value (mirrors the sibling PORTAL/AIRBNB/STRIPE
        // receivers). Both a correct-looking and a bogus signature must Err.
        std::env::set_var("VOICE_WEBHOOK_SECRET", "");
        assert!(verify_hmac_signature(&good, body).is_err());
        assert!(verify_hmac_signature("anything", body).is_err());
        std::env::remove_var("VOICE_WEBHOOK_SECRET");
    }

    #[tokio::test]
    async fn verify_endpoint_alexa_branch_fails_closed() {
        // SECURITY regression: the JSON /verify endpoint must NEVER report an
        // Alexa signature as valid. It has no certificate-chain URL and no raw
        // signed body, so it cannot perform Amazon's cert-based verification;
        // the old code returned `valid: !signature.is_empty()`, accepting any
        // non-empty string as a valid Alexa signature (a forgery vector).

        // A forged, attacker-controlled non-empty "signature" is rejected — this
        // is exactly the input the old code accepted as valid.
        let resp = verify_webhook_signature(Json(VerifyWebhookRequest {
            platform: "alexa".to_string(),
            signature: "any-forged-non-empty-string".to_string(),
            body: r#"{"request":{"type":"IntentRequest","intent":{"name":"evil"}}}"#.to_string(),
            timestamp: None,
        }))
        .await;
        assert!(
            !resp.0.valid,
            "a non-empty (forged) Alexa signature must NOT be accepted"
        );
        assert_eq!(resp.0.platform, "alexa");
        assert!(
            resp.0
                .error
                .as_deref()
                .is_some_and(|e| e.contains("cannot verify Alexa signatures")),
            "unexpected error: {:?}",
            resp.0.error
        );

        // Even a plausibly base64-shaped signature is rejected — there is no
        // input to this endpoint that yields `valid: true` for Alexa.
        let resp = verify_webhook_signature(Json(VerifyWebhookRequest {
            platform: "alexa".to_string(),
            signature: BASE64.encode([0u8; 256]),
            body: "b".to_string(),
            timestamp: None,
        }))
        .await;
        assert!(!resp.0.valid, "no Alexa input may be reported as valid");

        // Empty signature -> still invalid, with the "Missing signature" hint.
        let resp = verify_webhook_signature(Json(VerifyWebhookRequest {
            platform: "alexa".to_string(),
            signature: String::new(),
            body: "b".to_string(),
            timestamp: None,
        }))
        .await;
        assert!(!resp.0.valid);
        assert_eq!(resp.0.error.as_deref(), Some("Missing signature"));
    }

    #[tokio::test]
    async fn verify_endpoint_unknown_platform() {
        let resp = verify_webhook_signature(Json(VerifyWebhookRequest {
            platform: "siri".to_string(),
            signature: "x".to_string(),
            body: "b".to_string(),
            timestamp: None,
        }))
        .await;
        assert!(!resp.0.valid);
        assert_eq!(resp.0.platform, "siri");
        assert_eq!(resp.0.error.as_deref(), Some("Unknown platform"));
    }
}
