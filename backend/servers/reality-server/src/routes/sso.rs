//! SSO routes for Reality Portal (Epic 10A-SSO).
//!
//! Implements OIDC consumer flow to authenticate users via Property Management OAuth provider.
//! Supports both web-based authorization code flow and mobile deep-link token flow.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::state::AppState;

/// Create SSO router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Web SSO flow
        .route("/login", get(sso_login))
        .route("/callback", get(sso_callback))
        .route("/logout", post(sso_logout))
        // Mobile deep-link SSO
        .route("/mobile/token", post(create_mobile_sso_token))
        .route("/mobile/validate", post(validate_mobile_sso_token))
        // Session management
        .route("/session", get(get_session))
        .route("/refresh", post(refresh_session))
        // Story 96.3: SSO Token Exchange & Session Sync
        .route("/exchange", post(exchange_pm_token))
        .route("/sync", post(sync_session))
        .route("/roles", get(get_mapped_roles))
}

// ==================== Web SSO Flow ====================

/// SSO login query parameters.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SsoLoginQuery {
    /// Where to redirect after successful login
    pub redirect_uri: Option<String>,
    /// Optional state for CSRF protection
    pub state: Option<String>,
}

/// Initiate SSO login - redirects to PM OAuth provider.
#[utoipa::path(
    get,
    path = "/api/v1/sso/login",
    tag = "SSO",
    params(
        ("redirect_uri" = Option<String>, Query, description = "Post-login redirect URI"),
        ("state" = Option<String>, Query, description = "CSRF state token")
    ),
    responses(
        (status = 302, description = "Redirect to PM OAuth authorize endpoint")
    )
)]
pub async fn sso_login(
    State(state): State<AppState>,
    Query(params): Query<SsoLoginQuery>,
) -> impl IntoResponse {
    // Generate PKCE code verifier and challenge
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    // Store code verifier in session (to be retrieved in callback)
    let session_id = uuid::Uuid::new_v4().to_string();

    // Store session data for callback
    state.sso_sessions.lock().await.insert(
        session_id.clone(),
        PendingSsoSession {
            code_verifier,
            redirect_uri: params.redirect_uri.clone(),
            state: params.state.clone(),
            created_at: chrono::Utc::now(),
        },
    );

    // Build OAuth authorize URL
    let oauth_authorize_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        state.config.pm_oauth_authorize_url,
        state.config.pm_client_id,
        urlencoding::encode(&state.config.sso_callback_url),
        urlencoding::encode("profile email"),
        urlencoding::encode(&session_id),
        urlencoding::encode(&code_challenge),
    );

    Redirect::temporary(&oauth_authorize_url)
}

/// SSO callback query parameters.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SsoCallbackQuery {
    /// Authorization code from PM OAuth
    pub code: Option<String>,
    /// State (session ID) for CSRF verification
    pub state: Option<String>,
    /// Error code if authorization failed
    pub error: Option<String>,
    /// Error description
    pub error_description: Option<String>,
}

