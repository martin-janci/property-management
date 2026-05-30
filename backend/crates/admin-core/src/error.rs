//! Error types for `admin-core`.
//!
//! These map cleanly to HTTP responses inside the `RequireCapability`
//! extractor; the body shape mirrors what the brainstorming session called
//! out for leak #21 (`{ error: "mfa_required" }` etc).

use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde::Serialize;
use thiserror::Error;

/// Top-level admin-core error. Each variant carries the HTTP status the
/// `RequireCapability` extractor will respond with.
#[derive(Debug, Error)]
pub enum AdminError {
    /// No `Authorization` header / token invalid.
    #[error("unauthenticated")]
    Unauthenticated,

    /// Authenticated, but not a `platform`-kind principal. Authorization is
    /// gated on principal_kind, never on a JWT role boolean alone.
    #[error("forbidden: not a platform principal")]
    NotPlatformPrincipal,

    /// Authenticated platform principal, but does not hold the requested
    /// capability (or the grant has expired / been revoked).
    #[error("forbidden: missing capability {0}")]
    MissingCapability(crate::Capability),

    /// Authenticated platform principal, but has never enrolled in MFA.
    /// The frontend routes to the enrollment screen.
    #[error("mfa not enrolled")]
    MfaNotEnrolled,

    /// Authenticated, but no MFA verification within `RECENT_MFA_WINDOW`.
    /// The frontend re-prompts for the user's TOTP and retries.
    #[error("mfa required")]
    MfaRequired(MfaRequired),

    /// Internal error talking to the database / repository layer.
    #[error("internal: {0}")]
    Internal(String),
}

/// Body shape for the `mfa_required` 401 response. The frontend reads this
/// and triggers the existing 2FA prompt component.
#[derive(Debug, Clone, Serialize)]
pub struct MfaRequired {
    pub error: &'static str,
    pub message: &'static str,
}

impl Default for MfaRequired {
    fn default() -> Self {
        Self {
            error: "mfa_required",
            message: "Recent MFA verification required for this admin action.",
        }
    }
}

/// Compact JSON shape for non-MFA errors.
#[derive(Debug, Serialize)]
struct ErrorBody<'a> {
    error: &'a str,
    message: String,
}

impl IntoResponse for AdminError {
    fn into_response(self) -> Response {
        match self {
            AdminError::Unauthenticated => (
                StatusCode::UNAUTHORIZED,
                Json(ErrorBody {
                    error: "unauthenticated",
                    message: "Missing or invalid authentication.".into(),
                }),
            )
                .into_response(),
            AdminError::NotPlatformPrincipal => (
                StatusCode::FORBIDDEN,
                Json(ErrorBody {
                    error: "not_platform_principal",
                    message: "Only platform principals may invoke admin endpoints.".into(),
                }),
            )
                .into_response(),
            AdminError::MissingCapability(cap) => (
                StatusCode::FORBIDDEN,
                Json(ErrorBody {
                    error: "missing_capability",
                    // P1-04: as_str() gives stable snake_case, not Rust Debug repr.
                    message: format!("Missing capability: {}", cap.as_str()),
                }),
            )
                .into_response(),
            AdminError::MfaNotEnrolled => (
                StatusCode::FORBIDDEN,
                Json(ErrorBody {
                    error: "mfa_not_enrolled",
                    message: "Platform principals must enroll in MFA before using capability-gated actions. Start enrollment from the admin UI.".into(),
                }),
            )
                .into_response(),
            AdminError::MfaRequired(body) => (StatusCode::UNAUTHORIZED, Json(body)).into_response(),
            AdminError::Internal(msg) => {
                tracing::error!(target: "admin_core", error = %msg, "admin_core internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody {
                        error: "internal",
                        message: "Internal server error.".into(),
                    }),
                )
                    .into_response()
            }
        }
    }
}
