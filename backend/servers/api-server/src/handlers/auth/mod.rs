//! Authentication handlers (UC-14).
//!
//! Implements user authentication, registration, password management.
//! This module provides the core business logic for authentication operations,
//! delegating to services and repositories for password hashing, JWT token
//! generation, refresh token handling, and 2FA verification.

use crate::services::AuthService;
use crate::state::AppState;
use chrono::{DateTime, Duration, Utc};
use common::errors::{ErrorResponse, ValidationError};
use db::models::{
    AuditAction, CreateAuditLog, CreatePasswordResetToken, CreateRefreshToken, CreateUser, Locale,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

// ============================================================================
// Error Types
// ============================================================================

/// Authentication handler errors.
#[derive(Debug, Error)]
pub enum AuthHandlerError {
    #[error("Invalid email format")]
    InvalidEmail,

    #[error("Password validation failed: {0:?}")]
    InvalidPassword(Vec<String>),

    #[error("Name cannot be empty")]
    EmptyName,

    #[error("Email already exists")]
    EmailExists,

    #[error("User not found")]
    UserNotFound,

    #[error("Email not verified")]
    EmailNotVerified,

    #[error("Account suspended")]
    AccountSuspended,

    #[error("Account inactive")]
    AccountInactive,

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Invalid or expired token")]
    InvalidToken,

    #[error("Token expired")]
    TokenExpired,

    #[error("Token revoked")]
    TokenRevoked,

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("MFA required")]
    MfaRequired,

    #[error("Invalid MFA code")]
    InvalidMfaCode,

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Database error: {0}")]
    Database(String),
}

impl From<AuthHandlerError> for ErrorResponse {
    fn from(err: AuthHandlerError) -> Self {
        match err {
            AuthHandlerError::InvalidEmail => {
                ErrorResponse::new("INVALID_EMAIL", "Invalid email format")
            }
            AuthHandlerError::InvalidPassword(errors) => {
                let details: Vec<ValidationError> = errors
                    .into_iter()
                    .map(|msg| ValidationError {
                        field: "password".to_string(),
                        message: msg.clone(),
                        code: "INVALID_PASSWORD".to_string(),
                    })
                    .collect();
                ErrorResponse::new("VALIDATION_ERROR", "Password does not meet requirements")
                    .with_details(details)
            }
            AuthHandlerError::EmptyName => {
                ErrorResponse::new("INVALID_NAME", "Name cannot be empty")
            }
            AuthHandlerError::EmailExists => {
                ErrorResponse::new("EMAIL_EXISTS", "An account with this email already exists")
            }
            AuthHandlerError::UserNotFound => {
                ErrorResponse::new("USER_NOT_FOUND", "User account not found")
            }
            AuthHandlerError::EmailNotVerified => {
                ErrorResponse::new("EMAIL_NOT_VERIFIED", "Please verify your email first")
            }
            AuthHandlerError::AccountSuspended => {
                ErrorResponse::new("ACCOUNT_SUSPENDED", "Account suspended. Contact support.")
            }
            AuthHandlerError::AccountInactive => {
                ErrorResponse::new("ACCOUNT_INACTIVE", "Account is no longer active")
            }
            AuthHandlerError::InvalidCredentials => {
                ErrorResponse::new("INVALID_CREDENTIALS", "Invalid email or password")
            }
            AuthHandlerError::InvalidToken => {
                ErrorResponse::new("INVALID_TOKEN", "Invalid or expired token")
            }
            AuthHandlerError::TokenExpired => {
                ErrorResponse::new("TOKEN_EXPIRED", "Token has expired")
            }
            AuthHandlerError::TokenRevoked => {
                ErrorResponse::new("TOKEN_REVOKED", "This session has been revoked")
            }
            AuthHandlerError::RateLimited(msg) => ErrorResponse::new("RATE_LIMITED", msg),
            AuthHandlerError::MfaRequired => {
                ErrorResponse::new("MFA_REQUIRED", "Two-factor authentication required")
            }
            AuthHandlerError::InvalidMfaCode => {
                ErrorResponse::new("INVALID_MFA_CODE", "Invalid verification code")
            }
            AuthHandlerError::Internal(msg) => ErrorResponse::new("INTERNAL_ERROR", msg),
            AuthHandlerError::Database(msg) => ErrorResponse::new("DATABASE_ERROR", msg),
        }
    }
}

