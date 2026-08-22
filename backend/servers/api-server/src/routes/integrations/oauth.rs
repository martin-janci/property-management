//! Integration OAuth-surface routes.
//!
//! Covers OAuth callback flows for external platforms that use OAuth2:
//! - Airbnb OAuth callback (exchanges authorization code for tokens)
//! - Airbnb token-exchange (POST, server-side code → token swap, Story 83.1)
//! - Airbnb listings index (GET, fetches listings via stored token)
//! - Airbnb reservations index (GET, fetches reservations via stored token)
//!
//! Note: Google/Microsoft calendar OAuth tokens are handled at the calendar
//! provider level via `sync::sync_calendar` (token is already stored after
//! the provider's own OAuth flow, outside of this service).

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use integrations::{
    AirbnbClient, AirbnbListing, AirbnbOAuthConfig, AirbnbReservation, BookingOAuthClient,
    BookingOAuthConfig,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use api_core::TenantExtractor;

use super::{
    install::{AirbnbCallbackResponse, OAuthCallbackQuery},
    sync::{verify_manager_role_in_org, verify_org_access, OrgIdPath},
    token_rotation::with_token_refresh,
};
use crate::state::AppState;
use common::errors::ErrorResponse;

// ==================== Router ====================

/// Create OAuth-surface router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Airbnb OAuth callback (Story 83.1) — browser redirect lands here.
        .route(
            "/organizations/{org_id}/airbnb/callback",
            get(airbnb_oauth_callback),
        )
        // Airbnb server-side token exchange — POST variant for back-channel flows.
        .route(
            "/organizations/{org_id}/airbnb/token/exchange",
            post(airbnb_token_exchange),
        )
        // Airbnb listings index — proxies through stored token with auto-refresh.
        .route(
            "/organizations/{org_id}/airbnb/listings",
            get(list_airbnb_listings),
        )
        // Airbnb reservations index — proxies through stored token with auto-refresh.
        .route(
            "/organizations/{org_id}/airbnb/reservations",
            get(list_airbnb_reservations),
        )
        // Booking.com server-side OAuth token exchange (Coverage 83-2).
        .route(
            "/organizations/{org_id}/booking/token/exchange",
            post(booking_token_exchange),
        )
}

// ==================== Booking.com Token Exchange (POST) ====================

/// Request body for the server-side Booking.com OAuth token-exchange endpoint.
#[derive(Debug, Deserialize, ToSchema)]
pub struct BookingTokenExchangeRequest {
    /// The authorization code obtained from the Booking.com OAuth redirect.
    pub code: String,
    /// Optional override for the redirect URI used during the authorize call.
    pub redirect_uri: Option<String>,
    /// Optional Booking.com hotel/property ID to associate with the connection.
    pub property_id: Option<String>,
}

/// Response from the Booking.com server-side token-exchange endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct BookingTokenExchangeResponse {
    pub success: bool,
    pub connection_id: Option<Uuid>,
    pub message: String,
}

