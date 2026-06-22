//! Cryptographic utilities for integration data encryption.
//!
//! Provides AES-256-GCM encryption for sensitive data like OAuth tokens
//! and webhook secrets. Uses INTEGRATION_ENCRYPTION_KEY environment variable.
//!
//! # Security
//! - AES-256-GCM provides authenticated encryption
//! - Unique nonce (96 bits) generated for each encryption
//! - Key must be exactly 32 bytes (256 bits)
//!
//! # Usage
//! ```ignore
//! use integrations::crypto::{encrypt, decrypt, IntegrationCrypto};
//!
//! let crypto = IntegrationCrypto::from_env()?;
//! let encrypted = crypto.encrypt("sensitive_token")?;
//! let decrypted = crypto.decrypt(&encrypted)?;
//! ```

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use thiserror::Error;

/// Encryption key length (256 bits = 32 bytes).
const KEY_LENGTH: usize = 32;

/// Nonce length (96 bits = 12 bytes).
const NONCE_LENGTH: usize = 12;

/// Prefix for encrypted values to distinguish from plaintext.
const ENCRYPTED_PREFIX: &str = "enc:";

/// Environment variable name for the encryption key.
pub const ENCRYPTION_KEY_ENV: &str = "INTEGRATION_ENCRYPTION_KEY";

/// Crypto errors.
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Encryption key not configured: {0}")]
    KeyNotConfigured(String),

    #[error("Invalid encryption key: must be {KEY_LENGTH} bytes (hex-encoded: {0} chars)")]
    InvalidKeyLength(usize),

    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Secure random number generation failed: {0}")]
    RngFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Invalid encrypted data format")]
    InvalidFormat,

    #[error("Base64 decode error: {0}")]
    Base64Error(String),

    #[error("Hex decode error: {0}")]
    HexError(String),
}

/// Integration crypto service for encrypting/decrypting sensitive data.
#[derive(Clone)]
pub struct IntegrationCrypto {
    cipher: Aes256Gcm,
}

impl IntegrationCrypto {
    /// Create a new IntegrationCrypto from a hex-encoded key.
    ///
    /// # Arguments
    /// * `hex_key` - 64-character hex string representing 32 bytes
    ///
    /// # Errors
    /// Returns error if key is invalid length or not valid hex.
    pub fn new(hex_key: &str) -> Result<Self, CryptoError> {
        let key_bytes = hex::decode(hex_key).map_err(|e| CryptoError::HexError(e.to_string()))?;

        if key_bytes.len() != KEY_LENGTH {
            return Err(CryptoError::InvalidKeyLength(hex_key.len()));
        }

        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        Ok(Self { cipher })
    }

    /// Create IntegrationCrypto from environment variable.
    ///
    /// Reads INTEGRATION_ENCRYPTION_KEY environment variable.
    pub fn from_env() -> Result<Self, CryptoError> {
        let key = std::env::var(ENCRYPTION_KEY_ENV).map_err(|_| {
            CryptoError::KeyNotConfigured(format!(
                "Environment variable {} is not set. Generate a 32-byte key with: \
                 openssl rand -hex 32",
                ENCRYPTION_KEY_ENV
            ))
        })?;

        Self::new(&key)
    }

    /// Try to create IntegrationCrypto from environment, returning None if not configured.
    ///
    /// Useful for development environments where encryption may be optional.
    pub fn try_from_env() -> Option<Self> {
        Self::from_env().ok()
    }

    /// Encrypt plaintext data.
    ///
    /// Returns a base64-encoded string containing nonce + ciphertext.
    pub fn encrypt(&self, plaintext: &str) -> Result<String, CryptoError> {
        // Generate random nonce directly from OS CSPRNG.
        //
        // SECURITY: AES-GCM nonce reuse catastrophically breaks confidentiality
        // (same nonce+key reveals plaintext XORs). Using OsRng directly avoids
        // any risk from a thread-local CSPRNG being seeded from a low-entropy
        // or predictable source at process start.
        use rand::TryRng;
        let mut nonce_bytes = [0u8; NONCE_LENGTH];
        // SECURITY: do NOT panic on a CSPRNG failure — a transient OS entropy
        // error must surface as a handled error so the caller can refuse to
        // persist the secret, never crash the process mid-encrypt.
        rand::rngs::SysRng
            .try_fill_bytes(&mut nonce_bytes)
            .map_err(|e| CryptoError::RngFailed(e.to_string()))?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

        // Prepend nonce to ciphertext and encode as base64
        let mut result = nonce_bytes.to_vec();
        result.extend(ciphertext);

        Ok(BASE64.encode(&result))
    }

    /// Decrypt base64-encoded ciphertext.
    ///
    /// Expects format: base64(nonce + ciphertext)
    pub fn decrypt(&self, encrypted: &str) -> Result<String, CryptoError> {
        // Decode base64
        let data = BASE64
            .decode(encrypted)
            .map_err(|e| CryptoError::Base64Error(e.to_string()))?;

        // Must have at least nonce + some ciphertext
        if data.len() < NONCE_LENGTH + 1 {
            return Err(CryptoError::InvalidFormat);
        }

        // Split nonce and ciphertext
        let (nonce_bytes, ciphertext) = data.split_at(NONCE_LENGTH);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt
        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::DecryptionFailed("Authentication failed".to_string()))?;

