//! E-Signature Integration - Epic 84.2: E-Signature Email Integration
//!
//! Provides a lightweight self-hosted HMAC-SHA256 signing token system
//! for document signature workflows, with optional DocuSign stub for future use.
//!
//! # Token format (v1)
//!
//! The wire format is `v1.<base64url(payload)>` where the payload is a
//! pipe-delimited string:
//!
//! ```text
//! v1|signer_email|request_id|org_id|signer_status|nonce|expires_at|mac_hex
//! ```
//!
//! The HMAC-SHA256 input is the same pipe-delimited fields *excluding* the
//! `mac_hex` suffix, binding every component to the secret. Including
//! `org_id` defends against cross-tenant replay; including `nonce` lets the
//! future `/sign` consumer enforce one-shot use; including `signer_status`
//! lets the consumer reject a token whose signer has already completed.
//!
//! Tokens are versioned (`v1.` prefix) so a future v2 layout can ship
//! without invalidating in-flight v1 tokens immediately.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// Wire-format version prefix for current tokens. Bumped on breaking layout
/// changes (e.g. adding a new bound field to the HMAC input).
pub const TOKEN_VERSION_PREFIX: &str = "v1.";

/// Errors from the e-signature subsystem.
#[derive(Debug, Error)]
pub enum ESignatureError {
    #[error("Invalid or expired signing token")]
    InvalidToken,
    #[error("Token has expired")]
    TokenExpired,
    #[error("HMAC error: {0}")]
    HmacError(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Base64 decode error: {0}")]
    DecodeError(String),
}

/// Signing provider variant in use.
#[derive(Debug, Clone)]
pub enum ESignatureProvider {
    Lightweight(LightweightProvider),
    Docusign(DocusignClient),
}

/// A v1 signed token binding signer identity, tenant, status, and a nonce
/// for one-shot use.
///
/// See the module-level docs for the wire format. The `nonce` is exposed so
/// that the issuing caller can persist it on the signer record and the
/// future `/sign` consumer can reject reuse and tampering.
#[derive(Debug, Clone)]
pub struct SigningToken {
    /// The signer's email address.
    pub signer_email: String,
    /// The signature request UUID.
    pub request_id: String,
    /// Tenant (organization) UUID. Binding this to the HMAC prevents
    /// cross-tenant token replay even when `request_id` collides.
    pub org_id: String,
    /// The signer's status at the moment the token was issued (e.g. "pending"
    /// or "viewed"). The future `/sign` consumer should reject tokens whose
    /// embedded status no longer matches the persisted signer.
    pub signer_status_at_issue: String,
    /// One-shot nonce generated at issue time. Stored on the signer record
    /// (by the future consumer) to detect replay.
    pub nonce: Uuid,
    /// Unix timestamp when this token expires.
    pub expires_at: u64,
    /// Raw HMAC-SHA256 bytes (32 bytes).
    pub mac_bytes: Vec<u8>,
}

impl SigningToken {
    /// Pipe-delimited HMAC input (everything except `mac_hex`).
    fn mac_input(&self) -> String {
        format!(
            "v1|{}|{}|{}|{}|{}|{}",
            self.signer_email,
            self.request_id,
            self.org_id,
            self.signer_status_at_issue,
            self.nonce,
            self.expires_at
        )
    }

    /// Create and sign a new v1 token.
    ///
    /// `secret` must be the raw secret bytes (at least
    /// [`MIN_TOKEN_SECRET_LEN`] bytes — enforced by [`LightweightConfig::from_env`]).
    /// A fresh `nonce` is generated for every call.
    ///
    /// Issue #527 (c): `org_id` is bound into the HMAC input so a token
    /// minted for org A cannot be replayed against a request in org B,
    /// even if a signer email collides across tenants.
    pub fn new(
        signer_email: &str,
        request_id: &str,
        org_id: &str,
        signer_status_at_issue: &str,
        ttl: Duration,
        secret: &[u8],
    ) -> Result<Self, ESignatureError> {
        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| ESignatureError::HmacError(e.to_string()))?
            .as_secs()
            + ttl.as_secs();

        let mut token = Self {
            signer_email: signer_email.to_string(),
            request_id: request_id.to_string(),
            org_id: org_id.to_string(),
            signer_status_at_issue: signer_status_at_issue.to_string(),
            nonce: Uuid::new_v4(),
            expires_at,
            mac_bytes: Vec::new(),
        };
        token.mac_bytes = token.compute_mac(secret)?;
        Ok(token)
    }

