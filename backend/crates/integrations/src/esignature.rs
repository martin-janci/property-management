//! E-Signature Integration - Epic 84.2: E-Signature Email Integration
//!
//! Provides a lightweight self-hosted HMAC-SHA256 signing token system
//! for document signature workflows, with optional DocuSign stub for future use.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

type HmacSha256 = Hmac<Sha256>;

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

/// A signed token embedding signer identity and expiry.
#[derive(Debug, Clone)]
pub struct SigningToken {
    /// The signer's email address.
    pub signer_email: String,
    /// The signature request UUID.
    pub request_id: String,
    /// Unix timestamp when this token expires.
    pub expires_at: u64,
    /// Raw HMAC-SHA256 bytes (32 bytes).
    pub mac_bytes: Vec<u8>,
}

impl SigningToken {
    /// Create and sign a new token.  must be the raw secret bytes.
    pub fn new(
        signer_email: &str,
        request_id: &str,
        ttl: Duration,
        secret: &[u8],
    ) -> Result<Self, ESignatureError> {
        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| ESignatureError::HmacError(e.to_string()))?
            .as_secs()
            + ttl.as_secs();

        let message = format!("{}|{}|{}", signer_email, request_id, expires_at);
        let mut mac = HmacSha256::new_from_slice(secret)
            .map_err(|e| ESignatureError::HmacError(e.to_string()))?;
        mac.update(message.as_bytes());
        let mac_bytes = mac.finalize().into_bytes().to_vec();