/// SSO callback - exchanges authorization code for tokens.
#[utoipa::path(
    get,
    path = "/api/v1/sso/callback",
    tag = "SSO",
    params(
        ("code" = Option<String>, Query, description = "Authorization code"),
        ("state" = Option<String>, Query, description = "State token"),
        ("error" = Option<String>, Query, description = "Error code"),
        ("error_description" = Option<String>, Query, description = "Error description")
    ),
    responses(
        (status = 302, description = "Redirect to original destination with session"),
        (status = 400, description = "Invalid callback parameters", body = SsoError)
    )
)]
pub async fn sso_callback(
    State(state): State<AppState>,
    Query(params): Query<SsoCallbackQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<SsoError>)> {
    // Check for OAuth errors
    if let Some(error) = params.error {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(SsoError {
                error,
                error_description: params.error_description,
            }),
        ));
    }

    // Validate required parameters
    let code = params.code.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(SsoError::new("missing_code", "Authorization code required")),
        )
    })?;

    let session_id = params.state.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(SsoError::new("missing_state", "State parameter required")),
        )
    })?;

    // Retrieve and remove pending session
    let pending_session = state
        .sso_sessions
        .lock()
        .await
        .remove(&session_id)
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(SsoError::new("invalid_state", "Invalid or expired session")),
            )
        })?;

    // Check session expiry (10 minutes max)
    if chrono::Utc::now() - pending_session.created_at > chrono::Duration::minutes(10) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(SsoError::new("session_expired", "SSO session expired")),
        ));
    }

    // Exchange code for tokens with PM OAuth server
    let tokens = exchange_code_for_tokens(&state, &code, &pending_session.code_verifier)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(SsoError::new("token_exchange_failed", &e.to_string())),
            )
        })?;

    // Get user info from PM
    let user_info = get_user_info(&state, &tokens.access_token)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(SsoError::new("user_info_failed", &e.to_string())),
            )
        })?;

    // Create or update local portal user
    let portal_user = state
        .user_service
        .upsert_sso_user(&user_info)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SsoError::new("user_create_failed", &e.to_string())),
            )
        })?;

    // Create portal session
    let session_token = state
        .session_service
        .create_session(portal_user.id, &tokens)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SsoError::new("session_create_failed", &e.to_string())),
            )
        })?;

    // Build redirect URL with session cookie
    let redirect_uri = pending_session
        .redirect_uri
        .unwrap_or_else(|| "/".to_string());

    // Set session cookie and redirect
    Ok((
        [(
            axum::http::header::SET_COOKIE,
            format!(
                "portal_session={}; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age={}",
                session_token,
                7 * 24 * 60 * 60 // 7 days
            ),
        )],
        Redirect::temporary(&redirect_uri),
    ))
}

/// Logout from SSO session.
#[utoipa::path(
    post,
    path = "/api/v1/sso/logout",
    tag = "SSO",
    responses(
        (status = 200, description = "Logged out successfully"),
        (status = 401, description = "Not authenticated")
    )
)]
pub async fn sso_logout(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    // Extract session token from cookie
    let session_token = extract_session_cookie(&headers).ok_or(StatusCode::UNAUTHORIZED)?;

    // Invalidate session
    state
        .session_service
        .invalidate_session(&session_token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Clear session cookie
    Ok((
        [(
            axum::http::header::SET_COOKIE,
            "portal_session=; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=0".to_string(),
        )],
        StatusCode::OK,
    ))
}

// ==================== Mobile Deep-Link SSO ====================

/// Request to create mobile SSO token.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateMobileSsoTokenRequest {
    /// PM access token to verify user identity
    pub pm_access_token: String,
}

/// Mobile SSO token response.
#[derive(Debug, Serialize, ToSchema)]
pub struct MobileSsoTokenResponse {
    /// Short-lived SSO token for deep-link
    pub sso_token: String,
    /// Token expiry in seconds (5 minutes)
    pub expires_in: u64,
    /// Deep-link URL format
    pub deep_link: String,
}