    fn compute_mac(&self, secret: &[u8]) -> Result<Vec<u8>, ESignatureError> {
        let mut mac = HmacSha256::new_from_slice(secret)
            .map_err(|e| ESignatureError::HmacError(e.to_string()))?;
        mac.update(self.mac_input().as_bytes());
        Ok(mac.finalize().into_bytes().to_vec())
    }

    /// Verify that this token's MAC is correct for the given secret AND
    /// that the token has not expired.
    pub fn verify(&self, secret: &[u8]) -> Result<(), ESignatureError> {
        // Check expiry first
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| ESignatureError::HmacError(e.to_string()))?
            .as_secs();
        if now > self.expires_at {
            return Err(ESignatureError::TokenExpired);
        }

        let mut mac = HmacSha256::new_from_slice(secret)
            .map_err(|e| ESignatureError::HmacError(e.to_string()))?;
        mac.update(self.mac_input().as_bytes());
        // Constant-time comparison via HMAC verify_slice
        mac.verify_slice(&self.mac_bytes)
            .map_err(|_| ESignatureError::InvalidToken)
    }

    /// Encode the token to a versioned URL-safe string.
    pub fn encode(&self) -> String {
        let mac_hex = hex::encode(&self.mac_bytes);
        let raw = format!(
            "v1|{}|{}|{}|{}|{}|{}|{}",
            self.signer_email,
            self.request_id,
            self.org_id,
            self.signer_status_at_issue,
            self.nonce,
            self.expires_at,
            mac_hex
        );
        format!(
            "{}{}",
            TOKEN_VERSION_PREFIX,
            URL_SAFE_NO_PAD.encode(raw.as_bytes())
        )
    }

    /// Decode a URL-safe versioned token string.
    ///
    /// Only the `v1.` layout is accepted — older formats (or any string
    /// without the prefix) yield [`ESignatureError::InvalidToken`]. This
    /// lets us roll new versions without ambiguity.
    pub fn decode(encoded: &str) -> Result<Self, ESignatureError> {
        let body = encoded
            .strip_prefix(TOKEN_VERSION_PREFIX)
            .ok_or(ESignatureError::InvalidToken)?;
        let raw_bytes = URL_SAFE_NO_PAD
            .decode(body)
            .map_err(|e| ESignatureError::DecodeError(e.to_string()))?;
        let raw = String::from_utf8(raw_bytes)
            .map_err(|e| ESignatureError::DecodeError(e.to_string()))?;

        // `v1|email|request_id|org_id|status|nonce|expires_at|mac_hex` — 8 parts.
        let parts: Vec<&str> = raw.splitn(8, '|').collect();
        if parts.len() != 8 || parts[0] != "v1" {
            return Err(ESignatureError::InvalidToken);
        }

        let signer_email = parts[1].to_string();
        let request_id = parts[2].to_string();
        let org_id = parts[3].to_string();
        let signer_status_at_issue = parts[4].to_string();
        let nonce = Uuid::parse_str(parts[5]).map_err(|_| ESignatureError::InvalidToken)?;
        let expires_at = parts[6]
            .parse::<u64>()
            .map_err(|_| ESignatureError::InvalidToken)?;
        let mac_bytes = hex::decode(parts[7]).map_err(|_| ESignatureError::InvalidToken)?;

        Ok(Self {
            signer_email,
            request_id,
            org_id,
            signer_status_at_issue,
            nonce,
            expires_at,
            mac_bytes,
        })
    }
}

/// Configuration for the lightweight self-hosted provider.
#[derive(Debug, Clone)]
pub struct LightweightConfig {
    /// HMAC secret (raw bytes, min 32 bytes recommended).
    pub token_secret: Vec<u8>,
    /// Previous HMAC secret used during key-rotation grace windows.
    ///
    /// When set, [`LightweightProvider::verify_token`] retries verification
    /// against this key if the current key rejects the MAC. This lets
    /// operators rotate `ESIGN_TOKEN_SECRET` without invalidating every
    /// outstanding signing URL (up to the token TTL).
    pub previous_token_secret: Option<Vec<u8>>,
    /// Base URL of the signing portal, e.g. .
    pub base_url: String,
    /// Optional webhook URL to notify on signing events.
    pub webhook_url: Option<String>,
    /// Token TTL in seconds (default: 7 days).
    pub token_ttl_secs: u64,
}

