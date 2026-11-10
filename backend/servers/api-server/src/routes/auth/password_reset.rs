//! Forgot-password & reset-password routes.
//!
//! Split out of `routes::auth` as a mechanical, behavior-preserving refactor
//! (Story 1.4). Shared helpers (rate limiting, `AuthService`, token/cookie
//! utilities), models, and Axum/serde/utoipa imports resolve via `use super::*`
//! against the parent `auth` module — a child module can see its parent's
//! private items, so no helper visibility had to change.

use super::*;
// ==================== Forgot Password (Story 1.4) ====================

/// Forgot password request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ForgotPasswordRequest {
    /// Email address
    pub email: String,
}

/// Forgot password response.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ForgotPasswordResponse {
    /// Success message (always same to prevent enumeration)
    pub message: String,
}

/// Decide how `forgot_password` reacts to the result of invalidating a user's
/// outstanding password-reset tokens.
///
/// Issuing a fresh reset token while a previous one is still valid leaves
/// multiple live reset links for the same account, so a DB failure while
/// invalidating the old tokens must be surfaced (propagated as a 500) rather
/// than swallowed — matching the error convention of the surrounding handlers
/// and the sibling `/resend-verification` flow
/// (`registration::invalidation_outcome`).
fn invalidation_outcome(result: Result<u64, sqlx::Error>) -> Result<(), AuthError> {
    result.map(|_| ()).map_err(|e| {
        tracing::error!(error = %e, "Failed to invalidate old password reset tokens");
        err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DATABASE_ERROR",
            "Failed to process password reset request",
        )
    })
}

/// Forgot password endpoint - initiates password reset.
#[utoipa::path(
    post,
    path = "/api/v1/auth/forgot-password",
    tag = "Authentication",
    request_body = ForgotPasswordRequest,
    responses(
        (status = 200, description = "Password reset email sent (if account exists)", body = ForgotPasswordResponse),
        (status = 429, description = "Too many requests for this email", body = ErrorResponse),
    )
)]
pub async fn forgot_password(
    State(state): State<AppState>,
    Json(req): Json<ForgotPasswordRequest>,
) -> Result<Json<ForgotPasswordResponse>, AuthError> {
    // Throttle per email before doing any work: this endpoint sends an email
    // and invalidates the recipient's outstanding reset token on every call, so
    // it is a mailbomb / token-clobber surface (see limiter docs above).
    if !forgot_password_rate_allowed(&req.email) {
        tracing::warn!(
            email_hash = %common::email_log_hash(&req.email),
            "forgot-password blocked due to rate limiting"
        );
        return Err(email_dispatch_rate_limited());
    }

    // Always return success to prevent email enumeration
    let response = ForgotPasswordResponse {
        message: "If an account exists with this email, a password reset link has been sent."
            .to_string(),
    };

    // Try to find user and send email
    match state.user_repo.find_by_email(&req.email).await {
        Ok(Some(user)) if user.status == "active" => {
            // Invalidate any existing reset tokens for this user before issuing
            // a new one. A DB failure here must NOT be swallowed: proceeding
            // would mint a fresh reset token while leaving the prior token(s)
            // valid, so the account would have multiple live reset links at
            // once. Surface it as a 500 so the handler aborts instead of
            // issuing a new token alongside stale ones.
            invalidation_outcome(
                state
                    .password_reset_repo
                    .invalidate_user_tokens(user.id)
                    .await,
            )?;

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
                return Ok(Json(response));
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
                user_id = %user.id,
                status = %user.status,
                "Password reset requested for non-active account"
            );
        }
        Ok(None) => {
            // User not found - don't reveal this
            tracing::debug!(
                email_hash = %common::email_log_hash(&req.email),
                "Password reset requested for unknown email"
            );
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error finding user for password reset");
        }
    }

    Ok(Json(response))
}

// ==================== Reset Password (Story 1.4) ====================

/// Reset password request.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResetPasswordRequest {
    /// Reset token from email
    pub token: String,
    /// New password (min 8 characters, 1 uppercase, 1 number)
    pub new_password: String,
}