/// Create a short-lived mobile SSO token.
#[utoipa::path(
    post,
    path = "/api/v1/sso/mobile/token",
    tag = "SSO",
    request_body = CreateMobileSsoTokenRequest,
    responses(
        (status = 200, description = "SSO token created", body = MobileSsoTokenResponse),
        (status = 401, description = "Invalid PM token", body = SsoError)
    )
)]
pub async fn create_mobile_sso_token(
    State(state): State<AppState>,
    Json(request): Json<CreateMobileSsoTokenRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<SsoError>)> {
    // Validate PM access token by introspecting it
    let token_info = introspect_pm_token(&state, &request.pm_access_token)
        .await
        .map_err(|e| {
            (
                StatusCode::UNAUTHORIZED,
                Json(SsoError::new("invalid_token", &e.to_string())),
            )
        })?;

    if !token_info.active {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(SsoError::new("token_inactive", "PM token is not active")),
        ));
    }

    // Get user info
    let user_info = get_user_info(&state, &request.pm_access_token)
        .await
        .map_err(|e| {
            (
                StatusCode::UNAUTHORIZED,
                Json(SsoError::new("user_info_failed", &e.to_string())),
            )
        })?;

    // Create short-lived SSO token (5 minutes, one-time use)
    let sso_token = state
        .sso_token_service
        .create_mobile_token(&user_info, chrono::Duration::minutes(5))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SsoError::new("token_create_failed", &e.to_string())),
            )
        })?;

    Ok(Json(MobileSsoTokenResponse {
        sso_token: sso_token.clone(),
        expires_in: 300, // 5 minutes
        deep_link: format!("reality://sso?token={}", urlencoding::encode(&sso_token)),
    }))
}

/// Request to validate mobile SSO token.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ValidateMobileSsoTokenRequest {
    /// SSO token from deep-link
    pub sso_token: String,
}

/// Session response after mobile SSO validation.
#[derive(Debug, Serialize, ToSchema)]
pub struct SessionResponse {
    /// Session token for API authentication
    pub session_token: String,
    /// User information
    pub user: SsoUserInfo,
    /// Session expiry in seconds
    pub expires_in: u64,
}

/// Validate mobile SSO token and create session.
#[utoipa::path(
    post,
    path = "/api/v1/sso/mobile/validate",
    tag = "SSO",
    request_body = ValidateMobileSsoTokenRequest,
    responses(
        (status = 200, description = "Session created", body = SessionResponse),
        (status = 401, description = "Invalid or expired token", body = SsoError)
    )
)]
pub async fn validate_mobile_sso_token(
    State(state): State<AppState>,
    Json(request): Json<ValidateMobileSsoTokenRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<SsoError>)> {
    // Validate and consume SSO token (one-time use)
    let user_info = state
        .sso_token_service
        .validate_and_consume_token(&request.sso_token)
        .await
        .map_err(|e| {
            (
                StatusCode::UNAUTHORIZED,
                Json(SsoError::new("invalid_token", &e.to_string())),
            )
        })?;

    // Create or update portal user
    let portal_user = state
        .user_service
        .upsert_sso_user(&user_info)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SsoError::new("user_create_failed", &e.to_string())),
            )
        })?;

    // Create session
    let session_token = state
        .session_service
        .create_mobile_session(portal_user.id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SsoError::new("session_create_failed", &e.to_string())),
            )
        })?;

    Ok(Json(SessionResponse {
        session_token,
        user: user_info,
        expires_in: 7 * 24 * 60 * 60, // 7 days
    }))
}

// ==================== Session Management ====================

/// Get current session information.
#[utoipa::path(
    get,
    path = "/api/v1/sso/session",
    tag = "SSO",
    responses(
        (status = 200, description = "Session info", body = SessionInfo),
        (status = 401, description = "Not authenticated")
    )
)]
pub async fn get_session(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    let session_token = extract_session_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;

    let session_info = state
        .session_service
        .get_session(&session_token)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    Ok(Json(session_info))
}

/// Refresh session with PM tokens.
#[utoipa::path(
    post,
    path = "/api/v1/sso/refresh",
    tag = "SSO",
    responses(
        (status = 200, description = "Session refreshed", body = SessionInfo),
        (status = 401, description = "Session expired or invalid")
    )
)]
pub async fn refresh_session(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    let session_token = extract_session_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;

    let session_info = state
        .session_service
        .refresh_session(&session_token)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    Ok(Json(session_info))
}

// ==================== Types ====================

/// SSO error response.
#[derive(Debug, Serialize, ToSchema)]
pub struct SsoError {
    pub error: String,
    pub error_description: Option<String>,
}

