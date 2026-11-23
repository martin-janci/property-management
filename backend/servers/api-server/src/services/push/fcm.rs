//! FCM (Firebase Cloud Messaging) HTTP v1 push transport.
//!
//! [`FcmHttpAdapter`] implements [`PushTransport`](common::notifications::PushTransport)
//! for Android: it looks up a user's device tokens, calls the FCM HTTP v1 send
//! API with an auto-refreshed service-account OAuth2 token, and evicts stale
//! (`UNREGISTERED`) tokens.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use common::notifications::{Notification, NotificationError, PushTransport, TransportResult};
use db::{
    models::{device_push_token::PushPlatform, DevicePushToken},
    repositories::DevicePushTokenRepository,
    DbPool,
};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::combined::ProviderOutcome;
use super::{
    build_push_http_client, delete_stale_token_by_string, select_dispatch_targets,
    PushDeliveryReceipt, PushTargetFilter,
};

// ============================================================================
// FCM response types (HTTP v1)
// ============================================================================

/// Top-level FCM HTTP v1 send response.
#[derive(Debug, Deserialize)]
struct FcmSendResponse {
    /// Resource name of the message if accepted (`projects/.../messages/…`).
    #[allow(dead_code)]
    name: Option<String>,
    /// Error block present when the message was rejected.
    error: Option<FcmError>,
}

/// FCM error object nested inside `FcmSendResponse`.
#[derive(Debug, Deserialize)]
struct FcmError {
    /// Canonical `google.rpc.Code` status string (e.g. `NOT_FOUND`,
    /// `INVALID_ARGUMENT`, `UNAVAILABLE`). This is the *transport-level* status
    /// — the FCM-specific error code lives in [`FcmError::details`].
    status: Option<String>,
    #[allow(dead_code)]
    message: Option<String>,
    /// Structured error details. FCM HTTP v1 reports the actionable
    /// `MessagingErrorCode` (e.g. `UNREGISTERED`) here, inside a detail whose
    /// `@type` ends in `FcmError`, rather than in [`FcmError::status`].
    #[serde(default)]
    details: Vec<FcmErrorDetail>,
}

/// One entry of the FCM HTTP v1 `error.details[]` array.
///
/// We only care about `errorCode`; the `@type` discriminator and any other
/// fields are ignored by serde (no `deny_unknown_fields`).
#[derive(Debug, Deserialize)]
struct FcmErrorDetail {
    #[serde(rename = "errorCode")]
    error_code: Option<String>,
}

impl FcmError {
    /// The FCM-specific error code (`error.details[].errorCode`) if present,
    /// otherwise the canonical transport status — used for logging.
    fn log_code(&self) -> Option<&str> {
        self.fcm_error_code().or(self.status.as_deref())
    }

    /// The first `errorCode` reported in `error.details[]`.
    fn fcm_error_code(&self) -> Option<&str> {
        self.details.iter().find_map(|d| d.error_code.as_deref())
    }

    /// Whether this error means the registration token is permanently invalid
    /// and should be evicted from the DB.
    ///
    /// Only `UNREGISTERED` (FCM v1) / legacy `NOT_REGISTERED` /
    /// `INVALID_REGISTRATION` are treated as stale. `INVALID_ARGUMENT` is
    /// deliberately NOT treated as stale: in FCM v1 it can signal a malformed
    /// *message payload*, and evicting on it would wipe every token the moment
    /// a payload bug ships. The FCM-specific `errorCode` in `error.details[]`
    /// is the primary signal; `error.status` is kept as a fall-back so legacy
    /// responses and test mocks that set the code there still evict.
    fn indicates_stale_token(&self) -> bool {
        let stale = |code: &str| {
            matches!(
                code,
                "UNREGISTERED" | "NOT_REGISTERED" | "INVALID_REGISTRATION"
            )
        };
        self.fcm_error_code().is_some_and(stale) || self.status.as_deref().is_some_and(stale)
    }
}

/// Classify an FCM HTTP v1 send result into `(delivered, token_expired)`.
///
/// `status_success` is whether the HTTP response status was 2xx. `parsed` is the
/// body deserialized into [`FcmSendResponse`], or `None` when the body did not
/// match that shape (serde parse error).
///
/// A 2xx status means FCM has already *accepted* the message for delivery, so a
/// 2xx is treated as delivered even when the body does not parse — reporting it
/// as a failed attempt makes the fanout worker requeue the job and re-deliver
/// the push (duplicate). Only a non-2xx response (or a 2xx that parses with an
/// explicit `error` block) counts as a failed attempt the caller may retry.
fn classify_fcm_outcome(status_success: bool, parsed: Option<&FcmSendResponse>) -> (bool, bool) {
    match parsed {
        Some(resp) => {
            if status_success && resp.error.is_none() {
                (true, false)
            } else {
                let expired = resp
                    .error
                    .as_ref()
                    .map(|e| e.indicates_stale_token())
                    .unwrap_or(false);
                (false, expired)
            }
        }
        // Body did not parse into our shape. A 2xx has already been accepted by
        // FCM, so retrying would re-deliver the push — treat it as delivered.
        // A non-2xx unparseable body stays a failed (retryable) attempt.
        None => (status_success, false),
    }
}

// ============================================================================
// FCM adapter configuration
// ============================================================================

/// Runtime configuration for the FCM transport.
#[derive(Clone, Debug)]
pub struct FcmConfig {
    /// GCP project ID — used to build the FCM HTTP v1 endpoint:
    /// `https://fcm.googleapis.com/v1/projects/{project_id}/messages:send`
    pub project_id: Option<String>,
    /// **Legacy / deprecated** pre-minted OAuth2 bearer token for FCM HTTP v1.
    ///
    /// Read once at startup and never refreshed, so it stops working after
    /// Google's ~1h access-token TTL — Android push then fails until the
    /// process is restarted. Prefer a service account (`FCM_SERVICE_ACCOUNT_JSON`
    /// / `GOOGLE_APPLICATION_CREDENTIALS`), which the adapter mints and refreshes
    /// automatically via [`FcmTokenProvider`]. This field is used only as a
    /// fall-back when no service account is configured.
    pub oauth_token: Option<String>,
    /// Base URL override for FCM v1 sends (e.g. a wiremock server in tests).
    ///
    /// When `None`, the production FCM base URL is used:
    /// `https://fcm.googleapis.com`.  Set this to override only in tests — the
    /// path segment (`/v1/projects/{id}/messages:send`) is appended by the
    /// adapter regardless.
    pub fcm_base_url: Option<String>,
}

