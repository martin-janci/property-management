//! Portal user routes - separate from Property Management users.
//!
//! Supports SSO with Property Management via OAuth 2.0.
//! D1.2: handlers now use the unified `RequestPrincipal` extractor.

use crate::extractors::auth::extract_session_token;
use crate::handlers::users::{
    PasswordResetConfirmResult, PasswordResetRequestResult, RegistrationResult, UserHandler,
};
use crate::services::{delivery_decision, ResetDelivery};
use crate::state::AppState;
use api_core::extractors::RequestPrincipal;
use axum::{
    extract::State,
    routing::{get, post, put},
    Json, Router,
};
use db::models::user::Locale;
use db::models::PortalUser;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// H7 — `profile_image_url` ends up in `<img src=>` on the frontend. Reject
// `javascript:`, `data:`, `file:` URLs. Inlined locally (see H7 brief);
// swap for a shared `url_validator` module once it lands on main.
const MAX_PROFILE_URL_LEN: usize = 2048;

fn validate_image_or_link_url(s: &str) -> Result<(), String> {
    if s.chars().count() > MAX_PROFILE_URL_LEN {
        return Err(format!(
            "URL must be at most {} characters",
            MAX_PROFILE_URL_LEN
        ));
    }
    let parsed = url::Url::parse(s).map_err(|_| "Invalid URL".to_string())?;
    match parsed.scheme() {
        "https" | "http" => Ok(()),
        _ => Err("URL scheme must be http or https".to_string()),
    }
}

/// Create users router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Public routes
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/password-reset", post(request_password_reset))
        .route("/password-reset/confirm", post(confirm_password_reset))
        // Authenticated routes
        .route("/logout", post(logout))
        .route("/me", get(get_me))
        .route("/me", put(update_me))
    // OAuth/SSO routes are handled in sso.rs
}

/// Registration request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    /// Email address
    pub email: String,
    /// Password (min 8 chars, 1 uppercase, 1 number)
    pub password: String,
    /// Display name
    pub name: String,
}

/// Registration response.
#[derive(Debug, Serialize, ToSchema)]
pub struct RegisterResponse {
    /// Success message
    pub message: String,
    /// User ID
    pub user_id: String,
}

/// Login request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    /// Email address
    pub email: String,
    /// Password
    pub password: String,
}

/// Login response.
#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    /// Session token
    pub token: String,
    /// Token expiration (ISO 8601)
    pub expires_at: String,
    /// User info
    pub user: UserInfo,
}

/// User info in responses.
#[derive(Debug, Serialize, ToSchema)]
pub struct UserInfo {
    /// User ID
    pub id: String,
    /// Email
    pub email: String,
    /// Name
    pub name: String,
    /// Profile image URL
    pub profile_image_url: Option<String>,
    /// Is linked to PM account
    pub is_linked_to_pm: bool,
}

impl From<PortalUser> for UserInfo {
    fn from(user: PortalUser) -> Self {
        Self {
            id: user.id.to_string(),
            email: user.email,
            name: user.name,
            profile_image_url: user.profile_image_url,
            is_linked_to_pm: user.pm_user_id.is_some(),
        }
    }
}

/// Update profile request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateProfileRequest {
    /// Display name
    pub name: Option<String>,
    /// Profile image URL
    pub profile_image_url: Option<String>,
    /// Locale
    pub locale: Option<String>,
}

