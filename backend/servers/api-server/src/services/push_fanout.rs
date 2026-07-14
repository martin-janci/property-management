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
//! | Variable                    | Required | Description                                                         |
//! |-----------------------------|----------|---------------------------------------------------------------------|
//! | `FCM_PROJECT_ID`            | No       | GCP project ID for FCM HTTP v1 (`projects/{id}/…`)                  |
//! | `FCM_SERVER_KEY`            | No       | Legacy FCM server key (fall-back if no project ID)                  |
//! | `FCM_OAUTH_TOKEN`           | No       | OAuth2 bearer token for FCM HTTP v1 (read once at startup)          |
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
use serde::Deserialize;
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
    /// Legacy server key (fall-back when `project_id` is absent).
    pub server_key: Option<String>,
    /// OAuth2 bearer token for FCM HTTP v1 API.
    /// Read once at startup so it is not re-read on every send in the hot path.
    pub oauth_token: Option<String>,
    /// Base URL override for FCM v1 sends (e.g. a wiremock server in tests).
    ///
    /// When `None`, the production FCM base URL is used:
    /// `https://fcm.googleapis.com`.  Set this to override only in tests — the
    /// path segments (`/v1/projects/{id}/messages:send` and `/fcm/send`) are
    /// appended by the adapter regardless.
    pub fcm_base_url: Option<String>,
}

impl FcmConfig {
    /// Load from environment variables.
    pub fn from_env() -> Self {
        Self {
            project_id: std::env::var("FCM_PROJECT_ID").ok(),
            server_key: std::env::var("FCM_SERVER_KEY").ok(),
            oauth_token: std::env::var("FCM_OAUTH_TOKEN").ok(),
            fcm_base_url: None,
        }
    }

    /// Return `true` when at least one credential is configured.
    pub fn is_configured(&self) -> bool {
        self.project_id.is_some() || self.server_key.is_some()
    }