impl FcmConfig {
    /// Load from environment variables.
    pub fn from_env() -> Self {
        Self {
            project_id: std::env::var("FCM_PROJECT_ID").ok(),
            oauth_token: std::env::var("FCM_OAUTH_TOKEN").ok(),
            fcm_base_url: None,
        }
    }

    /// Return `true` when FCM HTTP v1 is configured.
    ///
    /// A GCP project id is mandatory: it is the only supported FCM transport
    /// now that Google has decommissioned the legacy `/fcm/send` server-key API.
    pub fn is_configured(&self) -> bool {
        self.project_id.is_some()
    }

    /// Return the effective FCM base URL (production default when unset).
    pub fn base_url(&self) -> &str {
        self.fcm_base_url
            .as_deref()
            .unwrap_or("https://fcm.googleapis.com")
    }
}

// ============================================================================
// FCM HTTP v1 OAuth2 access-token provider (auto-refreshing)
// ============================================================================
//
// FCM HTTP v1 requires an OAuth2 access token in the `Authorization: Bearer`
// header. Google's access tokens expire after ~1 hour, so a token read once at
// startup (`FCM_OAUTH_TOKEN`) breaks Android push after the first hour.
//
// [`FcmTokenProvider`] fixes that: given Google service-account credentials it
// mints a short-lived assertion JWT (RS256), exchanges it at the OAuth2 token
// endpoint for an access token, caches the result, and re-mints a fresh token
// shortly *before* the cached one expires. The legacy static token is kept only
// as a fall-back for deployments that still set `FCM_OAUTH_TOKEN`.

/// The OAuth2 scope required to send messages via FCM HTTP v1.
const FCM_OAUTH_SCOPE: &str = "https://www.googleapis.com/auth/firebase.messaging";

/// Default Google OAuth2 token endpoint (used when the service-account JSON
/// omits `token_uri`).
const DEFAULT_GOOGLE_TOKEN_URI: &str = "https://oauth2.googleapis.com/token";

/// How long before an access token's real expiry we proactively refresh it.
///
/// Google tokens live ~3600 s; refreshing 5 minutes early keeps a comfortable
/// margin so an in-flight send never races the expiry boundary.
const FCM_TOKEN_REFRESH_SKEW_SECS: i64 = 300;

/// Google service-account credentials needed for the OAuth2 JWT-bearer flow.
///
/// Parsed from the service-account JSON file Google issues (`type`,
/// `client_email`, `private_key`, `token_uri`, …); only the three fields the
/// token exchange needs are retained.
#[derive(Clone, Debug)]
pub struct FcmServiceAccount {
    /// Service-account identity (`iss` / `sub` of the assertion JWT).
    client_email: String,
    /// PEM-encoded PKCS#8 RSA private key (`private_key` in the JSON).
    private_key_pem: String,
    /// OAuth2 token endpoint the assertion is exchanged at.
    token_uri: String,
}

/// The subset of the Google service-account JSON we deserialize.
#[derive(Debug, Deserialize)]
struct ServiceAccountJson {
    client_email: String,
    private_key: String,
    #[serde(default = "default_token_uri")]
    token_uri: String,
}

fn default_token_uri() -> String {
    DEFAULT_GOOGLE_TOKEN_URI.to_string()
}

impl FcmServiceAccount {
    /// Parse a service-account from the raw JSON string Google issues.
    pub fn from_json(raw: &str) -> Result<Self, String> {
        let parsed: ServiceAccountJson =
            serde_json::from_str(raw).map_err(|e| format!("invalid service-account JSON: {e}"))?;
        if parsed.client_email.trim().is_empty() || parsed.private_key.trim().is_empty() {
            return Err("service-account JSON missing client_email or private_key".to_string());
        }
        Ok(Self {
            client_email: parsed.client_email,
            private_key_pem: parsed.private_key,
            token_uri: parsed.token_uri,
        })
    }

    /// Load a service-account from the environment.
    ///
    /// Prefers the inline `FCM_SERVICE_ACCOUNT_JSON`; otherwise reads the file
    /// referenced by `GOOGLE_APPLICATION_CREDENTIALS`. Returns `None` when
    /// neither is set; logs and returns `None` on a parse / read error (so the
    /// server still starts — push just degrades to the legacy / no-op path).
    pub fn from_env() -> Option<Self> {
        if let Ok(inline) = std::env::var("FCM_SERVICE_ACCOUNT_JSON") {
            if !inline.trim().is_empty() {
                return match Self::from_json(&inline) {
                    Ok(sa) => Some(sa),
                    Err(e) => {
                        tracing::error!(error = %e, "[8A-3] FCM: failed to parse FCM_SERVICE_ACCOUNT_JSON");
                        None
                    }
                };
            }
        }
        if let Ok(path) = std::env::var("GOOGLE_APPLICATION_CREDENTIALS") {
            if !path.trim().is_empty() {
                return match std::fs::read_to_string(&path) {
                    Ok(raw) => match Self::from_json(&raw) {
                        Ok(sa) => Some(sa),
                        Err(e) => {
                            tracing::error!(error = %e, path = %path, "[8A-3] FCM: failed to parse service-account file");
                            None
                        }
                    },
                    Err(e) => {
                        tracing::error!(error = %e, path = %path, "[8A-3] FCM: failed to read GOOGLE_APPLICATION_CREDENTIALS file");
                        None
                    }
                };
            }
        }
        None
    }
}

/// Claims for the Google OAuth2 JWT-bearer assertion.
#[derive(Debug, Serialize)]
struct GoogleAssertionClaims<'a> {
    iss: &'a str,
    scope: &'a str,
    aud: &'a str,
    iat: i64,
    exp: i64,
}

/// The token-endpoint response we care about.
#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
    expires_in: i64,
}

/// A cached OAuth2 access token plus the instant at which it should be refreshed.
#[derive(Clone, Debug)]
struct CachedAccessToken {
    token: String,
    /// Refresh once `Utc::now()` reaches this point — set to the real expiry
    /// minus [`FCM_TOKEN_REFRESH_SKEW_SECS`].
    refresh_after: chrono::DateTime<chrono::Utc>,
}

/// Compute the instant at which a freshly-minted token should be refreshed.
///
/// `refresh_after = now + max(expires_in - skew, expires_in / 2)` — clamped so a
/// short-lived token (or a bogus tiny `expires_in`) still gets *some* cache
/// lifetime rather than refreshing on every single send.
fn refresh_after_from(
    now: chrono::DateTime<chrono::Utc>,
    expires_in_secs: i64,
    skew_secs: i64,
) -> chrono::DateTime<chrono::Utc> {
    let lifetime = (expires_in_secs - skew_secs)
        .max(expires_in_secs / 2)
        .max(0);
    now + chrono::Duration::seconds(lifetime)
}

