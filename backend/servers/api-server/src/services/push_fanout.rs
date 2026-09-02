//! Epic 8A-3: Push notification fanout worker.
//!
//! # What this module provides
//!
//! 1. `FcmHttpAdapter` — a real `PushTransport` implementation that:
//!    - Looks up all registered device tokens for a user via `DevicePushTokenRepository`.
//!    - Calls the FCM HTTP v1 API (`https://fcm.googleapis.com/v1/projects/{project}/messages:send`)
//!      using a bearer service-account JWT or a legacy server key.
//!    - Handles `NOT_REGISTERED` / `INVALID_REGISTRATION` error codes from FCM by
//!      deleting the stale token from the DB.
//!
//! 2. `ApnsHttpAdapter` — a `PushTransport` implementation for Apple Push Notification service:
//!    - Authenticates using a P8 ECDSA key (ES256 JWT, refreshed every 50 minutes).
//!    - Calls the APNs HTTP/2 provider API (`https://api.push.apple.com:443/3/device/{token}`).
//!    - Handles `BadDeviceToken` / `Unregistered` error codes from APNs by
//!      deleting the stale token from the DB.
//!    - Configurable base URL override for testing (no live APNs in unit tests).
//!
//! 3. `CombinedPushAdapter` — routes delivery to the correct provider by platform:
//!    - FCM tokens → `FcmHttpAdapter`
//!    - APNs tokens → `ApnsHttpAdapter`
//!    - Configured as the shared adapter inside `PushFanoutWorker`.
//!
//! 4. `PushFanoutWorker` — a lightweight background tokio task that:
//!    - Polls a Redis list (`push_fanout_queue`) for pending push jobs published by
//!      the notification pipeline.
//!    - Falls back to a no-op polling loop when Redis / FCM are not configured, so
//!      the server always starts cleanly.
//!
//! # Configuration (environment variables)
//!
//! | Variable                     | Required | Description                                                        |
//! |------------------------------|----------|--------------------------------------------------------------------|
//! | `FCM_PROJECT_ID`             | No       | GCP project ID for FCM HTTP v1 (`projects/{id}/…`) — required to enable Android push |
//! | `FCM_SERVICE_ACCOUNT_JSON`   | No       | Inline Google service-account JSON. **Preferred** for HTTP v1: the adapter mints and auto-refreshes an OAuth2 access token before Google's ~1h TTL expiry. |
//! | `GOOGLE_APPLICATION_CREDENTIALS` | No   | Path to a Google service-account JSON file (same effect as `FCM_SERVICE_ACCOUNT_JSON`, used when the inline var is unset). |
//! | `FCM_OAUTH_TOKEN`           | No       | **Legacy / deprecated.** A pre-minted OAuth2 bearer token, read once at startup and never refreshed — Android push stops working after Google's ~1h token TTL. Use `FCM_SERVICE_ACCOUNT_JSON` instead; only used as a fall-back when no service account is configured. |
//! | `APNS_P8_KEY`               | No       | PEM-encoded P8 ECDSA private key for APNs provider auth             |
//! | `APNS_KEY_ID`               | No       | 10-char Key ID printed on the P8 file in App Store Connect          |
//! | `APNS_TEAM_ID`              | No       | 10-char Apple Team ID                                               |
//! | `APNS_TOPIC`                | No       | APNs topic / bundle ID (default: `three.two.bit.ppt.management`)    |
//! | `PUSH_FANOUT_ENABLED`       | No       | Set to `false` / `0` to disable the worker                         |
//! | `PUSH_FANOUT_POLL_SECS`     | No       | Polling interval in seconds (default: 30)                           |
//!
//! If neither FCM nor APNs is configured the worker logs a warning and becomes a
//! no-op loop — the server will not crash.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use db::{
    models::{device_push_token::PushPlatform, DevicePushToken},
    repositories::DevicePushTokenRepository,
    DbPool,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::time::interval;
use tracing::Instrument;
use uuid::Uuid;

use common::notifications::{Notification, NotificationError, PushTransport, TransportResult};

// APNs JWT auth uses ES256; jsonwebtoken provides this via rust_crypto feature.
use jsonwebtoken::{Algorithm, EncodingKey, Header};

// ============================================================================
// Dispatch-target selection (preference-aware)
// ============================================================================

/// A single device that has been selected as a push dispatch target.
///
/// Produced by [`select_dispatch_targets`] from the raw rows stored in
/// `device_push_tokens`. Keeping selection in a small pure struct (rather than
/// passing whole [`DevicePushToken`] rows around the delivery loop) makes the
/// "which devices do we actually send to" decision unit-testable without a DB
/// or a live FCM/APNs gateway.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushTarget {
    /// The `device_push_tokens.id` of the selected row (used for receipts /
    /// stale-token eviction).
    pub token_id: Uuid,
    /// The raw FCM registration token / APNs device token.
    pub token: String,
    /// Which OS gateway this target is delivered through.
    pub platform: PushPlatform,
    /// Bundle / package id the token was registered under, if any.
    pub app_id: Option<String>,
}

/// Filter controlling which stored device tokens become dispatch targets.
///
/// This is the device-token-level companion to the channel-level
/// `PreferenceRouter` in `notification_pipeline.rs`. The pipeline decides
/// *whether* the push channel is enabled for a user at all; this filter then
/// decides *which of the user's registered devices* should receive the push.
///
/// All fields default to "no restriction" so an empty filter selects every
/// non-empty token the user has registered.
#[derive(Debug, Clone, Default)]
pub struct PushTargetFilter {
    /// When set, only tokens registered under this bundle / package id are
    /// selected. Lets a notification originating from the Property-Management
    /// app avoid waking the Reality-Portal binary on the same device (and vice
    /// versa) when both are installed.
    pub app_id: Option<String>,
    /// When set, only tokens for these platforms are selected. `None` selects
    /// both FCM and APNs.
    pub platforms: Option<Vec<PushPlatform>>,
}

impl PushTargetFilter {
    /// Returns `true` when the given stored token row passes the filter.
    fn accepts(&self, token: &db::models::DevicePushToken) -> bool {
        // Never dispatch to an empty/blank token — it can only ever be rejected
        // by the gateway and would waste a request + risk a spurious stale-token
        // eviction.
        if token.token.trim().is_empty() {
            return false;
        }

        if let Some(ref want_app) = self.app_id {
            match token.app_id.as_deref() {
                Some(app) if app == want_app => {}
                _ => return false,
            }
        }

        if let Some(ref platforms) = self.platforms {
            if !platforms.contains(&token.push_platform()) {
                return false;
            }
        }

        true
    }
}

/// Select the set of dispatch targets from a user's stored device tokens.
///
/// Pure function (no I/O): the caller fetches the rows, this decides which ones
/// are valid push targets given `filter`. Blank tokens are always dropped.
///
/// The result preserves the input ordering so callers that fetched tokens
/// `ORDER BY last_seen_at DESC` keep "most-recently-seen device first".
pub fn select_dispatch_targets(
    tokens: &[db::models::DevicePushToken],
    filter: &PushTargetFilter,
) -> Vec<PushTarget> {
    tokens
        .iter()
        .filter(|t| filter.accepts(t))
        .map(|t| PushTarget {
            token_id: t.id,
            token: t.token.clone(),
            platform: t.push_platform(),
            app_id: t.app_id.clone(),
        })
        .collect()
}

// ============================================================================
// FCM delivery receipt (stored in memory for now; DB table in follow-up)
// ============================================================================

