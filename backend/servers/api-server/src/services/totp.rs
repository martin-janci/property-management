//! TOTP (Time-based One-Time Password) service for 2FA (Epic 9, Story 9.1).

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{
    password_hash::{phc::PasswordHash, PasswordHasher, PasswordVerifier},
    Argon2,
};
use rand::RngExt;
use thiserror::Error;
use totp_rs::{Algorithm, Secret, TOTP};

/// Errors that can occur during TOTP operations.
#[derive(Debug, Error)]
pub enum TotpError {
    #[error("Failed to generate secret: {0}")]
    SecretGenerationError(String),
    #[error("Failed to create TOTP instance: {0}")]
    TotpCreationError(String),
    #[error("Invalid TOTP code")]
    InvalidCode,
    #[error("Failed to hash backup code: {0}")]
    HashError(String),
    #[error("Failed to verify backup code: {0}")]
    VerifyError(String),
    #[error("Encryption error: {0}")]
    EncryptionError(String),
    #[error("Decryption error: {0}")]
    DecryptionError(String),
    #[error("Missing encryption key: TOTP_ENCRYPTION_KEY must be set")]
    MissingEncryptionKey,
}

/// Service for TOTP operations.
#[derive(Clone)]
pub struct TotpService {
    /// Issuer name for the TOTP (shows in authenticator app)
    issuer: String,
    /// Number of backup codes to generate
    backup_code_count: usize,
    /// Length of each backup code
    backup_code_length: usize,
    /// Encryption key for TOTP secrets (32 bytes for AES-256)
    encryption_key: Option<[u8; 32]>,
}

impl TotpService {
    /// Create a new TOTP service.
    ///
    /// # Security
    /// - Production (RUST_ENV != "development"): TOTP_ENCRYPTION_KEY is REQUIRED
    /// - Development: Falls back to a dev key with warning
    ///
    /// The encryption key must be 64 hex characters (32 bytes for AES-256).
    pub fn new(issuer: String) -> Self {
        let is_development = std::env::var("RUST_ENV").unwrap_or_default() == "development";

        let encryption_key = match std::env::var("TOTP_ENCRYPTION_KEY") {
            Ok(key) if key.len() == 64 => {
                let mut bytes = [0u8; 32];
                match hex::decode_to_slice(&key, &mut bytes) {
                    Ok(()) => {
                        tracing::info!("TOTP encryption enabled with configured key");
                        Some(bytes)
                    }
                    Err(e) => {
                        if is_development {
                            tracing::warn!(
                                "TOTP_ENCRYPTION_KEY has invalid hex, using dev key: {}",
                                e
                            );
                            Some(Self::insecure_dev_fallback_key())
                        } else {
                            panic!(
                                "TOTP_ENCRYPTION_KEY has invalid hex format: {}. \
                                Key must be 64 hex characters (32 bytes). \
                                Set RUST_ENV=development to use dev defaults.",
                                e
                            );
                        }
                    }
                }
            }
            Ok(key) => {
                if is_development {
                    tracing::warn!(
                        "TOTP_ENCRYPTION_KEY is {} chars (expected 64), using dev key",
                        key.len()
                    );
                    Some(Self::insecure_dev_fallback_key())
                } else {
                    panic!(
                        "TOTP_ENCRYPTION_KEY must be exactly 64 hex characters (got {}). \
                        Set RUST_ENV=development to use dev defaults.",
                        key.len()
                    );
                }
            }
            Err(_) => {
                if is_development {
                    tracing::warn!(
                        "TOTP_ENCRYPTION_KEY not set, using development key \
                        (DEVELOPMENT MODE ONLY - MFA secrets will not be secure)"
                    );
                    Some(Self::insecure_dev_fallback_key())
                } else {
                    panic!(
                        "TOTP_ENCRYPTION_KEY environment variable is required for production. \
                        Generate a 64-character hex key (32 bytes) for AES-256 encryption. \
                        Set RUST_ENV=development to use dev defaults."
                    );
                }
            }
        };

        Self {
            issuer,
            backup_code_count: 10,
            backup_code_length: 8,
            encryption_key,
        }
    }

    /// INSECURE development-only fallback encryption key.
    /// WARNING: This is a hardcoded key and should NEVER be used in production.
    /// It exists only for local development when TOTP_ENCRYPTION_KEY is not set.
    fn insecure_dev_fallback_key() -> [u8; 32] {
        // Fixed dev key - predictable for development/testing
        [
            0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xba, 0xbe, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55,
            0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x01, 0x02, 0x03, 0x04,
            0x05, 0x06, 0x07, 0x08,
        ]
    }