/// Returns `true` when the cached token is still safe to use at `now`.
fn token_is_fresh(cached: &CachedAccessToken, now: chrono::DateTime<chrono::Utc>) -> bool {
    now < cached.refresh_after
}

///
/// Holds the current token behind an async `Mutex`; the first caller past the
/// refresh boundary re-mints while others await, so at most one token exchange
/// is in flight at a time.
struct ServiceAccountTokenSource {
    service_account: FcmServiceAccount,
    /// Shared HTTP client for the OAuth2 token exchange. `None` when the client
    /// could not be built at startup (TLS/resolver init failure); token refresh
    /// is then skipped rather than panicking the fanout worker.
    http: Option<reqwest::Client>,
    cache: Mutex<Option<CachedAccessToken>>,
}

impl ServiceAccountTokenSource {
    fn new(service_account: FcmServiceAccount) -> Self {
        let http = match build_push_http_client(Duration::from_secs(10), false) {
            Ok(client) => Some(client),
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "[8A-3] FCM: failed to build OAuth2 token-exchange HTTP client; \
                     access-token refresh will be skipped until the process restarts"
                );
                None
            }
        };
        Self {
            service_account,
            http,
            cache: Mutex::new(None),
        }
    }

    /// Return a valid access token, refreshing it if the cached one is missing
    /// or within the refresh skew of expiry. Returns `None` if a refresh was
    /// required but failed (mint or exchange error) and no still-fresh token is
    /// cached.
    async fn access_token(&self) -> Option<String> {
        let mut guard = self.cache.lock().await;
        let now = chrono::Utc::now();
        if let Some(cached) = guard.as_ref() {
            if token_is_fresh(cached, now) {
                return Some(cached.token.clone());
            }
        }
        match self.refresh(now).await {
            Ok(fresh) => {
                let token = fresh.token.clone();
                *guard = Some(fresh);
                Some(token)
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "[8A-3] FCM: failed to refresh OAuth2 access token; Android push will be skipped this attempt"
                );
                // Keep any (stale) cached token in place; a still-usable token
                // would have short-circuited above, so nothing usable is cached.
                None
            }
        }
    }

    /// Mint an assertion JWT and exchange it for a fresh access token.
    async fn refresh(
        &self,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<CachedAccessToken, String> {
        let http = self
            .http
            .as_ref()
            .ok_or("OAuth2 token-exchange HTTP client unavailable (build failed at startup)")?;
        let assertion = self.mint_assertion(now)?;
        let resp = http
            .post(&self.service_account.token_uri)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", assertion.as_str()),
            ])
            .send()
            .await
            .map_err(|e| format!("token endpoint request failed: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("token endpoint returned {status}: {body}"));
        }
        let token_resp: GoogleTokenResponse = resp
            .json()
            .await
            .map_err(|e| format!("failed to parse token endpoint response: {e}"))?;

        Ok(CachedAccessToken {
            token: token_resp.access_token,
            refresh_after: refresh_after_from(
                now,
                token_resp.expires_in,
                FCM_TOKEN_REFRESH_SKEW_SECS,
            ),
        })
    }

    /// Build the RS256 assertion JWT for the OAuth2 JWT-bearer grant.
    fn mint_assertion(&self, now: chrono::DateTime<chrono::Utc>) -> Result<String, String> {
        let encoding_key =
            EncodingKey::from_rsa_pem(self.service_account.private_key_pem.as_bytes())
                .map_err(|e| format!("invalid service-account private key: {e}"))?;
        let iat = now.timestamp();
        let claims = GoogleAssertionClaims {
            iss: &self.service_account.client_email,
            scope: FCM_OAUTH_SCOPE,
            aud: &self.service_account.token_uri,
            iat,
            // Google caps assertion lifetime at 1 hour.
            exp: iat + 3600,
        };
        jsonwebtoken::encode(&Header::new(Algorithm::RS256), &claims, &encoding_key)
            .map_err(|e| format!("failed to sign assertion JWT: {e}"))
    }
}

/// Source of the OAuth2 bearer token used on the FCM HTTP v1 send path.
///
/// Cloneable and cheap to pass around: the refreshing variant shares one
/// `ServiceAccountTokenSource` (and its token cache) behind an `Arc`.
#[derive(Clone)]
enum FcmTokenProvider {
    /// No OAuth token available — FCM HTTP v1 sends are skipped (there is no
    /// legacy fallback; Google decommissioned the `/fcm/send` server-key API).
    None,
    /// Legacy pre-minted token from `FCM_OAUTH_TOKEN`, never refreshed.
    Static(String),
    /// Service-account credentials; mints and auto-refreshes access tokens.
    ServiceAccount(Arc<ServiceAccountTokenSource>),
}

impl FcmTokenProvider {
    /// Build a provider from the environment, preferring a service account over
    /// a legacy static token.
    ///
    /// `static_fallback` is the value of `FcmConfig::oauth_token`
    /// (`FCM_OAUTH_TOKEN`), used only when no service account is configured.
    fn from_env(static_fallback: Option<String>) -> Self {
        if let Some(sa) = FcmServiceAccount::from_env() {
            tracing::info!("[8A-3] FCM: using auto-refreshing OAuth2 service-account credentials");
            return FcmTokenProvider::ServiceAccount(Arc::new(ServiceAccountTokenSource::new(sa)));
        }
        match static_fallback {
            Some(t) => {
                tracing::warn!(
                    "[8A-3] FCM: using static FCM_OAUTH_TOKEN — it is NOT refreshed and will \
                     stop working after Google's ~1h token TTL. Set FCM_SERVICE_ACCOUNT_JSON or \
                     GOOGLE_APPLICATION_CREDENTIALS for auto-refreshed credentials."
                );
                FcmTokenProvider::Static(t)
            }
            None => FcmTokenProvider::None,
        }
    }

    /// Return a valid bearer token, refreshing it if needed.
    ///
    /// `None` means "no usable OAuth token right now" — the caller skips the
    /// FCM HTTP v1 send (there is no legacy fallback).
    async fn access_token(&self) -> Option<String> {
        match self {
            FcmTokenProvider::None => None,
            FcmTokenProvider::Static(t) => Some(t.clone()),
            FcmTokenProvider::ServiceAccount(source) => source.access_token().await,
        }
    }
}

// ============================================================================
// Story 8A-3: FCM HTTP transport adapter
// ============================================================================