/// Outcome of a single FCM/APNs delivery attempt for one device token.
#[derive(Debug, Clone)]
pub struct PushDeliveryReceipt {
    pub token_id: Uuid,
    pub user_id: Uuid,
    pub platform: PushPlatform,
    /// Whether the message was accepted by the upstream gateway.
    pub success: bool,
    /// Error string set when `success == false`.
    pub error: Option<String>,
    /// Stale token: the upstream gateway signalled the token is no longer valid.
    pub token_expired: bool,
    pub attempted_at: chrono::DateTime<chrono::Utc>,
}

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
    /// Machine-readable status string (e.g. `NOT_REGISTERED`, `INVALID_ARGUMENT`).
    status: Option<String>,
    #[allow(dead_code)]
    message: Option<String>,
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

/// A service-account-backed source of auto-refreshed FCM access tokens.
///
/// Holds the current token behind an async `Mutex`; the first caller past the
/// refresh boundary re-mints while others await, so at most one token exchange
/// is in flight at a time.
struct ServiceAccountTokenSource {
    service_account: FcmServiceAccount,
    http: reqwest::Client,
    cache: Mutex<Option<CachedAccessToken>>,
}

impl ServiceAccountTokenSource {
    fn new(service_account: FcmServiceAccount) -> Self {
        Self {
            service_account,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("reqwest client build should not fail"),
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
        let assertion = self.mint_assertion(now)?;
        let resp = self
            .http
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
    fcm_config: FcmConfig,
    http: reqwest::Client,
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
        Self {
            token_repo: DevicePushTokenRepository::new(pool),
            fcm_config,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("reqwest client build should not fail"),
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

        let resp = match self
            .http
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
                if status.is_success() && fcm_resp.error.is_none() {
                    (true, false)
                } else {
                    let err_status = fcm_resp
                        .error
                        .as_ref()
                        .and_then(|e| e.status.as_deref())
                        .unwrap_or("UNKNOWN");
                    let expired = matches!(
                        err_status,
                        "NOT_REGISTERED" | "UNREGISTERED" | "INVALID_REGISTRATION"
                    );
                    tracing::warn!(
                        fcm_status = %err_status,
                        token_expired = expired,
                        "[8A-3] FCM rejected message"
                    );
                    (false, expired)
                }
            }
            Err(e) => {
                tracing::warn!(
                    http_status = %status,
                    error = %e,
                    "[8A-3] Failed to parse FCM HTTP v1 response"
                );
                (false, false)
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
    async fn deliver(
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
                return Err(NotificationError::PushFailed(format!(
                    "DB error fetching tokens: {e}"
                )));
            }
        };
        self.deliver(user_id, &tokens, notification)
            .await
            .into_result()
    }
}

// ---------------------------------------------------------------------------
// Internal helper: delete a stale token by its string value using service-role pool
// ---------------------------------------------------------------------------

/// Delete a specific stale push token by its string value via the service-role pool.
///
/// Uses `DevicePushTokenRepository::delete_stale_token` which issues a targeted
/// `DELETE … WHERE user_id = $1 AND token = $2` so it cannot accidentally evict
/// tokens belonging to other users.
async fn delete_stale_token_by_string(
    repo: &DevicePushTokenRepository,
    user_id: Uuid,
    token: &str,
) -> Result<(), String> {
    repo.delete_stale_token(user_id, token)
        .await
        .map(|_deleted| ())
        .map_err(|e| format!("DB error deleting stale token: {e}"))
}

// ============================================================================
// Story 8A-3: APNs provider config and HTTP/2 adapter
// ============================================================================

/// Runtime configuration for the APNs transport.
///
/// APNs uses provider-auth JWTs signed with an ES256 P8 key (issued in
/// App Store Connect). A new JWT is generated every 50 minutes so it stays
/// well inside Apple's 1-hour token lifetime.
#[derive(Clone, Debug)]
pub struct ApnsConfig {
    /// PEM-encoded PKCS#8 EC private key (the contents of the `.p8` file).
    /// When `None` the adapter is disabled and delivery is skipped.
    pub p8_key_pem: Option<String>,
    /// 10-char Key ID printed on the downloaded `.p8` file.
    pub key_id: Option<String>,
    /// 10-char Apple Developer Team ID.
    pub team_id: Option<String>,
    /// APNs topic — normally the app's bundle ID.
    /// Default: `three.two.bit.ppt.management`.
    pub topic: String,
    /// Base URL override for tests (production: `https://api.push.apple.com`).
    pub apns_base_url: Option<String>,
}

impl ApnsConfig {
    /// Load from environment variables.
    pub fn from_env() -> Self {
        Self {
            p8_key_pem: std::env::var("APNS_P8_KEY").ok(),
            key_id: std::env::var("APNS_KEY_ID").ok(),
            team_id: std::env::var("APNS_TEAM_ID").ok(),
            topic: std::env::var("APNS_TOPIC")
                .unwrap_or_else(|_| "three.two.bit.ppt.management".to_string()),
            apns_base_url: None,
        }
    }

    /// Return `true` when all required credentials are present.
    pub fn is_configured(&self) -> bool {
        self.p8_key_pem.is_some() && self.key_id.is_some() && self.team_id.is_some()
    }

    /// Return the effective APNs base URL.
    pub fn base_url(&self) -> &str {
        self.apns_base_url
            .as_deref()
            .unwrap_or("https://api.push.apple.com")
    }
}

/// APNs JWT claims (`iss` = Team ID, `iat` = issued-at, signed with P8 ES256 key).
#[derive(Debug, serde::Serialize)]
struct ApnsJwtClaims {
    iss: String,
    iat: i64,
}

/// Mint a short-lived APNs provider JWT using ES256 and the P8 key.
///
/// Apple requires the JWT to be refreshed at least once per hour; we mint a
/// fresh token on every fanout call (negligible overhead vs. the HTTP round-trip).
fn mint_apns_jwt(config: &ApnsConfig) -> Option<String> {
    let key_pem = config.p8_key_pem.as_deref()?;
    let key_id = config.key_id.as_deref()?;
    let team_id = config.team_id.as_deref()?;

    let encoding_key = match EncodingKey::from_ec_pem(key_pem.as_bytes()) {
        Ok(k) => k,
        Err(e) => {
            tracing::error!(
                error = %e,
                "[8A-3] APNs: failed to parse APNS_P8_KEY as EC PEM — check the key format"
            );
            return None;
        }
    };

    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some(key_id.to_string());

    let claims = ApnsJwtClaims {
        iss: team_id.to_string(),
        iat: chrono::Utc::now().timestamp(),
    };

    match jsonwebtoken::encode(&header, &claims, &encoding_key) {
        Ok(token) => Some(token),
        Err(e) => {
            tracing::error!(
                error = %e,
                "[8A-3] APNs: failed to sign APNs JWT"
            );
            None
        }
    }
}

/// APNs error response body.
#[derive(Debug, Deserialize)]
struct ApnsErrorBody {
    reason: Option<String>,
}

/// A `PushTransport` implementation that delivers messages via APNs HTTP/2.
///
/// The adapter:
/// - Mints a provider-auth JWT (ES256, signed with the P8 key) on each send call.
/// - Posts to `https://api.push.apple.com/3/device/{token}` over HTTP/2.
/// - Handles `BadDeviceToken` / `Unregistered` by deleting the stale token.
/// - Is silently no-op when APNs credentials are not configured.
///
/// # Testing
///
/// Set `ApnsConfig::apns_base_url` to a local HTTP/1.1 test server — the adapter
/// uses the same `reqwest::Client` for both production and tests, so a wiremock
/// or `axum` test server can stand in for the real APNs gateway.
#[derive(Clone)]
pub struct ApnsHttpAdapter {
    token_repo: DevicePushTokenRepository,
    apns_config: ApnsConfig,
    http: reqwest::Client,
}

impl ApnsHttpAdapter {
    /// Create a new adapter backed by the given pool and APNs config.
    pub fn new(pool: DbPool, apns_config: ApnsConfig) -> Self {
        Self {
            token_repo: DevicePushTokenRepository::new(pool),
            apns_config,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                // APNs requires HTTP/2; enabling http2_prior_knowledge avoids the
                // TLS ALPN upgrade round-trip to the known-H2 gateway.
                .http2_prior_knowledge()
                .build()
                .expect("reqwest APNs client build should not fail"),
        }
    }