/// Exchange a Booking.com authorization code for access/refresh tokens (POST).
///
/// Secrets (`BOOKING_CLIENT_ID`, `BOOKING_CLIENT_SECRET`,
/// `INTEGRATION_ENCRYPTION_KEY`) must be present in server config — they are
/// never accepted in the request body. Fails closed (503) when Booking.com is
/// not configured, and refuses to store tokens (500) when encryption is absent.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/organizations/{org_id}/booking/token/exchange",
    params(OrgIdPath),
    request_body = BookingTokenExchangeRequest,
    responses(
        (status = 200, description = "Token exchange successful", body = BookingTokenExchangeResponse),
        (status = 400, description = "Invalid or expired authorization code"),
        (status = 403, description = "Caller is not a member of the organisation"),
        (status = 503, description = "Booking.com integration is not configured on this server")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Booking.com"
)]
pub async fn booking_token_exchange(
    State(state): State<AppState>,
    // Retained for its side effect: TenantExtractor rejects requests without a
    // valid tenant header before the handler runs. Not read directly — org-scope
    // authz is enforced by verify_manager_role_in_org(path.org_id) below.
    _tenant: TenantExtractor,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
    Json(body): Json<BookingTokenExchangeRequest>,
) -> Result<Json<BookingTokenExchangeResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Booking.com server-side token exchange requested"
    );

    if body.code.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "MISSING_CODE",
                "Authorization code is required",
            )),
        ));
    }

    // IDOR guard — caller must belong to the target org.
    verify_org_access(&state, auth.user_id, path.org_id).await?;
    // Manager-level gate: binding a paid OTA integration is an org-wide action.
    // Role is read from membership of the PATH org (#1525), not the JWT tenant.
    verify_manager_role_in_org(&state, auth.user_id, path.org_id).await?;

    let client_id = state.booking_config.client_id.clone();
    let client_secret = state.booking_config.client_secret.clone();
    let redirect_uri = body
        .redirect_uri
        .clone()
        .unwrap_or_else(|| state.booking_config.redirect_uri.clone());

    // Fail closed when Booking.com is not configured on this server.
    if client_id.is_empty() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new(
                "NOT_CONFIGURED",
                "Booking.com integration is not configured on this server",
            )),
        ));
    }

    let client = BookingOAuthClient::new(BookingOAuthConfig {
        client_id,
        client_secret,
        redirect_uri,
    });

    let tokens = client.exchange_code(&body.code).await.map_err(|e| {
        tracing::error!(error = %e, "Booking.com token exchange failed");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "TOKEN_EXCHANGE_FAILED",
                "Failed to exchange authorization code for tokens",
            )),
        )
    })?;

    // Encrypt before storage — fail closed if the key is missing.
    let crypto = integrations::IntegrationCrypto::try_from_env();
    let encryption_unavailable = |e: integrations::CryptoError| {
        tracing::error!(error = %e, "Refusing to store Booking.com tokens without encryption");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "ENCRYPTION_REQUIRED",
                "Integration token encryption is not configured",
            )),
        )
    };
    let encrypted_access = integrations::encrypt_required(crypto.as_ref(), &tokens.access_token)
        .map_err(encryption_unavailable)?;
    let encrypted_refresh =
        integrations::encrypt_optional_required(crypto.as_ref(), tokens.refresh_token.as_deref())
            .map_err(encryption_unavailable)?;

    let connection = state
        .rental_repo
        .upsert_booking_oauth_connection(
            path.org_id,
            None,
            &encrypted_access,
            encrypted_refresh.as_deref(),
            tokens.expires_at,
            body.property_id.as_deref(),
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to store Booking.com connection after token exchange");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to store connection",
                )),
            )
        })?;

    tracing::info!(
        connection_id = %connection.id,
        org_id = %path.org_id,
        "Booking.com server-side token exchange completed"
    );

    Ok(Json(BookingTokenExchangeResponse {
        success: true,
        connection_id: Some(connection.id),
        message: "Booking.com connected successfully".to_string(),
    }))
}

// ==================== Token Exchange (POST) ====================

/// Request body for the server-side Airbnb OAuth token-exchange endpoint.
///
/// Used when the frontend has already retrieved the `code` from the Airbnb
/// redirect and wants the backend to exchange it (back-channel POST instead
/// of a browser GET callback).
#[derive(Debug, Deserialize, ToSchema)]
pub struct AirbnbTokenExchangeRequest {
    /// The authorization code obtained from the Airbnb OAuth redirect.
    pub code: String,
    /// Optional override for the redirect URI used during the original
    /// authorize call.  Defaults to the server-configured
    /// `AIRBNB_REDIRECT_URI`.
    pub redirect_uri: Option<String>,
}

/// Response from the server-side token-exchange endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct AirbnbTokenExchangeResponse {
    pub success: bool,
    pub connection_id: Option<Uuid>,
    pub message: String,
    pub listings_count: Option<i32>,
}

