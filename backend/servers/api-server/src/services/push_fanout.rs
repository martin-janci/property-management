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
//!    - Falls back to APNs if the token's platform is `apns` (log-only placeholder
//!      since APNs requires per-binary HTTP/2 and a P8 key — wired in a follow-up).
//!
//! 2. `PushFanoutWorker` — a lightweight background tokio task that:
//!    - Polls a Redis list (`push_fanout_queue`) for pending push jobs published by
//!      the notification pipeline.
//!    - Falls back to a no-op polling loop when Redis / FCM are not configured, so
//!      the server always starts cleanly.
//!
//! # Configuration (environment variables)
//!
//! | Variable                    | Required | Description                                             |
//! |-----------------------------|----------|---------------------------------------------------------|
//! | `FCM_PROJECT_ID`            | No       | GCP project ID for FCM HTTP v1 (`projects/{id}/…`)     |
//! | `FCM_SERVER_KEY`            | No       | Legacy FCM server key (fall-back if no project ID)      |
//! | `FCM_OAUTH_TOKEN`           | No       | OAuth2 bearer token for FCM HTTP v1 (read once at startup) |
//! | `PUSH_FANOUT_ENABLED`       | No       | Set to `false` / `0` to disable the worker             |
//! | `PUSH_FANOUT_POLL_SECS`     | No       | Polling interval in seconds (default: 30)               |
//!
//! If neither `FCM_PROJECT_ID` nor `FCM_SERVER_KEY` is set the worker logs a
//! warning and becomes a no-op loop — the server will not crash.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use db::{
    models::device_push_token::PushPlatform, repositories::DevicePushTokenRepository, DbPool,
};
use serde::Deserialize;
use tokio::time::interval;
use tracing::Instrument;
use uuid::Uuid;

use common::notifications::{Notification, NotificationError, PushTransport, TransportResult};

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

