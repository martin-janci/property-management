//! OAuth 2.0 Authorization Server routes (Epic 10A).
//!
//! Implements OAuth 2.0 Authorization Code flow with PKCE support.

use admin_core::{require_capability, Capability, RequireCapability};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Form, Json, Router,
};
use base64::Engine;
use common::errors::ErrorResponse;
use db::models::{
    AuditAction, ConsentPageData, CreateAuditLog, IntrospectionResponse, OAuthClientSummary,
    OAuthError, RegisterClientRequest, RegisterClientResponse, RevokeTokenRequest, TokenRequest,
    TokenResponse, UpdateOAuthClient, UserGrantWithClient,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::state::AppState;

/// Create OAuth router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Public OAuth endpoints
        .route("/authorize", get(authorize_get))
        .route("/authorize", post(authorize_post))
        .route("/token", post(token))
        .route("/revoke", post(revoke))
        .route("/introspect", post(introspect))
        // User grant management
        .route("/grants", get(list_user_grants))
        .route(
            "/grants/{client_id}",
            axum::routing::delete(revoke_user_grant),
        )
}

/// Create OAuth admin router (for client management).
///
/// Every route is gated by `Capability::OauthClientWrite`. The previous
/// implementation hand-rolled a `claims.roles.contains("super_admin")` check
/// in each handler — that pattern trusts the JWT `roles` string claim, which
/// the rest of the admin surface has moved away from. The capability gate
/// re-derives the platform principal from the trusted `users` table on every
/// request, checks an active grant, and enforces recent MFA.
pub fn admin_router() -> Router<AppState> {
    // All admin routes require OauthClientWrite — hoist the layer to the router
    // rather than repeating it on every individual route.
    Router::new()
        .route("/clients", post(register_client))
        .route("/clients", get(list_clients))
        .route("/clients/{id}", get(get_client))
        .route("/clients/{id}", axum::routing::patch(update_client))
        .route("/clients/{id}", axum::routing::delete(revoke_client))
        .route(
            "/clients/{id}/regenerate-secret",
            post(regenerate_client_secret),
        )
        .layer(require_capability(Capability::OauthClientWrite))
}

// ==================== Authorization Endpoint ====================

/// Query parameters for authorization endpoint.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AuthorizeQuery {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

/// Authorization endpoint - GET to show consent page.
#[utoipa::path(
    get,
    path = "/api/v1/oauth/authorize",
    tag = "OAuth 2.0",
    params(
        ("response_type" = String, Query, description = "Must be 'code'"),
        ("client_id" = String, Query, description = "OAuth client ID"),
        ("redirect_uri" = String, Query, description = "Redirect URI"),
        ("scope" = Option<String>, Query, description = "Space-separated scopes"),
        ("state" = Option<String>, Query, description = "CSRF token"),
        ("code_challenge" = Option<String>, Query, description = "PKCE code challenge"),
        ("code_challenge_method" = Option<String>, Query, description = "PKCE method (S256)")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Consent page data", body = ConsentPageData),
        (status = 400, description = "Invalid request", body = OAuthError),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn authorize_get(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(params): Query<AuthorizeQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<OAuthError>)> {
    // SECURITY (issue #820): this endpoint advertises `bearer_auth` and a 401
    // in its OpenAPI, but the handler took `_headers` and never authenticated
    // the caller — the consent page (client name, requested scopes, redirect
    // URI) was served to any anonymous client. Require a valid access token
    // up front, exactly as `authorize_post` does, before doing any work.
    let token = extract_bearer_token(&headers).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(OAuthError::access_denied("Authentication required")),
        )
    })?;
    validate_access_token(&state, &token).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(OAuthError::access_denied("Invalid or expired token")),
        )
    })?;

    // Validate response_type
    if params.response_type != "code" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(OAuthError::invalid_request("response_type must be 'code'")),
        ));
    }

    // Parse scopes
    let scopes: Vec<String> = params
        .scope
        .as_ref()
        .map(|s| s.split_whitespace().map(String::from).collect())
        .unwrap_or_else(|| vec!["profile".to_string()]);

    // Validate the request and get consent page data
    let consent_data = state
        .oauth_service
        .validate_authorize_request(
            &params.client_id,
            &params.redirect_uri,
            &scopes,
            params.state,
            params.code_challenge.as_deref(),
        )
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(e.into())))?;

    Ok(Json(consent_data))
}

