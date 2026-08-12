//! OAuth 2.0 Provider service (Epic 10A).
//!
//! This module implements OAuth 2.0 Authorization Server functionality
//! with PKCE support (RFC 7636).

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{Duration, Utc};
use db::models::oauth::{
    ConsentPageData, CreateAccessToken, CreateAuthorizationCode, CreateOAuthClient,
    CreateRefreshToken, CreateUserOAuthGrant, IntrospectionResponse, OAuthClient,
    OAuthClientSummary, OAuthError, OAuthScope, RegisterClientRequest, RegisterClientResponse,
    ScopeDisplay, TokenRequest, TokenResponse, UpdateOAuthClient, UserGrantWithClient,
};
use db::models::oauth_token_event::{CreateOAuthTokenEvent, OAuthTokenKind};
use db::repositories::{OAuthRepository, OAuthTokenEventRepository, UserRepository};
use rand::TryRng;
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use super::auth::AuthService;

/// OAuth service errors.
#[derive(Debug, Error)]
pub enum OAuthServiceError {
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Invalid client: {0}")]
    InvalidClient(String),

    #[error("Invalid redirect URI")]
    InvalidRedirectUri,

    #[error("Invalid scope: {0}")]
    InvalidScope(String),

    #[error("Invalid grant")]
    InvalidGrant,

    #[error("Invalid code verifier")]
    InvalidCodeVerifier,

    #[error("Authorization code expired")]
    CodeExpired,

    #[error("Authorization code already used")]
    CodeAlreadyUsed,

    #[error("Token expired")]
    TokenExpired,

    #[error("Token revoked")]
    TokenRevoked,

    #[error("Token reuse detected - security breach")]
    TokenReuseDetected,

    #[error("Unsupported grant type: {0}")]
    UnsupportedGrantType(String),

    #[error("Client not found")]
    ClientNotFound,

    /// Phase 6 C17: the user's principal_kind is not in the client's allowed list.
    #[error("principal_kind not allowed for this client")]
    PrincipalKindNotAllowed,

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Internal error")]
    InternalError,
}

impl From<OAuthServiceError> for OAuthError {
    fn from(e: OAuthServiceError) -> Self {
        match e {
            OAuthServiceError::InvalidRequest(msg) => OAuthError::invalid_request(&msg),
            OAuthServiceError::InvalidClient(msg) => OAuthError::invalid_client(&msg),
            OAuthServiceError::InvalidRedirectUri => {
                OAuthError::invalid_request("Invalid redirect URI")
            }
            OAuthServiceError::InvalidScope(scope) => {
                OAuthError::invalid_scope(&format!("Invalid scope: {}", scope))
            }
            OAuthServiceError::InvalidGrant => {
                OAuthError::invalid_grant("Invalid authorization code")
            }
            OAuthServiceError::InvalidCodeVerifier => {
                OAuthError::invalid_grant("Invalid code verifier")
            }
            OAuthServiceError::CodeExpired => {
                OAuthError::invalid_grant("Authorization code expired")
            }
            OAuthServiceError::CodeAlreadyUsed => {
                OAuthError::invalid_grant("Authorization code already used")
            }
            OAuthServiceError::TokenExpired => OAuthError::invalid_grant("Token expired"),
            OAuthServiceError::TokenRevoked => OAuthError::invalid_grant("Token revoked"),
            OAuthServiceError::TokenReuseDetected => {
                OAuthError::invalid_grant("Token reuse detected")
            }
            OAuthServiceError::UnsupportedGrantType(gt) => {
                OAuthError::invalid_request(&format!("Unsupported grant type: {}", gt))
            }
            OAuthServiceError::ClientNotFound => OAuthError::invalid_client("Client not found"),
            OAuthServiceError::PrincipalKindNotAllowed => {
                OAuthError::access_denied("principal_kind_not_allowed_for_client")
            }
            OAuthServiceError::DatabaseError(_) => OAuthError::server_error("Database error"),
            OAuthServiceError::InternalError => OAuthError::server_error("Internal error"),
        }
    }
}

/// Configuration for OAuth service.
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    /// Authorization code expiration in seconds (default: 600 = 10 minutes).
    pub code_expires_secs: i64,
    /// Access token expiration in seconds (default: 900 = 15 minutes).
    pub access_token_expires_secs: i64,
    /// Refresh token expiration in seconds (default: 604800 = 7 days).
    pub refresh_token_expires_secs: i64,
}

impl Default for OAuthConfig {
    fn default() -> Self {
        Self {
            code_expires_secs: 600,             // 10 minutes
            access_token_expires_secs: 900,     // 15 minutes
            refresh_token_expires_secs: 604800, // 7 days
        }
    }
}

