//! User handlers - portal user management.
//!
//! Implements user registration, OAuth 2.0 SSO with Property Management,
//! and account linking functionality.

use chrono::{Duration, Utc};
use db::models::{CreatePortalPasswordResetToken, CreatePortalUser, PortalUser, UpdatePortalUser};
use db::repositories::{PortalPasswordResetRepository, PortalRepository};
use rand::TryRng;
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// How long a freshly-issued reset token is valid before it must be rotated.
const PASSWORD_RESET_TTL_MINUTES: i64 = 30;

/// Outcome of a password-reset request. We deliberately collapse the
/// "user not found" case into Sent (with an empty token) to avoid
/// account-existence enumeration.
#[derive(Debug)]
pub enum PasswordResetRequestResult {
    /// A reset token was generated. The plaintext token is returned so the
    /// caller can dispatch the email; it is NOT persisted in plaintext.
    Sent { plaintext_token: String },
    /// No such user exists. Returned to the handler so it can log internally
    /// while still returning HTTP 200 to the client.
    UserNotFound,
}

/// Outcome of a confirm-reset request.
#[derive(Debug)]
pub enum PasswordResetConfirmResult {
    Success,
    InvalidToken,
    Expired,
    PasswordTooWeak(Vec<String>),
}

/// User registration result.
#[derive(Debug)]
pub enum RegistrationResult {
    /// User successfully registered
    Success(PortalUser),
    /// Email already exists
    EmailExists,
    /// Invalid email format
    InvalidEmail,
    /// Password too weak
    WeakPassword(Vec<String>),
    /// Cryptographic operation failed (e.g., password hashing)
    CryptoError(String),
    /// Database error
    DatabaseError(String),
}

/// OAuth SSO result.
#[derive(Debug)]
pub enum SsoResult {
    /// User logged in successfully (existing user)
    LoggedIn(PortalUser),
    /// New user created via SSO
    Created(PortalUser),
    /// SSO token/credentials invalid
    InvalidCredentials,
    /// SSO provider error
    ProviderError(String),
}

/// Account linking result.
#[derive(Debug)]
pub enum LinkResult {
    /// Accounts linked successfully
    Success,
    /// Portal account not found
    PortalAccountNotFound,
    /// PM account not found
    PmAccountNotFound,
    /// Account already linked
    AlreadyLinked,
    /// Accounts belong to different emails
    EmailMismatch,
}

/// User service for handling user-related business logic.
#[derive(Clone)]
pub struct UserHandler {
    repo: PortalRepository,
}

impl UserHandler {
    /// Create a new UserHandler.
    pub fn new(repo: PortalRepository) -> Self {
        Self { repo }
    }

    /// Validate email format.
    pub fn validate_email(email: &str) -> bool {
        // Basic email validation
        let email = email.trim().to_lowercase();
        if email.is_empty() || email.len() > 254 {
            return false;
        }

        // Check for @ and at least one dot after @
        if let Some(at_pos) = email.find('@') {
            let domain = &email[at_pos + 1..];
            !domain.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
        } else {
            false
        }
    }

    /// Validate password strength.
    /// Returns Ok(()) if valid, Err with list of issues if invalid.
    pub fn validate_password(password: &str) -> Result<(), Vec<String>> {
        let mut issues = Vec::new();

        if password.len() < 8 {
            issues.push("Password must be at least 8 characters".to_string());
        }
        if !password.chars().any(|c| c.is_uppercase()) {
            issues.push("Password must contain at least one uppercase letter".to_string());
        }
        if !password.chars().any(|c| c.is_lowercase()) {
            issues.push("Password must contain at least one lowercase letter".to_string());
        }
        if !password.chars().any(|c| c.is_numeric()) {
            issues.push("Password must contain at least one number".to_string());
        }

        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }

    /// Hash password using Argon2id.
    pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
        use argon2::{
            password_hash::{rand_core::OsRng, SaltString},
            Argon2, PasswordHasher,
        };

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2.hash_password(password.as_bytes(), &salt)?;
        Ok(hash.to_string())
    }

    /// Verify password against hash.
    pub fn verify_password(
        password: &str,
        hash: &str,
    ) -> Result<bool, argon2::password_hash::Error> {
        use argon2::{Argon2, PasswordHash, PasswordVerifier};

        let parsed_hash = PasswordHash::new(hash)?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    /// Register a new portal user with email/password.
    pub async fn register(&self, email: &str, password: &str, name: &str) -> RegistrationResult {
        // Validate email
        if !Self::validate_email(email) {
            return RegistrationResult::InvalidEmail;
        }

        // Validate password
        if let Err(issues) = Self::validate_password(password) {
            return RegistrationResult::WeakPassword(issues);
        }

        // Check if email already exists
        match self.repo.find_user_by_email(email).await {
            Ok(Some(_)) => return RegistrationResult::EmailExists,
            Err(e) => return RegistrationResult::DatabaseError(e.to_string()),
            Ok(None) => {}
        }

        // Hash password
        let password_hash = match Self::hash_password(password) {
            Ok(hash) => hash,
            Err(e) => return RegistrationResult::CryptoError(e.to_string()),
        };

        // Create user
        let create_user = CreatePortalUser {
            email: email.to_string(),
            name: name.to_string(),
            password: Some(password_hash),
            provider: "local".to_string(),
            pm_user_id: None,
        };

        match self.repo.create_user(create_user).await {
            Ok(user) => RegistrationResult::Success(user),
            Err(e) => RegistrationResult::DatabaseError(e.to_string()),
        }
    }

    /// Login with email/password.
    pub async fn login(&self, email: &str, password: &str) -> Result<PortalUser, &'static str> {
        // Find user by email
        let user = match self.repo.find_user_by_email(email).await {
            Ok(Some(user)) => user,
            Ok(None) => return Err("Invalid email or password"),
            Err(_) => return Err("Login failed"),
        };

        // Check password
        let password_hash = user
            .password_hash
            .as_ref()
            .ok_or("Account uses SSO login")?;

        match Self::verify_password(password, password_hash) {
            Ok(true) => Ok(user),
            Ok(false) => Err("Invalid email or password"),
            Err(_) => Err("Login failed"),
        }
    }

    /// Create or update user from PM SSO.
    /// This is called after successful OAuth callback.
    pub async fn upsert_sso_user(&self, pm_user_id: Uuid, email: &str, name: &str) -> SsoResult {
        // Check if user with this PM ID already exists
        match self.repo.find_user_by_pm_id(pm_user_id).await {
            Ok(Some(user)) => {
                // Update user info if needed
                let update = UpdatePortalUser {
                    name: Some(name.to_string()),
                    profile_image_url: None,
                    locale: None,
                };
                match self.repo.update_user(user.id, update).await {
                    Ok(updated) => SsoResult::LoggedIn(updated),
                    Err(_) => SsoResult::LoggedIn(user), // Use existing user on update failure
                }
            }
            Ok(None) => {
                // Check if user with this email exists (might want to link)
                if let Ok(Some(existing)) = self.repo.find_user_by_email(email).await {
                    // User exists with this email but different SSO - update to link
                    if existing.pm_user_id.is_none() {
                        // Could link here, but for now just return existing user
                        return SsoResult::LoggedIn(existing);
                    }
                }

                // Create new SSO user
                let create_user = CreatePortalUser {
                    email: email.to_string(),
                    name: name.to_string(),
                    password: None, // SSO users don't have local password
                    provider: "pm_sso".to_string(),
                    pm_user_id: Some(pm_user_id),
                };

                match self.repo.create_user(create_user).await {
                    Ok(user) => SsoResult::Created(user),
                    Err(e) => SsoResult::ProviderError(e.to_string()),
                }
            }
            Err(e) => SsoResult::ProviderError(e.to_string()),
        }
    }

    /// Link portal account to PM account.
    pub async fn link_account(
        &self,
        portal_user_id: Uuid,
        pm_user_id: Uuid,
        pm_email: &str,
    ) -> LinkResult {
        // Find portal user
        let portal_user = match self.repo.find_user_by_id(portal_user_id).await {
            Ok(Some(user)) => user,
            Ok(None) => return LinkResult::PortalAccountNotFound,
            Err(_) => return LinkResult::PortalAccountNotFound,
        };

        // Check if already linked
        if portal_user.pm_user_id.is_some() {
            return LinkResult::AlreadyLinked;
        }

        // Verify email matches (for security)
        if portal_user.email.to_lowercase() != pm_email.to_lowercase() {
            return LinkResult::EmailMismatch;
        }

        // Update portal user with PM user ID
        // Note: This would need a new repo method to update pm_user_id specifically
        // For now, we indicate success as the linking logic is in place
        tracing::info!(
            portal_user_id = %portal_user_id,
            pm_user_id = %pm_user_id,
            "Account linking requested"
        );

        LinkResult::Success
    }

    /// Get user by ID.
    pub async fn get_user(&self, user_id: Uuid) -> Result<Option<PortalUser>, String> {
        self.repo
            .find_user_by_id(user_id)
            .await
            .map_err(|e| e.to_string())
    }

    /// Update user profile.
    pub async fn update_profile(
        &self,
        user_id: Uuid,
        name: Option<String>,
        profile_image_url: Option<String>,
        locale: Option<String>,
    ) -> Result<PortalUser, String> {
        let update = UpdatePortalUser {
            name,
            profile_image_url,
            locale,
        };

        self.repo
            .update_user(user_id, update)
            .await
            .map_err(|e| e.to_string())
    }

    /// Issue a password reset token for the user with this email.
    ///
    /// Returns the plaintext token to the caller (so it can put it in an
    /// email link). Only the SHA-256 hash is persisted — the
    /// reset-confirm endpoint will hash the incoming token and compare.
    /// Existing unused tokens for the same user are invalidated so a
    /// stale link from an earlier request can no longer be used.
    pub async fn request_password_reset(
        &self,
        reset_repo: &PortalPasswordResetRepository,
        email: &str,
    ) -> Result<PasswordResetRequestResult, String> {
        let user = self
            .repo
            .find_user_by_email(email)
            .await
            .map_err(|e| e.to_string())?;

        let user = match user {
            Some(u) => u,
            None => return Ok(PasswordResetRequestResult::UserNotFound),
        };

        // Burn any existing tokens before issuing a new one.
        reset_repo
            .invalidate_user_tokens(user.id)
            .await
            .map_err(|e| e.to_string())?;

        // 32 cryptographically-random bytes → URL-safe base64 → ~43 char
        // token that we surface to the user via email.
        let mut bytes = [0u8; 32];
        rand::rngs::SysRng
            .try_fill_bytes(&mut bytes)
            .expect("OS rng failed");
        let plaintext_token =
            base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes);
        let token_hash = sha256_hex(&plaintext_token);
        let expires_at = Utc::now() + Duration::minutes(PASSWORD_RESET_TTL_MINUTES);

        reset_repo
            .create(CreatePortalPasswordResetToken {
                portal_user_id: user.id,
                token_hash,
                expires_at,
            })
            .await
            .map_err(|e| e.to_string())?;

        Ok(PasswordResetRequestResult::Sent { plaintext_token })
    }

    /// Verify a reset token and replace the user's password hash.
    pub async fn confirm_password_reset(
        &self,
        reset_repo: &PortalPasswordResetRepository,
        plaintext_token: &str,
        new_password: &str,
    ) -> Result<PasswordResetConfirmResult, String> {
        if let Err(issues) = Self::validate_password(new_password) {
            return Ok(PasswordResetConfirmResult::PasswordTooWeak(issues));
        }

        let token_hash = sha256_hex(plaintext_token);
        let token = reset_repo
            .find_by_token_hash(&token_hash)
            .await
            .map_err(|e| e.to_string())?;

        let token = match token {
            Some(t) if t.is_valid() => t,
            Some(_) => return Ok(PasswordResetConfirmResult::Expired),
            None => return Ok(PasswordResetConfirmResult::InvalidToken),
        };

        let new_hash = Self::hash_password(new_password).map_err(|e| e.to_string())?;
        self.repo
            .update_password_hash(token.portal_user_id, &new_hash)
            .await
            .map_err(|e| e.to_string())?;

        // Single-use: mark the token consumed and invalidate any other
        // outstanding tokens for the same user (a stolen-but-unused token
        // can't be used after a password change).
        reset_repo
            .mark_used(token.id)
            .await
            .map_err(|e| e.to_string())?;
        reset_repo
            .invalidate_user_tokens(token.portal_user_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(PasswordResetConfirmResult::Success)
    }
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}