/// Consent form submission.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ConsentForm {
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: String,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub consent: String, // "approve" or "deny"
}

/// Authorization code response.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizeResponse {
    pub code: String,
    pub state: Option<String>,
}

/// Authorization endpoint - POST to process consent.
#[utoipa::path(
    post,
    path = "/api/v1/oauth/authorize",
    tag = "OAuth 2.0",
    security(("bearer_auth" = [])),
    request_body = ConsentForm,
    responses(
        (status = 200, description = "Authorization code issued", body = AuthorizeResponse),
        (status = 400, description = "Invalid request", body = OAuthError),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Access denied", body = OAuthError)
    )
)]
pub async fn authorize_post(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Form(form): Form<ConsentForm>,
) -> Result<impl IntoResponse, (StatusCode, Json<OAuthError>)> {
    // Extract and validate access token
    let token = extract_bearer_token(&headers).map_err(|_e| {
        (
            StatusCode::UNAUTHORIZED,
            Json(OAuthError::access_denied("Authentication required")),
        )
    })?;
    let claims = validate_access_token(&state, &token).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(OAuthError::access_denied("Invalid or expired token")),
        )
    })?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(OAuthError::access_denied("Invalid token")),
        )
    })?;

    // Check consent
    if form.consent != "approve" {
        return Err((
            StatusCode::FORBIDDEN,
            Json(OAuthError::access_denied("User denied authorization")),
        ));
    }

    // Parse scopes
    let scopes: Vec<String> = form.scope.split_whitespace().map(String::from).collect();

    // Create authorization code
    let code = state
        .oauth_service
        .create_authorization_code(
            user_id,
            &form.client_id,
            &form.redirect_uri,
            &scopes,
            form.code_challenge,
            form.code_challenge_method,
        )
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(e.into())))?;

    // Audit log
    if let Err(e) = state
        .audit_log_repo
        .create(CreateAuditLog {
            user_id: Some(user_id),
            action: AuditAction::OAuthAuthorize,
            resource_type: Some("oauth_authorization".to_string()),
            resource_id: None,
            org_id: None,
            details: Some(serde_json::json!({
                "client_id": form.client_id,
                "scopes": scopes,
            })),
            old_values: None,
            new_values: None,
            ip_address: None,
            user_agent: None,
        })
        .await
    {
        tracing::warn!(error = %e, "Failed to create audit log for OAuth authorization");
    }

    Ok(Json(AuthorizeResponse {
        code,
        state: form.state,
    }))
}

// ==================== Token Endpoint ====================

