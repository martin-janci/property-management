//! OAuth 2.0 Provider models (Epic 10A).
//!
//! This module contains models for OAuth 2.0 Authorization Server implementation
//! including clients, authorization codes, access tokens, and refresh tokens.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Available OAuth scopes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthScope {
    /// Access to user's basic profile (name, avatar)
    Profile,
    /// Access to user's email address
    Email,
    /// Read-only access to organization data
    OrgRead,
    /// Full access to user's data and actions
    Full,
}

impl OAuthScope {
    /// Get all available scopes.
    pub fn all() -> Vec<Self> {
        vec![Self::Profile, Self::Email, Self::OrgRead, Self::Full]
    }

    /// Parse scope from string.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "profile" => Some(Self::Profile),
            "email" => Some(Self::Email),
            "org:read" => Some(Self::OrgRead),
            "full" => Some(Self::Full),
            _ => None,
        }
    }

    /// Convert scope to string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Profile => "profile",
            Self::Email => "email",
            Self::OrgRead => "org:read",
            Self::Full => "full",
        }
    }

    /// Get human-readable description for consent page.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Profile => "Access your basic profile information (name, avatar)",
            Self::Email => "Access your email address",
            Self::OrgRead => "Read-only access to your organization data",
            Self::Full => "Full access to your account and data",
        }
    }
}

/// OAuth Client entity from database.
#[derive(Debug, Clone, FromRow)]
pub struct OAuthClient {
    pub id: Uuid,
    pub client_id: String,
    pub client_secret_hash: String,
    pub name: String,
    pub description: Option<String>,
    pub redirect_uris: sqlx::types::Json<Vec<String>>,
    pub scopes: sqlx::types::Json<Vec<String>>,
    pub is_confidential: bool,
    pub rotate_refresh_tokens: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Phase 6 C17: which principal_kind values may authenticate via this client.
    /// Defaults to all kinds for backward compatibility.
    #[sqlx(default)]
    pub allowed_principal_kinds: Vec<String>,
}

impl OAuthClient {
    /// Phase 6 C17: check if the given principal_kind is allowed to obtain
    /// tokens via this client. An empty allowed list (legacy rows before
    /// migration 00147 back-fills the default) is treated as "allow all".
    pub fn is_principal_kind_allowed(&self, kind: &str) -> bool {
        if self.allowed_principal_kinds.is_empty() {
            return true;
        }
        self.allowed_principal_kinds.iter().any(|k| k == kind)
    }

    /// Check if a redirect URI is allowed for this client.
    pub fn is_redirect_uri_allowed(&self, uri: &str) -> bool {
        self.redirect_uris.iter().any(|allowed| allowed == uri)
    }

    /// Check if a scope is allowed for this client.
    pub fn is_scope_allowed(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope)
    }

    /// Validate that all requested scopes are allowed.
    pub fn validate_scopes(&self, requested: &[String]) -> bool {
        requested.iter().all(|s| self.is_scope_allowed(s))
    }
}

/// Data for creating a new OAuth client.
#[derive(Debug, Clone)]
pub struct CreateOAuthClient {
    pub client_id: String,
    pub client_secret_hash: String,
    pub name: String,
    pub description: Option<String>,
    pub redirect_uris: Vec<String>,
    pub scopes: Vec<String>,
    pub is_confidential: bool,
    pub rotate_refresh_tokens: bool,
}

/// Data for updating an OAuth client.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateOAuthClient {
    pub name: Option<String>,
    pub description: Option<String>,
    pub redirect_uris: Option<Vec<String>>,
    pub scopes: Option<Vec<String>>,
    pub is_active: Option<bool>,
    pub rotate_refresh_tokens: Option<bool>,
    /// Operator-supplied justification for the change. The admin-web UI
    /// requires this when scopes are edited (privileged mutation) and
    /// forwards it as `reason`; it is persisted to the audit log only and
    /// is **not** written to the `oauth_clients` row.
    #[serde(default)]
    pub reason: Option<String>,
}