/// OAuth 2.0 Provider service.
#[derive(Clone)]
pub struct OAuthService {
    repo: OAuthRepository,
    user_repo: UserRepository,
    auth_service: AuthService,
    config: OAuthConfig,
    /// Best-effort token-usage analytics sink (Epic 10A, #2628). `None` leaves
    /// the issuance / refresh / revocation paths untouched; when wired (see
    /// [`OAuthService::with_token_event_repo`]) each lifecycle transition
    /// records an event. A recording failure only `warn!`s — it never alters
    /// the token response.
    token_event_repo: Option<OAuthTokenEventRepository>,
}

impl OAuthService {
    /// Create a new OAuthService.
    pub fn new(
        repo: OAuthRepository,
        user_repo: UserRepository,
        auth_service: AuthService,
    ) -> Self {
        Self {
            repo,
            user_repo,
            auth_service,
            config: OAuthConfig::default(),
            token_event_repo: None,
        }
    }

    /// Create a new OAuthService with custom config.
    pub fn with_config(
        repo: OAuthRepository,
        user_repo: UserRepository,
        auth_service: AuthService,
        config: OAuthConfig,
    ) -> Self {
        Self {
            repo,
            user_repo,
            auth_service,
            config,
            token_event_repo: None,
        }
    }

    /// Attach the token-usage analytics recorder (Epic 10A, #2628).
    ///
    /// Production wires this in [`crate::state::AppState::new`]; tests that don't
    /// care about analytics can skip it and the lifecycle paths simply won't
    /// emit events.
    pub fn with_token_event_repo(mut self, repo: OAuthTokenEventRepository) -> Self {
        self.token_event_repo = Some(repo);
        self
    }

    /// Best-effort record of an OAuth token lifecycle event (Epic 10A, #2628).
    ///
    /// The analytics `INSERT` is dispatched to a **detached background task**
    /// ([`tokio::spawn`]) rather than awaited inline, so the DB round-trip never
    /// sits on the hot token path (#2643): token issuance / refresh / revocation
    /// return their `TokenResponse` without waiting on this write. `record`
    /// itself is called only from inside the spawned task.
    ///
    /// Errors are still swallowed: a recording failure is logged at `warn!` and
    /// otherwise ignored, matching the repository's documented "log-and-ignore"
    /// contract — it must never alter or fail the token flow.
    ///
    /// Returns the spawned task's [`JoinHandle`] when a recorder is wired
    /// (`None` when analytics is disabled). Production callers drop it —
    /// fire-and-forget; tests can `.await` it to deterministically observe the
    /// persisted row without racing the runtime.
    ///
    /// [`JoinHandle`]: tokio::task::JoinHandle
    fn record_token_event(
        &self,
        event: CreateOAuthTokenEvent,
    ) -> Option<tokio::task::JoinHandle<()>> {
        let repo = self.token_event_repo.clone()?;
        Some(tokio::spawn(async move {
            if let Err(e) = repo.record(event).await {
                tracing::warn!(
                    error = %e,
                    "Failed to record OAuth token-usage event (analytics only, ignored)"
                );
            }
        }))
    }

    // ==================== Client Management ====================

    /// Register a new OAuth client.
    pub async fn register_client(
        &self,
        request: RegisterClientRequest,
    ) -> Result<RegisterClientResponse, OAuthServiceError> {
        // Validate scopes
        self.validate_scopes(&request.scopes)?;

        // Generate client_id and client_secret
        let client_id = self.generate_random_b64(16);
        let client_secret = self.generate_random_b64(32);
        let client_secret_hash = self
            .auth_service
            .hash_password(&client_secret)
            .map_err(|_| OAuthServiceError::InternalError)?;

        let data = CreateOAuthClient {
            client_id: client_id.clone(),
            client_secret_hash,
            name: request.name.clone(),
            description: request.description.clone(),
            redirect_uris: request.redirect_uris.clone(),
            scopes: request.scopes.clone(),
            is_confidential: request.is_confidential.unwrap_or(true),
            rotate_refresh_tokens: request.rotate_refresh_tokens.unwrap_or(true),
        };

        let client = self.repo.create_client(data).await?;

        Ok(RegisterClientResponse {
            id: client.id,
            client_id,
            client_secret, // Plaintext, shown only once
            name: client.name,
            redirect_uris: client.redirect_uris.0,
            scopes: client.scopes.0,
            created_at: client.created_at,
        })
    }

    /// Get client by client_id for validation.
    pub async fn get_client(
        &self,
        client_id: &str,
    ) -> Result<Option<OAuthClient>, OAuthServiceError> {
        Ok(self.repo.find_active_client_by_client_id(client_id).await?)
    }

    /// List all OAuth clients.
    pub async fn list_clients(&self) -> Result<Vec<OAuthClientSummary>, OAuthServiceError> {
        let clients = self.repo.list_clients().await?;
        Ok(clients.into_iter().map(OAuthClientSummary::from).collect())
    }