/// A `PushTransport` implementation that delivers messages via FCM HTTP v1.
///
/// The adapter:
/// - Fetches device tokens for the target user from the DB (service-role pool —
///   no RLS context set; see `device_push_tokens_service_policy`).
/// - Sends one FCM request per FCM token; APNs tokens are logged as unsupported
///   for now (APNs requires a separate HTTP/2 connection pool and P8 key).
/// - Deletes tokens the gateway reports as no longer registered.
/// - Is silently no-op when FCM credentials are not configured.
#[derive(Clone)]
pub struct FcmHttpAdapter {
    token_repo: DevicePushTokenRepository,
    pub(crate) fcm_config: FcmConfig,
    /// Shared HTTP client for FCM HTTP v1 sends. `None` when the client could
    /// not be built at startup (TLS/resolver init failure); sends are then
    /// skipped rather than panicking the fanout worker.
    http: Option<reqwest::Client>,
    /// Source of the OAuth2 bearer token for FCM HTTP v1 sends. Refreshes
    /// automatically when backed by a service account (see [`FcmTokenProvider`]).
    token_provider: FcmTokenProvider,
}

impl FcmHttpAdapter {
    /// Create a new adapter backed by the given pool and FCM config.
    ///
    /// The OAuth2 token provider is derived from the environment: a service
    /// account (`FCM_SERVICE_ACCOUNT_JSON` / `GOOGLE_APPLICATION_CREDENTIALS`)
    /// yields an auto-refreshing provider; otherwise the legacy
    /// `FcmConfig::oauth_token` becomes a static (non-refreshing) provider. This
    /// keeps the FCM base-URL / project-id config explicit (so tests can inject a
    /// wiremock URL) while the credential lifecycle is owned by the provider.
    pub fn new(pool: DbPool, fcm_config: FcmConfig) -> Self {
        let token_provider = FcmTokenProvider::from_env(fcm_config.oauth_token.clone());
        Self::with_token_provider(pool, fcm_config, token_provider)
    }

    /// Create an adapter with an explicit token provider (used by tests to
    /// inject a service-account source pointed at a mock token endpoint).
    fn with_token_provider(
        pool: DbPool,
        fcm_config: FcmConfig,
        token_provider: FcmTokenProvider,
    ) -> Self {
        let http = match build_push_http_client(Duration::from_secs(10), false) {
            Ok(client) => Some(client),
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "[8A-3] FCM: failed to build HTTP v1 send client; \
                     Android push will be skipped until the process restarts"
                );
                None
            }
        };
        Self {
            token_repo: DevicePushTokenRepository::new(pool),
            fcm_config,
            http,
            token_provider,
        }
    }

    /// Load FCM config from environment and wire the adapter.
    pub fn from_env(pool: DbPool) -> Self {
        Self::new(pool, FcmConfig::from_env())
    }

    // ------------------------------------------------------------------
    // Internal: send one FCM message via HTTP v1
    // ------------------------------------------------------------------

    /// Send a single push notification to one FCM registration token.
    ///
    /// Returns `(success, token_expired)`.  When `token_expired` is `true` the
    /// caller must delete the token from the DB.
    async fn send_fcm_v1(
        &self,
        project_id: &str,
        device_token: &str,
        notification: &Notification,
    ) -> (bool, bool) {
        let url = format!(
            "{}/v1/projects/{project_id}/messages:send",
            self.fcm_config.base_url()
        );

        // Build the FCM message payload.
        let body = serde_json::json!({
            "message": {
                "token": device_token,
                "notification": {
                    "title": notification.title,
                    "body": notification.body,
                },
                "data": {
                    "notification_id": notification.id.to_string(),
                    "category": notification.category.as_str(),
                    "priority": notification.priority.as_str(),
                }
            }
        });

        // For FCM HTTP v1 we need an OAuth2 bearer token. The provider mints and
        // auto-refreshes it from service-account credentials (or yields the
        // legacy static `FCM_OAUTH_TOKEN`). Crucially this is resolved per send,
        // so a token that has aged past Google's ~1h TTL is refreshed rather
        // than reused — the previous implementation read a static token once at
        // startup and Android push silently died after the first hour. When no
        // OAuth token is available the send is skipped: Google decommissioned
        // the legacy `/fcm/send` server-key API, so there is no fallback.
        let bearer = match self.token_provider.access_token().await {
            Some(t) => t,
            None => {
                tracing::warn!(
                    "[8A-3] FCM HTTP v1 send skipped — no OAuth2 access token available \
                     (set FCM_SERVICE_ACCOUNT_JSON or GOOGLE_APPLICATION_CREDENTIALS)"
                );
                return (false, false);
            }
        };

        let Some(http) = self.http.as_ref() else {
            tracing::warn!(
                "[8A-3] FCM HTTP v1 send skipped — HTTP client unavailable (build failed at startup)"
            );
            return (false, false);
        };

        let resp = match http
            .post(&url)
            .bearer_auth(&bearer)
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "[8A-3] FCM HTTP v1 request failed (network error)"
                );
                return (false, false);
            }
        };

        let status = resp.status();
        match resp.json::<FcmSendResponse>().await {
            Ok(fcm_resp) => {
                let (success, expired) = classify_fcm_outcome(status.is_success(), Some(&fcm_resp));
                if !success {
                    let err = fcm_resp.error.as_ref();
                    let err_code = err.and_then(|e| e.log_code()).unwrap_or("UNKNOWN");
                    tracing::warn!(
                        fcm_status = %err_code,
                        token_expired = expired,
                        "[8A-3] FCM rejected message"
                    );
                }
                (success, expired)
            }
            Err(e) => {
                let (success, expired) = classify_fcm_outcome(status.is_success(), None);
                if success {
                    // 2xx with a body we can't parse: FCM already accepted the
                    // message, so retrying would re-deliver it. Treat as
                    // delivered rather than a failed (retryable) attempt.
                    tracing::warn!(
                        http_status = %status,
                        error = %e,
                        "[8A-3] FCM returned 2xx with an unparseable body — treating as delivered (no retry)"
                    );
                } else {
                    tracing::warn!(
                        http_status = %status,
                        error = %e,
                        "[8A-3] Failed to parse FCM HTTP v1 response"
                    );
                }
                (success, expired)
            }
        }
    }
}