/// Minimum byte length required for `ESIGN_TOKEN_SECRET`.
///
/// Matches the 32-char floor enforced for `JWT_SECRET` in `api-server::main`
/// — both are HMAC-SHA256 keys, so the same security floor applies.
pub const MIN_TOKEN_SECRET_LEN: usize = 32;

impl LightweightConfig {
    /// Load configuration from environment variables.
    ///
    /// - `ESIGN_TOKEN_SECRET` — raw secret bytes used as the HMAC-SHA256 key.
    ///   Required in production. When `RUST_ENV=development` is set the
    ///   loader falls back to a clearly-marked development default and emits
    ///   a `warn!`; in any other environment a missing or too-short secret
    ///   is a hard error (the binary refuses to start). This mirrors the
    ///   handling of `JWT_SECRET`.
    /// - `ESIGN_PREVIOUS_TOKEN_SECRET` — optional previous key for graceful
    ///   rotation. When set, [`LightweightProvider::verify_token`] retries
    ///   against this key if the current key fails. Same 32-byte floor.
    /// - `BASE_URL` — base URL for signing links (default `http://localhost:3000`).
    /// - `ESIGN_WEBHOOK_URL` — optional outbound webhook URL.
    /// - `ESIGN_TOKEN_TTL` — TTL in seconds (default 604800 = 7 days).
    ///
    /// # Errors
    /// Returns `ESignatureError::ConfigError` if `ESIGN_TOKEN_SECRET` is
    /// missing in a non-development environment, or if it is shorter than
    /// [`MIN_TOKEN_SECRET_LEN`] bytes.
    pub fn from_env() -> Result<Self, ESignatureError> {
        let is_development = std::env::var("RUST_ENV").unwrap_or_default() == "development";

        let token_secret = match std::env::var("ESIGN_TOKEN_SECRET") {
            Ok(s) => s.into_bytes(),
            Err(_) if is_development => {
                tracing::warn!(
                    "ESIGN_TOKEN_SECRET not set, using development default (DEVELOPMENT MODE ONLY)"
                );
                // Distinct from any prod secret and long enough to clear the
                // 32-byte floor; the string is recognisable in logs.
                b"DEV_ONLY-esign-token-secret-do-not-use-in-prod".to_vec()
            }
            Err(_) => {
                return Err(ESignatureError::ConfigError(
                    "ESIGN_TOKEN_SECRET environment variable is required. \
                     Set RUST_ENV=development to use dev defaults."
                        .into(),
                ));
            }
        };

        if token_secret.len() < MIN_TOKEN_SECRET_LEN {
            return Err(ESignatureError::ConfigError(format!(
                "ESIGN_TOKEN_SECRET must be at least {MIN_TOKEN_SECRET_LEN} bytes long for minimum security",
            )));
        }

        // Optional previous key — only validated if present. Same 32-byte
        // floor; an explicitly empty string is treated as "not set" to make
        // it easy to clear once the rotation grace window has elapsed.
        let previous_token_secret = match std::env::var("ESIGN_PREVIOUS_TOKEN_SECRET") {
            Ok(s) if !s.is_empty() => {
                let bytes = s.into_bytes();
                if bytes.len() < MIN_TOKEN_SECRET_LEN {
                    return Err(ESignatureError::ConfigError(format!(
                        "ESIGN_PREVIOUS_TOKEN_SECRET must be at least {MIN_TOKEN_SECRET_LEN} bytes long when set",
                    )));
                }
                Some(bytes)
            }
            _ => None,
        };

        let base_url =
            std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
        let webhook_url = std::env::var("ESIGN_WEBHOOK_URL").ok();
        let token_ttl_secs = std::env::var("ESIGN_TOKEN_TTL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(604_800);

        Ok(Self {
            token_secret,
            previous_token_secret,
            base_url,
            webhook_url,
            token_ttl_secs,
        })
    }
}

/// A freshly-issued signing URL plus the nonce embedded in its token.
///
/// The future `/sign` consumer needs to persist this nonce on the signer
/// record so it can detect replay and tampering. The current callers
/// (route handlers, scheduler) generate the URL but the persistence step
/// lands when the consumer endpoint is wired.
#[derive(Debug, Clone)]
pub struct SignedUrl {
    /// The signing URL to embed in an email.
    pub url: String,
    /// One-shot nonce baked into the token. Persist on the signer to
    /// enforce single-use.
    pub nonce: Uuid,
}

/// Self-hosted lightweight signing provider using HMAC-SHA256 tokens.
#[derive(Debug, Clone)]
pub struct LightweightProvider {
    config: LightweightConfig,
}

impl LightweightProvider {
    pub fn new(config: LightweightConfig) -> Self {
        Self { config }
    }

