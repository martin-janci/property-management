//! General in-app feedback routes (#3544, Epic 50 Story 50.4).
//!
//! The mobile `FeedbackManager` POSTs general feedback / bug reports to
//! `POST /api/v1/feedback`. Historically no backend server mounted that route,
//! so every mobile submission returned 404 (#3544). This module provides the
//! real mounted handler with the contract the mobile client already sends
//! (`frontend/apps/mobile/src/onboarding/types.ts` `FeedbackSubmission`).

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use common::errors::ErrorResponse;
use db::repositories::NewFeedback;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::state::AppState;

/// Create the general feedback router.
pub fn router() -> Router<AppState> {
    Router::new().route("/", post(submit_feedback))
}

/// Feedback submission request — matches the mobile `FeedbackSubmission`
/// contract. `camelCase` aliases map the client's `deviceInfo` / `appContext`
/// keys onto the snake_case fields; `type` is renamed explicitly.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackRequest {
    /// Category: bug | feature | improvement | question | other.
    #[serde(rename = "type")]
    pub feedback_type: String,
    /// Short summary.
    pub title: String,
    /// Free-text body.
    pub description: String,
    /// Optional reply-to address.
    #[serde(default)]
    pub email: Option<String>,
    /// Optional screenshot payload (data URI / object key).
    #[serde(default)]
    pub screenshot: Option<String>,
    /// Optional client device diagnostics.
    #[serde(default)]
    pub device_info: Option<JsonValue>,
    /// Optional client app context (current screen, building, session).
    #[serde(default)]
    pub app_context: Option<JsonValue>,
}

/// Feedback submission response. `id` is the persisted record id the mobile
/// client reads back (`submitFeedback` returns `{ success, id }`).
#[derive(Debug, Serialize, ToSchema)]
pub struct FeedbackResponse {
    pub id: Uuid,
}

/// Allowed feedback categories (mirrors the mobile `FeedbackType` union).
const ALLOWED_TYPES: [&str; 5] = ["bug", "feature", "improvement", "question", "other"];

/// Submit general in-app feedback / a bug report.
///
/// Requires a valid bearer token — feedback is attributed to the signed-in
/// user, mirroring the help-article feedback endpoint's auth gate.
#[utoipa::path(
    post,
    path = "/api/v1/feedback",
    request_body = FeedbackRequest,
    responses(
        (status = 201, description = "Feedback recorded", body = FeedbackResponse),
        (status = 400, description = "Invalid feedback payload"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Feedback"
)]
pub async fn submit_feedback(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(request): Json<FeedbackRequest>,
) -> Result<(StatusCode, Json<FeedbackResponse>), (StatusCode, Json<ErrorResponse>)> {
    let user_id = extract_user_id(&headers, &state)?;

    // Validate the category against the known client enum. Unknown values are a
    // client bug, not stored silently.
    let feedback_type = request.feedback_type.trim();
    if !ALLOWED_TYPES.contains(&feedback_type) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_FEEDBACK_TYPE",
                "type must be one of: bug, feature, improvement, question, other",
            )),
        ));
    }

    if request.title.trim().is_empty() || request.description.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_FEEDBACK",
                "title and description are required",
            )),
        ));
    }

    // Best-effort triage hint: the submitter's active building/org, if the
    // client reported one in `appContext.buildingId`.
    let organization_id = request
        .app_context
        .as_ref()
        .and_then(|ctx| ctx.get("buildingId"))
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok());

    let email = request
        .email
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let id = state
        .feedback_repo
        .create_feedback(NewFeedback {
            user_id,
            feedback_type,
            title: request.title.trim(),
            description: request.description.trim(),
            email,
            screenshot: request.screenshot.as_deref(),
            device_info: request.device_info.clone(),
            app_context: request.app_context.clone(),
            organization_id,
        })
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to record feedback");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to record feedback",
                )),
            )
        })?;

    Ok((StatusCode::CREATED, Json(FeedbackResponse { id })))
}

/// Extract and validate the signed-in user's id from the bearer token.
///
/// Mirrors `routes::help::extract_user_token`: feedback is a lightweight,
/// non-tenant endpoint, so it validates the access token directly rather than
/// going through the full tenant-scoped `AuthUser` extractor.
fn extract_user_id(
    headers: &axum::http::HeaderMap,
    state: &AppState,
) -> Result<Uuid, (StatusCode, Json<ErrorResponse>)> {
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

    let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Bearer token required")),
        )
    })?;

    let claims = state
        .jwt_service
        .validate_access_token(token)
        .map_err(|e| {
            tracing::debug!(error = %e, "Invalid access token");
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "INVALID_TOKEN",
                    "Invalid or expired token",
                )),
            )
        })?;

    claims.sub.parse::<Uuid>().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })
}
