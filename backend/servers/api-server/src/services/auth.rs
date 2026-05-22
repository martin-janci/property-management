//! Authentication service (Epic 1).

use argon2::{
    password_hash::{phc::PasswordHash, PasswordHasher, PasswordVerifier},
    Argon2,
};
use rand::{rngs::SysRng as RandSysRng, TryRng};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Authentication service errors.
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Password hashing failed")]
    HashingFailed,

    #[error("Password verification failed")]
    VerificationFailed,

    #[error("Invalid password")]
    InvalidPassword,

    #[error("Token generation failed")]
    TokenGenerationFailed,
}

/// Authentication service for password hashing and token generation.
#[derive(Clone)]
pub struct AuthService {
    argon2: Argon2<'static>,
}

impl Default for AuthService {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthService {
    /// Create a new AuthService.
    pub fn new() -> Self {
        Self {
            argon2: Argon2::default(),
        }
    }

    /// Hash a password using Argon2id (NFR11).
    pub fn hash_password(&self, password: &str) -> Result<String, AuthError> {
        // password-hash 0.6: `hash_password` generates a random salt internally.
        let password_hash = self
            .argon2
            .hash_password(password.as_bytes())
            .map_err(|_| AuthError::HashingFailed)?
            .to_string();
        Ok(password_hash)
    }

    /// Verify a password against a hash.
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool, AuthError> {
        let parsed_hash = PasswordHash::new(hash).map_err(|_| AuthError::VerificationFailed)?;
        Ok(self
            .argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    /// Generate a secure random token (for email verification, password reset).
    ///
    /// Uses `rand::rngs::OsRng` (OS-backed CSPRNG) rather than `thread_rng`,
    /// so token generation does not depend on the ChaCha20 thread-local state
    /// being correctly seeded — important for one-shot tokens that are used as
    /// a shared secret between email recipient and server.
    pub fn generate_token(&self) -> String {
        let mut rng = RandSysRng;
        let mut bytes = [0u8; 32];
        rng.try_fill_bytes(&mut bytes).expect("OS rng failed");
        hex::encode(bytes)
    }

    /// Hash a token for storage (we don't store plain tokens).
    pub fn hash_token(&self, token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Validate password requirements.
    /// - Minimum 8 characters
    /// - At least 1 uppercase letter
    /// - At least 1 number
    pub fn validate_password(password: &str) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if password.len() < 8 {
            errors.push("Password must be at least 8 characters".to_string());
        }

        if !password.chars().any(|c| c.is_uppercase()) {
            errors.push("Password must contain at least one uppercase letter".to_string());
        }

        if !password.chars().any(|c| c.is_numeric()) {
            errors.push("Password must contain at least one number".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate email format.
    pub fn validate_email(email: &str) -> bool {
        // Basic email validation
        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 {
            return false;
        }
        let domain_parts: Vec<&str> = parts[1].split('.').collect();
        if domain_parts.len() < 2 {
            return false;
        }
        !parts[0].is_empty() && domain_parts.iter().all(|p| !p.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let service = AuthService::new();
        let password = "TestPassword123";
        let hash = service.hash_password(password).unwrap();

        assert!(hash.starts_with("$argon2"));
        assert!(service.verify_password(password, &hash).unwrap());
        assert!(!service.verify_password("wrong", &hash).unwrap());
    }

    #[test]
    fn test_token_generation() {
        let service = AuthService::new();
        let token1 = service.generate_token();
        let token2 = service.generate_token();

        assert_eq!(token1.len(), 64); // 32 bytes = 64 hex chars
        assert_ne!(token1, token2);
    }

    #[test]
    fn test_password_validation() {
        assert!(AuthService::validate_password("ValidPass1").is_ok());
        assert!(AuthService::validate_password("short").is_err());
        assert!(AuthService::validate_password("nouppercase1").is_err());
        assert!(AuthService::validate_password("NoNumbers").is_err());
    }

    #[test]
    fn test_email_validation() {
        assert!(AuthService::validate_email("test@example.com"));
        assert!(AuthService::validate_email("user.name@domain.co.uk"));
        assert!(!AuthService::validate_email("invalid"));
        assert!(!AuthService::validate_email("no@domain"));
        assert!(!AuthService::validate_email("@nodomain.com"));
    }
}