    /// Instantiate from environment variables.
    ///
    /// # Errors
    /// Propagates errors from [`LightweightConfig::from_env`] (e.g. missing
    /// `ESIGN_TOKEN_SECRET` outside of development).
    pub fn from_env() -> Result<Self, ESignatureError> {
        Ok(Self::new(LightweightConfig::from_env()?))
    }

    /// Generate a fresh signed signing URL for the given signer and request.
    ///
    /// The returned [`SignedUrl`] also carries the `nonce` that was embedded
    /// in the token; callers should persist it on the signer record so the
    /// future `/sign` consumer can reject replays.
    ///
    /// Issue #527 (c): `org_id` is bound into the HMAC.
    pub fn build_signing_url(
        &self,
        signer_email: &str,
        request_id: &str,
        org_id: &str,
        signer_status_at_issue: &str,
    ) -> Result<SignedUrl, ESignatureError> {
        let token = SigningToken::new(
            signer_email,
            request_id,
            org_id,
            signer_status_at_issue,
            Duration::from_secs(self.config.token_ttl_secs),
            &self.config.token_secret,
        )?;
        let nonce = token.nonce;
        let encoded = token.encode();
        let url = format!(
            "{}/sign?token={}",
            self.config.base_url.trim_end_matches('/'),
            encoded
        );
        Ok(SignedUrl { url, nonce })
    }

    /// Decode and verify an inbound signing token.
    ///
    /// Tries the current secret first; if the MAC fails (but the token is
    /// otherwise well-formed and not expired) and a previous secret is
    /// configured via `ESIGN_PREVIOUS_TOKEN_SECRET`, retries against the
    /// previous key. This supports rolling key rotation: operators set the
    /// new key as `ESIGN_TOKEN_SECRET`, move the old value to
    /// `ESIGN_PREVIOUS_TOKEN_SECRET`, wait for outstanding URLs to expire
    /// (≤ `ESIGN_TOKEN_TTL`), then clear the previous secret.
    ///
    /// Issue #527 (c): caller MAY pass `expected_org_id` to additionally
    /// confirm the token was minted for the same tenant. Pass `None` to skip
    /// the explicit org check (the HMAC already binds org_id; this is an
    /// extra defence-in-depth layer for callers that have the org available).
    pub fn verify_token(
        &self,
        encoded: &str,
        expected_org_id: Option<&str>,
    ) -> Result<SigningToken, ESignatureError> {
        let token = SigningToken::decode(encoded)?;
        match token.verify(&self.config.token_secret) {
            Ok(()) => {}
            Err(ESignatureError::InvalidToken) => {
                if let Some(prev) = &self.config.previous_token_secret {
                    token.verify(prev)?;
                } else {
                    return Err(ESignatureError::InvalidToken);
                }
            }
            Err(e) => return Err(e),
        }
        // Constant-time-ish org binding check (orgs are UUIDs so length
        // is fixed — a plain `!=` is acceptable here).
        if let Some(expected) = expected_org_id {
            if token.org_id != expected {
                return Err(ESignatureError::InvalidToken);
            }
        }
        Ok(token)
    }

    /// Return the configured base URL.
    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }
}

// ---------------------------------------------------------------------------
// DocuSign stub (future integration)
// ---------------------------------------------------------------------------

/// Minimal DocuSign configuration (for future expansion).
#[derive(Debug, Clone)]
pub struct DocusignConfig {
    pub integration_key: String,
    pub account_id: String,
    pub base_uri: String,
    pub private_key_pem: String,
    pub impersonation_user_id: String,
}

