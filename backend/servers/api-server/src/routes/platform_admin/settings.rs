//! Platform settings handlers.
//!
//! Backs the admin-web `/admin/platform` page. These are global,
//! operator-controlled settings (never tenant-facing) persisted in the
//! `platform_settings` singleton table (migration 00228).
//!
//! Both handlers are gated by the platform-admin capability middleware
//! (`RequireCapability(SiteSettingsWrite)` on the route) plus the
//! [`extract_super_admin_token`] JWT-role check as defence in depth.

use admin_core::RequireCapability;
use axum::{extract::State, http::StatusCode, Json};
use common::errors::ErrorResponse;
use db::models::platform_admin::PlatformSettings;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::extract_super_admin_token;
use crate::state::AppState;

/// Platform settings payload.
///
/// The JSON keys are the dotted keys used by the admin-web `SettingsForm`
/// (`platform.maintenance_mode`, …), so the response round-trips directly into
/// the form and the request accepts what the form submits.
#[derive(Debug, Serialize, ToSchema)]
pub struct PlatformSettingsResponse {
    /// Whether the platform is in maintenance mode.
    #[serde(rename = "platform.maintenance_mode")]
    pub maintenance_mode: bool,
    /// Whether public sign-up is enabled.
    #[serde(rename = "platform.signup_enabled")]
    pub signup_enabled: bool,
    /// Operator support contact email.
    #[serde(rename = "platform.support_email")]
    pub support_email: String,
}

impl From<PlatformSettings> for PlatformSettingsResponse {
    fn from(s: PlatformSettings) -> Self {
        Self {
            maintenance_mode: s.maintenance_mode,
            signup_enabled: s.signup_enabled,
            support_email: s.support_email,
        }
    }
}

/// `PATCH` request body. Every field is optional — omitted fields are left
/// unchanged (partial update).
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdatePlatformSettingsRequest {
    /// Whether the platform is in maintenance mode.
    #[serde(rename = "platform.maintenance_mode", default)]
    pub maintenance_mode: Option<bool>,
    /// Whether public sign-up is enabled.
    #[serde(rename = "platform.signup_enabled", default)]
    pub signup_enabled: Option<bool>,
    /// Operator support contact email.
    #[serde(rename = "platform.support_email", default)]
    pub support_email: Option<String>,
}

fn db_error(context: &str) -> impl Fn(sqlx::Error) -> (StatusCode, Json<ErrorResponse>) + '_ {
    move |e| {
        tracing::error!(error = %e, "{context}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", context)),
        )
    }
}

/// Get the current platform settings.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/settings",
    responses(
        (status = 200, description = "Platform settings retrieved", body = PlatformSettingsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Settings"
)]
pub async fn get_platform_settings(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<PlatformSettingsResponse>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let settings = state
        .platform_admin_repo
        .get_platform_settings()
        .await
        .map_err(db_error("Failed to read platform settings"))?;

    Ok(Json(settings.into()))
}

/// Update (patch) the platform settings.
#[utoipa::path(
    patch,
    path = "/api/v1/platform-admin/settings",
    request_body = UpdatePlatformSettingsRequest,
    responses(
        (status = 200, description = "Platform settings updated", body = PlatformSettingsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Settings"
)]
pub async fn update_platform_settings(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(request): Json<UpdatePlatformSettingsRequest>,
) -> Result<Json<PlatformSettingsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (user_id, _email) = extract_super_admin_token(&headers, &state)?;

    let settings = state
        .platform_admin_repo
        .update_platform_settings(
            request.maintenance_mode,
            request.signup_enabled,
            request.support_email,
            user_id,
        )
        .await
        .map_err(db_error("Failed to update platform settings"))?;

    tracing::info!(actor = %user_id, "Platform settings updated");

    Ok(Json(settings.into()))
}