    /// Return the effective FCM base URL (production default when unset).
    pub fn base_url(&self) -> &str {
        self.fcm_base_url
            .as_deref()
            .unwrap_or("https://fcm.googleapis.com")
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
}

impl FcmHttpAdapter {
    /// Create a new adapter backed by the given pool and FCM config.
    pub fn new(pool: DbPool, fcm_config: FcmConfig) -> Self {
        Self {
            token_repo: DevicePushTokenRepository::new(pool),
            fcm_config,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("reqwest client build should not fail"),
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

        // For FCM HTTP v1 we need an OAuth2 bearer token.  In production this
        // would come from a service-account key file via `google-cloud-auth`.
        // We re-use `FCM_SERVER_KEY` as a bearer token here to keep the
        // dependency footprint minimal (no GCP SDK).  If `FCM_PROJECT_ID` is
        // set but only a server key is available we fall back to the legacy
        // send API (see `send_fcm_legacy`).
        let bearer = match self.fcm_config.oauth_token.clone() {
            Some(t) => t,
            None => {
                // Fall back to legacy if no OAuth token is available
                return self.send_fcm_legacy(device_token, notification).await;
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

    /// Legacy FCM send via `https://fcm.googleapis.com/fcm/send` using a
    /// server key in the `Authorization: key=…` header.
    ///
    /// Only called when a `FCM_SERVER_KEY` is available and no OAuth token
    /// is present.
    async fn send_fcm_legacy(
        &self,
        device_token: &str,
        notification: &Notification,
    ) -> (bool, bool) {
        let server_key = match &self.fcm_config.server_key {
            Some(k) => k.clone(),
            None => {
                tracing::warn!("[8A-3] FCM legacy send attempted but FCM_SERVER_KEY is not set");
                return (false, false);
            }
        };

        let body = serde_json::json!({
            "to": device_token,
            "notification": {
                "title": notification.title,
                "body": notification.body,
            },
            "data": {
                "notification_id": notification.id.to_string(),
                "category": notification.category.as_str(),
            }
        });

        let legacy_url = format!("{}/fcm/send", self.fcm_config.base_url());
        let resp = match self
            .http
            .post(&legacy_url)
            .header("Authorization", format!("key={server_key}"))
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "[8A-3] FCM legacy request failed (network error)"
                );
                return (false, false);
            }
        };

        let status = resp.status();
        // Legacy response: `{"multicast_id":…,"success":1,"failure":0,"results":[{"message_id":"…"}]}`
        // Or on token error: `{"results":[{"error":"NotRegistered"}]}`
        match resp.json::<serde_json::Value>().await {
            Ok(v) => {
                let success_count = v.get("success").and_then(|n| n.as_i64()).unwrap_or(0);
                if success_count > 0 {
                    return (true, false);
                }
                // Check for token expiry errors in results array
                let expired = v
                    .get("results")
                    .and_then(|r| r.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|item| item.get("error"))
                    .and_then(|e| e.as_str())
                    .map(|e| matches!(e, "NotRegistered" | "InvalidRegistration"))
                    .unwrap_or(false);
                tracing::warn!(
                    http_status = %status,
                    token_expired = expired,
                    "[8A-3] FCM legacy send failed"
                );
                (false, expired)
            }
            Err(e) => {
                tracing::warn!(
                    http_status = %status,
                    error = %e,
                    "[8A-3] Failed to parse FCM legacy response"
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
                "[8A-3] Push skipped — FCM not configured (set FCM_PROJECT_ID or FCM_SERVER_KEY)"
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

        let project_id = self.fcm_config.project_id.clone();
        let mut any_sent = false;
        let mut stale_tokens: Vec<(Uuid, String)> = Vec::new();

        for target in &targets {
            let (success, expired) = if let Some(ref pid) = project_id {
                self.send_fcm_v1(pid, &target.token, notification).await
            } else {
                // No project ID — use legacy API
                self.send_fcm_legacy(&target.token, notification).await
            };

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
    async fn send(
        &self,
        user_id: Uuid,
        _device_tokens: &[String],
        notification: &Notification,
    ) -> TransportResult {
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
    async fn send(
        &self,
        user_id: Uuid,
        _device_tokens: &[String],
        notification: &Notification,
    ) -> TransportResult {
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
}

impl CombinedPushAdapter {
    /// Create a combined adapter from the shared pool.
    pub fn from_env(pool: DbPool) -> Self {
        Self {
            fcm: FcmHttpAdapter::from_env(pool.clone()),
            apns: ApnsHttpAdapter::from_env(pool),
        }
    }

    /// Create a combined adapter with explicit configs (useful in tests).
    pub fn new(pool: DbPool, fcm_config: FcmConfig, apns_config: ApnsConfig) -> Self {
        Self {
            fcm: FcmHttpAdapter::new(pool.clone(), fcm_config),
            apns: ApnsHttpAdapter::new(pool, apns_config),
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
    async fn send(
        &self,
        user_id: Uuid,
        _device_tokens: &[String],
        notification: &Notification,
    ) -> TransportResult {
        // Fetch the user's device tokens ONCE. Previously `FcmHttpAdapter` and
        // `ApnsHttpAdapter` each ran `get_tokens_for_user` independently — 2N
        // identical queries on the `dispatch_to_users` fanout hot path. Fetch
        // here and hand both providers the shared slice.
        let tokens = match self.fcm.token_repo.get_tokens_for_user(user_id).await {
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
/// worker `BLPOP`s from the head and delivers each one via [`FcmHttpAdapter`].
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
/// and delivers each job via [`FcmHttpAdapter::send`], up to
/// [`MAX_JOBS_PER_TICK`] jobs per tick.
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
            tracing::warn!(
                "[8A-3] PushFanoutWorker: FCM not configured (FCM_PROJECT_ID / FCM_SERVER_KEY unset)"
            );
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
    /// deliver each one via [`FcmHttpAdapter::send`], up to
    /// [`MAX_JOBS_PER_TICK`] jobs per tick. When Redis is not available we just
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

        // `device_tokens` is unused by the adapters (they re-fetch from the DB),
        // so an empty slice is fine here.
        match self
            .adapter
            .send(notification.user_id, &[], &notification)
            .await
        {
            Ok(()) => {
                tracing::debug!(
                    notification_id = %notification.id,
                    user_id = %notification.user_id,
                    "[8A-3] PushFanoutWorker delivered queued push job"
                );
            }
            Err(e) => {
                // Delivery failed (FCM not configured, DB error, or all sends
                // failed). The in-app channel remains the mandatory record, so
                // we log and move on rather than re-queue (avoids hot-looping on
                // a permanently failing job).
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
            server_key: None,
            oauth_token: None,
            fcm_base_url: None,
        };
        assert!(!config.is_configured());
    }

    #[test]
    fn fcm_config_is_configured_when_project_id_present() {
        let config = FcmConfig {
            project_id: Some("my-gcp-project".to_string()),
            server_key: None,
            oauth_token: None,
            fcm_base_url: None,
        };
        assert!(config.is_configured());
    }

    #[test]
    fn fcm_config_is_configured_when_server_key_present() {
        let config = FcmConfig {
            project_id: None,
            server_key: Some("AAAA...key".to_string()),
            oauth_token: None,
            fcm_base_url: None,
        };
        assert!(config.is_configured());
    }

    #[test]
    fn fcm_config_is_configured_when_both_present() {
        let config = FcmConfig {
            project_id: Some("proj".to_string()),
            server_key: Some("key".to_string()),
            oauth_token: None,
            fcm_base_url: None,
        };
        assert!(config.is_configured());
    }

    #[test]
    fn fcm_config_base_url_defaults_to_google() {
        let config = FcmConfig {
            project_id: None,
            server_key: None,
            oauth_token: None,
            fcm_base_url: None,
        };
        assert_eq!(config.base_url(), "https://fcm.googleapis.com");
    }

    #[test]
    fn fcm_config_base_url_override() {
        let config = FcmConfig {
            project_id: None,
            server_key: None,
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
                server_key: None,
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
                server_key: None,
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
}
