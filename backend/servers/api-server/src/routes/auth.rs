//! Authentication routes (UC-14, Epic 1).

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use common::errors::{ErrorResponse, ValidationError};
use db::models::{AuditAction, CreateAuditLog, CreateUser, Locale};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::services::AuthService;
use crate::state::AppState;

/// Create auth router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/verify-email", get(verify_email))
        .route("/resend-verification", post(resend_verification))
        .route("/login", post(login))
        .route("/refresh", post(refresh_token))
        .route("/logout", post(logout))
        .route("/forgot-password", post(forgot_password))
        .route("/reset-password", post(reset_password))
        // Session management (Story 1.5)
        .route("/sessions", get(list_sessions))
        .route("/sessions/revoke", post(revoke_session))
        .route("/sessions/revoke-all", post(revoke_all_sessions))
}

// ==================== Register (Story 1.1) ====================

/// Register request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    /// Email address
    pub email: String,
    /// Password (min 8 characters, 1 uppercase, 1 number)
    pub password: String,
    /// Display name
    pub name: String,
    /// Phone number (optional)
    pub phone: Option<String>,
    /// Preferred locale (sk, cs, de, en)
    pub locale: Option<String>,
}

/// Register response.
#[derive(Debug, Serialize, ToSchema)]
pub struct RegisterResponse {
    /// Success message
    pub message: String,
    /// User ID
    pub user_id: String,
}

/// Register endpoint.
#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    tag = "Authentication",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "Registration successful", body = RegisterResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 409, description = "Email already exists", body = ErrorResponse)
    )
)]
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Validate email format
    if !AuthService::validate_email(&req.email) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("INVALID_EMAIL", "Invalid email format")),
        ));
    }

    // Validate password requirements
    if let Err(errors) = AuthService::validate_password(&req.password) {
        let details: Vec<ValidationError> = errors
            .into_iter()
            .map(|msg| ValidationError {
                field: "password".to_string(),
                message: msg.clone(),
                code: "INVALID_PASSWORD".to_string(),
            })
            .collect();
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                ErrorResponse::new("VALIDATION_ERROR", "Password does not meet requirements")
                    .with_details(details),
            ),
        ));
    }

    // Validate name is not empty
    if req.name.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("INVALID_NAME", "Name cannot be empty")),
        ));
    }

    // Check if email already exists
    match state.user_repo.email_exists(&req.email).await {
        Ok(true) => {
            // Don't reveal whether account is verified or not
            return Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse::new(
                    "EMAIL_EXISTS",
                    "An account with this email already exists",
                )),
            ));
        }
        Ok(false) => {}
        Err(e) => {
            tracing::error!(error = %e, "Database error checking email");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check email",
                )),
            ));
        }
    }

    // Hash password
    let password_hash = match state.auth_service.hash_password(&req.password) {
        Ok(hash) => hash,
        Err(e) => {
            tracing::error!(error = %e, "Failed to hash password");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to process password",
                )),
            ));
        }
    };

    // Determine locale from request or default to English
    let locale = req
        .locale
        .as_ref()
        .map(|l| Locale::parse(l))
        .unwrap_or(Locale::English);

    // Create user
    let create_user = CreateUser {
        email: req.email.clone(),
        password_hash,
        name: req.name.clone(),
        phone: req.phone.clone(),
        locale: locale.clone(),
    };

    let user = match state.user_repo.create(create_user).await {
        Ok(user) => user,
        Err(e) => {
            tracing::error!(error = %e, email = %req.email, "Failed to create user");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to create account",
                )),
            ));
        }
    };

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

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            message: "Check your email to verify your account".to_string(),
            user_id: user.id.to_string(),
        }),
    ))
}

// ==================== Verify Email (Story 1.1) ====================

/// Verify email query parameters.
#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyEmailQuery {
    /// Verification token from email
    pub token: String,
}

/// Verify email response.
#[derive(Debug, Serialize, ToSchema)]
pub struct VerifyEmailResponse {
    /// Success message
    pub message: String,
}