    /// Encrypt a TOTP secret for storage.
    /// Returns hex-encoded nonce:ciphertext.
    pub fn encrypt_secret(&self, plaintext: &str) -> Result<String, TotpError> {
        let key = self.encryption_key.ok_or(TotpError::MissingEncryptionKey)?;

        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| TotpError::EncryptionError(e.to_string()))?;

        // Generate random 12-byte nonce from OS CSPRNG directly.
        // SECURITY: AES-GCM nonce reuse breaks confidentiality; see crypto.rs
        // for rationale.
        use rand::TryRng;
        let mut nonce_bytes = [0u8; 12];
        rand::rngs::SysRng
            .try_fill_bytes(&mut nonce_bytes)
            .expect("OS rng failed");
        let nonce = Nonce::from(nonce_bytes);

        // Encrypt the secret
        let ciphertext = cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| TotpError::EncryptionError(e.to_string()))?;

        // Return as hex: nonce:ciphertext
        Ok(format!(
            "{}:{}",
            hex::encode(nonce_bytes),
            hex::encode(ciphertext)
        ))
    }

    /// Decrypt an encrypted TOTP secret.
    /// Expects hex-encoded nonce:ciphertext format.
    pub fn decrypt_secret(&self, encrypted: &str) -> Result<String, TotpError> {
        let key = self.encryption_key.ok_or(TotpError::MissingEncryptionKey)?;

        // Reject any value that does not match the nonce:ciphertext format.
        // Previously this branch silently returned the value verbatim as a
        // "legacy unencrypted secret" — that path is a privilege-escalation
        // surface (anyone who can write to two_factor_auth.secret controls
        // the OTP seed). Migration of legacy rows must run through an
        // explicit, opt-in flag, not through silent acceptance.
        if !encrypted.contains(':') {
            return Err(TotpError::DecryptionError(
                "TOTP secret is not in encrypted nonce:ciphertext format; legacy plaintext secrets are rejected".to_string(),
            ));
        }

        let parts: Vec<&str> = encrypted.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(TotpError::DecryptionError(
                "Invalid encrypted format".to_string(),
            ));
        }

        let nonce_bytes =
            hex::decode(parts[0]).map_err(|e| TotpError::DecryptionError(e.to_string()))?;
        let ciphertext =
            hex::decode(parts[1]).map_err(|e| TotpError::DecryptionError(e.to_string()))?;

        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| TotpError::DecryptionError(e.to_string()))?;

        let nonce = Nonce::try_from(nonce_bytes.as_slice())
            .map_err(|_| TotpError::DecryptionError("Invalid nonce length".to_string()))?;

        let plaintext = cipher
            .decrypt(&nonce, ciphertext.as_ref())
            .map_err(|e| TotpError::DecryptionError(e.to_string()))?;

        String::from_utf8(plaintext).map_err(|e| TotpError::DecryptionError(e.to_string()))
    }

    /// Check if encryption is enabled.
    pub fn is_encryption_enabled(&self) -> bool {
        self.encryption_key.is_some()
    }

    /// Generate a new TOTP secret.
    /// Returns a base32-encoded secret suitable for authenticator apps.
    pub fn generate_secret(&self) -> Result<String, TotpError> {
        let secret = Secret::generate_secret();
        Ok(secret.to_encoded().to_string())
    }

    /// Verify a TOTP code against a secret.
    /// Uses a 30-second window and allows for 1 step skew (±30 seconds).
    pub fn verify_code(&self, secret: &str, code: &str) -> Result<bool, TotpError> {
        let secret_bytes = Secret::Encoded(secret.to_string())
            .to_bytes()
            .map_err(|e| TotpError::TotpCreationError(e.to_string()))?;

        let totp = TOTP::new(
            Algorithm::SHA1,
            6,  // digits
            1,  // skew (±1 step = ±30 seconds)
            30, // step in seconds
            secret_bytes,
            Some(self.issuer.clone()), // issuer
            "".to_string(),            // account_name (not used for verification)
        )
        .map_err(|e| TotpError::TotpCreationError(e.to_string()))?;

        Ok(totp.check_current(code).unwrap_or(false))
    }

    /// Generate a URI for QR code display.
    /// This URI can be encoded into a QR code for authenticator apps to scan.
    pub fn generate_qr_uri(&self, email: &str, secret: &str) -> Result<String, TotpError> {
        let secret_bytes = Secret::Encoded(secret.to_string())
            .to_bytes()
            .map_err(|e| TotpError::TotpCreationError(e.to_string()))?;

        let totp = TOTP::new(
            Algorithm::SHA1,
            6,  // digits
            1,  // skew
            30, // step
            secret_bytes,
            Some(self.issuer.clone()), // issuer
            email.to_string(),         // account_name
        )
        .map_err(|e| TotpError::TotpCreationError(e.to_string()))?;

        // Generate the otpauth:// URI
        Ok(totp.get_url())
    }

    /// Generate a base64-encoded PNG QR code for the otpauth URI.
    ///
    /// Returns the QR rendered server-side so the otpauth URI (which embeds
    /// the TOTP secret) never leaves the trust boundary — defends against
    /// the QR-via-Google-Charts leak (Phase 6 review R1). The frontend
    /// renders the result directly via `<img src="data:image/png;base64,...">`.
    pub fn generate_qr_base64(&self, email: &str, secret: &str) -> Result<String, TotpError> {
        let secret_bytes = Secret::Encoded(secret.to_string())
            .to_bytes()
            .map_err(|e| TotpError::TotpCreationError(e.to_string()))?;

        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            secret_bytes,
            Some(self.issuer.clone()),
            email.to_string(),
        )
        .map_err(|e| TotpError::TotpCreationError(e.to_string()))?;

        totp.get_qr_base64()
            .map_err(|e| TotpError::TotpCreationError(e.to_string()))
    }

    /// Generate backup codes.
    /// Returns a tuple of (plain codes for display, hashed codes for storage).
    pub fn generate_backup_codes(&self) -> Result<(Vec<String>, Vec<String>), TotpError> {
        let mut plain_codes = Vec::with_capacity(self.backup_code_count);
        let mut hashed_codes = Vec::with_capacity(self.backup_code_count);

        for _ in 0..self.backup_code_count {
            let code = self.generate_random_code();
            let hashed = self.hash_backup_code(&code)?;
            plain_codes.push(code);
            hashed_codes.push(hashed);
        }

        Ok((plain_codes, hashed_codes))
    }

    /// Verify a backup code against a list of hashed codes.
    /// Returns the index of the matching code if found.
    pub fn verify_backup_code(
        &self,
        code: &str,
        hashed_codes: &[String],
    ) -> Result<Option<usize>, TotpError> {
        // Normalize the code (remove any dashes/spaces, uppercase)
        let normalized_code = code.replace(['-', ' '], "").to_uppercase();

        for (index, hashed) in hashed_codes.iter().enumerate() {
            // Skip empty/used codes
            if hashed.is_empty() {
                continue;
            }

            if self.verify_backup_code_hash(&normalized_code, hashed)? {
                return Ok(Some(index));
            }
        }

        Ok(None)
    }

    /// Generate a random backup code.
    ///
    /// Uses OS CSPRNG directly; backup codes are shared secrets used to
    /// recover a TOTP-protected account.
    fn generate_random_code(&self) -> String {
        const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
        let mut rng = rand::rand_core::UnwrapErr(rand::rngs::SysRng);

        (0..self.backup_code_length)
            .map(|_| {
                let idx = rng.random_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// Hash a backup code using Argon2.
    /// Normalizes the code before hashing to match verification behavior.
    fn hash_backup_code(&self, code: &str) -> Result<String, TotpError> {
        // Normalize the code (remove any dashes/spaces, uppercase) to match verification behavior
        let normalized_code = code.replace(['-', ' '], "").to_uppercase();

        let argon2 = Argon2::default();
        // password-hash 0.6: `hash_password` generates a random salt internally.
        let hash = argon2
            .hash_password(normalized_code.as_bytes())
            .map_err(|e| TotpError::HashError(e.to_string()))?;
        Ok(hash.to_string())
    }

    /// Verify a backup code against its hash.
    fn verify_backup_code_hash(&self, code: &str, hash: &str) -> Result<bool, TotpError> {
        let parsed_hash =
            PasswordHash::new(hash).map_err(|e| TotpError::VerifyError(e.to_string()))?;
        Ok(Argon2::default()
            .verify_password(code.as_bytes(), &parsed_hash)
            .is_ok())
    }
}

impl Default for TotpService {
    fn default() -> Self {
        Self::new("Property Management".to_string())
    }
}

/// Create a new TotpService with a specific encryption key (for testing).
#[cfg(test)]
impl TotpService {
    pub fn with_encryption_key(issuer: String, key: [u8; 32]) -> Self {
        Self {
            issuer,
            backup_code_count: 10,
            backup_code_length: 8,
            encryption_key: Some(key),
        }
    }

    /// Create a test instance with default settings and test encryption key.
    /// Use this in tests instead of `default()` to avoid environment variable requirements.
    pub fn test_default() -> Self {
        Self::with_encryption_key(
            "Property Management".to_string(),
            Self::insecure_dev_fallback_key(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_secret() {
        let service = TotpService::test_default();
        let secret = service.generate_secret().unwrap();
        assert!(!secret.is_empty());
        // Base32 encoded secrets should only contain valid chars
        assert!(secret
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '='));
    }

    #[test]
    fn test_generate_qr_uri() {
        let service = TotpService::test_default();
        let secret = service.generate_secret().unwrap();
        let uri = service
            .generate_qr_uri("test@example.com", &secret)
            .unwrap();

        assert!(uri.starts_with("otpauth://totp/"));
        assert!(uri.contains("test@example.com") || uri.contains("test%40example.com"));
        assert!(uri.contains("issuer=Property"));
    }

    #[test]
    fn test_generate_backup_codes() {
        let service = TotpService::test_default();
        let (plain, hashed) = service.generate_backup_codes().unwrap();

        assert_eq!(plain.len(), 10);
        assert_eq!(hashed.len(), 10);

        // Each plain code should be 8 characters
        for code in &plain {
            assert_eq!(code.len(), 8);
        }

        // Hashed codes should be Argon2 format
        for hash in &hashed {
            assert!(hash.starts_with("$argon2"));
        }
    }

    #[test]
    fn test_verify_backup_code() {
        let service = TotpService::test_default();
        let (plain, hashed) = service.generate_backup_codes().unwrap();

        // Should find the first code
        let result = service.verify_backup_code(&plain[0], &hashed).unwrap();
        assert_eq!(result, Some(0));

        // Should find the last code
        let result = service.verify_backup_code(&plain[9], &hashed).unwrap();
        assert_eq!(result, Some(9));

        // Invalid code should return None
        let result = service.verify_backup_code("INVALID1", &hashed).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_verify_backup_code_with_dashes() {
        let service = TotpService::test_default();
        let (plain, hashed) = service.generate_backup_codes().unwrap();

        // Should work with dashes
        let code_with_dashes = format!("{}-{}", &plain[0][..4], &plain[0][4..]);
        let result = service
            .verify_backup_code(&code_with_dashes, &hashed)
            .unwrap();
        assert_eq!(result, Some(0));
    }

    #[test]
    fn test_encrypt_decrypt_secret() {
        // Create service with a test encryption key
        let key: [u8; 32] = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x1f,
        ];
        let service = TotpService::with_encryption_key("Test".to_string(), key);

        let secret = service.generate_secret().unwrap();

        // Encrypt
        let encrypted = service.encrypt_secret(&secret).unwrap();
        assert!(
            encrypted.contains(':'),
            "Encrypted format should be nonce:ciphertext"
        );
        assert_ne!(encrypted, secret, "Encrypted should differ from original");

        // Decrypt
        let decrypted = service.decrypt_secret(&encrypted).unwrap();
        assert_eq!(decrypted, secret, "Decrypted should match original");
    }

    #[test]
    fn test_decrypt_legacy_unencrypted_is_rejected() {
        // P0-14: legacy plaintext secrets used to be returned as-is — that
        // was a privilege-escalation surface (anyone who could write to
        // two_factor_auth.secret controlled the OTP seed). The new contract
        // is fail-closed: any value not in nonce:ciphertext format is a
        // hard error.
        let key: [u8; 32] = [0xaa; 32];
        let service = TotpService::with_encryption_key("Test".to_string(), key);

        let legacy_secret = "JBSWY3DPEHPK3PXP";
        let result = service.decrypt_secret(legacy_secret);
        assert!(
            matches!(result, Err(TotpError::DecryptionError(_))),
            "legacy plaintext TOTP secret must be rejected, got {:?}",
            result
        );
    }

    #[test]
    fn test_encryption_in_development_mode() {
        // Using test_default() to get a service with the development fallback key.
        // This test verifies that the dev mode fallback key works correctly for
        // encryption and decryption.
        let service = TotpService::test_default();

        // Encryption should succeed with the fallback key
        let result = service.encrypt_secret("test-secret");
        assert!(
            result.is_ok(),
            "Encryption should succeed with test default key"
        );

        // Verify we can decrypt what we encrypted
        let encrypted = result.unwrap();
        let decrypted = service.decrypt_secret(&encrypted);
        assert!(decrypted.is_ok(), "Decryption should succeed");
        assert_eq!(decrypted.unwrap(), "test-secret");
    }
}