impl SsoError {
    pub fn new(error: &str, description: &str) -> Self {
        Self {
            error: error.to_string(),
            error_description: Some(description.to_string()),
        }
    }
}

/// User info from PM OAuth.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SsoUserInfo {
    pub user_id: String,
    pub email: String,
    pub name: String,
    pub avatar_url: Option<String>,
}

/// Session information.
#[derive(Debug, Serialize, ToSchema)]
pub struct SessionInfo {
    pub user_id: uuid::Uuid,
    pub email: String,
    pub name: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Pending SSO session for PKCE flow.
#[derive(Debug)]
pub struct PendingSsoSession {
    pub code_verifier: String,
    pub redirect_uri: Option<String>,
    pub state: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Token response from PM OAuth.
#[derive(Debug, Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub expires_in: u64,
}

/// Token introspection response.
#[derive(Debug, Deserialize)]
pub struct TokenIntrospectionResponse {
    pub active: bool,
    pub sub: Option<String>,
    pub client_id: Option<String>,
    pub scope: Option<String>,
}

// ==================== Helper Functions ====================

/// Generate PKCE code verifier (43-128 chars).
///
/// Uses OS CSPRNG directly — the verifier is the proof-of-possession secret
/// that makes PKCE effective against authorization-code interception attacks,
/// so it must be unpredictable even under adversarial timing.
fn generate_code_verifier() -> String {
    use rand::TryRng;
    let mut bytes = [0u8; 32];
    rand::rngs::SysRng
        .try_fill_bytes(&mut bytes)
        .expect("OS rng failed");
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
}

/// Generate PKCE code challenge from verifier.
fn generate_code_challenge(verifier: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(verifier.as_bytes());
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, hash)
}

/// Exchange authorization code for tokens.
async fn exchange_code_for_tokens(
    state: &AppState,
    code: &str,
    code_verifier: &str,
) -> Result<OAuthTokens, anyhow::Error> {
    let client = reqwest::Client::new();
    let response = client
        .post(&state.config.pm_oauth_token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &state.config.sso_callback_url),
            ("client_id", &state.config.pm_client_id),
            ("client_secret", &state.config.pm_client_secret),
            ("code_verifier", code_verifier),
        ])
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Token exchange failed: {}", error_text));
    }

    Ok(response.json().await?)
}

/// Get user info from PM OAuth server.
async fn get_user_info(state: &AppState, access_token: &str) -> Result<SsoUserInfo, anyhow::Error> {
    let client = reqwest::Client::new();
    let response = client
        .get(&state.config.pm_userinfo_url)
        .bearer_auth(access_token)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Failed to get user info: {}", error_text));
    }

    Ok(response.json().await?)
}

/// Introspect PM token with caching (Epic 104.2).
///
/// Uses cached validation result if available and not expired (60 second TTL).
/// This reduces load on the PM API and improves response times for repeated
/// token validations.
async fn introspect_pm_token(
    state: &AppState,
    token: &str,
) -> Result<TokenIntrospectionResponse, anyhow::Error> {
    // Story 104.2: Check cache first
    if let Some(cached) = state.token_cache.get(token).await {
        tracing::debug!(active = cached.active, "SSO token validation cache hit");
        return Ok(TokenIntrospectionResponse {
            active: cached.active,
            sub: cached.sub,
            client_id: None,
            scope: cached.scope,
        });
    }

    // Cache miss - perform actual introspection
    tracing::debug!("SSO token validation cache miss, calling PM API");
    let client = reqwest::Client::new();
    let response = client
        .post(&state.config.pm_introspect_url)
        .form(&[
            ("token", token),
            ("client_id", &state.config.pm_client_id),
            ("client_secret", &state.config.pm_client_secret),
        ])
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        // Cache inactive tokens too (to prevent repeated failed validations)
        state.token_cache.set(token, false, None, None).await;
        return Err(anyhow::anyhow!(
            "Token introspection failed: {}",
            error_text
        ));
    }

    let result: TokenIntrospectionResponse = response.json().await?;

    // Story 104.2: Cache the validation result
    state
        .token_cache
        .set(
            token,
            result.active,
            result.sub.clone(),
            result.scope.clone(),
        )
        .await;

    tracing::debug!(active = result.active, "SSO token validated and cached");

    Ok(result)
}