/// Register a new portal user.
#[utoipa::path(
    post,
    path = "/api/v1/users/register",
    tag = "Users",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "Registration successful", body = RegisterResponse),
        (status = 400, description = "Invalid input"),
        (status = 409, description = "Email already exists")
    )
)]
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(axum::http::StatusCode, Json<RegisterResponse>), (axum::http::StatusCode, String)> {
    let handler = UserHandler::new(state.portal_repo.clone());

    match handler.register(&req.email, &req.password, &req.name).await {
        RegistrationResult::Success(user) => {
            tracing::info!(user_id = %user.id, "User registered");
            Ok((
                axum::http::StatusCode::CREATED,
                Json(RegisterResponse {
                    message:
                        "Registration successful. Please check your email to verify your account."
                            .to_string(),
                    user_id: user.id.to_string(),
                }),
            ))
        }
        RegistrationResult::EmailExists => Err((
            axum::http::StatusCode::CONFLICT,
            "An account with this email already exists".to_string(),
        )),
        RegistrationResult::InvalidEmail => Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid email format".to_string(),
        )),
        RegistrationResult::WeakPassword(issues) => Err((
            axum::http::StatusCode::BAD_REQUEST,
            format!("Password requirements not met: {}", issues.join(", ")),
        )),
        RegistrationResult::CryptoError(e) => {
            tracing::error!(error = %e, "Registration cryptographic error");
            Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Registration failed".to_string(),
            ))
        }
        RegistrationResult::DatabaseError(e) => {
            tracing::error!(error = %e, "Registration database error");
            Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Registration failed".to_string(),
            ))
        }
    }
}

/// Login with email/password.
#[utoipa::path(
    post,
    path = "/api/v1/users/login",
    tag = "Users",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials")
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (axum::http::StatusCode, String)> {
    let handler = UserHandler::new(state.portal_repo.clone());

    match handler.login(&req.email, &req.password).await {
        Ok(user) => {
            // Create session token
            let token = match state.session_service.create_mobile_session(user.id).await {
                Ok(token) => token,
                Err(e) => {
                    tracing::error!(error = %e, "Failed to create session");
                    return Err((
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        "Login failed".to_string(),
                    ));
                }
            };

            let expires_at = chrono::Utc::now() + chrono::Duration::days(7);

            tracing::info!(user_id = %user.id, "User logged in");

            Ok(Json(LoginResponse {
                token,
                expires_at: expires_at.to_rfc3339(),
                user: user.into(),
            }))
        }
        Err(message) => {
            tracing::debug!(
                email_hash = %common::email_log_hash(&req.email),
                "Login failed: {}",
                message
            );
            Err((axum::http::StatusCode::UNAUTHORIZED, message.to_string()))
        }
    }
}

/// Logout (invalidate session).
#[utoipa::path(
    post,
    path = "/api/v1/users/logout",
    tag = "Users",
    responses(
        (status = 204, description = "Logged out"),
        (status = 401, description = "Not authenticated")
    )
)]
pub async fn logout(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    principal: RequestPrincipal,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    // Extract the token from Authorization header or portal_session cookie.
    // Logout is the one place we still need the raw token (to compute the
    // session-row hash for invalidation); RequestPrincipal carries the
    // user id but not the token string.
    if let Some(token) = extract_session_token(&headers) {
        // Invalidate the session
        if let Err(e) = state.session_service.invalidate_session(&token).await {
            tracing::warn!(user_id = %principal.user_id, error = %e, "Failed to invalidate session");
        }
    }

    tracing::info!(user_id = %principal.user_id, "User logged out");

    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// Get current user profile.
#[utoipa::path(
    get,
    path = "/api/v1/users/me",
    tag = "Users",
    responses(
        (status = 200, description = "User profile", body = UserInfo),
        (status = 401, description = "Not authenticated")
    )
)]
pub async fn get_me(
    State(state): State<AppState>,
    principal: RequestPrincipal,
) -> Result<Json<UserInfo>, (axum::http::StatusCode, String)> {
    let handler = UserHandler::new(state.portal_repo.clone());

    match handler.get_user(principal.user_id).await {
        Ok(Some(user)) => Ok(Json(user.into())),
        Ok(None) => Err((
            axum::http::StatusCode::NOT_FOUND,
            "User not found".to_string(),
        )),
        Err(e) => {
            tracing::error!(error = %e, user_id = %principal.user_id, "Failed to get user");
            Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get profile".to_string(),
            ))
        }
    }
}