    /// Load APNs config from environment and wire the adapter.
    pub fn from_env(pool: DbPool) -> Self {
        Self::new(pool, ApnsConfig::from_env())
    }

    // ------------------------------------------------------------------
    // Internal: send one APNs notification to one device token
    // ------------------------------------------------------------------

    /// Send a single push notification to one APNs device token.
    ///
    /// Returns `(success, token_expired)`. When `token_expired` is `true` the
    /// caller must delete the token from the DB.
    async fn send_apns_one(
        &self,
        jwt: &str,
        device_token: &str,
        notification: &Notification,
    ) -> (bool, bool) {
        let url = format!("{}/3/device/{device_token}", self.apns_config.base_url());

        // APNs JSON payload — aps dictionary with alert sub-dictionary.
        let body = serde_json::json!({
            "aps": {
                "alert": {
                    "title": notification.title,
                    "body": notification.body,
                },
                "sound": "default",
            },
            // Custom data available to the notification service extension.
            "notification_id": notification.id.to_string(),
            "category": notification.category.as_str(),
            "priority": notification.priority.as_str(),
        });

        let resp = match self
            .http
            .post(&url)
            .bearer_auth(jwt)
            .header("apns-topic", &self.apns_config.topic)
            // Normal push: priority 10. Use 5 for background delivery.
            .header("apns-priority", "10")
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "[8A-3] APNs HTTP request failed (network error)"
                );
                return (false, false);
            }
        };

        let status = resp.status();

        if status.as_u16() == 200 {
            return (true, false);
        }

        // Non-200 → parse the APNs error body to decide on stale-token eviction.
        match resp.json::<ApnsErrorBody>().await {
            Ok(err_body) => {
                let reason = err_body.reason.as_deref().unwrap_or("UNKNOWN");
                let expired = matches!(reason, "BadDeviceToken" | "Unregistered");
                tracing::warn!(
                    apns_reason = %reason,
                    http_status = %status,
                    token_expired = expired,
                    "[8A-3] APNs rejected notification"
                );
                (false, expired)
            }
            Err(e) => {
                tracing::warn!(
                    http_status = %status,
                    error = %e,
                    "[8A-3] Failed to parse APNs error body"
                );
                (false, false)
            }
        }
    }
}

impl ApnsHttpAdapter {
    /// Deliver to the APNs (iOS) slice of a *pre-fetched* device-token set.
    ///
    /// Mirrors [`FcmHttpAdapter::deliver`]: the caller supplies the already
    /// fetched tokens, and the richer [`ProviderOutcome`] lets the combined
    /// adapter distinguish a real delivery from "no APNs targets".
    async fn deliver(
        &self,
        user_id: Uuid,
        tokens: &[DevicePushToken],
        notification: &Notification,
    ) -> ProviderOutcome {
        if !self.apns_config.is_configured() {
            tracing::info!(
                user_id = %user_id,
                "[8A-3] APNs push skipped — APNS_P8_KEY / APNS_KEY_ID / APNS_TEAM_ID not set"
            );
            return ProviderOutcome::NotConfigured;
        }

        // Select only APNs targets before minting a JWT, so an Android-only user
        // (no APNs tokens) short-circuits without the ES256 signing work.
        let filter = PushTargetFilter {
            platforms: Some(vec![PushPlatform::Apns]),
            ..Default::default()
        };
        let targets = select_dispatch_targets(tokens, &filter);

        if targets.is_empty() {
            tracing::debug!(
                user_id = %user_id,
                "[8A-3] APNs: no APNs targets for user — nothing to deliver"
            );
            return ProviderOutcome::NoTargets;
        }

        // Mint a fresh JWT for this batch (negligible overhead vs. HTTP round-trip).
        let jwt = match mint_apns_jwt(&self.apns_config) {
            Some(t) => t,
            None => {
                return ProviderOutcome::Failed(NotificationError::PushFailed(
                    "failed to mint APNs JWT".to_string(),
                ))
            }
        };

        let mut any_sent = false;
        let mut stale_tokens: Vec<(Uuid, String)> = Vec::new();

        for target in &targets {
            let (success, expired) = self.send_apns_one(&jwt, &target.token, notification).await;

            let receipt = PushDeliveryReceipt {
                token_id: target.token_id,
                user_id,
                platform: PushPlatform::Apns,
                success,
                error: if success {
                    None
                } else {
                    Some("APNs rejected".to_string())
                },
                token_expired: expired,
                attempted_at: chrono::Utc::now(),
            };
            tracing::info!(
                token_id = %receipt.token_id,
                user_id = %receipt.user_id,
                success = receipt.success,
                token_expired = receipt.token_expired,
                "[8A-3] APNs delivery receipt"
            );

            if expired {
                stale_tokens.push((target.token_id, target.token.clone()));
            }
            if success {
                any_sent = true;
            }
        }

        // Purge stale tokens reported by APNs.
        for (stale_id, stale_token) in stale_tokens {
            if let Err(e) =
                delete_stale_token_by_string(&self.token_repo, user_id, &stale_token).await
            {
                tracing::warn!(
                    error = %e,
                    token_id = %stale_id,
                    "[8A-3] APNs: failed to evict stale token"
                );
            } else {
                tracing::info!(
                    token_id = %stale_id,
                    "[8A-3] APNs: stale token evicted (BadDeviceToken/Unregistered)"
                );
            }
        }

        if any_sent {
            ProviderOutcome::Delivered
        } else {
            ProviderOutcome::Failed(NotificationError::PushFailed(
                "all APNs delivery attempts failed".to_string(),
            ))
        }
    }
}

#[async_trait]
impl PushTransport for ApnsHttpAdapter {
    async fn send(&self, user_id: Uuid, notification: &Notification) -> TransportResult {
        if !self.apns_config.is_configured() {
            return ProviderOutcome::NotConfigured.into_result();
        }
        let tokens = match self.token_repo.get_tokens_for_user(user_id).await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(
                    user_id = %user_id,
                    error = %e,
                    "[8A-3] APNs: failed to fetch device tokens"
                );
                return Err(NotificationError::PushFailed(format!(
                    "DB error fetching APNs tokens: {e}"
                )));
            }
        };
        self.deliver(user_id, &tokens, notification)
            .await
            .into_result()
    }
}

// ============================================================================
// CombinedPushAdapter — routes FCM and APNs by platform in one pass
// ============================================================================