impl FcmHttpAdapter {
    /// Deliver to the FCM (Android) slice of a *pre-fetched* device-token set.
    ///
    /// The device tokens are fetched by the caller (once) and handed in — this
    /// is what lets [`CombinedPushAdapter`] run FCM and APNs off a single
    /// `get_tokens_for_user` query instead of one per provider. The returned
    /// [`ProviderOutcome`] is richer than a bare `Result` so the combined
    /// adapter can tell "actually delivered" from "configured but had no FCM
    /// targets" — the distinction the honest-metrics contract needs.
    pub(crate) async fn deliver(
        &self,
        user_id: Uuid,
        tokens: &[DevicePushToken],
        notification: &Notification,
    ) -> ProviderOutcome {
        if !self.fcm_config.is_configured() {
            tracing::info!(
                user_id = %user_id,
                title = %notification.title,
                "[8A-3] Push skipped — FCM not configured (set FCM_PROJECT_ID)"
            );
            return ProviderOutcome::NotConfigured;
        }

        // Select only the FCM devices from the shared token set. APNs tokens are
        // routed to `ApnsHttpAdapter` by `CombinedPushAdapter`, so this adapter
        // never touches them (no more "APNs not yet implemented — log-only").
        let filter = PushTargetFilter {
            platforms: Some(vec![PushPlatform::Fcm]),
            ..Default::default()
        };
        let targets = select_dispatch_targets(tokens, &filter);

        if targets.is_empty() {
            tracing::debug!(
                user_id = %user_id,
                stored = tokens.len(),
                "[8A-3] No FCM dispatch targets selected for user — nothing to deliver"
            );
            return ProviderOutcome::NoTargets;
        }

        // `is_configured()` requires a project id, and we returned early above
        // when it was false, so a project id is guaranteed here. Guard
        // defensively rather than unwrap/panic on the fanout hot path.
        let Some(project_id) = self.fcm_config.project_id.clone() else {
            return ProviderOutcome::NotConfigured;
        };
        let mut any_sent = false;
        let mut stale_tokens: Vec<(Uuid, String)> = Vec::new();

        for target in &targets {
            let (success, expired) = self
                .send_fcm_v1(&project_id, &target.token, notification)
                .await;

            // Store delivery receipt (log + in-memory; DB table in follow-up)
            let receipt = PushDeliveryReceipt {
                token_id: target.token_id,
                user_id,
                platform: PushPlatform::Fcm,
                success,
                error: if success {
                    None
                } else {
                    Some("FCM rejected".to_string())
                },
                token_expired: expired,
                attempted_at: chrono::Utc::now(),
            };
            tracing::info!(
                token_id = %receipt.token_id,
                user_id = %receipt.user_id,
                success = receipt.success,
                token_expired = receipt.token_expired,
                "[8A-3] Push delivery receipt"
            );

            if expired {
                stale_tokens.push((target.token_id, target.token.clone()));
            }
            if success {
                any_sent = true;
            }
        }

        // Purge stale tokens (token_expired == true) so they are not retried.
        // We call `delete_stale_token_by_string` which is a best-effort log;
        // the next upsert from the device will evict the token via ON CONFLICT DO UPDATE.
        for (stale_id, stale_token) in stale_tokens {
            if let Err(e) =
                delete_stale_token_by_string(&self.token_repo, user_id, &stale_token).await
            {
                tracing::warn!(
                    error = %e,
                    token_id = %stale_id,
                    "[8A-3] Failed to mark stale push token for eviction"
                );
            } else {
                tracing::info!(
                    token_id = %stale_id,
                    "[8A-3] Stale push token queued for eviction (NOT_REGISTERED)"
                );
            }
        }

        if any_sent {
            ProviderOutcome::Delivered
        } else {
            ProviderOutcome::Failed(NotificationError::PushFailed(
                "all FCM delivery attempts failed".to_string(),
            ))
        }
    }
}