/// Update current user profile.
#[utoipa::path(
    put,
    path = "/api/v1/users/me",
    tag = "Users",
    request_body = UpdateProfileRequest,
    responses(
        (status = 200, description = "Profile updated", body = UserInfo),
        (status = 401, description = "Not authenticated"),
        (status = 400, description = "Invalid input")
    )
)]
pub async fn update_me(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<UserInfo>, (axum::http::StatusCode, String)> {
    // Validate name if provided
    if let Some(ref name) = req.name {
        if name.trim().is_empty() {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                "Name cannot be empty".to_string(),
            ));
        }
        if name.chars().count() > 100 {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                "Name must be less than 100 characters".to_string(),
            ));
        }
    }

    // Validate locale if provided
    if let Some(ref locale) = req.locale {
        let valid_locales = ["sk", "cs", "de", "en"];
        if !valid_locales.contains(&locale.as_str()) {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                format!("Locale must be one of: {}", valid_locales.join(", ")),
            ));
        }
    }

    // H7: validate profile image URL — reject `javascript:`, `data:`, `file:`
    // and enforce a length cap. Use a generic error so the rejected URL is
    // NOT echoed back (reflected-XSS guard).
    if let Some(ref purl) = req.profile_image_url {
        if !purl.is_empty() && validate_image_or_link_url(purl).is_err() {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                "profile_image_url must be a http(s) URL (max 2048 chars)".to_string(),
            ));
        }
    }

    let handler = UserHandler::new(state.portal_repo.clone());

    match handler
        .update_profile(
            principal.user_id,
            req.name,
            req.profile_image_url,
            req.locale,
        )
        .await
    {
        Ok(user) => {
            tracing::info!(user_id = %principal.user_id, "Profile updated");
            Ok(Json(user.into()))
        }
        Err(e) => {
            tracing::error!(error = %e, user_id = %principal.user_id, "Failed to update profile");
            Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update profile".to_string(),
            ))
        }
    }
}

// ===========================================================================
// Password reset (UC-44.3)
// ===========================================================================

/// Body for `POST /api/v1/users/password-reset`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct PasswordResetRequestBody {
    pub email: String,
}

/// Response for the request flow. The body always reads "ok" — the
/// response is intentionally identical regardless of whether the email
/// exists, to avoid letting attackers enumerate accounts.
#[derive(Debug, Serialize, ToSchema)]
pub struct PasswordResetRequestResponse {
    pub message: String,
}

/// Body for `POST /api/v1/users/password-reset/confirm`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct PasswordResetConfirmBody {
    pub token: String,
    pub new_password: String,
}

/// Response for the confirm flow.
#[derive(Debug, Serialize, ToSchema)]
pub struct PasswordResetConfirmResponse {
    pub message: String,
}