/// Verify email endpoint.
#[utoipa::path(
    get,
    path = "/api/v1/auth/verify-email",
    tag = "Authentication",
    params(
        ("token" = String, Query, description = "Email verification token")
    ),
    responses(
        (status = 200, description = "Email verified", body = VerifyEmailResponse),
        (status = 400, description = "Invalid or expired token", body = ErrorResponse)
    )
)]
pub async fn verify_email(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<VerifyEmailQuery>,
) -> Result<Json<VerifyEmailResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Hash the token to look it up
    let token_hash = state.auth_service.hash_token(&query.token);

    // Find the token
    let verification_token = match state.user_repo.find_verification_token(&token_hash).await {
        Ok(Some(token)) => token,
        Ok(None) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_TOKEN",
                    "This verification link is invalid or has already been used",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error finding verification token");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to verify token",
                )),
            ));
        }
    };

    // Check if token is expired
    if verification_token.is_expired() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "TOKEN_EXPIRED",
                "This verification link has expired. Please request a new one.",
            )),
        ));
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
            Ok(Json(VerifyEmailResponse {
                message: "Your email has been verified. You can now log in.".to_string(),
            }))
        }
        Ok(None) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "USER_NOT_FOUND",
                "User account not found or already verified",
            )),
        )),
        Err(e) => {
            tracing::error!(error = %e, "Failed to verify email");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to verify email",
                )),
            ))
        }
    }
}

// ==================== Resend Verification (Story 1.1) ====================

/// Resend verification request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ResendVerificationRequest {
    /// Email address
    pub email: String,
}

/// Resend verification response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ResendVerificationResponse {
    /// Success message
    pub message: String,
}

/// Resend verification email endpoint.
#[utoipa::path(
    post,
    path = "/api/v1/auth/resend-verification",
    tag = "Authentication",
    request_body = ResendVerificationRequest,
    responses(
        (status = 200, description = "Verification email sent (if account exists)", body = ResendVerificationResponse),
    )
)]
pub async fn resend_verification(
    State(state): State<AppState>,
    Json(req): Json<ResendVerificationRequest>,
) -> Json<ResendVerificationResponse> {
    // Always return success to prevent email enumeration
    let response = ResendVerificationResponse {
        message:
            "If an unverified account exists with this email, a verification link has been sent."
                .to_string(),
    };

    // Try to find user and send email
    match state.user_repo.find_by_email(&req.email).await {
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
                return Json(response);
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
            tracing::debug!(email = %req.email, "Resend verification request for non-pending account");
        }
    }

    Json(response)
}

// ==================== Login (Story 1.2) ====================

/// Login request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    /// Email address
    pub email: String,
    /// Password
    pub password: String,
    /// 2FA code (optional, for future use)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub two_factor_code: Option<String>,
}

/// Login response.
#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    /// JWT access token (empty if MFA required)
    pub access_token: String,
    /// Refresh token (empty if MFA required)
    pub refresh_token: String,
    /// Access token expiration in seconds
    pub expires_in: i64,
    /// Token type (always "Bearer")
    pub token_type: String,
    /// Whether MFA verification is required to complete login
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mfa_required: Option<bool>,
    /// Temporary token for MFA verification (only present if mfa_required)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mfa_token: Option<String>,
}