/// Reset password response.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
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
) -> Result<Json<ResetPasswordResponse>, AuthError> {
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
            return Err(err_response(
                StatusCode::BAD_REQUEST,
                "INVALID_TOKEN",
                "This password reset link is invalid or has already been used",
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error finding password reset token");
            return Err(err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                "Failed to validate reset token",
            ));
        }
    };

    // Check if token is expired
    if reset_token.is_expired() {
        return Err(err_response(
            StatusCode::BAD_REQUEST,
            "TOKEN_EXPIRED",
            "This password reset link has expired. Please request a new one.",
        ));
    }

    // Find the user
    let user = match state.user_repo.find_by_id(reset_token.user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Err(err_response(
                StatusCode::BAD_REQUEST,
                "USER_NOT_FOUND",
                "User account no longer exists",
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error finding user");
            return Err(err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                "Failed to reset password",
            ));
        }
    };

    // Check if user is still active
    if user.status != "active" {
        return Err(err_response(
            StatusCode::BAD_REQUEST,
            "ACCOUNT_INACTIVE",
            "Cannot reset password for inactive account",
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
                return Err(err_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "AUTH_POLICY_ERROR",
                    "Failed to validate password",
                ));
            }
        }
    }

    // Hash new password
    let password_hash = match state.auth_service.hash_password(&req.new_password) {
        Ok(hash) => hash,
        Err(e) => {
            tracing::error!(error = %e, "Failed to hash new password");
            return Err(err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "Failed to process password",
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
        return Err(err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DATABASE_ERROR",
            "Failed to update password",
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

// ==================== Change Password (Story 1.4, authenticated) ====================

/// Change password request (authenticated user).
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    /// The user's current password
    pub current_password: String,
    /// New password (min 8 characters, 1 uppercase, 1 number)
    pub new_password: String,
}

/// Change password response.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordResponse {
    /// Success message
    pub message: String,
}

/// Change password endpoint for the signed-in user.
///
/// ppt-web's `/settings/password` page has always called this route, but it
/// was never mounted, so every password change 404'd.
#[utoipa::path(
    post,
    path = "/api/v1/auth/password/change",
    tag = "Authentication",
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Password changed", body = ChangePasswordResponse),
        (status = 400, description = "Password does not meet requirements", body = ErrorResponse),
        (status = 401, description = "Not authenticated or current password wrong", body = ErrorResponse),
        (status = 422, description = "Password violates org policy", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn change_password(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<Json<ChangePasswordResponse>, AuthError> {
    let user_id = authenticated_user_id(&state, &headers)?;

    let user = match state.user_repo.find_by_id(user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Err(err_response(
                StatusCode::UNAUTHORIZED,
                "INVALID_TOKEN",
                "User account no longer exists",
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error finding user");
            return Err(err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                "Failed to change password",
            ));
        }
    };

    if user.status != "active" {
        return Err(err_response(
            StatusCode::FORBIDDEN,
            "ACCOUNT_INACTIVE",
            "Cannot change password for an inactive account",
        ));
    }

    // Verify the current password first. The same generic code as login is
    // returned so the page maps it to "current password is wrong".
    match state
        .auth_service
        .verify_password(&req.current_password, &user.password_hash)
    {
        Ok(true) => {}
        Ok(false) => {
            return Err(err_response(
                StatusCode::UNAUTHORIZED,
                "INVALID_CREDENTIALS",
                "Current password is incorrect",
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Password verification error");
            return Err(err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "Failed to verify password",
            ));
        }
    }

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
                ErrorResponse::new("WEAK_PASSWORD", "Password does not meet requirements")
                    .with_details(details),
            ),
        ));
    }

    // Per-org password policy (same gate as reset_password).
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
                return Err(err_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "AUTH_POLICY_ERROR",
                    "Failed to validate password",
                ));
            }
        }
    }

    let password_hash = match state.auth_service.hash_password(&req.new_password) {
        Ok(hash) => hash,
        Err(e) => {
            tracing::error!(error = %e, "Failed to hash new password");
            return Err(err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "Failed to process password",
            ));
        }
    };

    if let Err(e) = state
        .user_repo
        .update_password(user.id, &password_hash)
        .await
    {
        tracing::error!(error = %e, user_id = %user.id, "Failed to update password");
        return Err(err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DATABASE_ERROR",
            "Failed to update password",
        ));
    }

    // A password change is a security event: any pre-existing session (incl. a
    // potential attacker's, which is the usual reason a signed-in user changes
    // their password) must be invalidated. Mirror `reset_password`'s teardown,
    // but preserve the caller's OWN current session so the user is not
    // immediately logged out of the tab they changed their password from —
    // resolving the current session cookie-first / header-fallback the same way
    // `revoke_all_sessions` does. Header/cookie-less callers (access-token only)
    // resolve to `None`, which revokes every session — the safe default.
    let except_id = resolve_current_session_id(&state, &headers, user.id).await;
    if let Err(e) = state
        .session_repo
        .revoke_all_user_tokens(user.id, except_id)
        .await
    {
        // Log-and-continue, consistent with `reset_password`: the password was
        // already changed successfully, so surfacing a 500 here would mislead
        // the caller into thinking the change failed.
        tracing::error!(error = %e, user_id = %user.id, "Failed to revoke sessions after password change");
    }

    tracing::info!(user_id = %user.id, "Password changed by user");

    Ok(Json(ChangePasswordResponse {
        message: "Password has been changed.".to_string(),
    }))
}

#[cfg(test)]
mod forgot_password_token_tests {
    use super::invalidation_outcome;
    use axum::http::StatusCode;

    /// Regression: a DB failure while invalidating a user's existing
    /// password-reset tokens must NOT be silently swallowed. Before the fix
    /// `forgot_password` did `let _ = state.password_reset_repo
    /// .invalidate_user_tokens(..).await`, so a failed invalidation left the
    /// prior reset token(s) valid while a brand-new token was issued — leaving
    /// multiple live reset links for one account. The invalidation outcome must
    /// surface as an error so the handler aborts instead of proceeding as if
    /// invalidation succeeded.
    #[test]
    fn invalidation_db_error_is_surfaced_not_swallowed() {
        let outcome = invalidation_outcome(Err(sqlx::Error::PoolClosed));
        assert!(
            outcome.is_err(),
            "a DB failure while invalidating old reset tokens must be surfaced, not swallowed"
        );
        let (status, _) = outcome.unwrap_err();
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    /// A successful invalidation lets the handler proceed to issue the new token.
    #[test]
    fn invalidation_success_proceeds() {
        assert!(invalidation_outcome(Ok(1)).is_ok());
    }
}