    /// Update an OAuth client.
    pub async fn update_client(
        &self,
        id: Uuid,
        data: UpdateOAuthClient,
    ) -> Result<Option<OAuthClientSummary>, OAuthServiceError> {
        // Validate scopes if provided
        if let Some(ref scopes) = data.scopes {
            self.validate_scopes(scopes)?;
        }

        let client = self.repo.update_client(id, data).await?;
        Ok(client.map(OAuthClientSummary::from))
    }

    /// Regenerate client secret.
    pub async fn regenerate_client_secret(&self, id: Uuid) -> Result<String, OAuthServiceError> {
        let client_secret = self.generate_random_b64(32);
        let client_secret_hash = self
            .auth_service
            .hash_password(&client_secret)
            .map_err(|_| OAuthServiceError::InternalError)?;

        let updated = self
            .repo
            .update_client_secret(id, &client_secret_hash)
            .await?;

        if !updated {
            return Err(OAuthServiceError::ClientNotFound);
        }

        Ok(client_secret)
    }

    /// Revoke an OAuth client.
    pub async fn revoke_client(&self, id: Uuid) -> Result<bool, OAuthServiceError> {
        Ok(self.repo.revoke_client(id).await?)
    }

    /// Validate client credentials.
    pub async fn validate_client_credentials(
        &self,
        client_id: &str,
        client_secret: &str,
    ) -> Result<OAuthClient, OAuthServiceError> {
        let client = self.require_active_client(client_id).await?;

        let valid = self
            .auth_service
            .verify_password(client_secret, &client.client_secret_hash)
            .map_err(|_| OAuthServiceError::InternalError)?;

        if !valid {
            return Err(OAuthServiceError::InvalidClient(
                "Invalid client secret".to_string(),
            ));
        }

        Ok(client)
    }

    // ==================== Authorization Flow ====================

    /// Validate authorization request and return consent page data.
    pub async fn validate_authorize_request(
        &self,
        client_id: &str,
        redirect_uri: &str,
        requested_scopes: &[String],
        state: Option<String>,
        code_challenge: Option<&str>,
    ) -> Result<ConsentPageData, OAuthServiceError> {
        // Find and validate client
        let client = self.require_active_client(client_id).await?;

        // PKCE is required for the authorization-code flow for ALL clients
        // (public and confidential) per OAuth 2.1 §4.1.1 / RFC 7636. A
        // confidential client secret is not a substitute for PKCE: PKCE
        // protects the authorization-code in transit against interception,
        // which a client secret presented later at the token endpoint does
        // not. Reject any authorize request without a code_challenge.
        if code_challenge.is_none() {
            return Err(OAuthServiceError::InvalidRequest(
                "PKCE code_challenge is required for the authorization-code flow".to_string(),
            ));
        }

        // Validate redirect URI
        if !client.is_redirect_uri_allowed(redirect_uri) {
            return Err(OAuthServiceError::InvalidRedirectUri);
        }

        // Validate scopes (default to "profile" if none requested)
        let scopes = if requested_scopes.is_empty() {
            vec!["profile".to_string()]
        } else {
            for scope in requested_scopes {
                if !client.is_scope_allowed(scope) {
                    return Err(OAuthServiceError::InvalidScope(scope.clone()));
                }
            }
            requested_scopes.to_vec()
        };

        // Build scope display info
        let scope_displays = self.build_scope_displays(&scopes);

        Ok(ConsentPageData {
            client_id: client.client_id,
            client_name: client.name,
            client_description: client.description,
            scopes: scope_displays,
            redirect_uri: redirect_uri.to_string(),
            state,
        })
    }

    /// Create authorization code after user consent.
    pub async fn create_authorization_code(
        &self,
        user_id: Uuid,
        client_id: &str,
        redirect_uri: &str,
        scopes: &[String],
        code_challenge: Option<String>,
        code_challenge_method: Option<String>,
    ) -> Result<String, OAuthServiceError> {
        // Re-validate the requested scopes against the client's registered grant
        // before issuance. The consent POST path passes scopes straight from the
        // submitted form, so without this gate a caller could request scopes the
        // client was never authorized for (privilege escalation). Validating here
        // covers every caller of `create_authorization_code`, not just the GET path.
        let client = self.require_active_client(client_id).await?;

        for scope in scopes {
            if !client.is_scope_allowed(scope) {
                return Err(OAuthServiceError::InvalidScope(scope.clone()));
            }
        }

        // Generate authorization code
        let code = self.generate_secure_token();
        let code_hash = self.hash_token(&code);

        let expires_at = Utc::now() + Duration::seconds(self.config.code_expires_secs);

        let data = CreateAuthorizationCode {
            user_id,
            client_id: client_id.to_string(),
            code_hash,
            scopes: scopes.to_vec(),
            redirect_uri: redirect_uri.to_string(),
            code_challenge,
            code_challenge_method,
            expires_at,
        };

        self.repo.create_authorization_code(data).await?;

        // Create or update user grant
        self.repo
            .upsert_user_grant(CreateUserOAuthGrant {
                user_id,
                client_id: client_id.to_string(),
                scopes: scopes.to_vec(),
            })
            .await?;

        Ok(code)
    }