/// Login endpoint.
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "Authentication",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials", body = ErrorResponse),
        (status = 429, description = "Too many failed attempts", body = ErrorResponse)
    )
)]
pub async fn login(
    State(state): State<AppState>,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    headers: axum::http::HeaderMap,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ip_address = addr.ip().to_string();
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    // Check rate limiting
    match state.session_repo.check_rate_limit(&req.email).await {
        Ok(status) if !status.can_attempt() => {
            let remaining = status.lockout_remaining_secs.unwrap_or(900);
            tracing::warn!(
                email = %req.email,
                ip = %ip_address,
                remaining_secs = remaining,
                "Login attempt blocked due to rate limiting"
            );
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                Json(ErrorResponse::new(
                    "RATE_LIMITED",
                    format!(
                        "Too many failed login attempts. Please try again in {} minutes.",
                        remaining / 60 + 1
                    ),
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to check rate limit");
            // Continue anyway - don't block login due to rate limit check failure
        }
        _ => {}
    }

    // Find user by email
    let user = match state.user_repo.find_by_email(&req.email).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            // Record failed attempt (user not found)
            let _ = state
                .session_repo
                .record_login_attempt(&req.email, &ip_address, false)
                .await;
            tracing::debug!(email = %req.email, "Login failed: user not found");
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "INVALID_CREDENTIALS",
                    "Invalid email or password",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error finding user");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Login failed")),
            ));
        }
    };

    // Check if user can log in
    if user.status == "suspended" {
        let _ = state
            .session_repo
            .record_login_attempt(&req.email, &ip_address, false)
            .await;
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "ACCOUNT_SUSPENDED",
                "Account suspended. Contact support.",
            )),
        ));
    }

    if !user.is_verified() {
        let _ = state
            .session_repo
            .record_login_attempt(&req.email, &ip_address, false)
            .await;
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "EMAIL_NOT_VERIFIED",
                "Please verify your email first",
            )),
        ));
    }

    // Verify password
    let password_valid = match state
        .auth_service
        .verify_password(&req.password, &user.password_hash)
    {
        Ok(valid) => valid,
        Err(e) => {
            tracing::error!(error = %e, "Password verification error");
            let _ = state
                .session_repo
                .record_login_attempt(&req.email, &ip_address, false)
                .await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Login failed")),
            ));
        }
    };

    if !password_valid {
        let _ = state
            .session_repo
            .record_login_attempt(&req.email, &ip_address, false)
            .await;
        tracing::debug!(email = %req.email, "Login failed: invalid password");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "INVALID_CREDENTIALS",
                "Invalid email or password",
            )),
        ));
    }

    // Track whether the user has supplied & verified an MFA factor in this
    // request. Used by the per-org `AuthPolicyEnforcer::check_login` gate
    // below — an org policy can demand MFA-at-login for a role even if the
    // user has not yet enrolled a TOTP secret.
    let mut mfa_presented = false;

    // Check 2FA if enabled (Epic 9, Story 9.1)
    // Note: Login happens before RLS context is established. 2FA is user-level, not tenant-scoped.
    #[allow(deprecated)]
    let mfa_check_result = state.two_factor_repo.get_by_user_id(user.id).await;
    if let Ok(Some(mfa_record)) = mfa_check_result {
        if mfa_record.enabled {
            // 2FA is enabled - check if code was provided
            match &req.two_factor_code {
                Some(code) => {
                    // Decrypt secret if encrypted (Story 9.1 security fix)
                    let decrypted_secret = state
                        .totp_service
                        .decrypt_secret(&mfa_record.secret)
                        .map_err(|e| {
                            tracing::error!(error = %e, "Failed to decrypt TOTP secret");
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ErrorResponse::new(
                                    "DECRYPTION_ERROR",
                                    "Failed to verify MFA code",
                                )),
                            )
                        })?;

                    // Verify TOTP code
                    let is_valid = state
                        .totp_service
                        .verify_code(&decrypted_secret, code)
                        .unwrap_or(false);

                    // If TOTP failed, try backup codes
                    let backup_codes: Vec<String> =
                        serde_json::from_value(mfa_record.backup_codes.clone()).unwrap_or_default();
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
                        let _ = state
                            .session_repo
                            .record_login_attempt(&req.email, &ip_address, false)
                            .await;
                        return Err((
                            StatusCode::UNAUTHORIZED,
                            Json(ErrorResponse::new(
                                "INVALID_MFA_CODE",
                                "Invalid verification code",
                            )),
                        ));
                    }

                    // MFA factor verified (TOTP or backup code).
                    mfa_presented = true;

                    // If backup code was used, consume it and log it
                    if let Some(code_index) = backup_result {
                        // Note: Login happens before RLS context is established.
                        #[allow(deprecated)]
                        let _ = state
                            .two_factor_repo
                            .use_backup_code(user.id, code_index)
                            .await;

                        // Log backup code usage (Story 9.6 - Audit logging)
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
                                ip_address: Some(ip_address.clone()),
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
                    // No code provided - return MFA required response.
                    // The client should retry login with the two_factor_code included.
                    return Ok(Json(LoginResponse {
                        access_token: String::new(),
                        refresh_token: String::new(),
                        expires_in: 0,
                        token_type: "Bearer".to_string(),
                        mfa_required: Some(true),
                        mfa_token: None,
                    }));
                }
            }
        }
    }

    // D2.2: enforce per-org auth policy at login. If ANY org the user has an
    // active membership in requires MFA for the user's role in that org and
    // the user has not presented an MFA factor in this request, refuse the
    // login with the same `mfa_required` 401 response shape the existing 2FA
    // path uses. Email verification is already enforced above by
    // `User::is_verified` so we map only `MfaRequired` here.
    let enforcer = crate::services::AuthPolicyEnforcer::new(state.db.clone());
    if let Err(err) = enforcer.check_login(user.id, mfa_presented).await {
        let _ = state
            .session_repo
            .record_login_attempt(&req.email, &ip_address, false)
            .await;
        match err {
            crate::services::AuthPolicyError::MfaRequired(role) => {
                tracing::info!(user_id = %user.id, role = %role, "Login blocked: org policy demands MFA for role");
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse::new(
                        "MFA_REQUIRED",
                        format!("MFA required by org policy for role '{}'", role),
                    )),
                ));
            }
            other => {
                tracing::error!(error = %other, user_id = %user.id, "Auth policy check at login failed");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("AUTH_POLICY_ERROR", "Login failed")),
                ));
            }
        }
    }

    // Record successful login attempt
    let _ = state
        .session_repo
        .record_login_attempt(&req.email, &ip_address, true)
        .await;

    // Generate access token, embedding principal_kind (Phase 6 C17).
    let access_token = match state.jwt_service.generate_access_token_with_kind(
        user.id,
        &user.email,
        &user.name,
        None, // org_id - will be set when org context is selected
        None, // roles - will be set when org context is selected
        Some(user.principal_kind.clone()),
    ) {
        Ok(token) => token,
        Err(e) => {
            tracing::error!(error = %e, "Failed to generate access token");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "TOKEN_ERROR",
                    "Failed to create session",
                )),
            ));
        }
    };

    // Generate refresh token
    let (refresh_token, token_hash, expires_at) =
        match state
            .jwt_service
            .generate_refresh_token(user.id, &user.email, &user.name)
        {
            Ok(result) => result,
            Err(e) => {
                tracing::error!(error = %e, "Failed to generate refresh token");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "TOKEN_ERROR",
                        "Failed to create session",
                    )),
                ));
            }
        };

    // Store refresh token in database
    use db::models::CreateRefreshToken;
    let create_token = CreateRefreshToken {
        user_id: user.id,
        token_hash,
        expires_at,
        user_agent: user_agent.clone(),
        ip_address: Some(ip_address.clone()),
        device_info: None, // Can be set from client-provided header in future
    };

    if let Err(e) = state.session_repo.create_refresh_token(create_token).await {
        tracing::error!(error = %e, "Failed to store refresh token");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to create session",
            )),
        ));
    }

    tracing::info!(
        user_id = %user.id,
        email = %user.email,
        "User logged in successfully"
    );

    Ok(Json(LoginResponse {
        access_token,
        refresh_token,
        expires_in: state.jwt_service.access_token_lifetime(),
        token_type: "Bearer".to_string(),
        mfa_required: None,
        mfa_token: None,
    }))
}