/// Internal, richer-than-`Result` outcome of one provider's delivery pass.
///
/// The `PushTransport` trait can only return `Ok(())` / `Err(_)`, which
/// conflates "I actually delivered a push" with "I was configured but had no
/// tokens for my platform". [`CombinedPushAdapter`] needs that distinction to
/// keep delivery metrics honest: if *neither* provider performed a real
/// delivery, the pipeline must record the push as **Skipped**, not **Sent**
/// (the inflated-`sent` lie Issue #484 removed — it re-surfaced for iOS-only
/// users when FCM was configured but APNs was not).
enum ProviderOutcome {
    /// At least one device accepted the push.
    Delivered,
    /// Provider is configured, but the user has no tokens for this platform —
    /// nothing was delivered and nothing failed.
    NoTargets,
    /// Provider credentials are not configured; it attempted nothing.
    NotConfigured,
    /// Provider attempted delivery and every attempt failed.
    Failed(NotificationError),
}

impl ProviderOutcome {
    /// Collapse to the `PushTransport` trait's `Result`, used when a single
    /// adapter is driven directly (not through [`CombinedPushAdapter`]).
    /// `NoTargets` maps to `Ok(())` — a configured provider with no devices for
    /// its platform is not an error on the single-provider path.
    fn into_result(self) -> TransportResult {
        match self {
            ProviderOutcome::Delivered | ProviderOutcome::NoTargets => Ok(()),
            ProviderOutcome::NotConfigured => Err(NotificationError::PushNotConfigured),
            ProviderOutcome::Failed(e) => Err(e),
        }
    }
}

/// A combined push adapter that routes delivery to the correct provider by platform.
///
/// - FCM tokens (platform = `fcm`) are delivered via `FcmHttpAdapter`.
/// - APNs tokens (platform = `apns`) are delivered via `ApnsHttpAdapter`.
///
/// The user's device tokens are fetched **once** here and the shared slice is
/// handed to each provider (which selects its own platform), so a fanout over
/// N users costs N token queries, not 2N.
#[derive(Clone)]
pub struct CombinedPushAdapter {
    fcm: FcmHttpAdapter,
    apns: ApnsHttpAdapter,
    /// The combined adapter owns the single token fetch (see [`CombinedPushAdapter::send`]).
    /// Kept as its own field rather than reaching into `self.fcm.token_repo`, so the
    /// "fetch once" responsibility is expressed on the adapter that owns it and does not
    /// break if `FcmHttpAdapter` is later extracted to another module.
    token_repo: DevicePushTokenRepository,
}

impl CombinedPushAdapter {
    /// Create a combined adapter from the shared pool.
    pub fn from_env(pool: DbPool) -> Self {
        Self {
            fcm: FcmHttpAdapter::from_env(pool.clone()),
            apns: ApnsHttpAdapter::from_env(pool.clone()),
            token_repo: DevicePushTokenRepository::new(pool),
        }
    }

    /// Create a combined adapter with explicit configs (useful in tests).
    pub fn new(pool: DbPool, fcm_config: FcmConfig, apns_config: ApnsConfig) -> Self {
        Self {
            fcm: FcmHttpAdapter::new(pool.clone(), fcm_config),
            apns: ApnsHttpAdapter::new(pool.clone(), apns_config),
            token_repo: DevicePushTokenRepository::new(pool),
        }
    }

    /// Route a *pre-fetched* device-token set to both providers concurrently and
    /// combine their outcomes into a single pipeline result.
    ///
    /// Split out from [`CombinedPushAdapter::send`] (which owns the one DB fetch)
    /// so the routing + honest-metrics decision is unit-testable without a live
    /// DB or gateway: an unconfigured provider and a provider with no targets for
    /// its platform both short-circuit before any network I/O.
    async fn deliver_with_tokens(
        &self,
        user_id: Uuid,
        tokens: &[DevicePushToken],
        notification: &Notification,
    ) -> TransportResult {
        // Run FCM and APNs delivery concurrently off the shared token slice.
        let (fcm_outcome, apns_outcome) = tokio::join!(
            self.fcm.deliver(user_id, tokens, notification),
            self.apns.deliver(user_id, tokens, notification),
        );
        Self::combine(user_id, fcm_outcome, apns_outcome)
    }

    /// Combine the two providers' outcomes into the pipeline result.
    ///
    /// Honest-metrics contract:
    /// - **any** provider `Delivered` → `Ok(())` (Sent); a failure on the other
    ///   side is logged but does not suppress the successful delivery.
    /// - no delivery but **some** provider `Failed` → `Err(PushFailed)` (Failed).
    /// - no delivery and no failure (every provider `NoTargets` / `NotConfigured`)
    ///   → `Err(PushNotConfigured)` so the pipeline records **Skipped**, not Sent.
    fn combine(user_id: Uuid, fcm: ProviderOutcome, apns: ProviderOutcome) -> TransportResult {
        let delivered =
            matches!(fcm, ProviderOutcome::Delivered) || matches!(apns, ProviderOutcome::Delivered);

        let mut failures: Vec<String> = Vec::new();
        if let ProviderOutcome::Failed(e) = &fcm {
            failures.push(format!("FCM: {e}"));
        }
        if let ProviderOutcome::Failed(e) = &apns {
            failures.push(format!("APNs: {e}"));
        }

        if delivered {
            if !failures.is_empty() {
                tracing::warn!(
                    user_id = %user_id,
                    errors = %failures.join("; "),
                    "[8A-3] partial push delivery — one provider delivered, another failed"
                );
            }
            return Ok(());
        }

        if failures.is_empty() {
            // Nothing delivered and nothing failed: every provider was either
            // unconfigured or had no tokens for its platform. Record Skipped
            // (honest) rather than Sent (the iOS-only inflated-`sent` lie).
            tracing::info!(
                user_id = %user_id,
                "[8A-3] push skipped — no configured provider had a deliverable target"
            );
            Err(NotificationError::PushNotConfigured)
        } else {
            let msg = failures.join("; ");
            tracing::warn!(
                user_id = %user_id,
                error = %msg,
                "[8A-3] push delivery failed on all attempted providers"
            );
            Err(NotificationError::PushFailed(msg))
        }
    }
}

#[async_trait]
impl PushTransport for CombinedPushAdapter {
    async fn send(&self, user_id: Uuid, notification: &Notification) -> TransportResult {
        // Fetch the user's device tokens ONCE. Previously `FcmHttpAdapter` and
        // `ApnsHttpAdapter` each ran `get_tokens_for_user` independently — 2N
        // identical queries on the `dispatch_to_users` fanout hot path. Fetch
        // here and hand both providers the shared slice.
        let tokens = match self.token_repo.get_tokens_for_user(user_id).await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(
                    user_id = %user_id,
                    error = %e,
                    "[8A-3] Combined: failed to fetch device tokens for push delivery"
                );
                return Err(NotificationError::PushFailed(format!(
                    "DB error fetching tokens: {e}"
                )));
            }
        };
        self.deliver_with_tokens(user_id, &tokens, notification)
            .await
    }
}

// ============================================================================
// PushFanoutWorker — background tokio task
// ============================================================================

/// Configuration for the push fanout background worker.
#[derive(Clone, Debug)]
pub struct PushFanoutConfig {
    /// Whether the worker should start (default: `true` unless `PUSH_FANOUT_ENABLED=false`).
    pub enabled: bool,
    /// How often the worker polls for pending push jobs (seconds, default: 30).
    pub poll_interval_secs: u64,
}

impl Default for PushFanoutConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            poll_interval_secs: 30,
        }
    }
}