/// Exchange an Airbnb authorization code for access/refresh tokens (POST).
///
/// This is the server-side (back-channel) counterpart to the browser-redirect
/// `GET /organizations/{org_id}/airbnb/callback` handler. Use this when your
/// frontend already holds the `code` from the Airbnb redirect and wants to
/// hand it to the backend directly, avoiding a browser round-trip.
///
/// Secrets (`AIRBNB_CLIENT_ID`, `AIRBNB_CLIENT_SECRET`,
/// `INTEGRATION_ENCRYPTION_KEY`) must be present in server config — they
/// are never accepted in the request body.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/organizations/{org_id}/airbnb/token/exchange",
    params(OrgIdPath),
    request_body = AirbnbTokenExchangeRequest,
    responses(
        (status = 200, description = "Token exchange successful", body = AirbnbTokenExchangeResponse),
        (status = 400, description = "Invalid or expired authorization code"),
        (status = 403, description = "Caller is not a member of the organisation"),
        (status = 503, description = "Airbnb integration is not configured on this server")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Airbnb"
)]
pub async fn airbnb_token_exchange(
    State(state): State<AppState>,
    // Retained for its side effect: TenantExtractor rejects requests without a
    // valid tenant header before the handler runs. Not read directly — org-scope
    // authz is enforced by verify_manager_role_in_org(path.org_id) below.
    _tenant: TenantExtractor,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
    Json(body): Json<AirbnbTokenExchangeRequest>,
) -> Result<Json<AirbnbTokenExchangeResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Airbnb server-side token exchange requested"
    );

    if body.code.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "MISSING_CODE",
                "Authorization code is required",
            )),
        ));
    }

    // IDOR guard — caller must belong to the target org.
    verify_org_access(&state, auth.user_id, path.org_id).await?;
    // Manager-level gate: binding a paid OTA integration is an org-wide action.
    // Role is read from membership of the PATH org (#1525), not the JWT tenant.
    verify_manager_role_in_org(&state, auth.user_id, path.org_id).await?;

    let client_id = state.airbnb_config.client_id.clone();
    let client_secret = state.airbnb_config.client_secret.clone();
    let redirect_uri = body
        .redirect_uri
        .clone()
        .unwrap_or_else(|| state.airbnb_config.redirect_uri.clone());

    if client_id.is_empty() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new(
                "NOT_CONFIGURED",
                "Airbnb integration is not configured on this server",
            )),
        ));
    }

    let client = AirbnbClient::new(AirbnbOAuthConfig {
        client_id,
        client_secret,
        redirect_uri,
    });

    let tokens = client.exchange_code(&body.code).await.map_err(|e| {
        tracing::error!(error = %e, "Airbnb token exchange failed");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "TOKEN_EXCHANGE_FAILED",
                "Failed to exchange authorization code for tokens",
            )),
        )
    })?;

    // Encrypt before storage — fail closed if key is missing.
    let crypto = integrations::IntegrationCrypto::try_from_env();
    let encryption_unavailable = |e: integrations::CryptoError| {
        tracing::error!(error = %e, "Refusing to store Airbnb tokens without encryption");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "ENCRYPTION_REQUIRED",
                "Integration token encryption is not configured",
            )),
        )
    };
    let encrypted_access = integrations::encrypt_required(crypto.as_ref(), &tokens.access_token)
        .map_err(encryption_unavailable)?;
    let encrypted_refresh =
        integrations::encrypt_optional_required(crypto.as_ref(), tokens.refresh_token.as_deref())
            .map_err(encryption_unavailable)?;

    let connection = state
        .rental_repo
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
            tracing::error!(error = %e, "Failed to store Airbnb connection after token exchange");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to store connection",
                )),
            )
        })?;

    // Attempt to fetch an initial listing count to populate the status page.
    let listings_count = match client.fetch_listings(&tokens.access_token).await {
        Ok(listings) => {
            if let Some(first) = listings.first() {
                let _ = state
                    .rental_repo
                    .update_airbnb_listing_mapping(
                        connection.id,
                        &first.id,
                        Some(&format!("https://www.airbnb.com/rooms/{}", first.id)),
                    )
                    .await;
            }
            Some(listings.len() as i32)
        }
        Err(e) => {
            tracing::warn!(error = %e, "Failed to fetch Airbnb listings after token exchange");
            None
        }
    };

    tracing::info!(
        connection_id = %connection.id,
        org_id = %path.org_id,
        listings_count = ?listings_count,
        "Airbnb server-side token exchange completed"
    );

    Ok(Json(AirbnbTokenExchangeResponse {
        success: true,
        connection_id: Some(connection.id),
        message: "Airbnb connected successfully".to_string(),
        listings_count,
    }))
}