// ==================== Refresh Token (Story 1.3) ====================

/// Refresh token request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshTokenRequest {
    /// The refresh token to exchange for new tokens
    pub refresh_token: String,
}

/// Refresh token endpoint.
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    tag = "Authentication",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Token refreshed", body = LoginResponse),
        (status = 401, description = "Invalid or expired refresh token", body = ErrorResponse)
    )
)]
pub async fn refresh_token(
    State(state): State<AppState>,
    Json(req): Json<RefreshTokenRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Validate the refresh token JWT
    let claims = match state.jwt_service.validate_refresh_token(&req.refresh_token) {
        Ok(claims) => claims,
        Err(e) => {
            tracing::debug!(error = %e, "Invalid refresh token");
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "INVALID_TOKEN",
                    "Invalid or expired refresh token",
                )),
            ));
        }
    };

    // Hash the token to look it up in database
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(req.refresh_token.as_bytes());
    let token_hash = hex::encode(hasher.finalize());

    // Find the token in database
    let stored_token = match state.session_repo.find_by_token_hash(&token_hash).await {
        Ok(Some(token)) => token,
        Ok(None) => {
            tracing::warn!(user_id = %claims.sub, "Refresh token not found in database (possibly revoked)");
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "TOKEN_REVOKED",
                    "This session has been revoked",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error finding refresh token");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to validate session",
                )),
            ));
        }
    };

    // Check if token is still valid
    if !stored_token.is_valid() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("TOKEN_EXPIRED", "Session has expired")),
        ));
    }

    // Parse user ID from claims
    let user_id: uuid::Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Verify user still exists and is active
    let user = match state.user_repo.find_by_id(user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "USER_NOT_FOUND",
                    "User account no longer exists",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error finding user");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to validate user",
                )),
            ));
        }
    };

    if user.status != "active" {
        // Revoke the token since user is no longer active
        let _ = state.session_repo.revoke_token(stored_token.id).await;
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "ACCOUNT_INACTIVE",
                "Account is no longer active",
            )),
        ));
    }

    // Token rotation: revoke the old token
    if let Err(e) = state.session_repo.revoke_token(stored_token.id).await {
        tracing::error!(error = %e, "Failed to revoke old refresh token");
        // Continue anyway - better to issue new token than fail
    }

    // Generate new access token, re-embedding principal_kind (Phase 6 C17).
    let access_token = match state.jwt_service.generate_access_token_with_kind(
        user.id,
        &user.email,
        &user.name,
        None,
        None,
        Some(user.principal_kind.clone()),
    ) {
        Ok(token) => token,
        Err(e) => {
            tracing::error!(error = %e, "Failed to generate access token");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "TOKEN_ERROR",
                    "Failed to create session",
                )),
            ));
        }
    };

    // Generate new refresh token (rotation)
    let (new_refresh_token, new_token_hash, expires_at) = match state
        .jwt_service
        .generate_refresh_token(user.id, &user.email, &user.name)
    {
        Ok(result) => result,
        Err(e) => {
            tracing::error!(error = %e, "Failed to generate refresh token");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "TOKEN_ERROR",
                    "Failed to create session",
                )),
            ));
        }
    };

    // Store new refresh token
    use db::models::CreateRefreshToken;
    let create_token = CreateRefreshToken {
        user_id: user.id,
        token_hash: new_token_hash,
        expires_at,
        user_agent: stored_token.user_agent.clone(),
        ip_address: stored_token.ip_address.clone(),
        device_info: stored_token.device_info.clone(),
    };

    if let Err(e) = state.session_repo.create_refresh_token(create_token).await {
        tracing::error!(error = %e, "Failed to store new refresh token");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to create session",
            )),
        ));
    }

    tracing::info!(user_id = %user.id, "Token refreshed successfully");

    Ok(Json(LoginResponse {
        access_token,
        refresh_token: new_refresh_token,
        expires_in: state.jwt_service.access_token_lifetime(),
        token_type: "Bearer".to_string(),
        mfa_required: None,
        mfa_token: None,
    }))
}