impl DocusignConfig {
    pub fn from_env() -> Result<Self, ESignatureError> {
        Ok(Self {
            integration_key: std::env::var("DOCUSIGN_INTEGRATION_KEY").map_err(|_| {
                ESignatureError::ConfigError("DOCUSIGN_INTEGRATION_KEY missing".into())
            })?,
            account_id: std::env::var("DOCUSIGN_ACCOUNT_ID")
                .map_err(|_| ESignatureError::ConfigError("DOCUSIGN_ACCOUNT_ID missing".into()))?,
            base_uri: std::env::var("DOCUSIGN_BASE_URI")
                .unwrap_or_else(|_| "https://demo.docusign.net/restapi".to_string()),
            private_key_pem: std::env::var("DOCUSIGN_PRIVATE_KEY")
                .map_err(|_| ESignatureError::ConfigError("DOCUSIGN_PRIVATE_KEY missing".into()))?,
            impersonation_user_id: std::env::var("DOCUSIGN_USER_ID")
                .map_err(|_| ESignatureError::ConfigError("DOCUSIGN_USER_ID missing".into()))?,
        })
    }
}

/// Stub DocuSign client.  Full implementation is out of scope for this story.
#[derive(Debug, Clone)]
pub struct DocusignClient {
    pub config: DocusignConfig,
}