// ==================== Airbnb Listings ====================

/// Response body for the Airbnb listings endpoint.
///
/// `AirbnbListing` does not derive `utoipa::ToSchema` (it lives in the
/// `integrations` crate which does not depend on utoipa), so we omit the
/// derive here and annotate the utoipa path with a manual schema reference.
#[derive(Debug, Serialize)]
pub struct AirbnbListingsResponse {
    pub listings: Vec<AirbnbListing>,
    pub count: usize,
}

/// List Airbnb listings for an organisation.
///
/// Proxies the request through the organisation's stored (and if necessary,
/// auto-refreshed) Airbnb access token.  The caller never sees the token.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/organizations/{org_id}/airbnb/listings",
    params(OrgIdPath),
    responses(
        (status = 200, description = "Airbnb listings — `{listings: [...], count: N}`"),
        (status = 403, description = "Caller is not a member of the organisation, or lacks a manager-level role in it"),
        (status = 404, description = "No Airbnb connection found"),
        (status = 502, description = "Airbnb API error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Airbnb"
)]
pub async fn list_airbnb_listings(
    State(state): State<AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
) -> Result<Json<AirbnbListingsResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Fetching Airbnb listings"
    );

    verify_org_access(&state, auth.user_id, path.org_id).await?;
    // Manager-level gate (issue #1626): live Airbnb linked-listing data is
    // landlord/manager operational data — a plain `resident` member must not be
    // able to enumerate it via this live proxy, for parity with the DB-backed
    // `/airbnb/connections` read (#1639) and the Airbnb write paths. Role is read
    // from the caller's membership of the PATH org (#1525/#1585), not the JWT.
    verify_manager_role_in_org(&state, auth.user_id, path.org_id).await?;

    use super::token_rotation::TokenRotationOutcome;

    let outcome = with_token_refresh(&state, path.org_id, |access_token| {
        let state2 = state.clone();
        async move {
            let client = AirbnbClient::new(AirbnbOAuthConfig {
                client_id: state2.airbnb_config.client_id.clone(),
                client_secret: state2.airbnb_config.client_secret.clone(),
                redirect_uri: state2.airbnb_config.redirect_uri.clone(),
            });
            client.fetch_listings(&access_token).await
        }
    })
    .await;

    match outcome {
        TokenRotationOutcome::Ok(listings) => {
            let count = listings.len();
            Ok(Json(AirbnbListingsResponse { listings, count }))
        }
        TokenRotationOutcome::NoConnection => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "No Airbnb connection found for this organisation",
            )),
        )),
        TokenRotationOutcome::ExpiredNoRefresh => Err((
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse::new(
                "TOKEN_EXPIRED",
                "Airbnb access token expired and no refresh token is available",
            )),
        )),
        TokenRotationOutcome::DecryptionFailed(msg) => {
            tracing::error!(org_id = %path.org_id, detail = %msg, "Token decryption failed");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DECRYPTION_FAILED",
                    "Failed to decrypt stored Airbnb token",
                )),
            ))
        }
        TokenRotationOutcome::RefreshFailed(msg) => {
            tracing::error!(org_id = %path.org_id, detail = %msg, "Token refresh failed");
            Err((
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse::new(
                    "TOKEN_REFRESH_FAILED",
                    "Failed to refresh expired Airbnb token",
                )),
            ))
        }
        TokenRotationOutcome::CallFailed(e) => {
            tracing::error!(org_id = %path.org_id, error = %e, "Airbnb listings fetch failed");
            Err((
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse::new(
                    "AIRBNB_API_ERROR",
                    "Airbnb API request failed",
                )),
            ))
        }
    }
}

// ==================== Airbnb Reservations ====================

/// Query parameters for the Airbnb reservations endpoint.
#[derive(Debug, Deserialize, IntoParams)]
pub struct AirbnbReservationsQuery {
    /// Optional Airbnb listing ID to scope reservations to a single listing.
    pub listing_id: Option<String>,
}