impl PushFanoutConfig {
    /// Load from environment variables.
    pub fn from_env() -> Self {
        let enabled = std::env::var("PUSH_FANOUT_ENABLED")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(true);
        let poll_interval_secs = std::env::var("PUSH_FANOUT_POLL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);
        Self {
            enabled,
            poll_interval_secs,
        }
    }
}

/// Redis list key the notification pipeline enqueues push jobs onto.
///
/// Producers `RPUSH` a JSON-serialized [`Notification`] onto this list; the
/// worker `BLPOP`s from the head and delivers each one via [`CombinedPushAdapter`].
pub const PUSH_FANOUT_QUEUE_KEY: &str = "push_fanout_queue";

/// BLPOP block timeout (seconds) used when draining the queue.
///
/// Kept short so the single multiplexed `ConnectionManager` connection is not
/// held for long and other Redis commands are not starved. When the queue is
/// empty, BLPOP returns after this timeout and the drain yields back to the
/// poll ticker.
const BLPOP_TIMEOUT_SECS: f64 = 2.0;

/// Safety cap on the number of jobs drained per poll tick, so a flooded queue
/// can't make a single tick run unbounded.
const MAX_JOBS_PER_TICK: usize = 256;

/// Background worker that fans out pending push notifications.
///
/// ## Delivery model
///
/// The worker drains a Redis list ([`PUSH_FANOUT_QUEUE_KEY`]) for pending job
/// payloads enqueued by the notification pipeline.  Each payload is a
/// JSON-serialized [`Notification`]:
/// ```json
/// {
///   "id": "<uuid>",
///   "user_id": "<uuid>",
///   "category": "announcements",
///   "title": "...",
///   "body": "...",
///   "priority": "normal"
/// }
/// ```
///
/// On each tick the worker `BLPOP`s from the head of the list (short timeout)
/// and delivers each job via [`CombinedPushAdapter::send`] (one token fetch,
/// platform-routed FCM + APNs), up to [`MAX_JOBS_PER_TICK`] jobs per tick.
///
/// When Redis is not available (or the worker is disabled) the worker falls
/// through to a no-op heartbeat loop so the server always starts cleanly — the
/// in-process pipeline still delivers push synchronously in that mode.
pub struct PushFanoutWorker {
    adapter: Arc<CombinedPushAdapter>,
    config: PushFanoutConfig,
    pubsub: Option<integrations::PubSubService>,
}

impl PushFanoutWorker {
    /// Create a new worker from the shared pool and current environment.
    ///
    /// Reads FCM and APNs credentials from environment variables. If neither
    /// is configured the worker starts in heartbeat-only mode (no crash).
    pub fn new(
        pool: DbPool,
        pubsub: Option<integrations::PubSubService>,
        config: PushFanoutConfig,
    ) -> Self {
        let fcm_config = FcmConfig::from_env();
        let apns_config = ApnsConfig::from_env();

        if !fcm_config.is_configured() {
            tracing::warn!("[8A-3] PushFanoutWorker: FCM not configured (FCM_PROJECT_ID unset)");
        }
        if !apns_config.is_configured() {
            tracing::warn!(
                "[8A-3] PushFanoutWorker: APNs not configured (APNS_P8_KEY / APNS_KEY_ID / APNS_TEAM_ID unset)"
            );
        }
        if !fcm_config.is_configured() && !apns_config.is_configured() {
            tracing::warn!(
                "[8A-3] PushFanoutWorker: neither FCM nor APNs is configured; \
                 push delivery will be skipped (no crash — set env vars to enable)"
            );
        }

        Self {
            adapter: Arc::new(CombinedPushAdapter::new(pool, fcm_config, apns_config)),
            config,
            pubsub,
        }
    }

    /// Spawn the background task and return its `JoinHandle`.
    pub fn start(self) -> tokio::task::JoinHandle<()> {
        let poll_secs = self.config.poll_interval_secs;
        tokio::spawn(
            async move {
                if !self.config.enabled {
                    tracing::info!("[8A-3] PushFanoutWorker disabled — not starting");
                    return;
                }

                tracing::info!(
                    poll_interval_secs = self.config.poll_interval_secs,
                    fcm_configured = self.adapter.fcm.fcm_config.is_configured(),
                    apns_configured = self.adapter.apns.apns_config.is_configured(),
                    "[8A-3] PushFanoutWorker started"
                );

                let mut ticker = interval(Duration::from_secs(self.config.poll_interval_secs));

                loop {
                    ticker.tick().await;
                    self.process_pending_jobs().await;
                }
            }
            .instrument(tracing::info_span!("bg.push_fanout", poll_secs = poll_secs,)),
        )
    }

    /// Process pending push jobs from the queue.
    ///
    /// When Redis is available we `BLPOP` jobs off [`PUSH_FANOUT_QUEUE_KEY`] and
    /// deliver each one via [`CombinedPushAdapter::send`] (one token fetch,
    /// platform-routed FCM + APNs), up to [`MAX_JOBS_PER_TICK`] jobs per tick.
    /// When Redis is not available we just
    /// log a heartbeat (the in-process pipeline delivers push synchronously).
    async fn process_pending_jobs(&self) {
        let Some(ref pubsub) = self.pubsub else {
            // No Redis — nothing to drain; log at trace to avoid spam
            tracing::trace!(
                "[8A-3] PushFanoutWorker heartbeat (no Redis; in-process pipeline handles push)"
            );
            return;
        };

        let redis = pubsub.client();
        let mut drained = 0usize;

        // Drain up to MAX_JOBS_PER_TICK jobs. BLPOP with a short timeout means
        // an empty queue ends the drain promptly and yields back to the ticker;
        // a backed-up queue is bounded by MAX_JOBS_PER_TICK per tick.
        for _ in 0..MAX_JOBS_PER_TICK {
            let payload = match redis
                .queue_pop_blocking(PUSH_FANOUT_QUEUE_KEY, BLPOP_TIMEOUT_SECS)
                .await
            {
                Ok(Some(payload)) => payload,
                // Timed out with an empty queue — nothing more to do this tick.
                Ok(None) => break,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "[8A-3] PushFanoutWorker: BLPOP on push_fanout_queue failed; backing off until next tick"
                    );
                    break;
                }
            };

            self.deliver_job(&payload).await;
            drained += 1;
        }

        if drained > 0 {
            tracing::info!(
                drained = drained,
                "[8A-3] PushFanoutWorker drained push_fanout_queue"
            );
        }
    }

    /// Deliver a single queued job payload.
    ///
    /// The payload is a JSON-serialized [`Notification`]. A malformed payload is
    /// logged and dropped (it has already been popped off the list, so it is not
    /// retried — re-queuing a poison message would loop forever).
    async fn deliver_job(&self, payload: &str) {
        let notification: Notification = match serde_json::from_str(payload) {
            Ok(n) => n,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "[8A-3] PushFanoutWorker: dropping malformed push_fanout_queue payload"
                );
                return;
            }
        };

        match self.adapter.send(notification.user_id, &notification).await {
            Ok(()) => {
                tracing::debug!(
                    notification_id = %notification.id,
                    user_id = %notification.user_id,
                    "[8A-3] PushFanoutWorker delivered queued push job"
                );
            }
            Err(NotificationError::PushNotConfigured) => {
                // Not a failure: either no provider is configured, or the user
                // has no deliverable device tokens (`combine()` maps both to
                // `PushNotConfigured`, which the synchronous pipeline records as
                // Skipped). Mirror that here — a benign no-device user must not
                // emit warn-level "delivery failed" noise or trip warn-based
                // alerting.
                tracing::debug!(
                    notification_id = %notification.id,
                    user_id = %notification.user_id,
                    "[8A-3] PushFanoutWorker: push skipped — no configured provider had a deliverable target"
                );
            }
            Err(e) => {
                // Genuine failure (DB error or all provider sends failed). The
                // in-app channel remains the mandatory record, so we log and move
                // on rather than re-queue (avoids hot-looping on a permanently
                // failing job).
                tracing::warn!(
                    notification_id = %notification.id,
                    user_id = %notification.user_id,
                    error = %e,
                    "[8A-3] PushFanoutWorker: push delivery failed for queued job"
                );
            }
        }
    }
}

