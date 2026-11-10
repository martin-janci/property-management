//! Registration & email-verification routes.
//!
//! Split out of `routes::auth` as a mechanical, behavior-preserving refactor
//! (Story 1.1). Shared helpers (rate limiting, `AuthService`, token/cookie
//! utilities), models, and Axum/serde/utoipa imports resolve via `use super::*`
//! against the parent `auth` module — a child module can see its parent's
//! private items, so no helper visibility had to change.

use super::*;
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
///
/// Intentionally generic and identical for both a fresh registration and an
/// attempt to register an already-existing email (#956). It carries no
/// account-specific data (notably no user id), so the response cannot be used
/// to enumerate registered accounts.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisterResponse {
    /// Generic success message
    pub message: String,
}

/// Generic, account-agnostic message returned by `register` regardless of
/// whether the email was already registered. Keeps the success and
/// already-exists paths byte-for-byte identical (#956).
const REGISTER_GENERIC_MESSAGE: &str = "Check your email to verify your account";

/// Register endpoint.
#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    tag = "Authentication",
    request_body = RegisterRequest,
    responses(
        // A 201 with a generic body is returned whether or not the email is
        // already registered, to avoid account enumeration (#956).
        (status = 201, description = "Registration accepted", body = RegisterResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse)
    )
)]
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<impl IntoResponse, AuthError> {
    // Validate email format
    if !AuthService::validate_email(&req.email) {
        return Err(err_response(
            StatusCode::BAD_REQUEST,
            "INVALID_EMAIL",
            "Invalid email format",
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
        return Err(err_response(
            StatusCode::BAD_REQUEST,
            "INVALID_NAME",
            "Name cannot be empty",
        ));
    }

    // Check if email already exists. We must NOT signal this back to the
    // caller (no 409 / EMAIL_EXISTS) — a distinct response for an existing
    // address is an account-enumeration oracle. Instead we short-circuit with
    // the exact same generic 201 a fresh registration returns, and notify the
    // real account holder out-of-band (logged here; an out-of-band "someone
    // tried to register with your email" notification is a future enhancement).
    match state.user_repo.email_exists(&req.email).await {
        Ok(true) => {
            tracing::info!(
                email_hash = %common::email_log_hash(&req.email),
                "Registration attempted for an already-registered email; returning generic response"
            );
            return Ok((
                StatusCode::CREATED,
                Json(RegisterResponse {
                    message: REGISTER_GENERIC_MESSAGE.to_string(),
                }),
            ));
        }
        Ok(false) => {}
        Err(e) => {
            tracing::error!(error = %e, "Database error checking email");
            return Err(err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                "Failed to check email",
            ));
        }
    }

    // Hash password
    let password_hash = match state.auth_service.hash_password(&req.password) {
        Ok(hash) => hash,
        Err(e) => {
            tracing::error!(error = %e, "Failed to hash password");
            return Err(err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "Failed to process password",
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
            tracing::error!(
                error = %e,
                email_hash = %common::email_log_hash(&req.email),
                "Failed to create user"
            );
            return Err(err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                "Failed to create account",
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
        "User registered successfully"
    );

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            message: REGISTER_GENERIC_MESSAGE.to_string(),
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
#[serde(rename_all = "camelCase")]
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
) -> Result<Json<VerifyEmailResponse>, AuthError> {
    // Hash the token to look it up
    let token_hash = state.auth_service.hash_token(&query.token);

    // Find the token
    let verification_token = match state.user_repo.find_verification_token(&token_hash).await {
        Ok(Some(token)) => token,
        Ok(None) => {
            return Err(err_response(
                StatusCode::BAD_REQUEST,
                "INVALID_TOKEN",
                "This verification link is invalid or has already been used",
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error finding verification token");
            return Err(err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                "Failed to verify token",
            ));
        }
    };

    // Check if token is expired
    if verification_token.is_expired() {
        return Err(err_response(
            StatusCode::BAD_REQUEST,
            "TOKEN_EXPIRED",
            "This verification link has expired. Please request a new one.",
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
            tracing::info!(user_id = %user.id, "Email verified");
            Ok(Json(VerifyEmailResponse {
                message: "Your email has been verified. You can now log in.".to_string(),
            }))
        }
        Ok(None) => Err(err_response(
            StatusCode::BAD_REQUEST,
            "USER_NOT_FOUND",
            "User account not found or already verified",
        )),
        Err(e) => {
            tracing::error!(error = %e, "Failed to verify email");
            Err(err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                "Failed to verify email",
            ))
        }
    }
}

// ==================== Resend Verification (Story 1.1) ====================

/// Decide how the resend-verification handler reacts to the result of
/// invalidating a user's outstanding email-verification tokens.
///
/// Issuing a fresh verification token while a previous one remains valid widens
/// the window in which a stale link can be used, so a DB failure while
/// invalidating the old tokens must be surfaced (propagated as a 500) rather
/// than swallowed — matching the error convention of the surrounding handlers.
fn invalidation_outcome(result: Result<u64, sqlx::Error>) -> Result<(), AuthError> {
    result.map(|_| ()).map_err(|e| {
        tracing::error!(error = %e, "Failed to invalidate old verification tokens");
        err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DATABASE_ERROR",
            "Failed to resend verification email",
        )
    })
}

/// Resend verification request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ResendVerificationRequest {
    /// Email address
    pub email: String,
}

/// Resend verification response.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
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
        (status = 429, description = "Too many requests for this email", body = ErrorResponse),
    )
)]
pub async fn resend_verification(
    State(state): State<AppState>,
    Json(req): Json<ResendVerificationRequest>,
) -> Result<Json<ResendVerificationResponse>, AuthError> {
    // Throttle per email before doing any work: this endpoint sends an email
    // and rotates the recipient's verification token on every call, so it is a
    // mailbomb / token-clobber surface (see limiter docs above).
    if !resend_verification_rate_allowed(&req.email) {
        tracing::warn!(
            email_hash = %common::email_log_hash(&req.email),
            "resend-verification blocked due to rate limiting"
        );
        return Err(email_dispatch_rate_limited());
    }

    // Always return success to prevent email enumeration
    let response = ResendVerificationResponse {
        message:
            "If an unverified account exists with this email, a verification link has been sent."
                .to_string(),
    };

    // Try to find user and send email
    match state.user_repo.find_by_email(&req.email).await {
        Ok(Some(user)) if user.status == "pending" => {
            // Invalidate old verification tokens before issuing a new one. A
            // failure here must not be swallowed: proceeding would issue a
            // fresh token while leaving the prior token(s) valid.
            invalidation_outcome(state.user_repo.invalidate_user_tokens(user.id).await)?;

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
                return Ok(Json(response));
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
            tracing::debug!(
                email_hash = %common::email_log_hash(&req.email),
                "Resend verification request for non-pending account"
            );
        }
    }

    Ok(Json(response))
}

#[cfg(test)]
mod resend_verification_token_tests {
    use super::invalidation_outcome;
    use axum::http::StatusCode;

    /// Regression: a DB failure while invalidating a user's existing
    /// email-verification tokens must NOT be silently swallowed. Before the
    /// fix the handler did `let _ = state.user_repo.invalidate_user_tokens(..)`,
    /// so a failed invalidation left the prior verification token(s) valid
    /// while a brand-new token was issued (security issue: old verification
    /// tokens must be invalidated when a new one is issued). The invalidation
    /// outcome must surface as an error so the handler aborts instead of
    /// proceeding as if invalidation succeeded.
    #[test]
    fn invalidation_db_error_is_surfaced_not_swallowed() {
        let outcome = invalidation_outcome(Err(sqlx::Error::PoolClosed));
        assert!(
            outcome.is_err(),
            "a DB failure while invalidating old verification tokens must be surfaced, not swallowed"
        );
        let (status, _) = outcome.unwrap_err();
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    /// A successful invalidation lets the handler proceed to issue the new token.
    #[test]
    fn invalidation_success_proceeds() {
        assert!(invalidation_outcome(Ok(2)).is_ok());
    }
}