/// OAuth client summary for listing.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OAuthClientSummary {
    pub id: Uuid,
    pub client_id: String,
    pub name: String,
    pub description: Option<String>,
    pub scopes: Vec<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl From<OAuthClient> for OAuthClientSummary {
    fn from(client: OAuthClient) -> Self {
        Self {
            id: client.id,
            client_id: client.client_id,
            name: client.name,
            description: client.description,
            scopes: client.scopes.0,
            is_active: client.is_active,
            created_at: client.created_at,
        }
    }
}

/// OAuth Authorization Code entity from database.
#[derive(Debug, Clone, FromRow)]
pub struct OAuthAuthorizationCode {
    pub id: Uuid,
    pub user_id: Uuid,
    pub client_id: String,
    pub code_hash: String,
    pub scopes: sqlx::types::Json<Vec<String>>,
    pub redirect_uri: String,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl OAuthAuthorizationCode {
    /// Check if the authorization code is expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at < Utc::now()
    }

    /// Check if the authorization code has been used.
    pub fn is_used(&self) -> bool {
        self.used_at.is_some()
    }

    /// Check if the authorization code is valid (not expired and not used).
    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.is_used()
    }
}

/// Data for creating a new authorization code.
#[derive(Debug, Clone)]
pub struct CreateAuthorizationCode {
    pub user_id: Uuid,
    pub client_id: String,
    pub code_hash: String,
    pub scopes: Vec<String>,
    pub redirect_uri: String,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub expires_at: DateTime<Utc>,
}

/// OAuth Access Token entity from database.
#[derive(Debug, Clone, FromRow)]
pub struct OAuthAccessToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub client_id: String,
    pub token_hash: String,
    pub scopes: sqlx::types::Json<Vec<String>>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl OAuthAccessToken {
    /// Check if the access token is expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at < Utc::now()
    }

    /// Check if the access token has been revoked.
    pub fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }

    /// Check if the access token is valid.
    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.is_revoked()
    }
}

/// Data for creating a new access token.
#[derive(Debug, Clone)]
pub struct CreateAccessToken {
    pub user_id: Uuid,
    pub client_id: String,
    pub token_hash: String,
    pub scopes: Vec<String>,
    pub expires_at: DateTime<Utc>,
}

/// OAuth Refresh Token entity from database.
#[derive(Debug, Clone, FromRow)]
pub struct OAuthRefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub client_id: String,
    pub token_hash: String,
    pub scopes: sqlx::types::Json<Vec<String>>,
    pub family_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl OAuthRefreshToken {
    /// Check if the refresh token is expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at < Utc::now()
    }

    /// Check if the refresh token has been revoked.
    pub fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }

    /// Check if the refresh token is valid.
    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.is_revoked()
    }
}

/// Data for creating a new refresh token.
#[derive(Debug, Clone)]
pub struct CreateRefreshToken {
    pub user_id: Uuid,
    pub client_id: String,
    pub token_hash: String,
    pub scopes: Vec<String>,
    pub family_id: Uuid,
    pub expires_at: DateTime<Utc>,
}

/// User OAuth Grant entity from database.
#[derive(Debug, Clone, FromRow)]
pub struct UserOAuthGrant {
    pub id: Uuid,
    pub user_id: Uuid,
    pub client_id: String,
    pub scopes: sqlx::types::Json<Vec<String>>,
    pub granted_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl UserOAuthGrant {
    /// Check if the grant is active.
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none()
    }
}

/// Data for creating a new user OAuth grant.
#[derive(Debug, Clone)]
pub struct CreateUserOAuthGrant {
    pub user_id: Uuid,
    pub client_id: String,
    pub scopes: Vec<String>,
}

/// User grant with client info for display.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserGrantWithClient {
    pub id: Uuid,
    pub client_id: String,
    pub client_name: String,
    pub client_description: Option<String>,
    pub scopes: Vec<String>,
    pub granted_at: DateTime<Utc>,
}

/// Row type for user grant with client info query.
#[derive(Debug, Clone, FromRow)]
pub struct UserGrantWithClientRow {
    pub id: Uuid,
    pub client_id: String,
    pub client_name: String,
    pub client_description: Option<String>,
    pub scopes: sqlx::types::Json<Vec<String>>,
    pub granted_at: DateTime<Utc>,
}

