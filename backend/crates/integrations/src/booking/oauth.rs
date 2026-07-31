//! OAuth 2.0 (authorization-code) client for Booking.com.
//!
//! Booking.com's newer Connectivity APIs use an OAuth 2.0 authorization-code
//! grant in addition to the legacy Basic-auth Supply XML credentials handled by
//! [`super::BookingClient`].  This module mirrors the Airbnb OAuth client
//! (`integrations::airbnb`) so the api-server can host a parallel
//! `POST /organizations/{org_id}/booking/token/exchange` route (Coverage 83-2).
//!
//! Extracted verbatim from `booking/mod.rs` as a pure module split — no
//! behaviour change.

use super::BookingError;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Booking.com OAuth token endpoint (authorization-code + refresh grants).
pub const BOOKING_OAUTH_TOKEN_URL: &str = "https://account.booking.com/oauth2/token";

/// Booking.com OAuth authorization endpoint (front-channel redirect target).
pub const BOOKING_OAUTH_AUTH_URL: &str = "https://account.booking.com/oauth2/authorize";

/// OAuth configuration for Booking.com.
#[derive(Debug, Clone)]
pub struct BookingOAuthConfig {
    /// Booking.com OAuth client ID.
    pub client_id: String,
    /// Booking.com OAuth client secret.
    pub client_secret: String,
    /// OAuth redirect URI for the callback.
    pub redirect_uri: String,
}

/// OAuth tokens received from Booking.com.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookingOAuthTokens {
    /// Access token for API calls.
    pub access_token: String,
    /// Refresh token for obtaining new access tokens.
    pub refresh_token: Option<String>,
    /// When the access token expires.
    pub expires_at: Option<DateTime<Utc>>,
    /// Token type (usually "Bearer").
    pub token_type: String,
    /// OAuth scopes granted.
    pub scope: Option<String>,
}

/// Booking.com OAuth client (authorization-code grant).
///
/// Distinct from [`super::BookingClient`], which speaks the legacy Basic-auth Supply
/// XML protocol.  This client only handles the OAuth token lifecycle.
pub struct BookingOAuthClient {
    client: reqwest::Client,
    config: BookingOAuthConfig,
}

impl BookingOAuthClient {
    /// Create a new Booking.com OAuth client.
    pub fn new(config: BookingOAuthConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    /// Create a new client from individual OAuth parameters.
    pub fn with_credentials(
        client_id: String,
        client_secret: String,
        redirect_uri: String,
    ) -> Self {
        Self::new(BookingOAuthConfig {
            client_id,
            client_secret,
            redirect_uri,
        })
    }

    /// Generate the OAuth authorization URL the user is redirected to.
    pub fn generate_auth_url(&self, state: &str) -> String {
        let scopes = "reservations:read availability:write rates:write";
        format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&state={}&scope={}",
            BOOKING_OAUTH_AUTH_URL,
            urlencoding::encode(&self.config.client_id),
            urlencoding::encode(&self.config.redirect_uri),
            urlencoding::encode(state),
            urlencoding::encode(scopes),
        )
    }

    /// Exchange an authorization code for access and refresh tokens.
    pub async fn exchange_code(&self, code: &str) -> Result<BookingOAuthTokens, BookingError> {
        #[derive(Serialize)]
        struct TokenRequest<'a> {
            grant_type: &'a str,
            client_id: &'a str,
            client_secret: &'a str,
            code: &'a str,
            redirect_uri: &'a str,
        }

        let response = self
            .client
            .post(BOOKING_OAUTH_TOKEN_URL)
            .form(&TokenRequest {
                grant_type: "authorization_code",
                client_id: &self.config.client_id,
                client_secret: &self.config.client_secret,
                code,
                redirect_uri: &self.config.redirect_uri,
            })
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(BookingError::Auth(error));
        }

        let token_response: OAuthTokenResponse = response
            .json()
            .await
            .map_err(|e| BookingError::Api(e.to_string()))?;

        Ok(token_response.into_tokens(None))
    }

    /// Refresh an expired access token using the stored refresh token.
    pub async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<BookingOAuthTokens, BookingError> {
        #[derive(Serialize)]
        struct RefreshRequest<'a> {
            grant_type: &'a str,
            client_id: &'a str,
            client_secret: &'a str,
            refresh_token: &'a str,
        }

        let response = self
            .client
            .post(BOOKING_OAUTH_TOKEN_URL)
            .form(&RefreshRequest {
                grant_type: "refresh_token",
                client_id: &self.config.client_id,
                client_secret: &self.config.client_secret,
                refresh_token,
            })
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(BookingError::Auth(error));
        }

        let token_response: OAuthTokenResponse = response
            .json()
            .await
            .map_err(|e| BookingError::Api(e.to_string()))?;

        Ok(token_response.into_tokens(Some(refresh_token)))
    }
}