/// Token endpoint - exchange code for tokens or refresh tokens.
#[utoipa::path(
    post,
    path = "/api/v1/oauth/token",
    tag = "OAuth 2.0",
    request_body = TokenRequest,
    responses(
        (status = 200, description = "Tokens issued", body = TokenResponse),
        (status = 400, description = "Invalid request", body = OAuthError),
        (status = 401, description = "Invalid client", body = OAuthError),
        (status = 403, description = "principal_kind not allowed for this client", body = OAuthError)
    )
)]
pub async fn token(
    State(state): State<AppState>,
    Form(request): Form<TokenRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<OAuthError>)> {
    // client_id is required for all grants except some legacy public clients.
    // We look the client up first so we can enforce credential checks for
    // confidential clients even when the request omits client_secret — the
    // previous "if both present" gate let confidential clients be downgraded
    // to public by simply omitting the secret. (SEC-001)
    let client_id = request.client_id.as_ref().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(OAuthError::invalid_request("client_id required")),
        )
    })?;

    let client = state
        .oauth_service
        .get_client(client_id)
        .await
        .map_err(|e| (StatusCode::UNAUTHORIZED, Json(e.into())))?
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(OAuthError::invalid_client("unknown or inactive client")),
            )
        })?;

    if client.is_confidential {
        let client_secret = request.client_secret.as_ref().ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(OAuthError::invalid_client(
                    "client_secret required for confidential clients",
                )),
            )
        })?;
        state
            .oauth_service
            .validate_client_credentials(client_id, client_secret)
            .await
            .map_err(|e| (StatusCode::UNAUTHORIZED, Json(e.into())))?;
    } else if let Some(client_secret) = request.client_secret.as_ref() {
        // Public client that supplied a secret anyway — still validate it
        // so a misconfigured public client surfaces the mismatch.
        state
            .oauth_service
            .validate_client_credentials(client_id, client_secret)
            .await
            .map_err(|e| (StatusCode::UNAUTHORIZED, Json(e.into())))?;
    }

    // Phase 6 C17 + review R5: audit the principal_kind rejection as a real
    // audit_log row (not just a tracing line) so it surfaces in the unified
    // audit viewer and triggers any platform-admin alerts (leak #21 defense).
    // Both grant branches share this error shape — handled via the helper.
    use crate::services::OAuthServiceError;

    let response = match request.grant_type.as_str() {
        "authorization_code" => {
            match state.oauth_service.exchange_code_for_tokens(&request).await {
                Ok(r) => r,
                Err(e) => {
                    let status = match &e {
                        OAuthServiceError::PrincipalKindNotAllowed => StatusCode::FORBIDDEN,
                        _ => StatusCode::BAD_REQUEST,
                    };
                    if matches!(e, OAuthServiceError::PrincipalKindNotAllowed) {
                        audit_token_denied_principal_kind(
                            &state,
                            request.client_id.as_deref().unwrap_or("<unknown>"),
                            "authorization_code",
                        )
                        .await;
                    }
                    return Err((status, Json(e.into())));
                }
            }
        }
        "refresh_token" => {
            let refresh_token = request.refresh_token.as_ref().ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(OAuthError::invalid_request("refresh_token required")),
                )
            })?;

            // client_id already extracted+verified above.
            match state
                .oauth_service
                .refresh_tokens(refresh_token, client_id)
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    let status = match &e {
                        OAuthServiceError::PrincipalKindNotAllowed => StatusCode::FORBIDDEN,
                        _ => StatusCode::BAD_REQUEST,
                    };
                    if matches!(e, OAuthServiceError::PrincipalKindNotAllowed) {
                        audit_token_denied_principal_kind(&state, client_id, "refresh_token").await;
                    }
                    return Err((status, Json(e.into())));
                }
            }
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(OAuthError::invalid_request(&format!(
                    "Unsupported grant_type: {}",
                    request.grant_type
                ))),
            ));
        }
    };

    // RFC 6749 Section 5.1: Token responses must include Cache-Control and Pragma headers
    Ok((
        [
            (axum::http::header::CACHE_CONTROL, "no-store"),
            (axum::http::header::PRAGMA, "no-cache"),
        ],
        Json(response),
    ))
}

// ==================== Token Revocation ====================

/// Revoke a token (RFC 7009).
///
/// Requires client authentication (RFC 7009 §2.1): confidential clients must
/// present a valid `client_secret`, public clients authenticate by `client_id`
/// alone. The token is only revoked if it was issued to the authenticating
/// client — this prevents an unauthenticated caller (or another client) from
/// revoking arbitrary tokens, which would otherwise be a trivial DoS.
#[utoipa::path(
    post,
    path = "/api/v1/oauth/revoke",
    tag = "OAuth 2.0",
    request_body = RevokeTokenRequest,
    responses(
        (status = 200, description = "Token revoked"),
        (status = 400, description = "Invalid request", body = OAuthError),
        (status = 401, description = "Invalid client", body = OAuthError)
    )
)]
pub async fn revoke(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Form(request): Form<RevokeTokenRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<OAuthError>)> {
    let client_id = authenticate_revocation_client(
        &state,
        &headers,
        &request.client_id,
        &request.client_secret,
    )
    .await?;

    state
        .oauth_service
        .revoke_token(
            &request.token,
            request.token_type_hint.as_deref(),
            &client_id,
        )
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(e.into())))?;

    Ok(StatusCode::OK)
}