// ============================================================================
// Request/Response Types
// ============================================================================

/// Registration request data.
#[derive(Debug, Clone)]
pub struct RegisterData {
    pub email: String,
    pub password: String,
    pub name: String,
    pub phone: Option<String>,
    pub locale: Option<String>,
}

/// Registration result.
#[derive(Debug, Serialize)]
pub struct RegisterResult {
    pub user_id: Uuid,
    pub message: String,
}

/// Login request data.
#[derive(Debug, Clone)]
pub struct LoginData {
    pub email: String,
    pub password: String,
    pub two_factor_code: Option<String>,
    pub ip_address: String,
    pub user_agent: Option<String>,
}

/// Login result.
#[derive(Debug, Serialize)]
pub struct LoginResult {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub token_type: String,
    pub mfa_required: Option<bool>,
}

/// Token refresh request data.
#[derive(Debug, Clone)]
pub struct RefreshData {
    pub refresh_token: String,
}

/// Password reset request data.
#[derive(Debug, Clone)]
pub struct ForgotPasswordData {
    pub email: String,
}

/// Reset password request data.
#[derive(Debug, Clone)]
pub struct ResetPasswordData {
    pub token: String,
    pub new_password: String,
}

/// Session information.
#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub id: Uuid,
    pub device_info: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
    pub is_current: bool,
}

// ============================================================================
// Handler Implementation
// ============================================================================

/// Authentication handler providing business logic for auth operations.
pub struct AuthHandler;