        Ok(Self {
            signer_email: signer_email.to_string(),
            request_id: request_id.to_string(),
            expires_at,
            mac_bytes,
        })
    }

    /// Verify that this token's MAC is correct for the given secret.
    pub fn verify(&self, secret: &[u8]) -> Result<(), ESignatureError> {
        // Check expiry first
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| ESignatureError::HmacError(e.to_string()))?
            .as_secs();
        if now > self.expires_at {
            return Err(ESignatureError::TokenExpired);
        }

        let message = format!(
            "{}|{}|{}",
            self.signer_email, self.request_id, self.expires_at
        );
        let mut mac = HmacSha256::new_from_slice(secret)
            .map_err(|e| ESignatureError::HmacError(e.to_string()))?;
        mac.update(message.as_bytes());
        // Constant-time comparison via HMAC verify_slice
        mac.verify_slice(&self.mac_bytes)
            .map_err(|_| ESignatureError::InvalidToken)
    }

    /// Encode the token to a URL-safe base64 string: .
    pub fn encode(&self) -> String {
        let mac_hex = hex::encode(&self.mac_bytes);
        let raw = format!(
            "{}|{}|{}|{}",
            self.signer_email, self.request_id, self.expires_at, mac_hex
        );
        URL_SAFE_NO_PAD.encode(raw.as_bytes())
    }

    /// Decode a URL-safe base64 token string back to a .
    pub fn decode(encoded: &str) -> Result<Self, ESignatureError> {
        let raw_bytes = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|e| ESignatureError::DecodeError(e.to_string()))?;
        let raw = String::from_utf8(raw_bytes)
            .map_err(|e| ESignatureError::DecodeError(e.to_string()))?;

        let parts: Vec<&str> = raw.splitn(4, '|').collect();
        if parts.len() != 4 {
            return Err(ESignatureError::InvalidToken);
        }

        let signer_email = parts[0].to_string();
        let request_id = parts[1].to_string();
        let expires_at = parts[2]
            .parse::<u64>()
            .map_err(|_| ESignatureError::InvalidToken)?;
        let mac_bytes = hex::decode(parts[3]).map_err(|_| ESignatureError::InvalidToken)?;

        Ok(Self {
            signer_email,
            request_id,
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

        let base_url =
            std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
        let webhook_url = std::env::var("ESIGN_WEBHOOK_URL").ok();
        let token_ttl_secs = std::env::var("ESIGN_TOKEN_TTL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(604_800);

        Ok(Self {
            token_secret,
            base_url,
            webhook_url,
            token_ttl_secs,
        })
    }
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

    /// Generate a signed signing URL for the given signer and request.
    pub fn build_signing_url(
        &self,
        signer_email: &str,
        request_id: &str,
    ) -> Result<String, ESignatureError> {
        let token = SigningToken::new(
            signer_email,
            request_id,
            Duration::from_secs(self.config.token_ttl_secs),
            &self.config.token_secret,
        )?;
        let encoded = token.encode();
        Ok(format!(
            "{}/sign?token={}",
            self.config.base_url.trim_end_matches('/'),
            encoded
        ))
    }

    /// Decode and verify an inbound signing token from a URL query parameter.
    pub fn verify_token(&self, encoded: &str) -> Result<SigningToken, ESignatureError> {
        let token = SigningToken::decode(encoded)?;
        token.verify(&self.config.token_secret)?;
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

    #[test]
    fn test_signing_token_roundtrip() {
        let token = SigningToken::new(
            "alice@example.com",
            "req-uuid-1234",
            Duration::from_secs(3600),
            SECRET,
        )
        .expect("create token");

        let encoded = token.encode();
        let decoded = SigningToken::decode(&encoded).expect("decode token");

        assert_eq!(decoded.signer_email, "alice@example.com");
        assert_eq!(decoded.request_id, "req-uuid-1234");
        assert_eq!(decoded.expires_at, token.expires_at);
    }

    #[test]
    fn test_signing_token_verify_ok() {
        let token = SigningToken::new(
            "alice@example.com",
            "req-uuid-1234",
            Duration::from_secs(3600),
            SECRET,
        )
        .expect("create token");

        token.verify(SECRET).expect("verification should pass");
    }

    #[test]
    fn test_signing_token_wrong_secret_rejected() {
        let token = SigningToken::new(
            "alice@example.com",
            "req-uuid-1234",
            Duration::from_secs(3600),
            SECRET,
        )
        .expect("create token");

        let result = token.verify(b"wrong-secret-32-bytes-padding!!!");
        assert!(
            result.is_err(),
            "Should reject token signed with different secret"
        );
    }

    #[test]
    fn test_signing_token_expired_rejected() {
        // Build a token that expires in the past by manipulating expires_at
        let mut token = SigningToken::new(
            "alice@example.com",
            "req-uuid-1234",
            Duration::from_secs(3600),
            SECRET,
        )
        .expect("create token");

        // Set expiry in the past
        token.expires_at = 1; // Unix epoch + 1 second = well in the past

        // Re-sign with the past expiry so MAC matches
        let message = format!(
            "{}|{}|{}",
            token.signer_email, token.request_id, token.expires_at
        );
        let mut mac = HmacSha256::new_from_slice(SECRET).unwrap();
        mac.update(message.as_bytes());
        token.mac_bytes = mac.finalize().into_bytes().to_vec();

        let result = token.verify(SECRET);
        assert!(
            matches!(result, Err(ESignatureError::TokenExpired)),
            "Should return TokenExpired for past token"
        );
    }

    #[test]
    fn test_lightweight_provider_build_signing_url() {
        let config = LightweightConfig {
            token_secret: SECRET.to_vec(),
            base_url: "https://app.example.com".to_string(),
            webhook_url: None,
            token_ttl_secs: 3600,
        };
        let provider = LightweightProvider::new(config);
        let url = provider
            .build_signing_url("alice@example.com", "req-1")
            .expect("build url");

        assert!(url.starts_with("https://app.example.com/sign?token="));
    }

    #[test]
    fn test_lightweight_provider_verify_token_roundtrip() {
        let config = LightweightConfig {
            token_secret: SECRET.to_vec(),
            base_url: "https://app.example.com".to_string(),
            webhook_url: None,
            token_ttl_secs: 3600,
        };
        let provider = LightweightProvider::new(config);
        let url = provider
            .build_signing_url("alice@example.com", "req-1")
            .expect("build url");

        let token_str = url.split("token=").nth(1).unwrap();
        let verified = provider
            .verify_token(token_str)
            .expect("verify_token should succeed");

        assert_eq!(verified.signer_email, "alice@example.com");
        assert_eq!(verified.request_id, "req-1");
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
        let _env = EnvGuard::capture(&["ESIGN_TOKEN_SECRET", "RUST_ENV"]);
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
        let _env = EnvGuard::capture(&["ESIGN_TOKEN_SECRET", "RUST_ENV"]);
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
        let _env = EnvGuard::capture(&["ESIGN_TOKEN_SECRET", "RUST_ENV"]);
        std::env::set_var("RUST_ENV", "development");

        let cfg = LightweightConfig::from_env().expect("dev default should succeed");
        assert!(cfg.token_secret.len() >= MIN_TOKEN_SECRET_LEN);
    }

    #[test]
    fn from_env_accepts_valid_secret_in_prod() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _env = EnvGuard::capture(&["ESIGN_TOKEN_SECRET", "RUST_ENV"]);
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