/// Extract session cookie from headers.
fn extract_session_cookie(headers: &axum::http::HeaderMap) -> Option<String> {
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

/// Extract session token from Authorization header or cookie.
fn extract_session_token(headers: &axum::http::HeaderMap) -> Option<String> {
    // Try Authorization header first
    if let Some(auth) = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(token) = auth.strip_prefix("Bearer ") {
            return Some(token.to_string());
        }
    }

    // Fall back to cookie
    extract_session_cookie(headers)
}

// ==================== Story 96.3: SSO Token Exchange & Session Sync ====================

/// PM role to Reality Portal role mapping.
/// Maps Property Management roles to Reality Portal access levels.
pub mod role_mapping {
    /// PM role constants.
    pub mod pm_roles {
        pub const OWNER: &str = "owner";
        pub const MANAGER: &str = "manager";
        pub const TECHNICAL_MANAGER: &str = "technical_manager";
        pub const TENANT: &str = "tenant";
        pub const RESIDENT: &str = "resident";
        pub const PROPERTY_MANAGER: &str = "property_manager";
        pub const REAL_ESTATE_AGENT: &str = "real_estate_agent";
    }

    /// Reality Portal access levels.
    pub mod portal_roles {
        pub const AGENT: &str = "agent"; // Can manage listings
        pub const PROPERTY_OWNER: &str = "property_owner"; // Can list own properties
        pub const VERIFIED_USER: &str = "verified_user"; // Verified identity
        pub const USER: &str = "user"; // Basic portal user
    }

    /// Map PM role to Reality Portal role.
    pub fn map_pm_role_to_portal(pm_role: &str) -> &'static str {
        match pm_role {
            "real_estate_agent" => portal_roles::AGENT,
            "property_manager" => portal_roles::AGENT,
            "manager" => portal_roles::PROPERTY_OWNER,
            "owner" => portal_roles::PROPERTY_OWNER,
            "technical_manager" => portal_roles::VERIFIED_USER,
            "tenant" | "resident" => portal_roles::USER,
            _ => portal_roles::USER,
        }
    }

    /// Check if a PM role grants listing management access.
    pub fn can_manage_listings(pm_role: &str) -> bool {
        matches!(
            pm_role,
            "real_estate_agent" | "property_manager" | "manager" | "owner"
        )
    }

    /// Get all permissions for a portal role.
    pub fn get_portal_permissions(portal_role: &str) -> Vec<&'static str> {
        match portal_role {
            portal_roles::AGENT => vec![
                "listings:create",
                "listings:update",
                "listings:delete",
                "inquiries:manage",
                "analytics:view",
            ],
            portal_roles::PROPERTY_OWNER => {
                vec!["listings:create", "listings:update", "inquiries:view"]
            }
            portal_roles::VERIFIED_USER => vec!["inquiries:create", "favorites:manage"],
            portal_roles::USER => vec!["favorites:manage"],
            _ => vec![],
        }
    }
}

/// Request to exchange PM token for Reality Portal session.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ExchangeTokenRequest {
    /// PM access token to exchange.
    pub pm_access_token: String,
    /// Optional: specific PM roles to include (for role filtering).
    pub roles: Option<Vec<String>>,
}

/// Response with Reality Portal session and mapped roles.
#[derive(Debug, Serialize, ToSchema)]
pub struct ExchangeTokenResponse {
    /// Reality Portal session token.
    pub session_token: String,
    /// User information.
    pub user: SsoUserInfo,
    /// Mapped Reality Portal role.
    pub portal_role: String,
    /// Permissions granted.
    pub permissions: Vec<String>,
    /// Original PM roles.
    pub pm_roles: Vec<String>,
    /// Session expiry in seconds.
    pub expires_in: u64,
}