// ==================== Logout (Story 1.3) ====================

/// Logout request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct LogoutRequest {
    /// The refresh token to revoke
    pub refresh_token: String,
}

/// Logout response.
#[derive(Debug, Serialize, ToSchema)]
pub struct LogoutResponse {
    /// Success message
    pub message: String,
}

/// Logout endpoint.
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "Authentication",
    request_body = LogoutRequest,
    responses(
        (status = 200, description = "Logout successful", body = LogoutResponse),
        (status = 401, description = "Invalid token")
    )
)]
pub async fn logout(
    State(state): State<AppState>,
    Json(req): Json<LogoutRequest>,
) -> Result<Json<LogoutResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Hash the token to look it up
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(req.refresh_token.as_bytes());
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

    // Always return success to prevent token enumeration
    Ok(Json(LogoutResponse {
        message: "Logged out successfully".to_string(),
    }))
}

// ==================== Forgot Password (Story 1.4) ====================

/// Forgot password request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ForgotPasswordRequest {
    /// Email address
    pub email: String,
}

/// Forgot password response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ForgotPasswordResponse {
    /// Success message (always same to prevent enumeration)
    pub message: String,
}

/// Forgot password endpoint - initiates password reset.
#[utoipa::path(
    post,
    path = "/api/v1/auth/forgot-password",
    tag = "Authentication",
    request_body = ForgotPasswordRequest,
    responses(
        (status = 200, description = "Password reset email sent (if account exists)", body = ForgotPasswordResponse),
    )
)]
pub async fn forgot_password(
    State(state): State<AppState>,
    Json(req): Json<ForgotPasswordRequest>,
) -> Json<ForgotPasswordResponse> {
    // Always return success to prevent email enumeration
    let response = ForgotPasswordResponse {
        message: "If an account exists with this email, a password reset link has been sent."
            .to_string(),
    };

    // Try to find user and send email
    match state.user_repo.find_by_email(&req.email).await {
        Ok(Some(user)) if user.status == "active" => {
            // Invalidate any existing reset tokens for this user
            let _ = state
                .password_reset_repo
                .invalidate_user_tokens(user.id)
                .await;

            // Generate reset token (1 hour expiry)
            let token = state.auth_service.generate_token();
            let token_hash = state.auth_service.hash_token(&token);
            let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);

            // Store token
            use db::models::CreatePasswordResetToken;
            let create_token = CreatePasswordResetToken {
                user_id: user.id,
                token_hash,
                expires_at,
            };

            if let Err(e) = state.password_reset_repo.create(create_token).await {
                tracing::error!(error = %e, user_id = %user.id, "Failed to create password reset token");
                return Json(response);
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
            // User exists but not active (pending/suspended/deleted)
            tracing::debug!(
                email = %req.email,
                status = %user.status,
                "Password reset requested for non-active account"
            );
        }
        Ok(None) => {
            // User not found - don't reveal this
            tracing::debug!(email = %req.email, "Password reset requested for unknown email");
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error finding user for password reset");
        }
    }

    Json(response)
}