    /// Exchange authorization code for tokens.
    pub async fn exchange_code_for_tokens(
        &self,
        request: &TokenRequest,
    ) -> Result<TokenResponse, OAuthServiceError> {
        let code = request
            .code
            .as_ref()
            .ok_or_else(|| OAuthServiceError::InvalidGrant)?;
        let redirect_uri = request
            .redirect_uri
            .as_ref()
            .ok_or_else(|| OAuthServiceError::InvalidGrant)?;

        // Atomically find and consume authorization code (prevents race condition)
        let code_hash = self.hash_token(code);
        let auth_code = self
            .repo
            .find_and_consume_authorization_code(&code_hash)
            .await?
            .ok_or_else(|| OAuthServiceError::InvalidGrant)?;

        // Validate redirect URI matches.
        // RFC 6749 §5.2: redirect_uri mismatch at the token endpoint is invalid_grant,
        // not invalid_request (invalid_request applies at the authorize stage only).
        if auth_code.redirect_uri != *redirect_uri {
            return Err(OAuthServiceError::InvalidGrant);
        }

        // Validate PKCE. PKCE is mandatory for every authorization-code flow
        // (enforced at the authorize stage in `validate_authorize_request`), so
        // a stored code with no challenge must never be exchangeable — treat its
        // absence as invalid_grant (defense in depth against codes minted before
        // enforcement or via a bypassed authorize path). A present challenge
        // requires a matching code_verifier.
        let challenge = auth_code
            .code_challenge
            .as_deref()
            .ok_or(OAuthServiceError::InvalidGrant)?;
        let verifier = request
            .code_verifier
            .as_ref()
            .ok_or(OAuthServiceError::InvalidCodeVerifier)?;

        if !Self::verify_pkce(
            verifier,
            challenge,
            auth_code.code_challenge_method.as_deref(),
        ) {
            return Err(OAuthServiceError::InvalidCodeVerifier);
        }

        // Get client for rotation settings and principal_kind enforcement (Phase 6 C17)
        let client = self.require_active_client(&auth_code.client_id).await?;

        // Phase 6 C17: enforce principal_kind at token issuance.
        // Look up the user's kind and verify the client allows it.
        self.check_principal_kind_allowed(auth_code.user_id, &auth_code.client_id, &client)
            .await?;

        // Generate tokens. Public clients receive no refresh token in the
        // response (per spec) and we now skip persisting one as well — a
        // discarded refresh row in DB was an unreachable replay risk.
        let (access_token, refresh_token) = self
            .issue_tokens(
                auth_code.user_id,
                &auth_code.client_id,
                &auth_code.scopes.0,
                None, // New token family
                client.is_confidential,
            )
            .await?;

        // Best-effort token-usage analytics (Epic 10A, #2628): record the
        // issuance from the already-resolved grant off the hot path (#2643).
        // The write runs in a detached task; dropping the handle is
        // fire-and-forget and never fails the exchange.
        drop(self.record_token_event(CreateOAuthTokenEvent::issued(
            auth_code.client_id.clone(),
            Some(auth_code.user_id),
            auth_code.scopes.0.clone(),
        )));

        Ok(TokenResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: self.config.access_token_expires_secs,
            refresh_token,
            scope: auth_code.scopes.0.join(" "),
        })
    }

    /// Refresh access token.
    pub async fn refresh_tokens(
        &self,
        refresh_token_str: &str,
        client_id: &str,
    ) -> Result<TokenResponse, OAuthServiceError> {
        // Find refresh token — include revoked rows so the family-reuse
        // detection branch below is reachable when a previously-revoked
        // token is replayed. Production grant decisions still go through
        // is_revoked() / is_expired() guards a few lines down.
        let token_hash = self.hash_token(refresh_token_str);
        let refresh_token = self
            .repo
            .find_refresh_token_by_hash_including_revoked(&token_hash)
            .await?
            .ok_or_else(|| OAuthServiceError::InvalidGrant)?;

        // Validate token belongs to client
        if refresh_token.client_id != client_id {
            return Err(OAuthServiceError::InvalidClient(
                "Token doesn't belong to client".to_string(),
            ));
        }

        // Check if token was already revoked (reuse detection)
        if refresh_token.is_revoked() {
            // Security breach! Revoke entire token family
            self.repo
                .revoke_token_family(refresh_token.family_id)
                .await?;
            return Err(OAuthServiceError::TokenReuseDetected);
        }

        // Check expiration
        if refresh_token.is_expired() {
            return Err(OAuthServiceError::TokenExpired);
        }

        // Get client for rotation settings
        let client = self.require_active_client(client_id).await?;

        // Phase 6 C17 + review R6: re-check principal_kind on refresh so a
        // user whose kind is no longer permitted by this client (e.g. after
        // a config change) cannot mint new tokens via an existing refresh
        // grant. Mirrors the check in exchange_code_for_tokens.
        self.check_principal_kind_allowed(refresh_token.user_id, client_id, &client)
            .await?;

        // Revoke old refresh token
        self.repo.revoke_refresh_token(refresh_token.id).await?;

        // Issue new tokens (with same family for rotation detection)
        let family_id = if client.rotate_refresh_tokens {
            Some(refresh_token.family_id)
        } else {
            None
        };

        // A refresh-token flow only reaches this point for confidential
        // clients (public clients never received one to refresh with), so
        // request a refresh token on the new exchange as well.
        let (access_token, new_refresh_token) = self
            .issue_tokens(
                refresh_token.user_id,
                client_id,
                &refresh_token.scopes.0,
                family_id,
                client.is_confidential,
            )
            .await?;

        // Best-effort token-usage analytics (Epic 10A, #2628): record the
        // refresh from the already-resolved grant off the hot path (#2643).
        // Detached write; dropping the handle is fire-and-forget and never
        // fails the refresh.
        drop(self.record_token_event(CreateOAuthTokenEvent::refreshed(
            client_id,
            Some(refresh_token.user_id),
            refresh_token.scopes.0.clone(),
        )));

        Ok(TokenResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: self.config.access_token_expires_secs,
            refresh_token: new_refresh_token,
            scope: refresh_token.scopes.0.join(" "),
        })
    }

    // ==================== Token Operations ====================

    /// Validate and introspect an access token.
    /// Introspect a token on behalf of an authenticated client.
    ///
    /// `authenticated_client_id` is the client that authenticated to the
    /// introspection endpoint. RFC 7662 §2.2 lets the server tailor the
    /// response to the requester; we bind the metadata to the calling client so
    /// one client cannot enumerate another client's token contents (scopes,
    /// subject, expiry). A token that belongs to a different client is reported
    /// as `inactive` — indistinguishable from an unknown/expired token, which
    /// also avoids leaking token existence across client boundaries.
    pub async fn introspect_token(
        &self,
        token: &str,
        authenticated_client_id: &str,
    ) -> Result<IntrospectionResponse, OAuthServiceError> {
        let token_hash = self.hash_token(token);

        // Try access token first
        if let Some(access_token) = self.repo.find_access_token_by_hash(&token_hash).await? {
            if !access_token.is_valid() || access_token.client_id != authenticated_client_id {
                return Ok(IntrospectionResponse::inactive());
            }

            return Ok(IntrospectionResponse {
                active: true,
                scope: Some(access_token.scopes.0.join(" ")),
                client_id: Some(access_token.client_id),
                username: None, // Would need user lookup
                token_type: Some("access_token".to_string()),
                exp: Some(access_token.expires_at.timestamp()),
                iat: Some(access_token.created_at.timestamp()),
                sub: Some(access_token.user_id.to_string()),
            });
        }

        // Try refresh token
        if let Some(refresh_token) = self.repo.find_refresh_token_by_hash(&token_hash).await? {
            if !refresh_token.is_valid() || refresh_token.client_id != authenticated_client_id {
                return Ok(IntrospectionResponse::inactive());
            }

            return Ok(IntrospectionResponse {
                active: true,
                scope: Some(refresh_token.scopes.0.join(" ")),
                client_id: Some(refresh_token.client_id),
                username: None,
                token_type: Some("refresh_token".to_string()),
                exp: Some(refresh_token.expires_at.timestamp()),
                iat: Some(refresh_token.created_at.timestamp()),
                sub: Some(refresh_token.user_id.to_string()),
            });
        }

        Ok(IntrospectionResponse::inactive())
    }

    /// Revoke a token on behalf of an authenticated client.
    ///
    /// `authenticated_client_id` is the client that authenticated to the
    /// revocation endpoint. RFC 7009 §2.1 requires that the server "first
    /// validate the client credentials ... and then verify whether the token
    /// was issued to the client making the revocation request." A token that
    /// belongs to a different client is left untouched — this closes the
    /// unauthenticated cross-client revocation / DoS hole where any party that
    /// learned a token could revoke it. Returning `Ok(())` regardless keeps the
    /// RFC-mandated behaviour of not disclosing whether the token existed.
    pub async fn revoke_token(
        &self,
        token: &str,
        _token_type_hint: Option<&str>,
        authenticated_client_id: &str,
    ) -> Result<(), OAuthServiceError> {
        let token_hash = self.hash_token(token);

        // Try to revoke as access token — only if it belongs to this client.
        if let Some(access_token) = self.repo.find_access_token_by_hash(&token_hash).await? {
            if access_token.client_id == authenticated_client_id {
                self.repo.revoke_access_token_by_hash(&token_hash).await?;
                // Best-effort token-usage analytics (Epic 10A, #2628): record
                // the revocation from the resolved token row off the hot path
                // (#2643). Detached write; dropping the handle is
                // fire-and-forget and never fails the revoke (RFC 7009 must
                // still return 200).
                drop(self.record_token_event(CreateOAuthTokenEvent::revoked(
                    access_token.client_id.clone(),
                    Some(access_token.user_id),
                    OAuthTokenKind::Access,
                )));
            }
            return Ok(());
        }

        // Try to revoke as refresh token — only if it belongs to this client.
        if let Some(refresh_token) = self.repo.find_refresh_token_by_hash(&token_hash).await? {
            if refresh_token.client_id == authenticated_client_id {
                self.repo.revoke_refresh_token_by_hash(&token_hash).await?;
                // Best-effort token-usage analytics (Epic 10A, #2628): detached
                // write off the hot path (#2643); fire-and-forget.
                drop(self.record_token_event(CreateOAuthTokenEvent::revoked(
                    refresh_token.client_id.clone(),
                    Some(refresh_token.user_id),
                    OAuthTokenKind::Refresh,
                )));
            }
            return Ok(());
        }

        // Token not found is still a success per RFC 7009
        Ok(())
    }

    // ==================== User Grants ====================

    /// List user's authorized applications.
    pub async fn list_user_grants(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<UserGrantWithClient>, OAuthServiceError> {
        let grants = self.repo.list_user_grants(user_id).await?;
        Ok(grants.into_iter().map(UserGrantWithClient::from).collect())
    }

    /// Revoke user's authorization for a client.
    pub async fn revoke_user_grant(
        &self,
        user_id: Uuid,
        client_id: &str,
    ) -> Result<bool, OAuthServiceError> {
        Ok(self.repo.revoke_user_grant(user_id, client_id).await?)
    }

    // ==================== Cleanup ====================

    /// Cleanup expired OAuth data.
    pub async fn cleanup_expired(&self) -> Result<u64, OAuthServiceError> {
        Ok(self.repo.cleanup_expired().await?)
    }

    // ==================== Private Helpers ====================

    /// Issue access (and optionally refresh) tokens.
    ///
    /// When `with_refresh` is `false` no refresh token is generated, hashed, or
    /// persisted — this is the path public OAuth clients take, since the
    /// response never returns a refresh token to them anyway. Previously a
    /// refresh row was written for public clients and then discarded by the
    /// caller, leaving an orphan token in DB that could be replayed if the DB
    /// leaked.
    async fn issue_tokens(
        &self,
        user_id: Uuid,
        client_id: &str,
        scopes: &[String],
        family_id: Option<Uuid>,
        with_refresh: bool,
    ) -> Result<(String, Option<String>), OAuthServiceError> {
        let access_token = self.generate_secure_token();
        let access_token_hash = self.hash_token(&access_token);
        let access_expires = Utc::now() + Duration::seconds(self.config.access_token_expires_secs);

        // Create access token
        self.repo
            .create_access_token(CreateAccessToken {
                user_id,
                client_id: client_id.to_string(),
                token_hash: access_token_hash,
                scopes: scopes.to_vec(),
                expires_at: access_expires,
            })
            .await?;

        if !with_refresh {
            return Ok((access_token, None));
        }

        let refresh_token = self.generate_secure_token();
        let refresh_token_hash = self.hash_token(&refresh_token);
        let refresh_expires =
            Utc::now() + Duration::seconds(self.config.refresh_token_expires_secs);

        // Create refresh token
        self.repo
            .create_refresh_token(CreateRefreshToken {
                user_id,
                client_id: client_id.to_string(),
                token_hash: refresh_token_hash,
                scopes: scopes.to_vec(),
                family_id: family_id.unwrap_or_else(Uuid::new_v4),
                expires_at: refresh_expires,
            })
            .await?;

        Ok((access_token, Some(refresh_token)))
    }

    /// Fetch an active client by `client_id`, returning `InvalidClient` if not found.
    ///
    /// Centralises the repeated `find_active_client_by_client_id + ok_or_else`
    /// pattern so future changes to the error message or the lookup itself only
    /// need to be made in one place.
    async fn require_active_client(
        &self,
        client_id: &str,
    ) -> Result<OAuthClient, OAuthServiceError> {
        self.repo
            .find_active_client_by_client_id(client_id)
            .await?
            .ok_or_else(|| OAuthServiceError::InvalidClient("Client not found".to_string()))
    }

    /// Validate that every scope string in `scopes` is a recognised `OAuthScope`.
    ///
    /// Returns `InvalidScope` on the first unrecognised entry.
    fn validate_scopes(&self, scopes: &[String]) -> Result<(), OAuthServiceError> {
        for scope in scopes {
            if OAuthScope::parse(scope).is_none() {
                return Err(OAuthServiceError::InvalidScope(scope.clone()));
            }
        }
        Ok(())
    }

    /// Build `ScopeDisplay` metadata from a list of scope strings.
    fn build_scope_displays(&self, scopes: &[String]) -> Vec<ScopeDisplay> {
        scopes
            .iter()
            .filter_map(|s| OAuthScope::parse(s))
            .map(|s| ScopeDisplay {
                name: s.as_str().to_string(),
                description: s.description().to_string(),
            })
            .collect()
    }

    /// Enforce the Phase 6 C17 `principal_kind` policy for a given user + client pair.
    ///
    /// Shared between `exchange_code_for_tokens` and `refresh_tokens` so that
    /// the policy is applied consistently: if the user's `principal_kind` is not
    /// in the client's allowed list the grant is rejected and a warning is logged.
    async fn check_principal_kind_allowed(
        &self,
        user_id: Uuid,
        client_id: &str,
        client: &OAuthClient,
    ) -> Result<(), OAuthServiceError> {
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| OAuthServiceError::InvalidGrant)?;

        if !client.is_principal_kind_allowed(&user.principal_kind) {
            tracing::warn!(
                user_id = %user_id,
                principal_kind = %user.principal_kind,
                client_id = %client_id,
                "Token grant denied: principal_kind not allowed for OAuth client"
            );
            return Err(OAuthServiceError::PrincipalKindNotAllowed);
        }

        Ok(())
    }

    /// Generate `n` random bytes from the OS CSPRNG and return them base64url-encoded
    /// (no padding).
    ///
    /// Callers:
    /// - `generate_secure_token()` — 32 bytes → tokens / auth codes
    /// - `register_client` / `regenerate_client_secret` — 32 bytes → client secret
    /// - `register_client` — 16 bytes → client id
    ///
    /// Uses `SysRng` directly rather than `thread_rng` so entropy does not
    /// depend on the ChaCha20 thread-local state being seeded first.
    fn generate_random_b64(&self, n: usize) -> String {
        let mut bytes = vec![0u8; n];
        rand::rngs::SysRng
            .try_fill_bytes(&mut bytes)
            .expect("OS rng failed");
        URL_SAFE_NO_PAD.encode(&bytes)
    }

    /// Generate a 32-byte secure token (base64url encoded).
    ///
    /// Used for OAuth authorization codes, access tokens, and refresh tokens —
    /// all shared secrets that require CSPRNG-quality entropy.
    fn generate_secure_token(&self) -> String {
        self.generate_random_b64(32)
    }

    /// Hash a token using SHA-256.
    fn hash_token(&self, token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Verify PKCE code challenge.
    /// Only S256 method is supported per OAuth 2.1 recommendations.
    fn verify_pkce(verifier: &str, challenge: &str, method: Option<&str>) -> bool {
        // Only S256 is supported - plain method is deprecated per OAuth 2.1
        match method.unwrap_or("S256") {
            "S256" => {
                let mut hasher = Sha256::new();
                hasher.update(verifier.as_bytes());
                let computed = URL_SAFE_NO_PAD.encode(hasher.finalize());
                computed == challenge
            }
            // "plain" is intentionally not supported as it defeats the purpose of PKCE
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generate `n` random base64url-encoded bytes via the OS CSPRNG.
    fn random_b64(n: usize) -> String {
        let mut bytes = vec![0u8; n];
        rand::TryRng::try_fill_bytes(&mut rand::rngs::SysRng, &mut bytes).expect("OS rng failed");
        URL_SAFE_NO_PAD.encode(&bytes)
    }

    #[test]
    fn test_pkce_verification() {
        // RFC 7636 Appendix B known-answer vector: this pins the S256 challenge
        // computation so a regression in `verify_pkce` is actually caught.
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

        // Correct verifier + challenge under S256 (and default method == S256).
        assert!(OAuthService::verify_pkce(verifier, challenge, Some("S256")));
        assert!(OAuthService::verify_pkce(verifier, challenge, None));

        // Wrong verifier must not match the challenge.
        assert!(!OAuthService::verify_pkce("wrong", challenge, Some("S256")));

        // `plain` is intentionally unsupported, even when verifier == challenge.
        assert!(!OAuthService::verify_pkce(
            challenge,
            challenge,
            Some("plain")
        ));
    }

    #[test]
    fn test_token_generation() {
        let token1 = random_b64(32);
        let token2 = random_b64(32);

        // Should be 43 chars (32 bytes base64url encoded without padding)
        assert_eq!(token1.len(), 43);
        assert_ne!(token1, token2);
    }

    #[test]
    fn test_client_id_generation() {
        let id1 = random_b64(16);
        let id2 = random_b64(16);

        // Should be 22 chars (16 bytes base64url encoded without padding)
        assert_eq!(id1.len(), 22);
        assert_ne!(id1, id2);
    }

    // ─── #2643: token-usage recording is off the hot token path ──────────────

    /// With no analytics recorder wired, `record_token_event` short-circuits to
    /// `None` — it neither spawns a task nor touches the DB, so the token path
    /// is completely untouched. Runs without a database (the lazy pool is never
    /// connected because the `None` branch returns before any query), pinning
    /// the "analytics stays optional" contract. It still runs under a Tokio
    /// runtime because `PgPool::connect_lazy` needs a reactor to build the pool,
    /// mirroring the sibling `record_token_event_runs_off_the_hot_path` test.
    #[tokio::test]
    async fn record_token_event_without_recorder_returns_none() {
        // `connect_lazy` never opens a connection until first use; the `None`
        // branch below returns before any query, so no DB is required.
        let pool = sqlx::PgPool::connect_lazy("postgres://ppt:ppt@127.0.0.1/ppt")
            .expect("build lazy pool");
        let service = OAuthService::new(
            OAuthRepository::new(pool.clone()),
            UserRepository::new(pool.clone()),
            AuthService::new(),
        );
        assert!(
            service
                .record_token_event(CreateOAuthTokenEvent::issued(
                    "some-client",
                    None,
                    vec!["profile".to_string()],
                ))
                .is_none(),
            "no recorder wired ⇒ no task spawned, nothing recorded"
        );
    }

    /// #2643 — the token-usage recording is no longer awaited inline on the hot
    /// token path. `record_token_event` dispatches the analytics `INSERT` to a
    /// detached [`tokio::spawn`] task and returns its `JoinHandle`
    /// *synchronously*, so the token-granting caller regains control (and can
    /// return its `TokenResponse`) before the write has run.
    ///
    /// The test pins both halves of the contract:
    ///   1. **Off the hot path** — immediately after the (non-`async`) call the
    ///      spawned task has not completed (`!is_finished()`), proving the write
    ///      was not awaited inline. On the current-thread test runtime the
    ///      spawned task cannot have been polled yet (no `.await` happened since
    ///      the spawn), so the observation is deterministic.
    ///   2. **No dropped write** — driving the returned handle to completion
    ///      persists exactly one `oauth_token_events` row, so the recording is
    ///      preserved on the happy path.
    ///
    /// Against the pre-#2643 inline `.await` the caller blocked on the DB
    /// round-trip and there was no handle to observe (1) with at all.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn record_token_event_runs_off_the_hot_path(pool: sqlx::PgPool) {
        // oauth_token_events.client_id FKs to oauth_clients — seed the client.
        let client_id = format!("hotpath-{}", &Uuid::new_v4().to_string()[..8]);
        sqlx::query(
            r#"
            INSERT INTO oauth_clients
                (client_id, client_secret_hash, name, redirect_uris, scopes,
                 is_confidential, rotate_refresh_tokens)
            VALUES ($1, 'unused-hash', 'Hot Path Test',
                    '["https://app.example.com/cb"]'::jsonb,
                    '["profile"]'::jsonb, false, false)
            "#,
        )
        .bind(&client_id)
        .execute(&pool)
        .await
        .expect("seed oauth client");

        let service = OAuthService::new(
            OAuthRepository::new(pool.clone()),
            UserRepository::new(pool.clone()),
            AuthService::new(),
        )
        .with_token_event_repo(OAuthTokenEventRepository::new(pool.clone()));

        // Synchronous call (no `.await`): spawns the write and hands back the
        // task handle without touching the DB on the caller's thread.
        let handle = service
            .record_token_event(CreateOAuthTokenEvent::issued(
                client_id.clone(),
                None,
                vec!["profile".to_string()],
            ))
            .expect("recorder is wired ⇒ Some(handle)");

        // (1) Off the hot path: the analytics INSERT has NOT completed at the
        // point the caller regains control.
        assert!(
            !handle.is_finished(),
            "token-usage write must not be awaited inline on the hot path (#2643)"
        );

        // (2) No dropped write: driving the detached task to completion persists
        // the event exactly once.
        handle.await.expect("recording task must not panic");

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM oauth_token_events \
             WHERE client_id = $1 AND event_type = 'issued'",
        )
        .bind(&client_id)
        .fetch_one(&pool)
        .await
        .expect("count token events");
        assert_eq!(
            count, 1,
            "the detached task must persist exactly one issued event"
        );
    }
}