/// Exchange PM access token for Reality Portal session.
///
/// This endpoint allows PM users to access Reality Portal with their existing
/// credentials, mapping PM roles to appropriate portal permissions.
#[utoipa::path(
    post,
    path = "/api/v1/sso/exchange",
    tag = "SSO",
    request_body = ExchangeTokenRequest,
    responses(
        (status = 200, description = "Token exchanged successfully", body = ExchangeTokenResponse),
        (status = 401, description = "Invalid PM token", body = SsoError)
    )
)]
pub async fn exchange_pm_token(
    State(state): State<AppState>,
    Json(request): Json<ExchangeTokenRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<SsoError>)> {
    tracing::info!("Exchanging PM token for Reality Portal session");

    // Introspect PM token to validate and get user info
    let token_info = introspect_pm_token(&state, &request.pm_access_token)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "Failed to introspect PM token");
            (
                StatusCode::UNAUTHORIZED,
                Json(SsoError::new("invalid_token", &e.to_string())),
            )
        })?;

    if !token_info.active {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(SsoError::new("token_inactive", "PM token is not active")),
        ));
    }

    // Get user info from PM
    let user_info = get_user_info(&state, &request.pm_access_token)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "Failed to get user info from PM");
            (
                StatusCode::UNAUTHORIZED,
                Json(SsoError::new("user_info_failed", &e.to_string())),
            )
        })?;

    // Get PM roles from token scope or fetch from PM API
    let pm_roles = extract_roles_from_scope(token_info.scope.as_deref())
        .or_else(|| request.roles.clone())
        .unwrap_or_default();

    // Map PM roles to portal role (use highest privilege)
    let portal_role = pm_roles
        .iter()
        .map(|r| role_mapping::map_pm_role_to_portal(r))
        .max_by_key(|r| match *r {
            role_mapping::portal_roles::AGENT => 4,
            role_mapping::portal_roles::PROPERTY_OWNER => 3,
            role_mapping::portal_roles::VERIFIED_USER => 2,
            _ => 1,
        })
        .unwrap_or(role_mapping::portal_roles::USER);

    let permissions: Vec<String> = role_mapping::get_portal_permissions(portal_role)
        .iter()
        .map(|s| s.to_string())
        .collect();

    // Create or update portal user
    let portal_user = state
        .user_service
        .upsert_sso_user(&user_info)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create portal user");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SsoError::new("user_create_failed", &e.to_string())),
            )
        })?;

    // Create session (role is stored with user, not in session)
    let session_token = state
        .session_service
        .create_mobile_session(portal_user.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create session");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SsoError::new("session_create_failed", &e.to_string())),
            )
        })?;

    tracing::info!(
        user_id = %user_info.user_id,
        portal_role = %portal_role,
        pm_roles = ?pm_roles,
        "PM token exchanged successfully"
    );

    Ok(Json(ExchangeTokenResponse {
        session_token,
        user: user_info,
        portal_role: portal_role.to_string(),
        permissions,
        pm_roles,
        expires_in: 7 * 24 * 60 * 60, // 7 days
    }))
}

/// Request to sync session between PM and Reality Portal.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SyncSessionRequest {
    /// PM session token or access token.
    pub pm_token: String,
    /// Current Reality Portal session token (if exists).
    pub portal_session: Option<String>,
}

/// Session sync response.
#[derive(Debug, Serialize, ToSchema)]
pub struct SyncSessionResponse {
    /// Whether sync was successful.
    pub synced: bool,
    /// Updated or new portal session token.
    pub session_token: String,
    /// Whether session was refreshed.
    pub refreshed: bool,
    /// Session status.
    pub status: String,
}