// ==================== Reset Password (Story 1.4) ====================

/// Reset password request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ResetPasswordRequest {
    /// Reset token from email
    pub token: String,
    /// New password (min 8 characters, 1 uppercase, 1 number)
    pub new_password: String,
}

/// Reset password response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ResetPasswordResponse {
    /// Success message
    pub message: String,
}

/// Reset password endpoint - completes password reset.
#[utoipa::path(
    post,
    path = "/api/v1/auth/reset-password",
    tag = "Authentication",
    request_body = ResetPasswordRequest,
    responses(
        (status = 200, description = "Password reset successful", body = ResetPasswordResponse),
        (status = 400, description = "Invalid or expired token", body = ErrorResponse),
        (status = 400, description = "Password does not meet requirements", body = ErrorResponse)
    )
)]
pub async fn reset_password(
    State(state): State<AppState>,
    Json(req): Json<ResetPasswordRequest>,
) -> Result<Json<ResetPasswordResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Validate new password requirements
    if let Err(errors) = AuthService::validate_password(&req.new_password) {
        let details: Vec<ValidationError> = errors
            .into_iter()
            .map(|msg| ValidationError {
                field: "new_password".to_string(),
                message: msg.clone(),
                code: "INVALID_PASSWORD".to_string(),
            })
            .collect();
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                ErrorResponse::new("VALIDATION_ERROR", "Password does not meet requirements")
                    .with_details(details),
            ),
        ));
    }

    // Hash the token to look it up
    let token_hash = state.auth_service.hash_token(&req.token);

    // Find the reset token
    let reset_token = match state
        .password_reset_repo
        .find_by_token_hash(&token_hash)
        .await
    {
        Ok(Some(token)) => token,
        Ok(None) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_TOKEN",
                    "This password reset link is invalid or has already been used",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error finding password reset token");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to validate reset token",
                )),
            ));
        }
    };

    // Check if token is expired
    if reset_token.is_expired() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "TOKEN_EXPIRED",
                "This password reset link has expired. Please request a new one.",
            )),
        ));
    }

    // Find the user
    let user = match state.user_repo.find_by_id(reset_token.user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "USER_NOT_FOUND",
                    "User account no longer exists",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error finding user");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to reset password",
                )),
            ));
        }
    };

    // Check if user is still active
    if user.status != "active" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "ACCOUNT_INACTIVE",
                "Cannot reset password for inactive account",
            )),
        ));
    }

    // D2.2: validate the new password against the user's effective per-org
    // auth policy (strictest across active memberships; falls back to the
    // platform default for users with no orgs). The `AuthService::validate_password`
    // call above enforces the platform-default rules; this enforcer call adds
    // any per-org tightening (longer min length, required char classes).
    let enforcer = crate::services::AuthPolicyEnforcer::new(state.db.clone());
    if let Err(err) = enforcer
        .check_password_change(user.id, &req.new_password)
        .await
    {
        match err {
            crate::services::AuthPolicyError::PasswordPolicy(violations) => {
                let details: Vec<ValidationError> = violations
                    .into_iter()
                    .map(|msg| ValidationError {
                        field: "new_password".to_string(),
                        message: msg,
                        code: "INVALID_PASSWORD".to_string(),
                    })
                    .collect();
                return Err((
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(
                        ErrorResponse::new(
                            "PASSWORD_POLICY_VIOLATION",
                            "Password does not satisfy org policy",
                        )
                        .with_details(details),
                    ),
                ));
            }
            other => {
                tracing::error!(error = %other, user_id = %user.id, "Password policy check failed");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "AUTH_POLICY_ERROR",
                        "Failed to validate password",
                    )),
                ));
            }
        }
    }

    // Hash new password
    let password_hash = match state.auth_service.hash_password(&req.new_password) {
        Ok(hash) => hash,
        Err(e) => {
            tracing::error!(error = %e, "Failed to hash new password");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to process password",
                )),
            ));
        }
    };

    // Update password
    if let Err(e) = state
        .user_repo
        .update_password(user.id, &password_hash)
        .await
    {
        tracing::error!(error = %e, user_id = %user.id, "Failed to update password");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to update password",
            )),
        ));
    }

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

    Ok(Json(ResetPasswordResponse {
        message: "Password has been reset. Please log in with your new password.".to_string(),
    }))
}