/// Response body for the Airbnb reservations endpoint.
///
/// `AirbnbReservation` does not derive `utoipa::ToSchema` — see
/// `AirbnbListingsResponse` for the same rationale.
#[derive(Debug, Serialize)]
pub struct AirbnbReservationsResponse {
    pub reservations: Vec<AirbnbReservation>,
    pub count: usize,
}

/// List Airbnb reservations for an organisation.
///
/// Proxies the request through the organisation's stored (and if necessary,
/// auto-refreshed) Airbnb access token.  When `listing_id` is supplied only
/// reservations for that listing are returned.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/organizations/{org_id}/airbnb/reservations",
    params(OrgIdPath, AirbnbReservationsQuery),
    responses(
        (status = 200, description = "Airbnb reservations — `{reservations: [...], count: N}`"),
        (status = 403, description = "Caller is not a member of the organisation, or lacks a manager-level role in it"),
        (status = 404, description = "No Airbnb connection found"),
        (status = 502, description = "Airbnb API error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Airbnb"
)]
pub async fn list_airbnb_reservations(
    State(state): State<AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
    Query(query): Query<AirbnbReservationsQuery>,
) -> Result<Json<AirbnbReservationsResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        listing_id = ?query.listing_id,
        "Fetching Airbnb reservations"
    );

    verify_org_access(&state, auth.user_id, path.org_id).await?;
    // Manager-level gate (issue #1667, follow-up to #1626/#1635): live Airbnb
    // reservation data carries guest PII (names, check-in/check-out dates,
    // booking/listing IDs) — a plain `resident` member must not be able to
    // enumerate it via this live proxy, for parity with `/airbnb/listings`
    // (#1635) and `/airbnb/connections` (#1639). Role is read from the caller's
    // membership of the PATH org (#1525/#1585), not the JWT.
    verify_manager_role_in_org(&state, auth.user_id, path.org_id).await?;

    use super::token_rotation::TokenRotationOutcome;

    let listing_id = query.listing_id.clone();

    let outcome = with_token_refresh(&state, path.org_id, |access_token| {
        let state2 = state.clone();
        let lid = listing_id.clone();
        async move {
            let client = AirbnbClient::new(AirbnbOAuthConfig {
                client_id: state2.airbnb_config.client_id.clone(),
                client_secret: state2.airbnb_config.client_secret.clone(),
                redirect_uri: state2.airbnb_config.redirect_uri.clone(),
            });
            if let Some(ref id) = lid {
                client
                    .fetch_reservations(&access_token, id, None, None)
                    .await
            } else {
                client.fetch_all_reservations(&access_token).await
            }
        }
    })
    .await;

    match outcome {
        TokenRotationOutcome::Ok(reservations) => {
            let count = reservations.len();
            Ok(Json(AirbnbReservationsResponse {
                reservations,
                count,
            }))
        }
        TokenRotationOutcome::NoConnection => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "No Airbnb connection found for this organisation",
            )),
        )),
        TokenRotationOutcome::ExpiredNoRefresh => Err((
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse::new(
                "TOKEN_EXPIRED",
                "Airbnb access token expired and no refresh token is available",
            )),
        )),
        TokenRotationOutcome::DecryptionFailed(msg) => {
            tracing::error!(org_id = %path.org_id, detail = %msg, "Token decryption failed");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DECRYPTION_FAILED",
                    "Failed to decrypt stored Airbnb token",
                )),
            ))
        }
        TokenRotationOutcome::RefreshFailed(msg) => {
            tracing::error!(org_id = %path.org_id, detail = %msg, "Token refresh failed");
            Err((
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse::new(
                    "TOKEN_REFRESH_FAILED",
                    "Failed to refresh expired Airbnb token",
                )),
            ))
        }
        TokenRotationOutcome::CallFailed(e) => {
            tracing::error!(org_id = %path.org_id, error = %e, "Airbnb reservations fetch failed");
            Err((
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse::new(
                    "AIRBNB_API_ERROR",
                    "Airbnb API request failed",
                )),
            ))
        }
    }
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

    // Issue #765: the OAuth `state` must be a real CSRF token — single-use and
    // server-bound. Verify it against the value persisted when the flow was
    // initiated and consume it so it cannot be replayed. When the state store
    // (Redis) is unavailable we fall back to the stateless org-prefix check
    // above plus the membership check below (defence in depth).
    match super::oauth_state::verify_and_consume(&state, &query.state, path.org_id).await {
        super::oauth_state::ConsumeOutcome::Consumed => {}
        super::oauth_state::ConsumeOutcome::StoreUnavailable => {
            tracing::warn!(
                org_id = %path.org_id,
                "OAuth state store unavailable; relying on stateless state + membership checks"
            );
        }
        super::oauth_state::ConsumeOutcome::Rejected => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_STATE",
                    "OAuth state is invalid, expired, or already used",
                )),
            ));
        }
    }

    // Issue #765: prevent cross-org IDOR — the embedded-state org match above
    // is not an authorization check (the `state` value is client-visible and
    // forgeable). Require the authenticated caller to actually be a member of
    // the organization before binding Airbnb tokens to it.
    verify_org_access(&state, auth.user_id, path.org_id).await?;
    // SECURITY: binding Airbnb OAuth tokens to the org is a manager-only
    // mutation, matching `airbnb_token_exchange` and `booking_token_exchange`.
    // `verify_org_access` alone would let any org member complete a callback and
    // overwrite the org's outward-facing Airbnb channel credentials.
    verify_manager_role_in_org(&state, auth.user_id, path.org_id).await?;

    // Issue #711: pull Airbnb OAuth credentials from the AppState-cached
    // config rather than re-reading the env on every callback.
    let client_id = state.airbnb_config.client_id.clone();
    let client_secret = state.airbnb_config.client_secret.clone();
    let redirect_uri = state.airbnb_config.redirect_uri.clone();

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

    // Encrypt tokens before storage. Issue #765: encryption is MANDATORY — if
    // INTEGRATION_ENCRYPTION_KEY is unset we fail closed rather than persisting
    // OAuth tokens in plaintext.
    let crypto = integrations::IntegrationCrypto::try_from_env();
    let encryption_unavailable = |e: integrations::CryptoError| {
        tracing::error!(error = %e, "Refusing to store Airbnb OAuth tokens without encryption");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "ENCRYPTION_REQUIRED",
                "Integration token encryption is not configured",
            )),
        )
    };
    let encrypted_access = integrations::encrypt_required(crypto.as_ref(), &tokens.access_token)
        .map_err(encryption_unavailable)?;
    let encrypted_refresh =
        integrations::encrypt_optional_required(crypto.as_ref(), tokens.refresh_token.as_deref())
            .map_err(encryption_unavailable)?;

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

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_booking_token_exchange_request_deserialization_full() {
        let json = r#"{"code":"auth-code-123","redirect_uri":"https://app.example.com/booking/callback","property_id":"hotel-9876"}"#;
        let req: BookingTokenExchangeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code, "auth-code-123");
        assert_eq!(
            req.redirect_uri.as_deref(),
            Some("https://app.example.com/booking/callback")
        );
        assert_eq!(req.property_id.as_deref(), Some("hotel-9876"));
    }

    #[test]
    fn test_booking_token_exchange_request_minimal() {
        let json = r#"{"code":"auth-code-only"}"#;
        let req: BookingTokenExchangeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code, "auth-code-only");
        assert!(req.redirect_uri.is_none());
        assert!(req.property_id.is_none());
    }

    #[test]
    fn test_booking_token_exchange_request_missing_code_fails() {
        let json = r#"{"redirect_uri":"https://example.com/cb"}"#;
        let parsed: Result<BookingTokenExchangeRequest, _> = serde_json::from_str(json);
        assert!(parsed.is_err(), "missing code must fail to deserialize");
    }

    #[test]
    fn test_booking_token_exchange_response_serialization() {
        let id = Uuid::new_v4();
        let resp = BookingTokenExchangeResponse {
            success: true,
            connection_id: Some(id),
            message: "Booking.com connected successfully".to_string(),
        };
        let value: serde_json::Value = serde_json::to_value(&resp).unwrap();
        assert_eq!(value["success"], serde_json::json!(true));
        assert_eq!(value["connection_id"], serde_json::json!(id.to_string()));
    }

    #[test]
    fn test_booking_token_exchange_response_failure_shape() {
        let resp = BookingTokenExchangeResponse {
            success: false,
            connection_id: None,
            message: "failed".to_string(),
        };
        let value: serde_json::Value = serde_json::to_value(&resp).unwrap();
        assert_eq!(value["success"], serde_json::json!(false));
        assert!(value["connection_id"].is_null());
    }

    // ---- Airbnb token-exchange serde (Story 83.1 / Coverage 83-1) ----
    //
    // Mirrors the Booking.com coverage above for the primary Airbnb flow.
    // These are pure (de)serialization checks of the request/response wire
    // contract for `POST /organizations/{org_id}/airbnb/token/exchange`; they
    // do not touch the network or a DB. See `tests/airbnb_oauth_routes_tests.rs`
    // for the auth/IDOR integration coverage of the same endpoint.

    #[test]
    fn test_airbnb_token_exchange_request_deserialization_full() {
        let json = r#"{"code":"airbnb-auth-code-123","redirect_uri":"https://app.example.com/airbnb/callback"}"#;
        let req: AirbnbTokenExchangeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code, "airbnb-auth-code-123");
        assert_eq!(
            req.redirect_uri.as_deref(),
            Some("https://app.example.com/airbnb/callback")
        );
    }

    #[test]
    fn test_airbnb_token_exchange_request_minimal() {
        // Only `code` is required; `redirect_uri` falls back to the
        // server-configured AIRBNB_REDIRECT_URI when omitted.
        let json = r#"{"code":"airbnb-auth-code-only"}"#;
        let req: AirbnbTokenExchangeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code, "airbnb-auth-code-only");
        assert!(req.redirect_uri.is_none());
    }

    #[test]
    fn test_airbnb_token_exchange_request_missing_code_fails() {
        // A body without `code` must be rejected at deserialization so the
        // handler never forwards an empty authorization code to Airbnb.
        let json = r#"{"redirect_uri":"https://example.com/cb"}"#;
        let parsed: Result<AirbnbTokenExchangeRequest, _> = serde_json::from_str(json);
        assert!(parsed.is_err(), "missing code must fail to deserialize");
    }

    #[test]
    fn test_airbnb_token_exchange_request_rejects_null_code() {
        // `code` is a non-optional String — an explicit JSON null must fail.
        let json = r#"{"code":null}"#;
        let parsed: Result<AirbnbTokenExchangeRequest, _> = serde_json::from_str(json);
        assert!(parsed.is_err(), "null code must fail to deserialize");
    }

    #[test]
    fn test_airbnb_token_exchange_response_serialization() {
        let id = Uuid::new_v4();
        let resp = AirbnbTokenExchangeResponse {
            success: true,
            connection_id: Some(id),
            message: "Airbnb connected successfully".to_string(),
            listings_count: Some(3),
        };
        let value: serde_json::Value = serde_json::to_value(&resp).unwrap();
        assert_eq!(value["success"], serde_json::json!(true));
        assert_eq!(value["connection_id"], serde_json::json!(id.to_string()));
        assert_eq!(
            value["message"],
            serde_json::json!("Airbnb connected successfully")
        );
        assert_eq!(value["listings_count"], serde_json::json!(3));
    }

    #[test]
    fn test_airbnb_token_exchange_response_failure_shape() {
        // On failure the optional fields must serialize to JSON null rather
        // than being dropped, so the frontend can distinguish "not set" cleanly.
        let resp = AirbnbTokenExchangeResponse {
            success: false,
            connection_id: None,
            message: "failed".to_string(),
            listings_count: None,
        };
        let value: serde_json::Value = serde_json::to_value(&resp).unwrap();
        assert_eq!(value["success"], serde_json::json!(false));
        assert!(value["connection_id"].is_null());
        assert!(value["listings_count"].is_null());
    }
}