/// Authenticate the client calling the revocation endpoint and return its
/// `client_id`. Mirrors the `/token` endpoint policy: confidential clients must
/// supply a valid secret; public clients authenticate by `client_id` only.
async fn authenticate_revocation_client(
    state: &AppState,
    headers: &axum::http::HeaderMap,
    form_client_id: &Option<String>,
    form_client_secret: &Option<String>,
) -> Result<String, (StatusCode, Json<OAuthError>)> {
    // Basic-auth credentials take precedence over form fields, matching
    // `/introspect` and `extract_client_credentials`.
    let creds = extract_client_credentials(headers, form_client_id, form_client_secret);
    let (client_id, supplied_secret) = match creds {
        Some((id, secret)) => (id, Some(secret)),
        None => {
            let id = form_client_id.clone().ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(OAuthError::invalid_client("client authentication required")),
                )
            })?;
            (id, None)
        }
    };

    let client = state
        .oauth_service
        .get_client(&client_id)
        .await
        .map_err(|e| (StatusCode::UNAUTHORIZED, Json(e.into())))?
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(OAuthError::invalid_client("unknown or inactive client")),
            )
        })?;

    if client.is_confidential {
        let secret = supplied_secret.ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(OAuthError::invalid_client(
                    "client_secret required for confidential clients",
                )),
            )
        })?;
        state
            .oauth_service
            .validate_client_credentials(&client_id, &secret)
            .await
            .map_err(|e| (StatusCode::UNAUTHORIZED, Json(e.into())))?;
    } else if let Some(secret) = supplied_secret {
        // Public client that supplied a secret anyway — validate it so a
        // misconfigured client surfaces the mismatch instead of silently
        // succeeding.
        state
            .oauth_service
            .validate_client_credentials(&client_id, &secret)
            .await
            .map_err(|e| (StatusCode::UNAUTHORIZED, Json(e.into())))?;
    }

    Ok(client_id)
}

// ==================== Token Introspection ====================

/// Introspection request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct IntrospectionRequest {
    pub token: String,
    pub token_type_hint: Option<String>,
    /// Client ID for authentication (alternative to Basic auth)
    pub client_id: Option<String>,
    /// Client secret for authentication (alternative to Basic auth)
    pub client_secret: Option<String>,
}

/// Introspect a token (RFC 7662).
/// Requires client authentication via Basic auth or client credentials in form.
#[utoipa::path(
    post,
    path = "/api/v1/oauth/introspect",
    tag = "OAuth 2.0",
    request_body = IntrospectionRequest,
    responses(
        (status = 200, description = "Token info", body = IntrospectionResponse),
        (status = 401, description = "Invalid client", body = OAuthError)
    )
)]
pub async fn introspect(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Form(request): Form<IntrospectionRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<OAuthError>)> {
    // RFC 7662 Section 2.1: Introspection requires client authentication
    let (client_id, client_secret) =
        extract_client_credentials(&headers, &request.client_id, &request.client_secret)
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(OAuthError::invalid_client("Client authentication required")),
                )
            })?;

    // Validate client credentials
    state
        .oauth_service
        .validate_client_credentials(&client_id, &client_secret)
        .await
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(OAuthError::invalid_client("Invalid client credentials")),
            )
        })?;

    // Bind the response to the authenticated client: a token belonging to a
    // different client is reported as inactive (RFC 7662 §2.2), so a client
    // cannot enumerate another client's token metadata.
    let response = state
        .oauth_service
        .introspect_token(&request.token, &client_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(e.into())))?;

    Ok(Json(response))
}

// ==================== User Grant Management ====================

/// List user's authorized applications.
#[utoipa::path(
    get,
    path = "/api/v1/oauth/grants",
    tag = "OAuth 2.0",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "User's authorized apps", body = Vec<UserGrantWithClient>),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn list_user_grants(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let user_id = require_bearer_user_id(&state, &headers)?;

    let grants = state
        .oauth_service
        .list_user_grants(user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list user grants");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to list grants",
                )),
            )
        })?;

    Ok(Json(grants))
}

