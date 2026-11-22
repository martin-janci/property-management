//! APNs (Apple Push Notification service) HTTP/2 push transport.
//!
//! [`ApnsHttpAdapter`] implements [`PushTransport`](common::notifications::PushTransport)
//! for iOS: it authenticates with an ES256 provider JWT (minted from the P8
//! key), calls the APNs HTTP/2 provider API, and evicts stale
//! (`BadDeviceToken` / `Unregistered`) tokens.

use std::time::Duration;

use async_trait::async_trait;
use common::notifications::{Notification, NotificationError, PushTransport, TransportResult};
use db::{
    models::{device_push_token::PushPlatform, DevicePushToken},
    repositories::DevicePushTokenRepository,
    DbPool,
};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::Deserialize;
use uuid::Uuid;

use super::combined::ProviderOutcome;
use super::{
    build_push_http_client, delete_stale_token_by_string, select_dispatch_targets,
    PushDeliveryReceipt, PushTargetFilter,
};

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

/// Decide whether an APNs rejection `reason` means the device token is stale
/// and must be evicted from `device_push_tokens`.
///
/// This is the APNs counterpart to FCM's
/// [`FcmV1Error::indicates_stale_token`](super::fcm) — kept as a small pure
/// function so the "which rejection reasons evict the token" decision is
/// unit-testable without a live APNs gateway, and so a future refactor that
/// widens or narrows the matched set surfaces as a test diff.
///
/// Apple returns `BadDeviceToken` when the token is malformed / not valid for
/// the topic+environment, and `Unregistered` (HTTP 410) when the app was
/// uninstalled. Every other reason (throttling, payload-size, provider-token
/// problems, transient server errors, or an unrecognised value) is treated as
/// non-stale so a still-valid token is never evicted on a recoverable error.
fn apns_reason_indicates_stale(reason: &str) -> bool {
    matches!(reason, "BadDeviceToken" | "Unregistered")
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
    pub(crate) apns_config: ApnsConfig,
    /// Shared HTTP/2 client for APNs sends. `None` when the client could not be
    /// built at startup (TLS/resolver init failure); sends are then skipped
    /// rather than panicking the fanout worker.
    http: Option<reqwest::Client>,
}

impl ApnsHttpAdapter {
    /// Create a new adapter backed by the given pool and APNs config.
    pub fn new(pool: DbPool, apns_config: ApnsConfig) -> Self {
        let http = match build_push_http_client(Duration::from_secs(15), true) {
            Ok(client) => Some(client),
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "[8A-3] APNs: failed to build HTTP/2 send client; \
                     iOS push will be skipped until the process restarts"
                );
                None
            }
        };
        Self {
            token_repo: DevicePushTokenRepository::new(pool),
            apns_config,
            http,
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

        let Some(http) = self.http.as_ref() else {
            tracing::warn!(
                "[8A-3] APNs send skipped — HTTP client unavailable (build failed at startup)"
            );
            return (false, false);
        };

        let resp = match http
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
                let expired = apns_reason_indicates_stale(reason);
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
    pub(crate) async fn deliver(
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
                // Keep the raw DB error (which can carry connection-string
                // host/port/credentials) in the internal log only — never in the
                // returned error, which is persisted verbatim to the delivery
                // record via `into_failed(e.to_string())`.
                tracing::error!(
                    user_id = %user_id,
                    error = %e,
                    "[8A-3] APNs: failed to fetch device tokens"
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
    // DB-fetch error path must not leak raw DB errors (code-review
    // `push-db-err-leak`)
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

    fn configured_apns() -> ApnsConfig {
        ApnsConfig {
            p8_key_pem: Some("--- not a real key ---".to_string()),
            key_id: Some("ABCDE12345".to_string()),
            team_id: Some("TEAM123456".to_string()),
            topic: "three.two.bit.ppt.management".to_string(),
            apns_base_url: None,
        }
    }

    fn apns_test_notification() -> Notification {
        Notification::new(
            Uuid::new_v4(),
            common::notifications::NotificationCategory::Announcements,
            "Building update",
            "The lift will be serviced tomorrow.",
        )
    }

    /// Security regression (code-review `push-db-err-leak`): APNs `send()`'s
    /// device-token fetch failure must map to a FIXED, non-leaking message —
    /// never the raw `sqlx::Error` display (which can expose the DB
    /// host/port/credentials from the connection string). The raw error stays in
    /// the internal `tracing::error!` log only.
    #[tokio::test]
    async fn apns_db_fetch_error_message_does_not_leak_raw_db_error() {
        let adapter = ApnsHttpAdapter::new(failing_pool(), configured_apns());

        let result = adapter
            .send(Uuid::new_v4(), &apns_test_notification())
            .await;

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
    // Stale-token eviction decision (characterization — mirrors FCM's
    // `indicates_stale_token`). Pins exactly which APNs rejection reasons
    // evict a device token, so a refactor that widens or narrows the set
    // surfaces as a failing test rather than silently dropping (or keeping)
    // tokens in production.
    // ------------------------------------------------------------------

    #[test]
    fn apns_bad_device_token_reason_evicts() {
        assert!(apns_reason_indicates_stale("BadDeviceToken"));
    }

    #[test]
    fn apns_unregistered_reason_evicts() {
        // HTTP 410 Gone — app uninstalled.
        assert!(apns_reason_indicates_stale("Unregistered"));
    }

    #[test]
    fn apns_recoverable_reasons_do_not_evict() {
        // Throttling, payload-size, provider-token and transient-server errors
        // must NOT evict a still-valid token — only a genuinely dead token is
        // removed. Pinning the full non-stale set guards against a refactor
        // accidentally treating a recoverable error as fatal.
        for reason in [
            "PayloadTooLarge",
            "TooManyRequests",
            "ExpiredProviderToken",
            "InvalidProviderToken",
            "MissingProviderToken",
            "TopicDisallowed",
            "BadTopic",
            "DeviceTokenNotForTopic",
            "InternalServerError",
            "ServiceUnavailable",
            "Shutdown",
        ] {
            assert!(
                !apns_reason_indicates_stale(reason),
                "{reason} is recoverable and must not evict the token"
            );
        }
    }

    #[test]
    fn apns_unknown_or_empty_reason_does_not_evict() {
        // The delivery path substitutes "UNKNOWN" when APNs returns no `reason`
        // field; an unrecognised or empty reason must fail safe (keep token).
        assert!(!apns_reason_indicates_stale("UNKNOWN"));
        assert!(!apns_reason_indicates_stale(""));
        assert!(!apns_reason_indicates_stale("baddevicetoken")); // case-sensitive
    }

    #[test]
    fn apns_error_body_parses_reason_and_defaults_to_unknown() {
        // Characterize the wire-shape parsing that feeds the eviction decision:
        // a present `reason` is extracted verbatim and drives eviction, while a
        // body with no `reason` degrades to the non-stale "UNKNOWN" sentinel.
        let with_reason: ApnsErrorBody =
            serde_json::from_str(r#"{"reason":"Unregistered"}"#).expect("parses");
        let reason = with_reason.reason.as_deref().unwrap_or("UNKNOWN");
        assert_eq!(reason, "Unregistered");
        assert!(apns_reason_indicates_stale(reason));

        let empty_body: ApnsErrorBody = serde_json::from_str("{}").expect("parses");
        let reason = empty_body.reason.as_deref().unwrap_or("UNKNOWN");
        assert_eq!(reason, "UNKNOWN");
        assert!(!apns_reason_indicates_stale(reason));
    }
}