impl DocusignClient {
    pub fn new(config: DocusignConfig) -> Self {
        Self { config }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &[u8] = b"test-secret-key-that-is-32-bytes!";
    const ALT_SECRET: &[u8] = b"alt-secret-key-also-32-bytes-pad!";
    const ORG_ID: &str = "11111111-1111-1111-1111-111111111111";

    fn mk_token() -> SigningToken {
        SigningToken::new(
            "alice@example.com",
            "req-uuid-1234",
            ORG_ID,
            "pending",
            Duration::from_secs(3600),
            SECRET,
        )
        .expect("create token")
    }

    fn mk_config(token_secret: &[u8], previous: Option<&[u8]>) -> LightweightConfig {
        LightweightConfig {
            token_secret: token_secret.to_vec(),
            previous_token_secret: previous.map(|p| p.to_vec()),
            base_url: "https://app.example.com".to_string(),
            webhook_url: None,
            token_ttl_secs: 3600,
        }
    }

    #[test]
    fn test_signing_token_roundtrip() {
        let token = mk_token();
        let encoded = token.encode();
        assert!(
            encoded.starts_with(TOKEN_VERSION_PREFIX),
            "Encoded token must carry the version prefix"
        );

        let decoded = SigningToken::decode(&encoded).expect("decode token");
        assert_eq!(decoded.signer_email, token.signer_email);
        assert_eq!(decoded.request_id, token.request_id);
        assert_eq!(decoded.org_id, token.org_id);
        assert_eq!(decoded.signer_status_at_issue, token.signer_status_at_issue);
        assert_eq!(decoded.nonce, token.nonce);
        assert_eq!(decoded.expires_at, token.expires_at);
    }

    #[test]
    fn test_signing_token_verify_ok() {
        mk_token().verify(SECRET).expect("verification should pass");
    }

    #[test]
    fn test_signing_token_wrong_secret_rejected() {
        let result = mk_token().verify(b"wrong-secret-32-bytes-padding!!!");
        assert!(
            matches!(result, Err(ESignatureError::InvalidToken)),
            "Should reject token signed with different secret, got {result:?}"
        );
    }

    #[test]
    fn test_signing_token_expired_rejected() {
        let mut token = mk_token();
        // Backdate the expiry and re-sign so the MAC still validates against
        // SECRET; only the expiry check should fail.
        token.expires_at = 1;
        token.mac_bytes = token.compute_mac(SECRET).unwrap();

        let result = token.verify(SECRET);
        assert!(
            matches!(result, Err(ESignatureError::TokenExpired)),
            "Should return TokenExpired for past token"
        );
    }

    #[test]
    fn test_v1_prefix_required_for_decode() {
        let token = mk_token();
        let encoded = token.encode();
        // Strip the version prefix and confirm decode refuses the legacy
        // shape — there is no v0 fallback by design.
        let stripped = encoded.trim_start_matches(TOKEN_VERSION_PREFIX);
        let result = SigningToken::decode(stripped);
        assert!(
            matches!(result, Err(ESignatureError::InvalidToken)),
            "Unprefixed token must be rejected, got {result:?}"
        );
    }

    #[test]
    fn test_tampered_org_id_rejected() {
        let mut token = mk_token();
        // Tamper after signing — the embedded MAC was computed over the
        // original org_id, so verify must reject the altered token.
        token.org_id = "22222222-2222-2222-2222-222222222222".to_string();
        let result = token.verify(SECRET);
        assert!(
            matches!(result, Err(ESignatureError::InvalidToken)),
            "Tampered org_id must invalidate the MAC, got {result:?}"
        );
    }

    #[test]
    fn test_tampered_signer_email_rejected() {
        let mut token = mk_token();
        token.signer_email = "mallory@example.com".to_string();
        assert!(matches!(
            token.verify(SECRET),
            Err(ESignatureError::InvalidToken)
        ));
    }

    #[test]
    fn test_tampered_signer_status_rejected() {
        let mut token = mk_token();
        token.signer_status_at_issue = "signed".to_string();
        assert!(matches!(
            token.verify(SECRET),
            Err(ESignatureError::InvalidToken)
        ));
    }

    #[test]
    fn test_tampered_nonce_rejected() {
        let mut token = mk_token();
        token.nonce = Uuid::new_v4();
        assert!(matches!(
            token.verify(SECRET),
            Err(ESignatureError::InvalidToken)
        ));
    }

    #[test]
    fn test_tampered_expiry_rejected() {
        let mut token = mk_token();
        // Push expiry far into the future without re-signing.
        token.expires_at = token.expires_at.saturating_add(7 * 86_400);
        // MAC is wrong (signed over original expires_at), so this must be
        // InvalidToken — NOT TokenExpired (which it would be without the
        // tamper) — meaning the attacker can't extend the lifetime.
        let result = token.verify(SECRET);
        assert!(
            matches!(result, Err(ESignatureError::InvalidToken)),
            "Tampered expiry must fail MAC verification, got {result:?}"
        );
    }

    #[test]
    fn test_decode_oversize_garbage_rejected() {
        // Garbage bytes that aren't valid base64url with v1. prefix → DecodeError.
        let result = SigningToken::decode("v1.@@@not-base64@@@");
        assert!(
            matches!(result, Err(ESignatureError::DecodeError(_))),
            "Malformed base64 must surface DecodeError, got {result:?}"
        );
    }

    #[test]
    fn test_lightweight_provider_build_signing_url() {
        let provider = LightweightProvider::new(mk_config(SECRET, None));
        let signed = provider
            .build_signing_url("alice@example.com", "req-1", ORG_ID, "pending")
            .expect("build url");

        assert!(signed
            .url
            .starts_with("https://app.example.com/sign?token="));
        // Nonce should be a fresh UUID — different on the next call.
        let signed2 = provider
            .build_signing_url("alice@example.com", "req-1", ORG_ID, "pending")
            .expect("build url");
        assert_ne!(
            signed.nonce, signed2.nonce,
            "Each build_signing_url call must produce a fresh nonce"
        );
    }

    #[test]
    fn test_lightweight_provider_verify_token_roundtrip() {
        let provider = LightweightProvider::new(mk_config(SECRET, None));
        let signed = provider
            .build_signing_url("alice@example.com", "req-1", ORG_ID, "pending")
            .expect("build url");

        let token_str = signed.url.split("token=").nth(1).unwrap();
        let verified = provider
            .verify_token(token_str, Some(ORG_ID))
            .expect("verify_token should succeed");

        assert_eq!(verified.signer_email, "alice@example.com");
        assert_eq!(verified.request_id, "req-1");
        assert_eq!(verified.org_id, ORG_ID);
        assert_eq!(verified.signer_status_at_issue, "pending");
        assert_eq!(verified.nonce, signed.nonce);
    }

    #[test]
    fn test_lightweight_provider_verify_token_wrong_org_rejected() {
        let provider = LightweightProvider::new(mk_config(SECRET, None));
        let signed = provider
            .build_signing_url("alice@example.com", "req-1", ORG_ID, "pending")
            .expect("build url");

        let token_str = signed.url.split("token=").nth(1).unwrap();
        let other_org = "00000000-0000-0000-0000-000000000999";
        let result = provider.verify_token(token_str, Some(other_org));
        assert!(
            matches!(result, Err(ESignatureError::InvalidToken)),
            "Should reject token bound to a different org"
        );
    }

    #[test]
    fn test_previous_key_fallback_during_rotation() {
        // Issue a token under SECRET, then verify with a provider whose
        // *current* key is ALT_SECRET and previous key is SECRET. This is
        // the post-rotation state: the new key is live but old tokens must
        // still verify until they expire.
        let issuer = LightweightProvider::new(mk_config(SECRET, None));
        let signed = issuer
            .build_signing_url("alice@example.com", "req-1", ORG_ID, "pending")
            .expect("build url");
        let token_str = signed.url.split("token=").nth(1).unwrap();

        let post_rotation = LightweightProvider::new(mk_config(ALT_SECRET, Some(SECRET)));
        let verified = post_rotation
            .verify_token(token_str, Some(ORG_ID))
            .expect("fallback to previous key should accept old token");
        assert_eq!(verified.nonce, signed.nonce);

        // Without the previous-key fallback, the same token must fail.
        let no_fallback = LightweightProvider::new(mk_config(ALT_SECRET, None));
        let result = no_fallback.verify_token(token_str, Some(ORG_ID));
        assert!(
            matches!(result, Err(ESignatureError::InvalidToken)),
            "Without previous-key fallback, post-rotation token must fail"
        );
    }

    // ---------------------------------------------------------------------------
    // from_env() tests for #527 / Bucket A
    //
    // These tests mutate process-global env vars and must not run in parallel
    // with each other or with any other env-touching test in this crate.
    // A static mutex serialises them; we restore prior env state on drop.
    // ---------------------------------------------------------------------------

    use std::sync::Mutex;
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        keys: Vec<(&'static str, Option<String>)>,
    }
    impl EnvGuard {
        fn capture(keys: &[&'static str]) -> Self {
            let snap = keys
                .iter()
                .map(|k| (*k, std::env::var(k).ok()))
                .collect::<Vec<_>>();
            for k in keys {
                std::env::remove_var(k);
            }
            Self { keys: snap }
        }
    }
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (k, v) in &self.keys {
                match v {
                    Some(val) => std::env::set_var(k, val),
                    None => std::env::remove_var(k),
                }
            }
        }
    }