#[async_trait]
impl PushTransport for FcmHttpAdapter {
    async fn send(&self, user_id: Uuid, notification: &Notification) -> TransportResult {
        // Standalone use (not via `CombinedPushAdapter`): fetch the user's
        // tokens ourselves, then run the shared delivery core.
        if !self.fcm_config.is_configured() {
            return ProviderOutcome::NotConfigured.into_result();
        }
        let tokens = match self.token_repo.get_tokens_for_user(user_id).await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(
                    user_id = %user_id,
                    error = %e,
                    "[8A-3] Failed to fetch device tokens for push delivery"
                );
                return Err(NotificationError::PushFailed(
                    "failed to fetch device tokens".to_string(),
                ));
            }
        };
        self.deliver(user_id, &tokens, notification)
            .await
            .into_result()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // FCM HTTP v1 error parsing / stale-token eviction
    // ------------------------------------------------------------------

    #[test]
    fn fcm_v1_unregistered_error_marks_token_stale() {
        // Realistic FCM HTTP v1 404 body for an app-uninstalled / unregistered
        // token. The *actionable* error code (`UNREGISTERED`) is carried in
        // `error.details[].errorCode`; `error.status` is only the generic
        // google.rpc status (`NOT_FOUND`). Eviction must trigger on the
        // details code — matching on `status` alone never fires for real v1
        // responses, so stale Android tokens would accumulate and be retried
        // forever.
        let body = serde_json::json!({
            "error": {
                "code": 404,
                "message": "Requested entity was not found.",
                "status": "NOT_FOUND",
                "details": [{
                    "@type": "type.googleapis.com/google.firebase.fcm.v1.FcmError",
                    "errorCode": "UNREGISTERED"
                }]
            }
        })
        .to_string();
        let resp: FcmSendResponse = serde_json::from_str(&body).expect("parse FCM v1 error body");
        let err = resp.error.expect("error block present");
        assert!(
            err.indicates_stale_token(),
            "FCM v1 UNREGISTERED (in error.details) must evict the stale token"
        );
        assert_eq!(
            err.log_code(),
            Some("UNREGISTERED"),
            "log_code must prefer the FCM-specific details code over the status"
        );
    }

    #[test]
    fn fcm_v1_invalid_argument_does_not_evict_token() {
        // A malformed payload yields INVALID_ARGUMENT. Evicting on it would wipe
        // every device token the moment a payload bug ships, so it must NOT be
        // treated as a stale token.
        let body = serde_json::json!({
            "error": {
                "code": 400,
                "message": "Invalid value at 'message.data'.",
                "status": "INVALID_ARGUMENT",
                "details": [{
                    "@type": "type.googleapis.com/google.firebase.fcm.v1.FcmError",
                    "errorCode": "INVALID_ARGUMENT"
                }]
            }
        })
        .to_string();
        let resp: FcmSendResponse = serde_json::from_str(&body).expect("parse FCM v1 error body");
        let err = resp.error.expect("error block present");
        assert!(
            !err.indicates_stale_token(),
            "INVALID_ARGUMENT must not evict the token (could be a payload bug)"
        );
    }

    #[test]
    fn fcm_v1_status_only_unregistered_still_evicts() {
        // Fall-back path: a response that carries the stale code only in
        // `status` (legacy / test mocks, no `details`) must still evict.
        let body = serde_json::json!({ "error": { "status": "UNREGISTERED" } }).to_string();
        let resp: FcmSendResponse = serde_json::from_str(&body).expect("parse FCM error body");
        assert!(resp.error.expect("error block").indicates_stale_token());
    }

    #[test]
    fn fcm_v1_success_response_has_no_error() {
        // A 200 send response carries `name` and no `error` block.
        let body = serde_json::json!({
            "name": "projects/p/messages/0:1700000000000000%abcd"
        })
        .to_string();
        let resp: FcmSendResponse = serde_json::from_str(&body).expect("parse FCM success body");
        assert!(resp.error.is_none());
    }

    // ------------------------------------------------------------------
    // Outcome classification — a 2xx must never be retried on body-parse
    // failure (duplicate-delivery guard)
    // ------------------------------------------------------------------

    #[test]
    fn classify_2xx_unparseable_body_is_delivered_not_retried() {
        // FCM returned a 2xx — it has ALREADY accepted the message for
        // delivery — but the body did not deserialize into FcmSendResponse
        // (`parsed = None`). Reporting this as a failed attempt makes the
        // fanout worker requeue the job (PushFailed is retryable), so FCM
        // delivers the same push a second time. A 2xx must be treated as
        // delivered regardless of body shape.
        let (delivered, expired) = classify_fcm_outcome(true, None);
        assert!(
            delivered,
            "a 2xx FCM response with an unparseable body must be treated as delivered (no retry)"
        );
        assert!(!expired, "an unparseable 2xx body must not evict the token");
    }

    #[test]
    fn classify_non_2xx_unparseable_body_is_failed_attempt() {
        // A non-2xx response whose body we cannot parse is a genuine failed
        // attempt the caller may retry (transport / 5xx class). It must NOT be
        // reported as delivered.
        let (delivered, expired) = classify_fcm_outcome(false, None);
        assert!(
            !delivered,
            "a non-2xx unparseable response is a failed (retryable) attempt, not delivered"
        );
        assert!(!expired);
    }

    #[test]
    fn classify_2xx_success_body_is_delivered() {
        let resp = FcmSendResponse {
            name: Some("projects/p/messages/1".into()),
            error: None,
        };
        assert_eq!(classify_fcm_outcome(true, Some(&resp)), (true, false));
    }

    #[test]
    fn classify_rejection_with_unregistered_marks_stale() {
        // A non-2xx rejection carrying UNREGISTERED → not delivered, and the
        // token is flagged for eviction.
        let resp = FcmSendResponse {
            name: None,
            error: Some(FcmError {
                status: Some("NOT_FOUND".into()),
                message: None,
                details: vec![FcmErrorDetail {
                    error_code: Some("UNREGISTERED".into()),
                }],
            }),
        };
        assert_eq!(classify_fcm_outcome(false, Some(&resp)), (false, true));
    }

    #[test]
    fn fcm_config_explicit_none_is_unconfigured() {
        let config = FcmConfig {
            project_id: None,
            oauth_token: None,
            fcm_base_url: None,
        };
        assert!(!config.is_configured());
    }

    #[test]
    fn fcm_config_is_configured_when_project_id_present() {
        let config = FcmConfig {
            project_id: Some("my-gcp-project".to_string()),
            oauth_token: None,
            fcm_base_url: None,
        };
        assert!(config.is_configured());
    }

    #[test]
    fn fcm_config_base_url_defaults_to_google() {
        let config = FcmConfig {
            project_id: None,
            oauth_token: None,
            fcm_base_url: None,
        };
        assert_eq!(config.base_url(), "https://fcm.googleapis.com");
    }

    #[test]
    fn fcm_config_base_url_override() {
        let config = FcmConfig {
            project_id: None,
            oauth_token: None,
            fcm_base_url: Some("http://localhost:9999".to_string()),
        };
        assert_eq!(config.base_url(), "http://localhost:9999");
    }

    // ------------------------------------------------------------------
    // FCM HTTP v1 OAuth2 access-token provider (auto-refresh)
    // ------------------------------------------------------------------

    /// A throwaway 2048-bit RSA private key used ONLY to satisfy
    /// `EncodingKey::from_rsa_pem` in the assertion-minting tests. It is not a
    /// real credential and grants access to nothing — the mock token endpoint
    /// never validates the assertion signature.
    const TEST_RSA_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC/s+Ti7fA564gJ