/// Revoke user's authorization for a client.
#[utoipa::path(
    delete,
    path = "/api/v1/oauth/grants/{client_id}",
    tag = "OAuth 2.0",
    params(("client_id" = String, Path, description = "OAuth client ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 204, description = "Grant revoked"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 404, description = "Grant not found", body = ErrorResponse)
    )
)]
pub async fn revoke_user_grant(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(client_id): axum::extract::Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let user_id = require_bearer_user_id(&state, &headers)?;

    let revoked = state
        .oauth_service
        .revoke_user_grant(user_id, &client_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to revoke user grant");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to revoke grant",
                )),
            )
        })?;

    if !revoked {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("GRANT_NOT_FOUND", "Grant not found")),
        ));
    }

    // Audit log
    if let Err(e) = state
        .audit_log_repo
        .create(CreateAuditLog {
            user_id: Some(user_id),
            action: AuditAction::OAuthRevoke,
            resource_type: Some("oauth_grant".to_string()),
            resource_id: None,
            org_id: None,
            details: Some(serde_json::json!({ "client_id": client_id })),
            old_values: None,
            new_values: None,
            ip_address: None,
            user_agent: None,
        })
        .await
    {
        tracing::warn!(error = %e, "Failed to create audit log for OAuth grant revocation");
    }

    Ok(StatusCode::NO_CONTENT)
}

// ==================== Admin: Client Management ====================

/// Register a new OAuth client (admin only).
#[utoipa::path(
    post,
    path = "/api/v1/admin/oauth/clients",
    tag = "OAuth 2.0 Admin",
    security(("bearer_auth" = [])),
    request_body = RegisterClientRequest,
    responses(
        (status = 201, description = "Client registered", body = RegisterClientResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse)
    )
)]
pub async fn register_client(
    _cap: RequireCapability,
    principal: api_core::extractors::principal::RequestPrincipal,
    State(state): State<AppState>,
    Json(request): Json<RegisterClientRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let response = state
        .oauth_service
        .register_client(request)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to register OAuth client");
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("REGISTRATION_FAILED", e.to_string())),
            )
        })?;

    // Audit log — use trusted server-side principal, not a JWT claim.
    let user_id: Option<Uuid> = Some(principal.user_id);
    if let Err(e) = state
        .audit_log_repo
        .create(CreateAuditLog {
            user_id,
            action: AuditAction::OAuthClientCreate,
            resource_type: Some("oauth_client".to_string()),
            resource_id: None,
            org_id: None,
            details: Some(serde_json::json!({
                "client_id": response.client_id,
                "name": response.name,
                "scopes": response.scopes,
            })),
            old_values: None,
            new_values: None,
            ip_address: None,
            user_agent: None,
        })
        .await
    {
        tracing::warn!(error = %e, "Failed to create audit log for OAuth client registration");
    }

    Ok((StatusCode::CREATED, Json(response)))
}

/// List all OAuth clients (admin only).
#[utoipa::path(
    get,
    path = "/api/v1/admin/oauth/clients",
    tag = "OAuth 2.0 Admin",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Client list", body = Vec<OAuthClientSummary>),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse)
    )
)]
pub async fn list_clients(
    _cap: RequireCapability,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let clients = state.oauth_service.list_clients().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to list OAuth clients");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to list clients",
            )),
        )
    })?;

    Ok(Json(clients))
}

/// Get OAuth client details (admin only).
#[utoipa::path(
    get,
    path = "/api/v1/admin/oauth/clients/{id}",
    tag = "OAuth 2.0 Admin",
    params(("id" = Uuid, Path, description = "Client UUID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Client details", body = OAuthClientSummary),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Client not found", body = ErrorResponse)
    )
)]
pub async fn get_client(
    _cap: RequireCapability,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let client = state.oauth_repo.find_client_by_id(id).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get OAuth client");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", "Failed to get client")),
        )
    })?;

    match client {
        Some(c) => Ok(Json(OAuthClientSummary::from(c))),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("CLIENT_NOT_FOUND", "Client not found")),
        )),
    }
}