    #[test]
    fn from_env_fails_when_token_secret_missing_in_prod() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _env = EnvGuard::capture(&[
            "ESIGN_TOKEN_SECRET",
            "ESIGN_PREVIOUS_TOKEN_SECRET",
            "RUST_ENV",
        ]);
        // RUST_ENV is not "development" (it's unset), so missing secret => error.

        let result = LightweightConfig::from_env();
        assert!(
            matches!(result, Err(ESignatureError::ConfigError(_))),
            "Expected ConfigError for missing ESIGN_TOKEN_SECRET outside dev, got {result:?}"
        );
    }

    #[test]
    fn from_env_fails_when_token_secret_too_short() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _env = EnvGuard::capture(&[
            "ESIGN_TOKEN_SECRET",
            "ESIGN_PREVIOUS_TOKEN_SECRET",
            "RUST_ENV",
        ]);
        std::env::set_var("ESIGN_TOKEN_SECRET", "tooshort"); // 8 bytes — below floor

        let result = LightweightConfig::from_env();
        let msg = match result {
            Err(ESignatureError::ConfigError(m)) => m,
            other => panic!("Expected ConfigError for too-short secret, got {other:?}"),
        };
        assert!(
            msg.contains("at least"),
            "Error message should reference the length floor, got: {msg}"
        );
    }

    #[test]
    fn from_env_allows_dev_default_when_rust_env_development() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _env = EnvGuard::capture(&[
            "ESIGN_TOKEN_SECRET",
            "ESIGN_PREVIOUS_TOKEN_SECRET",
            "RUST_ENV",
        ]);
        std::env::set_var("RUST_ENV", "development");

        let cfg = LightweightConfig::from_env().expect("dev default should succeed");
        assert!(cfg.token_secret.len() >= MIN_TOKEN_SECRET_LEN);
    }

    #[test]
    fn from_env_accepts_valid_secret_in_prod() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _env = EnvGuard::capture(&[
            "ESIGN_TOKEN_SECRET",
            "ESIGN_PREVIOUS_TOKEN_SECRET",
            "RUST_ENV",
        ]);
        std::env::set_var(
            "ESIGN_TOKEN_SECRET",
            "this-is-a-valid-32-byte-or-longer-secret",
        );

        let cfg = LightweightConfig::from_env().expect("valid secret should succeed");
        assert_eq!(
            cfg.token_secret,
            b"this-is-a-valid-32-byte-or-longer-secret"
        );
    }
}
