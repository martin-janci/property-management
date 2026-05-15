//! Authentication extractor for Reality Server.
//!
//! Extracts authenticated user from session token (JWT in Authorization header or cookie).
//! Uses the SessionService to validate and retrieve session information.
//!
//! # D1.2 status
//!
//! `AuthenticatedUser` and `OptionalAuth` are NO LONGER consumed by any
//! route handler — every endpoint has migrated to
//! `api_core::extractors::RequestPrincipal` /
//! `api_core::extractors::OptionalRequestPrincipal`. The types remain here
//! for now as a safety net (a single revert restores them) and because the
//! `extract_session_token` helper is reused by the `users::logout` route to
//! pull a token out of either the Authorization header or the
//! `portal_session` cookie. A later cleanup pass can either delete the
//! types outright or repurpose `extract_session_token` into its own helper
//! module. Until then, the symbols are intentionally dead-code-allowed.

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use uuid::Uuid;

use crate::state::AppState;

/// Authenticated portal user extracted from session token.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// Portal user ID
    pub user_id: Uuid,
    /// User email
    pub email: String,
    /// User display name
    pub name: String,
}

impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Extract session token from Authorization header or cookie
        let session_token = extract_session_token(&parts.headers)
            .ok_or((StatusCode::UNAUTHORIZED, "Missing authentication token"))?;

        // Validate session and get user info
        let session_info = state
            .session_service
            .get_session(&session_token)
            .await
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid or expired session"))?;

        Ok(AuthenticatedUser {
            user_id: session_info.user_id,
            email: session_info.email,
            name: session_info.name,
        })
    }
}

/// Optional authentication extractor.
/// Returns Some(AuthenticatedUser) if authenticated, None otherwise.
#[derive(Debug, Clone)]
pub struct OptionalAuth(pub Option<AuthenticatedUser>);

impl FromRequestParts<AppState> for OptionalAuth {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        match AuthenticatedUser::from_request_parts(parts, state).await {
            Ok(user) => Ok(OptionalAuth(Some(user))),
            Err(_) => Ok(OptionalAuth(None)),
        }
    }
}

/// Extract session token from Authorization header or cookie.
fn extract_session_token(headers: &axum::http::HeaderMap) -> Option<String> {
    // Try Authorization header first (Bearer token)
    if let Some(auth) = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(token) = auth.strip_prefix("Bearer ") {
            return Some(token.to_string());
        }
    }

    // Fall back to cookie
    headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies
                .split(';')
                .find(|c| c.trim().starts_with("portal_session="))
                .map(|c| {
                    c.trim()
                        .strip_prefix("portal_session=")
                        .unwrap()
                        .to_string()
                })
        })
}