/// Update OAuth client (admin only).
#[utoipa::path(
    patch,
    path = "/api/v1/admin/oauth/clients/{id}",
    tag = "OAuth 2.0 Admin",
    params(("id" = Uuid, Path, description = "Client UUID")),
    security(("bearer_auth" = [])),
    request_body = UpdateOAuthClient,
    responses(
        (status = 200, description = "Client updated", body = OAuthClientSummary),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Client not found", body = ErrorResponse)
    )
)]
pub async fn update_client(
    _cap: RequireCapability,
    principal: api_core::extractors::principal::RequestPrincipal,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(data): Json<UpdateOAuthClient>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Snapshot the pre-update client so the audit row can record the scope
    // delta (old -> new), the security-relevant part of a privileged change.
    let before = state.oauth_repo.find_client_by_id(id).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to load OAuth client before update");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to load client",
            )),
        )
    })?;
    let old_scopes: Option<Vec<String>> = before.as_ref().map(|c| c.scopes.0.clone());

    // The operator reason is audit-trail-only — never persisted to the
    // oauth_clients row — so capture it before handing `data` to the service.
    let reason = data.reason.as_ref().and_then(|r| {
        let trimmed = r.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    });

    let client = state
        .oauth_service
        .update_client(id, data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update OAuth client");
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("UPDATE_FAILED", e.to_string())),
            )
        })?;

    match client {
        Some(c) => {
            // Audit log — privileged mutation. Use the trusted server-side
            // principal, capture the scope delta, and persist the operator
            // reason supplied by the admin-web UI. Previously this handler
            // wrote no audit row at all (findings #768 / #800).
            let new_scopes = c.scopes.clone();
            let scopes_changed = old_scopes
                .as_ref()
                .map(|old| {
                    let mut a = old.clone();
                    let mut b = new_scopes.clone();
                    a.sort();
                    b.sort();
                    a != b
                })
                .unwrap_or(false);
            let user_id: Option<Uuid> = Some(principal.user_id);
            if let Err(e) = state
                .audit_log_repo
                .create(CreateAuditLog {
                    user_id,
                    action: AuditAction::OAuthClientUpdate,
                    resource_type: Some("oauth_client".to_string()),
                    resource_id: Some(id),
                    org_id: None,
                    details: Some(serde_json::json!({
                        "client_id": c.client_id,
                        "name": c.name,
                        "scopes": new_scopes,
                        "scopes_changed": scopes_changed,
                        "reason": reason,
                    })),
                    old_values: old_scopes.map(|s| serde_json::json!({ "scopes": s })),
                    new_values: Some(serde_json::json!({ "scopes": new_scopes })),
                    ip_address: None,
                    user_agent: None,
                })
                .await
            {
                tracing::warn!(error = %e, "Failed to create audit log for OAuth client update");
            }

            Ok(Json(c))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("CLIENT_NOT_FOUND", "Client not found")),
        )),
    }
}

/// Revoke/deactivate OAuth client (admin only).
#[utoipa::path(
    delete,
    path = "/api/v1/admin/oauth/clients/{id}",
    tag = "OAuth 2.0 Admin",
    params(("id" = Uuid, Path, description = "Client UUID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 204, description = "Client revoked"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Client not found", body = ErrorResponse)
    )
)]
pub async fn revoke_client(
    _cap: RequireCapability,
    principal: api_core::extractors::principal::RequestPrincipal,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let revoked = state.oauth_service.revoke_client(id).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to revoke OAuth client");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("REVOKE_FAILED", e.to_string())),
        )
    })?;

    if !revoked {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("CLIENT_NOT_FOUND", "Client not found")),
        ));
    }

    // Audit log — use trusted server-side principal, not a JWT claim.
    let user_id: Option<Uuid> = Some(principal.user_id);
    if let Err(e) = state
        .audit_log_repo
        .create(CreateAuditLog {
            user_id,
            action: AuditAction::OAuthClientRevoke,
            resource_type: Some("oauth_client".to_string()),
            resource_id: Some(id),
            org_id: None,
            details: None,
            old_values: None,
            new_values: None,
            ip_address: None,
            user_agent: None,
        })
        .await
    {
        tracing::warn!(error = %e, "Failed to create audit log for OAuth client revocation");
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Regenerate client secret response.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegenerateSecretResponse {
    pub client_secret: String,
}

/// Regenerate client secret (admin only).
#[utoipa::path(
    post,
    path = "/api/v1/admin/oauth/clients/{id}/regenerate-secret",
    tag = "OAuth 2.0 Admin",
    params(("id" = Uuid, Path, description = "Client UUID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Secret regenerated", body = RegenerateSecretResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Client not found", body = ErrorResponse)
    )
)]
pub async fn regenerate_client_secret(
    _cap: RequireCapability,
    principal: api_core::extractors::principal::RequestPrincipal,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let secret = state
        .oauth_service
        .regenerate_client_secret(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to regenerate client secret");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("REGENERATE_FAILED", e.to_string())),
            )
        })?;

    // Audit log — use trusted server-side principal, not a JWT claim.
    let user_id: Option<Uuid> = Some(principal.user_id);
    if let Err(e) = state
        .audit_log_repo
        .create(CreateAuditLog {
            user_id,
            action: AuditAction::OAuthClientSecretRegenerate,
            resource_type: Some("oauth_client".to_string()),
            resource_id: Some(id),
            org_id: None,
            details: None,
            old_values: None,
            new_values: None,
            ip_address: None,
            user_agent: None,
        })
        .await
    {
        tracing::warn!(error = %e, "Failed to create audit log for client secret regeneration");
    }

    Ok(Json(RegenerateSecretResponse {
        client_secret: secret,
    }))
}

