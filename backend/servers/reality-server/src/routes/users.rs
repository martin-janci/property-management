//! Portal user routes - separate from Property Management users.
//!
//! Supports SSO with Property Management via OAuth 2.0.

use crate::extractors::AuthenticatedUser;
use crate::handlers::users::{RegistrationResult, UserHandler};
use crate::state::AppState;
use axum::{
    extract::State,
    routing::{get, post, put},
    Json, Router,
};
use db::models::PortalUser;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Create users router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Public routes
        .route("/register", post(register))
        .route("/login", post(login))
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
            tracing::info!(user_id = %user.id, email = %user.email, "User registered");
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
            tracing::debug!(email = %req.email, "Login failed: {}", message);
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
    auth: AuthenticatedUser,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    // Extract the token from Authorization header
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|auth| auth.strip_prefix("Bearer "))
        .or_else(|| {
            // Fall back to cookie
            headers
                .get(axum::http::header::COOKIE)
                .and_then(|v| v.to_str().ok())
                .and_then(|cookies| {
                    cookies
                        .split(';')
                        .find(|c| c.trim().starts_with("portal_session="))
                        .map(|c| c.trim().strip_prefix("portal_session=").unwrap())
                })
        });

    if let Some(token) = token {
        // Invalidate the session
        if let Err(e) = state.session_service.invalidate_session(token).await {
            tracing::warn!(user_id = %auth.user_id, error = %e, "Failed to invalidate session");
        }
    }

    tracing::info!(user_id = %auth.user_id, "User logged out");

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
    auth: AuthenticatedUser,
) -> Result<Json<UserInfo>, (axum::http::StatusCode, String)> {
    let handler = UserHandler::new(state.portal_repo.clone());

    match handler.get_user(auth.user_id).await {
        Ok(Some(user)) => Ok(Json(user.into())),
        Ok(None) => Err((
            axum::http::StatusCode::NOT_FOUND,
            "User not found".to_string(),
        )),
        Err(e) => {
            tracing::error!(error = %e, user_id = %auth.user_id, "Failed to get user");
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
    auth: AuthenticatedUser,
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
        if name.len() > 100 {
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

    let handler = UserHandler::new(state.portal_repo.clone());

    match handler
        .update_profile(auth.user_id, req.name, req.profile_image_url, req.locale)
        .await
    {
        Ok(user) => {
            tracing::info!(user_id = %auth.user_id, "Profile updated");
            Ok(Json(user.into()))
        }
        Err(e) => {
            tracing::error!(error = %e, user_id = %auth.user_id, "Failed to update profile");
            Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update profile".to_string(),
            ))
        }
    }
}