/// Issue a password-reset token and email the reset link to the user.
///
/// On 200 the endpoint returns the same generic body whether or not the account
/// exists, so callers can't distinguish "user found" from "user not found" by
/// status or timing alone. When a real email transport is configured the reset
/// link is delivered; in development builds it falls back to logging the link so
/// the flow stays testable. If NO transport is configured in a release build the
/// endpoint returns 503 rather than falsely claiming a link was sent.
#[utoipa::path(
    post,
    path = "/api/v1/users/password-reset",
    tag = "Users",
    request_body = PasswordResetRequestBody,
    responses(
        (status = 200, description = "Reset email sent (or user does not exist)",
                       body = PasswordResetRequestResponse),
        (status = 400, description = "Invalid email format"),
        (status = 503, description = "Email transport not configured — reset unavailable")
    )
)]
pub async fn request_password_reset(
    State(state): State<AppState>,
    Json(req): Json<PasswordResetRequestBody>,
) -> Result<Json<PasswordResetRequestResponse>, (axum::http::StatusCode, String)> {
    if !UserHandler::validate_email(&req.email) {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid email format".to_string(),
        ));
    }

    // Guard: decide how to answer BEFORE issuing a token. The decision uses
    // global server state only (transport configured? dev fallback allowed?),
    // so it never leaks whether the account exists.
    //
    // This closes the prod bug where the endpoint always returned
    // "a reset link has been sent" even though no email transport was wired —
    // the token was persisted, then discarded, and the user was permanently
    // stuck with no way to recover their account.
    let mailer = state.password_reset_mailer.clone();
    match delivery_decision(mailer.is_configured(), cfg!(debug_assertions)) {
        ResetDelivery::Unavailable => {
            // No transport and no dev fallback (release build): refuse rather
            // than lie. Enumeration-safe — the answer does not depend on the
            // email argument.
            tracing::error!(
                "Password reset requested but no email transport is configured; \
                 refusing to falsely claim a link was sent (set SMTP_* to enable)"
            );
            return Err((
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                "Password reset is temporarily unavailable. Please contact support.".to_string(),
            ));
        }
        // Send (real transport) and LogFallback (dev) both issue the token and
        // hand it to the mailer; the mailer decides whether it actually leaves
        // the process.
        ResetDelivery::Send | ResetDelivery::LogFallback => {}
    }

    let handler = UserHandler::new(state.portal_repo.clone());
    match handler
        .request_password_reset(&state.portal_password_reset_repo, &req.email)
        .await
    {
        Ok(PasswordResetRequestResult::Sent {
            plaintext_token,
            name,
            locale,
        }) => {
            let locale = Locale::parse(&locale);
            match mailer
                .send_reset_link(&req.email, &name, &plaintext_token, locale)
                .await
            {
                Ok(()) => {
                    tracing::info!(
                        email_hash = %common::email_log_hash(&req.email),
                        "Password reset link dispatched"
                    );
                }
                Err(e) => {
                    // Delivery failed at the transport. Keep the response
                    // enumeration-safe (generic 200) but log loudly so
                    // operators notice — we no longer silently drop the token.
                    tracing::error!(
                        error = %e,
                        email_hash = %common::email_log_hash(&req.email),
                        "Failed to send password reset email"
                    );
                }
            }
        }
        Ok(PasswordResetRequestResult::UserNotFound) => {
            tracing::debug!(
                email_hash = %common::email_log_hash(&req.email),
                "Password reset requested for unknown email"
            );
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to issue password reset token");
            // Still return 200 to avoid leaking the failure mode.
        }
    }

    Ok(Json(PasswordResetRequestResponse {
        message: "If an account exists for this email, a reset link has been sent.".to_string(),
    }))
}

/// Verify a reset token and update the password.
#[utoipa::path(
    post,
    path = "/api/v1/users/password-reset/confirm",
    tag = "Users",
    request_body = PasswordResetConfirmBody,
    responses(
        (status = 200, description = "Password updated", body = PasswordResetConfirmResponse),
        (status = 400, description = "Invalid token, expired token, or weak password")
    )
)]
pub async fn confirm_password_reset(
    State(state): State<AppState>,
    Json(req): Json<PasswordResetConfirmBody>,
) -> Result<Json<PasswordResetConfirmResponse>, (axum::http::StatusCode, String)> {
    if req.token.trim().is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "token is required".to_string(),
        ));
    }

    let handler = UserHandler::new(state.portal_repo.clone());
    match handler
        .confirm_password_reset(
            &state.portal_password_reset_repo,
            &req.token,
            &req.new_password,
        )
        .await
    {
        Ok(PasswordResetConfirmResult::Success) => Ok(Json(PasswordResetConfirmResponse {
            message: "Password updated successfully".to_string(),
        })),
        Ok(PasswordResetConfirmResult::InvalidToken) => Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid or already-used reset token".to_string(),
        )),
        Ok(PasswordResetConfirmResult::Expired) => Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Reset token has expired — request a new one".to_string(),
        )),
        Ok(PasswordResetConfirmResult::PasswordTooWeak(issues)) => {
            Err((axum::http::StatusCode::BAD_REQUEST, issues.join("; ")))
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to confirm password reset");
            Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to reset password".to_string(),
            ))
        }
    }
}