// ==================== Helper Functions ====================

/// Extract bearer token from Authorization header.
fn extract_bearer_token(
    headers: &axum::http::HeaderMap,
) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
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

    if !auth_header.starts_with("Bearer ") {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "INVALID_TOKEN_FORMAT",
                "Bearer token required",
            )),
        ));
    }

    Ok(auth_header[7..].to_string())
}

/// Extract client credentials from Basic auth header or form parameters.
/// Returns (client_id, client_secret) if found.
fn extract_client_credentials(
    headers: &axum::http::HeaderMap,
    form_client_id: &Option<String>,
    form_client_secret: &Option<String>,
) -> Option<(String, String)> {
    // Try Basic auth first
    if let Some(auth_header) = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
    {
        if let Some(encoded) = auth_header.strip_prefix("Basic ") {
            if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(encoded) {
                if let Ok(credentials) = String::from_utf8(decoded) {
                    if let Some((id, secret)) = credentials.split_once(':') {
                        return Some((id.to_string(), secret.to_string()));
                    }
                }
            }
        }
    }

    // Fall back to form parameters
    if let (Some(client_id), Some(client_secret)) = (form_client_id, form_client_secret) {
        return Some((client_id.clone(), client_secret.clone()));
    }

    None
}

/// JWT claims structure (only the `sub` field is used in this module; the
/// struct is kept minimal to avoid carrying stale copies of email/roles that
/// no handler in this file ever reads).
#[derive(Debug)]
struct JwtClaims {
    sub: String,
}

/// Validate access token and return minimal claims.
fn validate_access_token(
    state: &AppState,
    token: &str,
) -> Result<JwtClaims, (StatusCode, Json<ErrorResponse>)> {
    state
        .jwt_service
        .validate_access_token(token)
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "INVALID_TOKEN",
                    "Token is invalid or expired",
                )),
            )
        })
        .map(|claims| JwtClaims { sub: claims.sub })
}

/// Extract and validate the bearer token, then parse the caller's `Uuid`.
///
/// This three-step sequence (extract -> validate -> parse sub as Uuid) appears in
/// every user-authenticated handler in this module. Centralising it ensures the
/// error messages and HTTP statuses stay consistent and that future changes only
/// need to land in one place.
fn require_bearer_user_id(
    state: &AppState,
    headers: &axum::http::HeaderMap,
) -> Result<Uuid, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(headers)?;
    let claims = validate_access_token(state, &token)?;
    claims.sub.parse::<Uuid>().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })
}

/// Emit an audit log row for a `PrincipalKindNotAllowed` token denial and log a
/// structured warning.
///
/// The `token` handler's two grant branches (`authorization_code` and
/// `refresh_token`) both need this identical block. Extracting it keeps the
/// hot-path error path in one place so future policy changes (e.g. adding
/// `ip_address` or changing the audit action) require a single edit.
async fn audit_token_denied_principal_kind(state: &AppState, client_id: &str, grant_type: &str) {
    let _ = state
        .audit_log_repo
        .create(CreateAuditLog {
            user_id: None,
            action: AuditAction::OAuthTokenDeniedPrincipalKind,
            resource_type: Some("oauth_token".to_string()),
            resource_id: None,
            org_id: None,
            details: Some(serde_json::json!({
                "client_id": client_id,
                "grant_type": grant_type,
                "reason": "principal_kind_not_allowed_for_client",
            })),
            old_values: None,
            new_values: None,
            ip_address: None,
            user_agent: None,
        })
        .await;
    tracing::warn!(
        client_id = %client_id,
        grant_type = %grant_type,
        "OAuth token denied: principal_kind_not_allowed_for_client"
    );
}