impl AuthHandler {
    /// Register a new user.
    ///
    /// Validates input, creates user with hashed password, generates verification
    /// token, and sends verification email.
    pub async fn register(
        state: &AppState,
        data: RegisterData,
    ) -> Result<RegisterResult, AuthHandlerError> {
        // Validate email format
        if !AuthService::validate_email(&data.email) {
            return Err(AuthHandlerError::InvalidEmail);
        }

        // Validate password requirements
        if let Err(errors) = AuthService::validate_password(&data.password) {
            return Err(AuthHandlerError::InvalidPassword(errors));
        }

        // Validate name is not empty
        if data.name.trim().is_empty() {
            return Err(AuthHandlerError::EmptyName);
        }

        // Check if email already exists
        match state.user_repo.email_exists(&data.email).await {
            Ok(true) => return Err(AuthHandlerError::EmailExists),
            Ok(false) => {}
            Err(e) => {
                tracing::error!(error = %e, "Database error checking email");
                return Err(AuthHandlerError::Database("Failed to check email".into()));
            }
        }

        // Hash password
        let password_hash = state
            .auth_service
            .hash_password(&data.password)
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to hash password");
                AuthHandlerError::Internal("Failed to process password".into())
            })?;

        // Determine locale from request or default to English
        let locale = data
            .locale
            .as_ref()
            .map(|l| Locale::parse(l))
            .unwrap_or(Locale::English);

        // Create user
        let create_user = CreateUser {
            email: data.email.clone(),
            password_hash,
            name: data.name.clone(),
            phone: data.phone.clone(),
            locale: locale.clone(),
        };

        let user = state.user_repo.create(create_user).await.map_err(|e| {
            tracing::error!(error = %e, email = %data.email, "Failed to create user");
            AuthHandlerError::Database("Failed to create account".into())
        })?;

        // Generate verification token
        let token = state.auth_service.generate_token();
        let token_hash = state.auth_service.hash_token(&token);

        // Store verification token
        if let Err(e) = state
            .user_repo
            .create_verification_token(user.id, &token_hash)
            .await
        {
            tracing::error!(error = %e, user_id = %user.id, "Failed to create verification token");
            // Continue anyway - user can request resend
        }

        // Send verification email
        if let Err(e) = state
            .email_service
            .send_verification_email(&user.email, &user.name, &token, &locale)
            .await
        {
            tracing::error!(error = %e, user_id = %user.id, "Failed to send verification email");
            // Continue anyway - user can request resend
        }

        tracing::info!(
            user_id = %user.id,
            email = %user.email,
            "User registered successfully"
        );

        Ok(RegisterResult {
            user_id: user.id,
            message: "Check your email to verify your account".to_string(),
        })
    }

    /// Verify email with token.
    pub async fn verify_email(state: &AppState, token: &str) -> Result<String, AuthHandlerError> {
        // Hash the token to look it up
        let token_hash = state.auth_service.hash_token(token);

        // Find the token
        let verification_token = state
            .user_repo
            .find_verification_token(&token_hash)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Database error finding verification token");
                AuthHandlerError::Database("Failed to verify token".into())
            })?
            .ok_or(AuthHandlerError::InvalidToken)?;

        // Check if token is expired
        if verification_token.is_expired() {
            return Err(AuthHandlerError::TokenExpired);
        }

        // Mark token as used
        if let Err(e) = state
            .user_repo
            .use_verification_token(verification_token.id)
            .await
        {
            tracing::error!(error = %e, "Failed to mark verification token as used");
        }

        // Verify the user's email
        match state
            .user_repo
            .verify_email(verification_token.user_id)
            .await
        {
            Ok(Some(user)) => {
                tracing::info!(user_id = %user.id, email = %user.email, "Email verified");
                Ok("Your email has been verified. You can now log in.".to_string())
            }
            Ok(None) => Err(AuthHandlerError::UserNotFound),
            Err(e) => {
                tracing::error!(error = %e, "Failed to verify email");
                Err(AuthHandlerError::Database("Failed to verify email".into()))
            }
        }
    }

    /// Resend verification email.
    pub async fn resend_verification(
        state: &AppState,
        email: &str,
    ) -> Result<(), AuthHandlerError> {
        // Try to find user and send email
        match state.user_repo.find_by_email(email).await {
            Ok(Some(user)) if user.status == "pending" => {
                // Invalidate old tokens
                let _ = state.user_repo.invalidate_user_tokens(user.id).await;

                // Generate new token
                let token = state.auth_service.generate_token();
                let token_hash = state.auth_service.hash_token(&token);

                // Store new token
                if let Err(e) = state
                    .user_repo
                    .create_verification_token(user.id, &token_hash)
                    .await
                {
                    tracing::error!(error = %e, user_id = %user.id, "Failed to create verification token");
                    return Ok(()); // Silent failure to prevent enumeration
                }

                // Send email
                if let Err(e) = state
                    .email_service
                    .send_verification_email(&user.email, &user.name, &token, &user.locale_enum())
                    .await
                {
                    tracing::error!(error = %e, user_id = %user.id, "Failed to send verification email");
                }

                tracing::info!(user_id = %user.id, "Resent verification email");
            }
            _ => {
                // User not found or already verified - don't reveal this
                tracing::debug!(email = %email, "Resend verification request for non-pending account");
            }
        }

        Ok(())
    }

    /// Authenticate user and generate tokens.
    pub async fn login(state: &AppState, data: LoginData) -> Result<LoginResult, AuthHandlerError> {
        // Check rate limiting
        match state.session_repo.check_rate_limit(&data.email).await {
            Ok(status) if !status.can_attempt() => {
                let remaining = status.lockout_remaining_secs.unwrap_or(900);
                tracing::warn!(
                    email = %data.email,
                    ip = %data.ip_address,
                    remaining_secs = remaining,
                    "Login attempt blocked due to rate limiting"
                );
                return Err(AuthHandlerError::RateLimited(format!(
                    "Too many failed login attempts. Please try again in {} minutes.",
                    remaining / 60 + 1
                )));
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to check rate limit");
                // Continue anyway - don't block login due to rate limit check failure
            }
            _ => {}
        }

        // Find user by email
        let user = match state.user_repo.find_by_email(&data.email).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                // Record failed attempt (user not found)
                if let Err(err) = state
                    .session_repo
                    .record_login_attempt(&data.email, &data.ip_address, false)
                    .await
                {
                    tracing::warn!(
                        error = %err,
                        "record_login_attempt failed (rate limiter may be unprotected for this attempt)"
                    );
                }
                tracing::debug!(email = %data.email, "Login failed: user not found");
                return Err(AuthHandlerError::InvalidCredentials);
            }
            Err(e) => {
                tracing::error!(error = %e, "Database error finding user");
                return Err(AuthHandlerError::Database("Login failed".into()));
            }
        };

        // Check if user can log in
        if user.status == "suspended" {
            if let Err(err) = state
                .session_repo
                .record_login_attempt(&data.email, &data.ip_address, false)
                .await
            {
                tracing::warn!(
                    error = %err,
                    "record_login_attempt failed (rate limiter may be unprotected for this attempt)"
                );
            }
            return Err(AuthHandlerError::AccountSuspended);
        }

        if !user.is_verified() {
            if let Err(err) = state
                .session_repo
                .record_login_attempt(&data.email, &data.ip_address, false)
                .await
            {
                tracing::warn!(
                    error = %err,
                    "record_login_attempt failed (rate limiter may be unprotected for this attempt)"
                );
            }
            return Err(AuthHandlerError::EmailNotVerified);
        }

        // Verify password
        let password_valid = state
            .auth_service
            .verify_password(&data.password, &user.password_hash)
            .map_err(|e| {
                tracing::error!(error = %e, "Password verification error");
                AuthHandlerError::Internal("Login failed".into())
            })?;

        if !password_valid {
            if let Err(err) = state
                .session_repo
                .record_login_attempt(&data.email, &data.ip_address, false)
                .await
            {
                tracing::warn!(
                    error = %err,
                    "record_login_attempt failed (rate limiter may be unprotected for this attempt)"
                );
            }
            tracing::debug!(email = %data.email, "Login failed: invalid password");
            return Err(AuthHandlerError::InvalidCredentials);
        }

        // Check 2FA if enabled
        // Note: Using deprecated method here because login happens before RLS context is established.
        // 2FA is user-level, not tenant-scoped, so this is acceptable for now.
        #[allow(deprecated)]
        let mfa_record_opt = state.two_factor_repo.get_by_user_id(user.id).await;
        if let Ok(Some(mfa_record)) = mfa_record_opt {
            if mfa_record.enabled {
                match &data.two_factor_code {
                    Some(code) => {
                        // Decrypt secret if encrypted
                        let decrypted_secret = state
                            .totp_service
                            .decrypt_secret(&mfa_record.secret)
                            .map_err(|e| {
                                tracing::error!(error = %e, "Failed to decrypt TOTP secret");
                                AuthHandlerError::Internal("Failed to verify MFA code".into())
                            })?;

                        // Verify TOTP code
                        let is_valid = state
                            .totp_service
                            .verify_code(&decrypted_secret, code)
                            .unwrap_or(false);

                        // If TOTP failed, try backup codes
                        let backup_codes: Vec<String> =
                            serde_json::from_value(mfa_record.backup_codes.clone())
                                .unwrap_or_default();
                        let backup_result = if !is_valid {
                            state
                                .totp_service
                                .verify_backup_code(code, &backup_codes)
                                .ok()
                                .flatten()
                        } else {
                            None
                        };

                        if !is_valid && backup_result.is_none() {
                            if let Err(err) = state
                                .session_repo
                                .record_login_attempt(&data.email, &data.ip_address, false)
                                .await
                            {
                                tracing::warn!(
                                    error = %err,
                                    "record_login_attempt failed (rate limiter may be unprotected for this attempt)"
                                );
                            }
                            return Err(AuthHandlerError::InvalidMfaCode);
                        }

                        // If backup code was used, consume it and log it
                        if let Some(code_index) = backup_result {
                            // Note: Using deprecated method here because login happens before RLS context.
                            #[allow(deprecated)]
                            let _ = state
                                .two_factor_repo
                                .use_backup_code(user.id, code_index)
                                .await;

                            // Log backup code usage
                            if let Err(e) = state
                                .audit_log_repo
                                .create(CreateAuditLog {
                                    user_id: Some(user.id),
                                    action: AuditAction::MfaBackupCodeUsed,
                                    resource_type: Some("two_factor_auth".to_string()),
                                    resource_id: Some(user.id),
                                    org_id: None,
                                    details: Some(serde_json::json!({ "code_index": code_index })),
                                    old_values: None,
                                    new_values: None,
                                    ip_address: Some(data.ip_address.clone()),
                                    user_agent: None,
                                })
                                .await
                            {
                                tracing::error!(error = %e, user_id = %user.id, "Failed to create audit log for backup code usage");
                            }

                            tracing::info!(user_id = %user.id, "Backup code used for login");
                        }
                    }
                    None => {
                        // No code provided - return MFA required response
                        return Ok(LoginResult {
                            access_token: String::new(),
                            refresh_token: String::new(),
                            expires_in: 0,
                            token_type: "Bearer".to_string(),
                            mfa_required: Some(true),
                        });
                    }
                }
            }
        }

        // Record successful login attempt
        if let Err(err) = state
            .session_repo
            .record_login_attempt(&data.email, &data.ip_address, true)
            .await
        {
            tracing::warn!(
                error = %err,
                "record_login_attempt failed (rate limiter may be unprotected for this attempt)"
            );
        }

        // Generate access token
        let access_token = state
            .jwt_service
            .generate_access_token(user.id, &user.email, &user.name, None, None)
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to generate access token");
                AuthHandlerError::Internal("Failed to create session".into())
            })?;

        // Generate refresh token
        let (refresh_token, token_hash, expires_at) = state
            .jwt_service
            .generate_refresh_token(user.id, &user.email, &user.name)
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to generate refresh token");
                AuthHandlerError::Internal("Failed to create session".into())
            })?;

        // Store refresh token in database
        let create_token = CreateRefreshToken {
            user_id: user.id,
            token_hash,
            expires_at,
            user_agent: data.user_agent.clone(),
            ip_address: Some(data.ip_address.clone()),
            device_info: None,
        };

        state
            .session_repo
            .create_refresh_token(create_token)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to store refresh token");
                AuthHandlerError::Database("Failed to create session".into())
            })?;

        tracing::info!(
            user_id = %user.id,
            email = %user.email,
            "User logged in successfully"
        );

        Ok(LoginResult {
            access_token,
            refresh_token,
            expires_in: state.jwt_service.access_token_lifetime(),
            token_type: "Bearer".to_string(),
            mfa_required: None,
        })
    }

    /// Refresh access token using refresh token.
    pub async fn refresh_token(
        state: &AppState,
        data: RefreshData,
    ) -> Result<LoginResult, AuthHandlerError> {
        // Validate the refresh token JWT
        let claims = state
            .jwt_service
            .validate_refresh_token(&data.refresh_token)
            .map_err(|e| {
                tracing::debug!(error = %e, "Invalid refresh token");
                AuthHandlerError::InvalidToken
            })?;

        // Hash the token to look it up in database
        let mut hasher = Sha256::new();
        hasher.update(data.refresh_token.as_bytes());
        let token_hash = hex::encode(hasher.finalize());

        // Find the token in database
        let stored_token = state
            .session_repo
            .find_by_token_hash(&token_hash)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Database error finding refresh token");
                AuthHandlerError::Database("Failed to validate session".into())
            })?
            .ok_or_else(|| {
                tracing::warn!(user_id = %claims.sub, "Refresh token not found in database (possibly revoked)");
                AuthHandlerError::TokenRevoked
            })?;

        // Check if token is still valid
        if !stored_token.is_valid() {
            return Err(AuthHandlerError::TokenExpired);
        }

        // Parse user ID from claims
        let user_id: Uuid = claims
            .sub
            .parse()
            .map_err(|_| AuthHandlerError::InvalidToken)?;

        // Verify user still exists and is active
        let user = state
            .user_repo
            .find_by_id(user_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Database error finding user");
                AuthHandlerError::Database("Failed to validate user".into())
            })?
            .ok_or(AuthHandlerError::UserNotFound)?;

        if user.status != "active" {
            // Revoke the token since user is no longer active
            let _ = state.session_repo.revoke_token(stored_token.id).await;
            return Err(AuthHandlerError::AccountInactive);
        }

        // Token rotation: revoke the old token
        if let Err(e) = state.session_repo.revoke_token(stored_token.id).await {
            tracing::error!(error = %e, "Failed to revoke old refresh token");
            // Continue anyway - better to issue new token than fail
        }

        // Generate new access token
        let access_token = state
            .jwt_service
            .generate_access_token(user.id, &user.email, &user.name, None, None)
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to generate access token");
                AuthHandlerError::Internal("Failed to create session".into())
            })?;

        // Generate new refresh token (rotation)
        let (new_refresh_token, new_token_hash, expires_at) = state
            .jwt_service
            .generate_refresh_token(user.id, &user.email, &user.name)
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to generate refresh token");
                AuthHandlerError::Internal("Failed to create session".into())
            })?;

        // Store new refresh token
        let create_token = CreateRefreshToken {
            user_id: user.id,
            token_hash: new_token_hash,
            expires_at,
            user_agent: stored_token.user_agent.clone(),
            ip_address: stored_token.ip_address.clone(),
            device_info: stored_token.device_info.clone(),
        };

        state
            .session_repo
            .create_refresh_token(create_token)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to store new refresh token");
                AuthHandlerError::Database("Failed to create session".into())
            })?;

        tracing::info!(user_id = %user.id, "Token refreshed successfully");

        Ok(LoginResult {
            access_token,
            refresh_token: new_refresh_token,
            expires_in: state.jwt_service.access_token_lifetime(),
            token_type: "Bearer".to_string(),
            mfa_required: None,
        })
    }

    /// Logout user by revoking refresh token.
    pub async fn logout(state: &AppState, refresh_token: &str) -> Result<(), AuthHandlerError> {
        // Hash the token to look it up
        let mut hasher = Sha256::new();
        hasher.update(refresh_token.as_bytes());
        let token_hash = hex::encode(hasher.finalize());

        // Find and revoke the token
        match state.session_repo.find_by_token_hash(&token_hash).await {
            Ok(Some(token)) => {
                if let Err(e) = state.session_repo.revoke_token(token.id).await {
                    tracing::error!(error = %e, "Failed to revoke token");
                } else {
                    tracing::info!(user_id = %token.user_id, "User logged out");
                }
            }
            Ok(None) => {
                // Token not found - might already be revoked, that's fine
                tracing::debug!("Logout requested for unknown/revoked token");
            }
            Err(e) => {
                tracing::error!(error = %e, "Database error during logout");
            }
        }

        Ok(())
    }

    /// Initiate password reset flow.
    pub async fn forgot_password(
        state: &AppState,
        data: ForgotPasswordData,
    ) -> Result<(), AuthHandlerError> {
        // Try to find user and send email
        match state.user_repo.find_by_email(&data.email).await {
            Ok(Some(user)) if user.status == "active" => {
                // P1-07: cap reset requests per user. Without this, an
                // attacker can use forgot_password to flood the victim's
                // inbox (each request sends a real email) and to confirm
                // which emails are registered via timing differences.
                // Default: 3 requests / 15 min. The same generic 200 OK
                // is returned on rate-limit to preserve the
                // non-enumeration property.
                const MAX_RESETS_PER_WINDOW: i64 = 3;
                const WINDOW_MINUTES: i32 = 15;
                match state
                    .password_reset_repo
                    .count_recent_for_user(user.id, WINDOW_MINUTES)
                    .await
                {
                    Ok(count) if count >= MAX_RESETS_PER_WINDOW => {
                        tracing::warn!(
                            user_id = %user.id,
                            count = count,
                            "Password reset rate limit hit; silently dropping request"
                        );
                        return Ok(());
                    }
                    Err(err) => {
                        tracing::warn!(
                            error = %err,
                            "count_recent_for_user failed; falling open (proceeding without rate limit on this request)"
                        );
                        // Fall through — better to send the email than
                        // to deny a legitimate user when our DB is sick.
                    }
                    _ => { /* under the cap, continue */ }
                }

                // Invalidate any existing reset tokens for this user
                let _ = state
                    .password_reset_repo
                    .invalidate_user_tokens(user.id)
                    .await;

                // Generate reset token (1 hour expiry)
                let token = state.auth_service.generate_token();
                let token_hash = state.auth_service.hash_token(&token);
                let expires_at = Utc::now() + Duration::hours(1);

                // Store token
                let create_token = CreatePasswordResetToken {
                    user_id: user.id,
                    token_hash,
                    expires_at,
                };

                if let Err(e) = state.password_reset_repo.create(create_token).await {
                    tracing::error!(error = %e, user_id = %user.id, "Failed to create password reset token");
                    return Ok(()); // Silent failure to prevent enumeration
                }

                // Send reset email
                if let Err(e) = state
                    .email_service
                    .send_password_reset_email(&user.email, &user.name, &token, &user.locale_enum())
                    .await
                {
                    tracing::error!(error = %e, user_id = %user.id, "Failed to send password reset email");
                }

                tracing::info!(user_id = %user.id, "Password reset email sent");
            }
            Ok(Some(user)) => {
                // User exists but not active
                tracing::debug!(
                    email = %data.email,
                    status = %user.status,
                    "Password reset requested for non-active account"
                );
            }
            Ok(None) => {
                // User not found - don't reveal this
                tracing::debug!(email = %data.email, "Password reset requested for unknown email");
            }
            Err(e) => {
                tracing::error!(error = %e, "Database error finding user for password reset");
            }
        }

        Ok(())
    }

    /// Complete password reset with token and new password.
    pub async fn reset_password(
        state: &AppState,
        data: ResetPasswordData,
    ) -> Result<String, AuthHandlerError> {
        // Validate new password requirements
        if let Err(errors) = AuthService::validate_password(&data.new_password) {
            return Err(AuthHandlerError::InvalidPassword(errors));
        }

        // Hash the token to look it up
        let token_hash = state.auth_service.hash_token(&data.token);

        // Find the reset token
        let reset_token = state
            .password_reset_repo
            .find_by_token_hash(&token_hash)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Database error finding password reset token");
                AuthHandlerError::Database("Failed to validate reset token".into())
            })?
            .ok_or(AuthHandlerError::InvalidToken)?;

        // Check if token is expired
        if reset_token.is_expired() {
            return Err(AuthHandlerError::TokenExpired);
        }

        // Find the user
        let user = state
            .user_repo
            .find_by_id(reset_token.user_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Database error finding user");
                AuthHandlerError::Database("Failed to reset password".into())
            })?
            .ok_or(AuthHandlerError::UserNotFound)?;

        // Check if user is still active
        if user.status != "active" {
            return Err(AuthHandlerError::AccountInactive);
        }

        // Hash new password
        let password_hash = state
            .auth_service
            .hash_password(&data.new_password)
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to hash new password");
                AuthHandlerError::Internal("Failed to process password".into())
            })?;

        // Update password
        state
            .user_repo
            .update_password(user.id, &password_hash)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, user_id = %user.id, "Failed to update password");
                AuthHandlerError::Database("Failed to update password".into())
            })?;

        // Mark token as used
        if let Err(e) = state.password_reset_repo.mark_used(reset_token.id).await {
            tracing::error!(error = %e, "Failed to mark reset token as used");
            // Continue anyway - password was changed successfully
        }

        // Revoke all refresh tokens for security (force re-login)
        if let Err(e) = state
            .session_repo
            .revoke_all_user_tokens(user.id, None)
            .await
        {
            tracing::error!(error = %e, "Failed to revoke user sessions");
            // Continue anyway - password was changed
        }

        tracing::info!(user_id = %user.id, "Password reset successfully");

        Ok("Password has been reset. Please log in with your new password.".to_string())
    }

    /// List active sessions for a user.
    pub async fn list_sessions(
        state: &AppState,
        user_id: Uuid,
        current_refresh_token: Option<&str>,
    ) -> Result<Vec<SessionInfo>, AuthHandlerError> {
        // Get current token hash to identify current session
        let current_token_hash = current_refresh_token.map(|token| {
            let mut hasher = Sha256::new();
            hasher.update(token.as_bytes());
            hex::encode(hasher.finalize())
        });

        // Get all active sessions for user
        let sessions = state
            .session_repo
            .find_user_sessions(user_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch user sessions");
                AuthHandlerError::Database("Failed to fetch sessions".into())
            })?;

        let session_infos: Vec<SessionInfo> = sessions
            .into_iter()
            .map(|s| {
                let is_current = current_token_hash
                    .as_ref()
                    .map(|h| h == &s.token_hash)
                    .unwrap_or(false);

                SessionInfo {
                    id: s.id,
                    device_info: s.device_info,
                    ip_address: s.ip_address,
                    user_agent: s.user_agent,
                    created_at: s.created_at,
                    last_used_at: s.last_used_at,
                    is_current,
                }
            })
            .collect();

        Ok(session_infos)
    }

    /// Revoke a specific session.
    pub async fn revoke_session(
        state: &AppState,
        user_id: Uuid,
        session_id: Uuid,
    ) -> Result<(), AuthHandlerError> {
        // Verify session belongs to this user
        let sessions = state
            .session_repo
            .find_user_sessions(user_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch user sessions");
                AuthHandlerError::Database("Failed to verify session".into())
            })?;

        let session_exists = sessions.iter().any(|s| s.id == session_id);
        if !session_exists {
            return Err(AuthHandlerError::InvalidToken);
        }

        // Revoke the session
        match state.session_repo.revoke_token(session_id).await {
            Ok(true) => {
                tracing::info!(user_id = %user_id, session_id = %session_id, "Session revoked");
                Ok(())
            }
            Ok(false) => Err(AuthHandlerError::InvalidToken),
            Err(e) => {
                tracing::error!(error = %e, "Failed to revoke session");
                Err(AuthHandlerError::Database(
                    "Failed to revoke session".into(),
                ))
            }
        }
    }

    /// Revoke all sessions except current.
    pub async fn revoke_all_sessions(
        state: &AppState,
        user_id: Uuid,
        current_refresh_token: Option<&str>,
    ) -> Result<u64, AuthHandlerError> {
        // Get current session to exclude
        let current_session_id = if let Some(refresh_token) = current_refresh_token {
            let mut hasher = Sha256::new();
            hasher.update(refresh_token.as_bytes());
            let token_hash = hex::encode(hasher.finalize());

            // Find session by hash
            match state.session_repo.find_by_token_hash(&token_hash).await {
                Ok(Some(session)) => Some(session.id),
                _ => None,
            }
        } else {
            None
        };

        // Revoke all sessions except current
        match state
            .session_repo
            .revoke_all_user_tokens(user_id, current_session_id)
            .await
        {
            Ok(count) => {
                tracing::info!(
                    user_id = %user_id,
                    revoked_count = count,
                    "All other sessions revoked"
                );
                Ok(count)
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to revoke sessions");
                Err(AuthHandlerError::Database(
                    "Failed to revoke sessions".into(),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        let err = AuthHandlerError::InvalidEmail;
        let response: ErrorResponse = err.into();
        assert_eq!(response.code, "INVALID_EMAIL");
    }

    #[test]
    fn test_password_validation_error() {
        let err = AuthHandlerError::InvalidPassword(vec![
            "Too short".to_string(),
            "Missing uppercase".to_string(),
        ]);
        let response: ErrorResponse = err.into();
        assert_eq!(response.code, "VALIDATION_ERROR");
    }
}
