//! Integration OAuth-surface routes.
//!
//! Covers OAuth callback flows for external platforms that use OAuth2:
//! - Airbnb OAuth callback (exchanges authorization code for tokens)
//!
//! Note: Google/Microsoft calendar OAuth tokens are handled at the calendar
//! provider level via `sync::sync_calendar` (token is already stored after
//! the provider's own OAuth flow, outside of this service).

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use integrations::{AirbnbClient, AirbnbOAuthConfig};
use uuid::Uuid;

use super::{
    install::{AirbnbCallbackResponse, OAuthCallbackQuery},
    sync::OrgIdPath,
};
use crate::state::AppState;
use common::errors::ErrorResponse;

// ==================== Router ====================

/// Create OAuth-surface router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Airbnb OAuth callback (Story 83.1)
        .route(
            "/organizations/{org_id}/airbnb/callback",
            get(airbnb_oauth_callback),
        )
}

// ==================== Airbnb OAuth Callback ====================

/// Handle Airbnb OAuth callback.
///
/// Exchanges the authorization code for access/refresh tokens, stores
/// them encrypted in the database, and fetches initial listing data.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/organizations/{org_id}/airbnb/callback",
    params(OrgIdPath, OAuthCallbackQuery),
    responses(
        (status = 200, description = "OAuth callback processed", body = AirbnbCallbackResponse),
        (status = 400, description = "Invalid callback"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Airbnb"
)]
pub async fn airbnb_oauth_callback(
    State(state): State<AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<Json<AirbnbCallbackResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Processing Airbnb OAuth callback"
    );

    if query.state.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_STATE",
                "Invalid state parameter",
            )),
        ));
    }

    // Stateless state verification: org_id is embedded in the state parameter
    let state_parts: Vec<&str> = query.state.split(':').collect();
    if state_parts.len() < 2 {
        tracing::warn!(state = %query.state, "Invalid OAuth state format");
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_STATE",
                "Invalid OAuth state format",
            )),
        ));
    }

    let state_org_id = Uuid::parse_str(state_parts[0]).ok();
    if state_org_id != Some(path.org_id) {
        tracing::warn!(
            state_org = ?state_org_id,
            path_org = %path.org_id,
            "OAuth state org_id mismatch"
        );
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "STATE_MISMATCH",
                "OAuth state does not match organization",
            )),
        ));
    }

    let client_id = std::env::var("AIRBNB_CLIENT_ID").unwrap_or_default();
    let client_secret = std::env::var("AIRBNB_CLIENT_SECRET").unwrap_or_default();
    let redirect_uri = std::env::var("AIRBNB_REDIRECT_URI").unwrap_or_default();

    if client_id.is_empty() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new(
                "NOT_CONFIGURED",
                "Airbnb integration is not configured",
            )),
        ));
    }

    let client = AirbnbClient::new(AirbnbOAuthConfig {
        client_id,
        client_secret,
        redirect_uri,
    });

    let tokens = client.exchange_code(&query.code).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to exchange Airbnb authorization code");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "TOKEN_EXCHANGE_FAILED",
                "Failed to exchange authorization code for tokens",
            )),
        )
    })?;

    // Encrypt tokens before storage
    let crypto = integrations::IntegrationCrypto::from_env().ok();
    let (encrypted_access, encrypted_refresh) = match &crypto {
        Some(c) => (
            c.encrypt(&tokens.access_token)
                .unwrap_or(tokens.access_token.clone()),
            tokens
                .refresh_token
                .as_ref()
                .map(|rt| c.encrypt(rt).unwrap_or(rt.clone())),
        ),
        None => (tokens.access_token.clone(), tokens.refresh_token.clone()),
    };

    let rental_repo = &state.rental_repo;
    let connection = rental_repo
        .upsert_airbnb_connection(
            path.org_id,
            None,
            &encrypted_access,
            encrypted_refresh.as_deref(),
            tokens.expires_at,
            None,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to store Airbnb connection");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to store connection",
                )),
            )
        })?;

    // Fetch initial listings to populate external account info
    let listings_count = match client.fetch_listings(&tokens.access_token).await {
        Ok(listings) => {
            if let Some(first_listing) = listings.first() {
                let _ = rental_repo
                    .update_airbnb_listing_mapping(
                        connection.id,
                        &first_listing.id,
                        Some(&format!(
                            "https://www.airbnb.com/rooms/{}",
                            first_listing.id
                        )),
                    )
                    .await;
            }
            Some(listings.len() as i32)
        }
        Err(e) => {
            tracing::warn!(error = %e, "Failed to fetch Airbnb listings after OAuth");
            None
        }
    };

    let webhook_url = std::env::var("AIRBNB_WEBHOOK_URL").ok();
    if webhook_url.is_some() {
        tracing::info!(
            "Airbnb webhook URL configured, webhook setup should be done via partner dashboard"
        );
    }

    tracing::info!(
        connection_id = %connection.id,
        org_id = %path.org_id,
        listings_count = ?listings_count,
        "Airbnb OAuth completed successfully"
    );

    Ok(Json(AirbnbCallbackResponse {
        success: true,
        connection_id: Some(connection.id),
        message: "Airbnb connected successfully".to_string(),
        listings_count,
    }))
}