XBr7Schx8zydvks/oA8g53tDIR7XZ3vnQrYugzD1ww+L9kmAmWz5H/zOG4c9ZkNq
Vj5hpl19vGwJ/mcfSbvX8d55m4cM1tQg6XlbodgGz90McNRm6SH1xhDvkHkjaLge
BI7nbukDZhrfLS4uAndb9ClwxERDFnCAwzSS4jpzLucY+2/lkwxaLGpB8PuWUeRm
Jmwt5Bv6lMKDqz2vSJP+ij0pyXh53Xw3r8uPc/QrROqNFbmO+oXbtEh/aDQHJojT
YK4dvs8YVjLYz/s/Tmtd+fhirhh9X21VNhQk7URUIuHyrje9Y2rUkckxHY5SkzOj
2nYJodltAgMBAAECggEAIG7PTdxbHPV+A7VfRD3oqWyxR+/Su8442QSIzGPtW5yg
sBDPkUF1VlL8zZ1qtJTghKp2gylRoV/sjnBOaAd1QElRTwSJTlgTbXa4gMMBH3k2
FOZjN49DZO2kdI8fRFTzf6kVou5CrGyyX7O+OKYBSqeq6rCyZCrbJkXCABfYg6/c
WujJ4bKPIiF4723Gr4PPh1N2mEPz+aEJH5ikvdFSzOdbki2eut6z8gnMphBBvR8Y
T5LQuHRoWByyjUS/qUq+x9u54GkLrfx4NIe5mOooAOngo+9fdsN4+Qf29Rw8Pljb
ZYPINLT1WS8U8WlUMzszW/NLw+1Fakygdw8O+PRU2QKBgQDrRDH5tx7qyWT5ufmg
j+O4zbQbxm9VAud475SqmtDlAeMMYvD4LTylfv23563hhnhDJjJp7GnFFE6xge1K
sdZem7YaNvYlHVo8yt3NxiHIaLB2iKM/KXnIefEafGzNf5h7dbO0HDqiRD1YGXhy
exIFNdFjR18Ri5zzlJqTAp/vZQKBgQDQmN3AC4daqRl1wGye4ehO0XPzT+Kj3eel
bf9SIIOUA1cSebTahY1Fq1MtAt7UuKCaOg8n2f/FFvjuk/q/i7o3M6bjngTtnPM8
uENJnHmR3Lx+bzeMWjJ/T6zEUMrtV5+2NaAbq0caR+ZGPVa6E7jHk1ecjX+tvZe/
nRxdR+n1aQKBgQCbbNIXRwMF2Ub8NADWMjkfPcZfExk58FE7dAujKeQXZse4xySq
0DfgnaTAei5Fb7DDq9hiYez+ZgwW+N7rGdGlbvk/GFBE9L9Iqj0eVGa9H2x04o/2
ilAKQYUnGkxG9qSl63xs4LlbuflM2obYGrYs+wD5tYz46mMmCGaV+IXwgQKBgBRF
lt9P/4J3Botj/Opf5/So9EzECbGFIjr4eqSflknvHSole8b0zarkoHuyWLdxjeIP
HGPyEqIzvlNpPCgbSyiMM37RX4c8BoNzIM7pjwL24baj1lEkft3Sf2bAt0fjiRjr
Ezk9JvbN3/oZgfEpc36pugzzz2GyGCo9+YCzOXBpAoGAMpzIMkHEJJL6Vrs4bMC3
aZJwgYwjbWDgg/JCZIlOqPJHCA8Q7tNIKCUlpW6HkGlwLGuYq0SeSnLv4bJnaH/M
hbsuc8ONCSkQ+U7GAEGP6ERKl30weXlvVntAs+/lAbFCoEZVmeNMN7+ZgpOJCP+8
BjoqgEdIaMmiy1bJKyDvIng=
-----END PRIVATE KEY-----";

    fn test_service_account(token_uri: String) -> FcmServiceAccount {
        FcmServiceAccount {
            client_email: "test-sa@proj.iam.gserviceaccount.com".to_string(),
            private_key_pem: TEST_RSA_PEM.to_string(),
            token_uri,
        }
    }

    #[test]
    fn refresh_after_subtracts_skew_for_normal_ttl() {
        let now = chrono::Utc::now();
        // 3600s token, 300s skew → refresh 3300s out.
        let ra = refresh_after_from(now, 3600, 300);
        assert_eq!((ra - now).num_seconds(), 3300);
    }

    #[test]
    fn refresh_after_clamps_short_ttl_to_half() {
        let now = chrono::Utc::now();
        // A 120s token with a 300s skew must not produce a negative/zero
        // lifetime (which would refresh on every send) — clamp to expires_in/2.
        let ra = refresh_after_from(now, 120, 300);
        assert_eq!((ra - now).num_seconds(), 60);
    }

    #[test]
    fn token_is_fresh_before_refresh_point_and_stale_after() {
        let now = chrono::Utc::now();
        let fresh = CachedAccessToken {
            token: "t".into(),
            refresh_after: now + chrono::Duration::seconds(10),
        };
        assert!(token_is_fresh(&fresh, now));
        let stale = CachedAccessToken {
            token: "t".into(),
            refresh_after: now - chrono::Duration::seconds(1),
        };
        assert!(!token_is_fresh(&stale, now));
    }

    #[test]
    fn service_account_from_json_parses_and_defaults_token_uri() {
        let raw = serde_json::json!({
            "type": "service_account",
            "client_email": "sa@proj.iam.gserviceaccount.com",
            "private_key": "-----BEGIN PRIVATE KEY-----\nabc\n-----END PRIVATE KEY-----",
        })
        .to_string();
        let sa = FcmServiceAccount::from_json(&raw).expect("valid SA JSON parses");
        assert_eq!(sa.client_email, "sa@proj.iam.gserviceaccount.com");
        assert_eq!(sa.token_uri, DEFAULT_GOOGLE_TOKEN_URI);
    }

    #[test]
    fn service_account_from_json_rejects_missing_fields() {
        let raw = serde_json::json!({ "client_email": "x@y.z" }).to_string();
        assert!(FcmServiceAccount::from_json(&raw).is_err());
    }

    #[test]
    fn static_provider_returns_its_token_and_none_provider_yields_none() {
        // Pure provider behaviour, no async runtime needed for the None arm via
        // a blocking check is awkward; use a runtime for both.
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let s = FcmTokenProvider::Static("legacy-token".into());
            assert_eq!(s.access_token().await.as_deref(), Some("legacy-token"));
            let n = FcmTokenProvider::None;
            assert!(n.access_token().await.is_none());
        });
    }

    /// The token source fetches an access token from the OAuth2 endpoint once
    /// and then serves subsequent calls from its cache (only ONE exchange for
    /// two `access_token()` calls inside the TTL).
    #[tokio::test]
    async fn service_account_source_fetches_then_caches() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/token"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "access_token": "tok-A",
                    "expires_in": 3600,
                    "token_type": "Bearer"
                })),
            )
            .expect(1) // cached on the second call — exactly one exchange
            .mount(&server)
            .await;

        let source =
            ServiceAccountTokenSource::new(test_service_account(format!("{}/token", server.uri())));

        assert_eq!(source.access_token().await.as_deref(), Some("tok-A"));
        assert_eq!(
            source.access_token().await.as_deref(),
            Some("tok-A"),
            "second call within TTL must be served from cache"
        );
        // wiremock verifies expect(1) on drop.
    }

    /// Regression guard for the static-token bug: a cached token that has aged
    /// past its refresh point must trigger a fresh exchange, NOT be reused. The
    /// previous implementation read `FCM_OAUTH_TOKEN` once at startup and had no
    /// refresh path at all, so Android push died after Google's ~1h TTL.
    #[tokio::test]
    async fn service_account_source_refreshes_stale_token() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/token"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "access_token": "fresh-tok",
                    "expires_in": 3600,
                    "token_type": "Bearer"
                })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let source =
            ServiceAccountTokenSource::new(test_service_account(format!("{}/token", server.uri())));

        // Pre-seed the cache with an already-expired token.
        {
            let mut guard = source.cache.lock().await;
            *guard = Some(CachedAccessToken {
                token: "stale-tok".to_string(),
                refresh_after: chrono::Utc::now() - chrono::Duration::seconds(10),
            });
        }

        let got = source.access_token().await;
        assert_eq!(
            got.as_deref(),
            Some("fresh-tok"),
            "an expired cached token must be refreshed, not reused"
        );
    }

    /// A token-endpoint failure surfaces as `None` (caller falls back / skips)
    /// rather than handing back a stale/garbage bearer.
    #[tokio::test]
    async fn service_account_source_returns_none_on_endpoint_error() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/token"))
            .respond_with(
                wiremock::ResponseTemplate::new(401).set_body_json(serde_json::json!({
                    "error": "invalid_grant"
                })),
            )
            .mount(&server)
            .await;

        let source =
            ServiceAccountTokenSource::new(test_service_account(format!("{}/token", server.uri())));

        assert!(source.access_token().await.is_none());
    }

    // ------------------------------------------------------------------
    // Shared adapter-level test helpers (DB-less: a lazy pool that fails fast,
    // a Static token provider, and a configured FcmConfig). These let the
    // HTTP-v1 send path and the standalone `send()` error path run without a
    // live Postgres or FCM.
    // ------------------------------------------------------------------

    /// A lazy pool with a short acquire timeout pointed at an unreachable
    /// address: the first query fails fast with a connection error, so `send()`'s
    /// device-token fetch exercises the DB-error path without a live Postgres.
    fn failing_pool() -> DbPool {
        sqlx::postgres::PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(250))
            .connect_lazy("postgres://never-used:never@127.0.0.1:1/never")
            .expect("connect_lazy builds without connecting")
    }

    /// A configured FCM config (project id present → `is_configured()`), with an
    /// optional base-URL override so a test can point the HTTP-v1 send at a
    /// wiremock server.
    fn configured_fcm_config(base_url: Option<String>) -> FcmConfig {
        FcmConfig {
            project_id: Some("test-project".to_string()),
            oauth_token: None,
            fcm_base_url: base_url,
        }
    }

    fn fcm_test_notification() -> Notification {
        Notification::new(
            Uuid::new_v4(),
            common::notifications::NotificationCategory::Announcements,
            "Building update",
            "The lift will be serviced tomorrow.",
        )
    }

    // ------------------------------------------------------------------
    // 2xx-FCM-delivered guard (#3725) — END TO END through the real HTTP-v1
    // send path.
    //
    // The classify_fcm_outcome unit tests above pin the pure decision. These
    // drive the actual `send_fcm_v1` request/response wiring (status +
    // body-parse Err arm) against a mock FCM, so a future refactor that rewires
    // the arm (e.g. passes `false` instead of `status.is_success()`, or drops
    // the classifier) is caught even if the pure function stays correct.
    // ------------------------------------------------------------------

    /// A 2xx whose body does NOT deserialize into `FcmSendResponse` must be
    /// treated as delivered (no retry). FCM has already accepted the message, so
    /// reporting a failed attempt makes the fanout worker requeue the job
    /// (PushFailed is retryable) and FCM delivers the same push a second time.
    /// Pre-fix the body-parse-error arm returned `(false, false)`, so this fails
    /// against the old behaviour and passes against the fix.
    #[tokio::test]
    async fn send_fcm_v1_treats_2xx_unparseable_body_as_delivered_not_retried() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path(
                "/v1/projects/test-project/messages:send",
            ))
            // 200 OK, but a body that is not valid JSON for FcmSendResponse.
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("OK"))
            .mount(&server)
            .await;

        let adapter = FcmHttpAdapter::with_token_provider(
            failing_pool(),
            configured_fcm_config(Some(server.uri())),
            FcmTokenProvider::Static("test-bearer".into()),
        );

        let (delivered, expired) = adapter
            .send_fcm_v1("test-project", "device-token-123", &fcm_test_notification())
            .await;

        assert!(
            delivered,
            "a 2xx FCM response with an unparseable body must be treated as delivered (no retry); \
             reporting a failed attempt makes the worker requeue and re-deliver the push"
        );
        assert!(
            !expired,
            "a 2xx unparseable body must not evict the device token"
        );
    }

    /// Boundary guard: a NON-2xx (e.g. 503) whose body we cannot parse stays a
    /// failed, retryable attempt — it must NOT be reported as delivered. Pins
    /// the fix as narrow (only 2xx is forgiven) so a future over-correction that
    /// treats every unparseable body as delivered regresses here.
    #[tokio::test]
    async fn send_fcm_v1_treats_non_2xx_unparseable_body_as_failed_attempt() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path(
                "/v1/projects/test-project/messages:send",
            ))
            .respond_with(
                wiremock::ResponseTemplate::new(503).set_body_string("upstream unavailable"),
            )
            .mount(&server)
            .await;

        let adapter = FcmHttpAdapter::with_token_provider(
            failing_pool(),
            configured_fcm_config(Some(server.uri())),
            FcmTokenProvider::Static("test-bearer".into()),
        );

        let (delivered, expired) = adapter
            .send_fcm_v1("test-project", "device-token-123", &fcm_test_notification())
            .await;

        assert!(
            !delivered,
            "a non-2xx unparseable response is a failed (retryable) attempt, not delivered"
        );
        assert!(!expired);
    }

    // ------------------------------------------------------------------
    // Push DB-error leak guard (#3731) — FCM standalone `send()`.
    //
    // #3731 fixed the raw-sqlx-error leak in apns.rs, combined.rs AND fcm.rs,
    // but only apns.rs and combined.rs received regression tests; the fcm.rs
    // fix shipped with no coverage. This closes that shadow-debt gap.
    // ------------------------------------------------------------------

    /// Security regression (code-review `push-db-err-leak`): FCM standalone
    /// `send()`'s device-token fetch failure must map to a FIXED, non-leaking
    /// message — never the raw `sqlx::Error` display, which can expose the DB
    /// host/port/credentials from the connection string (that message is
    /// persisted verbatim to the delivery record via `into_failed(e.to_string())`).
    /// Pre-fix returned `format!("DB error fetching tokens: {e}")`, so this fails
    /// against the old behaviour and passes against the fix. The raw error stays
    /// in the internal `tracing::error!` log only.
    #[tokio::test]
    async fn fcm_db_fetch_error_message_does_not_leak_raw_db_error() {
        let adapter = FcmHttpAdapter::with_token_provider(
            failing_pool(),
            configured_fcm_config(None),
            FcmTokenProvider::Static("test-bearer".into()),
        );

        let result = adapter.send(Uuid::new_v4(), &fcm_test_notification()).await;

        match result {
            Err(NotificationError::PushFailed(msg)) => {
                assert_eq!(
                    msg, "failed to fetch device tokens",
                    "DB fetch failure must map to a fixed non-leaking message, got {msg:?}"
                );
            }
            other => panic!("expected PushFailed(non-leaking message), got {other:?}"),
        }
    }
}
