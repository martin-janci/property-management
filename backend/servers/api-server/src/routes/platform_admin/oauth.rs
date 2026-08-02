//! Platform-admin OAuth token-usage analytics endpoint (Epic 10A — data audit,
//! follow-up #2628).
//!
//! Exposes the read side of the `oauth_token_events` analytics data layer that
//! landed in PR #2526: the per-client and whole-ecosystem token-usage rollups
//! that power the platform-admin ecosystem-health dashboard. The producer that
//! fills the table lives on the OAuth token lifecycle path
//! (`services::oauth`), and a daily scheduler prune bounds its growth
//! (`services::scheduler::retention`).
//!
//! Gating mirrors the other read-only platform-admin analytics handlers
//! (`get_platform_stats`, `get_health_dashboard`): the `RequireCapability`
//! layer enforces the platform-principal + MFA + active-grant triple, and
//! [`extract_super_admin_token`] re-checks the super-admin role as defence in
//! depth.

use admin_core::RequireCapability;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Duration, Utc};
use common::errors::ErrorResponse;
use db::models::oauth_token_event::{OAuthClientTokenUsage, OAuthTokenEventTotals};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::extract_super_admin_token;
use crate::state::AppState;

/// Default look-back window (in days) when the caller omits `since`.
const DEFAULT_WINDOW_DAYS: i64 = 30;

/// Query parameters for the OAuth token-usage rollup.
#[derive(Debug, Deserialize, utoipa::IntoParams, ToSchema)]
pub struct TokenUsageQuery {
    /// Inclusive lower bound for events, as an RFC 3339 / ISO 8601 timestamp
    /// (e.g. `2026-07-01T00:00:00Z`). Absent ⇒ the last 30 days.
    #[serde(default)]
    pub since: Option<String>,
}

/// Combined OAuth token-usage analytics for the ecosystem-health dashboard.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OAuthTokenUsageResponse {
    /// Whole-ecosystem totals over the window.
    pub totals: OAuthTokenEventTotals,
    /// Per-client rollup, ordered by issuance volume.
    pub per_client: Vec<OAuthClientTokenUsage>,
    /// The resolved window lower bound actually used for the rollup (echoed so
    /// the dashboard can label the range even when `since` was defaulted).
    pub since: DateTime<Utc>,
}

/// Get OAuth token-usage analytics (per-client + totals) for the ecosystem
/// health dashboard.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/oauth/token-usage",
    params(TokenUsageQuery),
    responses(
        (status = 200, description = "Token-usage rollup retrieved", body = OAuthTokenUsageResponse),
        (status = 400, description = "Invalid `since` timestamp"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - OAuth"
)]
pub async fn get_oauth_token_usage(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<TokenUsageQuery>,
) -> Result<Json<OAuthTokenUsageResponse>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    // Resolve the window lower bound: an explicit RFC 3339 `since`, or the
    // default 30-day look-back.
    let since = match query.since.as_deref() {
        Some(raw) => DateTime::parse_from_rfc3339(raw)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                tracing::debug!(error = %e, since = %raw, "Invalid `since` query parameter");
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new(
                        "INVALID_SINCE",
                        "`since` must be an RFC 3339 timestamp",
                    )),
                )
            })?,
        None => Utc::now() - Duration::days(DEFAULT_WINDOW_DAYS),
    };

    let totals = state
        .oauth_token_event_repo
        .totals(since)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get OAuth token-usage totals");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get OAuth token-usage totals",
                )),
            )
        })?;

    let per_client = state
        .oauth_token_event_repo
        .usage_per_client(since)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get OAuth per-client token usage");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get OAuth per-client token usage",
                )),
            )
        })?;

    Ok(Json(OAuthTokenUsageResponse {
        totals,
        per_client,
        since,
    }))
}