// ==================== Session Management (Story 1.5) ====================

/// Session info returned to clients.
#[derive(Debug, Serialize, ToSchema)]
pub struct SessionInfo {
    /// Session ID
    pub id: String,
    /// Device info (if available)
    pub device_info: Option<String>,
    /// IP address
    pub ip_address: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
    /// When the session was created
    pub created_at: String,
    /// When the session was last used
    pub last_used_at: String,
    /// Whether this is the current session
    pub is_current: bool,
}

/// List sessions response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ListSessionsResponse {
    /// Active sessions
    pub sessions: Vec<SessionInfo>,
}

/// List active sessions endpoint.
#[utoipa::path(
    get,
    path = "/api/v1/auth/sessions",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Sessions retrieved", body = ListSessionsResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn list_sessions(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ListSessionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Extract and validate access token
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: uuid::Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Get current token hash to identify current session
    use sha2::{Digest, Sha256};
    let current_token_hash =
        if let Some(refresh_token) = headers.get("X-Refresh-Token").and_then(|h| h.to_str().ok()) {
            let mut hasher = Sha256::new();
            hasher.update(refresh_token.as_bytes());
            Some(hex::encode(hasher.finalize()))
        } else {
            None
        };

    // Get all active sessions for user
    let sessions = match state.session_repo.find_user_sessions(user_id).await {
        Ok(sessions) => sessions,
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch user sessions");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to fetch sessions",
                )),
            ));
        }
    };

    let session_infos: Vec<SessionInfo> = sessions
        .into_iter()
        .map(|s| {
            let is_current = current_token_hash
                .as_ref()
                .map(|h| h == &s.token_hash)
                .unwrap_or(false);

            SessionInfo {
                id: s.id.to_string(),
                device_info: s.device_info,
                ip_address: s.ip_address,
                user_agent: s.user_agent,
                created_at: s.created_at.to_rfc3339(),
                last_used_at: s.last_used_at.to_rfc3339(),
                is_current,
            }
        })
        .collect();

    Ok(Json(ListSessionsResponse {
        sessions: session_infos,
    }))
}