/// Synchronize session state between PM and Reality Portal.
///
/// This ensures that logout in PM invalidates the Reality Portal session,
/// and that role changes are propagated.
#[utoipa::path(
    post,
    path = "/api/v1/sso/sync",
    tag = "SSO",
    request_body = SyncSessionRequest,
    responses(
        (status = 200, description = "Session synchronized", body = SyncSessionResponse),
        (status = 401, description = "PM session invalid", body = SsoError)
    )
)]
pub async fn sync_session(
    State(state): State<AppState>,
    Json(request): Json<SyncSessionRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<SsoError>)> {
    tracing::info!("Syncing session between PM and Reality Portal");

    // Validate PM token is still active
    let token_info = introspect_pm_token(&state, &request.pm_token)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "PM token introspection failed during sync");
            (
                StatusCode::UNAUTHORIZED,
                Json(SsoError::new("pm_session_invalid", &e.to_string())),
            )
        })?;

    // If PM token is inactive, invalidate portal session
    if !token_info.active {
        if let Some(portal_session) = &request.portal_session {
            let _ = state
                .session_service
                .invalidate_session(portal_session)
                .await;
            tracing::info!("Invalidated portal session due to inactive PM token");
        }

        return Err((
            StatusCode::UNAUTHORIZED,
            Json(SsoError::new(
                "pm_session_expired",
                "PM session has expired, portal session invalidated",
            )),
        ));
    }

    // Get current user info
    let user_info = get_user_info(&state, &request.pm_token)
        .await
        .map_err(|e| {
            (
                StatusCode::UNAUTHORIZED,
                Json(SsoError::new("user_info_failed", &e.to_string())),
            )
        })?;

    // If portal session exists, refresh it; otherwise create new
    let (session_token, refreshed) = if let Some(portal_session) = &request.portal_session {
        match state.session_service.refresh_session(portal_session).await {
            Ok(_info) => (portal_session.clone(), true),
            Err(_) => {
                // Session invalid, create new one
                let portal_user = state
                    .user_service
                    .upsert_sso_user(&user_info)
                    .await
                    .map_err(|e| {
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(SsoError::new("user_create_failed", &e.to_string())),
                        )
                    })?;

                let new_session = state
                    .session_service
                    .create_mobile_session(portal_user.id)
                    .await
                    .map_err(|e| {
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(SsoError::new("session_create_failed", &e.to_string())),
                        )
                    })?;

                (new_session, false)
            }
        }
    } else {
        // No portal session, create new one
        let portal_user = state
            .user_service
            .upsert_sso_user(&user_info)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(SsoError::new("user_create_failed", &e.to_string())),
                )
            })?;

        let new_session = state
            .session_service
            .create_mobile_session(portal_user.id)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(SsoError::new("session_create_failed", &e.to_string())),
                )
            })?;

        (new_session, false)
    };

    Ok(Json(SyncSessionResponse {
        synced: true,
        session_token,
        refreshed,
        status: "active".to_string(),
    }))
}

/// Mapped roles response.
#[derive(Debug, Serialize, ToSchema)]
pub struct MappedRolesResponse {
    /// PM role to portal role mappings.
    pub role_mappings: Vec<RoleMapping>,
    /// All available portal roles.
    pub portal_roles: Vec<PortalRoleInfo>,
}

/// Individual role mapping.
#[derive(Debug, Serialize, ToSchema)]
pub struct RoleMapping {
    /// PM role name.
    pub pm_role: String,
    /// Mapped portal role.
    pub portal_role: String,
    /// Whether this role can manage listings.
    pub can_manage_listings: bool,
}

/// Portal role information.
#[derive(Debug, Serialize, ToSchema)]
pub struct PortalRoleInfo {
    /// Role identifier.
    pub role: String,
    /// Role description.
    pub description: String,
    /// Permissions granted.
    pub permissions: Vec<String>,
}