        String::from_utf8(plaintext)
            .map_err(|e| CryptoError::DecryptionFailed(format!("Invalid UTF-8: {}", e)))
    }

    /// Encrypt an optional string value.
    ///
    /// Returns None if input is None.
    pub fn encrypt_optional(&self, plaintext: Option<&str>) -> Result<Option<String>, CryptoError> {
        match plaintext {
            Some(text) => Ok(Some(self.encrypt(text)?)),
            None => Ok(None),
        }
    }

    /// Decrypt an optional encrypted value.
    ///
    /// Returns None if input is None.
    pub fn decrypt_optional(&self, encrypted: Option<&str>) -> Result<Option<String>, CryptoError> {
        match encrypted {
            Some(text) => Ok(Some(self.decrypt(text)?)),
            None => Ok(None),
        }
    }
}

/// Encrypt a value, failing closed if encryption is not available.
///
/// Unlike [`encrypt_if_available`], this NEVER returns plaintext. It is the
/// required path for persisting secrets (OAuth/access/refresh tokens, platform
/// credentials): if `INTEGRATION_ENCRYPTION_KEY` is unset (`crypto` is `None`)
/// or the encryption operation itself fails, an error is returned so the caller
/// can refuse to store the secret rather than leaking it in plaintext.
///
/// On success the value carries the same "enc:" prefix used by
/// [`encrypt_if_available`], so [`decrypt_if_available`] reads both transparently.
///
/// # Errors
/// - [`CryptoError::KeyNotConfigured`] when `crypto` is `None`.
/// - Propagates the underlying [`CryptoError`] when encryption fails.
pub fn encrypt_required(
    crypto: Option<&IntegrationCrypto>,
    plaintext: &str,
) -> Result<String, CryptoError> {
    match crypto {
        Some(c) => Ok(format!("{}{}", ENCRYPTED_PREFIX, c.encrypt(plaintext)?)),
        None => Err(CryptoError::KeyNotConfigured(format!(
            "{} must be set to store integration secrets; refusing to persist in plaintext",
            ENCRYPTION_KEY_ENV
        ))),
    }
}

/// Encrypt an optional value, failing closed if encryption is not available.
///
/// Returns `Ok(None)` when the input is `None`. When the input is `Some`, the
/// fail-closed semantics of [`encrypt_required`] apply.
pub fn encrypt_optional_required(
    crypto: Option<&IntegrationCrypto>,
    plaintext: Option<&str>,
) -> Result<Option<String>, CryptoError> {
    match plaintext {
        Some(text) => Ok(Some(encrypt_required(crypto, text)?)),
        None => Ok(None),
    }
}

/// Encrypt a value if crypto is available, otherwise return plaintext.
///
/// Adds "enc:" prefix to encrypted values to distinguish them from plaintext.
/// This allows graceful degradation in development environments and safe migration.
///
/// # Security
/// This helper deliberately degrades to plaintext when no key is configured and
/// MUST NOT be used for persisting secrets such as OAuth tokens or platform
/// credentials — use [`encrypt_required`] for those so the write fails closed.
pub fn encrypt_if_available(crypto: Option<&IntegrationCrypto>, plaintext: &str) -> String {
    match crypto {
        Some(c) => match c.encrypt(plaintext) {
            Ok(encrypted) => format!("{}{}", ENCRYPTED_PREFIX, encrypted),
            Err(e) => {
                tracing::warn!("Encryption failed, storing in plaintext: {}", e);
                plaintext.to_string()
            }
        },
        None => {
            tracing::debug!("Crypto not configured, storing in plaintext");
            plaintext.to_string()
        }
    }
}