// ============================================================================
// Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use db::models::DevicePushToken;

    // ------------------------------------------------------------------
    // Test helpers for dispatch-target selection
    // ------------------------------------------------------------------

    /// Build a stored `DevicePushToken` row for selection tests.
    fn stored_token(token: &str, platform: &str, app_id: Option<&str>) -> DevicePushToken {
        DevicePushToken {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            token: token.to_string(),
            platform: platform.to_string(),
            app_id: app_id.map(|s| s.to_string()),
            device_name: None,
            last_seen_at: Utc::now(),
            created_at: Utc::now(),
        }
    }

    // ------------------------------------------------------------------
    // Token storage: row -> typed platform / response mapping
    // ------------------------------------------------------------------

    #[test]
    fn stored_token_maps_platform_string_to_enum() {
        assert_eq!(
            stored_token("t", "fcm", None).push_platform(),
            PushPlatform::Fcm
        );
        assert_eq!(
            stored_token("t", "apns", None).push_platform(),
            PushPlatform::Apns
        );
    }

    #[test]
    fn stored_token_unknown_platform_falls_back_to_fcm() {
        // Defensive: an unexpected platform value must not panic the fanout
        // loop — it degrades to FCM (and is logged at warn by `push_platform`).
        assert_eq!(
            stored_token("t", "windows-phone", None).push_platform(),
            PushPlatform::Fcm
        );
    }

    // ------------------------------------------------------------------
    // Dispatch-target selection
    // ------------------------------------------------------------------

    #[test]
    fn select_targets_empty_filter_selects_all_non_blank() {
        let tokens = vec![
            stored_token("fcm-aaa", "fcm", None),
            stored_token("apns-bbb", "apns", None),
        ];
        let targets = select_dispatch_targets(&tokens, &PushTargetFilter::default());
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].platform, PushPlatform::Fcm);
        assert_eq!(targets[1].platform, PushPlatform::Apns);
    }

    #[test]
    fn select_targets_drops_blank_tokens() {
        let tokens = vec![
            stored_token("   ", "fcm", None),
            stored_token("", "apns", None),
            stored_token("real-token", "fcm", None),
        ];
        let targets = select_dispatch_targets(&tokens, &PushTargetFilter::default());
        assert_eq!(
            targets.len(),
            1,
            "only the non-blank token is a valid target"
        );
        assert_eq!(targets[0].token, "real-token");
    }

    #[test]
    fn select_targets_preserves_input_order() {
        // Callers fetch ORDER BY last_seen_at DESC; selection must keep the
        // most-recently-seen device first.
        let tokens = vec![
            stored_token("first", "fcm", None),
            stored_token("second", "apns", None),
            stored_token("third", "fcm", None),
        ];
        let targets = select_dispatch_targets(&tokens, &PushTargetFilter::default());
        let order: Vec<&str> = targets.iter().map(|t| t.token.as_str()).collect();
        assert_eq!(order, vec!["first", "second", "third"]);
    }

    #[test]
    fn select_targets_filters_by_platform() {
        let tokens = vec![
            stored_token("fcm-1", "fcm", None),
            stored_token("apns-1", "apns", None),
            stored_token("fcm-2", "fcm", None),
        ];
        let filter = PushTargetFilter {
            platforms: Some(vec![PushPlatform::Fcm]),
            ..Default::default()
        };
        let targets = select_dispatch_targets(&tokens, &filter);
        assert_eq!(targets.len(), 2);
        assert!(targets.iter().all(|t| t.platform == PushPlatform::Fcm));
    }

    #[test]
    fn select_targets_filters_by_app_id() {
        let tokens = vec![
            stored_token("mgmt", "fcm", Some("three.two.bit.ppt.management")),
            stored_token("reality", "fcm", Some("three.two.bit.ppt.reality")),
            stored_token("legacy-no-app", "fcm", None),
        ];
        let filter = PushTargetFilter {
            app_id: Some("three.two.bit.ppt.management".to_string()),
            ..Default::default()
        };
        let targets = select_dispatch_targets(&tokens, &filter);
        assert_eq!(targets.len(), 1, "only the matching bundle id is selected");
        assert_eq!(targets[0].token, "mgmt");
    }

    #[test]
    fn select_targets_app_id_filter_excludes_null_app_id() {
        // A token registered without an app_id must NOT match a specific-bundle
        // filter — otherwise a Reality push could wake a legacy un-tagged token.
        let tokens = vec![stored_token("legacy", "fcm", None)];
        let filter = PushTargetFilter {
            app_id: Some("three.two.bit.ppt.management".to_string()),
            ..Default::default()
        };
        assert!(select_dispatch_targets(&tokens, &filter).is_empty());
    }

    #[test]
    fn select_targets_empty_input_is_empty() {
        assert!(select_dispatch_targets(&[], &PushTargetFilter::default()).is_empty());
    }

    #[test]
    fn select_targets_carries_token_id_for_receipts() {
        let token = stored_token("fcm-x", "fcm", None);
        let id = token.id;
        let targets = select_dispatch_targets(&[token], &PushTargetFilter::default());
        assert_eq!(
            targets[0].token_id, id,
            "token_id must survive selection for receipts/eviction"
        );
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

    #[test]
    fn push_fanout_config_defaults() {
        let config = PushFanoutConfig::default();
        assert!(config.enabled);
        assert_eq!(config.poll_interval_secs, 30);
    }

    #[test]
    fn push_fanout_queue_key_is_stable() {
        // The producer (notification pipeline) and the worker must agree on the
        // list key. Pin it so a rename can't silently split producer/consumer.
        assert_eq!(PUSH_FANOUT_QUEUE_KEY, "push_fanout_queue");
    }

    #[test]
    fn queued_job_payload_roundtrips_to_notification() {
        // A push job on the queue is a JSON-serialized `Notification`. The
        // worker's `deliver_job` deserializes exactly this shape, so a
        // round-trip here pins the queue contract.
        let user_id = Uuid::new_v4();
        let original = Notification::new(
            user_id,
            common::notifications::NotificationCategory::Announcements,
            "Building update",
            "The lift will be serviced tomorrow.",
        )
        .with_priority(common::notifications::NotificationPriority::High);

        let payload = serde_json::to_string(&original).expect("serialize notification");
        let decoded: Notification =
            serde_json::from_str(&payload).expect("deserialize notification");

        assert_eq!(decoded.id, original.id);
        assert_eq!(decoded.user_id, user_id);
        assert_eq!(decoded.title, "Building update");
        assert_eq!(decoded.body, "The lift will be serviced tomorrow.");
        assert_eq!(
            decoded.category,
            common::notifications::NotificationCategory::Announcements
        );
        assert_eq!(
            decoded.priority,
            common::notifications::NotificationPriority::High
        );
    }

    #[test]
    fn malformed_queue_payload_fails_to_deserialize() {
        // `deliver_job` drops payloads that don't parse as a `Notification`
        // rather than re-queuing them (a poison message must not hot-loop).
        let err = serde_json::from_str::<Notification>("{\"not\":\"a notification\"}");
        assert!(err.is_err());
    }

    // ------------------------------------------------------------------
    // APNs config and JWT minting
    // ------------------------------------------------------------------

    #[test]
    fn apns_config_unconfigured_when_no_credentials() {
        let config = ApnsConfig {
            p8_key_pem: None,
            key_id: None,
            team_id: None,
            topic: "three.two.bit.ppt.management".to_string(),
            apns_base_url: None,
        };
        assert!(!config.is_configured());
    }

    #[test]
    fn apns_config_unconfigured_when_partial_credentials() {
        // All three fields must be present; partial config is not usable.
        let config = ApnsConfig {
            p8_key_pem: Some("--- key ---".to_string()),
            key_id: Some("ABCDE12345".to_string()),
            team_id: None, // missing
            topic: "three.two.bit.ppt.management".to_string(),
            apns_base_url: None,
        };
        assert!(!config.is_configured());
    }

    #[test]
    fn apns_config_configured_when_all_present() {
        let config = ApnsConfig {
            p8_key_pem: Some("--- key ---".to_string()),
            key_id: Some("ABCDE12345".to_string()),
            team_id: Some("TEAM123456".to_string()),
            topic: "three.two.bit.ppt.management".to_string(),
            apns_base_url: None,
        };
        assert!(config.is_configured());
    }

    #[test]
    fn apns_config_default_topic() {
        let config = ApnsConfig {
            p8_key_pem: None,
            key_id: None,
            team_id: None,
            topic: "three.two.bit.ppt.management".to_string(),
            apns_base_url: None,
        };
        assert_eq!(config.topic, "three.two.bit.ppt.management");
    }

    #[test]
    fn apns_config_base_url_defaults_to_apple() {
        let config = ApnsConfig {
            p8_key_pem: None,
            key_id: None,
            team_id: None,
            topic: "three.two.bit.ppt.management".to_string(),
            apns_base_url: None,
        };
        assert_eq!(config.base_url(), "https://api.push.apple.com");
    }

    #[test]
    fn apns_config_base_url_override() {
        let config = ApnsConfig {
            p8_key_pem: None,
            key_id: None,
            team_id: None,
            topic: "three.two.bit.ppt.management".to_string(),
            apns_base_url: Some("http://localhost:7777".to_string()),
        };
        assert_eq!(config.base_url(), "http://localhost:7777");
    }

    #[test]
    fn mint_apns_jwt_returns_none_when_unconfigured() {
        let config = ApnsConfig {
            p8_key_pem: None,
            key_id: None,
            team_id: None,
            topic: "three.two.bit.ppt.management".to_string(),
            apns_base_url: None,
        };
        assert!(mint_apns_jwt(&config).is_none());
    }

    #[test]
    fn mint_apns_jwt_returns_none_for_invalid_key_pem() {
        // A syntactically invalid PEM should fail gracefully (not panic).
        let config = ApnsConfig {
            p8_key_pem: Some("this is not a valid PEM key".to_string()),
            key_id: Some("ABCDE12345".to_string()),
            team_id: Some("TEAM123456".to_string()),
            topic: "three.two.bit.ppt.management".to_string(),
            apns_base_url: None,
        };
        assert!(mint_apns_jwt(&config).is_none());
    }

    // ------------------------------------------------------------------
    // CombinedPushAdapter routing logic (via PushTargetFilter)
    // ------------------------------------------------------------------

    #[test]
    fn apns_filter_selects_only_apns_tokens() {
        let tokens = vec![
            stored_token("fcm-1", "fcm", None),
            stored_token("apns-1", "apns", None),
            stored_token("apns-2", "apns", None),
        ];
        let filter = PushTargetFilter {
            platforms: Some(vec![PushPlatform::Apns]),
            ..Default::default()
        };
        let targets = select_dispatch_targets(&tokens, &filter);
        assert_eq!(targets.len(), 2);
        assert!(targets.iter().all(|t| t.platform == PushPlatform::Apns));
    }

    #[test]
    fn fcm_filter_selects_only_fcm_tokens() {
        let tokens = vec![
            stored_token("fcm-1", "fcm", None),
            stored_token("apns-1", "apns", None),
            stored_token("fcm-2", "fcm", None),
        ];
        let filter = PushTargetFilter {
            platforms: Some(vec![PushPlatform::Fcm]),
            ..Default::default()
        };
        let targets = select_dispatch_targets(&tokens, &filter);
        assert_eq!(targets.len(), 2);
        assert!(targets.iter().all(|t| t.platform == PushPlatform::Fcm));
    }

    #[test]
    fn no_targets_when_only_wrong_platform_tokens() {
        let tokens = vec![
            stored_token("fcm-1", "fcm", None),
            stored_token("fcm-2", "fcm", None),
        ];
        let filter = PushTargetFilter {
            platforms: Some(vec![PushPlatform::Apns]),
            ..Default::default()
        };
        // APNs adapter sees no targets → graceful no-op.
        assert!(select_dispatch_targets(&tokens, &filter).is_empty());
    }

    // ------------------------------------------------------------------
    // PushDeliveryReceipt construction (APNs)
    // ------------------------------------------------------------------

    #[test]
    fn apns_delivery_receipt_captures_platform() {
        let receipt = PushDeliveryReceipt {
            token_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            platform: PushPlatform::Apns,
            success: true,
            error: None,
            token_expired: false,
            attempted_at: chrono::Utc::now(),
        };
        assert_eq!(receipt.platform, PushPlatform::Apns);
        assert!(receipt.success);
        assert!(!receipt.token_expired);
    }

    #[test]
    fn apns_delivery_receipt_marks_expired_on_bad_token() {
        let receipt = PushDeliveryReceipt {
            token_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            platform: PushPlatform::Apns,
            success: false,
            error: Some("APNs rejected".to_string()),
            token_expired: true,
            attempted_at: chrono::Utc::now(),
        };
        assert!(!receipt.success);
        assert!(receipt.token_expired);
    }

    // ------------------------------------------------------------------
    // gap-84-4: pipeline push transport is multi-platform (FCM + APNs)
    // ------------------------------------------------------------------

    /// Regression guard for gap-84-4: the notification pipeline's push channel
    /// now dispatches through `CombinedPushAdapter`, which must treat BOTH
    /// Android (FCM) and iOS (APNs) tokens as first-class delivery targets.
    ///
    /// Before this fix the pipeline wired the FCM-only `FcmHttpAdapter`, which
    /// logged APNs tokens as "not yet implemented" and dropped them — so an
    /// iOS-only user's push notifications were silently discarded on the
    /// synchronous dispatch path. This test pins that a mixed and an APNs-only
    /// token set both yield real dispatch targets so the combined adapter can
    /// route them to their respective gateways.
    #[test]
    fn combined_push_adapter_treats_apns_as_first_class_target() {
        // APNs-only user: on the old FCM-only path this produced zero real
        // delivery targets (dropped as "unsupported"); it must now select the
        // APNs token so `ApnsHttpAdapter` can deliver it.
        let apns_only = vec![stored_token("apns-device", "apns", None)];
        let targets = select_dispatch_targets(&apns_only, &PushTargetFilter::default());
        assert_eq!(
            targets.len(),
            1,
            "an APNs-only user must have a real push dispatch target"
        );
        assert_eq!(targets[0].platform, PushPlatform::Apns);

        // Mixed user: both platforms are selected (order preserved), so the
        // combined adapter fans out to FCM and APNs concurrently.
        let mixed = vec![
            stored_token("fcm-device", "fcm", None),
            stored_token("apns-device", "apns", None),
        ];
        let mixed_targets = select_dispatch_targets(&mixed, &PushTargetFilter::default());
        let platforms: Vec<PushPlatform> =
            mixed_targets.iter().map(|t| t.platform.clone()).collect();
        assert!(
            platforms.contains(&PushPlatform::Fcm) && platforms.contains(&PushPlatform::Apns),
            "a mixed FCM+APNs user must produce targets for both platforms; got {platforms:?}"
        );
    }

    // ------------------------------------------------------------------
    // Issue #2301: CombinedPushAdapter routing + honest-metrics guards
    // ------------------------------------------------------------------

    /// FCM configured (project id present), APNs left unconfigured.
    fn fcm_only_configured() -> (FcmConfig, ApnsConfig) {
        (
            FcmConfig {
                project_id: Some("test-project".to_string()),
                oauth_token: None,
                fcm_base_url: None,
            },
            ApnsConfig {
                p8_key_pem: None,
                key_id: None,
                team_id: None,
                topic: "three.two.bit.ppt.management".to_string(),
                apns_base_url: None,
            },
        )
    }

    /// APNs configured, FCM left unconfigured. (The APNs P8 is invalid so JWT
    /// minting would fail — but the tests below never reach minting because the
    /// user has no APNs targets, so no network I/O happens.)
    fn apns_only_configured() -> (FcmConfig, ApnsConfig) {
        (
            FcmConfig {
                project_id: None,
                oauth_token: None,
                fcm_base_url: None,
            },
            ApnsConfig {
                p8_key_pem: Some("--- not a real key ---".to_string()),
                key_id: Some("ABCDE12345".to_string()),
                team_id: Some("TEAM123456".to_string()),
                topic: "three.two.bit.ppt.management".to_string(),
                apns_base_url: None,
            },
        )
    }

    /// A lazy pool pointed at an unreachable address — never touched by the
    /// tests below (they call `deliver_with_tokens`, which takes tokens directly
    /// and never queries the DB).
    fn unused_pool() -> DbPool {
        sqlx::PgPool::connect_lazy("postgres://never-used:never@127.0.0.1:1/never")
            .expect("connect_lazy builds without connecting")
    }

    fn test_notification() -> Notification {
        Notification::new(
            Uuid::new_v4(),
            common::notifications::NotificationCategory::Announcements,
            "Building update",
            "The lift will be serviced tomorrow.",
        )
    }

    /// Honest-metrics regression guard (Issue #2301, finding 2).
    ///
    /// FCM is configured but APNs is **not**, and the user is iOS-only (only APNs
    /// tokens). Nothing is actually delivered — FCM has no FCM targets, APNs is
    /// unconfigured — so `CombinedPushAdapter` must report `PushNotConfigured`
    /// (the pipeline records **Skipped**), NOT `Ok(())` (**Sent**).
    ///
    /// Before this fix `(Ok(()), Err(PushNotConfigured))` mapped to `Ok(())`,
    /// re-introducing the inflated-`sent` lie (Issue #484) for every iOS-only
    /// user. This exercises the real `CombinedPushAdapter` routing + combine path
    /// (no DB, no gateway: both providers short-circuit before any I/O).
    #[tokio::test]
    async fn ios_only_user_with_fcm_only_configured_is_skipped_not_sent() {
        let (fcm, apns) = fcm_only_configured();
        let adapter = CombinedPushAdapter::new(unused_pool(), fcm, apns);
        let tokens = vec![stored_token("apns-device", "apns", None)];

        let result = adapter
            .deliver_with_tokens(Uuid::new_v4(), &tokens, &test_notification())
            .await;

        assert!(
            matches!(result, Err(NotificationError::PushNotConfigured)),
            "iOS-only user, only FCM configured → nothing delivered → must record \
             Skipped (PushNotConfigured), got {result:?}"
        );
    }

    /// Symmetric honest-metrics guard: APNs configured, FCM not, Android-only
    /// user (only FCM tokens). Nothing delivered → Skipped, not Sent.
    #[tokio::test]
    async fn android_only_user_with_apns_only_configured_is_skipped_not_sent() {
        let (fcm, apns) = apns_only_configured();
        let adapter = CombinedPushAdapter::new(unused_pool(), fcm, apns);
        let tokens = vec![stored_token("fcm-device", "fcm", None)];

        let result = adapter
            .deliver_with_tokens(Uuid::new_v4(), &tokens, &test_notification())
            .await;

        assert!(
            matches!(result, Err(NotificationError::PushNotConfigured)),
            "Android-only user, only APNs configured → nothing delivered → must \
             record Skipped (PushNotConfigured), got {result:?}"
        );
    }

    /// `combine` pins the honest-metrics decision table directly (no I/O).
    #[test]
    fn combine_records_skipped_when_nothing_delivered_and_nothing_failed() {
        let uid = Uuid::new_v4();
        for (fcm, apns) in [
            // iOS-only user, FCM configured but no FCM targets, APNs not configured.
            (ProviderOutcome::NoTargets, ProviderOutcome::NotConfigured),
            // Android-only symmetric.
            (ProviderOutcome::NotConfigured, ProviderOutcome::NoTargets),
            // Neither provider configured.
            (
                ProviderOutcome::NotConfigured,
                ProviderOutcome::NotConfigured,
            ),
            // Both configured, user has zero registered devices.
            (ProviderOutcome::NoTargets, ProviderOutcome::NoTargets),
        ] {
            assert!(
                matches!(
                    CombinedPushAdapter::combine(uid, fcm, apns),
                    Err(NotificationError::PushNotConfigured)
                ),
                "no real delivery and no failure must record Skipped"
            );
        }
    }

    #[test]
    fn combine_reports_sent_when_any_provider_delivered() {
        let uid = Uuid::new_v4();
        assert!(CombinedPushAdapter::combine(
            uid,
            ProviderOutcome::Delivered,
            ProviderOutcome::NotConfigured,
        )
        .is_ok());
        assert!(CombinedPushAdapter::combine(
            uid,
            ProviderOutcome::NotConfigured,
            ProviderOutcome::Delivered,
        )
        .is_ok());
        // A delivery on one side masks a failure on the other (partial success).
        assert!(CombinedPushAdapter::combine(
            uid,
            ProviderOutcome::Delivered,
            ProviderOutcome::Failed(NotificationError::PushFailed("apns down".into())),
        )
        .is_ok());
    }

    #[test]
    fn combine_reports_failed_when_an_attempt_failed_and_nothing_delivered() {
        let uid = Uuid::new_v4();
        // APNs attempted and failed while FCM had no targets — honest Failed,
        // NOT Sent (the old `(Ok, Err)` → `Ok` arm would have lied here too).
        assert!(matches!(
            CombinedPushAdapter::combine(
                uid,
                ProviderOutcome::NoTargets,
                ProviderOutcome::Failed(NotificationError::PushFailed("apns down".into())),
            ),
            Err(NotificationError::PushFailed(_))
        ));
        // Both providers attempted and failed.
        assert!(matches!(
            CombinedPushAdapter::combine(
                uid,
                ProviderOutcome::Failed(NotificationError::PushFailed("fcm".into())),
                ProviderOutcome::Failed(NotificationError::PushFailed("apns".into())),
            ),
            Err(NotificationError::PushFailed(_))
        ));
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
}