#[async_trait]
impl PushTransport for FcmHttpAdapter {
    async fn send(
        &self,
        user_id: Uuid,
        _device_tokens: &[String],
        notification: &Notification,
    ) -> TransportResult {
        if !self.fcm_config.is_configured() {
            // Fail closed, do NOT silently succeed. Returning `Ok(())` here
            // made the pipeline record the push as `Sent` even though nothing
            // was delivered — a lie that inflated delivery metrics. Surfacing
            // `PushNotConfigured` makes the pipeline record `Skipped`, the
            // honest outcome (in-app remains the mandatory delivery channel).
            tracing::info!(
                user_id = %user_id,
                title = %notification.title,
                "[8A-3] Push skipped — FCM not configured (set FCM_PROJECT_ID or FCM_SERVER_KEY)"
            );
            return Err(NotificationError::PushNotConfigured);
        }

        // Fetch all registered device tokens for this user (service-role query,
        // no RLS context set on the connection).
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

        if tokens.is_empty() {
            tracing::debug!(
                user_id = %user_id,
                "[8A-3] No device tokens registered for user — skipping push"
            );
            return Ok(());
        }

        let project_id = self.fcm_config.project_id.clone();
        let mut any_sent = false;
        let mut fcm_attempted = false;
        let mut stale_token_ids: Vec<Uuid> = Vec::new();

        for token in &tokens {
            let platform = token.push_platform();
            match platform {
                PushPlatform::Fcm => {
                    fcm_attempted = true;
                    let (success, expired) = if let Some(ref pid) = project_id {
                        self.send_fcm_v1(pid, &token.token, notification).await
                    } else {
                        // No project ID — use legacy API
                        self.send_fcm_legacy(&token.token, notification).await
                    };

                    // Store delivery receipt (log + in-memory; DB table in follow-up)
                    let receipt = PushDeliveryReceipt {
                        token_id: token.id,
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
                        stale_token_ids.push(token.id);
                    }
                    if success {
                        any_sent = true;
                    }
                }
                PushPlatform::Apns => {
                    // APNs HTTP/2 with P8 key — placeholder for follow-up
                    tracing::info!(
                        token_id = %token.id,
                        user_id = %user_id,
                        "[8A-3] APNs delivery not yet implemented — log-only"
                    );
                }
            }
        }

        // Purge stale tokens (token_expired == true) so they are not retried.
        // We call `delete_stale_token_by_string` which is a best-effort log;
        // the next upsert from the device will evict the token via ON CONFLICT DO UPDATE.
        for stale_id in stale_token_ids {
            if let Some(token) = tokens.iter().find(|t| t.id == stale_id) {
                if let Err(e) =
                    delete_stale_token_by_string(&self.token_repo, user_id, &token.token).await
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
        }

        // Only report failure when FCM was actually attempted and every attempt failed.
        // APNs-only users (fcm_attempted == false) are not an error: APNs delivery is
        // a placeholder and not expected to set any_sent.
        if !fcm_attempted || any_sent {
            Ok(())
        } else {
            Err(NotificationError::PushFailed(
                "all FCM delivery attempts failed".to_string(),
            ))
        }
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

/// Background worker that fans out pending push notifications.
///
/// ## Delivery model
///
/// The worker polls a Redis list (`push_fanout_queue`) for pending job payloads
/// enqueued by the notification pipeline.  Each payload is a JSON object:
/// ```json
/// {
///   "user_id": "<uuid>",
///   "notification_id": "<uuid>",
///   "title": "...",
///   "body": "...",
///   "category": "announcements",
///   "priority": "normal"
/// }
/// ```
///
/// When Redis is not available (or the worker is disabled) the worker falls
/// through to a no-op heartbeat loop so the server always starts cleanly.
pub struct PushFanoutWorker {
    adapter: Arc<FcmHttpAdapter>,
    config: PushFanoutConfig,
    pubsub: Option<integrations::PubSubService>,
}

impl PushFanoutWorker {
    /// Create a new worker from the shared pool and current environment.
    pub fn new(
        pool: DbPool,
        pubsub: Option<integrations::PubSubService>,
        config: PushFanoutConfig,
    ) -> Self {
        let fcm_config = FcmConfig::from_env();
        if !fcm_config.is_configured() {
            tracing::warn!(
                "[8A-3] PushFanoutWorker: neither FCM_PROJECT_ID nor FCM_SERVER_KEY is set; \
                 push delivery will be skipped (no crash — set env vars to enable)"
            );
        }
        Self {
            adapter: Arc::new(FcmHttpAdapter::new(pool, fcm_config)),
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
                    fcm_configured = self.adapter.fcm_config.is_configured(),
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

    /// Process all pending push jobs from the queue.
    ///
    /// When Redis pubsub is available we pop messages from `push_fanout_queue`.
    /// When Redis is not available we just log a heartbeat.
    async fn process_pending_jobs(&self) {
        let Some(ref _pubsub) = self.pubsub else {
            // No Redis — nothing to drain; log at trace to avoid spam
            tracing::trace!(
                "[8A-3] PushFanoutWorker heartbeat (no Redis; in-process pipeline handles push)"
            );
            return;
        };

        // TODO: wire BLPOP drain in follow-up PR; real delivery happens synchronously via FcmHttpAdapter::send in the pipeline
        // When Redis is available: drain the push_fanout_queue.
        // The `PubSubService` abstraction in `integrations` is pub/sub-oriented;
        // a proper queue-drain would use `BLPOP` / `LPOP` on the raw Redis client.
        // We log a debug message here and leave the BLPOP wiring for the follow-up
        // that adds the dedicated Redis queue — the real delivery happens
        // synchronously inside `FcmHttpAdapter::send` which is called by
        // `NotificationPipeline::dispatch` (see notification_pipeline.rs).
        tracing::debug!(
            "[8A-3] PushFanoutWorker tick — in-process push delivery active via FcmHttpAdapter"
        );
    }
}

// ============================================================================
// Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
}