/// Get role mappings between PM and Reality Portal.
///
/// Returns the mapping configuration for PM roles to Reality Portal roles,
/// useful for UI display and permission checking.
#[utoipa::path(
    get,
    path = "/api/v1/sso/roles",
    tag = "SSO",
    responses(
        (status = 200, description = "Role mappings", body = MappedRolesResponse)
    )
)]
pub async fn get_mapped_roles(State(_state): State<AppState>) -> impl IntoResponse {
    let role_mappings = vec![
        RoleMapping {
            pm_role: role_mapping::pm_roles::REAL_ESTATE_AGENT.to_string(),
            portal_role: role_mapping::portal_roles::AGENT.to_string(),
            can_manage_listings: true,
        },
        RoleMapping {
            pm_role: role_mapping::pm_roles::PROPERTY_MANAGER.to_string(),
            portal_role: role_mapping::portal_roles::AGENT.to_string(),
            can_manage_listings: true,
        },
        RoleMapping {
            pm_role: role_mapping::pm_roles::MANAGER.to_string(),
            portal_role: role_mapping::portal_roles::PROPERTY_OWNER.to_string(),
            can_manage_listings: true,
        },
        RoleMapping {
            pm_role: role_mapping::pm_roles::OWNER.to_string(),
            portal_role: role_mapping::portal_roles::PROPERTY_OWNER.to_string(),
            can_manage_listings: true,
        },
        RoleMapping {
            pm_role: role_mapping::pm_roles::TECHNICAL_MANAGER.to_string(),
            portal_role: role_mapping::portal_roles::VERIFIED_USER.to_string(),
            can_manage_listings: false,
        },
        RoleMapping {
            pm_role: role_mapping::pm_roles::TENANT.to_string(),
            portal_role: role_mapping::portal_roles::USER.to_string(),
            can_manage_listings: false,
        },
        RoleMapping {
            pm_role: role_mapping::pm_roles::RESIDENT.to_string(),
            portal_role: role_mapping::portal_roles::USER.to_string(),
            can_manage_listings: false,
        },
    ];

    let portal_roles = vec![
        PortalRoleInfo {
            role: role_mapping::portal_roles::AGENT.to_string(),
            description: "Real estate agent with full listing management".to_string(),
            permissions: role_mapping::get_portal_permissions(role_mapping::portal_roles::AGENT)
                .iter()
                .map(|s| s.to_string())
                .collect(),
        },
        PortalRoleInfo {
            role: role_mapping::portal_roles::PROPERTY_OWNER.to_string(),
            description: "Property owner who can list own properties".to_string(),
            permissions: role_mapping::get_portal_permissions(
                role_mapping::portal_roles::PROPERTY_OWNER,
            )
            .iter()
            .map(|s| s.to_string())
            .collect(),
        },
        PortalRoleInfo {
            role: role_mapping::portal_roles::VERIFIED_USER.to_string(),
            description: "Verified user with enhanced access".to_string(),
            permissions: role_mapping::get_portal_permissions(
                role_mapping::portal_roles::VERIFIED_USER,
            )
            .iter()
            .map(|s| s.to_string())
            .collect(),
        },
        PortalRoleInfo {
            role: role_mapping::portal_roles::USER.to_string(),
            description: "Basic portal user".to_string(),
            permissions: role_mapping::get_portal_permissions(role_mapping::portal_roles::USER)
                .iter()
                .map(|s| s.to_string())
                .collect(),
        },
    ];

    Json(MappedRolesResponse {
        role_mappings,
        portal_roles,
    })
}

/// Extract roles from OAuth scope string.
fn extract_roles_from_scope(scope: Option<&str>) -> Option<Vec<String>> {
    scope.map(|s| {
        s.split_whitespace()
            .filter(|part| part.starts_with("role:"))
            .map(|part| part.strip_prefix("role:").unwrap_or(part).to_string())
            .collect()
    })
}