/// Decrypt a value if crypto is available and value is encrypted.
///
/// Only attempts decryption if value has "enc:" prefix. Plaintext values are returned as-is.
/// This allows safe handling of mixed encrypted/plaintext data during migration.
///
/// # Security
/// On decryption failure, returns a placeholder string instead of the encrypted value
/// to prevent leaking encryption format details.
pub fn decrypt_if_available(crypto: Option<&IntegrationCrypto>, value: &str) -> String {
    // Check for encrypted prefix
    if let Some(encrypted_data) = value.strip_prefix(ENCRYPTED_PREFIX) {
        match crypto {
            Some(c) => c.decrypt(encrypted_data).unwrap_or_else(|e| {
                tracing::error!("Decryption failed: {}. Value may be corrupted.", e);
                // Return placeholder to avoid exposing encrypted data format
                "[DECRYPTION_FAILED]".to_string()
            }),
            None => {
                tracing::error!(
                    "Encrypted value found but crypto not configured. \
                     Set {} to decrypt.",
                    ENCRYPTION_KEY_ENV
                );
                // Return placeholder to avoid exposing encrypted data
                "[ENCRYPTION_KEY_REQUIRED]".to_string()
            }
        }
    } else {
        // No prefix - treat as plaintext (legacy data or crypto disabled)
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_crypto() -> IntegrationCrypto {
        // 32-byte key as 64 hex chars
        IntegrationCrypto::new("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
            .unwrap()
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let crypto = test_crypto();
        let plaintext = "my_secret_oauth_token_12345";

        let encrypted = crypto.encrypt(plaintext).unwrap();
        assert_ne!(encrypted, plaintext);

        let decrypted = crypto.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_different_nonces() {
        let crypto = test_crypto();
        let plaintext = "same_text";

        let encrypted1 = crypto.encrypt(plaintext).unwrap();
        let encrypted2 = crypto.encrypt(plaintext).unwrap();

        // Same plaintext should produce different ciphertext due to random nonce
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to same plaintext
        assert_eq!(crypto.decrypt(&encrypted1).unwrap(), plaintext);
        assert_eq!(crypto.decrypt(&encrypted2).unwrap(), plaintext);
    }

    #[test]
    fn test_empty_string() {
        let crypto = test_crypto();
        let plaintext = "";

        let encrypted = crypto.encrypt(plaintext).unwrap();
        let decrypted = crypto.decrypt(&encrypted).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_unicode_content() {
        let crypto = test_crypto();
        let plaintext = "Hello, World! Ahoj, Svet!";

        let encrypted = crypto.encrypt(plaintext).unwrap();
        let decrypted = crypto.decrypt(&encrypted).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_invalid_key_length() {
        // Use valid hex but wrong length (only 8 bytes instead of 32)
        let result = IntegrationCrypto::new("0102030405060708");
        assert!(matches!(result, Err(CryptoError::InvalidKeyLength(_))));
    }

    #[test]
    fn test_invalid_ciphertext() {
        let crypto = test_crypto();

        // Invalid base64
        assert!(crypto.decrypt("not-valid-base64!!!").is_err());

        // Valid base64 but too short
        assert!(crypto.decrypt(&BASE64.encode([1, 2, 3])).is_err());
    }

    #[test]
    fn test_tampered_ciphertext() {
        let crypto = test_crypto();
        let encrypted = crypto.encrypt("secret").unwrap();

        // Decode, tamper, re-encode
        let mut data = BASE64.decode(&encrypted).unwrap();
        data[NONCE_LENGTH + 5] ^= 0xFF; // Flip some bits in ciphertext
        let tampered = BASE64.encode(&data);

        // Decryption should fail authentication
        assert!(crypto.decrypt(&tampered).is_err());
    }

    #[test]
    fn test_encrypt_required_fails_closed_without_key() {
        // Issue #765: a missing encryption key must NOT silently fall back to
        // plaintext when persisting secrets.
        let result = encrypt_required(None, "super_secret_token");
        assert!(matches!(result, Err(CryptoError::KeyNotConfigured(_))));
    }

    #[test]
    fn test_encrypt_required_encrypts_with_key() {
        let crypto = test_crypto();
        let plaintext = "oauth_access_token";

        let encrypted = encrypt_required(Some(&crypto), plaintext).unwrap();
        // Never equal to plaintext, and carries the enc: prefix.
        assert_ne!(encrypted, plaintext);
        assert!(encrypted.starts_with(ENCRYPTED_PREFIX));

        // decrypt_if_available reads it back transparently.
        let decrypted = decrypt_if_available(Some(&crypto), &encrypted);
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_optional_required_semantics() {
        let crypto = test_crypto();

        // None stays None even without a key.
        assert!(encrypt_optional_required(None, None).unwrap().is_none());
        assert!(encrypt_optional_required(Some(&crypto), None)
            .unwrap()
            .is_none());

        // Some without a key fails closed.
        assert!(matches!(
            encrypt_optional_required(None, Some("refresh")),
            Err(CryptoError::KeyNotConfigured(_))
        ));

        // Some with a key encrypts.
        let encrypted = encrypt_optional_required(Some(&crypto), Some("refresh"))
            .unwrap()
            .unwrap();
        assert!(encrypted.starts_with(ENCRYPTED_PREFIX));
        assert_eq!(decrypt_if_available(Some(&crypto), &encrypted), "refresh");
    }

    #[test]
    fn test_rng_failed_error_renders() {
        // Regression: a CSPRNG failure during nonce generation must propagate as
        // a handled CryptoError::RngFailed rather than panicking the process.
        let err = CryptoError::RngFailed("entropy source unavailable".to_string());
        assert!(matches!(err, CryptoError::RngFailed(_)));
        let msg = err.to_string();
        assert!(msg.contains("Secure random number generation failed"));
        assert!(msg.contains("entropy source unavailable"));
    }

    #[test]
    fn test_optional_encrypt_decrypt() {
        let crypto = test_crypto();

        // None stays None
        assert!(crypto.encrypt_optional(None).unwrap().is_none());
        assert!(crypto.decrypt_optional(None).unwrap().is_none());

        // Some gets encrypted/decrypted
        let encrypted = crypto.encrypt_optional(Some("secret")).unwrap().unwrap();
        let decrypted = crypto.decrypt_optional(Some(&encrypted)).unwrap().unwrap();
        assert_eq!(decrypted, "secret");
    }
}