impl From<UserGrantWithClientRow> for UserGrantWithClient {
    fn from(row: UserGrantWithClientRow) -> Self {
        Self {
            id: row.id,
            client_id: row.client_id,
            client_name: row.client_name,
            client_description: row.client_description,
            scopes: row.scopes.0,
            granted_at: row.granted_at,
        }
    }
}

// ===================== Request/Response DTOs =====================

/// Authorization request parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AuthorizeRequest {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

impl AuthorizeRequest {
    /// Parse scopes from space-separated string.
    pub fn scopes(&self) -> Vec<String> {
        self.scope
            .as_ref()
            .map(|s| s.split_whitespace().map(String::from).collect())
            .unwrap_or_else(|| vec!["profile".to_string()])
    }
}

/// Token request parameters.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct TokenRequest {
    pub grant_type: String,
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
    pub code_verifier: Option<String>,
    pub refresh_token: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

/// Token response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    pub scope: String,
}

/// Token introspection response (RFC 7662).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct IntrospectionResponse {
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iat: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,
}

impl IntrospectionResponse {
    /// Create an inactive token response.
    pub fn inactive() -> Self {
        Self {
            active: false,
            scope: None,
            client_id: None,
            username: None,
            token_type: None,
            exp: None,
            iat: None,
            sub: None,
        }
    }
}

/// Consent page data for display.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConsentPageData {
    pub client_id: String,
    pub client_name: String,
    pub client_description: Option<String>,
    pub scopes: Vec<ScopeDisplay>,
    pub redirect_uri: String,
    pub state: Option<String>,
}

/// Scope information for consent display.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScopeDisplay {
    pub name: String,
    pub description: String,
}

/// Client registration request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisterClientRequest {
    pub name: String,
    pub description: Option<String>,
    pub redirect_uris: Vec<String>,
    pub scopes: Vec<String>,
    pub is_confidential: Option<bool>,
    pub rotate_refresh_tokens: Option<bool>,
}

/// Client registration response (secret shown only once).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisterClientResponse {
    pub id: Uuid,
    pub client_id: String,
    pub client_secret: String, // Plaintext, shown only once
    pub name: String,
    pub redirect_uris: Vec<String>,
    pub scopes: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Revoke token request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct RevokeTokenRequest {
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type_hint: Option<String>,
    /// Client ID for authentication (RFC 7009 §2.1; alternative to Basic auth).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    /// Client secret for authentication (alternative to Basic auth).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
}

/// OAuth error response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct OAuthError {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_uri: Option<String>,
}

impl OAuthError {
    /// Invalid request error.
    pub fn invalid_request(description: &str) -> Self {
        Self {
            error: "invalid_request".to_string(),
            error_description: Some(description.to_string()),
            error_uri: None,
        }
    }

    /// Unauthorized client error.
    pub fn unauthorized_client(description: &str) -> Self {
        Self {
            error: "unauthorized_client".to_string(),
            error_description: Some(description.to_string()),
            error_uri: None,
        }
    }

    /// Access denied error.
    pub fn access_denied(description: &str) -> Self {
        Self {
            error: "access_denied".to_string(),
            error_description: Some(description.to_string()),
            error_uri: None,
        }
    }

    /// Invalid grant error.
    pub fn invalid_grant(description: &str) -> Self {
        Self {
            error: "invalid_grant".to_string(),
            error_description: Some(description.to_string()),
            error_uri: None,
        }
    }

    /// Invalid client error.
    pub fn invalid_client(description: &str) -> Self {
        Self {
            error: "invalid_client".to_string(),
            error_description: Some(description.to_string()),
            error_uri: None,
        }
    }

    /// Invalid scope error.
    pub fn invalid_scope(description: &str) -> Self {
        Self {
            error: "invalid_scope".to_string(),
            error_description: Some(description.to_string()),
            error_uri: None,
        }
    }

    /// Server error.
    pub fn server_error(description: &str) -> Self {
        Self {
            error: "server_error".to_string(),
            error_description: Some(description.to_string()),
            error_uri: None,
        }
    }
}