/// Revoke session request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RevokeSessionRequest {
    /// Session ID to revoke
    pub session_id: String,
}

/// Revoke session response.
#[derive(Debug, Serialize, ToSchema)]
pub struct RevokeSessionResponse {
    /// Success message
    pub message: String,
}

/// Revoke a specific session.
#[utoipa::path(
    post,
    path = "/api/v1/auth/sessions/revoke",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    request_body = RevokeSessionRequest,
    responses(
        (status = 200, description = "Session revoked", body = RevokeSessionResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 404, description = "Session not found", body = ErrorResponse)
    )
)]
pub async fn revoke_session(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<RevokeSessionRequest>,
) -> Result<Json<RevokeSessionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Extract and validate access token
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: uuid::Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Parse session ID
    let session_id: uuid::Uuid = req.session_id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_SESSION_ID",
                "Invalid session ID format",
            )),
        )
    })?;

    // Verify session belongs to this user
    let sessions = match state.session_repo.find_user_sessions(user_id).await {
        Ok(sessions) => sessions,
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch user sessions");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to verify session",
                )),
            ));
        }
    };

    let session_exists = sessions.iter().any(|s| s.id == session_id);
    if !session_exists {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("SESSION_NOT_FOUND", "Session not found")),
        ));
    }

    // Revoke the session
    match state.session_repo.revoke_token(session_id).await {
        Ok(true) => {
            tracing::info!(user_id = %user_id, session_id = %session_id, "Session revoked");
            Ok(Json(RevokeSessionResponse {
                message: "Session revoked successfully".to_string(),
            }))
        }
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "SESSION_NOT_FOUND",
                "Session already revoked",
            )),
        )),
        Err(e) => {
            tracing::error!(error = %e, "Failed to revoke session");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to revoke session",
                )),
            ))
        }
    }
}

/// Revoke all sessions response.
#[derive(Debug, Serialize, ToSchema)]
pub struct RevokeAllSessionsResponse {
    /// Success message
    pub message: String,
    /// Number of sessions revoked
    pub revoked_count: u64,
}

/// Revoke all sessions except current.
#[utoipa::path(
    post,
    path = "/api/v1/auth/sessions/revoke-all",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Sessions revoked", body = RevokeAllSessionsResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn revoke_all_sessions(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<RevokeAllSessionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Extract and validate access token
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: uuid::Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Get current session to exclude
    let current_session_id =
        if let Some(refresh_token) = headers.get("X-Refresh-Token").and_then(|h| h.to_str().ok()) {
            use sha2::{Digest, Sha256};
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
            Ok(Json(RevokeAllSessionsResponse {
                message: format!("{} session(s) revoked", count),
                revoked_count: count,
            }))
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to revoke sessions");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to revoke sessions",
                )),
            ))
        }
    }
}

// ==================== Helper Functions ====================

/// Extract bearer token from Authorization header.
fn extract_bearer_token(
    headers: &axum::http::HeaderMap,
) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "MISSING_TOKEN",
                    "Authorization header required",
                )),
            )
        })?;

    if !auth_header.starts_with("Bearer ") {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Bearer token required")),
        ));
    }

    Ok(auth_header[7..].to_string())
}

/// Validate access token and return claims.
fn validate_access_token(
    state: &AppState,
    token: &str,
) -> Result<crate::services::jwt::Claims, (StatusCode, Json<ErrorResponse>)> {
    state.jwt_service.validate_access_token(token).map_err(|e| {
        tracing::debug!(error = %e, "Invalid access token");
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "INVALID_TOKEN",
                "Invalid or expired token",
            )),
        )
    })
}