/// Raw token-endpoint response shape (shared by exchange + refresh).
#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    #[serde(default = "default_token_type")]
    token_type: String,
    scope: Option<String>,
}

fn default_token_type() -> String {
    "Bearer".to_string()
}

impl OAuthTokenResponse {
    /// Convert the wire response into [`BookingOAuthTokens`], computing
    /// `expires_at` from `expires_in` and falling back to `fallback_refresh`
    /// when the endpoint did not rotate the refresh token.
    fn into_tokens(self, fallback_refresh: Option<&str>) -> BookingOAuthTokens {
        let expires_at = self
            .expires_in
            .map(|secs| Utc::now() + Duration::seconds(secs));
        BookingOAuthTokens {
            access_token: self.access_token,
            refresh_token: self
                .refresh_token
                .or_else(|| fallback_refresh.map(|s| s.to_string())),
            expires_at,
            token_type: self.token_type,
            scope: self.scope,
        }
    }
}

#[cfg(test)]
mod oauth_tests {
    use super::*;

    #[test]
    fn test_generate_auth_url() {
        let client = BookingOAuthClient::new(BookingOAuthConfig {
            client_id: "test_client_id".to_string(),
            client_secret: "test_secret".to_string(),
            redirect_uri: "http://localhost:8080/callback".to_string(),
        });
        let url = client.generate_auth_url("test_state");
        assert!(url.contains("account.booking.com"));
        assert!(url.contains("test_client_id"));
        assert!(url.contains("test_state"));
        assert!(url.contains("response_type=code"));
        assert!(!url.contains("test_secret"));
    }

    #[test]
    fn test_token_response_deserialization_full() {
        let json = r#"{"access_token":"at-123","refresh_token":"rt-456","expires_in":3600,"token_type":"Bearer","scope":"reservations:read"}"#;
        let parsed: OAuthTokenResponse = serde_json::from_str(json).unwrap();
        let tokens = parsed.into_tokens(None);
        assert_eq!(tokens.access_token, "at-123");
        assert_eq!(tokens.refresh_token.as_deref(), Some("rt-456"));
        assert_eq!(tokens.token_type, "Bearer");
        assert_eq!(tokens.scope.as_deref(), Some("reservations:read"));
        assert!(tokens.expires_at.is_some());
    }

    #[test]
    fn test_token_response_defaults_token_type() {
        let json = r#"{"access_token":"at-789","expires_in":7200}"#;
        let parsed: OAuthTokenResponse = serde_json::from_str(json).unwrap();
        let tokens = parsed.into_tokens(None);
        assert_eq!(tokens.token_type, "Bearer");
        assert!(tokens.refresh_token.is_none());
        assert!(tokens.scope.is_none());
    }

    #[test]
    fn test_refresh_falls_back_to_existing_refresh_token() {
        let json = r#"{"access_token":"at-new","expires_in":3600,"token_type":"Bearer"}"#;
        let parsed: OAuthTokenResponse = serde_json::from_str(json).unwrap();
        let tokens = parsed.into_tokens(Some("rt-original"));
        assert_eq!(tokens.access_token, "at-new");
        assert_eq!(tokens.refresh_token.as_deref(), Some("rt-original"));
    }

    #[test]
    fn test_rotated_refresh_token_wins_over_fallback() {
        let json = r#"{"access_token":"at-new","refresh_token":"rt-rotated","expires_in":3600}"#;
        let parsed: OAuthTokenResponse = serde_json::from_str(json).unwrap();
        let tokens = parsed.into_tokens(Some("rt-original"));
        assert_eq!(tokens.refresh_token.as_deref(), Some("rt-rotated"));
    }

    #[test]
    fn test_oauth_tokens_roundtrip_serde() {
        let tokens = BookingOAuthTokens {
            access_token: "at".to_string(),
            refresh_token: Some("rt".to_string()),
            expires_at: None,
            token_type: "Bearer".to_string(),
            scope: Some("rates:write".to_string()),
        };
        let json = serde_json::to_string(&tokens).unwrap();
        let back: BookingOAuthTokens = serde_json::from_str(&json).unwrap();
        assert_eq!(back.access_token, "at");
        assert_eq!(back.refresh_token.as_deref(), Some("rt"));
        assert_eq!(back.scope.as_deref(), Some("rates:write"));
    }
}
